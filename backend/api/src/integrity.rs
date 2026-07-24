// Issue #886: Data integrity verification and checksums.
//
// Detecting silent data corruption is critical for registry reliability. This
// module computes SHA-256 checksums over the canonicalized bytes of contract
// data and ABIs, persists them as baselines, and re-verifies them on demand and
// on a periodic schedule.
//
// Capabilities:
//   • Compute & store checksums for contract core data and every stored ABI.
//   • Verify a single contract or the whole system against its baselines.
//   • Detect corruption (checksum mismatch), missing baselines, and missing
//     data, logging each as a durable `integrity_issues` record and alerting via
//     structured `tracing::error!` events.
//   • Block access to data whose live checksum no longer matches its baseline.
//   • Repair minor corruption (missing baselines) automatically, and re-baseline
//     mismatches when an operator explicitly forces it.
//   • Report integrity status across the registry.
//
// The background task runs a full-system verification on a configurable interval.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

// ── Resource / issue / severity constants ─────────────────────────────────────

pub const RESOURCE_CONTRACT: &str = "contract";
pub const RESOURCE_ABI: &str = "abi";

const ISSUE_MISMATCH: &str = "checksum_mismatch";
const ISSUE_MISSING_CHECKSUM: &str = "missing_checksum";
const ISSUE_MISSING_DATA: &str = "missing_data";

const SEVERITY_MINOR: &str = "minor";
const SEVERITY_MAJOR: &str = "major";
const SEVERITY_CRITICAL: &str = "critical";

// ── Persisted types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DataChecksum {
    pub id: i64,
    pub resource_type: String,
    pub resource_id: String,
    pub contract_id: Option<Uuid>,
    pub algorithm: String,
    pub checksum: String,
    pub byte_size: i64,
    pub status: String,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IntegrityVerificationRun {
    pub id: i64,
    pub scope: String,
    pub contract_id: Option<Uuid>,
    pub status: String,
    pub total_checked: i64,
    pub valid_count: i64,
    pub mismatch_count: i64,
    pub missing_count: i64,
    pub repaired_count: i64,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IntegrityIssue {
    pub id: i64,
    pub run_id: Option<i64>,
    pub resource_type: String,
    pub resource_id: String,
    pub contract_id: Option<Uuid>,
    pub issue_type: String,
    pub expected_checksum: Option<String>,
    pub actual_checksum: Option<String>,
    pub severity: String,
    pub status: String,
    pub details: Value,
    pub detected_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

// ── Request / response DTOs ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ChecksumComputationResult {
    pub contract_id: Uuid,
    pub checksums: Vec<DataChecksum>,
}

/// Per-contract verification verdict returned by the verify endpoints.
#[derive(Debug, Serialize)]
pub struct VerificationResult {
    pub contract_id: Uuid,
    pub total_checked: i64,
    pub valid_count: i64,
    pub mismatch_count: i64,
    pub missing_count: i64,
    /// `true` when every checked resource matched its baseline.
    pub healthy: bool,
    pub issues: Vec<IntegrityIssue>,
}

#[derive(Debug, Default, Deserialize)]
pub struct FullVerificationRequest {
    /// When true, compute baselines for contracts that lack a contract-level
    /// checksum before verifying, making the sweep genuinely system-wide.
    #[serde(default)]
    pub compute_missing: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct RepairRequest {
    /// Re-baseline checksum mismatches (overwrite the stored baseline with the
    /// freshly computed value). Off by default because it masks real corruption.
    #[serde(default)]
    pub force_rebaseline: bool,
}

#[derive(Debug, Deserialize)]
pub struct IssueListQuery {
    pub status: Option<String>,
    pub severity: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct IntegrityStatusReport {
    pub total_checksums: i64,
    pub valid_checksums: i64,
    pub mismatch_checksums: i64,
    pub missing_checksums: i64,
    pub open_issues: i64,
    pub open_critical_issues: i64,
    pub last_run: Option<IntegrityVerificationRun>,
    pub recent_runs: Vec<IntegrityVerificationRun>,
}

#[derive(Debug, Serialize)]
pub struct RepairResult {
    pub contract_id: Uuid,
    pub repaired_count: i64,
    pub remaining_open_issues: i64,
    pub issues: Vec<IntegrityIssue>,
}

// ── Checksum primitives ───────────────────────────────────────────────────────

/// Deterministically serialize a JSON value with object keys sorted so the
/// resulting bytes (and therefore the checksum) are stable regardless of the
/// key ordering returned by Postgres for JSONB columns.
fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push(b'{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                // `to_string` on a string Value yields a correctly-escaped JSON string.
                out.extend_from_slice(Value::String((*key).clone()).to_string().as_bytes());
                out.push(b':');
                write_canonical(&map[*key], out);
            }
            out.push(b'}');
        }
        Value::Array(arr) => {
            out.push(b'[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_canonical(item, out);
            }
            out.push(b']');
        }
        other => out.extend_from_slice(other.to_string().as_bytes()),
    }
}

