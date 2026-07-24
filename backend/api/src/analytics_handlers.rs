//! Contract interaction analytics handlers (issue #415)
//!
//! Provides two endpoints:
//!
//!   GET /api/contracts/:id/analytics  – per-contract analytics with real
//!                                       deployment counts, view count, and
//!                                       a 7-day interaction trend.
//!
//!   GET /api/analytics/summary        – registry-wide aggregates broken down
//!                                       by category and by network, enabling
//!                                       trending-by-category identification.

use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use shared::{DeploymentStats, InteractorStats, Network, TopUser};
use sqlx::FromRow;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration as StdDuration;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

// ── Response types ────────────────────────────────────────────────────────────

/// One data-point in the 7-day interaction trend.
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TrendPoint {
    pub date: NaiveDate,
    /// Total interactions recorded on this day across all interaction types.
    pub count: i64,
}

/// Enhanced per-contract analytics response (issue #725).
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractAnalyticsResponse {
    pub contract_id: String,
    /// Total number of times this contract's profile page has been viewed.
    pub view_count: i64,
    /// Deployment statistics sourced from the daily-aggregate rollup table.
    pub deployments: DeploymentStats,
    /// Interactor statistics computed from raw contract_interactions.
    pub interactors: InteractorStats,
    /// Interaction timeline bucketed by the requested granularity.
    pub timeline: Vec<TrendPoint>,
    /// 7-day daily interaction trend used to identify rapidly-growing contracts.
    pub interaction_trend: Vec<TrendPoint>,
    /// Inclusive start of the requested time range.
    pub from_date: NaiveDate,
    /// Inclusive end of the requested time range.
    pub to_date: NaiveDate,
    /// Bucket granularity used for `timeline`: "daily", "weekly", or "monthly".
    pub bucket: String,
    /// Fraction of interactions that resulted in an error (publish_failed).
    /// Range [0.0, 1.0].  0.0 when there are no interactions.
    pub error_rate: f64,
    /// Whether this response was served from the cache.
    pub cached: bool,
}

/// Query parameters for GET /api/contracts/{id}/analytics (issue #725).
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ContractAnalyticsQuery {
    /// Inclusive start date (YYYY-MM-DD).  Defaults to 30 days before `to_date`.
    pub from_date: Option<NaiveDate>,
    /// Inclusive end date (YYYY-MM-DD).  Defaults to today (UTC).
    pub to_date: Option<NaiveDate>,
    /// Aggregation bucket: "daily" (default), "weekly", or "monthly".
    pub bucket: Option<String>,
    /// Response format: "json" (default) or "csv".
    pub format: Option<String>,
}

/// Analytics summary for a single category.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CategoryAnalytics {
    /// `null` for contracts that have no category assigned.
    pub category: Option<String>,
    pub contract_count: i64,
    pub total_interactions: i64,
    pub avg_interactions_per_contract: f64,
    pub total_views: i64,
}

/// Analytics summary for a single network.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct NetworkAnalytics {
    pub network: String,
    pub contract_count: i64,
    pub verified_count: i64,
    pub total_interactions: i64,
    pub total_views: i64,
}

/// Analytics summary for a single publisher.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PublisherAnalytics {
    pub publisher_id: Uuid,
    pub name: String,
    pub contract_count: i64,
    pub total_views: i64,
}

/// One data-point for registry-wide deployment trends.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DeploymentTrendPoint {
    pub date: NaiveDate,
    pub count: i64,
}

/// A summary of a recently added contract.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RecentContract {
    pub id: Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub publisher_name: Option<String>,
    pub network: String,
    pub category: Option<String>,
    pub contract_id: String,
}

