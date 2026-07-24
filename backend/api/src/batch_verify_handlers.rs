use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use futures_util::stream::{self, StreamExt};
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use shared::{
    BatchVerifyItem, BatchVerifyJobResponse, BatchVerifyJobResult, BatchVerifyJobStatus,
    BatchVerifyRequest, Contract,
};
use std::collections::HashMap;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{onchain_verification::OnChainVerifier, state::AppState};

// ── In-memory job store ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct BatchVerifyJob {
    request: BatchVerifyRequest,
    response: BatchVerifyJobResponse,
}

static BATCH_JOBS: Lazy<RwLock<HashMap<Uuid, BatchVerifyJob>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

// ── Synchronous batch verify (original endpoint, kept for backwards compat) ───

/// POST /api/contracts/batch-verify
///
/// Synchronous batch: verifies up to 50 contracts inline and returns all results
/// before the request completes. For larger batches use the async job endpoints.
pub async fn batch_verify_contracts(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<BatchVerifyRequest>,
) -> impl IntoResponse {
    const SYNC_BATCH_LIMIT: usize = 50;

    if req.contracts.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "invalid_request",
                "message": "contracts must not be empty"
            })),
        );
    }

    if req.contracts.len() > SYNC_BATCH_LIMIT {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "batch_too_large",
                "message": format!(
                    "Synchronous batch is limited to {SYNC_BATCH_LIMIT} contracts. \
                     Use POST /api/contracts/batch-verify/jobs for larger batches."
                ),
                "limit": SYNC_BATCH_LIMIT,
                "submitted": req.contracts.len()
            })),
        );
    }

    let verifier = OnChainVerifier::new();
    let results = stream::iter(req.contracts.into_iter().map(|item| {
        let state = state.clone();
        let verifier = verifier.clone();
        async move { verify_batch_item(&state, &verifier, item).await }
    }))
    .buffer_unordered(8)
    .collect::<Vec<Value>>()
    .await;

    let verified = results
        .iter()
        .filter(|r| r.get("verified").and_then(Value::as_bool) == Some(true))
        .count();
    let cached = results
        .iter()
        .filter(|r| r.pointer("/on_chain/cached").and_then(Value::as_bool) == Some(true))
        .count();

    (
        StatusCode::OK,
        Json(json!({
            "total": results.len(),
            "verified": verified,
            "failed": results.len().saturating_sub(verified),
            "cached": cached,
            "results": results
        })),
    )
}

// ── Async job endpoints ───────────────────────────────────────────────────────

/// POST /api/contracts/batch-verify/jobs
///
/// Submits a batch verification job. Returns immediately with a job ID and
/// status URL. Poll `GET /api/contracts/batch-verify/jobs/:job_id` for results.
pub async fn submit_batch_verify_job(
    State(state): State<AppState>,
    Json(req): Json<BatchVerifyRequest>,
) -> impl IntoResponse {
    const MAX_JOB_BATCH: usize = 500;

    if req.contracts.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "invalid_request",
                "message": "contracts must not be empty"
            })),
        )
            .into_response();
    }

    if req.contracts.len() > MAX_JOB_BATCH {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "batch_too_large",
                "message": format!(
                    "Batch jobs are limited to {MAX_JOB_BATCH} contracts per request."
                ),
                "limit": MAX_JOB_BATCH,
                "submitted": req.contracts.len()
            })),
        )
            .into_response();
    }

    let job_id = Uuid::new_v4();
    let total = req.contracts.len();
    let submitted_at = Utc::now();
    let status_url = format!("/api/contracts/batch-verify/jobs/{job_id}");

    let response = BatchVerifyJobResponse {
        job_id,
        status: BatchVerifyJobStatus::Pending,
        total,
        verified: 0,
        failed: 0,
        submitted_at,
        completed_at: None,
        results: Vec::new(),
        status_url: status_url.clone(),
    };

    BATCH_JOBS.write().await.insert(
        job_id,
        BatchVerifyJob {
            request: req.clone(),
            response: response.clone(),
        },
    );

    // Mark as Processing immediately so pollers see the transition.
    {
        let mut jobs = BATCH_JOBS.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            job.response.status = BatchVerifyJobStatus::Processing;
        }
    }

    // Spawn the background worker.
    tokio::spawn(run_batch_verify_job(job_id, req, state));

    (StatusCode::ACCEPTED, Json(response)).into_response()
}

