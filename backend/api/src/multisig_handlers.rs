use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared::Network;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    metrics,
    state::AppState,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "proposal_status", rename_all = "lowercase")]
pub enum ProposalStatus {
    Pending,
    Approved,
    Executed,
    Expired,
    Rejected,
}

impl ProposalStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Executed => "executed",
            Self::Expired => "expired",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalDecision {
    Approved,
    Rejected,
}

impl ApprovalDecision {
    fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateMultisigPolicyRequest {
    pub name: String,
    pub threshold: i32,
    pub signer_addresses: Vec<String>,
    pub expiry_seconds: Option<i32>,
    pub created_by: String,
    pub ordered_approvals: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePublisherKeyRequest {
    pub key_name: String,
    pub public_key: String,
    pub algorithm: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDeployProposalRequest {
    pub contract_name: String,
    pub contract_id: String,
    pub wasm_hash: String,
    pub network: Network,
    pub description: Option<String>,
    pub policy_id: Uuid,
    pub proposer: String,
}

#[derive(Debug, Deserialize)]
pub struct SignProposalRequest {
    pub signer_address: String,
    pub signature_data: Option<String>,
    pub decision: Option<ApprovalDecision>,
    pub comment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListProposalsQuery {
    pub status: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MultisigPolicy {
    pub id: Uuid,
    pub name: String,
    pub threshold: i32,
    pub signer_addresses: Vec<String>,
    pub expiry_seconds: i32,
    pub ordered_approvals: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct PublisherMultisigKey {
    pub id: Uuid,
    pub publisher_id: Uuid,
    pub key_name: String,
    pub public_key: String,
    pub algorithm: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct DeployProposal {
    pub id: Uuid,
    pub contract_name: String,
    pub contract_id: String,
    pub wasm_hash: String,
    pub network: Network,
    pub description: Option<String>,
    pub policy_id: Uuid,
    pub status: ProposalStatus,
    pub expires_at: DateTime<Utc>,
    pub executed_at: Option<DateTime<Utc>>,
    pub approved_at: Option<DateTime<Utc>>,
    pub rejected_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub proposer: String,
    pub required_approvals: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ProposalSignature {
    pub signer_address: String,
    pub signature_data: Option<String>,
    pub signed_at: DateTime<Utc>,
    pub decision: String,
    pub comment: Option<String>,
    pub step_index: Option<i32>,
    pub reviewed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListProposalsResponse {
    pub items: Vec<DeployProposal>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct SignProposalResponse {
    pub signatures_collected: i64,
    pub signatures_needed: i64,
    pub threshold_met: bool,
    pub proposal_status: String,
}

#[derive(Debug, Serialize)]
pub struct ExecuteProposalResponse {
    pub contract_id: String,
    pub wasm_hash: String,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ProposalInfoResponse {
    pub proposal: DeployProposal,
    pub policy: MultisigPolicy,
    pub signatures: Vec<ProposalSignature>,
    pub signatures_needed: i64,
}

#[derive(Debug, FromRow)]
struct ProposalSigningState {
    status: ProposalStatus,
    expires_at: DateTime<Utc>,
    required_approvals: i32,
    signer_addresses: Vec<String>,
    ordered_approvals: bool,
}

pub async fn create_policy(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateMultisigPolicyRequest>,
) -> ApiResult<Json<MultisigPolicy>> {
    if payload.name.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidName",
            "Policy name cannot be empty",
        ));
    }

    if payload.created_by.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidCreator",
            "created_by cannot be empty",
        ));
    }

    if payload.signer_addresses.is_empty() {
        return Err(ApiError::bad_request(
            "InvalidSigners",
            "At least one signer is required",
        ));
    }

    if payload.threshold < 1 || payload.threshold as usize > payload.signer_addresses.len() {
        return Err(ApiError::bad_request(
            "InvalidThreshold",
            "threshold must be between 1 and the number of signers",
        ));
    }

    let unique_signers: std::collections::HashSet<&String> =
        payload.signer_addresses.iter().collect();
    if unique_signers.len() != payload.signer_addresses.len() {
        return Err(ApiError::bad_request(
            "DuplicateSigners",
            "signer_addresses must not contain duplicates",
        ));
    }

    let expiry_seconds = payload.expiry_seconds.unwrap_or(86400);
    if expiry_seconds < 60 {
        return Err(ApiError::bad_request(
            "InvalidExpiry",
            "expiry_seconds must be at least 60 seconds",
        ));
    }

    let ordered_approvals = payload.ordered_approvals.unwrap_or(false);

    let policy: MultisigPolicy = sqlx::query_as(
        "INSERT INTO multisig_policies (
            name, threshold, signer_addresses, expiry_seconds, created_by, ordered_approvals
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, name, threshold, signer_addresses, expiry_seconds, ordered_approvals, created_by, created_at",
    )
    .bind(payload.name.trim())
    .bind(payload.threshold)
    .bind(payload.signer_addresses)
    .bind(expiry_seconds)
    .bind(payload.created_by.trim())
    .bind(ordered_approvals)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to create multisig policy");
        ApiError::db_error("Failed to create multisig policy")
    })?;

    Ok(Json(policy))
}

pub async fn create_deploy_proposal(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<CreateDeployProposalRequest>,
) -> ApiResult<Json<DeployProposal>> {
    if payload.contract_name.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidContractName",
            "contract_name cannot be empty",
        ));
    }
    if payload.contract_id.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidContractId",
            "contract_id cannot be empty",
        ));
    }
    if payload.wasm_hash.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidWasmHash",
            "wasm_hash cannot be empty",
        ));
    }
    if payload.proposer.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidProposer",
            "proposer cannot be empty",
        ));
    }

    let policy = sqlx::query_as::<_, MultisigPolicy>(
        "SELECT id, name, threshold, signer_addresses, expiry_seconds, ordered_approvals, created_by, created_at
         FROM multisig_policies
         WHERE id = $1",
    )
    .bind(payload.policy_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to fetch multisig policy");
        ApiError::db_error("Failed to load multisig policy")
    })?
    .ok_or_else(|| ApiError::not_found("PolicyNotFound", "multisig policy not found"))?;

    let expires_at = Utc::now() + chrono::Duration::seconds(i64::from(policy.expiry_seconds));

    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = ?e, "failed to start transaction");
        ApiError::db_error("Failed to create deploy proposal")
    })?;

    let proposal: DeployProposal = sqlx::query_as(
        "INSERT INTO deploy_proposals (
            contract_name, contract_id, wasm_hash, network, description,
            policy_id, status, expires_at, proposer, required_approvals
         )
         VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, $8, $9)
         RETURNING
            id, contract_name, contract_id, wasm_hash, network, description,
            policy_id, status, expires_at, executed_at, approved_at, rejected_at,
            rejection_reason, proposer, required_approvals, created_at, updated_at",
    )
    .bind(payload.contract_name.trim())
    .bind(payload.contract_id.trim())
    .bind(payload.wasm_hash.trim())
    .bind(payload.network)
    .bind(payload.description.as_deref())
    .bind(payload.policy_id)
    .bind(expires_at)
    .bind(payload.proposer.trim())
    .bind(policy.threshold)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to create deploy proposal");
        ApiError::db_error("Failed to create deploy proposal")
    })?;

    for signer in &policy.signer_addresses {
        sqlx::query(
            "INSERT INTO multisig_approval_notifications (
                proposal_id, signer_address, notification_type, payload
             )
             VALUES ($1, $2, 'approval_requested', $3)",
        )
        .bind(proposal.id)
        .bind(signer)
        .bind(json!({
            "proposal_id": proposal.id,
            "contract_id": &proposal.contract_id,
            "network": &proposal.network,
        }))
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to queue multisig notification");
            ApiError::db_error("Failed to queue multisig notifications")
        })?;
    }

    sqlx::query(
        "INSERT INTO multisig_approval_audit_events (
            proposal_id, actor_address, action, metadata
         )
         VALUES ($1, $2, 'proposal_created', $3)",
    )
    .bind(proposal.id)
    .bind(payload.proposer.trim())
    .bind(json!({
        "required_approvals": proposal.required_approvals,
        "ordered_approvals": policy.ordered_approvals,
    }))
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to insert audit event");
        ApiError::db_error("Failed to record audit trail")
    })?;

    tx.commit().await.map_err(|e| {
        tracing::error!(error = ?e, "failed to commit deploy proposal transaction");
        ApiError::db_error("Failed to finalize deploy proposal")
    })?;

    metrics::MULTISIG_PROPOSALS.inc();
    Ok(Json(proposal))
}

