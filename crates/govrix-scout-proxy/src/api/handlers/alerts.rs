//! Anomaly alert API handlers.
//!
//! Route map:
//!   GET  /api/v1/alerts                       — list_alerts
//!   GET  /api/v1/alerts/{id}                  — get_alert
//!   POST /api/v1/alerts/{id}/acknowledge      — acknowledge_alert
//!   GET  /api/v1/alerts/stream                — SSE live feed

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use chrono::{DateTime, Utc};
use futures_util::stream::Stream;
use serde::Deserialize;
use serde_json::json;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use uuid::Uuid;

use govrix_scout_store::AlertFilter;

use crate::api::state::AppState;

// ── Request types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct ListAlertsParams {
    pub agent_id: Option<String>,
    pub severity: Option<String>,
    pub detector: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET /api/v1/alerts
pub async fn list_alerts(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListAlertsParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(100).clamp(1, 1000);
    let filter = AlertFilter {
        agent_id: params.agent_id,
        severity: params.severity,
        detector: params.detector,
        since: params.since,
        until: params.until,
        limit,
    };

    match govrix_scout_store::list_alerts(&state.pool, filter).await {
        Ok(rows) => {
            let total = rows.len();
            (
                StatusCode::OK,
                Json(json!({
                    "data": rows,
                    "total": total,
                    "limit": limit,
                })),
            )
        }
        Err(e) => {
            tracing::error!("list_alerts store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to query alerts", "detail": e.to_string() })),
            )
        }
    }
}

/// GET /api/v1/alerts/{id}
pub async fn get_alert(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid alert id — must be a UUID" })),
            );
        }
    };

    match govrix_scout_store::get_alert(&state.pool, uuid).await {
        Ok(Some(row)) => (StatusCode::OK, Json(json!({ "data": row }))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "alert not found", "id": id })),
        ),
        Err(e) => {
            tracing::error!("get_alert store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch alert", "detail": e.to_string() })),
            )
        }
    }
}

/// POST /api/v1/alerts/{id}/acknowledge
pub async fn acknowledge_alert(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid alert id — must be a UUID" })),
            );
        }
    };

    match govrix_scout_store::acknowledge_alert(&state.pool, uuid).await {
        Ok(true) => (
            StatusCode::OK,
            Json(json!({ "data": { "id": id, "acknowledged": true } })),
        ),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "alert not found or already acknowledged", "id": id })),
        ),
        Err(e) => {
            tracing::error!("acknowledge_alert store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to acknowledge alert", "detail": e.to_string() })),
            )
        }
    }
}

/// GET /api/v1/alerts/stream — Server-Sent Events live feed.
///
/// Each subscriber gets its own broadcast `Receiver`. If no broadcast sender
/// was configured (no anomaly engine wired in), the stream simply emits keep-
/// alives forever — clients can still connect without errors.
pub async fn stream(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx_opt = state.alert_tx.as_ref().map(|tx| tx.subscribe());

    let stream = async_stream::stream_for_alerts(rx_opt);

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}

// ── SSE plumbing ─────────────────────────────────────────────────────────────

mod async_stream {
    use super::*;
    use crate::anomaly::AnomalyAlert;
    use tokio::sync::broadcast::Receiver;

    pub fn stream_for_alerts(
        rx_opt: Option<Receiver<AnomalyAlert>>,
    ) -> impl Stream<Item = Result<Event, Infallible>> {
        match rx_opt {
            Some(rx) => {
                let s = BroadcastStream::new(rx).filter_map(|res| match res {
                    Ok(alert) => {
                        let payload = serde_json::json!({
                            "id":        alert.id,
                            "timestamp": alert.timestamp.to_rfc3339(),
                            "agent_id":  alert.agent_id,
                            "session_id": alert.session_id,
                            "detector":  alert.detector,
                            "severity":  alert.severity.to_string(),
                            "score":     alert.score,
                            "details":   alert.details,
                        });
                        Some(Ok(Event::default()
                            .event("alert")
                            .data(payload.to_string())))
                    }
                    Err(_) => None,
                });
                Box::pin(s) as std::pin::Pin<Box<dyn Stream<Item = _> + Send>>
            }
            None => {
                let s = tokio_stream::pending::<Result<Event, Infallible>>();
                Box::pin(s) as std::pin::Pin<Box<dyn Stream<Item = _> + Send>>
            }
        }
    }
}
