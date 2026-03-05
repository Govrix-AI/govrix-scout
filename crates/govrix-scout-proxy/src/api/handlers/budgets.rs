//! Budget configuration handlers.
//!
//! Route map:
//!   GET    /api/v1/agents/{id}/budget   -- get_agent_budget
//!   PUT    /api/v1/agents/{id}/budget   -- set_agent_budget
//!   DELETE /api/v1/agents/{id}/budget   -- delete_agent_budget
//!   GET    /api/v1/budgets/overview     -- budget_overview

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::api::state::AppState;

// ── Request types ─────────────────────────────────────────────────────────────

/// Body for PUT /api/v1/agents/{id}/budget
#[derive(Debug, Deserialize)]
pub struct SetBudgetBody {
    pub daily_token_limit: Option<i64>,
    pub daily_cost_limit_usd: Option<f64>,
    pub monthly_cost_limit_usd: Option<f64>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Get budget config + today's usage for an agent.
///
/// GET /api/v1/agents/{id}/budget
pub async fn get_agent_budget(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let config = govrix_scout_store::budget::get_budget_config(&state.pool, &id).await;
    let usage = govrix_scout_store::budget::get_budget_today(&state.pool, &id).await;

    match (config, usage) {
        (Ok(config), Ok(usage_opt)) => {
            let (tokens_used, cost_used) = usage_opt.unwrap_or((0, 0.0));
            (
                StatusCode::OK,
                Json(json!({
                    "data": {
                        "agent_id": id,
                        "config": config,
                        "usage": {
                            "tokens_used_today": tokens_used,
                            "cost_used_today": cost_used,
                        },
                    }
                })),
            )
        }
        (Err(e), _) | (_, Err(e)) => {
            tracing::error!("get_agent_budget store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch budget", "detail": e.to_string() })),
            )
        }
    }
}

/// Set or update budget limits for an agent.
///
/// PUT /api/v1/agents/{id}/budget
pub async fn set_agent_budget(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<SetBudgetBody>,
) -> impl IntoResponse {
    match govrix_scout_store::budget::upsert_budget_config(
        &state.pool,
        &id,
        body.daily_token_limit,
        body.daily_cost_limit_usd,
        body.monthly_cost_limit_usd,
    )
    .await
    {
        Ok(()) => (
            StatusCode::OK,
            Json(json!({
                "data": {
                    "agent_id": id,
                    "daily_token_limit": body.daily_token_limit,
                    "daily_cost_limit_usd": body.daily_cost_limit_usd,
                    "monthly_cost_limit_usd": body.monthly_cost_limit_usd,
                },
                "message": "budget updated",
            })),
        ),
        Err(e) => {
            tracing::error!("set_agent_budget store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to set budget", "detail": e.to_string() })),
            )
        }
    }
}

/// Delete budget config for an agent.
///
/// DELETE /api/v1/agents/{id}/budget
pub async fn delete_agent_budget(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match govrix_scout_store::budget::delete_budget_config(&state.pool, &id).await {
        Ok(true) => (
            StatusCode::OK,
            Json(json!({ "data": { "agent_id": id }, "message": "budget config deleted" })),
        ),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "no budget config found", "agent_id": id })),
        ),
        Err(e) => {
            tracing::error!("delete_agent_budget store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to delete budget", "detail": e.to_string() })),
            )
        }
    }
}

/// Get budget overview for all agents.
///
/// GET /api/v1/budgets/overview
pub async fn budget_overview(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match govrix_scout_store::budget::get_budget_overview(&state.pool).await {
        Ok(rows) => {
            let total = rows.len();
            (
                StatusCode::OK,
                Json(json!({
                    "data": rows,
                    "total": total,
                })),
            )
        }
        Err(e) => {
            tracing::error!("budget_overview store error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    json!({ "error": "failed to fetch budget overview", "detail": e.to_string() }),
                ),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_budget_body_deserializes() {
        let json_str = r#"{"daily_token_limit": 100000, "daily_cost_limit_usd": 5.50}"#;
        let body: SetBudgetBody = serde_json::from_str(json_str).unwrap();
        assert_eq!(body.daily_token_limit, Some(100000));
        assert!((body.daily_cost_limit_usd.unwrap() - 5.50).abs() < 1e-9);
        assert!(body.monthly_cost_limit_usd.is_none());
    }

    #[test]
    fn set_budget_body_all_null() {
        let json_str = r#"{}"#;
        let body: SetBudgetBody = serde_json::from_str(json_str).unwrap();
        assert!(body.daily_token_limit.is_none());
        assert!(body.daily_cost_limit_usd.is_none());
        assert!(body.monthly_cost_limit_usd.is_none());
    }
}
