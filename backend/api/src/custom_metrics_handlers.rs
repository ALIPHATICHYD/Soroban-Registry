use crate::validation::extractors::ValidatedJson;
use crate::validation::handler_requests::RecordMetricsBatchRequest;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use shared::{CustomMetric, CustomMetricAggregate, CustomMetricType, RecordCustomMetricRequest};
use sqlx::{QueryBuilder, Row};

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

fn db_error(operation: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation, error = ?err, "database operation failed");
    ApiError::internal("Database operation failed")
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct MetricQuery {
    pub metric: Option<String>,
    pub resolution: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct MetricCatalogQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct MetricSeriesResponse {
    pub contract_id: String,
    pub metric_name: String,
    pub metric_type: Option<CustomMetricType>,
    pub resolution: String,
    pub points: Vec<MetricSeriesPoint>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct MetricSeriesPoint {
    pub bucket_start: DateTime<Utc>,
    pub bucket_end: DateTime<Utc>,
    pub sample_count: i32,
    pub sum_value: Option<f64>,
    pub avg_value: Option<f64>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub p50_value: Option<f64>,
    pub p95_value: Option<f64>,
    pub p99_value: Option<f64>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct MetricSampleResponse {
    pub contract_id: String,
    pub metric_name: String,
    pub metric_type: Option<CustomMetricType>,
    pub resolution: String,
    pub samples: Vec<MetricSample>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct MetricSample {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub unit: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct MetricCatalogEntry {
    pub metric_name: String,
    pub metric_type: CustomMetricType,
    pub last_seen: DateTime<Utc>,
    pub sample_count: i64,
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/metrics/catalog",
    params(
        ("id" = String, Path, description = "Contract UUID"),
        MetricCatalogQuery
    ),
    responses(
        (status = 200, description = "Catalog of available metrics", body = [MetricCatalogEntry])
    ),
    tag = "Metrics"
)]
pub async fn get_metric_catalog(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(query): Query<MetricCatalogQuery>,
) -> ApiResult<Json<Vec<MetricCatalogEntry>>> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    let rows = sqlx::query(
        "SELECT metric_name, metric_type, MAX(timestamp) as last_seen, COUNT(*) as sample_count \
         FROM contract_custom_metrics WHERE contract_id = $1 \
         GROUP BY metric_name, metric_type \
         ORDER BY last_seen DESC LIMIT $2",
    )
    .bind(&contract_id)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_error("fetch metric catalog", e))?;

    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let metric_name: String = row.try_get("metric_name").unwrap_or_default();
        let metric_type: CustomMetricType = row
            .try_get("metric_type")
            .map_err(|e| db_error("parse metric_type", e))?;
        let last_seen: DateTime<Utc> = row
            .try_get("last_seen")
            .map_err(|e| db_error("parse last_seen", e))?;
        let sample_count: i64 = row.try_get("sample_count").unwrap_or(0);
        entries.push(MetricCatalogEntry {
            metric_name,
            metric_type,
            last_seen,
            sample_count,
        });
    }

    Ok(Json(entries))
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/metrics",
    params(
        ("id" = String, Path, description = "Contract UUID"),
        MetricQuery
    ),
    responses(
        (status = 200, description = "Time-series metric data", body = Object),
        (status = 400, description = "Missing or invalid parameters")
    ),
    tag = "Metrics"
)]
pub async fn get_contract_metrics(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(query): Query<MetricQuery>,
) -> ApiResult<impl IntoResponse> {
    let metric_name = match query.metric {
        Some(name) if !name.trim().is_empty() => name,
        _ => {
            return Err(ApiError::bad_request(
                "MissingMetric",
                "Query parameter 'metric' is required", // e.g. ?metric=custom_trades_volume
            ));
        }
    };

    let resolution = query.resolution.as_deref().unwrap_or("hour").to_lowercase();

    let from_ts = query
        .from
        .as_deref()
        .and_then(|raw| raw.parse::<DateTime<Utc>>().ok());
    let to_ts = query
        .to
        .as_deref()
        .and_then(|raw| raw.parse::<DateTime<Utc>>().ok());

    let limit = query.limit.unwrap_or(500).clamp(1, 5000);

    if resolution == "raw" {
        let mut qb = QueryBuilder::new(
            "SELECT id, contract_id, metric_name, metric_type, value, unit, metadata, ledger_sequence, \
             transaction_hash, timestamp, network, created_at \
             FROM contract_custom_metrics WHERE contract_id = ",
        );
        qb.push_bind(&contract_id);
        qb.push(" AND metric_name = ");
        qb.push_bind(&metric_name);

        if let Some(from_ts) = from_ts {
            qb.push(" AND timestamp >= ");
            qb.push_bind(from_ts);
        }

        if let Some(to_ts) = to_ts {
            qb.push(" AND timestamp <= ");
            qb.push_bind(to_ts);
        }

        qb.push(" ORDER BY timestamp DESC LIMIT ");
        qb.push_bind(limit);

        let samples = qb
            .build_query_as::<CustomMetric>()
            .fetch_all(&state.db)
            .await
            .map_err(|e| db_error("fetch raw metrics", e))?;

        if samples.is_empty() {
            let metric_type = fetch_metric_type(&state, &contract_id, &metric_name).await?;
            return Ok((
                StatusCode::OK,
                Json(MetricSampleResponse {
                    contract_id,
                    metric_name,
                    metric_type,
                    resolution,
                    samples: Vec::new(),
                }),
            )
                .into_response());
        }

        let metric_type = Some(samples[0].metric_type.clone());
        let series = MetricSampleResponse {
            contract_id,
            metric_name,
            metric_type,
            resolution,
            samples: samples
                .into_iter()
                .map(|row| MetricSample {
                    timestamp: row.timestamp,
                    value: row.value.to_string().parse::<f64>().unwrap_or(0.0),
                    unit: row.unit,
                    metadata: row.metadata,
                })
                .collect(),
        };

        return Ok((StatusCode::OK, Json(series)).into_response());
    }

    let (table, bucket_column) = match resolution.as_str() {
        "day" | "daily" => ("contract_custom_metrics_daily", "bucket_start"),
        _ => ("contract_custom_metrics_hourly", "bucket_start"),
    };

    let mut qb = QueryBuilder::new(format!(
        "SELECT contract_id, metric_name, metric_type, bucket_start, bucket_end, sample_count, \
         sum_value, avg_value, min_value, max_value, p50_value, p95_value, p99_value \
         FROM {} WHERE contract_id = ",
        table
    ));
    qb.push_bind(&contract_id);
    qb.push(" AND metric_name = ");
    qb.push_bind(&metric_name);

    if let Some(from_ts) = from_ts {
        qb.push(" AND ");
        qb.push(bucket_column);
        qb.push(" >= ");
        qb.push_bind(from_ts);
    }

    if let Some(to_ts) = to_ts {
        qb.push(" AND ");
        qb.push(bucket_column);
        qb.push(" <= ");
        qb.push_bind(to_ts);
    }

    qb.push(" ORDER BY ");
    qb.push(bucket_column);
    qb.push(" DESC LIMIT ");
    qb.push_bind(limit);

    let points = qb
        .build_query_as::<CustomMetricAggregate>()
        .fetch_all(&state.db)
        .await
        .map_err(|e| db_error("fetch aggregated metrics", e))?;

    if points.is_empty() {
        let metric_type = fetch_metric_type(&state, &contract_id, &metric_name).await?;
        return Ok((
            StatusCode::OK,
            Json(MetricSeriesResponse {
                contract_id,
                metric_name,
                metric_type,
                resolution,
                points: Vec::new(),
            }),
        )
            .into_response());
    }

    let metric_type = Some(points[0].metric_type.clone());
    let series = MetricSeriesResponse {
        contract_id,
        metric_name,
        metric_type,
        resolution,
        points: points
            .into_iter()
            .map(|row| MetricSeriesPoint {
                bucket_start: row.bucket_start,
                bucket_end: row.bucket_end,
                sample_count: row.sample_count,
                sum_value: row
                    .sum_value
                    .map(|v| v.to_string().parse::<f64>().unwrap_or(0.0)),
                avg_value: row
                    .avg_value
                    .map(|v| v.to_string().parse::<f64>().unwrap_or(0.0)),
                min_value: row
                    .min_value
                    .map(|v| v.to_string().parse::<f64>().unwrap_or(0.0)),
                max_value: row
                    .max_value
                    .map(|v| v.to_string().parse::<f64>().unwrap_or(0.0)),
                p50_value: row
                    .p50_value
                    .map(|v| v.to_string().parse::<f64>().unwrap_or(0.0)),
                p95_value: row
                    .p95_value
                    .map(|v| v.to_string().parse::<f64>().unwrap_or(0.0)),
                p99_value: row
                    .p99_value
                    .map(|v| v.to_string().parse::<f64>().unwrap_or(0.0)),
            })
            .collect(),
    };

    Ok((StatusCode::OK, Json(series)).into_response())
}

