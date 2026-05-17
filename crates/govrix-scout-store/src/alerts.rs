//! Anomaly alert persistence — insert and query the `anomaly_alerts` hypertable.
//!
//! The proxy crate's `AnomalyAlert` type is not visible here (store does not
//! depend on proxy). Callers marshal their alert into [`NewAlert`] at the call
//! site before invoking [`insert_alerts_batch`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::StorePool;

// ── Input / output types ─────────────────────────────────────────────────────

/// Input shape for batch-inserting alerts.
///
/// Mirrors the columns of the `anomaly_alerts` table. The proxy crate marshals
/// its `AnomalyAlert` into this shape at the call site.
#[derive(Debug, Clone)]
pub struct NewAlert {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub agent_id: String,
    pub session_id: Option<Uuid>,
    pub detector: String,
    pub severity: String,
    pub score: f64,
    pub details: serde_json::Value,
}

/// Row shape returned by alert queries.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AlertRow {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub agent_id: String,
    pub session_id: Option<Uuid>,
    pub detector: String,
    pub severity: String,
    pub score: f64,
    pub details: serde_json::Value,
    pub acknowledged_at: Option<DateTime<Utc>>,
}

/// Optional filters for listing alerts.
#[derive(Debug, Clone, Default)]
pub struct AlertFilter {
    pub agent_id: Option<String>,
    pub severity: Option<String>,
    pub detector: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: i64,
}

impl AlertFilter {
    pub fn new() -> Self {
        Self {
            limit: 100,
            ..Default::default()
        }
    }
}

// ── Write path ───────────────────────────────────────────────────────────────

/// Insert a batch of alerts using a single UNNEST round-trip.
///
/// Returns the number of rows inserted. Fail-soft callers should treat any
/// `Err` as non-fatal — the proxy must continue capturing events even if
/// alert persistence fails.
pub async fn insert_alerts_batch(
    pool: &StorePool,
    alerts: &[NewAlert],
) -> Result<usize, sqlx::Error> {
    if alerts.is_empty() {
        return Ok(0);
    }

    let n = alerts.len();
    let mut ids: Vec<Uuid> = Vec::with_capacity(n);
    let mut timestamps: Vec<DateTime<Utc>> = Vec::with_capacity(n);
    let mut agent_ids: Vec<String> = Vec::with_capacity(n);
    let mut session_ids: Vec<Option<Uuid>> = Vec::with_capacity(n);
    let mut detectors: Vec<String> = Vec::with_capacity(n);
    let mut severities: Vec<String> = Vec::with_capacity(n);
    let mut scores: Vec<f64> = Vec::with_capacity(n);
    let mut details: Vec<serde_json::Value> = Vec::with_capacity(n);

    for a in alerts {
        ids.push(a.id);
        timestamps.push(a.timestamp);
        agent_ids.push(a.agent_id.clone());
        session_ids.push(a.session_id);
        detectors.push(a.detector.clone());
        severities.push(a.severity.clone());
        scores.push(a.score);
        details.push(a.details.clone());
    }

    sqlx::query(
        r#"
        INSERT INTO anomaly_alerts (
            id, timestamp, agent_id, session_id,
            detector, severity, score, details
        )
        SELECT * FROM UNNEST(
            $1::uuid[], $2::timestamptz[], $3::text[], $4::uuid[],
            $5::text[], $6::text[], $7::float8[], $8::jsonb[]
        )
        "#,
    )
    .bind(&ids)
    .bind(&timestamps)
    .bind(&agent_ids)
    .bind(&session_ids)
    .bind(&detectors)
    .bind(&severities)
    .bind(&scores)
    .bind(&details)
    .execute(pool)
    .await?;

    Ok(n)
}

// ── Read path ────────────────────────────────────────────────────────────────