/// Top-level response for GET /api/analytics/summary.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AnalyticsSummaryResponse {
    pub category_distribution: Vec<CategoryAnalytics>,
    pub network_usage: Vec<NetworkAnalytics>,
    pub top_publishers: Vec<PublisherAnalytics>,
    pub deployment_trends: Vec<DeploymentTrendPoint>,
    pub recent_additions: Vec<RecentContract>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct AnalyticsTimeSeriesQuery {
    /// Inclusive start date (YYYY-MM-DD). Defaults to 30 days before end_date.
    pub start_date: Option<NaiveDate>,
    /// Inclusive end date (YYYY-MM-DD). Defaults to today (UTC).
    pub end_date: Option<NaiveDate>,
    /// Grouping dimension: network, category, or publisher. Defaults to network.
    pub group_by: Option<String>,
    /// Optional contract network filter.
    pub network: Option<Network>,
    /// Optional category filter.
    pub category: Option<String>,
    /// Optional publisher UUID filter.
    pub publisher_id: Option<Uuid>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AnalyticsTimeSeriesPoint {
    pub date: NaiveDate,
    pub deployments: i64,
    pub verifications: i64,
    pub updates: i64,
    pub total_events: i64,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AnalyticsTimeSeriesGroup {
    pub key: String,
    pub points: Vec<AnalyticsTimeSeriesPoint>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AnalyticsTimeSeriesResponse {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub group_by: String,
    pub series: Vec<AnalyticsTimeSeriesGroup>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimeSeriesGroupBy {
    Network,
    Category,
    Publisher,
}

impl TimeSeriesGroupBy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::Category => "category",
            Self::Publisher => "publisher",
        }
    }

    fn parse(raw: Option<&str>) -> ApiResult<Self> {
        match raw.unwrap_or("network") {
            "network" => Ok(Self::Network),
            "category" => Ok(Self::Category),
            "publisher" => Ok(Self::Publisher),
            _ => Err(ApiError::bad_request_with(
                "InvalidGroupBy",
                "group_by must be one of: network, category, publisher",
            )),
        }
    }
}

#[derive(Debug, FromRow)]
struct AnalyticsTimeSeriesRow {
    day: NaiveDate,
    group_key: String,
    deployments: i64,
    verifications: i64,
    updates: i64,
    total_events: i64,
}

#[derive(Debug, serde::Deserialize)]
pub struct WebVitalMetric {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub rating: Option<String>,
    pub delta: Option<f64>,
    pub navigation_type: Option<String>,
}

pub async fn record_web_vitals(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    ValidatedJson(metric): ValidatedJson<WebVitalMetric>,
) -> ApiResult<StatusCode> {
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let referer = headers
        .get(axum::http::header::REFERER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    sqlx::query(
        "INSERT INTO web_vitals (metric_id, name, value, rating, delta, navigation_type, url, user_agent) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )
    .bind(&metric.id)
    .bind(&metric.name)
    .bind(metric.value)
    .bind(metric.rating)
    .bind(metric.delta)
    .bind(metric.navigation_type)
    .bind(referer)
    .bind(user_agent)
    .execute(&state.db)
    .await
    .map_err(|err| db_err("record web vitals", err))?;

    Ok(StatusCode::OK)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn db_err(op: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = op, error = ?err, "database operation failed");
    ApiError::internal("An unexpected database error occurred")
}

fn parse_contract_id(id: &str) -> ApiResult<Uuid> {
    Uuid::parse_str(id).map_err(|_| {
        ApiError::bad_request_with(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Return enhanced analytics for a single contract (issue #725).
///
/// Supports time-range filtering, bucket aggregation (daily/weekly/monthly),
/// error-rate tracking, CSV export, and 6-hour caching for historical queries.
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/analytics",
    params(
        ("id" = String, Path, description = "Contract UUID"),
        ContractAnalyticsQuery
    ),
    responses(
        (status = 200, description = "Contract analytics (JSON or CSV)", body = ContractAnalyticsResponse),
        (status = 404, description = "Contract not found"),
        (status = 400, description = "Invalid parameters")
    ),
    tag = "Analytics"
)]
pub async fn get_contract_analytics(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ContractAnalyticsQuery>,
) -> axum::response::Response {
    match get_contract_analytics_inner(state, id, query).await {
        Ok(r) => r,
        Err(e) => e.into_response(),
    }
}

async fn get_contract_analytics_inner(
    state: AppState,
    id: String,
    query: ContractAnalyticsQuery,
) -> ApiResult<axum::response::Response> {
    // ── Validate & resolve time range ─────────────────────────────────────────
    let to_date = query.to_date.unwrap_or_else(|| Utc::now().date_naive());
    let from_date = query
        .from_date
        .unwrap_or_else(|| to_date - Duration::days(30));

    if from_date > to_date {
        return Err(ApiError::bad_request(
            "InvalidDateRange",
            "from_date must be before or equal to to_date",
        ));
    }
    if (to_date - from_date).num_days() > 366 {
        return Err(ApiError::bad_request(
            "DateRangeTooLarge",
            "Date range cannot exceed 366 days",
        ));
    }

    let bucket = query
        .bucket
        .as_deref()
        .unwrap_or("daily")
        .to_ascii_lowercase();
    if !matches!(bucket.as_str(), "daily" | "weekly" | "monthly") {
        return Err(ApiError::bad_request(
            "InvalidBucket",
            "bucket must be one of: daily, weekly, monthly",
        ));
    }

    let format = query
        .format
        .as_deref()
        .unwrap_or("json")
        .to_ascii_lowercase();

    let contract_uuid = parse_contract_id(&id)?;

    // ── Cache lookup (6-hour TTL for historical queries) ──────────────────────
    let is_historical = to_date < Utc::now().date_naive();
    let cache_key = format!("{}:{}:{}:{}", id, from_date, to_date, bucket);
    if is_historical {
        let (cached, hit) = state.cache.get("contract_analytics", &cache_key).await;
        if hit {
            if let Some(raw) = cached {
                if let Ok(mut resp) = serde_json::from_str::<ContractAnalyticsResponse>(&raw) {
                    resp.cached = true;
                    return build_analytics_response(resp, &format);
                }
            }
        }
    }

    // ── Confirm contract exists ───────────────────────────────────────────────
    let view_count: Option<i64> =
        sqlx::query_scalar("SELECT view_count FROM contracts WHERE id = $1")
            .bind(contract_uuid)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_err("fetch view_count", err))?;

    let view_count = view_count.ok_or_else(|| {
        ApiError::not_found(
            "ContractNotFound",
            format!("No contract found with ID: {}", id),
        )
    })?;

    // ── Deployment stats (within requested range) ─────────────────────────────
    let deploy_total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(count), 0)
         FROM   contract_interaction_daily_aggregates
         WHERE  contract_id = $1
           AND  interaction_type = 'deploy'
           AND  day BETWEEN $2 AND $3",
    )
    .bind(contract_uuid)
    .bind(from_date)
    .bind(to_date)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_err("fetch deploy total", err))?;

    let unique_deployers: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT user_address)
         FROM   contract_interactions
         WHERE  contract_id = $1
           AND  interaction_type = 'deploy'
           AND  user_address IS NOT NULL
           AND  DATE(interaction_timestamp) BETWEEN $2 AND $3",
    )
    .bind(contract_uuid)
    .bind(from_date)
    .bind(to_date)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_err("fetch unique deployers", err))?;

    let by_network_rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT network::TEXT, COALESCE(SUM(count), 0)::BIGINT AS total
         FROM   contract_interaction_daily_aggregates
         WHERE  contract_id = $1
           AND  interaction_type = 'deploy'
           AND  day BETWEEN $2 AND $3
         GROUP  BY network",
    )
    .bind(contract_uuid)
    .bind(from_date)
    .bind(to_date)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch deploy by network", err))?;

    let by_network: serde_json::Value =
        by_network_rows
            .into_iter()
            .fold(serde_json::json!({}), |mut acc, (net, count)| {
                acc[net] = serde_json::json!(count);
                acc
            });

    // ── Interactor stats ──────────────────────────────────────────────────────
    let unique_interactors: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT user_address)
         FROM   contract_interactions
         WHERE  contract_id = $1
           AND  user_address IS NOT NULL
           AND  DATE(interaction_timestamp) BETWEEN $2 AND $3",
    )
    .bind(contract_uuid)
    .bind(from_date)
    .bind(to_date)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_err("fetch unique interactors", err))?;

    let top_user_rows: Vec<(Option<String>, i64)> = sqlx::query_as(
        "SELECT user_address, COUNT(*) AS cnt
         FROM   contract_interactions
         WHERE  contract_id = $1
           AND  user_address IS NOT NULL
           AND  DATE(interaction_timestamp) BETWEEN $2 AND $3
         GROUP  BY user_address
         ORDER  BY cnt DESC
         LIMIT  10",
    )
    .bind(contract_uuid)
    .bind(from_date)
    .bind(to_date)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch top interactors", err))?;

    let top_users: Vec<TopUser> = top_user_rows
        .into_iter()
        .filter_map(|(addr, count)| addr.map(|a| TopUser { address: a, count }))
        .collect();

    // ── Error rate ────────────────────────────────────────────────────────────
    let total_in_range: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(count), 0)
         FROM   contract_interaction_daily_aggregates
         WHERE  contract_id = $1
           AND  day BETWEEN $2 AND $3",
    )
    .bind(contract_uuid)
    .bind(from_date)
    .bind(to_date)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_err("fetch total interactions", err))?;

    let failed_in_range: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(count), 0)
         FROM   contract_interaction_daily_aggregates
         WHERE  contract_id = $1
           AND  interaction_type = 'publish_failed'
           AND  day BETWEEN $2 AND $3",
    )
    .bind(contract_uuid)
    .bind(from_date)
    .bind(to_date)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_err("fetch failed interactions", err))?;

    let error_rate = if total_in_range > 0 {
        failed_in_range as f64 / total_in_range as f64
    } else {
        0.0
    };

    // ── Bucketed timeline ─────────────────────────────────────────────────────
    let trunc_expr = match bucket.as_str() {
        "weekly" => "DATE_TRUNC('week', day::TIMESTAMP)::DATE",
        "monthly" => "DATE_TRUNC('month', day::TIMESTAMP)::DATE",
        _ => "day", // daily
    };

    let timeline_sql = format!(
        r#"
        SELECT {trunc} AS bucket_date,
               COALESCE(SUM(count), 0)::BIGINT AS total_count
        FROM   contract_interaction_daily_aggregates
        WHERE  contract_id = $1
          AND  day BETWEEN $2 AND $3
        GROUP  BY bucket_date
        ORDER  BY bucket_date
        "#,
        trunc = trunc_expr
    );

    let timeline_rows: Vec<(NaiveDate, i64)> = sqlx::query_as(&timeline_sql)
        .bind(contract_uuid)
        .bind(from_date)
        .bind(to_date)
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_err("fetch bucketed timeline", err))?;

    let timeline: Vec<TrendPoint> = timeline_rows
        .into_iter()
        .map(|(date, count)| TrendPoint { date, count })
        .collect();

    // ── Fixed 7-day daily trend (always daily, always last 7 days) ────────────
    let seven_days_ago = Utc::now().date_naive() - Duration::days(7);
    let trend_rows: Vec<(NaiveDate, i64)> = sqlx::query_as(
        r#"
        SELECT d::DATE AS date,
               COALESCE(SUM(agg.count), 0)::BIGINT AS count
        FROM   generate_series($1::DATE, CURRENT_DATE, '1 day'::INTERVAL) d
        LEFT   JOIN contract_interaction_daily_aggregates agg
               ON  agg.contract_id = $2
               AND agg.day = d::DATE
        GROUP  BY d::DATE
        ORDER  BY d::DATE
        "#,
    )
    .bind(seven_days_ago)
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch 7-day trend", err))?;

    let interaction_trend: Vec<TrendPoint> = trend_rows
        .into_iter()
        .map(|(date, count)| TrendPoint { date, count })
        .collect();

    let resp = ContractAnalyticsResponse {
        contract_id: id,
        view_count,
        deployments: DeploymentStats {
            count: deploy_total,
            unique_users: unique_deployers,
            by_network,
        },
        interactors: InteractorStats {
            unique_count: unique_interactors,
            top_users,
        },
        timeline,
        interaction_trend,
        from_date,
        to_date,
        bucket: bucket.clone(),
        error_rate,
        cached: false,
    };

    // Cache historical results for 6 hours
    if is_historical {
        if let Ok(serialized) = serde_json::to_string(&resp) {
            state
                .cache
                .put(
                    "contract_analytics",
                    &cache_key,
                    serialized,
                    Some(StdDuration::from_secs(6 * 3600)),
                )
                .await;
        }
    }

    build_analytics_response(resp, &format)
}