/// Hex-encoded SHA-256 of `bytes`.
fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// A checksum computed from live data, not yet compared to a baseline.
struct ComputedChecksum {
    resource_type: &'static str,
    resource_id: String,
    contract_id: Uuid,
    checksum: String,
    byte_size: i64,
}

fn computed_from_value(
    resource_type: &'static str,
    resource_id: String,
    contract_id: Uuid,
    value: &Value,
) -> ComputedChecksum {
    let bytes = canonical_json_bytes(value);
    ComputedChecksum {
        resource_type,
        resource_id,
        contract_id,
        checksum: sha256_hex(&bytes),
        byte_size: bytes.len() as i64,
    }
}

// ── Live data → computed checksums ────────────────────────────────────────────

/// Compute checksums for a contract's core data and all of its stored ABIs.
/// Returns an empty vec when the contract does not exist.
async fn compute_contract_checksums(
    pool: &PgPool,
    contract_id: Uuid,
) -> Result<Vec<ComputedChecksum>, sqlx::Error> {
    let mut computed = Vec::new();

    // Core contract data. Only the fields that define the contract's identity and
    // metadata are included; volatile columns like `updated_at` are excluded so a
    // routine touch does not register as corruption.
    let core: Option<(
        String,
        String,
        String,
        Option<String>,
        String,
        Option<String>,
        Vec<String>,
        Uuid,
    )> = sqlx::query_as(
        r#"
            SELECT contract_id, wasm_hash, name, description,
                   network::text, category, tags, publisher_id
            FROM contracts
            WHERE id = $1
            "#,
    )
    .bind(contract_id)
    .fetch_optional(pool)
    .await?;

    let Some((on_chain_id, wasm_hash, name, description, network, category, tags, publisher_id)) =
        core
    else {
        return Ok(computed);
    };

    let core_value = json!({
        "contract_id": on_chain_id,
        "wasm_hash": wasm_hash,
        "name": name,
        "description": description,
        "network": network,
        "category": category,
        "tags": tags,
        "publisher_id": publisher_id,
    });
    computed.push(computed_from_value(
        RESOURCE_CONTRACT,
        contract_id.to_string(),
        contract_id,
        &core_value,
    ));

    // Every stored ABI version for this contract.
    let abis: Vec<(Uuid, String, Value)> = sqlx::query_as(
        "SELECT id, version, abi FROM contract_abis WHERE contract_id = $1 ORDER BY created_at",
    )
    .bind(contract_id)
    .fetch_all(pool)
    .await?;

    for (abi_id, version, abi) in abis {
        // Bind the version into the hashed payload so renaming a version is detected.
        let abi_value = json!({ "version": version, "abi": abi });
        computed.push(computed_from_value(
            RESOURCE_ABI,
            abi_id.to_string(),
            contract_id,
            &abi_value,
        ));
    }

    Ok(computed)
}

/// Upsert baseline rows for the supplied computed checksums, marking them valid.
async fn store_checksums(pool: &PgPool, computed: &[ComputedChecksum]) -> Result<(), sqlx::Error> {
    for c in computed {
        sqlx::query(
            r#"
            INSERT INTO data_checksums
                (resource_type, resource_id, contract_id, algorithm, checksum, byte_size, status, last_verified_at)
            VALUES ($1, $2, $3, 'sha256', $4, $5, 'valid', NOW())
            ON CONFLICT (resource_type, resource_id) DO UPDATE
            SET checksum = EXCLUDED.checksum,
                byte_size = EXCLUDED.byte_size,
                contract_id = EXCLUDED.contract_id,
                status = 'valid',
                last_verified_at = NOW(),
                updated_at = NOW()
            "#,
        )
        .bind(c.resource_type)
        .bind(&c.resource_id)
        .bind(c.contract_id)
        .bind(&c.checksum)
        .bind(c.byte_size)
        .execute(pool)
        .await?;
    }
    Ok(())
}

