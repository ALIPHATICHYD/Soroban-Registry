use crate::validation::extractors::ValidatedJson;
use crate::validation::handler_requests::ValidatedJsonBody;
use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use shared::{
    ContractPerformanceSummaryResponse, CreateAlertConfigRequest, PerformanceAlert,
    PerformanceAlertConfig, PerformanceAnomaly, PerformanceBenchmark, PerformanceComparisonEntry,
    PerformanceMetric, PerformanceMetricSnapshot, PerformanceRegression, PerformanceTrendPoint,
    RecordPerformanceBenchmarkRequest, RecordPerformanceMetricRequest,
};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

// ───────────────────── Query params ─────────────────────

#[derive(Debug, serde::Deserialize)]
pub struct ListMetricsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub metric_type: Option<String>,
    pub function_name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ListBenchmarksQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub benchmark_name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ListAlertsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub resolved: Option<bool>,
    pub severity: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PerformanceComparisonQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub benchmark_name: Option<String>,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, FromRow)]
struct BenchmarkRow {
    id: Uuid,
    contract_id: Uuid,
    contract_version_id: Option<Uuid>,
    version: Option<String>,
    benchmark_name: String,
    execution_time_ms: Decimal,
    gas_used: i64,
    sample_size: i32,
    source: String,
    recorded_at: DateTime<Utc>,
    metadata: Option<Value>,
}

#[derive(Debug, FromRow)]
struct MetricSnapshotRow {
    metric_type: String,
    benchmark_name: Option<String>,
    latest_value: Decimal,
    previous_value: Option<Decimal>,
    change_percent: Option<Decimal>,
    recorded_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct TrendPointRow {
    bucket_start: DateTime<Utc>,
    bucket_end: DateTime<Utc>,
    benchmark_name: String,
    avg_execution_time_ms: Decimal,
    avg_gas_used: Decimal,
    sample_count: i64,
}

#[derive(Debug, FromRow)]
struct RegressionRow {
    benchmark_name: String,
    current_version: Option<String>,
    previous_version: Option<String>,
    execution_time_regression_percent: Option<Decimal>,
    gas_regression_percent: Option<Decimal>,
    severity: String,
    detected_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct ComparisonRow {
    contract_id: Uuid,
    contract_name: String,
    category: Option<String>,
    benchmark_name: String,
    avg_execution_time_ms: Decimal,
    avg_gas_used: Decimal,
    sample_count: i64,
}

// ───────────────────── Handlers ─────────────────────

/// POST /api/contracts/:id/perf/metrics — record a performance metric
pub async fn record_metric(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    ValidatedJson(req): ValidatedJson<RecordPerformanceMetricRequest>,
) -> ApiResult<impl IntoResponse> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;

    let metric: PerformanceMetric = sqlx::query_as(
        r#"
        INSERT INTO performance_metrics
            (contract_id, metric_type, function_name, value, p50, p95, p99, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(contract_uuid)
    .bind(&req.metric_type)
    .bind(req.function_name.as_deref())
    .bind(rust_decimal::Decimal::try_from(req.value).unwrap_or_default())
    .bind(
        req.p50
            .map(|v| rust_decimal::Decimal::try_from(v).unwrap_or_default()),
    )
    .bind(
        req.p95
            .map(|v| rust_decimal::Decimal::try_from(v).unwrap_or_default()),
    )
    .bind(
        req.p99
            .map(|v| rust_decimal::Decimal::try_from(v).unwrap_or_default()),
    )
    .bind(&req.metadata)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_err("record performance metric", e))?;

    Ok((StatusCode::CREATED, Json(metric)))
}

/// POST /api/contracts/:id/perf/benchmarks — record a version-aware performance benchmark
pub async fn record_benchmark(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    ValidatedJson(req): ValidatedJson<RecordPerformanceBenchmarkRequest>,
) -> ApiResult<impl IntoResponse> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;
    let version_uuid = req
        .contract_version_id
        .as_deref()
        .map(|id| parse_uuid(id, "contract_version"))
        .transpose()?;