fn build_analytics_response(
    resp: ContractAnalyticsResponse,
    format: &str,
) -> ApiResult<axum::response::Response> {
    if format == "csv" {
        let csv = render_analytics_csv(&resp);
        let body = axum::body::Body::from(csv);
        let response = axum::response::Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/csv; charset=utf-8"),
            )
            .header(
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&format!(
                    "attachment; filename=\"analytics_{}.csv\"",
                    resp.contract_id
                ))
                .unwrap_or(HeaderValue::from_static(
                    "attachment; filename=\"analytics.csv\"",
                )),
            )
            .body(body)
            .map_err(|_| ApiError::internal("Failed to build CSV response"))?;
        return Ok(response);
    }
    Ok(Json(resp).into_response())
}

fn render_analytics_csv(resp: &ContractAnalyticsResponse) -> String {
    let mut csv = String::new();
    csv.push_str(&format!("# contract_id={}\n", resp.contract_id));
    csv.push_str(&format!("# from_date={}\n", resp.from_date));
    csv.push_str(&format!("# to_date={}\n", resp.to_date));
    csv.push_str(&format!("# bucket={}\n", resp.bucket));
    csv.push_str(&format!("# total_deployments={}\n", resp.deployments.count));
    csv.push_str(&format!(
        "# unique_deployers={}\n",
        resp.deployments.unique_users
    ));
    csv.push_str(&format!(
        "# unique_interactors={}\n",
        resp.interactors.unique_count
    ));
    csv.push_str(&format!("# view_count={}\n", resp.view_count));
    csv.push_str(&format!("# error_rate={:.6}\n", resp.error_rate));
    csv.push_str("date,interactions\n");
    for point in &resp.timeline {
        csv.push_str(&format!("{},{}\n", point.date, point.count));
    }
    csv
}