// ── Issue logging ─────────────────────────────────────────────────────────────

/// Record an integrity issue, de-duplicating against any existing open issue for
/// the same resource and issue type so repeated verification runs do not spam the
/// log. Emits a `tracing::error!` alert for mismatches and missing data.
#[allow(clippy::too_many_arguments)]
async fn log_issue(
    pool: &PgPool,
    run_id: Option<i64>,
    resource_type: &str,
    resource_id: &str,
    contract_id: Uuid,
    issue_type: &str,
    severity: &str,
    expected: Option<&str>,
    actual: Option<&str>,
) -> Result<(), sqlx::Error> {
    if severity == SEVERITY_CRITICAL || issue_type == ISSUE_MISSING_DATA {
        tracing::error!(
            target: "data_integrity",
            %resource_type,
            %resource_id,
            contract_id = %contract_id,
            %issue_type,
            %severity,
            expected = expected.unwrap_or(""),
            actual = actual.unwrap_or(""),
            "data integrity issue detected"
        );
    } else {
        tracing::warn!(
            target: "data_integrity",
            %resource_type,
            %resource_id,
            contract_id = %contract_id,
            %issue_type,
            %severity,
            "data integrity issue detected"
        );
    }

    sqlx::query(
        r#"
        INSERT INTO integrity_issues
            (run_id, resource_type, resource_id, contract_id, issue_type,
             expected_checksum, actual_checksum, severity, status, details)
        SELECT $1, $2, $3, $4, $5, $6, $7, $8, 'open', $9
        WHERE NOT EXISTS (
            SELECT 1 FROM integrity_issues
            WHERE resource_type = $2 AND resource_id = $3
              AND issue_type = $5 AND status = 'open'
        )
        "#,
    )
    .bind(run_id)
    .bind(resource_type)
    .bind(resource_id)
    .bind(contract_id)
    .bind(issue_type)
    .bind(expected)
    .bind(actual)
    .bind(severity)
    .bind(json!({ "detected_by": run_id.map(|_| "run").unwrap_or("on_access") }))
    .execute(pool)
    .await?;

    Ok(())
}

// ── Verification core ─────────────────────────────────────────────────────────

#[derive(Default)]
struct VerifyCounts {
    total: i64,
    valid: i64,
    mismatch: i64,
    missing: i64,
}

/// Recompute checksums for a contract and compare against stored baselines,
/// updating baseline status and logging any issues. Returns the tallied counts.
async fn verify_contract(
    pool: &PgPool,
    contract_id: Uuid,
    run_id: Option<i64>,
) -> Result<VerifyCounts, sqlx::Error> {
    let computed = compute_contract_checksums(pool, contract_id).await?;
    let baselines: Vec<DataChecksum> =
        sqlx::query_as::<_, DataChecksum>("SELECT * FROM data_checksums WHERE contract_id = $1")
            .bind(contract_id)
            .fetch_all(pool)
            .await?;

    let mut counts = VerifyCounts::default();

    // Verify each baseline against freshly computed data.
    for baseline in &baselines {
        counts.total += 1;
        let current = computed.iter().find(|c| {
            c.resource_type == baseline.resource_type && c.resource_id == baseline.resource_id
        });

        match current {
            None => {
                // Baseline exists but the underlying data is gone.
                counts.missing += 1;
                set_checksum_status(pool, baseline.id, "missing").await?;
                log_issue(
                    pool,
                    run_id,
                    &baseline.resource_type,
                    &baseline.resource_id,
                    contract_id,
                    ISSUE_MISSING_DATA,
                    SEVERITY_MAJOR,
                    Some(&baseline.checksum),
                    None,
                )
                .await?;
            }
            Some(c) if c.checksum != baseline.checksum => {
                // Live data no longer matches the trusted baseline: corruption.
                counts.mismatch += 1;
                set_checksum_status(pool, baseline.id, "mismatch").await?;
                log_issue(
                    pool,
                    run_id,
                    &baseline.resource_type,
                    &baseline.resource_id,
                    contract_id,
                    ISSUE_MISMATCH,
                    SEVERITY_CRITICAL,
                    Some(&baseline.checksum),
                    Some(&c.checksum),
                )
                .await?;
            }
            Some(_) => {
                counts.valid += 1;
                sqlx::query(
                    "UPDATE data_checksums SET status = 'valid', last_verified_at = NOW(), updated_at = NOW() WHERE id = $1",
                )
                .bind(baseline.id)
                .execute(pool)
                .await?;
            }
        }
    }

    // Data that exists but has no baseline yet (minor: just needs a baseline).
    for c in &computed {
        let has_baseline = baselines
            .iter()
            .any(|b| b.resource_type == c.resource_type && b.resource_id == c.resource_id);
        if !has_baseline {
            counts.total += 1;
            counts.missing += 1;
            log_issue(
                pool,
                run_id,
                c.resource_type,
                &c.resource_id,
                contract_id,
                ISSUE_MISSING_CHECKSUM,
                SEVERITY_MINOR,
                None,
                Some(&c.checksum),
            )
            .await?;
        }
    }

    Ok(counts)
}

