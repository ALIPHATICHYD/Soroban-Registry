use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use shared::models::{
    ContractRecommendationsResponse, Network, RecommendationReason, RecommendedContract,
};
use sqlx::FromRow;
use std::cmp::Ordering;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

const RECOMMENDATION_CACHE_NAMESPACE: &str = "contract_recommendations";
const RECOMMENDATION_CACHE_TTL_SECS: u64 = 300;
const CANDIDATE_SCAN_LIMIT: i64 = 200;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct RecommendationQuery {
    /// Maximum number of recommendations to return (1..=20)
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Optional network filter. If omitted, ranking still favors same-network contracts.
    pub network: Option<Network>,
    /// Deterministic subject identifier for A/B assignment (wallet, session, etc.)
    pub subject: Option<String>,
    /// Optional algorithm override for experimentation (hybrid_v1 or hybrid_v2)
    pub algorithm: Option<String>,
}

fn default_limit() -> i64 {
    8
}

#[derive(Debug, Clone)]
struct Weights {
    category: f64,
    network: f64,
    functionality: f64,
    popularity: f64,
}

#[derive(Debug, Clone, FromRow)]
struct RecommendationCandidateRow {
    id: Uuid,
    contract_id: String,
    name: String,
    description: Option<String>,
    network: Network,
    category: Option<String>,
    #[allow(dead_code)]
    tags: Vec<String>,
    popularity: i64,
    similarity_score: f64,
    category_match: f64,
    network_match: f64,
    tag_overlap: f64,
}

#[derive(Debug, Clone)]
struct ScoredCandidate {
    row: RecommendationCandidateRow,
    recommendation_score: f64,
    functionality_score: f64,
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/recommendations",
    params(
        ("id" = String, Path, description = "Contract UUID"),
        RecommendationQuery
    ),
    responses(
        (status = 200, description = "Recommended contracts with explanation", body = ContractRecommendationsResponse),
        (status = 404, description = "Contract not found")
    ),
    tag = "Contracts"
)]
pub async fn get_contract_recommendations(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<RecommendationQuery>,
) -> ApiResult<impl IntoResponse> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;

    let limit = query.limit.clamp(1, 20);
    let subject = query.subject.unwrap_or_else(|| "anonymous".to_string());

    let (selected_variant, selected_algorithm) = choose_ab_variant_and_algorithm(
        &state,
        contract_uuid,
        &subject,
        query.algorithm.as_deref(),
    )
    .await?;

    let cache_key = format!(
        "{}:{}:{}:{}:{}",
        contract_uuid,
        limit,
        query
            .network
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "any".to_string()),
        selected_algorithm,
        selected_variant
    );

    let (cached_payload, cache_hit) = state
        .cache
        .get(RECOMMENDATION_CACHE_NAMESPACE, &cache_key)
        .await;

    if let (Some(serialized), true) = (cached_payload, cache_hit) {
        if let Ok(mut response) =
            serde_json::from_str::<ContractRecommendationsResponse>(&serialized)
        {
            response.cached = true;
            return Ok(Json(response));
        }
    }

    let candidates =
        fetch_recommendation_candidates(&state, contract_uuid, query.network.clone()).await?;
    let weights = weights_for_algorithm(&selected_algorithm);
    let scored = score_candidates(candidates, &weights);

    let recommendations: Vec<RecommendedContract> = scored
        .into_iter()
        .take(limit as usize)
        .map(to_recommended_contract)
        .collect();

    let response = ContractRecommendationsResponse {
        contract_id: contract_uuid,
        algorithm: selected_algorithm.clone(),
        ab_variant: selected_variant.clone(),
        cached: false,
        generated_at: Utc::now(),
        recommendations: recommendations.clone(),
    };

    if let Ok(serialized) = serde_json::to_string(&response) {
        state
            .cache
            .put(
                RECOMMENDATION_CACHE_NAMESPACE,
                &cache_key,
                serialized,
                Some(Duration::from_secs(RECOMMENDATION_CACHE_TTL_SECS)),
            )
            .await;
    }

    record_recommendation_impressions(
        &state,
        contract_uuid,
        &selected_algorithm,
        &selected_variant,
        &subject,
        &recommendations,
    )
    .await;

    Ok(Json(response))
}

