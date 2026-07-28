use std::collections::{HashMap, HashSet};

use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Json, Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use shared::models::{
    BatchSimilarityAnalysisItem, BatchSimilarityAnalysisRequest, BatchSimilarityAnalysisResponse,
    ContractSimilarityResponse, ContractSimilarityResult, SimilarityMatchType,
    SimilarityReviewStatus,
};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct SimilarityQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    10
}

#[derive(Debug, FromRow)]
struct ContractAnalysisRow {
    id: Uuid,
    contract_id: String,
    name: String,
    publisher_id: Uuid,
    category: Option<String>,
    wasm_hash: String,
    is_verified: bool,
    total_interactions: i64,
    source_code: Option<String>,
    abi: Option<Value>,
}

#[derive(Debug, Clone)]
struct ContractAnalysisInput {
    id: Uuid,
    contract_address: String,
    name: String,
    publisher_id: Uuid,
    category: Option<String>,
    wasm_hash: String,
    is_verified: bool,
    total_interactions: i64,
    representation_type: String,
    exact_hash: String,
    simhash: u64,
    token_count: usize,
    source_length: usize,
    name_tokens: HashSet<String>,
}

#[derive(Debug, Clone)]
struct SimilarityCandidate {
    contract_id: Uuid,
    similar_contract_id: Uuid,
    similar_contract_name: String,
    similar_contract_address: String,
    similarity_score: f64,
    exact_clone: bool,
    match_type: SimilarityMatchType,
    suspicious: bool,
    flagged_for_review: bool,
    review_status: SimilarityReviewStatus,
    reasons: Value,
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/similar",
    params(
        ("id" = String, Path, description = "Contract UUID"),
        ("limit" = Option<i64>, Query, description = "Maximum number of matches to return")
    ),
    responses(
        (status = 200, description = "Similar contracts", body = ContractSimilarityResponse),
        (status = 404, description = "Contract not found")
    ),
    tag = "Analysis"
)]
pub async fn get_similar_contracts(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<SimilarityQuery>,
) -> ApiResult<impl IntoResponse> {
    let contract_id = parse_uuid(&id, "contract")?;
    let limit = query.limit.clamp(1, 50);

    let target = fetch_contract_input(&state.db, contract_id).await?;
    let others = fetch_other_contract_inputs(&state.db, contract_id).await?;
    let results = analyze_and_persist(&state.db, &target, &others, limit).await?;

    Ok(Json(ContractSimilarityResponse {
        contract_id,
        total_matches: results.len(),
        suspicious_matches: results.iter().filter(|item| item.suspicious).count(),
        items: results.into_iter().map(to_similarity_result).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/contracts/similarity/analyze",
    request_body = BatchSimilarityAnalysisRequest,
    responses(
        (status = 200, description = "Batch similarity analysis results", body = BatchSimilarityAnalysisResponse)
    ),
    tag = "Analysis"
)]
pub async fn analyze_contract_similarity_batch(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<BatchSimilarityAnalysisRequest>,
) -> ApiResult<impl IntoResponse> {
    let limit = req.limit_per_contract.unwrap_or(10).clamp(1, 50);

    let contract_ids = if req.contract_ids.is_empty() {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM contracts ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await
            .map_err(|e| db_err("load contracts for similarity batch", e))?
    } else {
        req.contract_ids
            .iter()
            .map(|id| parse_uuid(id, "contract"))
            .collect::<Result<Vec<_>, _>>()?
    };

    let mut items = Vec::new();
    let mut total_flagged = 0usize;

    for contract_id in contract_ids {
        let target = fetch_contract_input(&state.db, contract_id).await?;
        let others = fetch_other_contract_inputs(&state.db, contract_id).await?;
        let results = analyze_and_persist(&state.db, &target, &others, limit).await?;
        total_flagged += results
            .iter()
            .filter(|item| item.flagged_for_review)
            .count();

        items.push(BatchSimilarityAnalysisItem {
            contract_id,
            analyzed_contracts: others.len(),
            suspicious_matches: results.iter().filter(|item| item.suspicious).count(),
            items: results.into_iter().map(to_similarity_result).collect(),
        });
    }

    Ok(Json(BatchSimilarityAnalysisResponse {
        analyzed_contracts: items.len(),
        total_flagged_for_review: total_flagged,
        items,
    }))
}

async fn fetch_contract_input(
    pool: &PgPool,
    contract_id: Uuid,
) -> ApiResult<ContractAnalysisInput> {
    let row: Option<ContractAnalysisRow> = sqlx::query_as(
        r#"
        SELECT
            c.id,
            c.contract_id,
            c.name,
            c.publisher_id,
            c.category,
            c.wasm_hash,
            c.is_verified,
            COALESCE(ci.total_interactions, 0) AS total_interactions,
            v.source_code,
            ca.abi
        FROM contracts c
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::BIGINT AS total_interactions
            FROM contract_interactions ci
            WHERE ci.contract_id = c.id
        ) ci ON TRUE
        LEFT JOIN LATERAL (
            SELECT source_code
            FROM verifications
            WHERE contract_id = c.id AND source_code IS NOT NULL
            ORDER BY verified_at DESC NULLS LAST, created_at DESC
            LIMIT 1
        ) v ON TRUE
        LEFT JOIN LATERAL (
            SELECT abi
            FROM contract_abis
            WHERE contract_id = c.id
            ORDER BY created_at DESC
            LIMIT 1
        ) ca ON TRUE
        WHERE c.id = $1
        "#,
    )
    .bind(contract_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| db_err("load contract similarity input", e))?;

    row.map(build_analysis_input).ok_or_else(|| {
        ApiError::not_found(
            "ContractNotFound",
            format!("No contract found with ID: {}", contract_id),
        )
    })
}

async fn fetch_other_contract_inputs(
    pool: &PgPool,
    contract_id: Uuid,
) -> ApiResult<Vec<ContractAnalysisInput>> {
    let rows: Vec<ContractAnalysisRow> = sqlx::query_as(
        r#"
        SELECT
            c.id,
            c.contract_id,
            c.name,
            c.publisher_id,
            c.category,
            c.wasm_hash,
            c.is_verified,
            COALESCE(ci.total_interactions, 0) AS total_interactions,
            v.source_code,
            ca.abi
        FROM contracts c
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::BIGINT AS total_interactions
            FROM contract_interactions ci
            WHERE ci.contract_id = c.id
        ) ci ON TRUE
        LEFT JOIN LATERAL (
            SELECT source_code
            FROM verifications
            WHERE contract_id = c.id AND source_code IS NOT NULL
            ORDER BY verified_at DESC NULLS LAST, created_at DESC
            LIMIT 1
        ) v ON TRUE
        LEFT JOIN LATERAL (
            SELECT abi
            FROM contract_abis
            WHERE contract_id = c.id
            ORDER BY created_at DESC
            LIMIT 1
        ) ca ON TRUE
        WHERE c.id <> $1
        "#,
    )
    .bind(contract_id)
    .fetch_all(pool)
    .await
    .map_err(|e| db_err("load candidate contract similarity inputs", e))?;

    Ok(rows.into_iter().map(build_analysis_input).collect())
}

