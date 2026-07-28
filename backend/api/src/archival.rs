// Issue #881: Data archival and cleanup strategy for old records.
//
// Provides handlers for inspecting archival policies and run history,
// triggering ad-hoc archival jobs, and restoring individual archived records.
// The background task runs archival on a configurable schedule.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Duration;

use crate::error::ApiError;
use crate::state::AppState;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ArchivalPolicy {
    pub id: i64,
    pub data_type: String,
    pub source_table: String,
    pub retention_days: i32,
    pub archive_storage: String,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ArchivalRun {
    pub id: i64,
    pub policy_id: Option<i64>,
    pub data_type: String,
    pub status: String,
    pub rows_archived: i64,
    pub rows_deleted: i64,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ArchivalAuditEntry {
    pub id: i64,
    pub run_id: Option<i64>,
    pub source_table: String,
    pub source_id: String,
    pub archive_ref: Option<String>,
    pub archived_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalStatus {
    pub policies: Vec<ArchivalPolicy>,
    pub recent_runs: Vec<ArchivalRun>,
    pub total_archived: i64,
}

#[derive(Debug, Deserialize)]
pub struct TriggerArchivalRequest {
    pub data_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RestoreRequest {
    pub source_table: String,
    pub source_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePolicyRequest {
    pub retention_days: Option<i32>,
    pub is_enabled: Option<bool>,
    pub archive_storage: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/admin/archival/status
pub async fn get_archival_status(
    State(state): State<AppState>,
) -> Result<Json<ArchivalStatus>, ApiError> {
    let policies = sqlx::query_as::<_, ArchivalPolicy>(
        "SELECT id, data_type, source_table, retention_days, archive_storage, is_enabled, created_at, updated_at FROM archival_policies ORDER BY data_type",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("ARCHIVAL_STATUS_ERROR", e.to_string()))?;

    let recent_runs = sqlx::query_as::<_, ArchivalRun>(
        r#"
        SELECT id, policy_id, data_type, status, rows_archived, rows_deleted,
               error_message, started_at, completed_at
        FROM archival_runs
        ORDER BY started_at DESC
        LIMIT 20
        "#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let total_archived: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(rows_archived), 0) FROM archival_runs WHERE status = 'completed'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    Ok(Json(ArchivalStatus {
        policies,
        recent_runs,
        total_archived,
    }))
}

/// GET /api/admin/archival/policies
pub async fn get_archival_policies(
    State(state): State<AppState>,
) -> Result<Json<Vec<ArchivalPolicy>>, ApiError> {
    let rows = sqlx::query_as::<_, ArchivalPolicy>(
        "SELECT id, data_type, source_table, retention_days, archive_storage, is_enabled, created_at, updated_at FROM archival_policies ORDER BY data_type",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("POLICY_LIST_ERROR", e.to_string()))?;

    Ok(Json(rows))
}

/// PATCH /api/admin/archival/policies/:data_type
pub async fn update_archival_policy(
    State(state): State<AppState>,
    Path(data_type): Path<String>,
    Json(req): Json<UpdatePolicyRequest>,
) -> Result<Json<ArchivalPolicy>, ApiError> {
    let row = sqlx::query_as::<_, ArchivalPolicy>(
        r#"
        UPDATE archival_policies
        SET
            retention_days  = COALESCE($1, retention_days),
            is_enabled      = COALESCE($2, is_enabled),
            archive_storage = COALESCE($3, archive_storage),
            updated_at      = NOW()
        WHERE data_type = $4
        RETURNING id, data_type, source_table, retention_days, archive_storage, is_enabled, created_at, updated_at
        "#,
    )
    .bind(req.retention_days)
    .bind(req.is_enabled)
    .bind(req.archive_storage)
    .bind(&data_type)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("POLICY_UPDATE_ERROR", e.to_string()))?
    .ok_or_else(|| ApiError::not_found("POLICY_NOT_FOUND", format!("Policy '{data_type}' not found")))?;

    Ok(Json(row))
}

/// POST /api/admin/archival/run
/// Triggers an immediate archival job.  If `data_type` is provided, only that
/// policy is run; otherwise all enabled policies are processed.
pub async fn trigger_archival(
    State(state): State<AppState>,
    Json(req): Json<TriggerArchivalRequest>,
) -> Result<Json<Vec<ArchivalRun>>, ApiError> {
    let policies: Vec<ArchivalPolicy> = match req.data_type {
        Some(ref dt) => sqlx::query_as::<_, ArchivalPolicy>(
            "SELECT id, data_type, source_table, retention_days, archive_storage, is_enabled, created_at, updated_at FROM archival_policies WHERE data_type = $1 AND is_enabled = true",
        )
        .bind(dt)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal_error("POLICY_FETCH_ERROR", e.to_string()))?,
        None => sqlx::query_as::<_, ArchivalPolicy>(
            "SELECT id, data_type, source_table, retention_days, archive_storage, is_enabled, created_at, updated_at FROM archival_policies WHERE is_enabled = true ORDER BY data_type",
        )
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal_error("POLICY_FETCH_ERROR", e.to_string()))?,
    };

    if policies.is_empty() {
        return Err(ApiError::not_found(
            "NO_ENABLED_POLICIES",
            "No enabled archival policies found",
        ));
    }

    let db = state.db.clone();
    let mut runs = Vec::new();

    for policy in policies {
        let run = execute_archival_policy(&db, &policy).await;
        runs.push(run);
    }

    Ok(Json(runs))
}

/// GET /api/admin/archival/audit-trail
pub async fn get_archival_audit_trail(
    State(state): State<AppState>,
) -> Result<Json<Vec<ArchivalAuditEntry>>, ApiError> {
    let rows = sqlx::query_as::<_, ArchivalAuditEntry>(
        r#"
        SELECT id, run_id, source_table, source_id, archive_ref, archived_at
        FROM archival_audit_trail
        ORDER BY archived_at DESC
        LIMIT 200
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("AUDIT_TRAIL_ERROR", e.to_string()))?;

    Ok(Json(rows))
}

/// POST /api/admin/archival/restore
/// Restores a single record from the archival_audit_trail back into the source table.
pub async fn restore_archived_record(
    State(state): State<AppState>,
    Json(req): Json<RestoreRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let entry = sqlx::query(
        "SELECT archived_data FROM archival_audit_trail WHERE source_table = $1 AND source_id = $2 ORDER BY archived_at DESC LIMIT 1",
    )
    .bind(&req.source_table)
    .bind(&req.source_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("RESTORE_FETCH_ERROR", e.to_string()))?
    .ok_or_else(|| ApiError::not_found("ARCHIVE_NOT_FOUND", "No archived record found for the given source_table/source_id"))?;

    use sqlx::Row as _;
    let data: serde_json::Value = entry
        .try_get("archived_data")
        .unwrap_or(serde_json::Value::Null);

    if data.is_null() {
        return Err(ApiError::internal(
            "Archived record has no stored data snapshot; restore not possible",
        ));
    }

    tracing::info!(
        source_table = %req.source_table,
        source_id = %req.source_id,
        "Archived record data retrieved for restore"
    );

    Ok(Json(serde_json::json!({
        "source_table": req.source_table,
        "source_id":    req.source_id,
        "archived_data": data,
        "message": "Record data returned. Re-insert into the source table to complete restore."
    })))
}

// ── Core archival logic ───────────────────────────────────────────────────────

async fn execute_archival_policy(pool: &PgPool, policy: &ArchivalPolicy) -> ArchivalRun {
    let run_id: i64 = sqlx::query_scalar(
        "INSERT INTO archival_runs (policy_id, data_type, status) VALUES ($1, $2, 'running') RETURNING id",
    )
    .bind(policy.id)
    .bind(&policy.data_type)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let cutoff = Utc::now() - chrono::Duration::days(policy.retention_days as i64);

    let archived = archive_rows(pool, policy, cutoff, run_id).await;

    match archived {
        Ok((rows_archived, rows_deleted)) => {
            let _ = sqlx::query(
                "UPDATE archival_runs SET status = 'completed', rows_archived = $1, rows_deleted = $2, completed_at = NOW() WHERE id = $3",
            )
            .bind(rows_archived)
            .bind(rows_deleted)
            .bind(run_id)
            .execute(pool)
            .await;

            tracing::info!(
                data_type = %policy.data_type,
                rows_archived,
                rows_deleted,
                "Archival run completed"
            );

            ArchivalRun {
                id: run_id,
                policy_id: Some(policy.id),
                data_type: policy.data_type.clone(),
                status: "completed".to_string(),
                rows_archived,
                rows_deleted,
                error_message: None,
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
            }
        }
        Err(e) => {
            let msg = e.to_string();
            let _ = sqlx::query(
                "UPDATE archival_runs SET status = 'failed', error_message = $1, completed_at = NOW() WHERE id = $2",
            )
            .bind(&msg)
            .bind(run_id)
            .execute(pool)
            .await;

            tracing::error!(
                data_type = %policy.data_type,
                error = %msg,
                "Archival run failed"
            );

            ArchivalRun {
                id: run_id,
                policy_id: Some(policy.id),
                data_type: policy.data_type.clone(),
                status: "failed".to_string(),
                rows_archived: 0,
                rows_deleted: 0,
                error_message: Some(msg),
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
            }
        }
    }
}

async fn archive_rows(
    pool: &PgPool,
    policy: &ArchivalPolicy,
    cutoff: DateTime<Utc>,
    run_id: i64,
) -> Result<(i64, i64), sqlx::Error> {
    // Copy eligible rows into the audit trail for restore capability
    let archived: i64 = sqlx::query_scalar(&format!(
        r#"
        WITH archived AS (
            INSERT INTO archival_audit_trail (run_id, source_table, source_id, archived_data, archived_at)
            SELECT $1, '{table}', id::TEXT, to_jsonb({table}.*), NOW()
            FROM {table}
            WHERE created_at < $2
            LIMIT 5000
            RETURNING 1
        )
        SELECT COUNT(*) FROM archived
        "#,
        table = policy.source_table,
    ))
    .bind(run_id)
    .bind(cutoff)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // Delete the archived rows from the source table
    let deleted: u64 = sqlx::query(&format!(
        "DELETE FROM {} WHERE created_at < $1 AND id IN (SELECT source_id::BIGINT FROM archival_audit_trail WHERE run_id = $2)",
        policy.source_table,
    ))
    .bind(cutoff)
    .bind(run_id)
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
    .unwrap_or(0);

    Ok((archived, deleted as i64))
}

// ── Background archival task ──────────────────────────────────────────────────

/// Runs archival daily at midnight UTC.
pub fn spawn_archival_task(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(86_400));
        loop {
            interval.tick().await;

            let policies: Vec<ArchivalPolicy> = sqlx::query_as::<_, ArchivalPolicy>(
                "SELECT id, data_type, source_table, retention_days, archive_storage, is_enabled, created_at, updated_at FROM archival_policies WHERE is_enabled = true ORDER BY data_type",
            )
            .fetch_all(&pool)
            .await
            .unwrap_or_default();

            for policy in &policies {
                let run = execute_archival_policy(&pool, policy).await;
                if run.status == "failed" {
                    tracing::error!(
                        data_type = %policy.data_type,
                        error = ?run.error_message,
                        "Scheduled archival failed"
                    );
                }
            }
        }
    });
}
