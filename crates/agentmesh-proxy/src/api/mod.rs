//! Management API server (Axum, port 4001).
//!
//! Serves non-hot-path endpoints:
//! - Health checks (/health, /ready)
//! - Prometheus metrics (/metrics)
//! - REST API for events, agents, costs, reports, config
//!
//! Uses axum's Router — acceptable here since this is NOT the proxy hot path.
//!
//! Architecture:
//! - `state`      — shared AppState (pool + config + uptime)
//! - `router`     — axum Router wiring all routes
//! - `handlers/`  — one module per resource (health, events, agents, costs, reports, config)
//! - `middleware/` — cors, auth

pub mod handlers;
pub mod middleware;
pub mod router;
pub mod state;

use std::net::SocketAddr;
use std::sync::Arc;

use agentmesh_common::config::Config;

use crate::events::Metrics;

/// Start the Axum management API server with database connectivity.
///
/// This is the primary entry point. It connects to PostgreSQL, builds AppState,
/// and serves the full REST API.
///
/// The `metrics` Arc is shared with the proxy server so the `/metrics` endpoint
/// reflects live counter values written by the proxy hot path.
pub async fn serve_with_pool(
    addr: SocketAddr,
    pool: agentmesh_store::StorePool,
    config: Config,
    metrics: Arc<Metrics>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = state::AppState::new(pool, config, metrics);
    let app = router::create_router_with_auth(state);

    tracing::info!("management API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Start the Axum management API server without a database pool.
///
/// Used during startup before the pool is available, or in tests.
/// Returns stub responses for all database-backed endpoints.
pub async fn serve(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = router::build_router();

    tracing::info!("management API (no-db mode) listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
