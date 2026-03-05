//! Project management handlers.
//!
//! Route map:
//!   GET    /api/v1/projects              — list_projects
//!   POST   /api/v1/projects              — create_project
//!   GET    /api/v1/projects/{id}         — get_project
//!   PUT    /api/v1/projects/{id}         — update_project
//!   DELETE /api/v1/projects/{id}         — delete_project
//!   GET    /api/v1/projects/{id}/agents  — list_project_agents
//!   GET    /api/v1/projects/{id}/costs   — project_costs
//!   PUT    /api/v1/agents/{id}/project   — assign_agent_project

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::api::state::AppState;

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateProjectBody {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectBody {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignProjectBody {
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectCostsParams {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// List all projects with agent count and total cost.
///
/// GET /api/v1/projects
pub async fn list_projects(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match govrix_scout_store::projects::list_projects(&state.pool).await {
        Ok(projects) => {
            let total = projects.len();
            (
                StatusCode::OK,
                Json(json!({ "data": projects, "total": total })),
            )
        }
        Err(e) => {
            tracing::error!("list_projects error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to list projects", "detail": e.to_string() })),
            )
        }
    }
}

/// Create a new project.
///
/// POST /api/v1/projects
pub async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateProjectBody>,
) -> impl IntoResponse {
    match govrix_scout_store::projects::create_project(
        &state.pool,
        &body.name,
        body.description.as_deref(),
    )
    .await
    {
        Ok(project) => (StatusCode::CREATED, Json(json!({ "data": project }))),
        Err(e) => {
            tracing::error!("create_project error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to create project", "detail": e.to_string() })),
            )
        }
    }
}

/// Get a single project by ID.
///
/// GET /api/v1/projects/{id}
pub async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid project id" })),
            )
        }
    };

    match govrix_scout_store::projects::get_project(&state.pool, uuid).await {
        Ok(Some(project)) => (StatusCode::OK, Json(json!({ "data": project }))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "project not found" })),
        ),
        Err(e) => {
            tracing::error!("get_project error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch project", "detail": e.to_string() })),
            )
        }
    }
}

/// Update a project.
///
/// PUT /api/v1/projects/{id}
pub async fn update_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<UpdateProjectBody>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid project id" })),
            )
        }
    };

    match govrix_scout_store::projects::update_project(
        &state.pool,
        uuid,
        body.name.as_deref(),
        body.description.as_deref(),
    )
    .await
    {
        Ok(true) => match govrix_scout_store::projects::get_project(&state.pool, uuid).await {
            Ok(Some(project)) => (StatusCode::OK, Json(json!({ "data": project }))),
            _ => (
                StatusCode::OK,
                Json(json!({ "data": { "id": id }, "updated": true })),
            ),
        },
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "project not found or no changes" })),
        ),
        Err(e) => {
            tracing::error!("update_project error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to update project", "detail": e.to_string() })),
            )
        }
    }
}

/// Delete a project (only if no agents assigned).
///
/// DELETE /api/v1/projects/{id}
pub async fn delete_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid project id" })),
            )
        }
    };

    match govrix_scout_store::projects::delete_project(&state.pool, uuid).await {
        Ok(Ok(true)) => (
            StatusCode::OK,
            Json(json!({ "data": { "id": id }, "message": "project deleted" })),
        ),
        Ok(Ok(false)) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "project not found" })),
        ),
        Ok(Err(reason)) => (StatusCode::CONFLICT, Json(json!({ "error": reason }))),
        Err(e) => {
            tracing::error!("delete_project error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to delete project", "detail": e.to_string() })),
            )
        }
    }
}

/// List agents in a project.
///
/// GET /api/v1/projects/{id}/agents
pub async fn list_project_agents(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid project id" })),
            )
        }
    };

    match govrix_scout_store::projects::list_project_agents(&state.pool, uuid).await {
        Ok(agents) => {
            let total = agents.len();
            (
                StatusCode::OK,
                Json(json!({ "data": agents, "total": total })),
            )
        }
        Err(e) => {
            tracing::error!("list_project_agents error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to list agents", "detail": e.to_string() })),
            )
        }
    }
}

/// Get project cost summary.
///
/// GET /api/v1/projects/{id}/costs
pub async fn project_costs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<ProjectCostsParams>,
) -> impl IntoResponse {
    let uuid = match Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid project id" })),
            )
        }
    };

    let now = Utc::now();
    let to = params.to.unwrap_or(now);
    let from = params.from.unwrap_or_else(|| to - Duration::days(30));

    match govrix_scout_store::projects::get_project_cost_summary(&state.pool, uuid, from, to).await
    {
        Ok(summary) => (StatusCode::OK, Json(json!({ "data": summary }))),
        Err(e) => {
            tracing::error!("project_costs error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to fetch costs", "detail": e.to_string() })),
            )
        }
    }
}

/// Assign agent to project (or unassign).
///
/// PUT /api/v1/agents/{id}/project
pub async fn assign_agent_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<AssignProjectBody>,
) -> impl IntoResponse {
    let project_uuid = match &body.project_id {
        Some(pid) => match Uuid::parse_str(pid) {
            Ok(u) => Some(u),
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "invalid project_id" })),
                )
            }
        },
        None => None,
    };

    match govrix_scout_store::projects::assign_agent_to_project(&state.pool, &id, project_uuid)
        .await
    {
        Ok(true) => (
            StatusCode::OK,
            Json(
                json!({ "data": { "agent_id": id, "project_id": body.project_id }, "message": "agent assigned" }),
            ),
        ),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "agent not found" })),
        ),
        Err(e) => {
            tracing::error!("assign_agent_project error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "failed to assign agent", "detail": e.to_string() })),
            )
        }
    }
}
