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

// ── Configuration ─────────────────────────────────────────────────────────────

const DEFAULT_INTERVAL_SECONDS: u64 = 86_400;
const DEFAULT_BATCH_SIZE: u32 = 5_000;

/// Operational settings for the scheduled archival sweep.
///
/// The retention window itself is per-policy and lives in
/// `archival_policies.retention_days`, adjustable at runtime through
/// `PATCH /api/admin/archival/policies/:data_type`. These control the job that
/// applies those policies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivalConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub batch_size: u32,
}

impl Default for ArchivalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: DEFAULT_INTERVAL_SECONDS,
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }
}

impl ArchivalConfig {
    pub fn from_env() -> Self {
        let config = Self {
            enabled: env_bool("ARCHIVAL_ENABLED", true),
            interval_seconds: env_u64("ARCHIVAL_INTERVAL_SECONDS", DEFAULT_INTERVAL_SECONDS),
            batch_size: env_u32("ARCHIVAL_BATCH_SIZE", DEFAULT_BATCH_SIZE),
        };

        tracing::info!(
            enabled = config.enabled,
            interval_seconds = config.interval_seconds,
            batch_size = config.batch_size,
            "archival: config loaded"
        );

        config
    }
}

fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(raw) => match raw.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" => true,
            "false" | "0" | "no" => false,
            _ => {
                tracing::warn!("Invalid value for {key} (`{raw}`), using default {default}");
                default
            }
        },
        Err(_) => default,
    }
}

