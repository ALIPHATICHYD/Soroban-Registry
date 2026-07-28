use crate::error::ApiError;
use crate::state::AppState;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use shared::models::Network;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryStats {
    // Totals
    pub total_contracts: i64,
    pub total_publishers: i64,
    pub verified_contracts: i64,
    pub verification_percentage: f64,
    pub total_categories: i64,
    pub total_networks: i64,

    // Growth
    pub contracts_last_7d: i64,
    pub contracts_last_30d: i64,
    pub new_publishers_last_30d: i64,

    // Top contracts
    pub top_contracts: Vec<TopContract>,

    // Network breakdown
    pub network_stats: Vec<NetworkStats>,

    // Category breakdown
    pub category_stats: Vec<CategoryStats>,

    // Timestamp
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TopContract {
    pub id: uuid::Uuid,
    pub name: String,
    pub contract_id: String,
    pub network: Network,
    pub is_verified: bool,
    pub interaction_count: i64,
    pub unique_users: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct NetworkStats {
    pub network: Network,
    pub contract_count: i64,
    pub verified_count: i64,
    pub verification_rate: f64,
    pub publisher_count: i64,
    pub category_count: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct CategoryStats {
    pub category: String,
    pub contract_count: i64,
    pub verified_count: i64,
    pub verification_rate: f64,
    pub publisher_count: i64,
    pub network_coverage: i64,
    pub popularity_rank: i32,
}

/// Get comprehensive registry statistics
/// Endpoint: GET /api/stats
/// Query params: timeframe=7d|30d|all (default all)
pub async fn get_stats_handler(
    State(state): State<AppState>,
    Query(params): Query<StatsQuery>,
) -> Result<Json<RegistryStats>, ApiError> {
    let timeframe = params.timeframe.as_deref().unwrap_or("all");

    // Get basic counts
    let total_contracts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal_error("DB_ERROR", e.to_string()))?;

    let total_publishers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM publishers")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal_error("DB_ERROR", e.to_string()))?;

    let verified_contracts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM contracts WHERE is_verified = true")
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal_error("DB_ERROR", e.to_string()))?;

    let verification_percentage = if total_contracts > 0 {
        (verified_contracts as f64 / total_contracts as f64) * 100.0
    } else {
        0.0
    };

    let total_categories: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT category) FROM contracts WHERE category IS NOT NULL",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("DB_ERROR", e.to_string()))?;

    let total_networks: i64 = sqlx::query_scalar("SELECT COUNT(DISTINCT network) FROM contracts")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal_error("DB_ERROR", e.to_string()))?;

    // Growth stats
    let interval = match timeframe {
        "7d" => "7 days",
        "30d" => "30 days",
        _ => "ALL",
    };

    let contracts_last_7d: i64 = if timeframe == "all" {
        0
    } else {
        sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM contracts WHERE created_at > NOW() - INTERVAL '{}'",
            interval
        ))
        .fetch_one(&state.db)
        .await
        .unwrap_or(0)
    };

    let contracts_last_30d: i64 = if timeframe == "all" {
        total_contracts
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM contracts WHERE created_at > NOW() - INTERVAL '30 days'",
        )
        .fetch_one(&state.db)
        .await
        .unwrap_or(0)
    };

    let new_publishers_last_30d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM publishers WHERE created_at > NOW() - INTERVAL '30 days'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    // Top contracts by interactions
    let top_contracts: Vec<TopContract> = sqlx::query_as(
        r#"
        SELECT 
            c.id, c.name, c.contract_id, c.network, c.is_verified,
            COALESCE(ci.interaction_count, 0) as interaction_count,
            COALESCE(ci.unique_users, 0) as unique_users
        FROM contracts c
        LEFT JOIN (
            SELECT contract_id, COUNT(*) as interaction_count, COUNT(DISTINCT user_address) as unique_users
            FROM contract_interactions 
            WHERE created_at > NOW() - INTERVAL '30 days'
            GROUP BY contract_id
        ) ci ON c.id = ci.contract_id
        ORDER BY ci.interaction_count DESC NULLS LAST
        LIMIT 10
        "#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // Network breakdown
    let network_stats: Vec<NetworkStats> = sqlx::query_as(
        r#"
        SELECT 
            network,
            COUNT(*) as contract_count,
            COUNT(*) FILTER (WHERE is_verified = true) as verified_count,
            ROUND(
                (COUNT(*) FILTER (WHERE is_verified = true)::numeric / COUNT(*) * 100), 
                2
            ) as verification_rate,
            COUNT(DISTINCT publisher_id) as publisher_count,
            COUNT(DISTINCT category) as category_count
        FROM contracts
        GROUP BY network
        ORDER BY network
        "#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // Category breakdown
    let category_stats: Vec<CategoryStats> = sqlx::query_as(
        r#"
        SELECT 
            COALESCE(category, 'Uncategorized') as category,
            COUNT(*) as contract_count,
            COUNT(*) FILTER (WHERE is_verified = true) as verified_count,
            ROUND(
                (COUNT(*) FILTER (WHERE is_verified = true)::numeric / COUNT(*) * 100), 
                2
            ) as verification_rate,
            COUNT(DISTINCT publisher_id) as publisher_count,
            COUNT(DISTINCT network) as network_coverage,
            RANK() OVER (ORDER BY COUNT(*) DESC) as popularity_rank
        FROM contracts
        GROUP BY category
        ORDER BY contract_count DESC
        LIMIT 20
        "#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Ok(Json(RegistryStats {
        total_contracts,
        total_publishers,
        verified_contracts,
        verification_percentage,
        total_categories,
        total_networks,
        contracts_last_7d,
        contracts_last_30d,
        new_publishers_last_30d,
        top_contracts,
        network_stats,
        category_stats,
        generated_at: chrono::Utc::now(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    pub timeframe: Option<String>,
}
