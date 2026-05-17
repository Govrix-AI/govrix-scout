//! TOML configuration parsing with environment variable overrides.
//!
//! All environment variables use the `GOVRIX_` prefix.
//! Env vars take precedence over values in the TOML file.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Top-level Govrix Scout configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub proxy: ProxyConfig,
    pub database: DatabaseConfig,
    pub api: ApiConfig,
    pub dashboard: DashboardConfig,
    pub retention: RetentionConfig,
    pub telemetry: TelemetryConfig,
    pub events: EventsConfig,
    pub anomaly: AnomalyConfig,
}

/// Proxy server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Port the proxy listens on.
    pub port: u16,
    /// Bind address.
    pub bind: String,
    /// Maximum request body size to tee (bytes). Larger bodies are not analyzed.
    pub max_body_tee_bytes: usize,
    /// Whether to fail-open if upstream is unreachable.
    pub fail_open: bool,
    /// Upstream connection timeout in milliseconds.
    pub upstream_timeout_ms: u64,
    /// Default upstream base URL (overridden per-provider).
    pub upstream_openai: String,
    pub upstream_anthropic: String,
    /// End-to-end proxy handler timeout in seconds. Long-running streaming
    /// responses are bounded by this; default 300s.
    pub request_timeout_secs: u64,
    /// Kill-switch (agent status) cache TTL in seconds. The proxy caches
    /// agent status to avoid a DB round-trip on every request.
    pub kill_switch_ttl_secs: u64,
}

/// Database configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL. Use `GOVRIX_DATABASE_URL` env var in production.
    pub url: String,
    /// Maximum connections in the pool (kept for backward compat — the proxy
    /// now splits into `api_pool_max` + `writer_pool_max`; this value is used
    /// when neither override is set).
    pub max_connections: u32,
    /// Minimum connections to keep alive.
    pub min_connections: u32,
    /// Connection acquire timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Maximum connections for the API/read pool (default 30).
    pub api_pool_max: u32,
    /// Maximum connections for the writer/hot-path pool (default 70).
    /// Sum of `api_pool_max` + `writer_pool_max` should stay under Postgres's
    /// `max_connections` limit (typically 100).
    pub writer_pool_max: u32,
}

/// Event pipeline configuration (channel, writer tasks, batching).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EventsConfig {
    /// Capacity of the bounded mpsc event channel.
    pub channel_capacity: usize,
    /// Number of background writer tasks (each pulls batches from the channel).
    pub writer_tasks: usize,
    /// Maximum events per batch before flushing.
    pub batch_size: usize,
    /// Maximum interval (ms) to wait before flushing a partial batch.
    pub batch_interval_ms: u64,
    /// Sink implementation: `"mpsc"` (default, in-process) or `"redis"`
    /// (Tier-2, multi-replica via Redis Streams; requires `redis-sink` feature).
    pub sink: String,
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 100_000,
            writer_tasks: 4,
            batch_size: 500,
            batch_interval_ms: 50,
            sink: "mpsc".to_string(),
        }
    }
}

/// Anomaly detection configuration (Stage 2 — post-flush detectors).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnomalyConfig {
    /// Master switch for the post-flush anomaly orchestrator.
    pub enabled: bool,
    /// |z-score| threshold for cost anomalies (default 3.0).
    pub cost_z_threshold: f64,
    /// Latency must exceed `p99 * latency_p99_multiplier` to alert (default 1.5).
    pub latency_p99_multiplier: f64,
    /// Maximum number of (agent, model) sketch entries kept in memory.
    pub state_lru_cap: usize,
    /// How many hours of historical data to load on cold-start seed.
    pub cold_start_window_hours: u32,
    /// Error rate must exceed `baseline * error_rate_multiplier` to alert.
    pub error_rate_multiplier: f64,
}

impl Default for AnomalyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cost_z_threshold: 3.0,
            latency_p99_multiplier: 1.5,
            state_lru_cap: 10_000,
            cold_start_window_hours: 24,
            error_rate_multiplier: 2.0,
        }
    }
}

/// REST API server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Port for the management API (Axum).
    pub port: u16,
    /// Bind address.
    pub bind: String,
    /// Whether to enable CORS for dashboard access.
    pub cors_enabled: bool,
    /// Allowed CORS origins (comma-separated, or "*").
    pub cors_origins: Vec<String>,
}

/// Dashboard serving configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// Port the dashboard Vite dev server or nginx serves on.
    pub port: u16,
}

/// Data retention configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Event retention in days (TimescaleDB retention policy).
    pub events_days: u32,
    /// Cost aggregation retention in days.
    pub cost_days: u32,
}