async fn set_checksum_status(pool: &PgPool, id: i64, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE data_checksums SET status = $1, last_verified_at = NOW(), updated_at = NOW() WHERE id = $2",
    )
    .bind(status)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Returns true when the contract currently has at least one open critical
/// integrity issue (i.e. confirmed checksum mismatch). Used to block access.
async fn has_blocking_issue(pool: &PgPool, contract_id: Uuid) -> Result<bool, sqlx::Error> {
    let blocking: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM integrity_issues
        WHERE contract_id = $1 AND status = 'open' AND severity = 'critical'
        "#,
    )
    .bind(contract_id)
    .fetch_one(pool)
    .await?;
    Ok(blocking > 0)
}

/// Guard helper other handlers can call to refuse serving data that failed its
/// integrity check. Returns `409 Conflict` when a blocking issue is present.
pub async fn guard_contract_access(pool: &PgPool, contract_id: Uuid) -> Result<(), ApiError> {
    match has_blocking_issue(pool, contract_id).await {
        Ok(true) => Err(ApiError::conflict(
            "DATA_INTEGRITY_VIOLATION",
            "Access blocked: contract data failed integrity verification",
        )),
        Ok(false) => Ok(()),
        // Fail open on infrastructure errors so the guard can never take down reads.
        Err(e) => {
            tracing::error!(error = %e, contract_id = %contract_id, "integrity guard query failed");
            Ok(())
        }
    }
}

// ── Handlers: per-contract ────────────────────────────────────────────────────

/// POST /api/contracts/:id/integrity/checksums
/// Compute and store baseline checksums for a contract's data and ABIs.
pub async fn compute_checksums_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> Result<Json<ChecksumComputationResult>, ApiError> {
    let computed = compute_contract_checksums(&state.db, contract_id)
        .await
        .map_err(|e| ApiError::internal_error("CHECKSUM_COMPUTE_ERROR", e.to_string()))?;

    if computed.is_empty() {
        return Err(ApiError::not_found(
            "CONTRACT_NOT_FOUND",
            "Contract not found",
        ));
    }

    store_checksums(&state.db, &computed)
        .await
        .map_err(|e| ApiError::internal_error("CHECKSUM_STORE_ERROR", e.to_string()))?;

    let checksums = sqlx::query_as::<_, DataChecksum>(
        "SELECT * FROM data_checksums WHERE contract_id = $1 ORDER BY resource_type, resource_id",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("CHECKSUM_FETCH_ERROR", e.to_string()))?;

    Ok(Json(ChecksumComputationResult {
        contract_id,
        checksums,
    }))
}

/// GET /api/contracts/:id/integrity
/// List the stored checksum baselines for a contract.
pub async fn get_checksums_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> Result<Json<Vec<DataChecksum>>, ApiError> {
    let checksums = sqlx::query_as::<_, DataChecksum>(
        "SELECT * FROM data_checksums WHERE contract_id = $1 ORDER BY resource_type, resource_id",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("CHECKSUM_FETCH_ERROR", e.to_string()))?;

    Ok(Json(checksums))
}

