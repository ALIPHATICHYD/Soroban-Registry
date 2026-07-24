// migration_handlers.rs
// Database migration framework: version tracking, checksums, advisory locking,
// dry-run preview, apply, rollback, audit trail, and startup validation.
// Issue #877.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
    validation::extractors::{FieldError, Validatable, ValidatedJson},
};

// ─────────────────────────────────────────────────────────
// Models
// ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SchemaVersion {
    pub id: i32,
    pub version: i32,
    pub description: String,
    pub filename: String,
    pub checksum: String,
    pub applied_at: DateTime<Utc>,
    pub applied_by: String,
    pub execution_time_ms: Option<i32>,
    pub rolled_back_at: Option<DateTime<Utc>>,
    pub rollback_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SchemaRollbackScript {
    pub id: i32,
    pub version: i32,
    pub down_sql: String,
    pub checksum: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MigrationAuditEntry {
    pub id: i64,
    pub operation: String,
    pub version: Option<i32>,
    pub actor: String,
    pub success: bool,
    pub detail: Option<String>,
    pub error_msg: Option<String>,
    pub duration_ms: Option<i32>,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MigrationStatusResponse {
    pub current_version: Option<i32>,
    pub total_applied: i64,
    pub total_rolled_back: i64,
    pub pending_count: i64,
    pub versions: Vec<SchemaVersion>,
    pub has_lock: bool,
    pub healthy: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MigrationValidationResponse {
    pub valid: bool,
    pub mismatches: Vec<ChecksumMismatch>,
    pub missing: Vec<i32>,
}

#[derive(Debug, Serialize)]
pub struct ChecksumMismatch {
    pub version: i32,
    pub filename: String,
    pub expected_checksum: String,
    pub actual_checksum: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterMigrationRequest {
    pub version: i32,
    pub description: String,
    pub filename: String,
    pub sql_content: String,
    pub down_sql: Option<String>,
}

// `impl Validatable for RegisterMigrationRequest` lives in
// validation::handler_requests (the centralized location, issue #893).

#[derive(Debug, Deserialize)]
pub struct ApplyMigrationRequest {
    pub version: i32,
    pub description: String,
    pub filename: String,
    pub sql_content: String,
    pub down_sql: Option<String>,
    /// When true, parse and return the statements without executing them.
    #[serde(default)]
    pub dry_run: bool,
}

impl Validatable for ApplyMigrationRequest {
    fn sanitize(&mut self) {
        self.description = self.description.trim().to_string();
        self.filename = self.filename.trim().to_string();
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut errors = Vec::new();
        if self.version <= 0 {
            errors.push(FieldError::new("version", "must be a positive integer"));
        }
        if self.description.trim().is_empty() {
            errors.push(FieldError::new("description", "must not be empty"));
        }
        if self.filename.trim().is_empty() {
            errors.push(FieldError::new("filename", "must not be empty"));
        }
        if self.sql_content.trim().is_empty() {
            errors.push(FieldError::new("sql_content", "must not be empty"));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RegisterMigrationResponse {
    pub version: i32,
    pub checksum: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ApplyMigrationResponse {
    pub version: i32,
    pub checksum: String,
    pub dry_run: bool,
    pub statements_preview: Vec<String>,
    pub execution_time_ms: Option<i32>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct RollbackResponse {
    pub version: i32,
    pub rolled_back_at: DateTime<Utc>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct LockStatusResponse {
    pub locked: bool,
    pub locked_by: Option<String>,
    pub locked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub operation: Option<String>,
    pub version: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub entries: Vec<MigrationAuditEntry>,
    pub total: i64,
}

// ─────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────

/// Compute SHA-256 hex checksum of SQL content.
pub fn compute_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Split SQL text into individual statements (semicolon-delimited, ignoring empty).
fn split_statements(sql: &str) -> Vec<String> {
    sql.split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Advisory lock key for migration operations (arbitrary fixed i64).
const MIGRATION_ADVISORY_LOCK_KEY: i64 = 252_252_252;

async fn try_acquire_lock(pool: &sqlx::PgPool) -> Result<bool, sqlx::Error> {
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(MIGRATION_ADVISORY_LOCK_KEY)
        .fetch_one(pool)
        .await?;
    Ok(acquired)
}

async fn release_lock(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(MIGRATION_ADVISORY_LOCK_KEY)
        .execute(pool)
        .await?;
    Ok(())
}

/// Append a row to `migration_audit_log`. Best-effort: failures are logged but
/// never returned as errors so that auditing never blocks the primary operation.
async fn audit(
    pool: &sqlx::PgPool,
    operation: &str,
    version: Option<i32>,
    success: bool,
    detail: Option<&str>,
    error_msg: Option<&str>,
    duration_ms: Option<i32>,
) {
    let result = sqlx::query(
        r#"
        INSERT INTO migration_audit_log
            (operation, version, success, detail, error_msg, duration_ms)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(operation)
    .bind(version)
    .bind(success)
    .bind(detail)
    .bind(error_msg)
    .bind(duration_ms)
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::warn!(
            operation,
            ?version,
            "Failed to write migration audit entry: {e}"
        );
    }
}

// ─────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────

/// GET /api/admin/migrations/status
pub async fn get_migration_status(
    State(state): State<AppState>,
) -> ApiResult<Json<MigrationStatusResponse>> {
    let versions: Vec<SchemaVersion> = sqlx::query_as(
        r#"
        SELECT id, version, description, filename, checksum,
               applied_at, applied_by, execution_time_ms,
               rolled_back_at, rollback_by
        FROM schema_versions
        ORDER BY version ASC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let current_version = versions
        .iter()
        .filter(|v| v.rolled_back_at.is_none())
        .map(|v| v.version)
        .max();

    let total_applied = versions
        .iter()
        .filter(|v| v.rolled_back_at.is_none())
        .count() as i64;

    let total_rolled_back = versions
        .iter()
        .filter(|v| v.rolled_back_at.is_some())
        .count() as i64;

    // Check advisory lock status: try-and-immediately-release avoids side effects.
    let has_lock: bool =
        sqlx::query_scalar("SELECT NOT pg_try_advisory_lock($1) OR pg_advisory_unlock($1)")
            .bind(MIGRATION_ADVISORY_LOCK_KEY)
            .fetch_one(&state.db)
            .await
            .unwrap_or(false);

    let mut warnings = Vec::new();
    let active_versions: Vec<i32> = versions
        .iter()
        .filter(|v| v.rolled_back_at.is_none())
        .map(|v| v.version)
        .collect();

    if let (Some(&min), Some(&max)) = (active_versions.first(), active_versions.last()) {
        for expected in min..=max {
            if !active_versions.contains(&expected) {
                warnings.push(format!("Gap detected: version {} is missing", expected));
            }
        }
    }

    let healthy = warnings.is_empty();

    Ok(Json(MigrationStatusResponse {
        current_version,
        total_applied,
        total_rolled_back,
        pending_count: 0,
        versions,
        has_lock,
        healthy,
        warnings,
    }))
}

/// POST /api/admin/migrations/register
///
/// Register migration metadata (checksum, rollback script) without executing it.
/// Use `/apply` to both register and execute.
pub async fn register_migration(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<RegisterMigrationRequest>,
) -> ApiResult<Json<RegisterMigrationResponse>> {
    let acquired = try_acquire_lock(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Lock error: {e}")))?;

    if !acquired {
        audit(
            &state.db,
            "register",
            Some(body.version),
            false,
            None,
            Some("Lock not acquired"),
            None,
        )
        .await;
        return Err(ApiError::conflict(
            "MigrationLocked",
            "Another migration operation is in progress. Please try again later.",
        ));
    }

    let start = std::time::Instant::now();
    let result = register_migration_inner(&state, &body).await;
    let elapsed = start.elapsed().as_millis() as i32;

    let _ = release_lock(&state.db).await;

    match &result {
        Ok(r) => {
            audit(
                &state.db,
                "register",
                Some(body.version),
                true,
                Some(&format!("checksum={}", r.checksum)),
                None,
                Some(elapsed),
            )
            .await;
        }
        Err(e) => {
            audit(
                &state.db,
                "register",
                Some(body.version),
                false,
                None,
                Some(&e.to_string()),
                Some(elapsed),
            )
            .await;
        }
    }

    result
}

async fn register_migration_inner(
    state: &AppState,
    body: &RegisterMigrationRequest,
) -> ApiResult<Json<RegisterMigrationResponse>> {
    let exists: bool =
        sqlx::query_scalar("SELECT COUNT(*) > 0 FROM schema_versions WHERE version = $1")
            .bind(body.version)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    if exists {
        return Err(ApiError::conflict(
            "VersionExists",
            format!("Migration version {} is already registered", body.version),
        ));
    }

    let checksum = compute_checksum(&body.sql_content);

    sqlx::query(
        r#"
        INSERT INTO schema_versions (version, description, filename, checksum)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(body.version)
    .bind(&body.description)
    .bind(&body.filename)
    .bind(&checksum)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    if let Some(down_sql) = &body.down_sql {
        let down_checksum = compute_checksum(down_sql);
        sqlx::query(
            r#"
            INSERT INTO schema_rollback_scripts (version, down_sql, checksum)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(body.version)
        .bind(down_sql)
        .bind(&down_checksum)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;
    }

    Ok(Json(RegisterMigrationResponse {
        version: body.version,
        checksum,
        message: format!("Migration version {} registered successfully", body.version),
    }))
}

/// POST /api/admin/migrations/apply
///
/// Apply (execute) a migration's SQL against the database and register it.
/// Pass `dry_run: true` to preview the parsed statements without executing.
pub async fn apply_migration(
    State(state): State<AppState>,
    ValidatedJson(body): ValidatedJson<ApplyMigrationRequest>,
) -> ApiResult<Json<ApplyMigrationResponse>> {
    let statements = split_statements(&body.sql_content);
    let checksum = compute_checksum(&body.sql_content);

    if body.dry_run {
        audit(
            &state.db,
            "dry_run",
            Some(body.version),
            true,
            Some(&format!("{} statements parsed", statements.len())),
            None,
            Some(0),
        )
        .await;

        return Ok(Json(ApplyMigrationResponse {
            version: body.version,
            checksum,
            dry_run: true,
            statements_preview: statements,
            execution_time_ms: None,
            message: format!(
                "Dry-run for version {}: {} statement(s) would be executed",
                body.version,
                split_statements(&body.sql_content).len()
            ),
        }));
    }

    let acquired = try_acquire_lock(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Lock error: {e}")))?;

    if !acquired {
        audit(
            &state.db,
            "apply",
            Some(body.version),
            false,
            None,
            Some("Lock not acquired"),
            None,
        )
        .await;
        return Err(ApiError::conflict(
            "MigrationLocked",
            "Another migration operation is in progress. Please try again later.",
        ));
    }

    let start = std::time::Instant::now();
    let result = apply_migration_inner(&state, &body, &checksum, &statements).await;
    let elapsed = start.elapsed().as_millis() as i32;

    let _ = release_lock(&state.db).await;

    match &result {
        Ok(_) => {
            audit(
                &state.db,
                "apply",
                Some(body.version),
                true,
                Some(&format!(
                    "checksum={}, statements={}",
                    checksum,
                    statements.len()
                )),
                None,
                Some(elapsed),
            )
            .await;
        }
        Err(e) => {
            audit(
                &state.db,
                "apply",
                Some(body.version),
                false,
                None,
                Some(&e.to_string()),
                Some(elapsed),
            )
            .await;
        }
    }

    result
}

async fn apply_migration_inner(
    state: &AppState,
    body: &ApplyMigrationRequest,
    checksum: &str,
    statements: &[String],
) -> ApiResult<Json<ApplyMigrationResponse>> {
    let exists: bool =
        sqlx::query_scalar("SELECT COUNT(*) > 0 FROM schema_versions WHERE version = $1")
            .bind(body.version)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    if exists {
        return Err(ApiError::conflict(
            "VersionExists",
            format!("Migration version {} is already applied", body.version),
        ));
    }

    // Execute each statement in sequence.
    for stmt in statements {
        sqlx::query(stmt).execute(&state.db).await.map_err(|e| {
            ApiError::internal(format!(
                "Failed to execute statement for version {}: {e}",
                body.version
            ))
        })?;
    }

    let elapsed_ms = 0i32; // timing captured by outer apply_migration
    sqlx::query(
        r#"
        INSERT INTO schema_versions
            (version, description, filename, checksum, execution_time_ms)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(body.version)
    .bind(&body.description)
    .bind(&body.filename)
    .bind(checksum)
    .bind(elapsed_ms)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    if let Some(down_sql) = &body.down_sql {
        let down_checksum = compute_checksum(down_sql);
        sqlx::query(
            r#"
            INSERT INTO schema_rollback_scripts (version, down_sql, checksum)
            VALUES ($1, $2, $3)
            ON CONFLICT (version) DO UPDATE
                SET down_sql = EXCLUDED.down_sql,
                    checksum = EXCLUDED.checksum
            "#,
        )
        .bind(body.version)
        .bind(down_sql)
        .bind(&down_checksum)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;
    }

    Ok(Json(ApplyMigrationResponse {
        version: body.version,
        checksum: checksum.to_string(),
        dry_run: false,
        statements_preview: statements.to_vec(),
        execution_time_ms: None,
        message: format!(
            "Migration version {} applied successfully ({} statement(s))",
            body.version,
            statements.len()
        ),
    }))
}

/// POST /api/admin/migrations/:version/rollback
pub async fn rollback_migration(
    State(state): State<AppState>,
    Path(version): Path<i32>,
) -> ApiResult<Json<RollbackResponse>> {
    let acquired = try_acquire_lock(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Lock error: {e}")))?;

    if !acquired {
        audit(
            &state.db,
            "rollback",
            Some(version),
            false,
            None,
            Some("Lock not acquired"),
            None,
        )
        .await;
        return Err(ApiError::conflict(
            "MigrationLocked",
            "Another migration operation is in progress. Please try again later.",
        ));
    }

    let start = std::time::Instant::now();
    let result = rollback_migration_inner(&state, version).await;
    let elapsed = start.elapsed().as_millis() as i32;

    let _ = release_lock(&state.db).await;

    match &result {
        Ok(_) => {
            audit(
                &state.db,
                "rollback",
                Some(version),
                true,
                None,
                None,
                Some(elapsed),
            )
            .await;
        }
        Err(e) => {
            audit(
                &state.db,
                "rollback",
                Some(version),
                false,
                None,
                Some(&e.to_string()),
                Some(elapsed),
            )
            .await;
        }
    }

    result
}

async fn rollback_migration_inner(
    state: &AppState,
    version: i32,
) -> ApiResult<Json<RollbackResponse>> {
    let migration: Option<SchemaVersion> = sqlx::query_as(
        r#"
        SELECT id, version, description, filename, checksum,
               applied_at, applied_by, execution_time_ms,
               rolled_back_at, rollback_by
        FROM schema_versions
        WHERE version = $1
        "#,
    )
    .bind(version)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let migration = migration.ok_or_else(|| {
        ApiError::not_found(
            "NotFound",
            format!("Migration version {} not found", version),
        )
    })?;

    if migration.rolled_back_at.is_some() {
        return Err(ApiError::conflict(
            "AlreadyRolledBack",
            format!("Migration version {} has already been rolled back", version),
        ));
    }

    let rollback: Option<SchemaRollbackScript> = sqlx::query_as(
        r#"
        SELECT id, version, down_sql, checksum, created_at
        FROM schema_rollback_scripts
        WHERE version = $1
        "#,
    )
    .bind(version)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let rollback = rollback.ok_or_else(|| {
        ApiError::not_found(
            "NoRollbackScript",
            format!("No rollback script found for migration version {}", version),
        )
    })?;

    let actual_checksum = compute_checksum(&rollback.down_sql);
    if actual_checksum != rollback.checksum {
        return Err(ApiError::conflict(
            "ChecksumMismatch",
            format!(
                "Rollback script checksum mismatch for version {}. Expected: {}, Got: {}",
                version, rollback.checksum, actual_checksum
            ),
        ));
    }

    sqlx::query(&rollback.down_sql)
        .execute(&state.db)
        .await
        .map_err(|e| {
            ApiError::internal(format!(
                "Failed to execute rollback for version {}: {}",
                version, e
            ))
        })?;

    let now = Utc::now();
    sqlx::query(
        "UPDATE schema_versions SET rolled_back_at = $1, rollback_by = current_user WHERE version = $2",
    )
    .bind(now)
    .bind(version)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    Ok(Json(RollbackResponse {
        version,
        rolled_back_at: now,
        message: format!("Migration version {} rolled back successfully", version),
    }))
}

/// GET /api/admin/migrations/validate
pub async fn validate_migrations(
    State(state): State<AppState>,
) -> ApiResult<Json<MigrationValidationResponse>> {
    let versions: Vec<SchemaVersion> = sqlx::query_as(
        r#"
        SELECT id, version, description, filename, checksum,
               applied_at, applied_by, execution_time_ms,
               rolled_back_at, rollback_by
        FROM schema_versions
        WHERE rolled_back_at IS NULL
        ORDER BY version ASC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let mut missing = Vec::new();
    if let (Some(first), Some(last)) = (versions.first(), versions.last()) {
        for v in first.version..=last.version {
            if !versions.iter().any(|sv| sv.version == v) {
                missing.push(v);
            }
        }
    }

    let rollback_scripts: Vec<SchemaRollbackScript> = sqlx::query_as(
        r#"
        SELECT id, version, down_sql, checksum, created_at
        FROM schema_rollback_scripts
        ORDER BY version ASC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let mut mismatches = Vec::new();
    for script in &rollback_scripts {
        let actual = compute_checksum(&script.down_sql);
        if actual != script.checksum {
            mismatches.push(ChecksumMismatch {
                version: script.version,
                filename: format!("rollback_v{}", script.version),
                expected_checksum: script.checksum.clone(),
                actual_checksum: actual,
            });
        }
    }

    let valid = mismatches.is_empty() && missing.is_empty();

    audit(
        &state.db,
        "validate",
        None,
        true,
        Some(&format!(
            "valid={}, mismatches={}, missing={}",
            valid,
            mismatches.len(),
            missing.len()
        )),
        None,
        None,
    )
    .await;

    Ok(Json(MigrationValidationResponse {
        valid,
        mismatches,
        missing,
    }))
}

/// GET /api/admin/migrations/:version
pub async fn get_migration_version(
    State(state): State<AppState>,
    Path(version): Path<i32>,
) -> ApiResult<Json<SchemaVersion>> {
    let migration: Option<SchemaVersion> = sqlx::query_as(
        r#"
        SELECT id, version, description, filename, checksum,
               applied_at, applied_by, execution_time_ms,
               rolled_back_at, rollback_by
        FROM schema_versions
        WHERE version = $1
        "#,
    )
    .bind(version)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    migration.map(Json).ok_or_else(|| {
        ApiError::not_found(
            "NotFound",
            format!("Migration version {} not found", version),
        )
    })
}

/// GET /api/admin/migrations/lock
pub async fn get_lock_status(State(state): State<AppState>) -> ApiResult<Json<LockStatusResponse>> {
    let can_lock = try_acquire_lock(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Lock error: {e}")))?;

    if can_lock {
        let _ = release_lock(&state.db).await;
    }

    let lock_row: Option<(Option<String>, Option<DateTime<Utc>>)> =
        sqlx::query_as("SELECT locked_by, locked_at FROM schema_migration_locks WHERE id = 1")
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let (locked_by, locked_at) = lock_row.unwrap_or((None, None));

    Ok(Json(LockStatusResponse {
        locked: !can_lock,
        locked_by,
        locked_at,
    }))
}

/// GET /api/admin/migrations/audit
///
/// Returns the migration audit trail with optional filtering by operation or version.
pub async fn get_migration_audit(
    State(state): State<AppState>,
    Query(params): Query<AuditLogQuery>,
) -> ApiResult<Json<AuditLogResponse>> {
    let limit = params.limit.unwrap_or(50).clamp(1, 500);
    let offset = params.offset.unwrap_or(0).max(0);

    let total: i64 =
        match (&params.operation, &params.version) {
            (Some(op), Some(v)) => sqlx::query_scalar(
                "SELECT COUNT(*) FROM migration_audit_log WHERE operation = $1 AND version = $2",
            )
            .bind(op)
            .bind(v)
            .fetch_one(&state.db)
            .await,
            (Some(op), None) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM migration_audit_log WHERE operation = $1")
                    .bind(op)
                    .fetch_one(&state.db)
                    .await
            }
            (None, Some(v)) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM migration_audit_log WHERE version = $1")
                    .bind(v)
                    .fetch_one(&state.db)
                    .await
            }
            (None, None) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM migration_audit_log")
                    .fetch_one(&state.db)
                    .await
            }
        }
        .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let entries: Vec<MigrationAuditEntry> = match (&params.operation, &params.version) {
        (Some(op), Some(v)) => sqlx::query_as(
            r#"
            SELECT id, operation, version, actor, success, detail, error_msg, duration_ms, occurred_at
            FROM migration_audit_log
            WHERE operation = $1 AND version = $2
            ORDER BY occurred_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(op)
        .bind(v)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await,
        (Some(op), None) => sqlx::query_as(
            r#"
            SELECT id, operation, version, actor, success, detail, error_msg, duration_ms, occurred_at
            FROM migration_audit_log
            WHERE operation = $1
            ORDER BY occurred_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(op)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await,
        (None, Some(v)) => sqlx::query_as(
            r#"
            SELECT id, operation, version, actor, success, detail, error_msg, duration_ms, occurred_at
            FROM migration_audit_log
            WHERE version = $1
            ORDER BY occurred_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(v)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await,
        (None, None) => sqlx::query_as(
            r#"
            SELECT id, operation, version, actor, success, detail, error_msg, duration_ms, occurred_at
            FROM migration_audit_log
            ORDER BY occurred_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await,
    }
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    Ok(Json(AuditLogResponse { entries, total }))
}

/// Startup check: validates migration state and logs warnings.
pub async fn check_migrations_on_startup(pool: &sqlx::PgPool) {
    let table_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_name = 'schema_versions'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !table_exists {
        tracing::warn!("schema_versions table not found. Migration versioning is not initialized.");
        return;
    }

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM schema_versions WHERE rolled_back_at IS NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let current_version: Option<i32> =
        sqlx::query_scalar("SELECT MAX(version) FROM schema_versions WHERE rolled_back_at IS NULL")
            .fetch_one(pool)
            .await
            .unwrap_or(None);

    tracing::info!(
        applied_migrations = count,
        current_version = ?current_version,
        "Migration versioning status"
    );

    let mismatch_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM schema_rollback_scripts s
        WHERE s.checksum != encode(sha256(s.down_sql::bytea), 'hex')
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if mismatch_count > 0 {
        tracing::warn!(
            mismatch_count = mismatch_count,
            "Rollback script checksum mismatches detected! Migration integrity may be compromised."
        );
    }

    let versions: Vec<i32> = sqlx::query_scalar(
        "SELECT version FROM schema_versions WHERE rolled_back_at IS NULL ORDER BY version",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if let (Some(&min), Some(&max)) = (versions.first(), versions.last()) {
        let expected_count = (max - min + 1) as usize;
        if versions.len() != expected_count {
            tracing::warn!(
                expected = expected_count,
                actual = versions.len(),
                "Version gaps detected in migration history"
            );
        }
    }
}
