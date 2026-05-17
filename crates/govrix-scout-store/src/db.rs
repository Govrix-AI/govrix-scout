//! Database connection pool management.
//!
//! Creates and manages a `sqlx::PgPool` configured from `govrix-scout-common::Config`.

use govrix_scout_common::config::DatabaseConfig;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

/// Type alias for the shared connection pool.
pub type StorePool = PgPool;

/// Create a new PostgreSQL connection pool from the given `DatabaseConfig`.
///
/// This function should be called once at startup and the pool shared via `Arc` or
/// injected into handler state.
pub async fn connect(cfg: &DatabaseConfig) -> Result<StorePool, sqlx::Error> {
    tracing::info!(
        url = %cfg.url.replace(
            // Redact password in log output
            cfg.url.split('@').next().unwrap_or(""),
            "[redacted]"
        ),
        max_connections = cfg.max_connections,
        "connecting to PostgreSQL"
    );

    let pool = PgPoolOptions::new()
        .max_connections(cfg.max_connections)
        .min_connections(cfg.min_connections)
        .acquire_timeout(Duration::from_secs(cfg.connect_timeout_secs))
        .connect(&cfg.url)
        .await?;

    tracing::info!("PostgreSQL connection pool established");
    Ok(pool)
}

/// Connect using a custom max-connection cap, otherwise mirroring `connect`.
///
/// Used to build separate API and writer pools backed by the same database URL
/// so the read-heavy API and write-heavy event writer do not contend for
/// connections.
pub async fn connect_with_max(cfg: &DatabaseConfig, max: u32) -> Result<StorePool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(max)
        .min_connections(cfg.min_connections)
        .acquire_timeout(Duration::from_secs(cfg.connect_timeout_secs))
        .connect(&cfg.url)
        .await?;
    Ok(pool)
}

/// Build (api_pool, writer_pool) split per `DatabaseConfig::api_pool_max` and
/// `writer_pool_max`. Falls back to `max_connections` when the splits are zero.
pub async fn connect_split(cfg: &DatabaseConfig) -> Result<(StorePool, StorePool), sqlx::Error> {
    let api_max = if cfg.api_pool_max == 0 {
        cfg.max_connections
    } else {
        cfg.api_pool_max
    };
    let writer_max = if cfg.writer_pool_max == 0 {
        cfg.max_connections
    } else {
        cfg.writer_pool_max
    };
    tracing::info!(
        api_max,
        writer_max,
        "connecting to PostgreSQL with split pools"
    );
    let api_pool = connect_with_max(cfg, api_max).await?;
    let writer_pool = connect_with_max(cfg, writer_max).await?;
    Ok((api_pool, writer_pool))
}

/// Run a connectivity check against the database.
pub async fn health_check(pool: &StorePool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}