/// Return registry-wide analytics aggregated by category and network.
///
/// Interaction totals are read from the daily-aggregate rollup so the query
/// is fast even on large registries.  This endpoint is the primary way to
/// identify which categories or networks are trending.
#[utoipa::path(
    get,
    path = "/api/analytics/summary",
    responses(
        (status = 200, description = "Registry analytics summary by category and network",
         body = AnalyticsSummaryResponse)
    ),
    tag = "Analytics"
)]
pub async fn get_analytics_summary(
    State(state): State<AppState>,
) -> ApiResult<Json<AnalyticsSummaryResponse>> {
    // ── By category ───────────────────────────────────────────────────────────

    let category_rows: Vec<(Option<String>, i64, i64, i64)> = sqlx::query_as(
        r#"
        SELECT
            c.category,
            COUNT(DISTINCT c.id)::BIGINT                   AS contract_count,
            COALESCE(SUM(agg.count), 0)::BIGINT            AS total_interactions,
            COALESCE(SUM(c.view_count), 0)::BIGINT         AS total_views
        FROM contracts c
        LEFT JOIN contract_interaction_daily_aggregates agg
               ON agg.contract_id = c.id
        GROUP BY c.category
        ORDER BY total_interactions DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch category analytics", err))?;

    let by_category: Vec<CategoryAnalytics> = category_rows
        .into_iter()
        .map(
            |(category, contract_count, total_interactions, total_views)| {
                let avg = if contract_count > 0 {
                    total_interactions as f64 / contract_count as f64
                } else {
                    0.0
                };
                CategoryAnalytics {
                    category,
                    contract_count,
                    total_interactions,
                    avg_interactions_per_contract: avg,
                    total_views,
                }
            },
        )
        .collect();

    // ── By network ────────────────────────────────────────────────────────────

    let network_rows: Vec<(String, i64, i64, i64, i64)> = sqlx::query_as(
        r#"
        SELECT
            c.network::TEXT,
            COUNT(DISTINCT c.id)::BIGINT                              AS contract_count,
            COUNT(DISTINCT c.id) FILTER (WHERE c.is_verified)::BIGINT AS verified_count,
            COALESCE(SUM(agg.total_count), 0)::BIGINT                 AS total_interactions,
            COALESCE(SUM(c.view_count), 0)::BIGINT                    AS total_views
        FROM contracts c
        LEFT JOIN (
            SELECT
                contract_id,
                SUM(count) AS total_count
            FROM contract_interaction_daily_aggregates
            GROUP BY contract_id
        ) agg
            ON agg.contract_id = c.id
        GROUP BY c.network
        ORDER BY total_interactions DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch network analytics", err))?;

    let by_network: Vec<NetworkAnalytics> = network_rows
        .into_iter()
        .map(
            |(network, contract_count, verified_count, total_interactions, total_views)| {
                NetworkAnalytics {
                    network,
                    contract_count,
                    verified_count,
                    total_interactions,
                    total_views,
                }
            },
        )
        .collect();

    // ── Top publishers ────────────────────────────────────────────────────────
    let publisher_rows: Vec<(Uuid, String, i64, i64)> = sqlx::query_as(
        r#"
        SELECT
            p.id,
            p.name,
            COUNT(c.id)::BIGINT           AS contract_count,
            COALESCE(SUM(c.view_count), 0)::BIGINT AS total_views
        FROM publishers p
        LEFT JOIN contracts c ON c.publisher_id = p.id
        GROUP BY p.id, p.name
        ORDER BY contract_count DESC
        LIMIT 10
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch top publishers", err))?;

    let top_publishers: Vec<PublisherAnalytics> = publisher_rows
        .into_iter()
        .map(
            |(publisher_id, name, contract_count, total_views)| PublisherAnalytics {
                publisher_id,
                name,
                contract_count,
                total_views,
            },
        )
        .collect();

    // ── Registry-wide deployment trends (last 30 days) ─────────────────────────
    let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);
    let trend_rows: Vec<(NaiveDate, i64)> = sqlx::query_as(
        r#"
        SELECT
            d::DATE AS date,
            COALESCE(SUM(agg.count), 0)::BIGINT AS count
        FROM generate_series(
            ($1::TIMESTAMPTZ)::DATE,
            CURRENT_DATE,
            '1 day'::INTERVAL
        ) d
        LEFT JOIN contract_interaction_daily_aggregates agg
               ON agg.day = d::DATE
              AND agg.interaction_type = 'deploy'
        GROUP BY d::DATE
        ORDER BY d::DATE
        "#,
    )
    .bind(thirty_days_ago)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch deployment trends", err))?;

    let deployment_trends: Vec<DeploymentTrendPoint> = trend_rows
        .into_iter()
        .map(|(date, count)| DeploymentTrendPoint { date, count })
        .collect();

    // ── Recent additions (last 10) ──────────────────────────────────────────
    let recent_rows: Vec<(
        Uuid,
        String,
        chrono::DateTime<chrono::Utc>,
        Option<String>,
        String,
        Option<String>,
        String,
    )> = sqlx::query_as(
        r#"
        SELECT
            c.id,
            c.name,
            c.created_at,
            p.name as publisher_name,
            c.network::TEXT,
            c.category,
            c.contract_id
        FROM contracts c
        LEFT JOIN publishers p ON p.id = c.publisher_id
        ORDER BY c.created_at DESC
        LIMIT 10
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch recent additions", err))?;

    let recent_additions: Vec<RecentContract> = recent_rows
        .into_iter()
        .map(
            |(id, name, created_at, publisher_name, network, category, contract_id)| {
                RecentContract {
                    id,
                    name,
                    created_at,
                    publisher_name,
                    network,
                    category,
                    contract_id,
                }
            },
        )
        .collect();

    Ok(Json(AnalyticsSummaryResponse {
        category_distribution: by_category,
        network_usage: by_network,
        top_publishers,
        deployment_trends,
        recent_additions,
    }))
}

#[utoipa::path(
    get,
    path = "/api/analytics/timeseries",
    params(AnalyticsTimeSeriesQuery),
    responses(
        (status = 200, description = "Daily analytics aggregates grouped by dimension", body = AnalyticsTimeSeriesResponse),
        (status = 400, description = "Invalid query parameters")
    ),
    tag = "Analytics"
)]
pub async fn get_analytics_timeseries(
    State(state): State<AppState>,
    Query(query): Query<AnalyticsTimeSeriesQuery>,
) -> ApiResult<Json<AnalyticsTimeSeriesResponse>> {
    let end_date = query.end_date.unwrap_or_else(|| Utc::now().date_naive());
    let start_date = query
        .start_date
        .unwrap_or_else(|| end_date - Duration::days(30));

    if start_date > end_date {
        return Err(ApiError::bad_request(
            "InvalidDateRange",
            "start_date must be less than or equal to end_date",
        ));
    }

    if (end_date - start_date).num_days() > 366 {
        return Err(ApiError::bad_request(
            "DateRangeTooLarge",
            "date range cannot exceed 366 days",
        ));
    }

    let group_by = TimeSeriesGroupBy::parse(query.group_by.as_deref())?;

    let rows: Vec<AnalyticsTimeSeriesRow> = match group_by {
        TimeSeriesGroupBy::Network => sqlx::query_as(
            r#"
                SELECT
                    a.date AS day,
                    c.network::TEXT AS group_key,
                    COALESCE(SUM(a.deployment_count), 0)::BIGINT AS deployments,
                    COALESCE(SUM(a.verification_count), 0)::BIGINT AS verifications,
                    COALESCE(SUM(a.update_count), 0)::BIGINT AS updates,
                    COALESCE(SUM(a.total_events), 0)::BIGINT AS total_events
                FROM analytics_daily_aggregates a
                JOIN contracts c ON c.id = a.contract_id
                WHERE a.date BETWEEN $1 AND $2
                  AND ($3::network_type IS NULL OR c.network = $3)
                  AND ($4::TEXT IS NULL OR c.category = $4)
                  AND ($5::UUID IS NULL OR c.publisher_id = $5)
                GROUP BY a.date, c.network
                ORDER BY a.date ASC, c.network::TEXT ASC
                "#,
        )
        .bind(start_date)
        .bind(end_date)
        .bind(query.network)
        .bind(query.category.as_deref())
        .bind(query.publisher_id)
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_err("fetch analytics timeseries grouped by network", err))?,
        TimeSeriesGroupBy::Category => sqlx::query_as(
            r#"
                SELECT
                    a.date AS day,
                    COALESCE(c.category, 'uncategorized') AS group_key,
                    COALESCE(SUM(a.deployment_count), 0)::BIGINT AS deployments,
                    COALESCE(SUM(a.verification_count), 0)::BIGINT AS verifications,
                    COALESCE(SUM(a.update_count), 0)::BIGINT AS updates,
                    COALESCE(SUM(a.total_events), 0)::BIGINT AS total_events
                FROM analytics_daily_aggregates a
                JOIN contracts c ON c.id = a.contract_id
                WHERE a.date BETWEEN $1 AND $2
                  AND ($3::network_type IS NULL OR c.network = $3)
                  AND ($4::TEXT IS NULL OR c.category = $4)
                  AND ($5::UUID IS NULL OR c.publisher_id = $5)
                GROUP BY a.date, COALESCE(c.category, 'uncategorized')
                ORDER BY a.date ASC, group_key ASC
                "#,
        )
        .bind(start_date)
        .bind(end_date)
        .bind(query.network)
        .bind(query.category.as_deref())
        .bind(query.publisher_id)
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_err("fetch analytics timeseries grouped by category", err))?,
        TimeSeriesGroupBy::Publisher => sqlx::query_as(
            r#"
                SELECT
                    a.date AS day,
                    COALESCE(p.stellar_address, c.publisher_id::TEXT) AS group_key,
                    COALESCE(SUM(a.deployment_count), 0)::BIGINT AS deployments,
                    COALESCE(SUM(a.verification_count), 0)::BIGINT AS verifications,
                    COALESCE(SUM(a.update_count), 0)::BIGINT AS updates,
                    COALESCE(SUM(a.total_events), 0)::BIGINT AS total_events
                FROM analytics_daily_aggregates a
                JOIN contracts c ON c.id = a.contract_id
                LEFT JOIN publishers p ON p.id = c.publisher_id
                WHERE a.date BETWEEN $1 AND $2
                  AND ($3::network_type IS NULL OR c.network = $3)
                  AND ($4::TEXT IS NULL OR c.category = $4)
                  AND ($5::UUID IS NULL OR c.publisher_id = $5)
                GROUP BY a.date, COALESCE(p.stellar_address, c.publisher_id::TEXT)
                ORDER BY a.date ASC, group_key ASC
                "#,
        )
        .bind(start_date)
        .bind(end_date)
        .bind(query.network)
        .bind(query.category.as_deref())
        .bind(query.publisher_id)
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_err("fetch analytics timeseries grouped by publisher", err))?,
    };

    let mut by_group: HashMap<String, BTreeMap<NaiveDate, AnalyticsTimeSeriesPoint>> =
        HashMap::new();

    for row in rows {
        by_group.entry(row.group_key).or_default().insert(
            row.day,
            AnalyticsTimeSeriesPoint {
                date: row.day,
                deployments: row.deployments,
                verifications: row.verifications,
                updates: row.updates,
                total_events: row.total_events,
            },
        );
    }

    let mut series: Vec<AnalyticsTimeSeriesGroup> = by_group
        .into_iter()
        .map(|(key, date_map)| {
            let mut cursor = start_date;
            let mut points = Vec::new();

            while cursor <= end_date {
                if let Some(point) = date_map.get(&cursor) {
                    points.push(AnalyticsTimeSeriesPoint {
                        date: point.date,
                        deployments: point.deployments,
                        verifications: point.verifications,
                        updates: point.updates,
                        total_events: point.total_events,
                    });
                } else {
                    points.push(AnalyticsTimeSeriesPoint {
                        date: cursor,
                        deployments: 0,
                        verifications: 0,
                        updates: 0,
                        total_events: 0,
                    });
                }

                cursor = cursor.succ_opt().expect("valid calendar date");
            }

            AnalyticsTimeSeriesGroup { key, points }
        })
        .collect();

    series.sort_by(|a, b| a.key.cmp(&b.key));

    Ok(Json(AnalyticsTimeSeriesResponse {
        start_date,
        end_date,
        group_by: group_by.as_str().to_string(),
        series,
    }))
}