async fn choose_ab_variant_and_algorithm(
    state: &AppState,
    contract_id: Uuid,
    subject: &str,
    override_algorithm: Option<&str>,
) -> ApiResult<(String, String)> {
    let running_test_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ab_tests WHERE contract_id = $1 AND status = 'running' ORDER BY started_at DESC NULLS LAST LIMIT 1",
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| db_err("lookup running ab test for recommendations", e))?;

    let variant = if let Some(test_id) = running_test_id {
        sqlx::query_scalar::<_, Option<String>>("SELECT assign_variant($1, $2)::text")
            .bind(test_id)
            .bind(subject)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| db_err("assign ab variant for recommendations", e))?
            .flatten()
            .unwrap_or_else(|| "control".to_string())
    } else {
        "baseline".to_string()
    };

    let algorithm = match override_algorithm {
        Some("hybrid_v1") => "hybrid_v1".to_string(),
        Some("hybrid_v2") => "hybrid_v2".to_string(),
        _ => {
            if variant == "treatment" {
                "hybrid_v2".to_string()
            } else {
                "hybrid_v1".to_string()
            }
        }
    };

    Ok((variant, algorithm))
}

async fn fetch_recommendation_candidates(
    state: &AppState,
    contract_id: Uuid,
    network_filter: Option<Network>,
) -> ApiResult<Vec<RecommendationCandidateRow>> {
    let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| db_err("check source contract for recommendations", e))?;

    if exists.is_none() {
        return Err(ApiError::not_found(
            "ContractNotFound",
            format!("No contract found with ID: {}", contract_id),
        ));
    }

    let candidates: Vec<RecommendationCandidateRow> = sqlx::query_as(
        r#"
        WITH base AS (
            SELECT id, category, network, tags
            FROM contracts
            WHERE id = $1
        ),
        popularity AS (
            SELECT contract_id, COALESCE(SUM(interaction_count), 0)::BIGINT AS popularity
            FROM contract_interaction_daily_aggregates
            WHERE day >= CURRENT_DATE - INTERVAL '30 days'
            GROUP BY contract_id
        ),
        similarity AS (
            SELECT similar_contract_id AS candidate_id, MAX(similarity_score)::DOUBLE PRECISION AS similarity_score
            FROM contract_similarity_reports
            WHERE contract_id = $1
            GROUP BY similar_contract_id
        )
        SELECT
            c.id,
            c.contract_id,
            c.name,
            c.description,
            c.network,
            c.category,
            c.tags,
            COALESCE(popularity.popularity, 0)::BIGINT AS popularity,
            COALESCE(similarity.similarity_score, 0.0)::DOUBLE PRECISION AS similarity_score,
            CASE
                WHEN base.category IS NOT NULL AND c.category = base.category THEN 1.0
                ELSE 0.0
            END::DOUBLE PRECISION AS category_match,
            CASE
                WHEN c.network = base.network THEN 1.0
                ELSE 0.0
            END::DOUBLE PRECISION AS network_match,
            CASE
                WHEN COALESCE(array_length(base.tags, 1), 0) = 0 THEN 0.0
                ELSE (
                    SELECT COALESCE(COUNT(*)::DOUBLE PRECISION, 0.0)
                    FROM unnest(base.tags) AS t(tag)
                    WHERE t.tag = ANY(c.tags)
                ) / GREATEST(
                    COALESCE(array_length(base.tags, 1), 1),
                    COALESCE(array_length(c.tags, 1), 1)
                )
            END::DOUBLE PRECISION AS tag_overlap
        FROM contracts c
        CROSS JOIN base
        LEFT JOIN popularity ON popularity.contract_id = c.id
        LEFT JOIN similarity ON similarity.candidate_id = c.id
        WHERE c.id <> $1
          AND ($2::network_type IS NULL OR c.network = $2)
        ORDER BY c.updated_at DESC
        LIMIT $3
        "#,
    )
    .bind(contract_id)
    .bind(network_filter)
    .bind(CANDIDATE_SCAN_LIMIT)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("fetch recommendation candidates", e))?;

    Ok(candidates)
}