pub async fn sign_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(payload): ValidatedJson<SignProposalRequest>,
) -> ApiResult<Json<SignProposalResponse>> {
    let proposal_id = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request_with("InvalidProposalId", "proposal id must be a valid UUID")
    })?;

    if payload.signer_address.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidSigner",
            "signer_address cannot be empty",
        ));
    }

    let decision = payload.decision.unwrap_or(ApprovalDecision::Approved);
    let signer = payload.signer_address.trim().to_string();

    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = ?e, "failed to start signing transaction");
        ApiError::db_error("Failed to sign proposal")
    })?;

    let signing_state = sqlx::query_as::<_, ProposalSigningState>(
        "SELECT
            p.status,
            p.expires_at,
            p.required_approvals,
            mp.signer_addresses,
            mp.ordered_approvals
         FROM deploy_proposals p
         JOIN multisig_policies mp ON mp.id = p.policy_id
         WHERE p.id = $1
         FOR UPDATE",
    )
    .bind(proposal_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to load proposal");
        ApiError::db_error("Failed to load proposal")
    })?
    .ok_or_else(|| ApiError::not_found("ProposalNotFound", "deployment proposal not found"))?;

    if signing_state.status != ProposalStatus::Pending {
        return Err(ApiError::conflict(
            "InvalidProposalState",
            format!(
                "proposal cannot be signed while in '{}' state",
                signing_state.status.as_str()
            ),
        ));
    }

    if signing_state.expires_at <= Utc::now() {
        sqlx::query(
            "UPDATE deploy_proposals SET status = 'expired', updated_at = NOW() WHERE id = $1",
        )
        .bind(proposal_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to mark proposal expired");
            ApiError::db_error("Failed to update proposal status")
        })?;
        tx.commit().await.map_err(|e| {
            tracing::error!(error = ?e, "failed to finalize expiry update");
            ApiError::db_error("Failed to update proposal status")
        })?;
        return Err(ApiError::conflict(
            "ProposalExpired",
            "proposal has already expired",
        ));
    }

    if !signing_state.signer_addresses.iter().any(|s| s == &signer) {
        return Err(ApiError::forbidden(
            "signer_address is not authorized by this multisig policy",
        ));
    }

    let mut step_index: Option<i32> = None;
    if signing_state.ordered_approvals {
        let signer_position = signing_state
            .signer_addresses
            .iter()
            .position(|address| address == &signer)
            .ok_or_else(|| {
                ApiError::forbidden("signer_address is not authorized by this multisig policy")
            })? as i32;

        let approved_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM proposal_signatures
             WHERE proposal_id = $1 AND decision = 'approved'",
        )
        .bind(proposal_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to count signatures");
            ApiError::db_error("Failed to evaluate signature ordering")
        })?;

        if signer_position != approved_count as i32 {
            return Err(ApiError::conflict(
                "OutOfOrderApproval",
                "this policy requires ordered approvals",
            ));
        }
        step_index = Some(signer_position);
    }

    let inserted = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO proposal_signatures (
            proposal_id, signer_address, signature_data, decision, comment, step_index
         )
         VALUES ($1, $2, $3, $4::approval_decision_type, $5, $6)
         ON CONFLICT (proposal_id, signer_address) DO NOTHING
         RETURNING id",
    )
    .bind(proposal_id)
    .bind(&signer)
    .bind(payload.signature_data.as_deref())
    .bind(decision.as_str())
    .bind(payload.comment.as_deref())
    .bind(step_index)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to insert proposal signature");
        ApiError::db_error("Failed to record signature")
    })?;

    if inserted.is_none() {
        return Err(ApiError::conflict(
            "AlreadySigned",
            "this signer already submitted a decision for the proposal",
        ));
    }

    metrics::MULTISIG_SIGNATURES.inc();

    sqlx::query(
        "INSERT INTO multisig_approval_audit_events (
            proposal_id, actor_address, action, decision, comment, metadata
         )
         VALUES ($1, $2, $3, $4::approval_decision_type, $5, $6)",
    )
    .bind(proposal_id)
    .bind(&signer)
    .bind(match decision {
        ApprovalDecision::Approved => "signature_approved",
        ApprovalDecision::Rejected => "signature_rejected",
    })
    .bind(decision.as_str())
    .bind(payload.comment.as_deref())
    .bind(json!({ "step_index": step_index }))
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to insert audit event");
        ApiError::db_error("Failed to record audit trail")
    })?;

    let signatures_collected: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM proposal_signatures
         WHERE proposal_id = $1 AND decision = 'approved'",
    )
    .bind(proposal_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to count approved signatures");
        ApiError::db_error("Failed to evaluate proposal threshold")
    })?;

    let mut proposal_status = ProposalStatus::Pending;
    if decision == ApprovalDecision::Rejected {
        let updated_status = sqlx::query_scalar::<_, ProposalStatus>(
            "UPDATE deploy_proposals
             SET status = 'rejected',
                 rejected_at = COALESCE(rejected_at, NOW()),
                 rejection_reason = COALESCE($2, rejection_reason),
                 updated_at = NOW()
             WHERE id = $1
             RETURNING status",
        )
        .bind(proposal_id)
        .bind(payload.comment.as_deref())
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to reject proposal");
            ApiError::db_error("Failed to update proposal status")
        })?;

        proposal_status = updated_status;
        metrics::MULTISIG_REJECTIONS.inc();
    } else if signatures_collected >= i64::from(signing_state.required_approvals) {
        let updated_status = sqlx::query_scalar::<_, ProposalStatus>(
            "UPDATE deploy_proposals
             SET status = 'approved',
                 approved_at = COALESCE(approved_at, NOW()),
                 updated_at = NOW()
             WHERE id = $1 AND status = 'pending'
             RETURNING status",
        )
        .bind(proposal_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to approve proposal");
            ApiError::db_error("Failed to update proposal status")
        })?
        .unwrap_or(ProposalStatus::Approved);

        proposal_status = updated_status;

        sqlx::query(
            "INSERT INTO multisig_approval_audit_events (
                proposal_id, actor_address, action, metadata
             )
             VALUES ($1, $2, 'proposal_approved', $3)",
        )
        .bind(proposal_id)
        .bind(&signer)
        .bind(json!({
            "signatures_collected": signatures_collected,
            "required_approvals": signing_state.required_approvals,
        }))
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to insert proposal_approved audit event");
            ApiError::db_error("Failed to record audit trail")
        })?;
    }

    tx.commit().await.map_err(|e| {
        tracing::error!(error = ?e, "failed to commit signing transaction");
        ApiError::db_error("Failed to finalize signature")
    })?;

    let threshold_met = proposal_status == ProposalStatus::Approved;
    let signatures_needed =
        (i64::from(signing_state.required_approvals) - signatures_collected).max(0);

    Ok(Json(SignProposalResponse {
        signatures_collected,
        signatures_needed,
        threshold_met,
        proposal_status: proposal_status.as_str().to_string(),
    }))
}