// ── Unit tests ────────────────────────────────────────────────────────────────
mod tests {
    use super::*;

    #[test]
    fn parse_contract_id_rejects_non_uuid() {
        assert!(parse_contract_id("not-a-uuid").is_err());
    }

    #[test]
    fn parse_contract_id_accepts_valid_uuid() {
        let id = Uuid::new_v4().to_string();
        assert!(parse_contract_id(&id).is_ok());
    }

    #[test]
    fn category_analytics_avg_is_zero_for_empty_category() {
        let analytics = CategoryAnalytics {
            category: None,
            contract_count: 0,
            total_interactions: 0,
            avg_interactions_per_contract: {
                let contract_count: i64 = 0;
                let total_interactions: i64 = 0;
                if contract_count > 0 {
                    total_interactions as f64 / contract_count as f64
                } else {
                    0.0
                }
            },
            total_views: 0,
        };
        assert_eq!(analytics.avg_interactions_per_contract, 0.0);
    }

    #[test]
    fn category_analytics_avg_computed_correctly() {
        let contract_count: i64 = 4;
        let total_interactions: i64 = 100;
        let avg = total_interactions as f64 / contract_count as f64;
        assert_eq!(avg, 25.0);
    }
}

// ── Dashboard endpoint ────────────────────────────────────────────────────────