fn weights_for_algorithm(algorithm: &str) -> Weights {
    if algorithm == "hybrid_v2" {
        Weights {
            category: 0.20,
            network: 0.20,
            functionality: 0.40,
            popularity: 0.20,
        }
    } else {
        Weights {
            category: 0.35,
            network: 0.25,
            functionality: 0.25,
            popularity: 0.15,
        }
    }
}

fn score_candidates(
    rows: Vec<RecommendationCandidateRow>,
    weights: &Weights,
) -> Vec<ScoredCandidate> {
    let max_popularity = rows
        .iter()
        .map(|row| row.popularity)
        .max()
        .unwrap_or(1)
        .max(1) as f64;

    let mut scored: Vec<ScoredCandidate> = rows
        .into_iter()
        .map(|row| {
            let popularity_norm = (row.popularity as f64 / max_popularity).clamp(0.0, 1.0);
            let functionality_score = row.similarity_score.max(row.tag_overlap).clamp(0.0, 1.0);
            let score = weights.category * row.category_match
                + weights.network * row.network_match
                + weights.functionality * functionality_score
                + weights.popularity * popularity_norm;

            ScoredCandidate {
                row,
                recommendation_score: score,
                functionality_score,
            }
        })
        .collect();

    scored.sort_by(|a, b| {
        b.recommendation_score
            .partial_cmp(&a.recommendation_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.row.popularity.cmp(&a.row.popularity))
            .then_with(|| a.row.name.cmp(&b.row.name))
    });

    scored
}

fn to_recommended_contract(candidate: ScoredCandidate) -> RecommendedContract {
    let mut reasons = Vec::new();

    if candidate.row.category_match > 0.0 {
        reasons.push(RecommendationReason {
            code: "same_category".to_string(),
            message: format!(
                "Shares category {}",
                candidate
                    .row
                    .category
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string())
            ),
            weight: 0.35,
        });
    }

    if candidate.row.network_match > 0.0 {
        reasons.push(RecommendationReason {
            code: "same_network".to_string(),
            message: format!("Available on {}", candidate.row.network),
            weight: 0.25,
        });
    }

    if candidate.functionality_score >= 0.10 {
        reasons.push(RecommendationReason {
            code: "similar_functionality".to_string(),
            message: if candidate.row.similarity_score >= candidate.row.tag_overlap {
                "High similarity from prior contract analysis".to_string()
            } else {
                "Overlap in contract tags and capability hints".to_string()
            },
            weight: 0.25,
        });
    }

    if candidate.row.popularity > 0 {
        reasons.push(RecommendationReason {
            code: "popular_contract".to_string(),
            message: format!(
                "Observed {} interactions in the last 30 days",
                candidate.row.popularity
            ),
            weight: 0.15,
        });
    }

    if reasons.is_empty() {
        reasons.push(RecommendationReason {
            code: "fresh_candidate".to_string(),
            message: "Relevant by recency and baseline ranking".to_string(),
            weight: 0.05,
        });
    }

    let explanation = reasons
        .iter()
        .map(|r| r.message.clone())
        .collect::<Vec<_>>()
        .join("; ");

    RecommendedContract {
        id: candidate.row.id,
        contract_id: candidate.row.contract_id,
        name: candidate.row.name,
        description: candidate.row.description,
        network: candidate.row.network,
        category: candidate.row.category,
        popularity_score: candidate.row.popularity,
        similarity_score: candidate.row.similarity_score,
        recommendation_score: candidate.recommendation_score,
        reasons,
        explanation,
    }
}

