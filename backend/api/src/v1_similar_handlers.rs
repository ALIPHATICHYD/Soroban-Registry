//! GET /api/v1/contracts/{id}/similar — similar contracts with type filter and 6-hour cache (issue #871)

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

const SIMILAR_CACHE_NS: &str = "v1_similar_contracts";
const SIMILAR_CACHE_TTL_SECS: u64 = 6 * 3_600; // 6 hours

// ── Query / response types ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct SimilarV1Query {
    /// Similarity dimension: category, functionality, or network
    #[serde(default)]
    pub r#type: SimilarityType,
    /// Maximum results (1–50)
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    10
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SimilarityType {
    #[default]
    Category,
    Functionality,
    Network,
}

impl std::fmt::Display for SimilarityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Category => write!(f, "category"),
            Self::Functionality => write!(f, "functionality"),
            Self::Network => write!(f, "network"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SimilarContractItem {
    pub contract_id: String,
    /// Similarity score 0.0–1.0
    pub score: f64,
    pub similarity_type: String,
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SimilarContractsV1Response {
    pub contract_id: String,
    pub items: Vec<SimilarContractItem>,
    pub cached: bool,
    pub generated_at: DateTime<Utc>,
}

// ── Handler ───────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/v1/contracts/{id}/similar",
    params(
        ("id" = String, Path, description = "Contract UUID or string ID"),
        SimilarV1Query
    ),
    responses(
        (status = 200, description = "Similar contracts ranked by score", body = SimilarContractsV1Response),
        (status = 404, description = "Contract not found"),
    ),
    tag = "Contracts"
)]
pub async fn get_similar_contracts_v1(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<SimilarV1Query>,
) -> ApiResult<impl IntoResponse> {
    let limit = query.limit.clamp(1, 50);
    let sim_type = query.r#type.clone();

    let contract_uuid = resolve_contract(&state, &id).await?;

    let cache_key = format!("{}:{}:{}", contract_uuid, sim_type, limit);
    let (cached_val, cache_hit) = state.cache.get(SIMILAR_CACHE_NS, &cache_key).await;

    if let (Some(json_str), true) = (cached_val, cache_hit) {
        if let Ok(mut resp) = serde_json::from_str::<SimilarContractsV1Response>(&json_str) {
            resp.cached = true;
            return Ok(Json(resp));
        }
    }

    let items = match sim_type {
        SimilarityType::Category => fetch_by_category(&state, contract_uuid, limit).await?,
        SimilarityType::Functionality => {
            fetch_by_functionality(&state, contract_uuid, limit).await?
        }
        SimilarityType::Network => fetch_by_network(&state, contract_uuid, limit).await?,
    };

    let response = SimilarContractsV1Response {
        contract_id: id.clone(),
        items,
        cached: false,
        generated_at: Utc::now(),
    };

    if let Ok(serialized) = serde_json::to_string(&response) {
        state
            .cache
            .put(
                SIMILAR_CACHE_NS,
                &cache_key,
                serialized,
                Some(Duration::from_secs(SIMILAR_CACHE_TTL_SECS)),
            )
            .await;
    }

    Ok(Json(response))
}

// ── Similarity queries ────────────────────────────────────────────────────────

async fn fetch_by_category(
    state: &AppState,
    contract_uuid: Uuid,
    limit: i64,
) -> ApiResult<Vec<SimilarContractItem>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        contract_id: String,
        name: Option<String>,
        same_category: bool,
        total_interactions: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT
            c.contract_id,
            c.name,
            (c.category = target.category AND c.category IS NOT NULL) AS same_category,
            COALESCE(cs.total_interactions, 0)                        AS total_interactions
        FROM contracts c
        CROSS JOIN (SELECT category FROM contracts WHERE id = $1) target
        LEFT JOIN contract_stats cs ON cs.contract_id = c.id
        WHERE c.id <> $1
          AND c.is_deprecated = FALSE
          AND c.category IS NOT NULL
          AND c.category = target.category
        ORDER BY total_interactions DESC
        LIMIT $2
        "#,
    )
    .bind(contract_uuid)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("db error (category similarity): {e}")))?;

    let max_interactions = rows
        .iter()
        .map(|r| r.total_interactions)
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    Ok(rows
        .into_iter()
        .map(|r| SimilarContractItem {
            contract_id: r.contract_id,
            score: if r.same_category {
                0.5 + 0.5 * (r.total_interactions as f64 / max_interactions)
            } else {
                0.0
            },
            similarity_type: "category".to_string(),
            name: r.name,
        })
        .collect())
}