/// Query parameters for the dashboard endpoint.
#[derive(Debug, Deserialize)]
pub struct DashboardParams {
    /// Number of recent additions to return (default: 10, max: 50).
    pub limit: Option<i64>,
    /// Preset time window: 7d, 30d, 90d, custom (default: 30d).
    pub timeframe: Option<String>,
    /// Inclusive start date (YYYY-MM-DD) for custom windows.
    pub start_date: Option<NaiveDate>,
    /// Inclusive end date (YYYY-MM-DD), defaults to today (UTC).
    pub end_date: Option<NaiveDate>,
    /// Optional network filter.
    pub network: Option<Network>,
    /// Optional category filter.
    pub category: Option<String>,
    /// Optional verification-status filter.
    pub verified: Option<bool>,
    /// Number of top contracts to return (default: 10, max: 50).
    pub top_limit: Option<i64>,
}

/// One entry in the network_usage array returned by the dashboard endpoint.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct NetworkUsageEntry {
    pub network: String,
    pub count: i64,
}

/// One entry in the category_distribution array returned by the dashboard endpoint.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct CategoryDistributionEntry {
    pub category: String,
    pub count: i64,
}

/// One entry in the deployment_trends array returned by the dashboard endpoint.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct DeploymentTrendEntry {
    pub date: NaiveDate,
    pub count: i64,
}

/// One entry in the interaction_trends array returned by the dashboard endpoint.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct InteractionTrendEntry {
    pub date: NaiveDate,
    pub count: i64,
}

/// One entry in the recent_additions array returned by the dashboard endpoint.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RecentAdditionEntry {
    pub id: Uuid,
    pub contract_id: String,
    pub name: String,
    pub network: String,
    pub category: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// One entry in the top_contracts array returned by the dashboard endpoint.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TopContractEntry {
    pub id: Uuid,
    pub contract_id: String,
    pub name: String,
    pub network: String,
    pub category: Option<String>,
    pub is_verified: bool,
    pub interaction_count: i64,
}

/// Response shape for GET /api/analytics/dashboard.
///
/// This is the exact shape consumed by the frontend analytics page at /analytics.
/// Fields:
/// - `recent_additions` → `RecentAdditionsTimeline` and `DeploymentTimeline` components
/// - `network_usage`    → `NetworkUsageStats` component  (expects `[{network, count}]`)
/// - `category_distribution` → `CategoryDistributionPie` (expects `[{category, count}]`)
/// - `deployment_trends`     → `DeploymentTrendGraph`    (expects `[{date, count}]`)
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AnalyticsDashboardResponse {
    pub total_contracts: i64,
    pub active_deployments: i64,
    pub this_month_interactions: i64,
    pub time_range_start: NaiveDate,
    pub time_range_end: NaiveDate,
    pub recent_additions: Vec<RecentAdditionEntry>,
    pub network_usage: Vec<NetworkUsageEntry>,
    pub category_distribution: Vec<CategoryDistributionEntry>,
    pub deployment_trends: Vec<DeploymentTrendEntry>,
    pub interaction_trends: Vec<InteractionTrendEntry>,
    pub top_contracts: Vec<TopContractEntry>,
}