/// POST /api/contracts/:id/integrity/verify
/// Recompute and compare a contract's checksums, logging any issues found.
pub async fn verify_contract_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> Result<Json<VerificationResult>, ApiError> {
    let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal_error("CONTRACT_LOOKUP_ERROR", e.to_string()))?;
    if exists.is_none() {
        return Err(ApiError::not_found(
            "CONTRACT_NOT_FOUND",
            "Contract not found",
        ));
    }

    let counts = verify_contract(&state.db, contract_id, None)
        .await
        .map_err(|e| ApiError::internal_error("VERIFY_ERROR", e.to_string()))?;

    let issues = sqlx::query_as::<_, IntegrityIssue>(
        "SELECT * FROM integrity_issues WHERE contract_id = $1 AND status = 'open' ORDER BY detected_at DESC",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("ISSUE_FETCH_ERROR", e.to_string()))?;

    Ok(Json(VerificationResult {
        contract_id,
        total_checked: counts.total,
        valid_count: counts.valid,
        mismatch_count: counts.mismatch,
        missing_count: counts.missing,
        healthy: counts.mismatch == 0 && counts.missing == 0,
        issues,
    }))
}

/// GET /api/contracts/:id/integrity/access-check
/// Returns 200 when integrity is intact, or 409 when access should be blocked.
pub async fn access_check_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    guard_contract_access(&state.db, contract_id).await?;
    Ok(Json(
        json!({ "contract_id": contract_id, "access": "allowed" }),
    ))
}

/// POST /api/contracts/:id/integrity/repair
/// Repair minor corruption: create missing baselines and (optionally) re-baseline
/// confirmed mismatches when `force_rebaseline` is set.
pub async fn repair_contract_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    body: Option<Json<RepairRequest>>,
) -> Result<Json<RepairResult>, ApiError> {
    let req = body.map(|Json(r)| r).unwrap_or_default();

    let computed = compute_contract_checksums(&state.db, contract_id)
        .await
        .map_err(|e| ApiError::internal_error("CHECKSUM_COMPUTE_ERROR", e.to_string()))?;
    if computed.is_empty() {
        return Err(ApiError::not_found(
            "CONTRACT_NOT_FOUND",
            "Contract not found",
        ));
    }

    let open_issues = sqlx::query_as::<_, IntegrityIssue>(
        "SELECT * FROM integrity_issues WHERE contract_id = $1 AND status = 'open'",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("ISSUE_FETCH_ERROR", e.to_string()))?;

    let mut repaired = 0i64;

    for issue in &open_issues {
        let current = computed
            .iter()
            .find(|c| c.resource_type == issue.resource_type && c.resource_id == issue.resource_id);

        let can_repair = match issue.issue_type.as_str() {
            // Minor: data is present, baseline simply absent → create it.
            ISSUE_MISSING_CHECKSUM => current.is_some(),
            // Critical: only re-baseline when the operator explicitly forces it.
            ISSUE_MISMATCH => req.force_rebaseline && current.is_some(),
            // Missing data cannot be reconstructed here.
            _ => false,
        };

        if !can_repair {
            continue;
        }

        if let Some(c) = current {
            store_checksums(&state.db, std::slice::from_ref(c))
                .await
                .map_err(|e| ApiError::internal_error("CHECKSUM_STORE_ERROR", e.to_string()))?;
            sqlx::query(
                "UPDATE integrity_issues SET status = 'repaired', resolved_at = NOW() WHERE id = $1",
            )
            .bind(issue.id)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::internal_error("ISSUE_UPDATE_ERROR", e.to_string()))?;
            repaired += 1;
            tracing::info!(
                target: "data_integrity",
                contract_id = %contract_id,
                resource_id = %issue.resource_id,
                issue_type = %issue.issue_type,
                "integrity issue repaired"
            );
        }
    }

    let remaining: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM integrity_issues WHERE contract_id = $1 AND status = 'open'",
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("ISSUE_COUNT_ERROR", e.to_string()))?;

    let issues = sqlx::query_as::<_, IntegrityIssue>(
        "SELECT * FROM integrity_issues WHERE contract_id = $1 ORDER BY detected_at DESC LIMIT 50",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("ISSUE_FETCH_ERROR", e.to_string()))?;

    Ok(Json(RepairResult {
        contract_id,
        repaired_count: repaired,
        remaining_open_issues: remaining,
        issues,
    }))
}

// ── Handlers: admin / system-wide ─────────────────────────────────────────────

