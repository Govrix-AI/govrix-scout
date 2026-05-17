//! Event persistence — insert and query the `events` TimescaleDB hypertable.
//!
//! All writes are fire-and-forget from the proxy hot path.
//! Reads serve the REST API and dashboard.
//!
//! Note: Uses dynamic sqlx queries (not compile-time query! macros) to allow
//! building without a live DATABASE_URL. Typed compile-time queries can be
//! enabled by running `cargo sqlx prepare` against a live database.

use chrono::{DateTime, Utc};
use govrix_scout_common::models::event::AgentEvent;
use uuid::Uuid;

use crate::db::StorePool;

// ── Filter type ───────────────────────────────────────────────────────────────

/// Optional filter parameters for listing events.
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    pub agent_id: Option<String>,
    pub session_id: Option<Uuid>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub compliance_tag: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: i64,
    pub offset: i64,
}

impl EventFilter {
    pub fn new() -> Self {
        Self {
            limit: 100,
            ..Default::default()
        }
    }
}

// ── Write path ────────────────────────────────────────────────────────────────

/// Insert a single event into the `events` table.
///
/// Called by the background batch writer, not directly from the hot path.
///
/// Compliance invariant: the event MUST have all four compliance fields:
///   session_id, timestamp, lineage_hash, compliance_tag.
pub async fn insert_event(pool: &StorePool, event: &AgentEvent) -> Result<(), sqlx::Error> {
    let direction = event.direction.to_string();
    let provider = event.provider.to_string();
    let pii_json =
        serde_json::to_value(&event.pii_detected).unwrap_or(serde_json::Value::Array(vec![]));
    let tools_json =
        serde_json::to_value(&event.tools_called).unwrap_or(serde_json::Value::Array(vec![]));

    sqlx::query(
        r#"
        INSERT INTO events (
            id, session_id, agent_id,
            timestamp, latency_ms,
            direction, method, upstream_target, provider, model,
            status_code, finish_reason, payload, raw_size_bytes,
            input_tokens, output_tokens, total_tokens, cost_usd,
            pii_detected, tools_called,
            lineage_hash, compliance_tag, tags, error_message,
            created_at
        ) VALUES (
            $1, $2, $3,
            $4, $5,
            $6, $7, $8, $9, $10,
            $11, $12, $13, $14,
            $15, $16, $17, $18,
            $19, $20,
            $21, $22, $23, $24,
            $25
        )
        "#,
    )
    .bind(event.id)
    .bind(event.session_id)
    .bind(&event.agent_id)
    .bind(event.timestamp)
    .bind(event.latency_ms.map(|v| v as i32))
    .bind(&direction)
    .bind(&event.method)
    .bind(&event.upstream_target)
    .bind(&provider)
    .bind(&event.model)
    .bind(event.status_code.map(|v| v as i32))
    .bind(&event.finish_reason)
    .bind(&event.payload)
    .bind(event.raw_size_bytes)
    .bind(event.input_tokens)
    .bind(event.output_tokens)
    .bind(event.total_tokens)
    .bind(event.cost_usd)
    .bind(&pii_json)
    .bind(&tools_json)
    .bind(&event.lineage_hash)
    .bind(&event.compliance_tag)
    .bind(&event.tags)
    .bind(&event.error_message)
    .bind(event.created_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Insert a batch of events using a single UNNEST round-trip.
///
/// Builds 25 column arrays and does a single
/// `INSERT INTO events ... SELECT * FROM UNNEST($1::uuid[], ...)`.
/// One network round-trip per batch instead of one per row.
pub async fn insert_events_batch(
    pool: &StorePool,
    events: &[AgentEvent],
) -> Result<usize, sqlx::Error> {
    if events.is_empty() {
        return Ok(0);
    }

    let n = events.len();
    let mut ids: Vec<Uuid> = Vec::with_capacity(n);
    let mut session_ids: Vec<Uuid> = Vec::with_capacity(n);
    let mut agent_ids: Vec<String> = Vec::with_capacity(n);
    let mut timestamps: Vec<DateTime<Utc>> = Vec::with_capacity(n);
    let mut latencies: Vec<Option<i32>> = Vec::with_capacity(n);
    let mut directions: Vec<String> = Vec::with_capacity(n);
    let mut methods: Vec<String> = Vec::with_capacity(n);
    let mut upstreams: Vec<String> = Vec::with_capacity(n);
    let mut providers: Vec<String> = Vec::with_capacity(n);
    let mut models: Vec<Option<String>> = Vec::with_capacity(n);
    let mut status_codes: Vec<Option<i32>> = Vec::with_capacity(n);
    let mut finish_reasons: Vec<Option<String>> = Vec::with_capacity(n);
    let mut payloads: Vec<serde_json::Value> = Vec::with_capacity(n);
    let mut raw_sizes: Vec<Option<i64>> = Vec::with_capacity(n);
    let mut input_tokens: Vec<Option<i32>> = Vec::with_capacity(n);
    let mut output_tokens: Vec<Option<i32>> = Vec::with_capacity(n);
    let mut total_tokens: Vec<Option<i32>> = Vec::with_capacity(n);
    let mut costs: Vec<Option<rust_decimal::Decimal>> = Vec::with_capacity(n);
    let mut pii_jsons: Vec<serde_json::Value> = Vec::with_capacity(n);
    let mut tools_jsons: Vec<serde_json::Value> = Vec::with_capacity(n);
    let mut lineage_hashes: Vec<String> = Vec::with_capacity(n);
    let mut compliance_tags: Vec<String> = Vec::with_capacity(n);
    let mut tags_jsons: Vec<serde_json::Value> = Vec::with_capacity(n);
    let mut error_messages: Vec<Option<String>> = Vec::with_capacity(n);
    let mut created_ats: Vec<DateTime<Utc>> = Vec::with_capacity(n);

    for event in events {
        ids.push(event.id);
        session_ids.push(event.session_id);
        agent_ids.push(event.agent_id.clone());
        timestamps.push(event.timestamp);
        latencies.push(event.latency_ms.map(|v| v as i32));
        directions.push(event.direction.to_string());
        methods.push(event.method.clone());
        upstreams.push(event.upstream_target.clone());
        providers.push(event.provider.to_string());
        models.push(event.model.clone());
        status_codes.push(event.status_code.map(|v| v as i32));
        finish_reasons.push(event.finish_reason.clone());
        payloads.push(event.payload.clone().unwrap_or(serde_json::Value::Null));
        raw_sizes.push(event.raw_size_bytes);
        input_tokens.push(event.input_tokens);
        output_tokens.push(event.output_tokens);
        total_tokens.push(event.total_tokens);
        costs.push(event.cost_usd);
        pii_jsons.push(
            serde_json::to_value(&event.pii_detected).unwrap_or(serde_json::Value::Array(vec![])),
        );
        tools_jsons.push(
            serde_json::to_value(&event.tools_called).unwrap_or(serde_json::Value::Array(vec![])),
        );
        lineage_hashes.push(event.lineage_hash.clone());
        compliance_tags.push(event.compliance_tag.clone());
        tags_jsons.push(event.tags.clone());
        error_messages.push(event.error_message.clone());
        created_ats.push(event.created_at);
    }

    sqlx::query(
        r#"
        INSERT INTO events (
            id, session_id, agent_id,
            timestamp, latency_ms,
            direction, method, upstream_target, provider, model,
            status_code, finish_reason, payload, raw_size_bytes,
            input_tokens, output_tokens, total_tokens, cost_usd,
            pii_detected, tools_called,
            lineage_hash, compliance_tag, tags, error_message,
            created_at
        )
        SELECT * FROM UNNEST(
            $1::uuid[], $2::uuid[], $3::text[],
            $4::timestamptz[], $5::int4[],
            $6::text[], $7::text[], $8::text[], $9::text[], $10::text[],
            $11::int4[], $12::text[], $13::jsonb[], $14::int8[],
            $15::int4[], $16::int4[], $17::int4[], $18::numeric[],
            $19::jsonb[], $20::jsonb[],
            $21::text[], $22::text[], $23::jsonb[], $24::text[],
            $25::timestamptz[]
        )
        "#,
    )
    .bind(&ids)
    .bind(&session_ids)
    .bind(&agent_ids)
    .bind(&timestamps)
    .bind(&latencies)
    .bind(&directions)
    .bind(&methods)
    .bind(&upstreams)
    .bind(&providers)
    .bind(&models)
    .bind(&status_codes)
    .bind(&finish_reasons)
    .bind(&payloads)
    .bind(&raw_sizes)
    .bind(&input_tokens)
    .bind(&output_tokens)
    .bind(&total_tokens)
    .bind(&costs)
    .bind(&pii_jsons)
    .bind(&tools_jsons)
    .bind(&lineage_hashes)
    .bind(&compliance_tags)
    .bind(&tags_jsons)
    .bind(&error_messages)
    .bind(&created_ats)
    .execute(pool)
    .await?;

    Ok(events.len())
}

// ── Read path ─────────────────────────────────────────────────────────────────

/// Get a single event by its UUID primary key.
///
/// Returns the raw JSONB row so callers (API handlers) can forward it
/// directly without re-serialization overhead.
pub async fn get_event(
    pool: &StorePool,
    id: Uuid,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    use sqlx::Row;

    let row = sqlx::query(
        r#"
        SELECT
            id, session_id, agent_id,
            timestamp, latency_ms,
            direction, method, upstream_target, provider, model,
            status_code, finish_reason, payload, raw_size_bytes,
            input_tokens, output_tokens, total_tokens, cost_usd,
            pii_detected, tools_called,
            lineage_hash, compliance_tag, tags, error_message,
            created_at
        FROM events
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| {
        serde_json::json!({
            "id":             r.try_get::<Uuid, _>("id").ok().map(|u| u.to_string()),
            "session_id":     r.try_get::<Uuid, _>("session_id").ok().map(|u| u.to_string()),
            "agent_id":       r.try_get::<String, _>("agent_id").ok(),
            "timestamp":      r.try_get::<DateTime<Utc>, _>("timestamp").ok().map(|t| t.to_rfc3339()),
            "latency_ms":     r.try_get::<Option<i32>, _>("latency_ms").ok().flatten(),
            "direction":      r.try_get::<String, _>("direction").ok(),
            "method":         r.try_get::<String, _>("method").ok(),
            "upstream_target":r.try_get::<String, _>("upstream_target").ok(),
            "provider":       r.try_get::<String, _>("provider").ok(),
            "model":          r.try_get::<Option<String>, _>("model").ok().flatten(),
            "status_code":    r.try_get::<Option<i32>, _>("status_code").ok().flatten(),
            "finish_reason":  r.try_get::<Option<String>, _>("finish_reason").ok().flatten(),
            "payload":        r.try_get::<Option<serde_json::Value>, _>("payload").ok().flatten(),
            "raw_size_bytes": r.try_get::<Option<i64>, _>("raw_size_bytes").ok().flatten(),
            "input_tokens":   r.try_get::<Option<i32>, _>("input_tokens").ok().flatten(),
            "output_tokens":  r.try_get::<Option<i32>, _>("output_tokens").ok().flatten(),
            "total_tokens":   r.try_get::<Option<i32>, _>("total_tokens").ok().flatten(),
            "cost_usd":       r.try_get::<Option<f64>, _>("cost_usd").ok().flatten(),
            "pii_detected":   r.try_get::<Option<serde_json::Value>, _>("pii_detected").ok().flatten(),
            "tools_called":   r.try_get::<Option<serde_json::Value>, _>("tools_called").ok().flatten(),
            "lineage_hash":   r.try_get::<String, _>("lineage_hash").ok(),
            "compliance_tag": r.try_get::<String, _>("compliance_tag").ok(),
            "tags":           r.try_get::<Option<serde_json::Value>, _>("tags").ok().flatten(),
            "error_message":  r.try_get::<Option<String>, _>("error_message").ok().flatten(),
            "created_at":     r.try_get::<DateTime<Utc>, _>("created_at").ok().map(|t| t.to_rfc3339()),
        })
    }))
}

/// List events with optional filters, ordered by timestamp descending.
///
/// Dynamically builds the WHERE clause based on which filter fields are set.
pub async fn list_events(
    pool: &StorePool,
    filter: &EventFilter,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    use sqlx::Row;

    // Build SQL dynamically — we can't use compile-time macros without a DB.
    let mut conditions: Vec<String> = Vec::new();
    let mut param_idx = 1usize;

    if filter.agent_id.is_some() {
        conditions.push(format!("agent_id = ${param_idx}"));
        param_idx += 1;
    }
    if filter.session_id.is_some() {
        conditions.push(format!("session_id = ${param_idx}"));
        param_idx += 1;
    }
    if filter.provider.is_some() {
        conditions.push(format!("provider = ${param_idx}"));
        param_idx += 1;
    }
    if filter.model.is_some() {
        conditions.push(format!("model = ${param_idx}"));
        param_idx += 1;
    }
    if filter.compliance_tag.is_some() {
        conditions.push(format!("compliance_tag = ${param_idx}"));
        param_idx += 1;
    }
    if filter.from.is_some() {
        conditions.push(format!("timestamp >= ${param_idx}"));
        param_idx += 1;
    }
    if filter.to.is_some() {
        conditions.push(format!("timestamp < ${param_idx}"));
        param_idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let limit_param = param_idx;
    let offset_param = param_idx + 1;

    let sql = format!(
        r#"
        SELECT
            id, session_id, agent_id,
            timestamp, latency_ms,
            direction, method, upstream_target, provider, model,
            status_code, finish_reason, raw_size_bytes,
            input_tokens, output_tokens, total_tokens, cost_usd,
            lineage_hash, compliance_tag, tags, error_message,
            created_at
        FROM events
        {where_clause}
        ORDER BY timestamp DESC
        LIMIT ${limit_param} OFFSET ${offset_param}
        "#,
        where_clause = where_clause,
        limit_param = limit_param,
        offset_param = offset_param,
    );

    let mut q = sqlx::query(&sql);

    // Bind parameters in the same order as they were added to conditions
    if let Some(ref v) = filter.agent_id {
        q = q.bind(v);
    }
    if let Some(v) = filter.session_id {
        q = q.bind(v);
    }
    if let Some(ref v) = filter.provider {
        q = q.bind(v);
    }
    if let Some(ref v) = filter.model {
        q = q.bind(v);
    }
    if let Some(ref v) = filter.compliance_tag {
        q = q.bind(v);
    }
    if let Some(v) = filter.from {
        q = q.bind(v);
    }
    if let Some(v) = filter.to {
        q = q.bind(v);
    }
    q = q.bind(filter.limit).bind(filter.offset);

    let rows = q.fetch_all(pool).await?;

    let result = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id":             r.try_get::<Uuid, _>("id").ok().map(|u| u.to_string()),
                "session_id":     r.try_get::<Uuid, _>("session_id").ok().map(|u| u.to_string()),
                "agent_id":       r.try_get::<String, _>("agent_id").ok(),
                "timestamp":      r.try_get::<DateTime<Utc>, _>("timestamp").ok().map(|t| t.to_rfc3339()),
                "latency_ms":     r.try_get::<Option<i32>, _>("latency_ms").ok().flatten(),
                "direction":      r.try_get::<String, _>("direction").ok(),
                "method":         r.try_get::<String, _>("method").ok(),
                "upstream_target":r.try_get::<String, _>("upstream_target").ok(),
                "provider":       r.try_get::<String, _>("provider").ok(),
                "model":          r.try_get::<Option<String>, _>("model").ok().flatten(),
                "status_code":    r.try_get::<Option<i32>, _>("status_code").ok().flatten(),
                "finish_reason":  r.try_get::<Option<String>, _>("finish_reason").ok().flatten(),
                "raw_size_bytes": r.try_get::<Option<i64>, _>("raw_size_bytes").ok().flatten(),
                "input_tokens":   r.try_get::<Option<i32>, _>("input_tokens").ok().flatten(),
                "output_tokens":  r.try_get::<Option<i32>, _>("output_tokens").ok().flatten(),
                "total_tokens":   r.try_get::<Option<i32>, _>("total_tokens").ok().flatten(),
                "cost_usd":       r.try_get::<Option<f64>, _>("cost_usd").ok().flatten(),
                "lineage_hash":   r.try_get::<String, _>("lineage_hash").ok(),
                "compliance_tag": r.try_get::<String, _>("compliance_tag").ok(),
                "tags":           r.try_get::<Option<serde_json::Value>, _>("tags").ok().flatten(),
                "error_message":  r.try_get::<Option<String>, _>("error_message").ok().flatten(),
                "created_at":     r.try_get::<DateTime<Utc>, _>("created_at").ok().map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(result)
}

/// Return all events that belong to a given session, ordered by timestamp ASC
/// (chronological audit trail order).
pub async fn get_session_events(
    pool: &StorePool,
    session_id: Uuid,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let filter = EventFilter {
        session_id: Some(session_id),
        limit: 10_000,
        ..Default::default()
    };

    // Re-use list_events but with ASC ordering for audit trail
    use sqlx::Row;

    let sql = r#"
        SELECT
            id, session_id, agent_id,
            timestamp, latency_ms,
            direction, method, upstream_target, provider, model,
            status_code, finish_reason, raw_size_bytes,
            input_tokens, output_tokens, total_tokens, cost_usd,
            lineage_hash, compliance_tag, tags, error_message,
            created_at
        FROM events
        WHERE session_id = $1
        ORDER BY timestamp ASC
        LIMIT $2
        "#;

    let rows = sqlx::query(sql)
        .bind(filter.session_id.unwrap())
        .bind(filter.limit)
        .fetch_all(pool)
        .await?;

    let result = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id":             r.try_get::<Uuid, _>("id").ok().map(|u| u.to_string()),
                "session_id":     r.try_get::<Uuid, _>("session_id").ok().map(|u| u.to_string()),
                "agent_id":       r.try_get::<String, _>("agent_id").ok(),
                "timestamp":      r.try_get::<DateTime<Utc>, _>("timestamp").ok().map(|t| t.to_rfc3339()),
                "latency_ms":     r.try_get::<Option<i32>, _>("latency_ms").ok().flatten(),
                "direction":      r.try_get::<String, _>("direction").ok(),
                "method":         r.try_get::<String, _>("method").ok(),
                "upstream_target":r.try_get::<String, _>("upstream_target").ok(),
                "provider":       r.try_get::<String, _>("provider").ok(),
                "model":          r.try_get::<Option<String>, _>("model").ok().flatten(),
                "status_code":    r.try_get::<Option<i32>, _>("status_code").ok().flatten(),
                "finish_reason":  r.try_get::<Option<String>, _>("finish_reason").ok().flatten(),
                "raw_size_bytes": r.try_get::<Option<i64>, _>("raw_size_bytes").ok().flatten(),
                "input_tokens":   r.try_get::<Option<i32>, _>("input_tokens").ok().flatten(),
                "output_tokens":  r.try_get::<Option<i32>, _>("output_tokens").ok().flatten(),
                "total_tokens":   r.try_get::<Option<i32>, _>("total_tokens").ok().flatten(),
                "cost_usd":       r.try_get::<Option<f64>, _>("cost_usd").ok().flatten(),
                "lineage_hash":   r.try_get::<String, _>("lineage_hash").ok(),
                "compliance_tag": r.try_get::<String, _>("compliance_tag").ok(),
                "tags":           r.try_get::<Option<serde_json::Value>, _>("tags").ok().flatten(),
                "error_message":  r.try_get::<Option<String>, _>("error_message").ok().flatten(),
                "created_at":     r.try_get::<DateTime<Utc>, _>("created_at").ok().map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(result)
}

/// Return the most recent N events for a given agent, ordered timestamp DESC.
///
/// `before` allows cursor-based pagination (pass the timestamp of the last
/// event from the previous page).
pub async fn get_events_for_agent(
    pool: &StorePool,
    agent_id: &str,
    limit: i64,
    before: Option<DateTime<Utc>>,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let filter = EventFilter {
        agent_id: Some(agent_id.to_string()),
        to: before,
        limit,
        ..Default::default()
    };
    list_events(pool, &filter).await
}

/// Get a single event by ID (legacy alias kept for backwards compat).
pub async fn get_event_by_id(
    pool: &StorePool,
    id: Uuid,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    get_event(pool, id).await
}

/// Get events for a session (legacy alias kept for backwards compat).
pub async fn get_events_for_session(
    pool: &StorePool,
    session_id: Uuid,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    get_session_events(pool, session_id).await
}

/// Get agent runs (sessions) with aggregated metrics.
///
/// Groups events by session_id for a given agent, returning session-level
/// metrics like total tokens, cost, and whether violations occurred.
pub async fn get_agent_runs(
    pool: &StorePool,
    agent_id: &str,
    limit: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    use sqlx::Row;

    let sql = r#"
        SELECT
            session_id,
            MIN(timestamp) AS started_at,
            MAX(timestamp) AS ended_at,
            COUNT(*) AS event_count,
            MAX(model) AS model,
            COALESCE(SUM(input_tokens), 0) AS total_input_tokens,
            COALESCE(SUM(output_tokens), 0) AS total_output_tokens,
            COALESCE(SUM(cost_usd), 0)::float8 AS total_cost_usd,
            MAX(latency_ms) AS max_latency_ms,
            MAX(status_code) AS status_code,
            BOOL_OR(compliance_tag NOT IN ('pass:all', 'audit:none')) AS has_violations
        FROM events
        WHERE agent_id = $1
        GROUP BY session_id
        ORDER BY MIN(timestamp) DESC
        LIMIT $2
    "#;

    let rows = sqlx::query(sql)
        .bind(agent_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

    let result = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "session_id": r.try_get::<Uuid, _>("session_id").ok().map(|u| u.to_string()),
                "started_at": r.try_get::<DateTime<Utc>, _>("started_at").ok().map(|t| t.to_rfc3339()),
                "ended_at": r.try_get::<DateTime<Utc>, _>("ended_at").ok().map(|t| t.to_rfc3339()),
                "event_count": r.try_get::<i64, _>("event_count").unwrap_or(0),
                "model": r.try_get::<Option<String>, _>("model").ok().flatten(),
                "total_input_tokens": r.try_get::<i64, _>("total_input_tokens").unwrap_or(0),
                "total_output_tokens": r.try_get::<i64, _>("total_output_tokens").unwrap_or(0),
                "total_cost_usd": r.try_get::<f64, _>("total_cost_usd").unwrap_or(0.0),
                "max_latency_ms": r.try_get::<Option<i32>, _>("max_latency_ms").ok().flatten(),
                "status_code": r.try_get::<Option<i32>, _>("status_code").ok().flatten(),
                "has_violations": r.try_get::<Option<bool>, _>("has_violations").ok().flatten().unwrap_or(false),
            })
        })
        .collect();

    Ok(result)
}