    let row: BenchmarkRow = sqlx::query_as(
        r#"
        INSERT INTO contract_performance_benchmarks (
            contract_id,
            contract_version_id,
            benchmark_name,
            execution_time_ms,
            gas_used,
            sample_size,
            source,
            metadata
        )
        VALUES ($1, $2, $3, $4, $5, COALESCE($6, 1), COALESCE($7, 'manual'), $8)
        RETURNING
            id,
            contract_id,
            contract_version_id,
            NULL::TEXT AS version,
            benchmark_name,
            execution_time_ms,
            gas_used,
            sample_size,
            source,
            recorded_at,
            metadata
        "#,
    )
    .bind(contract_uuid)
    .bind(version_uuid)
    .bind(&req.benchmark_name)
    .bind(decimal_from_f64(req.execution_time_ms))
    .bind(req.gas_used)
    .bind(req.sample_size)
    .bind(req.source.as_deref())
    .bind(&req.metadata)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_err("record performance benchmark", e))?;

    Ok((StatusCode::CREATED, Json(benchmark_from_row(row))))
}

/// GET /api/contracts/:id/perf/benchmarks — list recorded benchmarks for a contract
pub async fn list_benchmarks(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<ListBenchmarksQuery>,
) -> ApiResult<Json<Value>> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    let benchmark_name_clause = params
        .benchmark_name
        .as_ref()
        .map(|name| format!(" AND b.benchmark_name = '{}'", name.replace('\'', "''")))
        .unwrap_or_default();
    let version_clause = params
        .version
        .as_ref()
        .map(|version| format!(" AND cv.version = '{}'", version.replace('\'', "''")))
        .unwrap_or_default();

    let query = format!(
        r#"
        SELECT
            b.id,
            b.contract_id,
            b.contract_version_id,
            cv.version,
            b.benchmark_name,
            b.execution_time_ms,
            b.gas_used,
            b.sample_size,
            b.source,
            b.recorded_at,
            b.metadata
        FROM contract_performance_benchmarks b
        LEFT JOIN contract_versions cv ON cv.id = b.contract_version_id
        WHERE b.contract_id = $1
        {benchmark_name_clause}
        {version_clause}
        ORDER BY b.recorded_at DESC
        LIMIT {limit} OFFSET {offset}
        "#
    );

    let count_query = format!(
        r#"
        SELECT COUNT(*)
        FROM contract_performance_benchmarks b
        LEFT JOIN contract_versions cv ON cv.id = b.contract_version_id
        WHERE b.contract_id = $1
        {benchmark_name_clause}
        {version_clause}
        "#
    );

    let rows: Vec<BenchmarkRow> = sqlx::query_as(&query)
        .bind(contract_uuid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_err("list performance benchmarks", e))?;

    let total: i64 = sqlx::query_scalar(&count_query)
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|e| db_err("count performance benchmarks", e))?;

    let items: Vec<PerformanceBenchmark> = rows.into_iter().map(benchmark_from_row).collect();

    Ok(Json(json!({
        "items": items,
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}

/// GET /api/contracts/:id/perf/metrics — list performance metrics for a contract
pub async fn list_metrics(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<ListMetricsQuery>,
) -> ApiResult<Json<Value>> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    // Build dynamic query filters
    let mut query = String::from("SELECT * FROM performance_metrics WHERE contract_id = $1");
    let mut count_query =
        String::from("SELECT COUNT(*) FROM performance_metrics WHERE contract_id = $1");

    if let Some(ref mt) = params.metric_type {
        let clause = format!(" AND metric_type::text = '{}'", mt.replace('\'', "''"));
        query.push_str(&clause);
        count_query.push_str(&clause);
    }
    if let Some(ref func) = params.function_name {
        let clause = format!(" AND function_name = '{}'", func.replace('\'', "''"));
        query.push_str(&clause);
        count_query.push_str(&clause);
    }

    query.push_str(&format!(
        " ORDER BY timestamp DESC LIMIT {} OFFSET {}",
        limit, offset
    ));

    let metrics: Vec<PerformanceMetric> = sqlx::query_as(&query)
        .bind(contract_uuid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_err("list performance metrics", e))?;

    let total: i64 = sqlx::query_scalar(&count_query)
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|e| db_err("count performance metrics", e))?;

    Ok(Json(json!({
        "items": metrics,
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}

/// GET /api/contracts/:id/perf/anomalies — list performance anomalies
pub async fn list_anomalies(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<ListAlertsQuery>,
) -> ApiResult<Json<Value>> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    let mut query = String::from("SELECT * FROM performance_anomalies WHERE contract_id = $1");
    let mut count_query =
        String::from("SELECT COUNT(*) FROM performance_anomalies WHERE contract_id = $1");

    if let Some(resolved) = params.resolved {
        let clause = format!(" AND resolved = {}", resolved);
        query.push_str(&clause);
        count_query.push_str(&clause);
    }
    if let Some(ref severity) = params.severity {
        let clause = format!(" AND severity::text = '{}'", severity.replace('\'', "''"));
        query.push_str(&clause);
        count_query.push_str(&clause);
    }

    query.push_str(&format!(
        " ORDER BY detected_at DESC LIMIT {} OFFSET {}",
        limit, offset
    ));

    let anomalies: Vec<PerformanceAnomaly> = sqlx::query_as(&query)
        .bind(contract_uuid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_err("list performance anomalies", e))?;

    let total: i64 = sqlx::query_scalar(&count_query)
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|e| db_err("count performance anomalies", e))?;

    Ok(Json(json!({
        "items": anomalies,
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}

/// GET /api/contracts/:id/perf/alerts — list performance alerts
pub async fn list_alerts(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<ListAlertsQuery>,
) -> ApiResult<Json<Value>> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;
    let limit = params.limit.clamp(1, 100);
    let offset = params.offset.max(0);

    let mut query = String::from("SELECT * FROM performance_alerts WHERE contract_id = $1");
    let mut count_query =
        String::from("SELECT COUNT(*) FROM performance_alerts WHERE contract_id = $1");

    if let Some(resolved) = params.resolved {
        let clause = format!(" AND resolved = {}", resolved);
        query.push_str(&clause);
        count_query.push_str(&clause);
    }
    if let Some(ref severity) = params.severity {
        let clause = format!(" AND severity::text = '{}'", severity.replace('\'', "''"));
        query.push_str(&clause);
        count_query.push_str(&clause);
    }

    query.push_str(&format!(
        " ORDER BY triggered_at DESC LIMIT {} OFFSET {}",
        limit, offset
    ));

    let alerts: Vec<PerformanceAlert> = sqlx::query_as(&query)
        .bind(contract_uuid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_err("list performance alerts", e))?;

    let total: i64 = sqlx::query_scalar(&count_query)
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|e| db_err("count performance alerts", e))?;

    Ok(Json(json!({
        "items": alerts,
        "total": total,
        "limit": limit,
        "offset": offset,
    })))
}

/// POST /api/perf/alerts/:alert_id/acknowledge — acknowledge a performance alert
pub async fn acknowledge_alert(
    State(state): State<AppState>,
    Path(alert_id): Path<String>,
    ValidatedJson(body): ValidatedJson<ValidatedJsonBody>,
) -> ApiResult<Json<PerformanceAlert>> {
    let alert_uuid = parse_uuid(&alert_id, "alert")?;
    let acknowledged_by = body
        .get("acknowledged_by")
        .and_then(|v| v.as_str())
        .unwrap_or("system");

    let alert: PerformanceAlert = sqlx::query_as(
        r#"
        UPDATE performance_alerts
        SET acknowledged = true,
            acknowledged_at = NOW(),
            acknowledged_by = $2
        WHERE id = $1 AND acknowledged = false
        RETURNING *
        "#,
    )
    .bind(alert_uuid)
    .bind(acknowledged_by)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::not_found(
            "AlertNotFound",
            "No unacknowledged alert found with this ID",
        ),
        _ => db_err("acknowledge alert", e),
    })?;

    Ok(Json(alert))
}

/// POST /api/perf/alerts/:alert_id/resolve — resolve a performance alert
pub async fn resolve_alert(
    State(state): State<AppState>,
    Path(alert_id): Path<String>,
) -> ApiResult<Json<PerformanceAlert>> {
    let alert_uuid = parse_uuid(&alert_id, "alert")?;

    let alert: PerformanceAlert = sqlx::query_as(
        r#"
        UPDATE performance_alerts
        SET resolved = true, resolved_at = NOW()
        WHERE id = $1 AND resolved = false
        RETURNING *
        "#,
    )
    .bind(alert_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => {
            ApiError::not_found("AlertNotFound", "No unresolved alert found with this ID")
        }
        _ => db_err("resolve alert", e),
    })?;

    Ok(Json(alert))
}

/// POST /api/contracts/:id/perf/alert-configs — configure an alert threshold
pub async fn create_alert_config(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    ValidatedJson(req): ValidatedJson<CreateAlertConfigRequest>,
) -> ApiResult<impl IntoResponse> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;

    let config: PerformanceAlertConfig = sqlx::query_as(
        r#"
        INSERT INTO performance_alert_configs
            (contract_id, metric_type, threshold_type, threshold_value, severity)
        VALUES ($1, $2, $3, $4, COALESCE($5, 'warning'))
        ON CONFLICT (contract_id, metric_type, threshold_type)
        DO UPDATE SET
            threshold_value = EXCLUDED.threshold_value,
            severity = EXCLUDED.severity,
            updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(contract_uuid)
    .bind(&req.metric_type)
    .bind(&req.threshold_type)
    .bind(rust_decimal::Decimal::try_from(req.threshold_value).unwrap_or_default())
    .bind(&req.severity)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_err("create alert config", e))?;

    Ok((StatusCode::CREATED, Json(config)))
}

/// GET /api/contracts/:id/perf/alert-configs — list alert configurations
pub async fn list_alert_configs(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> ApiResult<Json<Vec<PerformanceAlertConfig>>> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;

    let configs: Vec<PerformanceAlertConfig> = sqlx::query_as(
        "SELECT * FROM performance_alert_configs WHERE contract_id = $1 ORDER BY created_at DESC",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("list alert configs", e))?;

    Ok(Json(configs))
}

/// GET /api/contracts/:id/perf/trends — list performance trends
pub async fn list_trends(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<ListMetricsQuery>,
) -> ApiResult<Json<Value>> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;
    let limit = params.limit.clamp(1, 100);
    let trend_limit = limit * 30;

    let rows: Vec<TrendPointRow> = sqlx::query_as(
        r#"
        WITH grouped AS (
            SELECT
                date_trunc('day', recorded_at) AS bucket_start,
                benchmark_name,
                AVG(execution_time_ms) AS avg_execution_time_ms,
                AVG(gas_used::DECIMAL) AS avg_gas_used,
                COUNT(*) AS sample_count
            FROM contract_performance_benchmarks
            WHERE contract_id = $1
            GROUP BY date_trunc('day', recorded_at), benchmark_name
        )
        SELECT
            bucket_start,
            bucket_start + INTERVAL '1 day' AS bucket_end,
            benchmark_name,
            avg_execution_time_ms,
            avg_gas_used,
            sample_count
        FROM grouped
        ORDER BY bucket_start DESC, benchmark_name ASC
        LIMIT $2
        "#,
    )
    .bind(contract_uuid)
    .bind(trend_limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("list benchmark trends", e))?;

    let items: Vec<PerformanceTrendPoint> = rows.into_iter().map(trend_from_row).collect();

    Ok(Json(json!({
        "items": items,
        "limit": limit,
    })))
}

