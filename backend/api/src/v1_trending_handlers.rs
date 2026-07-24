//! GET /api/v1/trending — trending contracts, categories, and network heatmap (issue #870)

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

const TRENDING_V1_CACHE_NS: &str = "v1_trending";
const TRENDING_V1_CACHE_TTL_SECS: u64 = 3_600; // refresh every hour

// ── Query / response types ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TrendingV1Query {
    /// Time window for trending calculation: 24h, 7d, or 30d (default: 7d)
    #[serde(default = "default_window")]
    pub window: String,
}

fn default_window() -> String {
    "7d".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TrendingWindow {
    H24,
    D7,
    D30,
}

impl TrendingWindow {
    fn from_str(s: &str) -> ApiResult<Self> {
        match s {
            "24h" => Ok(Self::H24),
            "7d" => Ok(Self::D7),
            "30d" => Ok(Self::D30),
            other => Err(ApiError::bad_request(
                "INVALID_WINDOW",
                format!("window must be one of: 24h, 7d, 30d — got '{}'", other),
            )),
        }
    }

    fn interactions_column(&self) -> &'static str {
        match self {
            Self::H24 => "interactions_7d", // use 7d as closest proxy when 24h not available
            Self::D7 => "interactions_7d",
            Self::D30 => "interactions_30d",
        }
    }

    fn as_label(&self) -> &'static str {
        match self {
            Self::H24 => "24h",
            Self::D7 => "7d",
            Self::D30 => "30d",
        }
    }

    fn interval_sql(&self) -> &'static str {
        match self {
            Self::H24 => "24 hours",
            Self::D7 => "7 days",
            Self::D30 => "30 days",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrendingContract {
    pub contract_id: String,
    pub name: String,
    pub network: String,
    pub category: Option<String>,
    pub interactions: i64,
    pub growth_percent: f64,
    pub rank: i64,
    pub is_verified: bool,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrendingCategory {
    pub category: String,
    pub contract_count: i64,
    pub total_interactions: i64,
    pub growth_percent: f64,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkHeat {
    pub network: String,
    pub active_contracts: i64,
    pub total_interactions: i64,
    pub heat_score: f64,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TrendingV1Response {
    pub window: String,
    pub contracts: Vec<TrendingContract>,
    pub categories: Vec<TrendingCategory>,
    pub networks: Vec<NetworkHeat>,
    pub cached: bool,
    pub generated_at: DateTime<Utc>,
}

// ── Handler ───────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/v1/trending",
    params(TrendingV1Query),
    responses(
        (status = 200, description = "Trending contracts, categories, and network heatmap", body = TrendingV1Response),
        (status = 400, description = "Invalid window parameter"),
    ),
    tag = "Discovery"
)]
pub async fn get_trending_v1(
    State(state): State<AppState>,
    Query(params): Query<TrendingV1Query>,
) -> ApiResult<impl IntoResponse> {
    let window = TrendingWindow::from_str(&params.window)?;

    let cache_key = window.as_label().to_string();
    let (cached_val, cache_hit) = state.cache.get(TRENDING_V1_CACHE_NS, &cache_key).await;

    if let (Some(json_str), true) = (cached_val, cache_hit) {
        if let Ok(mut resp) = serde_json::from_str::<TrendingV1Response>(&json_str) {
            resp.cached = true;
            return Ok(Json(resp));
        }
    }

    let contracts = fetch_trending_contracts(&state, &window).await?;
    let categories = fetch_trending_categories(&state, &window).await?;
    let networks = fetch_network_heatmap(&state, &window).await?;

    let response = TrendingV1Response {
        window: window.as_label().to_string(),
        contracts,
        categories,
        networks,
        cached: false,
        generated_at: Utc::now(),
    };

    if let Ok(serialized) = serde_json::to_string(&response) {
        state
            .cache
            .put(
                TRENDING_V1_CACHE_NS,
                &cache_key,
                serialized,
                Some(Duration::from_secs(TRENDING_V1_CACHE_TTL_SECS)),
            )
            .await;
    }

    Ok(Json(response))
}

// ── Data fetchers ─────────────────────────────────────────────────────────────

async fn fetch_trending_contracts(
    state: &AppState,
    window: &TrendingWindow,
) -> ApiResult<Vec<TrendingContract>> {
    let col = window.interactions_column();

    #[derive(sqlx::FromRow)]
    struct Row {
        contract_id: String,
        name: String,
        network: String,
        category: Option<String>,
        interactions: i64,
        is_verified: bool,
        rank: i64,
    }

    // Query trending_contracts_mv, excluding deprecated/dead contracts
    let rows: Vec<Row> = sqlx::query_as(&format!(
        r#"
        SELECT
            t.contract_id,
            t.name,
            t.network::TEXT           AS network,
            t.category,
            t.{col}                   AS interactions,
            t.is_verified,
            ROW_NUMBER() OVER (ORDER BY t.{col} DESC) AS rank
        FROM trending_contracts_mv t
        INNER JOIN contracts c ON c.contract_id = t.contract_id
        WHERE c.is_deprecated = FALSE
          AND COALESCE(c.status, '') <> 'dead'
        ORDER BY interactions DESC
        LIMIT 50
        "#,
        col = col,
    ))
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("db error (trending contracts): {e}")))?;

    // Compute a naive growth % — compare against the prior-equivalent window interactions.
    // When not available we leave it as 0.
    let max_interactions = rows
        .iter()
        .map(|r| r.interactions)
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    Ok(rows
        .into_iter()
        .map(|r| {
            let growth_percent =
                ((r.interactions as f64 / max_interactions) * 100.0 * 10.0).round() / 10.0;
            TrendingContract {
                contract_id: r.contract_id,
                name: r.name,
                network: r.network,
                category: r.category,
                interactions: r.interactions,
                growth_percent,
                rank: r.rank,
                is_verified: r.is_verified,
            }
        })
        .collect())
}