/// GET /api/contracts/batch-verify/jobs/:job_id
///
/// Returns the current status and, once complete, per-contract results.
pub async fn get_batch_verify_job(Path(job_id): Path<Uuid>) -> impl IntoResponse {
    let jobs = BATCH_JOBS.read().await;
    match jobs.get(&job_id) {
        Some(job) => (StatusCode::OK, Json(job.response.clone())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "job_not_found",
                "message": "No batch verification job found for the supplied ID"
            })),
        )
            .into_response(),
    }
}

// ── Background worker ─────────────────────────────────────────────────────────

async fn run_batch_verify_job(job_id: Uuid, req: BatchVerifyRequest, state: AppState) {
    let verifier = OnChainVerifier::new();

    let results: Vec<BatchVerifyJobResult> = stream::iter(req.contracts.into_iter().map(|item| {
        let state = state.clone();
        let verifier = verifier.clone();
        async move { run_single_item(&state, &verifier, item).await }
    }))
    .buffer_unordered(8)
    .collect()
    .await;

    let verified = results.iter().filter(|r| r.verified).count();
    let failed = results.len().saturating_sub(verified);

    let status = if failed == 0 {
        BatchVerifyJobStatus::Completed
    } else if verified == 0 {
        BatchVerifyJobStatus::Failed
    } else {
        BatchVerifyJobStatus::PartialFailure
    };

    let mut jobs = BATCH_JOBS.write().await;
    if let Some(job) = jobs.get_mut(&job_id) {
        job.response.status = status;
        job.response.verified = verified;
        job.response.failed = failed;
        job.response.completed_at = Some(Utc::now());
        job.response.results = results;
    }
}

