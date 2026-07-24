use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{NaiveDate, Utc};
use shared::models::{
    BackupRestoration, ContractBackup, CreateBackupRequest, RestoreBackupRequest,
};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    crypto::EncryptionService,
    disaster_recovery_models::{
        CreateDisasterRecoveryPlanRequest, DisasterRecoveryPlan, ExecuteRecoveryRequest,
        RecoveryMetrics,
    },
    error::{ApiError, ApiResult},
    state::AppState,
};

/// Encrypt a backup JSON field into a stored JSON string envelope (#895).
/// Encrypted backups keep sensitive contract metadata and state snapshots
/// unreadable at rest; in pass-through mode (no keys configured) the value is
/// stored unchanged.
fn encrypt_backup_field(
    enc: &EncryptionService,
    value: &serde_json::Value,
) -> ApiResult<serde_json::Value> {
    let envelope = enc
        .encrypt_json(value)
        .map_err(|e| ApiError::internal(format!("Failed to encrypt backup field: {e}")))?;
    Ok(serde_json::Value::String(envelope))
}

/// Decrypt an encrypted backup field in place, leaving legacy plaintext rows
/// untouched. Transparent to API consumers.
fn decrypt_backup_field(
    enc: &EncryptionService,
    field: &mut Option<serde_json::Value>,
) -> ApiResult<()> {
    if let Some(serde_json::Value::String(stored)) = field.as_ref() {
        let decrypted = enc
            .decrypt_json(stored)
            .map_err(|e| ApiError::internal(format!("Failed to decrypt backup field: {e}")))?;
        *field = Some(decrypted);
    }
    Ok(())
}

fn decrypt_backup(enc: &EncryptionService, backup: &mut ContractBackup) -> ApiResult<()> {
    decrypt_backup_field(enc, &mut backup.metadata)?;
    decrypt_backup_field(enc, &mut backup.state_snapshot)?;
    Ok(())
}

pub async fn create_backup(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateBackupRequest>,
) -> ApiResult<Json<ContractBackup>> {
    let contract: shared::Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("contract", "Contract not found"))?;

    let backup_date = Utc::now().date_naive();

    let metadata = serde_json::json!({
        "name": contract.name,
        "description": contract.description,
        "network": contract.network,
        "category": contract.category,
        "tags": contract.tags,
    });

    let state_snapshot = if req.include_state {
        Some(serde_json::json!({"placeholder": "state data"}))
    } else {
        None
    };

    // Encrypt sensitive backup contents at rest (#895).
    let metadata = encrypt_backup_field(&state.encryption, &metadata)?;
    let state_snapshot = match state_snapshot {
        Some(snapshot) => Some(encrypt_backup_field(&state.encryption, &snapshot)?),
        None => None,
    };

    let mut backup = sqlx::query_as::<_, ContractBackup>(
        r#"
        INSERT INTO contract_backups 
        (contract_id, backup_date, wasm_hash, metadata, state_snapshot, storage_size_bytes, primary_region, backup_regions)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (contract_id, backup_date) DO UPDATE 
        SET wasm_hash = $3, metadata = $4, state_snapshot = $5
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(backup_date)
    .bind(&contract.wasm_hash)
    .bind(&metadata)
    .bind(&state_snapshot)
    .bind(1024i64) // Placeholder size
    .bind("us-east-1")
    .bind(vec!["us-west-2", "eu-west-1"])
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create backup: {}", e)))?;

    // Return decrypted contents so encryption stays transparent to callers.
    decrypt_backup(&state.encryption, &mut backup)?;

    Ok(Json(backup))
}

