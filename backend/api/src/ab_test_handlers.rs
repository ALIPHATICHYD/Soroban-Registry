use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{json, Value};
use shared::models::{
    AbTest, AbTestMetric, AbTestResult, CreateAbTestRequest, RecordAbTestMetricRequest,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

// ───────────────────── Query params ─────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct ListAbTestsQuery {
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

/// POST /api/contracts/:id/ab-tests — create a new A/B test
pub async fn create_ab_test(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    ValidatedJson(req): ValidatedJson<CreateAbTestRequest>,
) -> ApiResult<impl IntoResponse> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;
    let variant_a_uuid = parse_uuid(&req.variant_a_deployment_id, "variant_a_deployment")?;
    let variant_b_uuid = parse_uuid(&req.variant_b_deployment_id, "variant_b_deployment")?;
    let traffic_split = req.traffic_split.unwrap_or(50.0);
    let significance = req.significance_threshold.unwrap_or(95.0);
    let min_sample = req.min_sample_size.unwrap_or(1000);

    // Ensure no running test for this contract
    let existing: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM ab_tests WHERE contract_id = $1 AND status = 'running'")
            .bind(contract_uuid)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| db_err("check existing ab test", e))?;

    if existing.is_some() {
        return Err(ApiError::conflict(
            "AbTestAlreadyRunning",
            "A running A/B test already exists for this contract",
        ));
    }

    let test: AbTest = sqlx::query_as(
        r#"
        INSERT INTO ab_tests
            (contract_id, name, description, traffic_split,
             variant_a_deployment_id, variant_b_deployment_id,
             primary_metric, hypothesis, significance_threshold,
             min_sample_size, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING *
        "#,
    )
    .bind(contract_uuid)
    .bind(&req.name)
    .bind(req.description.as_deref())
    .bind(rust_decimal::Decimal::try_from(traffic_split).unwrap_or_default())
    .bind(variant_a_uuid)
    .bind(variant_b_uuid)
    .bind(&req.primary_metric)
    .bind(req.hypothesis.as_deref())
    .bind(rust_decimal::Decimal::try_from(significance).unwrap_or_default())
    .bind(min_sample)
    .bind(req.created_by.as_deref())
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_err("create ab test", e))?;

    // Create variant records
    let _ = sqlx::query(
        r#"
        INSERT INTO ab_test_variants (test_id, variant_type, deployment_id, traffic_percentage)
        VALUES ($1, 'control', $2, $3), ($1, 'treatment', $4, $5)
        "#,
    )
    .bind(test.id)
    .bind(variant_a_uuid)
    .bind(rust_decimal::Decimal::try_from(traffic_split).unwrap_or_default())
    .bind(variant_b_uuid)
    .bind(rust_decimal::Decimal::try_from(100.0 - traffic_split).unwrap_or_default())
    .execute(&state.db)
    .await;

    Ok((StatusCode::CREATED, Json(test)))
}

/// GET /api/contracts/:id/ab-tests — list A/B tests for a contract
pub async fn list_ab_tests(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<ListAbTestsQuery>,
) -> ApiResult<Json<Value>> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    let (tests, total): (Vec<AbTest>, i64) = if let Some(ref status) = params.status {
        let items: Vec<AbTest> = sqlx::query_as(
            "SELECT * FROM ab_tests WHERE contract_id = $1 AND status::text = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4",
        )
        .bind(contract_uuid)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_err("list ab tests", e))?;

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ab_tests WHERE contract_id = $1 AND status::text = $2",
        )
        .bind(contract_uuid)
        .bind(status)
        .fetch_one(&state.db)
        .await
        .map_err(|e| db_err("count ab tests", e))?;

        (items, count)
    } else {
        let items: Vec<AbTest> = sqlx::query_as(
            "SELECT * FROM ab_tests WHERE contract_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(contract_uuid)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_err("list ab tests", e))?;

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ab_tests WHERE contract_id = $1")
            .bind(contract_uuid)
            .fetch_one(&state.db)
            .await
            .map_err(|e| db_err("count ab tests", e))?;

        (items, count)
    };

    Ok(Json(json!({
        "items": tests,
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}

/// GET /api/ab-tests/:test_id — get a specific A/B test
pub async fn get_ab_test(
    State(state): State<AppState>,
    Path(test_id): Path<String>,
) -> ApiResult<Json<AbTest>> {
    let test_uuid = parse_uuid(&test_id, "test")?;

    let test: AbTest = sqlx::query_as("SELECT * FROM ab_tests WHERE id = $1")
        .bind(test_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "AbTestNotFound",
                format!("No A/B test found with ID: {}", test_id),
            ),
            _ => db_err("get ab test", e),
        })?;

    Ok(Json(test))
}

/// POST /api/ab-tests/:test_id/start — start a draft A/B test
pub async fn start_ab_test(
    State(state): State<AppState>,
    Path(test_id): Path<String>,
) -> ApiResult<Json<AbTest>> {
    let test_uuid = parse_uuid(&test_id, "test")?;

    let test: AbTest = sqlx::query_as(
        r#"
        UPDATE ab_tests
        SET status = 'running', started_at = NOW()
        WHERE id = $1 AND status = 'draft'
        RETURNING *
        "#,
    )
    .bind(test_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => {
            ApiError::not_found("AbTestNotFound", "No draft A/B test found to start")
        }
        _ => db_err("start ab test", e),
    })?;

    Ok(Json(test))
}

/// POST /api/ab-tests/:test_id/stop — stop a running A/B test
pub async fn stop_ab_test(
    State(state): State<AppState>,
    Path(test_id): Path<String>,
) -> ApiResult<Json<AbTest>> {
    let test_uuid = parse_uuid(&test_id, "test")?;

    let test: AbTest = sqlx::query_as(
        r#"
        UPDATE ab_tests
        SET status = 'completed', ended_at = NOW()
        WHERE id = $1 AND status = 'running'
        RETURNING *
        "#,
    )
    .bind(test_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => {
            ApiError::not_found("AbTestNotFound", "No running A/B test found to stop")
        }
        _ => db_err("stop ab test", e),
    })?;

    Ok(Json(test))
}

