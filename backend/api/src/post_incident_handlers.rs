use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    disaster_recovery_models::{
        ActionItem, CreateActionItemRequest, CreatePostIncidentReportRequest, PostIncidentReport,
        PostIncidentReportRow,
    },
    error::{ApiError, ApiResult},
    state::AppState,
};

pub async fn create_post_incident_report(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreatePostIncidentReportRequest>,
) -> ApiResult<Json<PostIncidentReport>> {
    let report_row = sqlx::query_as::<_, PostIncidentReportRow>(
        r#"
        INSERT INTO post_incident_reports 
        (incident_id, contract_id, title, description, root_cause, impact_assessment, recovery_steps, lessons_learned, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(req.incident_id)
    .bind(Uuid::nil()) // We'll need to get the actual contract ID from the incident
    .bind(&req.title)
    .bind(&req.description)
    .bind(&req.root_cause)
    .bind(&req.impact_assessment)
    .bind(&req.recovery_steps)
    .bind(&req.lessons_learned)
    .bind(&req.created_by)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create post-incident report: {}", e)))?;

    // Create action items associated with this report
    for action_item in req.action_items {
        sqlx::query(
            r#"
            INSERT INTO action_items 
            (report_id, description, owner, due_date, status)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(report_row.id)
        .bind(&action_item.description)
        .bind(&action_item.owner)
        .bind(action_item.due_date)
        .bind("todo") // Default status
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create action item: {}", e)))?;
    }

    // Fetch the action items
    let action_items = sqlx::query_as::<_, ActionItem>(
        "SELECT * FROM action_items WHERE report_id = $1 ORDER BY created_at ASC",
    )
    .bind(report_row.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to fetch action items: {}", e)))?;

    Ok(Json(report_row.into_report(action_items)))
}

pub async fn get_post_incident_report(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
) -> ApiResult<Json<PostIncidentReport>> {
    let report_row = sqlx::query_as::<_, PostIncidentReportRow>(
        "SELECT * FROM post_incident_reports WHERE id = $1",
    )
    .bind(report_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("post_incident_report", "Post-incident report not found"))?;

    let action_items = sqlx::query_as::<_, ActionItem>(
        "SELECT * FROM action_items WHERE report_id = $1 ORDER BY created_at ASC",
    )
    .bind(report_row.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to fetch action items: {}", e)))?;

    Ok(Json(report_row.into_report(action_items)))
}

pub async fn get_contract_post_incident_reports(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<Vec<PostIncidentReport>>> {
    let report_rows = sqlx::query_as::<_, PostIncidentReportRow>(
        "SELECT * FROM post_incident_reports WHERE contract_id = $1 ORDER BY created_at DESC",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // For each report, fetch action items (could be optimized with a JOIN in production)
    let mut reports = Vec::with_capacity(report_rows.len());
    for row in report_rows {
        let action_items = sqlx::query_as::<_, ActionItem>(
            "SELECT * FROM action_items WHERE report_id = $1 ORDER BY created_at ASC",
        )
        .bind(row.id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to fetch action items: {}", e)))?;
        reports.push(row.into_report(action_items));
    }

    Ok(Json(reports))
}

pub async fn update_action_item_status(
    State(state): State<AppState>,
    Path((action_item_id, status)): Path<(Uuid, String)>,
) -> ApiResult<StatusCode> {
    let valid_statuses = ["todo", "in_progress", "completed"];
    if !valid_statuses.contains(&status.as_str()) {
        return Err(ApiError::bad_request(
            "invalid_status",
            "Status must be one of: todo, in_progress, completed",
        ));
    }

    sqlx::query("UPDATE action_items SET status = $1, updated_at = NOW() WHERE id = $2")
        .bind(&status)
        .bind(action_item_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update action item: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_action_items_for_report(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
) -> ApiResult<Json<Vec<ActionItem>>> {
    let action_items = sqlx::query_as::<_, ActionItem>(
        "SELECT * FROM action_items WHERE report_id = $1 ORDER BY created_at ASC",
    )
    .bind(report_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(action_items))
}

pub async fn create_action_item(
    State(state): State<AppState>,
    Path(report_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateActionItemRequest>,
) -> ApiResult<Json<ActionItem>> {
    let action_item = sqlx::query_as::<_, ActionItem>(
        r#"
        INSERT INTO action_items 
        (report_id, description, owner, due_date, status)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(report_id)
    .bind(&req.description)
    .bind(&req.owner)
    .bind(req.due_date)
    .bind("todo") // Default status
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create action item: {}", e)))?;

    Ok(Json(action_item))
}