pub async fn list_backups(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<Vec<ContractBackup>>> {
    let mut backups = sqlx::query_as::<_, ContractBackup>(
        "SELECT * FROM contract_backups WHERE contract_id = $1 ORDER BY backup_date DESC LIMIT 30",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    for backup in backups.iter_mut() {
        decrypt_backup(&state.encryption, backup)?;
    }

    Ok(Json(backups))
}

pub async fn restore_backup(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<RestoreBackupRequest>,
) -> ApiResult<Json<BackupRestoration>> {
    let start = std::time::Instant::now();

    let backup_date = NaiveDate::parse_from_str(&req.backup_date, "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request_with("invalid_date", "Invalid date format"))?;

    let backup = sqlx::query_as::<_, ContractBackup>(
        "SELECT * FROM contract_backups WHERE contract_id = $1 AND backup_date = $2",
    )
    .bind(contract_id)
    .bind(backup_date)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("backup", "Backup not found"))?;

    // Simulate restoration
    let duration_ms = start.elapsed().as_millis() as i32;

    let contract = sqlx::query("SELECT publisher_id FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;
    let publisher_id: Uuid = contract.get("publisher_id");

    let restoration = sqlx::query_as::<_, BackupRestoration>(
        r#"
        INSERT INTO backup_restorations (backup_id, restored_by, restore_duration_ms, success)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(backup.id)
    .bind(publisher_id)
    .bind(duration_ms)
    .bind(true)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to log restoration: {}", e)))?;

    Ok(Json(restoration))
}

pub async fn verify_backup(
    State(state): State<AppState>,
    Path((contract_id, backup_date)): Path<(Uuid, String)>,
) -> ApiResult<StatusCode> {
    let date = NaiveDate::parse_from_str(&backup_date, "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request_with("invalid_date", "Invalid date format"))?;

    sqlx::query(
        "UPDATE contract_backups SET verified = true WHERE contract_id = $1 AND backup_date = $2",
    )
    .bind(contract_id)
    .bind(date)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to verify backup: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_backup_stats(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    let stats = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total_backups,
            COUNT(*) FILTER (WHERE verified = true) as verified_backups,
            SUM(storage_size_bytes) as total_size_bytes,
            MAX(backup_date) as latest_backup
        FROM contract_backups 
        WHERE contract_id = $1
        "#,
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let total_backups: Option<i64> = stats.get("total_backups");
    let verified_backups: Option<i64> = stats.get("verified_backups");
    let total_size_bytes: Option<i64> = stats.get("total_size_bytes");
    let latest_backup: Option<chrono::NaiveDate> = stats.get("latest_backup");

    Ok(Json(serde_json::json!({
        "total_backups": total_backups.unwrap_or(0),
        "verified_backups": verified_backups.unwrap_or(0),
        "total_size_bytes": total_size_bytes.unwrap_or(0),
        "latest_backup": latest_backup,
    })))
}

// Disaster Recovery Plan Handlers

pub async fn create_disaster_recovery_plan(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<CreateDisasterRecoveryPlanRequest>,
) -> ApiResult<Json<DisasterRecoveryPlan>> {
    let drp = sqlx::query_as::<_, DisasterRecoveryPlan>(
        r#"
        INSERT INTO disaster_recovery_plans 
        (contract_id, rto_minutes, rpo_minutes, recovery_strategy, backup_frequency_minutes)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (contract_id) DO UPDATE 
        SET rto_minutes = $2, rpo_minutes = $3, recovery_strategy = $4, backup_frequency_minutes = $5, updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(req.rto_minutes)
    .bind(req.rpo_minutes)
    .bind(&req.recovery_strategy)
    .bind(req.backup_frequency_minutes)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create disaster recovery plan: {}", e)))?;

    Ok(Json(drp))
}

pub async fn get_disaster_recovery_plan(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<DisasterRecoveryPlan>> {
    let drp = sqlx::query_as::<_, DisasterRecoveryPlan>(
        "SELECT * FROM disaster_recovery_plans WHERE contract_id = $1",
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| {
        ApiError::not_found("disaster_recovery_plan", "Disaster recovery plan not found")
    })?;

    Ok(Json(drp))
}

pub async fn execute_recovery(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<ExecuteRecoveryRequest>,
) -> ApiResult<Json<RecoveryMetrics>> {
    let start_time = std::time::Instant::now();

    // Find the most recent backup based on RPO requirements
    let backup_date = if let Some(target) = req.recovery_target {
        if target == "latest" {
            // Get the latest backup
            let row: Option<(chrono::NaiveDate,)> = sqlx::query_as(
                "SELECT backup_date FROM contract_backups WHERE contract_id = $1 ORDER BY backup_date DESC LIMIT 1"
            )
            .bind(contract_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;
            row.map(|r| r.0)
                .ok_or_else(|| ApiError::not_found("backup", "No backups found for contract"))?
        } else {
            NaiveDate::parse_from_str(&target, "%Y-%m-%d")
                .map_err(|_| ApiError::bad_request_with("invalid_date", "Invalid date format"))?
        }
    } else {
        // Get the latest backup within RPO window
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT rpo_minutes FROM disaster_recovery_plans WHERE contract_id = $1",
        )
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;
        let rpo_minutes = row.map(|r| r.0).unwrap_or(5); // Default to 5 minutes if no DRP exists

        let cutoff_date = (Utc::now() - chrono::Duration::minutes(rpo_minutes as i64)).date_naive();

        let row: Option<(chrono::NaiveDate,)> = sqlx::query_as(
            "SELECT backup_date FROM contract_backups WHERE contract_id = $1 AND backup_date >= $2 ORDER BY backup_date DESC LIMIT 1"
        )
        .bind(contract_id)
        .bind(cutoff_date)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;
        row.map(|r| r.0).ok_or_else(|| {
            ApiError::not_found("backup", "No recent backup found within RPO window")
        })?
    };

    // Perform the restoration
    let restore_req = RestoreBackupRequest {
        backup_date: backup_date.to_string(),
    };

    let restoration =
        restore_backup_from_date(State(state.clone()), Path((contract_id, restore_req))).await?;

    let duration_seconds = start_time.elapsed().as_secs() as i32;
    let data_loss_seconds = (chrono::Duration::minutes(1)).num_seconds() as i32; // Default to 1 minute

    let metrics = RecoveryMetrics {
        rto_achieved_seconds: duration_seconds,
        rpo_ached_seconds: data_loss_seconds,
        recovery_success: restoration.0.success,
        recovery_duration_seconds: duration_seconds,
        data_loss_seconds,
    };

    Ok(Json(metrics))
}

// Helper function for restoration
async fn restore_backup_from_date(
    State(state): State<AppState>,
    Path((contract_id, req)): Path<(Uuid, RestoreBackupRequest)>,
) -> ApiResult<Json<BackupRestoration>> {
    let start = std::time::Instant::now();

    let backup_date = NaiveDate::parse_from_str(&req.backup_date, "%Y-%m-%d")
        .map_err(|_| ApiError::bad_request_with("invalid_date", "Invalid date format"))?;

    let backup = sqlx::query_as::<_, ContractBackup>(
        "SELECT * FROM contract_backups WHERE contract_id = $1 AND backup_date = $2",
    )
    .bind(contract_id)
    .bind(backup_date)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("backup", "Backup not found"))?;

    // Simulate restoration
    let duration_ms = start.elapsed().as_millis() as i32;

    let contract = sqlx::query("SELECT publisher_id FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;
    let publisher_id: Uuid = contract.get("publisher_id");

    let restoration = sqlx::query_as::<_, BackupRestoration>(
        r#"
        INSERT INTO backup_restorations (backup_id, restored_by, restore_duration_ms, success)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(backup.id)
    .bind(publisher_id)
    .bind(duration_ms)
    .bind(true)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to log restoration: {}", e)))?;

    Ok(Json(restoration))
}