async fn analyze_and_persist(
    pool: &PgPool,
    target: &ContractAnalysisInput,
    others: &[ContractAnalysisInput],
    limit: i64,
) -> ApiResult<Vec<SimilarityCandidate>> {
    upsert_signature(pool, target).await?;
    for other in others {
        upsert_signature(pool, other).await?;
    }

    let mut candidates = Vec::new();
    for other in others {
        if let Some(candidate) = compare_contracts(target, other) {
            upsert_similarity_report(pool, &candidate).await?;
            candidates.push(candidate);
        }
    }

    candidates.sort_by(|a, b| {
        b.similarity_score
            .partial_cmp(&a.similarity_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.truncate(limit as usize);
    Ok(candidates)
}

async fn upsert_signature(pool: &PgPool, input: &ContractAnalysisInput) -> ApiResult<()> {
    sqlx::query(
        r#"
        INSERT INTO contract_similarity_signatures (
            contract_id,
            representation_type,
            exact_hash,
            simhash,
            token_count,
            source_length,
            wasm_hash,
            computed_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
        ON CONFLICT (contract_id)
        DO UPDATE SET
            representation_type = EXCLUDED.representation_type,
            exact_hash = EXCLUDED.exact_hash,
            simhash = EXCLUDED.simhash,
            token_count = EXCLUDED.token_count,
            source_length = EXCLUDED.source_length,
            wasm_hash = EXCLUDED.wasm_hash,
            computed_at = NOW()
        "#,
    )
    .bind(input.id)
    .bind(&input.representation_type)
    .bind(&input.exact_hash)
    .bind(input.simhash as i64)
    .bind(input.token_count as i32)
    .bind(input.source_length as i32)
    .bind(&input.wasm_hash)
    .execute(pool)
    .await
    .map_err(|e| db_err("upsert similarity signature", e))?;

    Ok(())
}

async fn upsert_similarity_report(pool: &PgPool, item: &SimilarityCandidate) -> ApiResult<()> {
    let review_status = if item.flagged_for_review {
        SimilarityReviewStatus::Pending
    } else {
        SimilarityReviewStatus::None
    };

    sqlx::query(
        r#"
        INSERT INTO contract_similarity_reports (
            contract_id,
            similar_contract_id,
            similarity_score,
            exact_clone,
            match_type,
            suspicious,
            flagged_for_review,
            review_status,
            reasons
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (contract_id, similar_contract_id)
        DO UPDATE SET
            similarity_score = EXCLUDED.similarity_score,
            exact_clone = EXCLUDED.exact_clone,
            match_type = EXCLUDED.match_type,
            suspicious = EXCLUDED.suspicious,
            flagged_for_review = EXCLUDED.flagged_for_review,
            review_status = EXCLUDED.review_status,
            reasons = EXCLUDED.reasons,
            updated_at = NOW()
        "#,
    )
    .bind(item.contract_id)
    .bind(item.similar_contract_id)
    .bind(item.similarity_score)
    .bind(item.exact_clone)
    .bind(item.match_type.clone())
    .bind(item.suspicious)
    .bind(item.flagged_for_review)
    .bind(review_status)
    .bind(&item.reasons)
    .execute(pool)
    .await
    .map_err(|e| db_err("upsert similarity report", e))?;

    Ok(())
}

fn build_analysis_input(row: ContractAnalysisRow) -> ContractAnalysisInput {
    let (representation_type, raw_representation) = if let Some(source) = row.source_code {
        ("source_code".to_string(), source)
    } else if let Some(abi) = row.abi {
        ("abi".to_string(), abi.to_string())
    } else {
        (
            "metadata".to_string(),
            format!(
                "{} {} {} {}",
                row.name,
                row.category.clone().unwrap_or_default(),
                row.contract_id,
                row.wasm_hash
            ),
        )
    };

    let normalized_representation = normalize_text(&raw_representation);
    let features = feature_counts(&normalized_representation);
    let exact_hash = sha256_hex(normalized_representation.as_bytes());
    let simhash = simhash64(&features);
    let name_tokens = tokenize(&normalize_text(&row.name)).into_iter().collect();

    ContractAnalysisInput {
        id: row.id,
        contract_address: row.contract_id,
        name: row.name,
        publisher_id: row.publisher_id,
        category: row.category,
        wasm_hash: row.wasm_hash,
        is_verified: row.is_verified,
        total_interactions: row.total_interactions,
        representation_type,
        exact_hash,
        simhash,
        token_count: features.len(),
        source_length: raw_representation.len(),
        name_tokens,
    }
}

fn compare_contracts(
    target: &ContractAnalysisInput,
    other: &ContractAnalysisInput,
) -> Option<SimilarityCandidate> {
    let exact_wasm_match = target.wasm_hash == other.wasm_hash;
    let exact_representation_match = target.exact_hash == other.exact_hash;
    let hamming = hamming_distance(target.simhash, other.simhash);
    let simhash_similarity = 1.0 - (hamming as f64 / 64.0);
    let name_similarity = jaccard_similarity(&target.name_tokens, &other.name_tokens);
    let same_category = target.category.is_some() && target.category == other.category;

    let mut score = if exact_wasm_match {
        1.0
    } else if exact_representation_match {
        0.99
    } else {
        (simhash_similarity * 0.85)
            + (name_similarity * 0.15)
            + if same_category { 0.05 } else { 0.0 }
    };

    if score > 1.0 {
        score = 1.0;
    }

    if !exact_wasm_match && !exact_representation_match && score < 0.72 {
        return None;
    }

    let match_type = if exact_wasm_match || exact_representation_match {
        SimilarityMatchType::ExactClone
    } else if score >= 0.9 {
        SimilarityMatchType::NearDuplicate
    } else {
        SimilarityMatchType::Similar
    };

    let targets_popular_contract = other.is_verified || other.total_interactions >= 10;
    let suspicious = (matches!(match_type, SimilarityMatchType::ExactClone) || score >= 0.9)
        && target.publisher_id != other.publisher_id
        && targets_popular_contract;

    let reasons = json!({
        "representation_type": target.representation_type,
        "candidate_representation_type": other.representation_type,
        "exact_wasm_match": exact_wasm_match,
        "exact_representation_match": exact_representation_match,
        "simhash_similarity": round4(simhash_similarity),
        "name_similarity": round4(name_similarity),
        "same_category": same_category,
        "target_verified": target.is_verified,
        "candidate_verified": other.is_verified,
        "candidate_total_interactions": other.total_interactions,
        "hamming_distance": hamming
    });

    Some(SimilarityCandidate {
        contract_id: target.id,
        similar_contract_id: other.id,
        similar_contract_name: other.name.clone(),
        similar_contract_address: other.contract_address.clone(),
        similarity_score: round4(score),
        exact_clone: matches!(match_type, SimilarityMatchType::ExactClone),
        match_type,
        suspicious,
        flagged_for_review: suspicious,
        review_status: if suspicious {
            SimilarityReviewStatus::Pending
        } else {
            SimilarityReviewStatus::None
        },
        reasons,
    })
}

fn to_similarity_result(item: SimilarityCandidate) -> ContractSimilarityResult {
    ContractSimilarityResult {
        contract_id: item.contract_id,
        similar_contract_id: item.similar_contract_id,
        similar_contract_name: item.similar_contract_name,
        similar_contract_address: item.similar_contract_address,
        similarity_score: item.similarity_score,
        exact_clone: item.exact_clone,
        match_type: item.match_type,
        suspicious: item.suspicious,
        flagged_for_review: item.flagged_for_review,
        review_status: item.review_status,
        reasons: item.reasons,
    }
}

fn normalize_text(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn tokenize(input: &str) -> Vec<String> {
    input
        .split_whitespace()
        .filter(|token| token.len() >= 2)
        .map(ToString::to_string)
        .collect()
}

fn feature_counts(input: &str) -> HashMap<String, i32> {
    let mut counts = HashMap::new();
    for token in tokenize(input) {
        *counts.entry(token).or_insert(0) += 1;
    }

    let condensed: Vec<char> = input.chars().filter(|ch| !ch.is_whitespace()).collect();
    if condensed.len() >= 5 {
        for window in condensed.windows(5) {
            let shingle: String = window.iter().collect();
            *counts.entry(shingle).or_insert(0) += 1;
        }
    }

    counts
}

fn simhash64(features: &HashMap<String, i32>) -> u64 {
    let mut weights = [0i64; 64];
    for (feature, count) in features {
        let digest = Sha256::digest(feature.as_bytes());
        let mut hash_bytes = [0u8; 8];
        hash_bytes.copy_from_slice(&digest[..8]);
        let value = u64::from_be_bytes(hash_bytes);
        for (bit, weight) in weights.iter_mut().enumerate() {
            if value & (1u64 << bit) != 0 {
                *weight += *count as i64;
            } else {
                *weight -= *count as i64;
            }
        }
    }

    let mut result = 0u64;
    for (bit, weight) in weights.iter().enumerate() {
        if *weight >= 0 {
            result |= 1u64 << bit;
        }
    }
    result
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn hamming_distance(left: u64, right: u64) -> u32 {
    (left ^ right).count_ones()
}

fn jaccard_similarity(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }

    let intersection = left.intersection(right).count() as f64;
    let union = left.union(right).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
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

#[cfg(test)]
mod tests {
    use super::{feature_counts, hamming_distance, normalize_text, simhash64};

    #[test]
    fn normalizes_source_text_consistently() {
        assert_eq!(
            normalize_text("fn Transfer(amount: i64) { /* test */ }"),
            "fn transfer amount i64 test"
        );
    }

    #[test]
    fn similar_inputs_produce_close_hashes() {
        let a = simhash64(&feature_counts("transfer token amount to recipient"));
        let b = simhash64(&feature_counts("transfer token amount recipient"));
        assert!(hamming_distance(a, b) < 20);
    }
}