async fn run_single_item(
    state: &AppState,
    onchain_verifier: &OnChainVerifier,
    item: BatchVerifyItem,
) -> BatchVerifyJobResult {
    let contract = match sqlx::query_as::<_, Contract>(
        "SELECT * FROM contracts WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&item.contract_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(c)) => c,
        Ok(None) => {
            return BatchVerifyJobResult {
                contract_id: item.contract_id,
                verified: false,
                error: Some("contract not found in registry".to_string()),
                network: None,
                wasm_hash_matches: None,
                abi_valid: None,
            };
        }
        Err(err) => {
            return BatchVerifyJobResult {
                contract_id: item.contract_id,
                verified: false,
                error: Some(format!("database error: {}", err)),
                network: None,
                wasm_hash_matches: None,
                abi_valid: None,
            };
        }
    };

    let abi_json = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT abi FROM contract_abis WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(contract.id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .map(|v| v.to_string());

    let on_chain = match onchain_verifier
        .verify_contract(&state.cache, &contract, abi_json.as_deref())
        .await
    {
        Ok(r) => r,
        Err(err) => {
            return BatchVerifyJobResult {
                contract_id: contract.contract_id,
                verified: false,
                error: Some(err.to_string()),
                network: Some(contract.network.to_string()),
                wasm_hash_matches: None,
                abi_valid: None,
            };
        }
    };

    let source_ok = match (&item.source_code, &item.compiler_version) {
        (Some(src), Some(ver)) if !src.trim().is_empty() => {
            match verifier::verify_contract(
                src,
                &contract.wasm_hash,
                Some(ver),
                item.build_params.as_ref(),
            )
            .await
            {
                Ok(r) => r.verified,
                Err(_) => false,
            }
        }
        _ => true, // no source provided — skip source check
    };

    let verified = on_chain.contract_exists_on_chain
        && on_chain.wasm_hash_matches
        && on_chain.abi_valid
        && source_ok;

    let failure_summary = if !verified {
        let reasons: Vec<String> = on_chain
            .failure_reasons
            .iter()
            .map(|r| r.to_string())
            .collect();
        if reasons.is_empty() {
            None
        } else {
            Some(reasons.join("; "))
        }
    } else {
        None
    };

    BatchVerifyJobResult {
        contract_id: contract.contract_id,
        verified,
        error: failure_summary,
        network: Some(contract.network.to_string()),
        wasm_hash_matches: Some(on_chain.wasm_hash_matches),
        abi_valid: Some(on_chain.abi_valid),
    }
}

// ── Legacy synchronous helper (kept to avoid duplicating DB logic) ────────────

async fn verify_batch_item(
    state: &AppState,
    onchain_verifier: &OnChainVerifier,
    item: BatchVerifyItem,
) -> Value {
    let level = item.level.as_deref().unwrap_or("standard");

    let contract = match sqlx::query_as::<_, Contract>(
        "SELECT * FROM contracts WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&item.contract_id)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(contract)) => contract,
        Ok(None) => {
            return json!({
                "contract_id": item.contract_id,
                "verified": false,
                "error": "contract_not_found"
            });
        }
        Err(err) => {
            return json!({
                "contract_id": item.contract_id,
                "verified": false,
                "error": format!("database error: {}", err)
            });
        }
    };

    // basic: on-chain existence + wasm hash only — skip ABI and source
    if level == "basic" {
        let on_chain = match onchain_verifier
            .verify_contract(&state.cache, &contract, None)
            .await
        {
            Ok(result) => result,
            Err(err) => {
                return json!({
                    "contract_id": contract.contract_id,
                    "verified": false,
                    "error": err.to_string()
                });
            }
        };
        let verified = on_chain.contract_exists_on_chain && on_chain.wasm_hash_matches;
        return json!({
            "contract_id": contract.contract_id,
            "verified": verified,
            "level": "basic",
            "on_chain": on_chain
        });
    }

    // strict: source_code and compiler_version are mandatory
    if level == "strict" {
        match (&item.source_code, &item.compiler_version) {
            (Some(sc), Some(cv)) if !sc.trim().is_empty() && !cv.trim().is_empty() => {}
            _ => {
                return json!({
                    "contract_id": contract.contract_id,
                    "verified": false,
                    "error": "strict verification requires source_code and compiler_version"
                });
            }
        }
    }

    // standard / strict: full on-chain + optional source verification
    let abi_json = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT abi FROM contract_abis WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(contract.id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .map(|value| value.to_string());

    let on_chain = match onchain_verifier
        .verify_contract(&state.cache, &contract, abi_json.as_deref())
        .await
    {
        Ok(result) => result,
        Err(err) => {
            return json!({
                "contract_id": contract.contract_id,
                "verified": false,
                "error": err.to_string()
            });
        }
    };

    let source_verification = match (&item.source_code, &item.compiler_version) {
        (Some(source_code), Some(compiler_version)) if !source_code.trim().is_empty() => Some(
            verifier::verify_contract(
                source_code,
                &contract.wasm_hash,
                Some(compiler_version),
                item.build_params.as_ref(),
            )
            .await,
        ),
        _ => None,
    };

    let verified = on_chain.contract_exists_on_chain
        && on_chain.wasm_hash_matches
        && on_chain.abi_valid
        && source_verification
            .as_ref()
            .map(|r| r.as_ref().map(|v| v.verified).unwrap_or(false))
            .unwrap_or(true);

    let failure_reasons = on_chain.failure_reasons.clone();
    json!({
        "contract_id": contract.contract_id,
        "verified": verified,
        "level": level,
        "network": contract.network.to_string(),
        "on_chain": on_chain,
        "failure_reasons": failure_reasons,
        "source_verification": source_verification.map(|r| match r {
            Ok(v) => json!({
                "verified": v.verified,
                "compiled_wasm_hash": v.compiled_wasm_hash,
                "deployed_wasm_hash": v.deployed_wasm_hash,
                "message": v.message,
                "failure_kind": v.failure_kind
            }),
            Err(err) => json!({
                "verified": false,
                "error": err.to_string()
            })
        })
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use shared::BatchVerifyJobStatus;

    #[test]
    fn pending_job_serialises_without_results() {
        let resp = BatchVerifyJobResponse {
            job_id: Uuid::nil(),
            status: BatchVerifyJobStatus::Pending,
            total: 3,
            verified: 0,
            failed: 0,
            submitted_at: Utc::now(),
            completed_at: None,
            results: Vec::new(),
            status_url: "/api/contracts/batch-verify/jobs/00000000-0000-0000-0000-000000000000"
                .to_string(),
        };

        let json = serde_json::to_string(&resp).expect("serialisation should not fail");
        assert!(json.contains("\"status\":\"pending\""));
        assert!(!json.contains("\"results\":[{"));
    }

    #[test]
    fn partial_failure_status_when_some_contracts_fail() {
        let results = vec![
            BatchVerifyJobResult {
                contract_id: "CA".to_string(),
                verified: true,
                error: None,
                network: Some("testnet".to_string()),
                wasm_hash_matches: Some(true),
                abi_valid: Some(true),
            },
            BatchVerifyJobResult {
                contract_id: "CB".to_string(),
                verified: false,
                error: Some("contract not found in registry".to_string()),
                network: None,
                wasm_hash_matches: None,
                abi_valid: None,
            },
        ];

        let verified = results.iter().filter(|r| r.verified).count();
        let failed = results.len().saturating_sub(verified);

        let status = if failed == 0 {
            BatchVerifyJobStatus::Completed
        } else if verified == 0 {
            BatchVerifyJobStatus::Failed
        } else {
            BatchVerifyJobStatus::PartialFailure
        };

        assert_eq!(status, BatchVerifyJobStatus::PartialFailure);
        assert_eq!(verified, 1);
        assert_eq!(failed, 1);
    }

    #[test]
    fn all_failed_produces_failed_status() {
        let results = vec![
            BatchVerifyJobResult {
                contract_id: "CA".to_string(),
                verified: false,
                error: Some("not found".to_string()),
                network: None,
                wasm_hash_matches: None,
                abi_valid: None,
            },
            BatchVerifyJobResult {
                contract_id: "CB".to_string(),
                verified: false,
                error: Some("not found".to_string()),
                network: None,
                wasm_hash_matches: None,
                abi_valid: None,
            },
        ];

        let verified = results.iter().filter(|r| r.verified).count();
        let failed = results.len().saturating_sub(verified);

        let status = if failed == 0 {
            BatchVerifyJobStatus::Completed
        } else if verified == 0 {
            BatchVerifyJobStatus::Failed
        } else {
            BatchVerifyJobStatus::PartialFailure
        };

        assert_eq!(status, BatchVerifyJobStatus::Failed);
    }

    #[test]
    fn failed_result_includes_error_omits_optional_fields_on_unknown_contract() {
        let result = BatchVerifyJobResult {
            contract_id: "CX".to_string(),
            verified: false,
            error: Some("contract not found in registry".to_string()),
            network: None,
            wasm_hash_matches: None,
            abi_valid: None,
        };

        let json = serde_json::to_string(&result).expect("serialisation should not fail");
        assert!(json.contains("\"error\""));
        // Optional None fields should be omitted (skip_serializing_if)
        assert!(!json.contains("\"network\""));
        assert!(!json.contains("\"wasm_hash_matches\""));
    }

    #[tokio::test]
    async fn get_job_returns_404_for_unknown_id() {
        let id = Uuid::new_v4();
        let jobs = BATCH_JOBS.read().await;
        assert!(jobs.get(&id).is_none());
    }
}