/// List alerts matching `filter`, ordered by timestamp DESC.
pub async fn list_alerts(
    pool: &StorePool,
    filter: AlertFilter,
) -> Result<Vec<AlertRow>, sqlx::Error> {
    use sqlx::Row;

    let mut conditions: Vec<String> = Vec::new();
    let mut idx = 1usize;

    if filter.agent_id.is_some() {
        conditions.push(format!("agent_id = ${idx}"));
        idx += 1;
    }
    if filter.severity.is_some() {
        conditions.push(format!("severity = ${idx}"));
        idx += 1;
    }
    if filter.detector.is_some() {
        conditions.push(format!("detector = ${idx}"));
        idx += 1;
    }
    if filter.since.is_some() {
        conditions.push(format!("timestamp >= ${idx}"));
        idx += 1;
    }
    if filter.until.is_some() {
        conditions.push(format!("timestamp < ${idx}"));
        idx += 1;
    }
    let limit_param = idx;

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        r#"
        SELECT id, timestamp, agent_id, session_id,
               detector, severity, score, details, acknowledged_at
        FROM anomaly_alerts
        {where_clause}
        ORDER BY timestamp DESC
        LIMIT ${limit_param}
        "#,
    );

    let mut q = sqlx::query(&sql);
    if let Some(ref v) = filter.agent_id {
        q = q.bind(v);
    }
    if let Some(ref v) = filter.severity {
        q = q.bind(v);
    }
    if let Some(ref v) = filter.detector {
        q = q.bind(v);
    }
    if let Some(v) = filter.since {
        q = q.bind(v);
    }
    if let Some(v) = filter.until {
        q = q.bind(v);
    }
    let limit = filter.limit.clamp(1, 1000);
    q = q.bind(limit);

    let rows = q.fetch_all(pool).await?;
    let out = rows
        .into_iter()
        .map(|r| AlertRow {
            id: r.try_get("id").unwrap_or_else(|_| Uuid::nil()),
            timestamp: r
                .try_get("timestamp")
                .unwrap_or_else(|_| chrono::DateTime::<Utc>::from_timestamp(0, 0).unwrap()),
            agent_id: r.try_get("agent_id").unwrap_or_default(),
            session_id: r.try_get("session_id").ok().flatten(),
            detector: r.try_get("detector").unwrap_or_default(),
            severity: r.try_get("severity").unwrap_or_default(),
            score: r.try_get("score").unwrap_or(0.0),
            details: r
                .try_get("details")
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
            acknowledged_at: r.try_get("acknowledged_at").ok().flatten(),
        })
        .collect();

    Ok(out)
}

/// Fetch a single alert by id.
pub async fn get_alert(pool: &StorePool, id: Uuid) -> Result<Option<AlertRow>, sqlx::Error> {
    use sqlx::Row;

    let row = sqlx::query(
        r#"
        SELECT id, timestamp, agent_id, session_id,
               detector, severity, score, details, acknowledged_at
        FROM anomaly_alerts
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| AlertRow {
        id: r.try_get("id").unwrap_or_else(|_| Uuid::nil()),
        timestamp: r
            .try_get("timestamp")
            .unwrap_or_else(|_| chrono::DateTime::<Utc>::from_timestamp(0, 0).unwrap()),
        agent_id: r.try_get("agent_id").unwrap_or_default(),
        session_id: r.try_get("session_id").ok().flatten(),
        detector: r.try_get("detector").unwrap_or_default(),
        severity: r.try_get("severity").unwrap_or_default(),
        score: r.try_get("score").unwrap_or(0.0),
        details: r
            .try_get("details")
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new())),
        acknowledged_at: r.try_get("acknowledged_at").ok().flatten(),
    }))
}

/// Mark an alert acknowledged (sets `acknowledged_at = now()`).
///
/// Returns `Ok(true)` if a row was updated, `Ok(false)` if no matching row.
pub async fn acknowledge_alert(pool: &StorePool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE anomaly_alerts SET acknowledged_at = NOW() WHERE id = $1 AND acknowledged_at IS NULL",
    )
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alert_filter_default_limit() {
        let f = AlertFilter::new();
        assert_eq!(f.limit, 100);
        assert!(f.agent_id.is_none());
    }

    #[test]
    fn new_alert_marshal() {
        let a = NewAlert {
            id: Uuid::now_v7(),
            timestamp: Utc::now(),
            agent_id: "a-1".into(),
            session_id: None,
            detector: "cost_zscore".into(),
            severity: "warn".into(),
            score: 4.2,
            details: serde_json::json!({"z": 4.2}),
        };
        assert_eq!(a.severity, "warn");
        assert!((a.score - 4.2).abs() < 1e-9);
    }
}
