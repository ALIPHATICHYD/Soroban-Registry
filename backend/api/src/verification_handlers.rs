//! Verification status endpoints (issue #724)
//!
//! POST /api/contracts/{id}/verify           – submit a contract for verification
//! GET  /api/contracts/{id}/verification-status   – current status with 1-hour cache
//! GET  /api/contracts/{id}/verification-history  – chronological audit trail

use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, Query, State},
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

// ── Request / response types ──────────────────────────────────────────────────

/// Body for POST /api/contracts/{id}/verify
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ContractVerifyRequest {
    pub source_code: String,
    pub build_params: serde_json::Value,
    pub compiler_version: String,
    /// Optional free-text notes from the submitter
    #[serde(default)]
    pub notes: Option<String>,
}

/// Verification level filter for history queries
#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
pub struct VerificationHistoryQuery {
    /// Filter by verification level: Basic | Intermediate | Advanced
    pub level: Option<String>,
    /// Maximum number of history records to return (default 50)
    pub limit: Option<i64>,
}

/// Current verification status for a contract
#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VerificationStatusResponse {
    pub contract_id: String,
    pub verification_status: String,
    pub is_verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub verification_method: Option<String>,
    pub auditor: Option<String>,
    pub report_url: Option<String>,
    pub verification_notes: Option<String>,
    pub cached: bool,
}

/// One entry in the verification history
#[derive(Debug, Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct VerificationHistoryEntry {
    pub id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub changed_by: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Response for the verification history endpoint
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct VerificationHistoryResponse {
    pub contract_id: String,
    pub total: usize,
    pub history: Vec<VerificationHistoryEntry>,
}

/// Response returned after submitting a verification request
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct VerificationSubmitResponse {
    pub verification_id: Uuid,
    pub contract_id: String,
    pub status: String,
    pub message: String,
    pub submitted_at: DateTime<Utc>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn db_err(op: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = op, error = ?err, "database error in verification handler");
    ApiError::internal("An unexpected database error occurred")
}

async fn resolve_contract_uuid(state: &AppState, id: &str) -> ApiResult<(Uuid, String)> {
    if let Ok(uuid) = Uuid::parse_str(id) {
        let contract_id: Option<String> =
            sqlx::query_scalar("SELECT contract_id FROM contracts WHERE id = $1")
                .bind(uuid)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| db_err("resolve contract by uuid", e))?;
        if let Some(cid) = contract_id {
            return Ok((uuid, cid));
        }
    }
    let row: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, contract_id FROM contracts WHERE contract_id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| db_err("resolve contract by address", e))?;
    row.ok_or_else(|| ApiError::not_found("ContractNotFound", "Contract not found"))
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Submit a contract for verification.
///
/// Creates a `pending` verification record and queues the contract for
/// source-code / on-chain verification.  Returns immediately with the new
/// record ID and initial status.
#[utoipa::path(
    post,
    path = "/api/contracts/{id}/verify",
    params(("id" = String, Path, description = "Contract UUID or on-chain address")),
    request_body = ContractVerifyRequest,
    responses(
        (status = 201, description = "Verification submitted", body = VerificationSubmitResponse),
        (status = 404, description = "Contract not found"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Verification"
)]
pub async fn submit_contract_verification(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<ContractVerifyRequest>,
) -> ApiResult<Json<VerificationSubmitResponse>> {
    let (contract_uuid, contract_address) = resolve_contract_uuid(&state, &id).await?;

    // Wrap the INSERT + conditional UPDATE in a transaction so that:
    //   • The verifications row and the contracts.verification_status change
    //     are committed atomically.
    //   • SELECT … FOR UPDATE prevents two concurrent submitters from both
    //     seeing 'unverified' and both transitioning to 'pending'.
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| db_err("begin submit-verification transaction", e))?;

    // Lock the contracts row for the duration of this transaction.
    sqlx::query("SELECT id FROM contracts WHERE id = $1 FOR UPDATE")
        .bind(contract_uuid)
        .execute(&mut *tx)
        .await
        .map_err(|e| db_err("lock contract row for submit verification", e))?;

    let verification_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO verifications
            (contract_id, status, source_code, build_params, compiler_version,
             error_message, version)
        VALUES ($1, 'pending', $2, $3, $4, NULL, 0)
        RETURNING id
        "#,
    )
    .bind(contract_uuid)
    .bind(&req.source_code)
    .bind(&req.build_params)
    .bind(&req.compiler_version)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| db_err("insert verification record", e))?;

    // Transition contract status to 'pending' only if it is currently
    // 'unverified'.  The FOR UPDATE lock above ensures no other transaction
    // can race this conditional update.
    sqlx::query(
        r#"
        UPDATE contracts
        SET    verification_status  = 'pending',
               verification_version = verification_version + 1,
               updated_at           = NOW()
        WHERE  id = $1
          AND  verification_status = 'unverified'
        "#,
    )
    .bind(contract_uuid)
    .execute(&mut *tx)
    .await
    .map_err(|e| db_err("set contract verification_status to pending", e))?;

    tx.commit()
        .await
        .map_err(|e| db_err("commit submit-verification transaction", e))?;

    // Invalidate cached status so the next GET reflects the pending state.
    state.cache.invalidate("verification_status", &id).await;

    Ok(Json(VerificationSubmitResponse {
        verification_id,
        contract_id: contract_address,
        status: "pending".to_string(),
        message: "Verification request submitted and queued for processing".to_string(),
        submitted_at: Utc::now(),
    }))
}