pub async fn execute_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ExecuteProposalResponse>> {
    let proposal_id = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request_with("InvalidProposalId", "proposal id must be a valid UUID")
    })?;

    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = ?e, "failed to start execute transaction");
        ApiError::db_error("Failed to execute proposal")
    })?;

    let proposal = sqlx::query_as::<_, (String, String, ProposalStatus)>(
        "SELECT contract_id, wasm_hash, status
         FROM deploy_proposals
         WHERE id = $1
         FOR UPDATE",
    )
    .bind(proposal_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to fetch proposal for execution");
        ApiError::db_error("Failed to load proposal")
    })?
    .ok_or_else(|| ApiError::not_found("ProposalNotFound", "deployment proposal not found"))?;

    let (contract_id, wasm_hash, status) = proposal;

    if status != ProposalStatus::Approved {
        return Err(ApiError::conflict(
            "ProposalNotApproved",
            "proposal must be approved before execution",
        ));
    }

    let executed_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        "UPDATE deploy_proposals
         SET status = 'executed',
             executed_at = COALESCE(executed_at, NOW()),
             updated_at = NOW()
         WHERE id = $1
         RETURNING executed_at",
    )
    .bind(proposal_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to mark proposal executed");
        ApiError::db_error("Failed to execute proposal")
    })?;

    sqlx::query(
        "INSERT INTO multisig_approval_audit_events (
            proposal_id, action, metadata
         )
         VALUES ($1, 'proposal_executed', $2)",
    )
    .bind(proposal_id)
    .bind(json!({
        "contract_id": &contract_id,
        "executed_at": executed_at,
    }))
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to insert execution audit event");
        ApiError::db_error("Failed to record audit trail")
    })?;

    tx.commit().await.map_err(|e| {
        tracing::error!(error = ?e, "failed to commit execute transaction");
        ApiError::db_error("Failed to finalize execution")
    })?;

    metrics::MULTISIG_EXECUTIONS.inc();

    Ok(Json(ExecuteProposalResponse {
        contract_id,
        wasm_hash,
        executed_at,
    }))
}