/// GET /api/contracts/:id/perf/summary — get a comprehensive performance summary
pub async fn get_performance_summary(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> ApiResult<Json<ContractPerformanceSummaryResponse>> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;
    let summary = build_performance_summary_internal(&state, contract_uuid).await?;
    Ok(Json(summary))
}

/// GET /api/contracts/:id/perf/comparison — compare with similar contracts in the same category
pub async fn get_performance_comparison(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<PerformanceComparisonQuery>,
) -> ApiResult<Json<Value>> {
    let contract_uuid = parse_uuid(&contract_id, "contract")?;
    let limit = params.limit.clamp(1, 25);
    let items: Vec<PerformanceComparisonEntry> = fetch_comparisons(
        &state,
        contract_uuid,
        params.benchmark_name.as_deref(),
        limit,
    )
    .await?;

    Ok(Json(json!({
        "contract_id": contract_uuid,
        "items": items,
        "limit": limit,
    })))
}

pub async fn get_contract_performance_overview(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> ApiResult<Json<ContractPerformanceSummaryResponse>> {
    get_performance_summary(State(state), Path(contract_id)).await
}

// ───────────────────── Helpers ─────────────────────

pub(crate) async fn build_performance_summary_internal(
    state: &AppState,
    contract_uuid: Uuid,
) -> ApiResult<ContractPerformanceSummaryResponse> {
    let latest_benchmark_rows: Vec<BenchmarkRow> = sqlx::query_as(
        r#"
        SELECT DISTINCT ON (b.benchmark_name)
            b.id,
            b.contract_id,
            b.contract_version_id,
            cv.version,
            b.benchmark_name,
            b.execution_time_ms,
            b.gas_used,
            b.sample_size,
            b.source,
            b.recorded_at,
            b.metadata
        FROM contract_performance_benchmarks b
        LEFT JOIN contract_versions cv ON cv.id = b.contract_version_id
        WHERE b.contract_id = $1
        ORDER BY b.benchmark_name, b.recorded_at DESC
        "#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("get latest performance benchmarks", e))?;

    let metric_rows: Vec<MetricSnapshotRow> = sqlx::query_as(
        r#"
        WITH ranked AS (
            SELECT
                pm.metric_type::TEXT AS metric_type,
                pm.function_name AS benchmark_name,
                pm.value,
                pm.timestamp AS recorded_at,
                ROW_NUMBER() OVER (
                    PARTITION BY pm.metric_type, pm.function_name
                    ORDER BY pm.timestamp DESC
                ) AS rn
            FROM performance_metrics pm
            WHERE pm.contract_id = $1
              AND pm.metric_type IN ('execution_time', 'gas_consumption')
        )
        SELECT
            current.metric_type,
            current.benchmark_name,
            current.value AS latest_value,
            previous.value AS previous_value,
            CASE
                WHEN previous.value IS NULL OR previous.value = 0 THEN NULL
                ELSE ((current.value - previous.value) / previous.value) * 100
            END AS change_percent,
            current.recorded_at
        FROM ranked current
        LEFT JOIN ranked previous
            ON previous.metric_type = current.metric_type
           AND previous.benchmark_name IS NOT DISTINCT FROM current.benchmark_name
           AND previous.rn = 2
        WHERE current.rn = 1
        ORDER BY current.recorded_at DESC
        "#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("get performance metric snapshots", e))?;

    let trend_rows: Vec<TrendPointRow> = sqlx::query_as(
        r#"
        WITH grouped AS (
            SELECT
                date_trunc('day', recorded_at) AS bucket_start,
                benchmark_name,
                AVG(execution_time_ms) AS avg_execution_time_ms,
                AVG(gas_used::DECIMAL) AS avg_gas_used,
                COUNT(*) AS sample_count
            FROM contract_performance_benchmarks
            WHERE contract_id = $1
            GROUP BY date_trunc('day', recorded_at), benchmark_name
        )
        SELECT
            bucket_start,
            bucket_start + INTERVAL '1 day' AS bucket_end,
            benchmark_name,
            avg_execution_time_ms,
            avg_gas_used,
            sample_count
        FROM grouped
        ORDER BY bucket_start DESC, benchmark_name ASC
        LIMIT 30
        "#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("get benchmark trends", e))?;

    let regression_rows: Vec<RegressionRow> = sqlx::query_as(
        r#"
        WITH ranked AS (
            SELECT
                b.benchmark_name,
                cv.version,
                b.execution_time_ms,
                b.gas_used::DECIMAL AS gas_used,
                b.recorded_at,
                ROW_NUMBER() OVER (
                    PARTITION BY b.benchmark_name
                    ORDER BY b.recorded_at DESC
                ) AS rn
            FROM contract_performance_benchmarks b
            LEFT JOIN contract_versions cv ON cv.id = b.contract_version_id
            WHERE b.contract_id = $1
        )
        SELECT
            current.benchmark_name,
            current.version AS current_version,
            previous.version AS previous_version,
            CASE
                WHEN previous.execution_time_ms IS NULL OR previous.execution_time_ms = 0 THEN NULL
                ELSE ((current.execution_time_ms - previous.execution_time_ms) / previous.execution_time_ms) * 100
            END AS execution_time_regression_percent,
            CASE
                WHEN previous.gas_used IS NULL OR previous.gas_used = 0 THEN NULL
                ELSE ((current.gas_used - previous.gas_used) / previous.gas_used) * 100
            END AS gas_regression_percent,
            CASE
                WHEN (
                    COALESCE(
                        CASE
                            WHEN previous.execution_time_ms IS NULL OR previous.execution_time_ms = 0 THEN 0
                            ELSE ((current.execution_time_ms - previous.execution_time_ms) / previous.execution_time_ms) * 100
                        END,
                        0
                    ) >= 30
                    OR COALESCE(
                        CASE
                            WHEN previous.gas_used IS NULL OR previous.gas_used = 0 THEN 0
                            ELSE ((current.gas_used - previous.gas_used) / previous.gas_used) * 100
                        END,
                        0
                    ) >= 30
                ) THEN 'critical'
                WHEN (
                    COALESCE(
                        CASE
                            WHEN previous.execution_time_ms IS NULL OR previous.execution_time_ms = 0 THEN 0
                            ELSE ((current.execution_time_ms - previous.execution_time_ms) / previous.execution_time_ms) * 100
                        END,
                        0
                    ) >= 20
                    OR COALESCE(
                        CASE
                            WHEN previous.gas_used IS NULL OR previous.gas_used = 0 THEN 0
                            ELSE ((current.gas_used - previous.gas_used) / previous.gas_used) * 100
                        END,
                        0
                    ) >= 20
                ) THEN 'warning'
                ELSE 'info'
            END AS severity,
            current.recorded_at AS detected_at
        FROM ranked current
        LEFT JOIN ranked previous
            ON previous.benchmark_name = current.benchmark_name
           AND previous.rn = 2
        WHERE current.rn = 1
          AND previous.rn IS NOT NULL
          AND (
            COALESCE(
                CASE
                    WHEN previous.execution_time_ms IS NULL OR previous.execution_time_ms = 0 THEN 0
                    ELSE ((current.execution_time_ms - previous.execution_time_ms) / previous.execution_time_ms) * 100
                END,
                0
            ) > 10
            OR COALESCE(
                CASE
                    WHEN previous.gas_used IS NULL OR previous.gas_used = 0 THEN 0
                    ELSE ((current.gas_used - previous.gas_used) / previous.gas_used) * 100
                END,
                0
            ) > 10
          )
        ORDER BY current.recorded_at DESC
        LIMIT 10
        "#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("get performance regressions", e))?;

    let unresolved_alerts: Vec<PerformanceAlert> = sqlx::query_as(
        r#"
        SELECT *
        FROM performance_alerts
        WHERE contract_id = $1 AND resolved = false
        ORDER BY triggered_at DESC
        LIMIT 10
        "#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("get unresolved performance alerts", e))?;

    Ok(ContractPerformanceSummaryResponse {
        contract_id: contract_uuid,
        latest_benchmarks: latest_benchmark_rows
            .into_iter()
            .map(benchmark_from_row)
            .collect(),
        metric_snapshots: metric_rows
            .into_iter()
            .map(metric_snapshot_from_row)
            .collect(),
        trends: trend_rows.into_iter().map(trend_from_row).collect(),
        regressions: regression_rows
            .into_iter()
            .map(regression_from_row)
            .collect(),
        comparisons: Vec::new(),
        unresolved_alerts,
    })
}

async fn fetch_comparisons(
    state: &AppState,
    contract_uuid: Uuid,
    benchmark_name: Option<&str>,
    limit: i64,
) -> ApiResult<Vec<PerformanceComparisonEntry>> {
    let benchmark_clause = benchmark_name
        .map(|name| format!(" AND b.benchmark_name = '{}'", name.replace('\'', "''")))
        .unwrap_or_default();

    let query = format!(
        r#"
        WITH base_contract AS (
            SELECT category
            FROM contracts
            WHERE id = $1
        )
        SELECT
            c.id AS contract_id,
            c.name AS contract_name,
            c.category,
            b.benchmark_name,
            AVG(b.execution_time_ms) AS avg_execution_time_ms,
            AVG(b.gas_used::DECIMAL) AS avg_gas_used,
            COUNT(*) AS sample_count
        FROM contracts c
        JOIN base_contract bc ON c.category IS NOT DISTINCT FROM bc.category
        JOIN contract_performance_benchmarks b ON b.contract_id = c.id
        WHERE c.id <> $1
        {benchmark_clause}
        GROUP BY c.id, c.name, c.category, b.benchmark_name
        ORDER BY avg_execution_time_ms ASC, avg_gas_used ASC
        LIMIT {limit}
        "#
    );

    let rows: Vec<ComparisonRow> = sqlx::query_as(&query)
        .bind(contract_uuid)
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_err("get performance comparisons", e))?;

    Ok(rows.into_iter().map(comparison_from_row).collect())
}

fn benchmark_from_row(row: BenchmarkRow) -> PerformanceBenchmark {
    PerformanceBenchmark {
        id: row.id,
        contract_id: row.contract_id,
        contract_version_id: row.contract_version_id,
        version: row.version,
        benchmark_name: row.benchmark_name,
        execution_time_ms: row.execution_time_ms,
        gas_used: row.gas_used,
        sample_size: row.sample_size,
        source: row.source,
        recorded_at: row.recorded_at,
        metadata: row.metadata,
    }
}

fn metric_snapshot_from_row(row: MetricSnapshotRow) -> PerformanceMetricSnapshot {
    PerformanceMetricSnapshot {
        metric_type: row.metric_type,
        benchmark_name: row.benchmark_name,
        latest_value: row.latest_value,
        previous_value: row.previous_value,
        change_percent: row.change_percent,
        recorded_at: row.recorded_at,
    }
}

fn trend_from_row(row: TrendPointRow) -> PerformanceTrendPoint {
    PerformanceTrendPoint {
        bucket_start: row.bucket_start,
        bucket_end: row.bucket_end,
        benchmark_name: row.benchmark_name,
        avg_execution_time_ms: row.avg_execution_time_ms,
        avg_gas_used: row.avg_gas_used,
        sample_count: row.sample_count,
    }
}

fn regression_from_row(row: RegressionRow) -> PerformanceRegression {
    PerformanceRegression {
        benchmark_name: row.benchmark_name,
        current_version: row.current_version,
        previous_version: row.previous_version,
        execution_time_regression_percent: row.execution_time_regression_percent,
        gas_regression_percent: row.gas_regression_percent,
        severity: row.severity,
        detected_at: row.detected_at,
    }
}

fn comparison_from_row(row: ComparisonRow) -> PerformanceComparisonEntry {
    PerformanceComparisonEntry {
        contract_id: row.contract_id,
        contract_name: row.contract_name,
        category: row.category,
        benchmark_name: row.benchmark_name,
        avg_execution_time_ms: row.avg_execution_time_ms,
        avg_gas_used: row.avg_gas_used,
        sample_count: row.sample_count,
    }
}

fn decimal_from_f64(value: f64) -> Decimal {
    Decimal::try_from(value).unwrap_or_default()
}

fn parse_uuid(id: &str, label: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(id).map_err(|_| {
        ApiError::bad_request_with("InvalidId", format!("Invalid {} ID format: {}", label, id))
    })
}

fn db_err(operation: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = operation, error = ?err, "database operation failed");
    ApiError::internal("An unexpected database error occurred")
}