async fn fetch_metric_type(
    state: &AppState,
    contract_id: &str,
    metric_name: &str,
) -> ApiResult<Option<CustomMetricType>> {
    let metric_type = sqlx::query_scalar::<_, CustomMetricType>(
        "SELECT metric_type FROM contract_custom_metrics \
         WHERE contract_id = $1 AND metric_name = $2 \
         ORDER BY timestamp DESC LIMIT 1",
    )
    .bind(contract_id)
    .bind(metric_name)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| db_error("fetch metric type", e))?;

    Ok(metric_type)
}

#[utoipa::path(
    post,
    path = "/api/contracts/{id}/metrics",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    request_body = RecordCustomMetricRequest,
    responses(
        (status = 201, description = "Metric recorded successfully", body = CustomMetric),
        (status = 400, description = "Contract mismatch or invalid input")
    ),
    tag = "Metrics"
)]
pub async fn record_contract_metric(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    ValidatedJson(payload): ValidatedJson<RecordCustomMetricRequest>,
) -> ApiResult<Json<CustomMetric>> {
    if payload.contract_id != contract_id {
        return Err(ApiError::bad_request(
            "ContractMismatch",
            "Contract ID in payload does not match path",
        ));
    }

    let timestamp = payload.timestamp.unwrap_or_else(Utc::now);
    let network = payload.network.unwrap_or(shared::Network::Testnet);

    let metric = sqlx::query_as::<_, CustomMetric>(
        "INSERT INTO contract_custom_metrics \
         (contract_id, metric_name, metric_type, value, unit, metadata, ledger_sequence, transaction_hash, timestamp, network) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         RETURNING id, contract_id, metric_name, metric_type, value, unit, metadata, ledger_sequence, transaction_hash, \
                   timestamp, network, created_at",
    )
    .bind(&payload.contract_id)
    .bind(&payload.metric_name)
    .bind(&payload.metric_type)
    .bind(payload.value)
    .bind(&payload.unit)
    .bind(&payload.metadata)
    .bind(payload.ledger_sequence)
    .bind(&payload.transaction_hash)
    .bind(timestamp)
    .bind(network)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_error("insert custom metric", e))?;

    Ok(Json(metric))
}