pub async fn proposal_info(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ProposalInfoResponse>> {
    let proposal_id = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request_with("InvalidProposalId", "proposal id must be a valid UUID")
    })?;

    let proposal = sqlx::query_as::<_, DeployProposal>(
        "SELECT
            id, contract_name, contract_id, wasm_hash, network, description,
            policy_id, status, expires_at, executed_at, approved_at, rejected_at,
            rejection_reason, proposer, required_approvals, created_at, updated_at
         FROM deploy_proposals
         WHERE id = $1",
    )
    .bind(proposal_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to load proposal info");
        ApiError::db_error("Failed to load proposal")
    })?
    .ok_or_else(|| ApiError::not_found("ProposalNotFound", "deployment proposal not found"))?;

    let policy = sqlx::query_as::<_, MultisigPolicy>(
        "SELECT id, name, threshold, signer_addresses, expiry_seconds, ordered_approvals, created_by, created_at
         FROM multisig_policies
         WHERE id = $1",
    )
    .bind(proposal.policy_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to load policy for proposal info");
        ApiError::db_error("Failed to load proposal policy")
    })?;

    let signatures = sqlx::query_as::<_, ProposalSignature>(
        "SELECT
            signer_address,
            signature_data,
            signed_at,
            decision::text AS decision,
            comment,
            step_index,
            reviewed_at
         FROM proposal_signatures
         WHERE proposal_id = $1
         ORDER BY signed_at ASC",
    )
    .bind(proposal_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to load proposal signatures");
        ApiError::db_error("Failed to load proposal signatures")
    })?;

    let signatures_collected: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM proposal_signatures
         WHERE proposal_id = $1 AND decision = 'approved'",
    )
    .bind(proposal_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to count approved signatures");
        ApiError::db_error("Failed to evaluate proposal threshold")
    })?;

    let signatures_needed = (i64::from(proposal.required_approvals) - signatures_collected).max(0);

    Ok(Json(ProposalInfoResponse {
        proposal,
        policy,
        signatures,
        signatures_needed,
    }))
}

