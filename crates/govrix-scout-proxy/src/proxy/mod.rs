//! Proxy server — hyper-based transparent proxy hot path.
//!
//! Architecture (from rust-proxy skill):
//! - Uses hyper directly, NOT axum, for the hot path
//! - SSE responses stream through without buffering
//! - Request body tee: read once, clone bytes for analysis
//! - All analysis is fire-and-forget from the forwarding path
//! - Fail-open: internal errors never block client traffic

pub mod agent_detect;
pub mod handler;
pub mod interceptor;
pub mod streaming;
pub mod upstream;

use std::net::SocketAddr;
use std::sync::Arc;

use crate::events::Metrics;
use crate::events_sink::EventSink;
use crate::policy::{NoOpPolicy, PolicyHook};
pub use interceptor::InterceptorState;
pub use upstream::UpstreamUrls;

/// Start the hyper proxy server with default upstream URLs.
///
/// Binds to `addr` and serves all incoming connections through `handler::proxy_handler`.
/// The `event_sender` is shared across all connections via Arc.
/// The `metrics` Arc is shared with the management API for real counter reads.
pub async fn serve(
    addr: SocketAddr,
    event_sink: Arc<dyn EventSink>,
    metrics: Arc<Metrics>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    serve_with_policy(addr, event_sink, metrics, Arc::new(NoOpPolicy)).await
}

/// Start the hyper proxy server with a custom policy hook and default upstream URLs.
pub async fn serve_with_policy(
    addr: SocketAddr,
    event_sink: Arc<dyn EventSink>,
    metrics: Arc<Metrics>,
    policy_hook: Arc<dyn PolicyHook>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    serve_full(
        addr,
        event_sink,
        metrics,
        policy_hook,
        UpstreamUrls::default(),
    )
    .await
}

/// Start the hyper proxy server with a custom policy hook and custom upstream URLs.
///
/// This is the fully-parameterized entry point. Use this for integration testing
/// with mock upstream servers.
pub async fn serve_full(
    addr: SocketAddr,
    event_sink: Arc<dyn EventSink>,
    metrics: Arc<Metrics>,
    policy_hook: Arc<dyn PolicyHook>,
    upstream_urls: UpstreamUrls,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    serve_full_with_pool(addr, event_sink, metrics, policy_hook, upstream_urls, None).await
}

/// Start the hyper proxy server with a database pool for kill-switch enforcement.
///
/// When `db_pool` is `Some`, the proxy checks agent status before forwarding
/// each request. Blocked agents receive a 403. When `None`, the check is skipped
/// and the proxy operates in fail-open mode (same as `serve_full`).
pub async fn serve_full_with_pool(
    addr: SocketAddr,
    event_sink: Arc<dyn EventSink>,
    metrics: Arc<Metrics>,
    policy_hook: Arc<dyn PolicyHook>,
    upstream_urls: UpstreamUrls,
    db_pool: Option<govrix_scout_store::StorePool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = if let Some(pool) = db_pool {
        InterceptorState::with_pool_and_upstream_urls(
            event_sink,
            metrics,
            policy_hook,
            upstream_urls,
            pool,
        )
    } else {
        InterceptorState::with_upstream_urls(event_sink, metrics, policy_hook, upstream_urls)
    };
    serve_state(addr, Arc::new(state), None).await
}

/// Start the hyper proxy with a fully-built `InterceptorState`.
///
/// Use this when callers need to tweak `max_body_tee_bytes`, the kill-switch
/// cache TTL, or the request-timeout via `InterceptorState::with_config`.
/// Pass `shutdown` to enable graceful shutdown — when the receiver fires,
/// the listener stops accepting new connections and the function returns.
pub async fn serve_state(
    addr: std::net::SocketAddr,
    state: Arc<InterceptorState>,
    shutdown: Option<tokio::sync::watch::Receiver<bool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use hyper::server::conn::http1;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("proxy listening on {}", addr);

    loop {
        let accept = listener.accept();
        let (stream, peer_addr) = match shutdown.clone() {
            Some(mut rx) => tokio::select! {
                res = accept => res?,
                _ = rx.changed() => {
                    if *rx.borrow() {
                        tracing::info!("proxy: shutdown signal received — stopping accept loop");
                        return Ok(());
                    } else {
                        continue;
                    }
                }
            },
            None => accept.await?,
        };

        let io = TokioIo::new(stream);
        let state_clone = Arc::clone(&state);

        tokio::spawn(async move {
            let svc = hyper::service::service_fn(move |req| {
                let state = Arc::clone(&state_clone);
                handler::proxy_handler(req, peer_addr, state)
            });

            if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
                tracing::debug!("connection error from {}: {}", peer_addr, e);
            }
        });
    }
}
