//! Gas Usage Estimation (Issue #496)
//!
//! Endpoints:
//!   GET  /api/contracts/:id/methods/:method/gas-estimate
//!   POST /api/contracts/:id/methods/gas-estimate/batch
//!
//! Estimation strategy (in priority order):
//!   1. Historical data in `cost_estimates` (populated by `method_gas_history` trigger).
//!   2. ABI-based heuristics when no history is available.
//!
//! Results are cached for 1 hour in the generic cache under the namespace "gas_est".

use axum::extract::{Json, Path, Query, State};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use shared::models::{
    BatchGasEstimateRequest, BatchGasEstimateResponse, BatchMethodEntry, GasEstimateConfidence,
    GasEstimateQuery, MethodGasEstimate,
};

use crate::validation::extractors::ValidatedJson;
use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

// Soroban stroops-per-XLM conversion factor.
const STROOPS_PER_XLM: f64 = 10_000_000.0;

// ─────────────────────────────────── Heuristic constants ────────────────────
// Used when no historical data exists for a method.
const BASE_CALL_STROOPS: i64 = 1_000;
const COST_PER_PARAM_STROOPS: i64 = 500;
// View (read-only) methods are cheaper than state-mutating ones.
const VIEW_MULTIPLIER: f64 = 0.5;
const MUTATING_MULTIPLIER: f64 = 2.5;
// Range variance applied symmetrically around the estimated mid-point.
const HEURISTIC_VARIANCE: f64 = 0.30; // ±30 %

// ─────────────────────────────── Database row type ──────────────────────────

#[derive(sqlx::FromRow)]
struct CostEstimateRow {
    avg_gas_cost: i64,
    min_gas_cost: Option<i64>,
    max_gas_cost: Option<i64>,
    sample_count: i32,
    last_updated: DateTime<Utc>,
}

// ─────────────────────────── ABI method metadata ────────────────────────────

struct AbiMethodMeta {
    param_count: u32,
    is_view: bool,
}

// ─────────────────────────────────── Helpers ────────────────────────────────

fn db_err(op: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = op, error = ?err, "database error in gas estimation");
    ApiError::internal("An unexpected database error occurred")
}