/// Observability / telemetry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Log level (trace, debug, info, warn, error).
    pub log_level: String,
    /// Whether to enable Prometheus metrics endpoint at /metrics.
    pub prometheus_enabled: bool,
    /// Prometheus metrics port.
    pub prometheus_port: u16,
}

// ── Defaults ──────────────────────────────────────────────────────────────────

// Config::default() delegates to each sub-struct's Default impl.
// clippy::derivable_impls would suggest adding #[derive(Default)] to Config,
// but we keep the explicit impl for clarity and to document the delegation.
#[allow(clippy::derivable_impls)]
impl Default for Config {
    fn default() -> Self {
        Self {
            proxy: ProxyConfig::default(),
            database: DatabaseConfig::default(),
            api: ApiConfig::default(),
            dashboard: DashboardConfig::default(),
            retention: RetentionConfig::default(),
            telemetry: TelemetryConfig::default(),
            events: EventsConfig::default(),
            anomaly: AnomalyConfig::default(),
        }
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            port: 4000,
            bind: "0.0.0.0".to_string(),
            max_body_tee_bytes: 1_048_576, // 1 MiB
            fail_open: true,
            upstream_timeout_ms: 30_000,
            upstream_openai: "https://api.openai.com".to_string(),
            upstream_anthropic: "https://api.anthropic.com".to_string(),
            request_timeout_secs: 300,
            kill_switch_ttl_secs: 30,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://govrix_scout:govrix_scout@localhost:5432/govrix_scout".to_string(),
            max_connections: 20,
            min_connections: 2,
            connect_timeout_secs: 10,
            api_pool_max: 30,
            writer_pool_max: 70,
        }
    }
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            port: 4001,
            bind: "0.0.0.0".to_string(),
            cors_enabled: true,
            cors_origins: vec!["http://localhost:3000".to_string()],
        }
    }
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self { port: 3000 }
    }
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            events_days: 7,
            cost_days: 90,
        }
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            prometheus_enabled: true,
            prometheus_port: 9090,
        }
    }
}

// ── Loading ───────────────────────────────────────────────────────────────────

impl Config {
    /// Load configuration from a TOML file, then apply environment overrides.
    ///
    /// Environment variables (all prefixed with `GOVRIX_`):
    /// - `GOVRIX_DATABASE_URL`        → database.url
    /// - `GOVRIX_PROXY_PORT`          → proxy.port
    /// - `GOVRIX_API_PORT`            → api.port
    /// - `GOVRIX_LOG_LEVEL`           → telemetry.log_level
    /// - `GOVRIX_PROXY_FAIL_OPEN`     → proxy.fail_open
    pub fn load(path: impl AsRef<Path>) -> Result<Self, crate::errors::ConfigError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|source| {
            crate::errors::ConfigError::FileRead {
                path: path.display().to_string(),
                source,
            }
        })?;
        let mut config: Config = toml::from_str(&content)?;
        config.apply_env_overrides();
        Ok(config)
    }

    /// Load from the default config path, falling back to `Config::default()` if missing.
    pub fn load_or_default(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        if path.exists() {
            Self::load(path).unwrap_or_else(|e| {
                tracing::warn!(
                    "failed to load config from {}: {}, using defaults",
                    path.display(),
                    e
                );
                Self::default()
            })
        } else {
            let mut config = Self::default();
            config.apply_env_overrides();
            config
        }
    }

    /// Apply `GOVRIX_*` environment variable overrides.
    fn apply_env_overrides(&mut self) {
        if let Ok(url) = std::env::var("GOVRIX_DATABASE_URL") {
            self.database.url = url;
        }
        if let Ok(port) = std::env::var("GOVRIX_PROXY_PORT") {
            if let Ok(p) = port.parse() {
                self.proxy.port = p;
            }
        }
        if let Ok(port) = std::env::var("GOVRIX_API_PORT") {
            if let Ok(p) = port.parse() {
                self.api.port = p;
            }
        }
        if let Ok(level) = std::env::var("GOVRIX_LOG_LEVEL") {
            self.telemetry.log_level = level;
        }
        if let Ok(val) = std::env::var("GOVRIX_PROXY_FAIL_OPEN") {
            self.proxy.fail_open = val.to_lowercase() != "false";
        }
        if let Ok(url) = std::env::var("GOVRIX_PROXY__UPSTREAM_OPENAI") {
            self.proxy.upstream_openai = url;
        }
        if let Ok(url) = std::env::var("GOVRIX_PROXY__UPSTREAM_ANTHROPIC") {
            self.proxy.upstream_anthropic = url;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();
        assert_eq!(config.proxy.port, 4000);
        assert_eq!(config.api.port, 4001);
        assert_eq!(config.retention.events_days, 7);
        assert!(config.proxy.fail_open);
    }
}