pub async fn list_proposals(
    State(state): State<AppState>,
    Query(query): Query<ListProposalsQuery>,
) -> ApiResult<Json<ListProposalsResponse>> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100) as i64;

    let (items, total) = if let Some(status) = query.status.as_deref() {
        match status {
            "pending" | "approved" | "executed" | "expired" | "rejected" => {}
            _ => {
                return Err(ApiError::bad_request(
                    "InvalidStatus",
                    "status must be one of: pending, approved, executed, expired, rejected",
                ))
            }
        }

        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM deploy_proposals WHERE status = $1::proposal_status",
        )
        .bind(status)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to count proposals");
            ApiError::db_error("Failed to load proposals")
        })?;

        let items = sqlx::query_as::<_, DeployProposal>(
            "SELECT
                id, contract_name, contract_id, wasm_hash, network, description,
                policy_id, status, expires_at, executed_at, approved_at, rejected_at,
                rejection_reason, proposer, required_approvals, created_at, updated_at
             FROM deploy_proposals
             WHERE status = $1::proposal_status
             ORDER BY created_at DESC
             LIMIT $2",
        )
        .bind(status)
        .bind(limit)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to list proposals");
            ApiError::db_error("Failed to load proposals")
        })?;

        (items, total)
    } else {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deploy_proposals")
            .fetch_one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "failed to count proposals");
                ApiError::db_error("Failed to load proposals")
            })?;

        let items = sqlx::query_as::<_, DeployProposal>(
            "SELECT
                id, contract_name, contract_id, wasm_hash, network, description,
                policy_id, status, expires_at, executed_at, approved_at, rejected_at,
                rejection_reason, proposer, required_approvals, created_at, updated_at
             FROM deploy_proposals
             ORDER BY created_at DESC
             LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to list proposals");
            ApiError::db_error("Failed to load proposals")
        })?;

        (items, total)
    };

    Ok(Json(ListProposalsResponse { items, total }))
}

