//! Projects store — CRUD for the `projects` table and agent-project assignments.

use crate::db::StorePool;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Create a new project.
pub async fn create_project(
    pool: &StorePool,
    name: &str,
    description: Option<&str>,
) -> Result<serde_json::Value, sqlx::Error> {
    use sqlx::Row;

    let row = sqlx::query(
        r#"
        INSERT INTO projects (name, description)
        VALUES ($1, $2)
        RETURNING id, name, description, created_at, updated_at
        "#,
    )
    .bind(name)
    .bind(description)
    .fetch_one(pool)
    .await?;

    Ok(serde_json::json!({
        "id": row.try_get::<Uuid, _>("id").ok().map(|u| u.to_string()),
        "name": row.try_get::<String, _>("name").ok(),
        "description": row.try_get::<Option<String>, _>("description").ok().flatten(),
        "created_at": row.try_get::<DateTime<Utc>, _>("created_at").ok().map(|t| t.to_rfc3339()),
        "updated_at": row.try_get::<DateTime<Utc>, _>("updated_at").ok().map(|t| t.to_rfc3339()),
    }))
}

/// Get a single project by ID.
pub async fn get_project(
    pool: &StorePool,
    id: Uuid,
) -> Result<Option<serde_json::Value>, sqlx::Error> {
    use sqlx::Row;

    let row = sqlx::query(
        r#"
        SELECT p.id, p.name, p.description, p.created_at, p.updated_at,
               COUNT(a.id) AS agent_count,
               COALESCE(SUM(a.total_cost_usd), 0)::float8 AS total_cost_usd
        FROM projects p
        LEFT JOIN agents a ON a.project_id = p.id
        WHERE p.id = $1
        GROUP BY p.id
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| {
        serde_json::json!({
            "id": r.try_get::<Uuid, _>("id").ok().map(|u| u.to_string()),
            "name": r.try_get::<String, _>("name").ok(),
            "description": r.try_get::<Option<String>, _>("description").ok().flatten(),
            "agent_count": r.try_get::<i64, _>("agent_count").unwrap_or(0),
            "total_cost_usd": r.try_get::<f64, _>("total_cost_usd").unwrap_or(0.0),
            "created_at": r.try_get::<DateTime<Utc>, _>("created_at").ok().map(|t| t.to_rfc3339()),
            "updated_at": r.try_get::<DateTime<Utc>, _>("updated_at").ok().map(|t| t.to_rfc3339()),
        })
    }))
}

/// List all projects with agent count and total cost.
pub async fn list_projects(
    pool: &StorePool,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        r#"
        SELECT p.id, p.name, p.description, p.created_at, p.updated_at,
               COUNT(a.id) AS agent_count,
               COALESCE(SUM(a.total_cost_usd), 0)::float8 AS total_cost_usd
        FROM projects p
        LEFT JOIN agents a ON a.project_id = p.id
        GROUP BY p.id
        ORDER BY p.name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<Uuid, _>("id").ok().map(|u| u.to_string()),
                "name": r.try_get::<String, _>("name").ok(),
                "description": r.try_get::<Option<String>, _>("description").ok().flatten(),
                "agent_count": r.try_get::<i64, _>("agent_count").unwrap_or(0),
                "total_cost_usd": r.try_get::<f64, _>("total_cost_usd").unwrap_or(0.0),
                "created_at": r.try_get::<DateTime<Utc>, _>("created_at").ok().map(|t| t.to_rfc3339()),
                "updated_at": r.try_get::<DateTime<Utc>, _>("updated_at").ok().map(|t| t.to_rfc3339()),
            })
        })
        .collect())
}

/// Update a project's name and/or description.
pub async fn update_project(
    pool: &StorePool,
    id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let mut sets = Vec::new();
    let mut param_idx = 2usize; // $1 is id

    if name.is_some() {
        sets.push(format!("name = ${param_idx}"));
        param_idx += 1;
    }
    if description.is_some() {
        sets.push(format!("description = ${param_idx}"));
        param_idx += 1;
    }

    if sets.is_empty() {
        return Ok(false);
    }

    sets.push("updated_at = now()".to_string());
    let _ = param_idx; // suppress warning

    let sql = format!("UPDATE projects SET {} WHERE id = $1", sets.join(", "));
    let mut q = sqlx::query(&sql);
    q = q.bind(id);
    if let Some(n) = name {
        q = q.bind(n);
    }
    if let Some(d) = description {
        q = q.bind(d);
    }

    let result = q.execute(pool).await?;
    Ok(result.rows_affected() > 0)
}