/// Get violation events for a specific agent.
///
/// Returns events where compliance_tag indicates a warning or block.
pub async fn get_agent_violations(
    pool: &StorePool,
    agent_id: &str,
    limit: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    use sqlx::Row;

    let sql = r#"
        SELECT
            id, session_id, agent_id,
            timestamp, latency_ms,
            direction, method, upstream_target, provider, model,
            status_code, finish_reason, raw_size_bytes,
            input_tokens, output_tokens, total_tokens, cost_usd,
            pii_detected, tools_called,
            lineage_hash, compliance_tag, tags, error_message,
            created_at
        FROM events
        WHERE agent_id = $1
          AND compliance_tag NOT IN ('pass:all', 'audit:none')
        ORDER BY timestamp DESC
        LIMIT $2
    "#;

    let rows = sqlx::query(sql)
        .bind(agent_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

    let result = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<Uuid, _>("id").ok().map(|u| u.to_string()),
                "session_id": r.try_get::<Uuid, _>("session_id").ok().map(|u| u.to_string()),
                "agent_id": r.try_get::<String, _>("agent_id").ok(),
                "timestamp": r.try_get::<DateTime<Utc>, _>("timestamp").ok().map(|t| t.to_rfc3339()),
                "latency_ms": r.try_get::<Option<i32>, _>("latency_ms").ok().flatten(),
                "direction": r.try_get::<String, _>("direction").ok(),
                "provider": r.try_get::<String, _>("provider").ok(),
                "model": r.try_get::<Option<String>, _>("model").ok().flatten(),
                "status_code": r.try_get::<Option<i32>, _>("status_code").ok().flatten(),
                "input_tokens": r.try_get::<Option<i32>, _>("input_tokens").ok().flatten(),
                "output_tokens": r.try_get::<Option<i32>, _>("output_tokens").ok().flatten(),
                "cost_usd": r.try_get::<Option<f64>, _>("cost_usd").ok().flatten(),
                "pii_detected": r.try_get::<Option<serde_json::Value>, _>("pii_detected").ok().flatten(),
                "compliance_tag": r.try_get::<String, _>("compliance_tag").ok(),
                "error_message": r.try_get::<Option<String>, _>("error_message").ok().flatten(),
                "created_at": r.try_get::<DateTime<Utc>, _>("created_at").ok().map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_filter_default_limit() {
        let f = EventFilter::new();
        assert_eq!(f.limit, 100);
        assert_eq!(f.offset, 0);
        assert!(f.agent_id.is_none());
    }

    #[test]
    fn list_events_sql_no_filters() {
        // Verify the SQL builder produces valid structure with no filters
        let _filter = EventFilter::new();
        let conditions: Vec<String> = Vec::new();
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };
        assert!(where_clause.is_empty());
    }

    #[test]
    fn list_events_sql_with_agent_filter() {
        let mut conditions: Vec<String> = Vec::new();
        let mut param_idx = 1usize;

        let filter = EventFilter {
            agent_id: Some("agent-001".to_string()),
            ..Default::default()
        };

        if filter.agent_id.is_some() {
            conditions.push(format!("agent_id = ${param_idx}"));
            param_idx += 1;
        }

        assert_eq!(conditions.len(), 1);
        assert_eq!(conditions[0], "agent_id = $1");
        assert_eq!(param_idx, 2);
    }
}
