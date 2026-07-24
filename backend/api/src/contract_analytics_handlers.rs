use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use std::time::Duration as StdDuration;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ContractAnalyticsQuery {
    /// Inclusive start date (YYYY-MM-DD). Defaults to 30 days before `until`.
    pub since: Option<NaiveDate>,
    /// Inclusive end date (YYYY-MM-DD). Defaults to today (UTC).
    pub until: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractAnalyticsResponse {
    pub since: NaiveDate,
    pub until: NaiveDate,
    pub previous_since: NaiveDate,
    pub previous_until: NaiveDate,
    /// Contracts created in the selected time range.
    pub total_contracts: i64,
    pub growth: GrowthMetrics,
    pub by_network: Vec<NetworkBreakdownEntry>,
    pub network_stats: Vec<NetworkStatsEntry>,
    pub by_category: Vec<CategoryBreakdownEntry>,
    pub trending_categories: Vec<CategoryBreakdownEntry>,
    pub time_series: Vec<NewContractSeriesPoint>,
    pub popular_contracts: Vec<PopularContractEntry>,
    pub generated_at: DateTime<Utc>,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GrowthMetrics {
    pub current_period_contracts: i64,
    pub previous_period_contracts: i64,
    pub contract_growth: i64,
    pub contract_growth_rate: f64,
    pub current_period_deployments: i64,
    pub previous_period_deployments: i64,
    pub deployment_growth: i64,
    pub deployment_growth_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NetworkBreakdownEntry {
    pub network: String,
    pub contract_count: i64,
    pub verified_count: i64,
    pub previous_contract_count: i64,
    pub contract_growth: i64,
    pub contract_growth_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NetworkStatsEntry {
    pub network: String,
    pub active_contracts: i64,
    pub total_deployments: i64,
    pub total_interactions: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CategoryBreakdownEntry {
    pub category: String,
    pub contract_count: i64,
    pub verified_count: i64,
    pub previous_contract_count: i64,
    pub contract_growth: i64,
    pub contract_growth_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NewContractSeriesPoint {
    pub date: NaiveDate,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PopularContractEntry {
    pub id: Uuid,
    pub contract_id: String,
    pub name: String,
    pub network: String,
    pub category: Option<String>,
    pub is_verified: bool,
    pub interaction_count: i64,
    pub deployment_count: i64,
}

#[derive(Debug, FromRow)]
struct NetworkBreakdownRow {
    network: String,
    contract_count: i64,
    verified_count: i64,
}

#[derive(Debug, FromRow)]
struct NetworkStatsRow {
    network: String,
    active_contracts: i64,
    total_deployments: i64,
    total_interactions: i64,
}

#[derive(Debug, FromRow)]
struct CategoryBreakdownRow {
    category: String,
    contract_count: i64,
    verified_count: i64,
}

#[derive(Debug, FromRow)]
struct PopularContractRow {
    id: Uuid,
    contract_id: String,
    name: String,
    network: String,
    category: Option<String>,
    is_verified: bool,
    interaction_count: i64,
    deployment_count: i64,
}

fn db_err(op: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = op, error = ?err, "database operation failed");
    ApiError::internal("An unexpected database error occurred")
}

fn growth_rate(current: i64, previous: i64) -> f64 {
    if previous == 0 {
        return if current > 0 { 100.0 } else { 0.0 };
    }

    ((current - previous) as f64 / previous as f64) * 100.0
}

fn resolve_ranges(
    query: &ContractAnalyticsQuery,
) -> ApiResult<(NaiveDate, NaiveDate, NaiveDate, NaiveDate, NaiveDate)> {
    let until = query.until.unwrap_or_else(|| Utc::now().date_naive());
    let since = query.since.unwrap_or_else(|| until - Duration::days(29));

    if since > until {
        return Err(ApiError::bad_request(
            "InvalidDateRange",
            "since must be less than or equal to until",
        ));
    }

    let period_days = (until - since).num_days() + 1;
    let previous_until = since - Duration::days(1);
    let previous_since = previous_until - Duration::days(period_days - 1);
    let time_series_since = std::cmp::max(since, until - Duration::days(29));

    Ok((
        since,
        until,
        previous_since,
        previous_until,
        time_series_since,
    ))
}

/// GET /api/v1/analytics/contracts
///
/// Returns registry-wide contract analytics, including growth, time-series,
/// popular contracts, and per-network/category breakdowns.
#[utoipa::path(
    get,
    path = "/api/v1/analytics/contracts",
    params(ContractAnalyticsQuery),
    responses(
        (status = 200, description = "Contract analytics summary", body = ContractAnalyticsResponse),
        (status = 400, description = "Invalid parameters")
    ),
    tag = "Analytics"
)]
pub async fn get_contract_analytics(
    State(state): State<AppState>,
    Query(query): Query<ContractAnalyticsQuery>,
) -> ApiResult<Json<ContractAnalyticsResponse>> {
    let (since, until, previous_since, previous_until, time_series_since) = resolve_ranges(&query)?;
    let cache_key = format!("{}:{}", since, until);

    let (cached_body, hit) = state.cache.get("analytics_contracts", &cache_key).await;
    if hit {
        if let Some(raw) = cached_body {
            if let Ok(mut response) = serde_json::from_str::<ContractAnalyticsResponse>(&raw) {
                response.cached = true;
                return Ok(Json(response));
            }
        }
    }

    let total_contracts: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM contracts
        WHERE DATE(created_at) BETWEEN $1 AND $2
        "#,
    )
    .bind(since)
    .bind(until)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_err("fetch total contracts", err))?;

    let current_contracts: i64 = total_contracts;

    let previous_contracts: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM contracts
        WHERE DATE(created_at) BETWEEN $1 AND $2
        "#,
    )
    .bind(previous_since)
    .bind(previous_until)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_err("fetch previous contracts", err))?;

    let current_deployments: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(count), 0)::BIGINT
        FROM contract_interaction_daily_aggregates
        WHERE day BETWEEN $1 AND $2
          AND interaction_type = 'deploy'
        "#,
    )
    .bind(since)
    .bind(until)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_err("fetch current deployments", err))?;

    let previous_deployments: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(count), 0)::BIGINT
        FROM contract_interaction_daily_aggregates
        WHERE day BETWEEN $1 AND $2
          AND interaction_type = 'deploy'
        "#,
    )
    .bind(previous_since)
    .bind(previous_until)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_err("fetch previous deployments", err))?;

    let growth = GrowthMetrics {
        current_period_contracts: current_contracts,
        previous_period_contracts: previous_contracts,
        contract_growth: current_contracts - previous_contracts,
        contract_growth_rate: growth_rate(current_contracts, previous_contracts),
        current_period_deployments: current_deployments,
        previous_period_deployments: previous_deployments,
        deployment_growth: current_deployments - previous_deployments,
        deployment_growth_rate: growth_rate(current_deployments, previous_deployments),
    };

    let current_network_rows: Vec<NetworkBreakdownRow> = sqlx::query_as(
        r#"
        SELECT
            c.network::TEXT AS network,
            COUNT(*)::BIGINT AS contract_count,
            COUNT(*) FILTER (WHERE c.is_verified)::BIGINT AS verified_count
        FROM contracts c
        WHERE DATE(c.created_at) BETWEEN $1 AND $2
        GROUP BY c.network
        ORDER BY contract_count DESC, c.network::TEXT ASC
        "#,
    )
    .bind(since)
    .bind(until)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch network breakdown", err))?;

    let previous_network_rows: Vec<NetworkBreakdownRow> = sqlx::query_as(
        r#"
        SELECT
            c.network::TEXT AS network,
            COUNT(*)::BIGINT AS contract_count,
            COUNT(*) FILTER (WHERE c.is_verified)::BIGINT AS verified_count
        FROM contracts c
        WHERE DATE(c.created_at) BETWEEN $1 AND $2
        GROUP BY c.network
        ORDER BY contract_count DESC, c.network::TEXT ASC
        "#,
    )
    .bind(previous_since)
    .bind(previous_until)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch previous network breakdown", err))?;

    let mut previous_network_counts = HashMap::new();
    for row in previous_network_rows {
        previous_network_counts.insert(row.network.clone(), row);
    }

    let by_network = current_network_rows
        .into_iter()
        .map(|row| {
            let previous = previous_network_counts
                .remove(&row.network)
                .map(|entry| entry.contract_count)
                .unwrap_or(0);
            let contract_growth = row.contract_count - previous;

            NetworkBreakdownEntry {
                network: row.network,
                contract_count: row.contract_count,
                verified_count: row.verified_count,
                previous_contract_count: previous,
                contract_growth,
                contract_growth_rate: growth_rate(row.contract_count, previous),
            }
        })
        .collect::<Vec<_>>();

    let network_stats_rows: Vec<NetworkStatsRow> = sqlx::query_as(
        r#"
        SELECT
            c.network::TEXT AS network,
            COUNT(DISTINCT c.id) FILTER (WHERE activity.contract_id IS NOT NULL)::BIGINT AS active_contracts,
            COALESCE(SUM(COALESCE(activity.total_deployments, 0)), 0)::BIGINT AS total_deployments,
            COALESCE(SUM(COALESCE(activity.total_interactions, 0)), 0)::BIGINT AS total_interactions
        FROM contracts c
        LEFT JOIN (
            SELECT
                contract_id,
                SUM(count) FILTER (WHERE interaction_type = 'deploy') AS total_deployments,
                SUM(count) AS total_interactions
            FROM contract_interaction_daily_aggregates
            WHERE day BETWEEN $1 AND $2
            GROUP BY contract_id
        ) activity ON activity.contract_id = c.id
        GROUP BY c.network
        ORDER BY total_deployments DESC, active_contracts DESC, c.network::TEXT ASC
        "#,
    )
    .bind(since)
    .bind(until)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch network stats", err))?;

    let network_stats = network_stats_rows
        .into_iter()
        .map(|row| NetworkStatsEntry {
            network: row.network,
            active_contracts: row.active_contracts,
            total_deployments: row.total_deployments,
            total_interactions: row.total_interactions,
        })
        .collect::<Vec<_>>();

    let current_category_rows: Vec<CategoryBreakdownRow> = sqlx::query_as(
        r#"
        SELECT
            COALESCE(c.category, 'Uncategorized') AS category,
            COUNT(*)::BIGINT AS contract_count,
            COUNT(*) FILTER (WHERE c.is_verified)::BIGINT AS verified_count
        FROM contracts c
        WHERE DATE(c.created_at) BETWEEN $1 AND $2
        GROUP BY COALESCE(c.category, 'Uncategorized')
        ORDER BY contract_count DESC, category ASC
        "#,
    )
    .bind(since)
    .bind(until)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch category breakdown", err))?;

    let previous_category_rows: Vec<CategoryBreakdownRow> = sqlx::query_as(
        r#"
        SELECT
            COALESCE(c.category, 'Uncategorized') AS category,
            COUNT(*)::BIGINT AS contract_count,
            COUNT(*) FILTER (WHERE c.is_verified)::BIGINT AS verified_count
        FROM contracts c
        WHERE DATE(c.created_at) BETWEEN $1 AND $2
        GROUP BY COALESCE(c.category, 'Uncategorized')
        ORDER BY contract_count DESC, category ASC
        "#,
    )
    .bind(previous_since)
    .bind(previous_until)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch previous category breakdown", err))?;

    let mut previous_category_counts = HashMap::new();
    for row in previous_category_rows {
        previous_category_counts.insert(row.category.clone(), row);
    }

    let mut by_category = current_category_rows
        .into_iter()
        .map(|row| {
            let previous = previous_category_counts
                .remove(&row.category)
                .map(|entry| entry.contract_count)
                .unwrap_or(0);
            let contract_growth = row.contract_count - previous;

            CategoryBreakdownEntry {
                category: row.category,
                contract_count: row.contract_count,
                verified_count: row.verified_count,
                previous_contract_count: previous,
                contract_growth,
                contract_growth_rate: growth_rate(row.contract_count, previous),
            }
        })
        .collect::<Vec<_>>();

    let mut trending_categories = by_category.clone();
    trending_categories.sort_by(|a, b| {
        b.contract_growth_rate
            .partial_cmp(&a.contract_growth_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.contract_growth.cmp(&a.contract_growth))
            .then_with(|| b.contract_count.cmp(&a.contract_count))
    });
    trending_categories.truncate(10);

    by_category.sort_by(|a, b| {
        b.contract_count
            .cmp(&a.contract_count)
            .then_with(|| a.category.cmp(&b.category))
    });

    let time_series_rows: Vec<(NaiveDate, i64)> = sqlx::query_as(
        r#"
        SELECT d::DATE AS date, COALESCE(t.count, 0)::BIGINT AS count
        FROM generate_series($1::DATE, $2::DATE, '1 day'::INTERVAL) d
        LEFT JOIN (
            SELECT DATE(created_at) AS day, COUNT(*)::BIGINT AS count
            FROM contracts
            WHERE DATE(created_at) BETWEEN $1 AND $2
            GROUP BY DATE(created_at)
        ) t ON t.day = d::DATE
        GROUP BY d::DATE
        ORDER BY d::DATE
        "#,
    )
    .bind(time_series_since)
    .bind(until)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch contract time series", err))?;

    let time_series = time_series_rows
        .into_iter()
        .map(|(date, count)| NewContractSeriesPoint { date, count })
        .collect::<Vec<_>>();

    let popular_rows: Vec<PopularContractRow> = sqlx::query_as(
        r#"
        SELECT
            c.id,
            c.contract_id,
            c.name,
            c.network::TEXT AS network,
            c.category,
            c.is_verified,
            COALESCE(SUM(activity.total_interactions), 0)::BIGINT AS interaction_count,
            COALESCE(SUM(activity.deployments), 0)::BIGINT AS deployment_count
        FROM contracts c
        LEFT JOIN (
            SELECT
                contract_id,
                SUM(count) AS total_interactions,
                SUM(count) FILTER (WHERE interaction_type = 'deploy') AS deployments
            FROM contract_interaction_daily_aggregates
            WHERE day BETWEEN $1 AND $2
            GROUP BY contract_id
        ) activity ON activity.contract_id = c.id
        GROUP BY c.id, c.contract_id, c.name, c.network, c.category, c.is_verified, c.created_at
        ORDER BY interaction_count DESC, deployment_count DESC, c.created_at DESC
        LIMIT 10
        "#,
    )
    .bind(since)
    .bind(until)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_err("fetch popular contracts", err))?;

    let popular_contracts = popular_rows
        .into_iter()
        .map(|row| PopularContractEntry {
            id: row.id,
            contract_id: row.contract_id,
            name: row.name,
            network: row.network,
            category: row.category,
            is_verified: row.is_verified,
            interaction_count: row.interaction_count,
            deployment_count: row.deployment_count,
        })
        .collect::<Vec<_>>();

    let response = ContractAnalyticsResponse {
        since,
        until,
        previous_since,
        previous_until,
        total_contracts,
        growth,
        by_network,
        network_stats,
        by_category,
        trending_categories,
        time_series,
        popular_contracts,
        generated_at: Utc::now(),
        cached: false,
    };

    if let Ok(serialized) = serde_json::to_string(&response) {
        state
            .cache
            .put(
                "analytics_contracts",
                &cache_key,
                serialized,
                Some(StdDuration::from_secs(3600)),
            )
            .await;
    }

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_rate_handles_zero_previous_values() {
        assert_eq!(growth_rate(0, 0), 0.0);
        assert_eq!(growth_rate(5, 0), 100.0);
    }

    #[test]
    fn growth_rate_calculates_percent_change() {
        assert_eq!(growth_rate(15, 10), 50.0);
        assert_eq!(growth_rate(5, 10), -50.0);
    }

    #[test]
    fn resolve_ranges_defaults_to_last_30_days() {
        let query = ContractAnalyticsQuery {
            since: None,
            until: None,
        };

        let (since, until, previous_since, previous_until, series_since) =
            resolve_ranges(&query).expect("range should resolve");

        assert_eq!(until, Utc::now().date_naive());
        assert_eq!(since, until - Duration::days(29));
        assert_eq!(previous_until, since - Duration::days(1));
        assert_eq!(previous_since, previous_until - Duration::days(29));
        assert_eq!(series_since, since);
    }
}
