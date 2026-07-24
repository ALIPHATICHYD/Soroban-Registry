use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, State},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use stellar_strkey::ed25519::PublicKey as StrKeyPublicKey;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct Validator {
    pub id: Uuid,
    pub stellar_address: String,
    pub name: Option<String>,
    pub status: String,
    pub stake_amount: Decimal,
    pub reputation_score: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ValidatorPerformance {
    pub validator_id: Uuid,
    pub total_verifications: i32,
    pub successful_verifications: i32,
    pub failed_verifications: i32,
    pub slashed_count: i32,
    pub last_active_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct VerificationTask {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub status: String,
    pub assigned_to: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct Attestation {
    pub id: Uuid,
    pub task_id: Uuid,
    pub validator_id: Uuid,
    pub decision: String,
    pub compiled_wasm_hash: Option<String>,
    pub error_message: Option<String>,
    pub signature: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ValidatorNetworkStatus {
    pub total_validators: i64,
    pub active_validators: i64,
    pub pending_tasks: i64,
    pub completed_verifications: i64,
    pub total_staked_amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RegisterValidatorRequest {
    pub stellar_address: String,
    pub name: String,
    pub stake_amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SubmitAttestationRequest {
    pub task_id: Uuid,
    pub decision: String,
    pub compiled_wasm_hash: Option<String>,
    pub error_message: Option<String>,
    pub signature: Option<String>,
}

/// Register a new validator (Issue # validators)
pub async fn register_validator(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<RegisterValidatorRequest>,
) -> ApiResult<Json<Validator>> {
    let validator: Validator = sqlx::query_as(
        r#"
        INSERT INTO validators (stellar_address, name, stake_amount, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING *
        "#,
    )
    .bind(&req.stellar_address)
    .bind(&req.name)
    .bind(req.stake_amount)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if let Some(db_err) = err.as_database_error() {
            if db_err.is_unique_violation() {
                return ApiError::conflict(
                    "ValidatorAlreadyExists",
                    "Validator with this address already registered",
                );
            }
        }
        ApiError::internal(format!("Failed to register validator: {}", err))
    })?;

    // Initialize performance record
    sqlx::query(
        "INSERT INTO validator_performance (validator_id) VALUES ($1) ON CONFLICT DO NOTHING",
    )
    .bind(validator.id)
    .execute(&state.db)
    .await
    .map_err(|err| {
        ApiError::internal(format!(
            "Failed to initialize validator performance: {}",
            err
        ))
    })?;

    Ok(Json(validator))
}

/// List all validators
pub async fn list_validators(State(state): State<AppState>) -> ApiResult<Json<Vec<Validator>>> {
    let validators: Vec<Validator> =
        sqlx::query_as("SELECT * FROM validators ORDER BY reputation_score DESC")
            .fetch_all(&state.db)
            .await
            .map_err(|err| ApiError::internal(format!("Failed to fetch validators: {}", err)))?;

    Ok(Json(validators))
}

/// Get network status
pub async fn get_network_status(
    State(state): State<AppState>,
) -> ApiResult<Json<ValidatorNetworkStatus>> {
    let stats: (i64, i64, i64, i64, Option<Decimal>) = sqlx::query_as(
        r#"
        SELECT
            (SELECT COUNT(*) FROM validators) as total_validators,
            (SELECT COUNT(*) FROM validators WHERE status = 'active') as active_validators,
            (SELECT COUNT(*) FROM verification_tasks WHERE status = 'pending') as pending_tasks,
            (SELECT COUNT(*) FROM verification_tasks WHERE status = 'completed') as completed_verifications,
            (SELECT SUM(stake_amount) FROM validators) as total_staked_amount
        "#
    )
    .fetch_one(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("Failed to fetch network status: {}", err)))?;

    Ok(Json(ValidatorNetworkStatus {
        total_validators: stats.0,
        active_validators: stats.1,
        pending_tasks: stats.2,
        completed_verifications: stats.3,
        total_staked_amount: stats.4.unwrap_or(Decimal::ZERO),
    }))
}

/// Fetch available verification tasks for a validator
pub async fn get_available_tasks(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<VerificationTask>>> {
    let tasks: Vec<VerificationTask> = sqlx::query_as(
        "SELECT * FROM verification_tasks WHERE status = 'pending' ORDER BY created_at ASC LIMIT 10"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("Failed to fetch pending tasks: {}", err)))?;

    Ok(Json(tasks))
}

/// Submit an attestation for a verification task
pub async fn submit_attestation(
    State(state): State<AppState>,
    Path(validator_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<SubmitAttestationRequest>,
) -> ApiResult<Json<Attestation>> {
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    // 1. Verify validator exists and get their stellar address
    let validator_address: String = sqlx::query_scalar(
        "SELECT stellar_address FROM validators WHERE id = $1 AND status = 'active'",
    )
    .bind(validator_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "ValidatorNotActive",
            "Validator is not active or not found",
        ),
        _ => ApiError::internal(err.to_string()),
    })?;

    // 2. Verify signature
    if let Some(sig_str) = &req.signature {
        let decoded_sig = BASE64.decode(sig_str).map_err(|_| {
            ApiError::bad_request_with("InvalidSignature", "Failed to decode base64 signature")
        })?;
        let signature = Signature::from_slice(&decoded_sig).map_err(|_| {
            ApiError::bad_request_with("InvalidSignature", "Invalid signature format")
        })?;

        let public_key_result = StrKeyPublicKey::from_string(&validator_address);
        let public_key_bytes = match public_key_result {
            Ok(pk) => pk.0,
            Err(_) => {
                return Err(ApiError::internal(
                    "Invalid validator address in database".to_string(),
                ))
            }
        };

        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
            .map_err(|_| ApiError::internal("Failed to derive verifying key".to_string()))?;

        // Message: "task_id:decision:compiled_wasm_hash"
        let message = format!(
            "{}:{}:{}",
            req.task_id,
            req.decision.to_string(),
            req.compiled_wasm_hash.as_deref().unwrap_or("")
        );

        verifying_key
            .verify(message.as_bytes(), &signature)
            .map_err(|_| ApiError::unauthorized("Invalid signature for the provided task data"))?;
    } else {
        return Err(ApiError::bad_request(
            "SignatureRequired",
            "Attestation must be signed by the validator",
        ));
    }

    // 3. Insert attestation
    let attestation: Attestation = sqlx::query_as(
        r#"
        INSERT INTO attestations (task_id, validator_id, decision, compiled_wasm_hash, error_message, signature)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(req.task_id)
    .bind(validator_id)
    .bind(&req.decision)
    .bind(&req.compiled_wasm_hash)
    .bind(&req.error_message)
    .bind(&req.signature)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| ApiError::internal(format!("Failed to submit attestation: {}", err)))?;

    // Update validator performance
    sqlx::query(
        r#"
        UPDATE validator_performance
        SET total_verifications = total_verifications + 1,
            successful_verifications = successful_verifications + CASE WHEN $2 = 'valid' THEN 1 ELSE 0 END,
            failed_verifications = failed_verifications + CASE WHEN $2 = 'invalid' THEN 1 ELSE 0 END,
            last_active_at = NOW(),
            updated_at = NOW()
        WHERE validator_id = $1
        "#
    )
    .bind(validator_id)
    .bind(&req.decision)
    .execute(&mut *tx)
    .await
    .map_err(|err| ApiError::internal(format!("Failed to update validator performance: {}", err)))?;

    // Check consensus for this task
    let attestation_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM attestations WHERE task_id = $1")
            .bind(req.task_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|err| ApiError::internal(err.to_string()))?;

    // Simple consensus: if we have 3 attestations, mark task as completed
    if attestation_count >= 3 {
        sqlx::query(
            "UPDATE verification_tasks SET status = 'completed', updated_at = NOW() WHERE id = $1",
        )
        .bind(req.task_id)
        .execute(&mut *tx)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

        // Also update the contract verification status if majority agrees level
        let valid_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM attestations WHERE task_id = $1 AND decision = 'valid'",
        )
        .bind(req.task_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

        if valid_count >= 2 {
            // Majority says valid
            let contract_id: Uuid =
                sqlx::query_scalar("SELECT contract_id FROM verification_tasks WHERE id = $1")
                    .bind(req.task_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|err| ApiError::internal(err.to_string()))?;

            sqlx::query(
                "UPDATE contracts SET is_verified = true, verified_at = NOW(), updated_at = NOW() WHERE id = $1"
            )
            .bind(contract_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| ApiError::internal(err.to_string()))?;
        }
    }

    tx.commit()
        .await
        .map_err(|err| ApiError::internal(err.to_string()))?;

    Ok(Json(attestation))
}

/// Get performance stats for a specific validator
pub async fn get_validator_performance(
    State(state): State<AppState>,
    Path(validator_id): Path<Uuid>,
) -> ApiResult<Json<ValidatorPerformance>> {
    let perf: ValidatorPerformance =
        sqlx::query_as("SELECT * FROM validator_performance WHERE validator_id = $1")
            .bind(validator_id)
            .fetch_one(&state.db)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => ApiError::not_found(
                    "PerformanceNotFound",
                    "No performance record for this validator",
                ),
                _ => ApiError::internal(err.to_string()),
            })?;

    Ok(Json(perf))
}