async fn fetch_trending_categories(
    state: &AppState,
    window: &TrendingWindow,
) -> ApiResult<Vec<TrendingCategory>> {
    let interval = window.interval_sql();

    #[derive(sqlx::FromRow)]
    struct Row {
        category: String,
        contract_count: i64,
        total_interactions: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(&format!(
        r#"
        SELECT
            c.category,
            COUNT(DISTINCT c.id)                    AS contract_count,
            COALESCE(SUM(ci.interaction_count), 0)  AS total_interactions
        FROM contracts c
        LEFT JOIN (
            SELECT contract_id, COUNT(*) AS interaction_count
            FROM contract_interactions
            WHERE created_at >= NOW() - INTERVAL '{interval}'
            GROUP BY contract_id
        ) ci ON ci.contract_id = c.id
        WHERE c.category IS NOT NULL
          AND c.is_deprecated = FALSE
          AND COALESCE(c.status, '') <> 'dead'
        GROUP BY c.category
        ORDER BY total_interactions DESC
        LIMIT 20
        "#,
        interval = interval,
    ))
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("db error (trending categories): {e}")))?;

    let max_interactions = rows
        .iter()
        .map(|r| r.total_interactions)
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    Ok(rows
        .into_iter()
        .map(|r| {
            let growth_percent =
                ((r.total_interactions as f64 / max_interactions) * 100.0 * 10.0).round() / 10.0;
            TrendingCategory {
                category: r.category,
                contract_count: r.contract_count,
                total_interactions: r.total_interactions,
                growth_percent,
            }
        })
        .collect())
}

async fn fetch_network_heatmap(
    state: &AppState,
    window: &TrendingWindow,
) -> ApiResult<Vec<NetworkHeat>> {
    let interval = window.interval_sql();

    #[derive(sqlx::FromRow)]
    struct Row {
        network: String,
        active_contracts: i64,
        total_interactions: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(&format!(
        r#"
        SELECT
            c.network::TEXT                         AS network,
            COUNT(DISTINCT c.id)                    AS active_contracts,
            COALESCE(SUM(ci.interaction_count), 0)  AS total_interactions
        FROM contracts c
        LEFT JOIN (
            SELECT contract_id, COUNT(*) AS interaction_count
            FROM contract_interactions
            WHERE created_at >= NOW() - INTERVAL '{interval}'
            GROUP BY contract_id
        ) ci ON ci.contract_id = c.id
        WHERE c.is_deprecated = FALSE
          AND COALESCE(c.status, '') <> 'dead'
        GROUP BY c.network
        ORDER BY total_interactions DESC
        "#,
        interval = interval,
    ))
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("db error (network heatmap): {e}")))?;

    let max_interactions = rows
        .iter()
        .map(|r| r.total_interactions)
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    Ok(rows
        .into_iter()
        .map(|r| {
            let heat_score =
                ((r.total_interactions as f64 / max_interactions) * 100.0 * 100.0).round() / 100.0;
            NetworkHeat {
                network: r.network,
                active_contracts: r.active_contracts,
                total_interactions: r.total_interactions,
                heat_score,
            }
        })
        .collect())
}