/// Delete a project. Returns false if project has assigned agents.
pub async fn delete_project(
    pool: &StorePool,
    id: Uuid,
) -> Result<Result<bool, String>, sqlx::Error> {
    use sqlx::Row;

    // Check for assigned agents
    let count_row = sqlx::query("SELECT COUNT(*) AS cnt FROM agents WHERE project_id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;
    let agent_count: i64 = count_row.try_get("cnt").unwrap_or(0);

    if agent_count > 0 {
        return Ok(Err(format!(
            "cannot delete project: {} agent(s) still assigned",
            agent_count
        )));
    }

    let result = sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(Ok(result.rows_affected() > 0))
}

/// Assign an agent to a project (or unassign by passing None).
pub async fn assign_agent_to_project(
    pool: &StorePool,
    agent_id: &str,
    project_id: Option<Uuid>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("UPDATE agents SET project_id = $1 WHERE id = $2")
        .bind(project_id)
        .bind(agent_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// List agents in a project.
pub async fn list_project_agents(
    pool: &StorePool,
    project_id: Uuid,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        r#"
        SELECT id, name, status, agent_type, first_seen_at, last_seen_at,
               total_requests, total_tokens_in, total_tokens_out,
               total_cost_usd::float8 AS total_cost_usd,
               last_model_used
        FROM agents
        WHERE project_id = $1
        ORDER BY last_seen_at DESC
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.try_get::<String, _>("id").ok(),
                "name": r.try_get::<Option<String>, _>("name").ok().flatten(),
                "status": r.try_get::<String, _>("status").ok(),
                "agent_type": r.try_get::<String, _>("agent_type").ok(),
                "first_seen_at": r.try_get::<DateTime<Utc>, _>("first_seen_at").ok().map(|t| t.to_rfc3339()),
                "last_seen_at": r.try_get::<DateTime<Utc>, _>("last_seen_at").ok().map(|t| t.to_rfc3339()),
                "total_requests": r.try_get::<i64, _>("total_requests").unwrap_or(0),
                "total_tokens_in": r.try_get::<i64, _>("total_tokens_in").unwrap_or(0),
                "total_tokens_out": r.try_get::<i64, _>("total_tokens_out").unwrap_or(0),
                "total_cost_usd": r.try_get::<f64, _>("total_cost_usd").unwrap_or(0.0),
                "last_model_used": r.try_get::<Option<String>, _>("last_model_used").ok().flatten(),
            })
        })
        .collect())
}

/// Get cost summary for a project over a time range.
pub async fn get_project_cost_summary(
    pool: &StorePool,
    project_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<serde_json::Value, sqlx::Error> {
    use sqlx::Row;

    let row = sqlx::query(
        r#"
        SELECT
            COUNT(*) AS total_requests,
            COALESCE(SUM(e.input_tokens), 0) AS total_input_tokens,
            COALESCE(SUM(e.output_tokens), 0) AS total_output_tokens,
            COALESCE(SUM(e.cost_usd), 0.0)::float8 AS total_cost_usd
        FROM events e
        INNER JOIN agents a ON e.agent_id = a.id
        WHERE a.project_id = $1
          AND e.timestamp >= $2
          AND e.timestamp < $3
        "#,
    )
    .bind(project_id)
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;

    Ok(serde_json::json!({
        "project_id": project_id.to_string(),
        "from": from.to_rfc3339(),
        "to": to.to_rfc3339(),
        "total_requests": row.try_get::<i64, _>("total_requests").unwrap_or(0),
        "total_input_tokens": row.try_get::<i64, _>("total_input_tokens").unwrap_or(0),
        "total_output_tokens": row.try_get::<i64, _>("total_output_tokens").unwrap_or(0),
        "total_cost_usd": row.try_get::<f64, _>("total_cost_usd").unwrap_or(0.0),
    }))
}