fn env_u32(key: &str, default: u32) -> u32 {
    match std::env::var(key) {
        Ok(raw) => match raw.parse::<u32>() {
            Ok(value) if value > 0 => value,
            _ => {
                tracing::warn!("Invalid value for {key} (`{raw}`), using default {default}");
                default
            }
        },
        Err(_) => default,
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    match std::env::var(key) {
        Ok(raw) => match raw.parse::<u64>() {
            Ok(value) if value > 0 => value,
            _ => {
                tracing::warn!("Invalid value for {key} (`{raw}`), using default {default}");
                default
            }
        },
        Err(_) => default,
    }
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ArchivalPolicy {
    pub id: i64,
    pub data_type: String,
    pub source_table: String,
    pub retention_days: i32,
    pub archive_storage: String,
    pub is_enabled: bool,
    /// Column compared against the retention cutoff.
    pub timestamp_column: String,
    /// When set, only rows whose `status` matches one of these values are
    /// eligible. `None` means age-only retention.
    pub terminal_states: Option<Vec<String>>,
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
        "SELECT id, data_type, source_table, retention_days, archive_storage, is_enabled, timestamp_column, terminal_states, created_at, updated_at FROM archival_policies ORDER BY data_type",
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
        "SELECT id, data_type, source_table, retention_days, archive_storage, is_enabled, timestamp_column, terminal_states, created_at, updated_at FROM archival_policies ORDER BY data_type",
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
        RETURNING id, data_type, source_table, retention_days, archive_storage, is_enabled, timestamp_column, terminal_states, created_at, updated_at
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
            "SELECT id, data_type, source_table, retention_days, archive_storage, is_enabled, timestamp_column, terminal_states, created_at, updated_at FROM archival_policies WHERE data_type = $1 AND is_enabled = true",
        )
        .bind(dt)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal_error("POLICY_FETCH_ERROR", e.to_string()))?,
        None => sqlx::query_as::<_, ArchivalPolicy>(
            "SELECT id, data_type, source_table, retention_days, archive_storage, is_enabled, timestamp_column, terminal_states, created_at, updated_at FROM archival_policies WHERE is_enabled = true ORDER BY data_type",
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
    execute_archival_policy_with(pool, policy, ArchivalConfig::from_env().batch_size).await
}

async fn execute_archival_policy_with(
    pool: &PgPool,
    policy: &ArchivalPolicy,
    batch_size: u32,
) -> ArchivalRun {
    let run_id: i64 = sqlx::query_scalar(
        "INSERT INTO archival_runs (policy_id, data_type, status) VALUES ($1, $2, 'running') RETURNING id",
    )
    .bind(policy.id)
    .bind(&policy.data_type)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let cutoff = Utc::now() - chrono::Duration::days(policy.retention_days as i64);

    let archived = archive_rows(pool, policy, cutoff, run_id, batch_size).await;

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

/// Reject anything that is not a plain SQL identifier.
///
/// `source_table` and `timestamp_column` are interpolated into the archival
/// statements rather than bound, so they must not carry arbitrary SQL even
/// though they originate from an admin-managed table.
fn validate_identifier(value: &str) -> Result<(), sqlx::Error> {
    let valid = !value.is_empty()
        && value.len() <= 63
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');

    if valid {
        Ok(())
    } else {
        Err(sqlx::Error::Protocol(format!(
            "archival policy contains an invalid SQL identifier: {value}"
        )))
    }
}

/// Whether a record qualifies for archival.
///
/// A record sitting exactly on the retention threshold is still inside the
/// window and is retained; only records strictly older than the cutoff are
/// archived. Where a policy declares terminal states, records in any other
/// state are retained regardless of age. The filtering itself runs in SQL, so
/// this is not called on the production path — it pins the boundary semantics
/// the queries below must match.
#[allow(dead_code)]
pub fn is_eligible(
    status: Option<&str>,
    terminal_states: Option<&[String]>,
    timestamp: DateTime<Utc>,
    cutoff: DateTime<Utc>,
) -> bool {
    let state_ok = match terminal_states {
        Some(states) => match status {
            Some(status) => states.iter().any(|s| s == status),
            None => false,
        },
        None => true,
    };

    state_ok && timestamp < cutoff
}

async fn archive_rows(
    pool: &PgPool,
    policy: &ArchivalPolicy,
    cutoff: DateTime<Utc>,
    run_id: i64,
    batch_size: u32,
) -> Result<(i64, i64), sqlx::Error> {
    validate_identifier(&policy.source_table)?;
    validate_identifier(&policy.timestamp_column)?;

    // Policies that name terminal states only ever archive rows sitting in one
    // of them, so live workflow records are never touched however old they are.
    let state_filter = if policy.terminal_states.is_some() {
        " AND status = ANY($3)"
    } else {
        ""
    };

    // Archive and delete share one transaction so a row can never be removed
    // from the source table without its audit-trail copy being committed.
    let mut tx = pool.begin().await?;

    let insert_sql = format!(
        r#"
        WITH eligible AS (
            SELECT * FROM {table}
            WHERE {ts} < $2{state_filter}
            ORDER BY {ts}
            LIMIT {batch_size}
        ),
        archived AS (
            INSERT INTO archival_audit_trail (run_id, source_table, source_id, archived_data, archived_at)
            SELECT $1, '{table}', e.id::TEXT, to_jsonb(e.*), NOW()
            FROM eligible e
            RETURNING 1
        )
        SELECT COUNT(*) FROM archived
        "#,
        table = policy.source_table,
        ts = policy.timestamp_column,
        state_filter = state_filter,
        batch_size = batch_size,
    );

    let mut insert = sqlx::query_scalar::<_, i64>(&insert_sql)
        .bind(run_id)
        .bind(cutoff);
    if let Some(states) = &policy.terminal_states {
        insert = insert.bind(states);
    }
    let archived: i64 = insert.fetch_one(&mut *tx).await?;

    // Delete exactly what was archived in this run. Comparing the key as text
    // keeps this correct for both UUID and integer primary keys; casting the
    // trail's TEXT source_id to BIGINT would fail outright on UUID tables.
    let deleted = sqlx::query(&format!(
        "DELETE FROM {table} WHERE id::TEXT IN (SELECT source_id FROM archival_audit_trail WHERE run_id = $1)",
        table = policy.source_table,
    ))
    .bind(run_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    tx.commit().await?;

    Ok((archived, deleted as i64))
}

// ── Background archival task ──────────────────────────────────────────────────

/// Runs archival daily at midnight UTC.
pub fn spawn_archival_task(pool: PgPool) {
    let config = ArchivalConfig::from_env();

    if !config.enabled {
        tracing::info!("archival: disabled, scheduled sweep not started");
        return;
    }

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(config.interval_seconds));
        loop {
            interval.tick().await;

            let policies: Vec<ArchivalPolicy> = sqlx::query_as::<_, ArchivalPolicy>(
                "SELECT id, data_type, source_table, retention_days, archive_storage, is_enabled, timestamp_column, terminal_states, created_at, updated_at FROM archival_policies WHERE is_enabled = true ORDER BY data_type",
            )
            .fetch_all(&pool)
            .await
            .unwrap_or_default();

            for policy in &policies {
                let run = execute_archival_policy_with(&pool, policy, config.batch_size).await;
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

#[cfg(test)]
mod tests {
    use super::*;

    const OWNERSHIP_TERMINAL: [&str; 3] = ["expired", "rejected", "duplicate"];

    fn terminal() -> Vec<String> {
        OWNERSHIP_TERMINAL.iter().map(|s| s.to_string()).collect()
    }

    fn cutoff_for(now: DateTime<Utc>, retention_days: i64) -> DateTime<Utc> {
        now - chrono::Duration::days(retention_days)
    }

    #[test]
    fn record_exactly_at_threshold_is_retained() {
        let now = Utc::now();
        let cutoff = cutoff_for(now, 90);

        assert!(!is_eligible(
            Some("expired"),
            Some(&terminal()),
            cutoff,
            cutoff
        ));
    }

    #[test]
    fn record_just_under_threshold_is_retained() {
        let now = Utc::now();
        let cutoff = cutoff_for(now, 90);

        assert!(!is_eligible(
            Some("expired"),
            Some(&terminal()),
            cutoff + chrono::Duration::seconds(1),
            cutoff
        ));
    }

    #[test]
    fn record_just_over_threshold_is_archived() {
        let now = Utc::now();
        let cutoff = cutoff_for(now, 90);

        assert!(is_eligible(
            Some("expired"),
            Some(&terminal()),
            cutoff - chrono::Duration::seconds(1),
            cutoff
        ));
    }

    #[test]
    fn active_transfer_states_are_never_archived_regardless_of_age() {
        let now = Utc::now();
        let cutoff = cutoff_for(now, 90);
        let ancient = now - chrono::Duration::days(3650);

        for status in ["pending", "confirmed", "completed"] {
            assert!(
                !is_eligible(Some(status), Some(&terminal()), ancient, cutoff),
                "{status} must never be archived"
            );
        }
    }

    #[test]
    fn every_terminal_state_past_the_window_is_archived() {
        let now = Utc::now();
        let cutoff = cutoff_for(now, 90);
        let old = now - chrono::Duration::days(91);

        for status in OWNERSHIP_TERMINAL {
            assert!(
                is_eligible(Some(status), Some(&terminal()), old, cutoff),
                "{status} should be archived"
            );
        }
    }

    #[test]
    fn completed_is_not_terminal_for_retention() {
        // 'completed' is the successful end state of a transfer and underpins
        // the ownership audit trail, so it is deliberately excluded.
        assert!(!OWNERSHIP_TERMINAL.contains(&"completed"));
    }

    #[test]
    fn age_only_policies_ignore_status() {
        let now = Utc::now();
        let cutoff = cutoff_for(now, 90);

        // Scan runs have no status column; eligibility is purely age-based.
        assert!(is_eligible(None, None, cutoff - chrono::Duration::seconds(1), cutoff));
        assert!(!is_eligible(None, None, cutoff, cutoff));
    }

    #[test]
    fn rows_without_status_are_retained_under_a_terminal_state_policy() {
        let now = Utc::now();
        let cutoff = cutoff_for(now, 90);

        assert!(!is_eligible(
            None,
            Some(&terminal()),
            now - chrono::Duration::days(3650),
            cutoff
        ));
    }

    #[test]
    fn default_config_matches_documented_defaults() {
        let config = ArchivalConfig::default();

        assert!(config.enabled);
        assert_eq!(config.interval_seconds, 86_400);
        assert_eq!(config.batch_size, 5_000);
    }

    #[test]
    fn identifiers_are_validated() {
        assert!(validate_identifier("ownership_transfers").is_ok());
        assert!(validate_identifier("scanned_at").is_ok());

        assert!(validate_identifier("").is_err());
        assert!(validate_identifier("users; DROP TABLE contracts").is_err());
        assert!(validate_identifier("table--comment").is_err());
        assert!(validate_identifier(&"a".repeat(64)).is_err());
    }
}