pub async fn list_publisher_keys(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<PublisherMultisigKey>>> {
    let publisher_id = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request_with("InvalidPublisherId", "publisher id must be a UUID")
    })?;

    let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM publishers WHERE id = $1")
        .bind(publisher_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "failed to validate publisher");
            ApiError::db_error("Failed to load publisher keys")
        })?;

    if exists.is_none() {
        return Err(ApiError::not_found(
            "PublisherNotFound",
            "publisher not found",
        ));
    }

    let keys = sqlx::query_as::<_, PublisherMultisigKey>(
        "SELECT
            id, publisher_id, key_name, public_key, algorithm,
            is_active, created_at, updated_at
         FROM publisher_multisig_keys
         WHERE publisher_id = $1
         ORDER BY is_active DESC, created_at DESC",
    )
    .bind(publisher_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to list publisher keys");
        ApiError::db_error("Failed to load publisher keys")
    })?;

    Ok(Json(keys))
}

pub async fn create_publisher_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(payload): ValidatedJson<CreatePublisherKeyRequest>,
) -> ApiResult<Json<PublisherMultisigKey>> {
    let publisher_id = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request_with("InvalidPublisherId", "publisher id must be a UUID")
    })?;

    if payload.key_name.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidKeyName",
            "key_name cannot be empty",
        ));
    }

    if payload.public_key.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidPublicKey",
            "public_key cannot be empty",
        ));
    }

    let algorithm = payload
        .algorithm
        .as_deref()
        .unwrap_or("ed25519")
        .to_ascii_lowercase();
    if algorithm != "ed25519" {
        return Err(ApiError::bad_request(
            "InvalidAlgorithm",
            "only ed25519 keys are currently supported",
        ));
    }

    let row = sqlx::query_as::<_, PublisherMultisigKey>(
        "INSERT INTO publisher_multisig_keys (
            publisher_id, key_name, public_key, algorithm, is_active
         )
         VALUES ($1, $2, $3, $4, TRUE)
         RETURNING
            id, publisher_id, key_name, public_key, algorithm,
            is_active, created_at, updated_at",
    )
    .bind(publisher_id)
    .bind(payload.key_name.trim())
    .bind(payload.public_key.trim())
    .bind(&algorithm)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to create publisher key");
        ApiError::db_error("Failed to create publisher key")
    })?;

    Ok(Json(row))
}

pub async fn deactivate_publisher_key(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<PublisherMultisigKey>> {
    let key_id = Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request_with("InvalidKeyId", "key id must be a UUID"))?;

    let row = sqlx::query_as::<_, PublisherMultisigKey>(
        "UPDATE publisher_multisig_keys
         SET is_active = FALSE,
             updated_at = NOW()
         WHERE id = $1
         RETURNING
            id, publisher_id, key_name, public_key, algorithm,
            is_active, created_at, updated_at",
    )
    .bind(key_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "failed to deactivate publisher key");
        ApiError::db_error("Failed to deactivate publisher key")
    })?
    .ok_or_else(|| ApiError::not_found("KeyNotFound", "publisher key not found"))?;

    Ok(Json(row))
}