/// POST /api/admin/integrity/verify
/// Trigger a full-system integrity verification sweep.
pub async fn trigger_full_verification_handler(
    State(state): State<AppState>,
    body: Option<Json<FullVerificationRequest>>,
) -> Result<Json<IntegrityVerificationRun>, ApiError> {
    let req = body.map(|Json(r)| r).unwrap_or_default();
    let run = run_full_verification(&state.db, req.compute_missing)
        .await
        .map_err(|e| ApiError::internal_error("FULL_VERIFY_ERROR", e.to_string()))?;
    Ok(Json(run))
}

/// GET /api/admin/integrity/status
/// Registry-wide integrity status report.
pub async fn get_integrity_status_handler(
    State(state): State<AppState>,
) -> Result<Json<IntegrityStatusReport>, ApiError> {
    let row = sqlx::query_as::<_, (i64, i64, i64, i64)>(
        r#"
        SELECT
            COUNT(*),
            COUNT(*) FILTER (WHERE status = 'valid'),
            COUNT(*) FILTER (WHERE status = 'mismatch'),
            COUNT(*) FILTER (WHERE status = 'missing')
        FROM data_checksums
        "#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("STATUS_ERROR", e.to_string()))?;

    let (open_issues, open_critical): (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE status = 'open'),
            COUNT(*) FILTER (WHERE status = 'open' AND severity = 'critical')
        FROM integrity_issues
        "#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("STATUS_ERROR", e.to_string()))?;

    let recent_runs = sqlx::query_as::<_, IntegrityVerificationRun>(
        "SELECT * FROM integrity_verification_runs ORDER BY started_at DESC LIMIT 10",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("STATUS_ERROR", e.to_string()))?;

    Ok(Json(IntegrityStatusReport {
        total_checksums: row.0,
        valid_checksums: row.1,
        mismatch_checksums: row.2,
        missing_checksums: row.3,
        open_issues,
        open_critical_issues: open_critical,
        last_run: recent_runs.first().cloned(),
        recent_runs,
    }))
}

/// GET /api/admin/integrity/runs
pub async fn list_runs_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<IntegrityVerificationRun>>, ApiError> {
    let runs = sqlx::query_as::<_, IntegrityVerificationRun>(
        "SELECT * FROM integrity_verification_runs ORDER BY started_at DESC LIMIT 50",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("RUN_LIST_ERROR", e.to_string()))?;
    Ok(Json(runs))
}

/// GET /api/admin/integrity/issues
pub async fn list_issues_handler(
    State(state): State<AppState>,
    Query(q): Query<IssueListQuery>,
) -> Result<Json<Vec<IntegrityIssue>>, ApiError> {
    let limit = q.limit.unwrap_or(100).clamp(1, 500);
    let issues = sqlx::query_as::<_, IntegrityIssue>(
        r#"
        SELECT * FROM integrity_issues
        WHERE ($1::text IS NULL OR status = $1)
          AND ($2::text IS NULL OR severity = $2)
        ORDER BY detected_at DESC
        LIMIT $3
        "#,
    )
    .bind(q.status)
    .bind(q.severity)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("ISSUE_LIST_ERROR", e.to_string()))?;
    Ok(Json(issues))
}

// ── Full-system verification ──────────────────────────────────────────────────

/// Run a system-wide verification sweep, recording a run row with aggregate
/// counts. When `compute_missing` is set, contracts without a contract-level
/// baseline are baselined first so the sweep covers the entire registry.
pub async fn run_full_verification(
    pool: &PgPool,
    compute_missing: bool,
) -> Result<IntegrityVerificationRun, sqlx::Error> {
    let run_id: i64 = sqlx::query_scalar(
        "INSERT INTO integrity_verification_runs (scope, status) VALUES ('full', 'running') RETURNING id",
    )
    .fetch_one(pool)
    .await?;

    let result = full_verification_inner(pool, run_id, compute_missing).await;

    match result {
        Ok(counts) => {
            let run = sqlx::query_as::<_, IntegrityVerificationRun>(
                r#"
                UPDATE integrity_verification_runs
                SET status = 'completed',
                    total_checked = $1,
                    valid_count = $2,
                    mismatch_count = $3,
                    missing_count = $4,
                    completed_at = NOW()
                WHERE id = $5
                RETURNING *
                "#,
            )
            .bind(counts.total)
            .bind(counts.valid)
            .bind(counts.mismatch)
            .bind(counts.missing)
            .bind(run_id)
            .fetch_one(pool)
            .await?;

            tracing::info!(
                target: "data_integrity",
                run_id,
                total = counts.total,
                valid = counts.valid,
                mismatch = counts.mismatch,
                missing = counts.missing,
                "full integrity verification completed"
            );
            Ok(run)
        }
        Err(e) => {
            let msg = e.to_string();
            let run = sqlx::query_as::<_, IntegrityVerificationRun>(
                "UPDATE integrity_verification_runs SET status = 'failed', error_message = $1, completed_at = NOW() WHERE id = $2 RETURNING *",
            )
            .bind(&msg)
            .bind(run_id)
            .fetch_one(pool)
            .await?;
            tracing::error!(target: "data_integrity", run_id, error = %msg, "full integrity verification failed");
            Ok(run)
        }
    }
}

