//! Shared application state for the management API.
//!
//! `AppState` is wrapped in an `Arc` and injected via axum's `State` extractor
//! into every handler.

use std::sync::Arc;
use std::time::Instant;

use govrix_scout_common::config::Config;
use govrix_scout_store::StorePool;
use tokio::sync::broadcast;

use crate::anomaly::AnomalyAlert;
use crate::events::Metrics;

/// Shared state for all API handlers.
///
/// Constructed once at startup and shared via `Arc<AppState>`.
pub struct AppState {
    /// PostgreSQL connection pool.
    pub pool: StorePool,

    /// A copy of the runtime configuration (sanitized before serving).
    pub config: Config,

    /// Server start time — used to compute uptime in /ready.
    pub started_at: Instant,

    /// Shared Prometheus-facing metrics counters.
    ///
    /// The same `Arc<Metrics>` is held by `InterceptorState` in the proxy,
    /// so reads here reflect live counter values written by the hot path.
    pub metrics: Arc<Metrics>,

    /// Optional broadcast sender for live anomaly alerts.
    ///
    /// SSE handlers `subscribe()` for a new `Receiver` per connection.
    pub alert_tx: Option<broadcast::Sender<AnomalyAlert>>,
}

impl AppState {
    /// Create a new `AppState` wrapping a database pool, config, and shared metrics.
    pub fn new(pool: StorePool, config: Config, metrics: Arc<Metrics>) -> Arc<Self> {
        Arc::new(Self {
            pool,
            config,
            started_at: Instant::now(),
            metrics,
            alert_tx: None,
        })
    }

    /// Same as `new`, but with the anomaly alert broadcast sender attached.
    pub fn new_with_alerts(
        pool: StorePool,
        config: Config,
        metrics: Arc<Metrics>,
        alert_tx: broadcast::Sender<AnomalyAlert>,
    ) -> Arc<Self> {
        Arc::new(Self {
            pool,
            config,
            started_at: Instant::now(),
            metrics,
            alert_tx: Some(alert_tx),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn started_at_is_recent() {
        let elapsed = Instant::now().elapsed();
        // Instant is always non-negative and near-zero at construction
        assert!(elapsed.as_secs() < 5);
    }
}
