use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{json, Value};
use shared::models::{
    AdvanceCanaryRequest, CanaryMetric, CanaryRelease, CreateCanaryRequest,
    RecordCanaryMetricRequest,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

// ───────────────────── Query params ─────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct ListCanaryQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub status: Option<String>,
}

fn default_limit() -> i64 {
    20
}

// ───────────────────── Handlers ─────────────────────

/// POST /api/contracts/:id/canary — create a new canary release
pub async fn create_canary(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    ValidatedJson(req): ValidatedJson<CreateCanaryRequest>,
) -> ApiResult<impl IntoResponse> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;
    let to_deployment_uuid = parse_uuid(&req.to_deployment_id, "to_deployment")?;
    let threshold = req.error_rate_threshold.unwrap_or(5.0);

    // Ensure no other active/pending canary for this contract
    let existing: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM canary_releases WHERE contract_id = $1 AND status IN ('pending', 'active')",
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| db_err("check existing canary", e))?;

    if existing.is_some() {
        return Err(ApiError::conflict(
            "CanaryAlreadyActive",
            "An active or pending canary release already exists for this contract",
        ));
    }

    let release: CanaryRelease = sqlx::query_as(
        r#"
        INSERT INTO canary_releases (contract_id, to_deployment_id, error_rate_threshold, created_by)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(contract_uuid)
    .bind(to_deployment_uuid)
    .bind(rust_decimal::Decimal::try_from(threshold).unwrap_or_default())
    .bind(req.created_by.as_deref())
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_err("create canary release", e))?;

    Ok((StatusCode::CREATED, Json(release)))
}

/// GET /api/contracts/:id/canary — list canary releases for a contract
pub async fn list_canaries(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<ListCanaryQuery>,
) -> ApiResult<Json<Value>> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    let (releases, total): (Vec<CanaryRelease>, i64) = if let Some(ref status) = params.status {
        let items: Vec<CanaryRelease> = sqlx::query_as(
            "SELECT * FROM canary_releases WHERE contract_id = $1 AND status::text = $2 ORDER BY started_at DESC LIMIT $3 OFFSET $4",
        )
        .bind(contract_uuid)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_err("list canaries", e))?;

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM canary_releases WHERE contract_id = $1 AND status::text = $2",
        )
        .bind(contract_uuid)
        .bind(status)
        .fetch_one(&state.db)
        .await
        .map_err(|e| db_err("count canaries", e))?;

        (items, count)
    } else {
        let items: Vec<CanaryRelease> = sqlx::query_as(
            "SELECT * FROM canary_releases WHERE contract_id = $1 ORDER BY started_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(contract_uuid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_err("list canaries", e))?;

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM canary_releases WHERE contract_id = $1")
                .bind(contract_uuid)
                .fetch_one(&state.db)
                .await
                .map_err(|e| db_err("count canaries", e))?;

        (items, count)
    };

    Ok(Json(json!({
        "items": releases,
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}

/// GET /api/canary/:canary_id — get a specific canary release
pub async fn get_canary(
    State(state): State<AppState>,
    Path(canary_id): Path<String>,
) -> ApiResult<Json<CanaryRelease>> {
    let canary_uuid = parse_uuid(&canary_id, "canary")?;

    let release: CanaryRelease = sqlx::query_as("SELECT * FROM canary_releases WHERE id = $1")
        .bind(canary_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "CanaryNotFound",
                format!("No canary release found with ID: {}", canary_id),
            ),
            _ => db_err("get canary", e),
        })?;

    Ok(Json(release))
}

/// POST /api/canary/:canary_id/advance — advance canary to next stage
pub async fn advance_canary(
    State(state): State<AppState>,
    Path(canary_id): Path<String>,
    ValidatedJson(req): ValidatedJson<AdvanceCanaryRequest>,
) -> ApiResult<Json<CanaryRelease>> {
    let canary_uuid = parse_uuid(&canary_id, "canary")?;

    let current: CanaryRelease = sqlx::query_as("SELECT * FROM canary_releases WHERE id = $1")
        .bind(canary_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "CanaryNotFound",
                format!("No canary release found with ID: {}", canary_id),
            ),
            _ => db_err("fetch canary for advance", e),
        })?;

    // Only active or pending canaries can be advanced
    let status_str = serde_json::to_value(&current.status)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    if status_str != "\"pending\"" && status_str != "\"active\"" {
        // Check via pattern matching on the enum instead
    }

    let (next_stage, next_percentage) = advance_stage(&current, req.target_percentage);

    let updated: CanaryRelease = sqlx::query_as(
        r#"
        UPDATE canary_releases
        SET status = 'active',
            current_stage = $2,
            current_percentage = $3
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(canary_uuid)
    .bind(next_stage)
    .bind(next_percentage)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_err("advance canary", e))?;

    // Record stage transition
    let _ = sqlx::query(
        r#"
        INSERT INTO canary_stage_history
            (canary_id, from_stage, to_stage, from_percentage, to_percentage, transitioned_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(canary_uuid)
    .bind(current.current_stage)
    .bind(&updated.current_stage)
    .bind(current.current_percentage)
    .bind(next_percentage)
    .bind(req.advanced_by.as_deref())
    .execute(&state.db)
    .await;

    Ok(Json(updated))
}

/// POST /api/canary/:canary_id/rollback — rollback a canary release
pub async fn rollback_canary(
    State(state): State<AppState>,
    Path(canary_id): Path<String>,
) -> ApiResult<Json<CanaryRelease>> {
    let canary_uuid = parse_uuid(&canary_id, "canary")?;

    let release: CanaryRelease = sqlx::query_as(
        r#"
        UPDATE canary_releases
        SET status = 'rolled_back', completed_at = NOW()
        WHERE id = $1 AND status IN ('pending', 'active', 'paused')
        RETURNING *
        "#,
    )
    .bind(canary_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::not_found(
            "CanaryNotFound",
            "No active canary release found to rollback",
        ),
        _ => db_err("rollback canary", e),
    })?;

    Ok(Json(release))
}

/// POST /api/canary/:canary_id/complete — complete a canary release
pub async fn complete_canary(
    State(state): State<AppState>,
    Path(canary_id): Path<String>,
) -> ApiResult<Json<CanaryRelease>> {
    let canary_uuid = parse_uuid(&canary_id, "canary")?;

    let release: CanaryRelease = sqlx::query_as(
        r#"
        UPDATE canary_releases
        SET status = 'completed', completed_at = NOW(), current_percentage = target_percentage
        WHERE id = $1 AND status = 'active'
        RETURNING *
        "#,
    )
    .bind(canary_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::not_found(
            "CanaryNotFound",
            "No active canary release found to complete",
        ),
        _ => db_err("complete canary", e),
    })?;

    Ok(Json(release))
}

