use axum::{
    extract::{Path, Query, State},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use shared::{
    ChainOfCustodyEntry, ChainOfCustodyResponse, PackageSignature, RevokeSignatureRequest,
    SignatureStatus, TransparencyEntryType, TransparencyLogEntry, TransparencyLogQueryParams,
    VerifySignatureResponse,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::db_internal_error,
    state::AppState,
};

fn map_json_rejection(err: axum::extract::rejection::JsonRejection) -> ApiError {
    ApiError::bad_request_with(
        "InvalidRequest",
        format!("Invalid JSON payload: {}", err.body_text()),
    )
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct SignRequest {
    pub contract_id: String,
    pub version: String,
    pub wasm_hash: String,
    pub signature: String,
    pub signing_address: String,
    pub public_key: String,
    pub algorithm: Option<String>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

pub async fn sign_package(
    State(state): State<AppState>,
    payload: Result<Json<SignRequest>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<Json<PackageSignature>> {
    let Json(req) = payload.map_err(map_json_rejection)?;

    if req.contract_id.is_empty() {
        return Err(ApiError::bad_request(
            "MissingContractId",
            "contract_id is required",
        ));
    }
    if req.signature.is_empty() {
        return Err(ApiError::bad_request(
            "MissingSignature",
            "signature is required",
        ));
    }

    let contract_uuid = parse_contract_uuid(&state, &req.contract_id).await?;

    let algorithm = req
        .algorithm
        .clone()
        .unwrap_or_else(|| "ed25519".to_string());

    let signature: PackageSignature = sqlx::query_as(
        r#"
        INSERT INTO package_signatures 
            (contract_id, version, wasm_hash, signature, signing_address, public_key, algorithm, expires_at, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(contract_uuid)
    .bind(&req.version)
    .bind(&req.wasm_hash)
    .bind(&req.signature)
    .bind(&req.signing_address)
    .bind(&req.public_key)
    .bind(&algorithm)
    .bind(req.expires_at)
    .bind(req.metadata.clone())
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("create package signature", err))?;

    append_transparency_log_entry(
        &state,
        TransparencyEntryType::PackageSigned,
        Some(contract_uuid),
        Some(signature.id),
        &req.signing_address,
        serde_json::to_value(&req).ok(),
    )
    .await?;

    tracing::info!(
        signature_id = %signature.id,
        contract_id = %contract_uuid,
        signing_address = %req.signing_address,
        "package signed"
    );

    Ok(Json(signature))
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequestInternal {
    pub contract_id: String,
    pub version: Option<String>,
    pub wasm_hash: String,
    pub signature: Option<String>,
    pub signing_address: Option<String>,
}

pub async fn verify_signature(
    State(state): State<AppState>,
    payload: Result<Json<VerifyRequestInternal>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<Json<VerifySignatureResponse>> {
    let Json(req) = payload.map_err(map_json_rejection)?;

    if let (Some(sig_b64), Some(signing_addr)) = (&req.signature, &req.signing_address) {
        verify_signature_locally(&state, &req, sig_b64, signing_addr).await
    } else {
        verify_signature_from_registry(&state, &req).await
    }
}

async fn verify_signature_locally(
    state: &AppState,
    req: &VerifyRequestInternal,
    sig_b64: &str,
    _signing_addr: &str,
) -> ApiResult<Json<VerifySignatureResponse>> {
    let contract_uuid = parse_contract_uuid(state, &req.contract_id).await?;

    let sig_bytes = BASE64.decode(sig_b64).map_err(|_| {
        ApiError::bad_request_with("InvalidSignature", "signature is not valid base64")
    })?;

    let sig_array: [u8; 64] = sig_bytes.as_slice().try_into().map_err(|_| {
        ApiError::bad_request_with("InvalidSignature", "signature must be 64 bytes")
    })?;

    let signature = Signature::from_bytes(&sig_array);

    let db_sig: Option<PackageSignature> = sqlx::query_as(
        r#"
        SELECT * FROM package_signatures 
        WHERE contract_id = $1 
          AND ($2::text IS NULL OR version = $2)
          AND wasm_hash = $3
          AND signature = $4
        ORDER BY signed_at DESC 
        LIMIT 1
        "#,
    )
    .bind(contract_uuid)
    .bind(&req.version)
    .bind(&req.wasm_hash)
    .bind(sig_b64)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("lookup signature", err))?;

    match db_sig {
        Some(db_sig) => {
            let public_key_bytes = BASE64
                .decode(&db_sig.public_key)
                .map_err(|_| ApiError::internal("Invalid public key in database"))?;

            let pk_array: [u8; 32] = public_key_bytes
                .as_slice()
                .try_into()
                .map_err(|_| ApiError::internal("Public key must be 32 bytes"))?;

            let verifying_key = VerifyingKey::from_bytes(&pk_array)
                .map_err(|_| ApiError::internal("Invalid verifying key"))?;

            let message =
                create_signing_message(&req.wasm_hash, &req.contract_id, db_sig.version.as_str());

            let crypto_valid = verifying_key.verify(&message, &signature).is_ok();
            let status_valid = db_sig.status == SignatureStatus::Valid;

            let valid = crypto_valid && status_valid;

            if valid {
                let _ = append_transparency_log_entry(
                    state,
                    TransparencyEntryType::SignatureVerified,
                    Some(contract_uuid),
                    Some(db_sig.id),
                    &db_sig.signing_address,
                    None,
                )
                .await;
            }

            let status_clone = db_sig.status.clone();
            Ok(Json(VerifySignatureResponse {
                valid,
                signature_id: Some(db_sig.id),
                signing_address: db_sig.signing_address,
                signed_at: Some(db_sig.signed_at),
                status: db_sig.status,
                message: if valid {
                    "Signature is valid".to_string()
                } else if !status_valid {
                    format!("Signature status is: {:?}", status_clone)
                } else {
                    "Cryptographic verification failed".to_string()
                },
            }))
        }
        None => Ok(Json(VerifySignatureResponse {
            valid: false,
            signature_id: None,
            signing_address: String::new(),
            signed_at: None,
            status: SignatureStatus::Valid,
            message: "Signature not found in registry".to_string(),
        })),
    }
}

async fn verify_signature_from_registry(
    state: &AppState,
    req: &VerifyRequestInternal,
) -> ApiResult<Json<VerifySignatureResponse>> {
    let contract_uuid = parse_contract_uuid(state, &req.contract_id).await?;

    let sig: Option<PackageSignature> = sqlx::query_as(
        r#"
        SELECT * FROM package_signatures 
        WHERE contract_id = $1 
          AND ($2::text IS NULL OR version = $2)
          AND wasm_hash = $3
          AND status = 'valid'
        ORDER BY signed_at DESC 
        LIMIT 1
        "#,
    )
    .bind(contract_uuid)
    .bind(&req.version)
    .bind(&req.wasm_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("lookup signature", err))?;

    match sig {
        Some(sig) => Ok(Json(VerifySignatureResponse {
            valid: true,
            signature_id: Some(sig.id),
            signing_address: sig.signing_address,
            signed_at: Some(sig.signed_at),
            status: sig.status,
            message: "Signature is valid".to_string(),
        })),
        None => Ok(Json(VerifySignatureResponse {
            valid: false,
            signature_id: None,
            signing_address: String::new(),
            signed_at: None,
            status: SignatureStatus::Valid,
            message: "No valid signature found for this package".to_string(),
        })),
    }
}

pub async fn revoke_signature(
    State(state): State<AppState>,
    Path(signature_id): Path<String>,
    payload: Result<Json<RevokeSignatureRequest>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<Json<serde_json::Value>> {
    let Json(req) = payload.map_err(map_json_rejection)?;

    let sig_uuid = Uuid::parse_str(&signature_id).map_err(|_| {
        ApiError::bad_request_with("InvalidSignatureId", "signature_id must be a UUID")
    })?;

    let existing: Option<PackageSignature> =
        sqlx::query_as("SELECT * FROM package_signatures WHERE id = $1")
            .bind(sig_uuid)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("lookup signature", err))?;

    let existing = existing.ok_or_else(|| {
        ApiError::not_found(
            "SignatureNotFound",
            format!("No signature with ID: {}", signature_id),
        )
    })?;

    if existing.status != SignatureStatus::Valid {
        return Err(ApiError::bad_request(
            "AlreadyRevoked",
            format!("Signature is already in status: {}", existing.status),
        ));
    }

    sqlx::query(
        r#"
        UPDATE package_signatures 
        SET status = 'revoked', revoked_at = NOW(), revoked_by = $1, revoked_reason = $2, updated_at = NOW()
        WHERE id = $3
        "#,
    )
    .bind(&req.revoked_by)
    .bind(&req.reason)
    .bind(sig_uuid)
    .execute(&state.db)
    .await
    .map_err(|err| db_internal_error("revoke signature", err))?;

    sqlx::query(
        r#"
        INSERT INTO signature_revocations (signature_id, revoked_by, reason)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(sig_uuid)
    .bind(&req.revoked_by)
    .bind(&req.reason)
    .execute(&state.db)
    .await
    .map_err(|err| db_internal_error("create revocation record", err))?;

    append_transparency_log_entry(
        &state,
        TransparencyEntryType::SignatureRevoked,
        Some(existing.contract_id),
        Some(sig_uuid),
        &req.revoked_by,
        serde_json::to_value(&req).ok(),
    )
    .await?;

    tracing::info!(
        signature_id = %sig_uuid,
        revoked_by = %req.revoked_by,
        reason = %req.reason,
        "signature revoked"
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "signature_id": signature_id,
        "revoked_at": Utc::now().to_rfc3339()
    })))
}

#[derive(Debug, Deserialize)]
pub struct LookupQuery {
    pub contract_id: String,
    pub version: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct LookupResponse {
    pub signatures: Vec<PackageSignature>,
    pub total: i64,
}

pub async fn lookup_signatures(
    State(state): State<AppState>,
    Query(query): Query<LookupQuery>,
) -> ApiResult<Json<LookupResponse>> {
    let contract_uuid = parse_contract_uuid(&state, &query.contract_id).await?;

    let signatures: Vec<PackageSignature> = sqlx::query_as(
        r#"
        SELECT * FROM package_signatures 
        WHERE contract_id = $1 
          AND ($2::text IS NULL OR version = $2)
        ORDER BY signed_at DESC
        "#,
    )
    .bind(contract_uuid)
    .bind(&query.version)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("lookup signatures", err))?;

    let total = signatures.len() as i64;

    Ok(Json(LookupResponse { signatures, total }))
}

pub async fn get_chain_of_custody(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> ApiResult<Json<ChainOfCustodyResponse>> {
    let contract_uuid = parse_contract_uuid(&state, &contract_id).await?;

    let logs: Vec<TransparencyLogEntry> = sqlx::query_as(
        r#"
        SELECT * FROM transparency_log 
        WHERE contract_id = $1 
        ORDER BY timestamp ASC
        "#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch transparency log", err))?;

    let entries: Vec<ChainOfCustodyEntry> = logs
        .into_iter()
        .map(|log| ChainOfCustodyEntry {
            action: log.entry_type.to_string(),
            actor: log.actor_address,
            timestamp: log.timestamp,
            signature_id: log.signature_id,
            details: log.payload,
        })
        .collect();

    Ok(Json(ChainOfCustodyResponse {
        contract_id: contract_id.clone(),
        entries,
    }))
}

#[derive(Debug, serde::Serialize)]
pub struct TransparencyLogResponse {
    pub items: Vec<TransparencyLogEntry>,
    pub total: i64,
}

pub async fn get_transparency_log(
    State(state): State<AppState>,
    Query(query): Query<TransparencyLogQueryParams>,
) -> ApiResult<Json<TransparencyLogResponse>> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let mut conditions = Vec::new();
    let mut param_count = 1;

    let contract_uuid = if let Some(cid) = &query.contract_id {
        let uuid = parse_contract_uuid(&state, cid).await.ok();
        if let Some(u) = uuid {
            conditions.push(format!("contract_id = ${}", param_count));
            param_count += 1;
            Some(u)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(_et) = &query.entry_type {
        conditions.push(format!("entry_type = ${}", param_count));
        param_count += 1;
    }
    if let Some(_addr) = &query.actor_address {
        conditions.push(format!("actor_address = ${}", param_count));
        param_count += 1;
    }
    if let Some(_from) = &query.from_timestamp {
        conditions.push(format!("timestamp >= ${}", param_count));
        param_count += 1;
    }
    if let Some(_to) = &query.to_timestamp {
        conditions.push(format!("timestamp <= ${}", param_count));
        param_count += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_query = format!("SELECT COUNT(*) FROM transparency_log {}", where_clause);
    let select_query = format!(
        "SELECT * FROM transparency_log {} ORDER BY timestamp DESC LIMIT ${} OFFSET ${}",
        where_clause,
        param_count,
        param_count + 1
    );

    let mut count_sql = sqlx::query_scalar::<_, i64>(&count_query);
    let mut select_sql = sqlx::query_as::<_, TransparencyLogEntry>(&select_query);

    if let Some(u) = contract_uuid {
        count_sql = count_sql.bind(u);
        select_sql = select_sql.bind(u);
    }
    if let Some(et) = &query.entry_type {
        count_sql = count_sql.bind(et.to_string());
        select_sql = select_sql.bind(et.to_string());
    }
    if let Some(addr) = &query.actor_address {
        count_sql = count_sql.bind(addr);
        select_sql = select_sql.bind(addr);
    }
    if let Some(from) = &query.from_timestamp {
        count_sql = count_sql.bind(from);
        select_sql = select_sql.bind(from);
    }
    if let Some(to) = &query.to_timestamp {
        count_sql = count_sql.bind(to);
        select_sql = select_sql.bind(to);
    }

    let total = count_sql
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count transparency log", err))?;

    select_sql = select_sql.bind(limit).bind(offset);

    let items = select_sql
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch transparency log", err))?;

    Ok(Json(TransparencyLogResponse { items, total }))
}

#[derive(Debug, serde::Serialize)]
pub struct TransparencyLogVerificationIssue {
    pub entry_id: Uuid,
    pub reason: String,
    pub expected_previous_hash: Option<String>,
    pub actual_previous_hash: Option<String>,
    pub expected_entry_hash: Option<String>,
    pub actual_entry_hash: String,
}

#[derive(Debug, serde::Serialize)]
pub struct TransparencyLogVerificationResponse {
    pub valid: bool,
    pub checked_entries: usize,
    pub issues: Vec<TransparencyLogVerificationIssue>,
}

pub async fn verify_transparency_log(
    State(state): State<AppState>,
) -> ApiResult<Json<TransparencyLogVerificationResponse>> {
    let entries: Vec<TransparencyLogEntry> = sqlx::query_as(
        r#"
        SELECT * FROM transparency_log
        ORDER BY timestamp ASC, id ASC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch transparency log for verification", err))?;

    let issues = validate_transparency_chain(&entries);

    Ok(Json(TransparencyLogVerificationResponse {
        valid: issues.is_empty(),
        checked_entries: entries.len(),
        issues,
    }))
}

async fn append_transparency_log_entry(
    state: &AppState,
    entry_type: TransparencyEntryType,
    contract_id: Option<Uuid>,
    signature_id: Option<Uuid>,
    actor_address: &str,
    payload: Option<serde_json::Value>,
) -> ApiResult<()> {
    let previous_hash: Option<String> = sqlx::query_scalar(
        "SELECT entry_hash FROM transparency_log ORDER BY timestamp DESC, id DESC LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch previous transparency hash", err))?;

    let entry_timestamp = Utc::now();
    let entry_hash = compute_transparency_hash(
        &entry_type,
        contract_id,
        signature_id,
        actor_address,
        previous_hash.as_deref(),
        entry_timestamp.timestamp(),
    );

    sqlx::query(
        r#"
        INSERT INTO transparency_log
            (entry_type, contract_id, signature_id, actor_address, previous_hash, entry_hash, payload, timestamp)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(entry_type.to_string())
    .bind(contract_id)
    .bind(signature_id)
    .bind(actor_address)
    .bind(previous_hash)
    .bind(entry_hash)
    .bind(payload)
    .bind(entry_timestamp)
    .execute(&state.db)
    .await
    .map_err(|err| db_internal_error("create transparency log entry", err))?;

    Ok(())
}

fn validate_transparency_chain(
    entries: &[TransparencyLogEntry],
) -> Vec<TransparencyLogVerificationIssue> {
    let mut issues = Vec::new();
    let mut expected_previous_hash: Option<String> = None;

    for entry in entries {
        if entry.previous_hash != expected_previous_hash {
            issues.push(TransparencyLogVerificationIssue {
                entry_id: entry.id,
                reason: "previous_hash mismatch".to_string(),
                expected_previous_hash: expected_previous_hash.clone(),
                actual_previous_hash: entry.previous_hash.clone(),
                expected_entry_hash: None,
                actual_entry_hash: entry.entry_hash.clone(),
            });
        }

        let recomputed_hash = compute_transparency_hash(
            &entry.entry_type,
            entry.contract_id,
            entry.signature_id,
            &entry.actor_address,
            entry.previous_hash.as_deref(),
            entry.timestamp.timestamp(),
        );

        if entry.entry_hash != recomputed_hash {
            issues.push(TransparencyLogVerificationIssue {
                entry_id: entry.id,
                reason: "entry_hash mismatch".to_string(),
                expected_previous_hash: expected_previous_hash.clone(),
                actual_previous_hash: entry.previous_hash.clone(),
                expected_entry_hash: Some(recomputed_hash),
                actual_entry_hash: entry.entry_hash.clone(),
            });
        }

        expected_previous_hash = Some(entry.entry_hash.clone());
    }

    issues
}

async fn parse_contract_uuid(state: &AppState, contract_id: &str) -> ApiResult<Uuid> {
    if let Ok(uuid) = Uuid::parse_str(contract_id) {
        return Ok(uuid);
    }

    let uuid: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM contracts WHERE contract_id = $1 LIMIT 1")
            .bind(contract_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("lookup contract", err))?;

    uuid.ok_or_else(|| {
        ApiError::not_found(
            "ContractNotFound",
            format!("No contract found: {}", contract_id),
        )
    })
}

pub(crate) fn create_signing_message(hash: &str, contract_id: &str, version: &str) -> Vec<u8> {
    format!("{}:{}:{}", contract_id, version, hash).into_bytes()
}

pub(crate) fn compute_transparency_hash(
    entry_type: &TransparencyEntryType,
    contract_id: Option<Uuid>,
    signature_id: Option<Uuid>,
    actor_address: &str,
    previous_hash: Option<&str>,
    timestamp: i64,
) -> String {
    let mut hasher = Sha256::new();
    if let Some(prev) = previous_hash {
        hasher.update(prev.as_bytes());
    }
    hasher.update(entry_type.to_string().as_bytes());
    if let Some(cid) = contract_id {
        hasher.update(cid.as_bytes());
    }
    if let Some(sid) = signature_id {
        hasher.update(sid.as_bytes());
    }
    hasher.update(actor_address.as_bytes());
    hasher.update(timestamp.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn make_log_entry(
        entry_type: TransparencyEntryType,
        previous_hash: Option<String>,
        timestamp: chrono::DateTime<Utc>,
    ) -> TransparencyLogEntry {
        let contract_id = Some(Uuid::new_v4());
        let signature_id = Some(Uuid::new_v4());
        let actor_address = "GTESTACTORADDRESS0000000000000000000000000000000000".to_string();
        let entry_hash = compute_transparency_hash(
            &entry_type,
            contract_id,
            signature_id,
            &actor_address,
            previous_hash.as_deref(),
            timestamp.timestamp(),
        );

        TransparencyLogEntry {
            id: Uuid::new_v4(),
            entry_type,
            contract_id,
            signature_id,
            actor_address,
            previous_hash,
            entry_hash,
            payload: None,
            timestamp,
            immutable: true,
        }
    }

    #[test]
    fn transparency_chain_detects_tampered_hash() {
        let t1 = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let t2 = Utc.timestamp_opt(1_700_000_100, 0).unwrap();

        let first = make_log_entry(TransparencyEntryType::PackageSigned, None, t1);
        let mut second = make_log_entry(
            TransparencyEntryType::SignatureRevoked,
            Some(first.entry_hash.clone()),
            t2,
        );

        second.entry_hash = "deadbeef".repeat(8);

        let issues = validate_transparency_chain(&[first, second]);
        assert!(issues.iter().any(|i| i.reason == "entry_hash mismatch"));
    }

    #[test]
    fn transparency_chain_detects_broken_previous_link() {
        let t1 = Utc.timestamp_opt(1_700_010_000, 0).unwrap();
        let t2 = Utc.timestamp_opt(1_700_010_100, 0).unwrap();

        let first = make_log_entry(TransparencyEntryType::PackageSigned, None, t1);
        let second = make_log_entry(
            TransparencyEntryType::SignatureVerified,
            Some("not_the_real_previous_hash".to_string()),
            t2,
        );

        let issues = validate_transparency_chain(&[first, second]);
        assert!(issues.iter().any(|i| i.reason == "previous_hash mismatch"));
    }
}