/// POST /api/ab-tests/:test_id/cancel — cancel an A/B test
pub async fn cancel_ab_test(
    State(state): State<AppState>,
    Path(test_id): Path<String>,
) -> ApiResult<Json<AbTest>> {
    let test_uuid = parse_uuid(&test_id, "test")?;

    let test: AbTest = sqlx::query_as(
        r#"
        UPDATE ab_tests
        SET status = 'cancelled', ended_at = NOW()
        WHERE id = $1 AND status IN ('draft', 'running', 'paused')
        RETURNING *
        "#,
    )
    .bind(test_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => {
            ApiError::not_found("AbTestNotFound", "No cancellable A/B test found")
        }
        _ => db_err("cancel ab test", e),
    })?;

    Ok(Json(test))
}

/// POST /api/ab-tests/:test_id/metrics — record an A/B test metric
pub async fn record_ab_test_metric(
    State(state): State<AppState>,
    Path(test_id): Path<String>,
    ValidatedJson(req): ValidatedJson<RecordAbTestMetricRequest>,
) -> ApiResult<impl IntoResponse> {
    let test_uuid = parse_uuid(&test_id, "test")?;

    // Determine user variant assignment (uses DB function)
    let user_addr = req.user_address.as_deref().unwrap_or("anonymous");

    let variant: Option<String> = sqlx::query_scalar("SELECT assign_variant($1, $2)::text")
        .bind(test_uuid)
        .bind(user_addr)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| db_err("assign variant", e))?
        .flatten();

    let variant_type = variant.unwrap_or_else(|| "control".to_string());

    let metric: AbTestMetric = sqlx::query_as(
        r#"
        INSERT INTO ab_test_metrics
            (test_id, variant_type, metric_name, metric_value, user_address, metadata)
        VALUES ($1, $2::variant_type, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(test_uuid)
    .bind(&variant_type)
    .bind(&req.metric_name)
    .bind(rust_decimal::Decimal::try_from(req.metric_value).unwrap_or_default())
    .bind(req.user_address.as_deref())
    .bind(&req.metadata)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_err("record ab test metric", e))?;

    Ok((StatusCode::CREATED, Json(metric)))
}

/// GET /api/ab-tests/:test_id/results — get A/B test results
pub async fn get_ab_test_results(
    State(state): State<AppState>,
    Path(test_id): Path<String>,
) -> ApiResult<Json<Value>> {
    let test_uuid = parse_uuid(&test_id, "test")?;

    let test: AbTest = sqlx::query_as("SELECT * FROM ab_tests WHERE id = $1")
        .bind(test_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "AbTestNotFound",
                format!("No A/B test found with ID: {}", test_id),
            ),
            _ => db_err("get ab test for results", e),
        })?;

    let results: Vec<AbTestResult> = sqlx::query_as(
        "SELECT * FROM ab_test_results WHERE test_id = $1 ORDER BY calculated_at DESC",
    )
    .bind(test_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("get ab test results", e))?;

    // Aggregate metric counts per variant
    let control_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ab_test_metrics WHERE test_id = $1 AND variant_type = 'control'",
    )
    .bind(test_uuid)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let treatment_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ab_test_metrics WHERE test_id = $1 AND variant_type = 'treatment'",
    )
    .bind(test_uuid)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    Ok(Json(json!({
        "test": test,
        "results": results,
        "metric_counts": {
            "control": control_count,
            "treatment": treatment_count,
            "total": control_count + treatment_count,
        }
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