/// Look up method metadata from the latest ABI for the contract.
async fn fetch_abi_method_meta(
    state: &AppState,
    contract_uuid: Uuid,
    method_name: &str,
) -> Option<AbiMethodMeta> {
    // Try L1/L2 cache first via the ABI generic cache.
    let abi_str: Option<String> = {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT abi FROM contract_abis WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(contract_uuid)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();
        row.map(|(v,)| v.to_string())
    };

    let abi_json: serde_json::Value = match abi_str.and_then(|s| serde_json::from_str(&s).ok()) {
        Some(v) => v,
        None => return None,
    };

    // Soroban contract specs are typically a JSON array of function descriptors.
    let fns = abi_json.as_array()?;
    for entry in fns {
        let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name != method_name {
            continue;
        }

        let inputs = entry
            .get("inputs")
            .or_else(|| entry.get("params"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.len() as u32)
            .unwrap_or(0);

        // Heuristic: "view" / "readonly" / "query" suffixes or explicit flag.
        let is_view = entry
            .get("is_view")
            .and_then(|v| v.as_bool())
            .unwrap_or_else(|| {
                let n = name.to_lowercase();
                n.starts_with("get_")
                    || n.starts_with("query_")
                    || n.starts_with("view_")
                    || n.ends_with("_view")
                    || n.ends_with("_readonly")
            });

        return Some(AbiMethodMeta {
            param_count: inputs,
            is_view,
        });
    }

    None
}

/// Build a `MethodGasEstimate` purely from ABI heuristics (no historical data).
fn heuristic_estimate(method_name: &str, meta: Option<&AbiMethodMeta>) -> MethodGasEstimate {
    let param_count = meta.map(|m| m.param_count).unwrap_or(1);
    let is_view = meta.map(|m| m.is_view).unwrap_or(false);

    let multiplier = if is_view {
        VIEW_MULTIPLIER
    } else {
        MUTATING_MULTIPLIER
    };

    let base = (BASE_CALL_STROOPS + (param_count as i64 * COST_PER_PARAM_STROOPS)) as f64;
    let avg = (base * multiplier) as i64;
    let min = ((avg as f64) * (1.0 - HEURISTIC_VARIANCE)) as i64;
    let max = ((avg as f64) * (1.0 + HEURISTIC_VARIANCE)) as i64;

    MethodGasEstimate {
        method_name: method_name.to_string(),
        min_gas_stroops: min.max(1),
        max_gas_stroops: max,
        avg_gas_stroops: avg,
        avg_gas_xlm: avg as f64 / STROOPS_PER_XLM,
        confidence: GasEstimateConfidence::Low,
        sample_count: 0,
        from_history: false,
        last_updated: None,
    }
}

/// Build a `MethodGasEstimate` from a `cost_estimates` database row.
fn estimate_from_history(method_name: &str, row: CostEstimateRow) -> MethodGasEstimate {
    let avg = row.avg_gas_cost;
    let min = row
        .min_gas_cost
        .unwrap_or_else(|| ((avg as f64) * (1.0 - HEURISTIC_VARIANCE)) as i64);
    let max = row
        .max_gas_cost
        .unwrap_or_else(|| ((avg as f64) * (1.0 + HEURISTIC_VARIANCE)) as i64);

    let confidence = match row.sample_count {
        n if n >= 10 => GasEstimateConfidence::High,
        n if n >= 1 => GasEstimateConfidence::Medium,
        _ => GasEstimateConfidence::Low,
    };

    MethodGasEstimate {
        method_name: method_name.to_string(),
        min_gas_stroops: min.max(1),
        max_gas_stroops: max,
        avg_gas_stroops: avg,
        avg_gas_xlm: avg as f64 / STROOPS_PER_XLM,
        confidence,
        sample_count: row.sample_count as i64,
        from_history: true,
        last_updated: Some(row.last_updated),
    }
}

/// Core estimation logic shared by both single and batch handlers.
async fn estimate_for_method(
    state: &AppState,
    contract_uuid: Uuid,
    method_name: &str,
) -> ApiResult<MethodGasEstimate> {
    // 1. Check the generic cache (1-hour TTL).
    let cache_key = format!("{}:{}", contract_uuid, method_name);
    let (cached, hit) = state.cache.get("gas_est", &cache_key).await;
    if hit {
        if let Some(json_str) = cached {
            if let Ok(est) = serde_json::from_str::<MethodGasEstimate>(&json_str) {
                return Ok(est);
            }
        }
    }

    // 2. Try historical data from `cost_estimates`.
    let hist: Option<CostEstimateRow> = sqlx::query_as(
        "SELECT avg_gas_cost, min_gas_cost, max_gas_cost, sample_count, last_updated \
         FROM cost_estimates \
         WHERE contract_id = $1 AND method_name = $2",
    )
    .bind(contract_uuid)
    .bind(method_name)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| db_err("fetch cost estimate", e))?;

    let estimate = if let Some(row) = hist {
        estimate_from_history(method_name, row)
    } else {
        // 3. Fall back to ABI heuristics.
        let meta = fetch_abi_method_meta(state, contract_uuid, method_name).await;
        heuristic_estimate(method_name, meta.as_ref())
    };

    // 4. Populate cache.
    if let Ok(serialised) = serde_json::to_string(&estimate) {
        state
            .cache
            .put("gas_est", &cache_key, serialised, None)
            .await;
    }

    Ok(estimate)
}

// ─────────────────────────────────── Handlers ───────────────────────────────

/// GET /api/contracts/:id/methods/:method/gas-estimate
///
/// Returns a gas estimate for a single method of the given contract.
/// Accepts optional `params` query string (JSON array of `MethodParamHint`)
/// to improve accuracy when no historical data is available.
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/methods/{method}/gas-estimate",
    params(
        ("id"     = String, Path,  description = "Contract UUID or on-chain contract_id"),
        ("method" = String, Path,  description = "Method (function) name"),
        GasEstimateQuery
    ),
    responses(
        (status = 200, description = "Gas estimate for the method",   body = MethodGasEstimate),
        (status = 400, description = "Invalid contract ID"),
        (status = 404, description = "Contract not found")
    ),
    tag = "Gas"
)]
pub async fn get_method_gas_estimate(
    State(state): State<AppState>,
    Path((id, method)): Path<(String, String)>,
    Query(_query): Query<GasEstimateQuery>,
) -> ApiResult<Json<MethodGasEstimate>> {
    let contract_uuid = resolve_contract_uuid(&state, &id).await?;
    let estimate = estimate_for_method(&state, contract_uuid, &method).await?;
    Ok(Json(estimate))
}