async fn fetch_by_functionality(
    state: &AppState,
    contract_uuid: Uuid,
    limit: i64,
) -> ApiResult<Vec<SimilarContractItem>> {
    // Use pre-computed similarity scores from the similarity analysis table if available,
    // otherwise fall back to category-based scoring.
    #[derive(sqlx::FromRow)]
    struct Row {
        contract_id: String,
        name: Option<String>,
        similarity_score: f64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT
            c.contract_id,
            c.name,
            COALESCE(cs.similarity_score, 0.0) AS similarity_score
        FROM contract_similarities cs
        INNER JOIN contracts c ON c.id = cs.similar_contract_id
        WHERE cs.contract_id = $1
          AND c.is_deprecated = FALSE
        ORDER BY cs.similarity_score DESC
        LIMIT $2
        "#,
    )
    .bind(contract_uuid)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    if !rows.is_empty() {
        return Ok(rows
            .into_iter()
            .map(|r| SimilarContractItem {
                contract_id: r.contract_id,
                score: (r.similarity_score * 100.0).round() / 100.0,
                similarity_type: "functionality".to_string(),
                name: r.name,
            })
            .collect());
    }

    // Fallback: same category, sorted by interaction count
    fetch_by_category(state, contract_uuid, limit)
        .await
        .map(|items| {
            items
                .into_iter()
                .map(|mut i| {
                    i.similarity_type = "functionality".to_string();
                    i
                })
                .collect()
        })
}

async fn fetch_by_network(
    state: &AppState,
    contract_uuid: Uuid,
    limit: i64,
) -> ApiResult<Vec<SimilarContractItem>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        contract_id: String,
        name: Option<String>,
        total_interactions: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT
            c.contract_id,
            c.name,
            COALESCE(cs.total_interactions, 0) AS total_interactions
        FROM contracts c
        CROSS JOIN (SELECT network FROM contracts WHERE id = $1) target
        LEFT JOIN contract_stats cs ON cs.contract_id = c.id
        WHERE c.id <> $1
          AND c.is_deprecated = FALSE
          AND c.network = target.network
        ORDER BY total_interactions DESC
        LIMIT $2
        "#,
    )
    .bind(contract_uuid)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("db error (network similarity): {e}")))?;

    let max_interactions = rows
        .iter()
        .map(|r| r.total_interactions)
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    Ok(rows
        .into_iter()
        .map(|r| SimilarContractItem {
            contract_id: r.contract_id,
            score: (0.3 + 0.7 * (r.total_interactions as f64 / max_interactions) * 100.0).round()
                / 100.0,
            similarity_type: "network".to_string(),
            name: r.name,
        })
        .collect())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn resolve_contract(state: &AppState, id: &str) -> ApiResult<Uuid> {
    if let Ok(uuid) = Uuid::parse_str(id) {
        let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM contracts WHERE id = $1")
            .bind(uuid)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("db error: {e}")))?;
        return exists.ok_or_else(|| {
            ApiError::not_found("CONTRACT_NOT_FOUND", format!("contract {} not found", id))
        });
    }

    let uuid: Option<Uuid> = sqlx::query_scalar("SELECT id FROM contracts WHERE contract_id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("db error: {e}")))?;

    uuid.ok_or_else(|| {
        ApiError::not_found("CONTRACT_NOT_FOUND", format!("contract {} not found", id))
    })
}