/// Get the current verification status for a contract.
///
/// Results are cached for 1 hour (TTL managed by the generic cache).
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/verification-status",
    params(("id" = String, Path, description = "Contract UUID or on-chain address")),
    responses(
        (status = 200, description = "Verification status", body = VerificationStatusResponse),
        (status = 404, description = "Contract not found")
    ),
    tag = "Verification"
)]
pub async fn get_contract_verification_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<VerificationStatusResponse>> {
    let cache_key = format!("status:{}", id);
    let (cached_value, hit) = state.cache.get("verification_status", &cache_key).await;
    if hit {
        if let Some(raw) = cached_value {
            if let Ok(mut resp) = serde_json::from_str::<VerificationStatusResponse>(&raw) {
                resp.cached = true;
                return Ok(Json(resp));
            }
        }
    }

    let (contract_uuid, contract_address) = resolve_contract_uuid(&state, &id).await?;

    // Fetch current status + latest verification record in one query
    let row: Option<(
        String,                // verification_status
        bool,                  // is_verified
        Option<DateTime<Utc>>, // verified_at
        Option<Uuid>,          // verified_by
        Option<String>,        // verification_notes
        Option<String>,        // compiler_version (used as verification_method)
    )> = sqlx::query_as(
        r#"
        SELECT
            c.verification_status::TEXT,
            c.is_verified,
            c.verified_at,
            c.verified_by,
            c.verification_notes,
            v.compiler_version
        FROM   contracts c
        LEFT   JOIN verifications v
               ON  v.contract_id = c.id
               AND v.id = (
                   SELECT id FROM verifications
                   WHERE  contract_id = c.id
                   ORDER  BY created_at DESC
                   LIMIT  1
               )
        WHERE  c.id = $1
        "#,
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| db_err("fetch verification status", e))?;

    let (vs, is_verified, verified_at, verified_by, notes, method) =
        row.ok_or_else(|| ApiError::not_found("ContractNotFound", "Contract not found"))?;

    let resp = VerificationStatusResponse {
        contract_id: contract_address,
        verification_status: vs,
        is_verified,
        verified_at,
        verification_method: method,
        auditor: verified_by.map(|u| u.to_string()),
        report_url: None,
        verification_notes: notes,
        cached: false,
    };

    // Cache for 1 hour
    if let Ok(serialized) = serde_json::to_string(&resp) {
        state
            .cache
            .put(
                "verification_status",
                &cache_key,
                serialized,
                Some(Duration::from_secs(3600)),
            )
            .await;
    }

    Ok(Json(resp))
}

/// Get the chronological verification history for a contract (newest first).
///
/// Optionally filter by verification level (Basic | Intermediate | Advanced).
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/verification-history",
    params(
        ("id" = String, Path, description = "Contract UUID or on-chain address"),
        VerificationHistoryQuery
    ),
    responses(
        (status = 200, description = "Verification history", body = VerificationHistoryResponse),
        (status = 404, description = "Contract not found")
    ),
    tag = "Verification"
)]
pub async fn get_contract_verification_history(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<VerificationHistoryQuery>,
) -> ApiResult<Json<VerificationHistoryResponse>> {
    let (contract_uuid, contract_address) = resolve_contract_uuid(&state, &id).await?;

    let limit = query.limit.unwrap_or(50).max(1).min(200);

    // verification_events holds the audit trail written by the DB trigger
    // (migration 20260329093000_add_contract_verification.sql).
    let mut history: Vec<VerificationHistoryEntry> = sqlx::query_as(
        r#"
        SELECT id, from_status::TEXT AS from_status, to_status::TEXT AS to_status,
               changed_by, notes, created_at
        FROM   verification_events
        WHERE  contract_id = $1
        ORDER  BY created_at DESC
        LIMIT  $2
        "#,
    )
    .bind(contract_uuid)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_err("fetch verification history", e))?;

    // Optional level filter (Basic / Intermediate / Advanced) applied post-fetch
    // so we don't need to store that field in the DB for now.
    if let Some(level) = &query.level {
        let level_lower = level.to_ascii_lowercase();
        history.retain(|entry| {
            entry
                .notes
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase()
                .contains(&level_lower)
                || entry.to_status.to_ascii_lowercase().contains(&level_lower)
        });
    }

    let total = history.len();
    Ok(Json(VerificationHistoryResponse {
        contract_id: contract_address,
        total,
        history,
    }))
}