/// Return the analytics dashboard data consumed by the frontend /analytics page.
///
/// This endpoint is registered at GET /api/analytics/dashboard and returns the
/// exact response shape the frontend expects.  All four data sets are fetched
/// in parallel-friendly sequential queries that use existing indexes.
#[utoipa::path(
    get,
    path = "/api/analytics/dashboard",
    params(
        ("limit" = Option<i64>, Query, description = "Number of recent additions (default 10, max 50)"),
        ("top_limit" = Option<i64>, Query, description = "Number of top contracts to return (default 10, max 50)"),
        ("timeframe" = Option<String>, Query, description = "Range preset: 7d, 30d, 90d, custom"),
        ("start_date" = Option<String>, Query, description = "Custom range start (YYYY-MM-DD), used when timeframe=custom"),
        ("end_date" = Option<String>, Query, description = "Custom range end (YYYY-MM-DD), defaults to current UTC date"),
        ("network" = Option<String>, Query, description = "Optional network filter: mainnet, testnet, futurenet"),
        ("category" = Option<String>, Query, description = "Optional category filter"),
        ("verified" = Option<bool>, Query, description = "Optional verification filter")
    ),
    responses(
        (status = 200, description = "Analytics dashboard data", body = AnalyticsDashboardResponse)
    ),
    tag = "Analytics"
)]
pub async fn get_analytics_dashboard(
    State(state): State<AppState>,
    Query(params): Query<DashboardParams>,
) -> ApiResult<Json<AnalyticsDashboardResponse>> {
    let limit = params.limit.unwrap_or(10).clamp(1, 50);
    let top_limit = params.top_limit.unwrap_or(10).clamp(1, 50);
    let end_date = params.end_date.unwrap_or_else(|| Utc::now().date_naive());

    let timeframe = params.timeframe.unwrap_or_else(|| "30d".to_string());
    let start_date = match timeframe.as_str() {
        "7d" => end_date - Duration::days(6),
        "30d" => end_date - Duration::days(29),
        "90d" => end_date - Duration::days(89),
        "custom" => params.start_date.unwrap_or(end_date - Duration::days(29)),
        _ => {
            return Err(ApiError::bad_request(
                "InvalidTimeframe",
                "timeframe must be one of: 7d, 30d, 90d, custom",
            ));
        }
    };

    if start_date > end_date {
        return Err(ApiError::bad_request(
            "InvalidDateRange",
            "start_date must be less than or equal to end_date",
        ));
    }

    if (end_date - start_date).num_days() > 366 {
        return Err(ApiError::bad_request(
            "DateRangeTooLarge",
            "date range cannot exceed 366 days",
        ));
    }

    // Summary metrics for cards.
    let total_contracts: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM contracts c
        WHERE ($1::network_type IS NULL OR c.network = $1)
          AND ($2::TEXT IS NULL OR c.category = $2)
          AND ($3::BOOL IS NULL OR c.is_verified = $3)
        "#,
    )
    .bind(params.network.clone())
    .bind(params.category.as_deref())
    .bind(params.verified)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_err("fetch total contracts", err))?;

    let active_deployments: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(DISTINCT agg.contract_id)::BIGINT
        FROM contract_interaction_daily_aggregates agg
        JOIN contracts c ON c.id = agg.contract_id
        WHERE agg.day BETWEEN $1 AND $2
          AND agg.interaction_type = 'deploy'
          AND ($3::network_type IS NULL OR c.network = $3)
          AND ($4::TEXT IS NULL OR c.category = $4)
          AND ($5::BOOL IS NULL OR c.is_verified = $5)
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .bind(params.network.clone())
    .bind(params.category.as_deref())
    .bind(params.verified)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_err("fetch active deployments", err))?;

    let month_start = end_date.with_day(1).unwrap_or(end_date);
    let this_month_interactions: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(agg.count), 0)::BIGINT
        FROM contract_interaction_daily_aggregates agg
        JOIN contracts c ON c.id = agg.contract_id
        WHERE agg.day BETWEEN $1 AND $2
          AND ($3::network_type IS NULL OR c.network = $3)
          AND ($4::TEXT IS NULL OR c.category = $4)
          AND ($5::BOOL IS NULL OR c.is_verified = $5)
        "#,
    )
    .bind(month_start)
    .bind(end_date)
    .bind(params.network.clone())
    .bind(params.category.as_deref())
    .bind(params.verified)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_err("fetch this month interactions", err))?;

    // ── Recent additions (newest contracts first) ─────────────────────────────
    let recent_rows: Vec<(
        Uuid,
        String,
        String,
        String,
        Option<String>,
        chrono::DateTime<chrono::Utc>,
    )> = sqlx::query_as(
        r#"
            SELECT id, contract_id, name, network::TEXT, category, created_at
            FROM   contracts
            WHERE  ($2::network_type IS NULL OR network = $2)
              AND  ($3::TEXT IS NULL OR category = $3)
              AND  ($4::BOOL IS NULL OR is_verified = $4)
            ORDER  BY created_at DESC
            LIMIT  $1
            "#,
    )
    .bind(limit)
    .bind(params.network.clone())
    .bind(params.category.as_deref())
    .bind(params.verified)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch recent additions", err))?;

    let recent_additions: Vec<RecentAdditionEntry> = recent_rows
        .into_iter()
        .map(
            |(id, contract_id, name, network, category, created_at)| RecentAdditionEntry {
                id,
                contract_id,
                name,
                network,
                category,
                created_at,
            },
        )
        .collect();

    // ── Network usage (contract counts per network) ───────────────────────────
    let network_rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT network::TEXT, COUNT(*)::BIGINT AS count
        FROM   contracts
                WHERE  ($1::network_type IS NULL OR network = $1)
                    AND  ($2::TEXT IS NULL OR category = $2)
                    AND  ($3::BOOL IS NULL OR is_verified = $3)
        GROUP  BY network
        ORDER  BY count DESC
        "#,
    )
    .bind(params.network.clone())
    .bind(params.category.as_deref())
    .bind(params.verified)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch network usage", err))?;

    let network_usage: Vec<NetworkUsageEntry> = network_rows
        .into_iter()
        .map(|(network, count)| NetworkUsageEntry { network, count })
        .collect();

    // ── Category distribution (top 10 categories by contract count) ───────────
    let category_rows: Vec<(Option<String>, i64)> = sqlx::query_as(
        r#"
        SELECT COALESCE(category, 'Uncategorized') AS category,
               COUNT(*)::BIGINT AS count
        FROM   contracts
                WHERE  ($1::network_type IS NULL OR network = $1)
                    AND  ($2::TEXT IS NULL OR category = $2)
                    AND  ($3::BOOL IS NULL OR is_verified = $3)
        GROUP  BY category
        ORDER  BY count DESC
        LIMIT  10
        "#,
    )
    .bind(params.network.clone())
    .bind(params.category.as_deref())
    .bind(params.verified)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch category distribution", err))?;

    let category_distribution: Vec<CategoryDistributionEntry> = category_rows
        .into_iter()
        .map(|(category, count)| CategoryDistributionEntry {
            category: category.unwrap_or_else(|| "Uncategorized".to_string()),
            count,
        })
        .collect();

    // ── Deployment trends (daily deploy counts for last 30 days) ─────────────
    // Uses the daily-aggregate rollup for 'deploy' interactions so the query
    // is fast even on large registries.  generate_series fills in zero-count
    // days so the chart always has a continuous 30-day series.
    let trend_rows: Vec<(NaiveDate, i64)> = sqlx::query_as(
        r#"
        SELECT d::DATE AS date,
                             COALESCE(t.count, 0)::BIGINT AS count
        FROM   generate_series(
                                     $1::DATE,
                                     $2::DATE,
                   '1 day'::INTERVAL
               ) d
                LEFT JOIN (
                    SELECT agg.day, SUM(agg.count)::BIGINT AS count
                    FROM contract_interaction_daily_aggregates agg
                    JOIN contracts c ON c.id = agg.contract_id
                    WHERE agg.day BETWEEN $1 AND $2
                        AND agg.interaction_type = 'deploy'
                        AND ($3::network_type IS NULL OR c.network = $3)
                        AND ($4::TEXT IS NULL OR c.category = $4)
                        AND ($5::BOOL IS NULL OR c.is_verified = $5)
                    GROUP BY agg.day
                ) t ON t.day = d::DATE
        GROUP  BY d::DATE
        ORDER  BY d::DATE
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .bind(params.network.clone())
    .bind(params.category.as_deref())
    .bind(params.verified)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch deployment trends", err))?;

    let deployment_trends: Vec<DeploymentTrendEntry> = trend_rows
        .into_iter()
        .map(|(date, count)| DeploymentTrendEntry { date, count })
        .collect();

    let interaction_rows: Vec<(NaiveDate, i64)> = sqlx::query_as(
        r#"
        SELECT d::DATE AS date,
               COALESCE(t.count, 0)::BIGINT AS count
        FROM   generate_series(
                   $1::DATE,
                   $2::DATE,
                   '1 day'::INTERVAL
               ) d
        LEFT JOIN (
          SELECT agg.day, SUM(agg.count)::BIGINT AS count
          FROM contract_interaction_daily_aggregates agg
          JOIN contracts c ON c.id = agg.contract_id
          WHERE agg.day BETWEEN $1 AND $2
            AND ($3::network_type IS NULL OR c.network = $3)
            AND ($4::TEXT IS NULL OR c.category = $4)
            AND ($5::BOOL IS NULL OR c.is_verified = $5)
          GROUP BY agg.day
        ) t ON t.day = d::DATE
        GROUP  BY d::DATE
        ORDER  BY d::DATE
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .bind(params.network.clone())
    .bind(params.category.as_deref())
    .bind(params.verified)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch interaction trends", err))?;

    let interaction_trends: Vec<InteractionTrendEntry> = interaction_rows
        .into_iter()
        .map(|(date, count)| InteractionTrendEntry { date, count })
        .collect();

    let top_rows: Vec<(Uuid, String, String, String, Option<String>, bool, i64)> = sqlx::query_as(
        r#"
        SELECT
          c.id,
          c.contract_id,
          c.name,
          c.network::TEXT,
          c.category,
          c.is_verified,
          COALESCE(SUM(agg.count), 0)::BIGINT AS interaction_count
        FROM contracts c
        LEFT JOIN contract_interaction_daily_aggregates agg
          ON agg.contract_id = c.id
         AND agg.day BETWEEN $1 AND $2
        WHERE ($3::network_type IS NULL OR c.network = $3)
          AND ($4::TEXT IS NULL OR c.category = $4)
          AND ($5::BOOL IS NULL OR c.is_verified = $5)
        GROUP BY c.id, c.contract_id, c.name, c.network, c.category, c.is_verified
        ORDER BY interaction_count DESC, c.created_at DESC
        LIMIT $6
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .bind(params.network.clone())
    .bind(params.category.as_deref())
    .bind(params.verified)
    .bind(top_limit)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch top contracts", err))?;

    let top_contracts: Vec<TopContractEntry> = top_rows
        .into_iter()
        .map(
            |(id, contract_id, name, network, category, is_verified, interaction_count)| {
                TopContractEntry {
                    id,
                    contract_id,
                    name,
                    network,
                    category,
                    is_verified,
                    interaction_count,
                }
            },
        )
        .collect();

    Ok(Json(AnalyticsDashboardResponse {
        total_contracts,
        active_deployments,
        this_month_interactions,
        time_range_start: start_date,
        time_range_end: end_date,
        recent_additions,
        network_usage,
        category_distribution,
        deployment_trends,
        interaction_trends,
        top_contracts,
    }))
}
