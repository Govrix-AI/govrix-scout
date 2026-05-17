//! Govrix Scout proxy binary entry point.
//!
//! Starts two servers in the same process:
//! 1. Proxy server (hyper, port 4000) — hot path, intercepts agent traffic
//! 2. Management API server (axum, port 4001) — health, config, REST API
//!
//! Architecture:
//! - Proxy hot path uses hyper directly (NOT axum) for minimal latency overhead
//! - Management API uses axum (routing overhead is acceptable for non-hot-path)
//! - Both servers share a single Tokio runtime and connection pool
//! - Fail-open: if proxy encounters internal errors, traffic still forwards
//!
//! Event pipeline:
//! - Bounded mpsc channel (10,000 capacity) for fire-and-forget event writes
//! - Background writer task drains channel and logs/stores events
//! - Dropped events are counted (channel-full → fail-open)

use std::net::SocketAddr;

use govrix_scout_common::config::Config;
use tracing_subscriber::EnvFilter;

use govrix_scout_proxy::{api, events, proxy};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Logging ───────────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("GOVRIX_LOG_LEVEL").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // ── Configuration ─────────────────────────────────────────────────────────
    let config_path =
        std::env::var("GOVRIX_CONFIG").unwrap_or_else(|_| "config/govrix.default.toml".to_string());
    let config = Config::load_or_default(&config_path);

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        proxy_port = config.proxy.port,
        api_port = config.api.port,
        "Govrix Scout starting"
    );

    // ── Database pools ────────────────────────────────────────────────────────
    // Split into `api_pool` (read-heavy) and `writer_pool` (hot-path writes).
    // On failure, fall back to no-db mode so the proxy still forwards traffic.
    let pool_result = govrix_scout_store::connect_split(&config.database).await;
    let (api_pool, writer_pool) = match pool_result {
        Ok((api, writer)) => {
            tracing::info!("PostgreSQL pools established (api + writer split)");
            (Some(api), Some(writer))
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "PostgreSQL unavailable — API will serve stub responses; proxy continues"
            );
            (None, None)
        }
    };
    // `kill_switch_pool` is a cheap clone of the api pool used by the kill-switch.
    let kill_switch_pool = api_pool.clone();

    // ── Event channel ─────────────────────────────────────────────────────────
    let (event_sender, event_rx) =
        events::create_channel_with_capacity(config.events.channel_capacity);
    let event_metrics = event_sender.metrics().clone();

    // Shared Prometheus-facing metrics counters
    let metrics = events::Metrics::new();
    let event_sender = event_sender.with_prometheus(metrics.clone());

    // Anomaly alert broadcast channel — fan-out from writer pool to SSE subscribers.
    let (alert_tx, _alert_rx0) = tokio::sync::broadcast::channel::<
        govrix_scout_proxy::anomaly::AnomalyAlert,
    >(events::ALERT_BROADCAST_CAPACITY);

    // Spawn writer pool — N tasks pulling batches from a shared receiver.
    let writer_cfg = events::WriterConfig {
        writer_tasks: config.events.writer_tasks,
        batch_size: config.events.batch_size,
        batch_interval_ms: config.events.batch_interval_ms,
        anomaly: config.anomaly.clone(),
        alert_tx: Some(alert_tx.clone()),
    };
    tracing::info!(
        anomaly_enabled = config.anomaly.enabled,
        cold_start_window_hours = config.anomaly.cold_start_window_hours,
        state_lru_cap = config.anomaly.state_lru_cap,
        "anomaly detection configured (cold-start seed runs per writer task)"
    );
    let _writer_handles = events::spawn_writer_pool(
        event_rx,
        event_metrics,
        writer_pool,
        metrics.clone(),
        writer_cfg,
    );
    tracing::info!(
        capacity = config.events.channel_capacity,
        writer_tasks = config.events.writer_tasks,
        batch_size = config.events.batch_size,
        batch_interval_ms = config.events.batch_interval_ms,
        "event channel initialized"
    );

    // ── Server addresses ──────────────────────────────────────────────────────
    let proxy_addr: SocketAddr = format!("{}:{}", config.proxy.bind, config.proxy.port)
        .parse()
        .expect("invalid proxy bind address");

    let api_addr: SocketAddr = format!("{}:{}", config.api.bind, config.api.port)
        .parse()
        .expect("invalid API bind address");

    // ── Policy engine ─────────────────────────────────────────────────────────
    // Build the policy engine (YAML policy evaluation + budget tracking).
    // If no policy file is configured, the engine starts in allow-all mode.
    let policy_config_path = std::env::var("GOVRIX_POLICY_FILE")
        .unwrap_or_else(|_| "config/policies.example.yaml".to_string());

    let policy_engine = {
        let path = std::path::Path::new(&policy_config_path);
        if path.exists() {
            match govrix_scout_proxy::policy::PolicyEngine::from_file(path) {
                Ok(engine) => {
                    tracing::info!(path = %policy_config_path, "policy engine loaded from file");
                    engine
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        path = %policy_config_path,
                        "failed to load policy file — using allow-all policy"
                    );
                    govrix_scout_proxy::policy::PolicyEngine::noop()
                }
            }
        } else {
            tracing::info!("no policy file found — using allow-all policy");
            govrix_scout_proxy::policy::PolicyEngine::noop()
        }
    };

    // ── Budget counter pre-load ───────────────────────────────────────────────
    if let Some(ref p) = api_pool {
        policy_engine.load_budget_from_db(p).await;
    }

    let policy_hook: std::sync::Arc<dyn govrix_scout_proxy::policy::PolicyHook> =
        std::sync::Arc::new(policy_engine);

    // ── Proxy server ──────────────────────────────────────────────────────────
    let proxy_event_sender = event_sender.clone();
    let proxy_metrics = metrics.clone();
    let upstream_urls = proxy::UpstreamUrls {
        openai: config.proxy.upstream_openai.clone(),
        anthropic: config.proxy.upstream_anthropic.clone(),
        ..proxy::UpstreamUrls::default()
    };
    tracing::info!(
        openai = %upstream_urls.openai,
        anthropic = %upstream_urls.anthropic,
        "upstream URLs configured"
    );

    // Build the interceptor state explicitly so we can apply config knobs.
    let interceptor_state = if let Some(pool) = kill_switch_pool {
        proxy::InterceptorState::with_pool_and_upstream_urls(
            proxy_event_sender,
            proxy_metrics,
            policy_hook,
            upstream_urls,
            pool,
        )
    } else {
        proxy::InterceptorState::with_upstream_urls(
            proxy_event_sender,
            proxy_metrics,
            policy_hook,
            upstream_urls,
        )
    }
    .with_config(
        config.proxy.max_body_tee_bytes,
        config.proxy.kill_switch_ttl_secs,
        config.proxy.request_timeout_secs,
    );
    let interceptor_state = std::sync::Arc::new(interceptor_state);

    // Graceful shutdown channel: flipped to true by Ctrl-C handler.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    let proxy_handle = tokio::spawn({
        let state = interceptor_state.clone();
        let shutdown_rx = shutdown_rx.clone();
        async move {
            if let Err(e) = proxy::serve_state(proxy_addr, state, Some(shutdown_rx)).await {
                tracing::error!("proxy server error: {}", e);
            }
        }
    });

    // ── Management API server ─────────────────────────────────────────────────
    let api_config = config.clone();
    let api_metrics = metrics.clone();
    let api_alert_tx = alert_tx.clone();
    let api_handle = tokio::spawn(async move {
        let result = match api_pool {
            Some(p) => {
                api::serve_with_pool_and_alerts(api_addr, p, api_config, api_metrics, api_alert_tx)
                    .await
            }
            None => api::serve(api_addr).await,
        };
        if let Err(e) = result {
            tracing::error!("API server error: {}", e);
        }
    });

    // ── Shutdown: SIGINT/Ctrl-C signals the watch channel ────────────────────
    tokio::select! {
        _ = proxy_handle => {
            tracing::warn!("proxy server exited unexpectedly");
        }
        _ = api_handle => {
            tracing::warn!("API server exited unexpectedly");
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("received SIGINT — signalling shutdown");
            let _ = shutdown_tx.send(true);
            // Brief grace period to let the proxy drain the mpsc and writers flush.
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            tracing::info!("shutdown complete");
        }
    }

    Ok(())
}