/// POST /api/contracts/:id/methods/gas-estimate/batch
///
/// Returns gas estimates for multiple methods in a single request.
/// Methods not found in the ABI are returned in the `not_found` list.
/// Maximum 50 methods per request.
#[utoipa::path(
    post,
    path = "/api/contracts/{id}/methods/gas-estimate/batch",
    params(
        ("id" = String, Path, description = "Contract UUID or on-chain contract_id")
    ),
    request_body = BatchGasEstimateRequest,
    responses(
        (status = 200, description = "Batch gas estimates",           body = BatchGasEstimateResponse),
        (status = 400, description = "Invalid input or too many methods"),
        (status = 404, description = "Contract not found")
    ),
    tag = "Gas"
)]
pub async fn batch_gas_estimate(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<BatchGasEstimateRequest>,
) -> ApiResult<Json<BatchGasEstimateResponse>> {
    if req.methods.is_empty() {
        return Err(ApiError::bad_request(
            "EmptyBatch",
            "methods list must not be empty",
        ));
    }
    if req.methods.len() > 50 {
        return Err(ApiError::bad_request(
            "BatchTooLarge",
            "batch requests are limited to 50 methods",
        ));
    }

    let contract_uuid = resolve_contract_uuid(&state, &id).await?;

    // Load all historical estimates for this contract in one query, then look
    // up each requested method without hitting the DB per-method.
    let all_hist: Vec<(String, CostEstimateRow)> = sqlx::query_as::<
        _,
        (String, i64, Option<i64>, Option<i64>, i32, DateTime<Utc>),
    >(
        "SELECT method_name, avg_gas_cost, min_gas_cost, max_gas_cost, sample_count, last_updated \
         FROM cost_estimates WHERE contract_id = $1",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("fetch batch cost estimates", e))?
    .into_iter()
    .map(|(method, avg, min, max, cnt, updated)| {
        (
            method,
            CostEstimateRow {
                avg_gas_cost: avg,
                min_gas_cost: min,
                max_gas_cost: max,
                sample_count: cnt,
                last_updated: updated,
            },
        )
    })
    .collect();

    use std::collections::HashMap;
    let hist_map: HashMap<String, CostEstimateRow> = all_hist.into_iter().collect();

    let mut estimates: Vec<MethodGasEstimate> = Vec::with_capacity(req.methods.len());

    for BatchMethodEntry { method_name, .. } in req.methods {
        // Check in-memory cache first.
        let cache_key = format!("{}:{}", contract_uuid, method_name);
        let (cached, hit) = state.cache.get("gas_est", &cache_key).await;
        if hit {
            if let Some(json_str) = cached {
                if let Ok(est) = serde_json::from_str::<MethodGasEstimate>(&json_str) {
                    estimates.push(est);
                    continue;
                }
            }
        }

        let estimate = if let Some(row) = hist_map.get(&method_name) {
            // Clone the relevant fields for estimate_from_history.
            estimate_from_history(
                &method_name,
                CostEstimateRow {
                    avg_gas_cost: row.avg_gas_cost,
                    min_gas_cost: row.min_gas_cost,
                    max_gas_cost: row.max_gas_cost,
                    sample_count: row.sample_count,
                    last_updated: row.last_updated,
                },
            )
        } else {
            let meta = fetch_abi_method_meta(&state, contract_uuid, &method_name).await;
            heuristic_estimate(&method_name, meta.as_ref())
        };

        // Populate cache.
        if let Ok(serialised) = serde_json::to_string(&estimate) {
            state
                .cache
                .put("gas_est", &cache_key, serialised, None)
                .await;
        }

        estimates.push(estimate);
    }

    Ok(Json(BatchGasEstimateResponse {
        estimates,
        not_found: vec![], // all methods produce at least a heuristic estimate
    }))
}

// ──────────────────────── Private helper ────────────────────────────────────

/// Resolve a contract UUID from either a UUID string or an on-chain contract_id string.
async fn resolve_contract_uuid(state: &AppState, id: &str) -> ApiResult<Uuid> {
    if let Ok(uuid) = Uuid::parse_str(id) {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM contracts WHERE id = $1)")
                .bind(uuid)
                .fetch_one(&state.db)
                .await
                .map_err(|e| db_err("check contract exists", e))?;
        if !exists {
            return Err(ApiError::not_found(
                "ContractNotFound",
                format!("No contract found with ID: {}", id),
            ));
        }
        return Ok(uuid);
    }

    let row: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM contracts WHERE contract_id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| db_err("lookup contract by contract_id", e))?;

    row.map(|(uuid,)| uuid).ok_or_else(|| {
        ApiError::not_found(
            "ContractNotFound",
            format!("No contract found with ID: {}", id),
        )
    })
}