async fn record_recommendation_impressions(
    state: &AppState,
    source_contract_id: Uuid,
    algorithm: &str,
    ab_variant: &str,
    subject: &str,
    recs: &[RecommendedContract],
) {
    if recs.is_empty() {
        return;
    }

    let db = state.db.clone();
    let subject_hash = hash_subject(subject);
    let algorithm = algorithm.to_string();
    let ab_variant = ab_variant.to_string();
    let events: Vec<(Uuid, serde_json::Value, f64, serde_json::Value)> = recs
        .iter()
        .map(|rec| {
            let reason_codes: Vec<String> = rec.reasons.iter().map(|r| r.code.clone()).collect();
            (
                rec.id,
                serde_json::to_value(reason_codes).unwrap_or_else(|_| serde_json::json!([])),
                rec.recommendation_score,
                serde_json::json!({
                    "network": rec.network.to_string(),
                    "category": rec.category.clone(),
                    "similarity_score": rec.similarity_score,
                    "popularity_score": rec.popularity_score
                }),
            )
        })
        .collect();

    tokio::spawn(async move {
        for (recommended_contract_id, reason_codes, score, context) in events {
            let _ = sqlx::query(
                r#"
                INSERT INTO contract_recommendation_events (
                    source_contract_id,
                    recommended_contract_id,
                    event_type,
                    algorithm_key,
                    ab_variant,
                    reason_codes,
                    score,
                    subject_hash,
                    context
                )
                VALUES ($1, $2, 'impression', $3, $4, $5, $6, $7, $8)
                "#,
            )
            .bind(source_contract_id)
            .bind(recommended_contract_id)
            .bind(&algorithm)
            .bind(&ab_variant)
            .bind(reason_codes)
            .bind(score)
            .bind(&subject_hash)
            .bind(context)
            .execute(&db)
            .await;
        }
    });
}

fn hash_subject(subject: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(subject.as_bytes());
    hex::encode(hasher.finalize())
}

fn db_err(operation: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = operation, error = ?err, "database operation failed");
    ApiError::internal("An unexpected database error occurred")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_row(
        name: &str,
        popularity: i64,
        category_match: f64,
        network_match: f64,
        similarity_score: f64,
        tag_overlap: f64,
    ) -> RecommendationCandidateRow {
        RecommendationCandidateRow {
            id: Uuid::new_v4(),
            contract_id: format!("C-{}", name),
            name: name.to_string(),
            description: None,
            network: Network::Testnet,
            category: Some("DeFi".to_string()),
            tags: vec!["dex".to_string()],
            popularity,
            similarity_score,
            category_match,
            network_match,
            tag_overlap,
        }
    }

    #[test]
    fn scoring_prioritizes_strong_similarity_and_popularity() {
        let weights = weights_for_algorithm("hybrid_v2");
        let rows = vec![
            sample_row("A", 50, 1.0, 1.0, 0.9, 0.3),
            sample_row("B", 1, 1.0, 1.0, 0.1, 0.1),
        ];

        let scored = score_candidates(rows, &weights);
        assert_eq!(scored[0].row.name, "A");
        assert!(scored[0].recommendation_score > scored[1].recommendation_score);
    }

    #[test]
    fn recommendation_contains_explainable_reasons() {
        let candidate = ScoredCandidate {
            row: sample_row("Explained", 123, 1.0, 1.0, 0.8, 0.2),
            recommendation_score: 0.91,
            functionality_score: 0.8,
        };

        let rec = to_recommended_contract(candidate);
        assert!(!rec.reasons.is_empty());
        assert!(
            rec.explanation.contains("Shares category") || rec.explanation.contains("similarity")
        );
    }

    #[test]
    fn subject_hash_is_stable() {
        let h1 = hash_subject("wallet:abc");
        let h2 = hash_subject("wallet:abc");
        let h3 = hash_subject("wallet:def");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }
}