async fn full_verification_inner(
    pool: &PgPool,
    run_id: i64,
    compute_missing: bool,
) -> Result<VerifyCounts, sqlx::Error> {
    if compute_missing {
        // Baseline every contract that has no contract-level checksum yet.
        let unbaselined: Vec<Uuid> = sqlx::query_scalar(
            r#"
            SELECT c.id FROM contracts c
            WHERE NOT EXISTS (
                SELECT 1 FROM data_checksums d
                WHERE d.contract_id = c.id AND d.resource_type = 'contract'
            )
            "#,
        )
        .fetch_all(pool)
        .await?;

        for contract_id in unbaselined {
            let computed = compute_contract_checksums(pool, contract_id).await?;
            store_checksums(pool, &computed).await?;
        }
    }

    // Verify every contract that currently has at least one baseline.
    let contract_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT contract_id FROM data_checksums WHERE contract_id IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    let mut totals = VerifyCounts::default();
    for contract_id in contract_ids {
        let counts = verify_contract(pool, contract_id, Some(run_id)).await?;
        totals.total += counts.total;
        totals.valid += counts.valid;
        totals.mismatch += counts.mismatch;
        totals.missing += counts.missing;
    }

    Ok(totals)
}

// ── Background task ───────────────────────────────────────────────────────────

/// Spawn the periodic full-system integrity verification task.
///
/// Interval is configurable via `INTEGRITY_VERIFICATION_INTERVAL_SECS`
/// (default 21600 = 6 hours). The first sweep runs one interval after startup.
pub fn spawn_integrity_verification_task(pool: PgPool) {
    let interval_secs = std::env::var("INTEGRITY_VERIFICATION_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(21_600);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        // Skip the immediate first tick so we do not verify before the app is warm.
        interval.tick().await;
        loop {
            interval.tick().await;
            tracing::info!(target: "data_integrity", "starting periodic full-system integrity verification");
            // `compute_missing = false`: the periodic sweep only re-verifies known
            // baselines; baselining new contracts is an explicit operation.
            if let Err(e) = run_full_verification(&pool, false).await {
                tracing::error!(target: "data_integrity", error = %e, "periodic integrity verification failed");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_json_is_key_order_independent() {
        let a: Value = serde_json::from_str(r#"{"b":1,"a":2,"c":{"y":1,"x":2}}"#).unwrap();
        let b: Value = serde_json::from_str(r#"{"c":{"x":2,"y":1},"a":2,"b":1}"#).unwrap();
        assert_eq!(canonical_json_bytes(&a), canonical_json_bytes(&b));
    }

    #[test]
    fn canonical_json_distinguishes_different_values() {
        let a: Value = serde_json::from_str(r#"{"a":1}"#).unwrap();
        let b: Value = serde_json::from_str(r#"{"a":2}"#).unwrap();
        assert_ne!(canonical_json_bytes(&a), canonical_json_bytes(&b));
    }

    #[test]
    fn sha256_hex_is_stable_and_correct_length() {
        let h1 = sha256_hex(b"hello");
        let h2 = sha256_hex(b"hello");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        // Known SHA-256 of "hello".
        assert_eq!(
            h1,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn computed_checksum_changes_when_value_changes() {
        let id = Uuid::nil();
        let v1 = json!({ "wasm_hash": "aaa", "name": "x" });
        let v2 = json!({ "wasm_hash": "bbb", "name": "x" });
        let c1 = computed_from_value(RESOURCE_CONTRACT, id.to_string(), id, &v1);
        let c2 = computed_from_value(RESOURCE_CONTRACT, id.to_string(), id, &v2);
        assert_ne!(c1.checksum, c2.checksum);
        assert!(c1.byte_size > 0);
    }
}