#[utoipa::path(
    post,
    path = "/api/contracts/{id}/metrics/batch",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    request_body = [RecordCustomMetricRequest],
    responses(
        (status = 201, description = "Batch of metrics recorded", body = Object)
    ),
    tag = "Metrics"
)]
pub async fn record_metrics_batch(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    ValidatedJson(payload): ValidatedJson<RecordMetricsBatchRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if payload.is_empty() {
        return Ok(Json(serde_json::json!({
            "inserted": 0,
            "errors": 0,
            "total": 0
        })));
    }

    let mut inserted = 0u64;
    let mut errors = 0u64;

    for metric in payload {
        if metric.contract_id != contract_id {
            errors += 1;
            continue;
        }
        let timestamp = metric.timestamp.unwrap_or_else(Utc::now);
        let network = metric.network.unwrap_or(shared::Network::Testnet);

        let result = sqlx::query(
            "INSERT INTO contract_custom_metrics \
             (contract_id, metric_name, metric_type, value, unit, metadata, ledger_sequence, transaction_hash, timestamp, network) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&metric.contract_id)
        .bind(&metric.metric_name)
        .bind(&metric.metric_type)
        .bind(metric.value)
        .bind(&metric.unit)
        .bind(&metric.metadata)
        .bind(metric.ledger_sequence)
        .bind(&metric.transaction_hash)
        .bind(timestamp)
        .bind(network)
        .execute(&state.db)
        .await;

        match result {
            Ok(_) => inserted += 1,
            Err(e) => {
                tracing::warn!(error = ?e, "failed to insert custom metric");
                errors += 1;
            }
        }
    }

    Ok(Json(serde_json::json!({
        "inserted": inserted,
        "errors": errors,
        "total": inserted + errors
    })))
}