/// POST /api/canary/:canary_id/metrics — record canary metrics
pub async fn record_canary_metric(
    State(state): State<AppState>,
    Path(canary_id): Path<String>,
    ValidatedJson(req): ValidatedJson<RecordCanaryMetricRequest>,
) -> ApiResult<impl IntoResponse> {
    let canary_uuid = parse_uuid(&canary_id, "canary")?;
    let error_rate = if req.requests > 0 {
        (req.errors as f64 / req.requests as f64) * 100.0
    } else {
        0.0
    };

    let metric: CanaryMetric = sqlx::query_as(
        r#"
        INSERT INTO canary_metrics
            (canary_id, requests, errors, error_rate, avg_response_time_ms, p95_response_time_ms, p99_response_time_ms)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(canary_uuid)
    .bind(req.requests)
    .bind(req.errors)
    .bind(rust_decimal::Decimal::try_from(error_rate).unwrap_or_default())
    .bind(req.avg_response_time_ms.map(|v| rust_decimal::Decimal::try_from(v).unwrap_or_default()))
    .bind(req.p95_response_time_ms.map(|v| rust_decimal::Decimal::try_from(v).unwrap_or_default()))
    .bind(req.p99_response_time_ms.map(|v| rust_decimal::Decimal::try_from(v).unwrap_or_default()))
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_err("record canary metric", e))?;

    // Update aggregate counts on the canary release
    let _ = sqlx::query(
        r#"
        UPDATE canary_releases
        SET total_requests = total_requests + $2,
            error_count = error_count + $3,
            current_error_rate = CASE
                WHEN (total_requests + $2) > 0
                THEN ((error_count + $3)::DECIMAL / (total_requests + $2)::DECIMAL) * 100.0
                ELSE 0.0
            END
        WHERE id = $1
        "#,
    )
    .bind(canary_uuid)
    .bind(req.requests)
    .bind(req.errors)
    .execute(&state.db)
    .await;

    Ok((StatusCode::CREATED, Json(metric)))
}

/// GET /api/canary/:canary_id/metrics — list canary metrics
pub async fn list_canary_metrics(
    State(state): State<AppState>,
    Path(canary_id): Path<String>,
    Query(params): Query<ListCanaryQuery>,
) -> ApiResult<Json<Value>> {
    let canary_uuid = parse_uuid(&canary_id, "canary")?;
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    let metrics: Vec<CanaryMetric> = sqlx::query_as(
        "SELECT * FROM canary_metrics WHERE canary_id = $1 ORDER BY timestamp DESC LIMIT $2 OFFSET $3",
    )
    .bind(canary_uuid)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("list canary metrics", e))?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM canary_metrics WHERE canary_id = $1")
        .bind(canary_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|e| db_err("count canary metrics", e))?;

    Ok(Json(json!({
        "items": metrics,
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}

// ───────────────────── Helpers ─────────────────────

fn parse_uuid(id: &str, label: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(id).map_err(|_| {
        ApiError::bad_request_with("InvalidId", format!("Invalid {} ID format: {}", label, id))
    })
}

fn db_err(operation: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = operation, error = ?err, "database operation failed");
    ApiError::internal("An unexpected database error occurred")
}

fn advance_stage(current: &CanaryRelease, target_override: Option<i32>) -> (&'static str, i32) {
    // Default stage progression
    match serde_json::to_string(&current.current_stage)
        .unwrap_or_default()
        .trim_matches('"')
    {
        "stage_1" => ("stage_2", target_override.unwrap_or(10)),
        "stage_2" => ("stage_3", target_override.unwrap_or(25)),
        "stage_3" => ("stage_4", target_override.unwrap_or(50)),
        "stage_4" => ("complete", target_override.unwrap_or(100)),
        _ => ("stage_2", target_override.unwrap_or(10)),
    }
}
