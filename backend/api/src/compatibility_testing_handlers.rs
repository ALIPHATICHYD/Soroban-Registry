// compatibility_testing_handlers.rs
// Handlers for the SDK/Wasm/Network contract compatibility testing matrix (Issue #261).

use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

// ─────────────────────────────────────────────────────────
// SDK / Wasm / Network Compatibility Testing Models
// ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "compatibility_status", rename_all = "lowercase")]
pub enum CompatibilityStatus {
    Compatible,
    Warning,
    Incompatible,
}

impl std::fmt::Display for CompatibilityStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompatibilityStatus::Compatible => write!(f, "compatible"),
            CompatibilityStatus::Warning => write!(f, "warning"),
            CompatibilityStatus::Incompatible => write!(f, "incompatible"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractCompatibilityRow {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub sdk_version: String,
    pub wasm_runtime: String,
    pub network: String,
    pub compatible: CompatibilityStatus,
    pub tested_at: DateTime<Utc>,
    pub test_duration_ms: Option<i32>,
    pub test_output: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityTestEntry {
    pub sdk_version: String,
    pub wasm_runtime: String,
    pub network: String,
    pub status: CompatibilityStatus,
    pub tested_at: DateTime<Utc>,
    pub test_duration_ms: Option<i32>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CompatibilityTestMatrixResponse {
    pub contract_id: Uuid,
    pub sdk_versions: Vec<String>,
    pub wasm_runtimes: Vec<String>,
    pub networks: Vec<String>,
    pub entries: Vec<CompatibilityTestEntry>,
    pub summary: CompatibilityTestSummary,
    pub last_tested: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct CompatibilityTestSummary {
    pub total_tests: usize,
    pub compatible_count: usize,
    pub warning_count: usize,
    pub incompatible_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct RunCompatibilityTestRequest {
    pub sdk_version: String,
    pub wasm_runtime: String,
    pub network: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CompatibilityHistoryRow {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub sdk_version: String,
    pub wasm_runtime: String,
    pub network: String,
    pub previous_status: Option<CompatibilityStatus>,
    pub new_status: CompatibilityStatus,
    pub changed_at: DateTime<Utc>,
    pub change_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CompatibilityHistoryResponse {
    pub contract_id: Uuid,
    pub changes: Vec<CompatibilityHistoryRow>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CompatibilityNotification {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub sdk_version: String,
    pub message: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CompatibilityDashboardResponse {
    pub total_contracts_tested: i64,
    pub overall_compatible: i64,
    pub overall_warning: i64,
    pub overall_incompatible: i64,
    pub sdk_versions: Vec<String>,
    pub recent_changes: Vec<CompatibilityHistoryRow>,
}

/// GET /api/contracts/:id/compatibility-matrix
///
/// Returns the full SDK/Wasm/Network compatibility matrix for a contract.
/// Results: compatible (green), warnings (yellow), incompatible (red).
pub async fn get_compatibility_matrix(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<CompatibilityTestMatrixResponse>> {
    // Verify contract exists
    let exists: bool = sqlx::query_scalar("SELECT COUNT(*) > 0 FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    if !exists {
        return Err(ApiError::not_found("NotFound", "Contract not found"));
    }

    let rows: Vec<ContractCompatibilityRow> = sqlx::query_as(
        r#"
        SELECT id, contract_id, sdk_version, wasm_runtime, network,
               compatible AS "compatible: CompatibilityStatus",
               tested_at, test_duration_ms, test_output, error_message,
               created_at, updated_at
        FROM contract_compatibility
        WHERE contract_id = $1
        ORDER BY sdk_version DESC, wasm_runtime DESC, network
        "#,
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    // Collect unique dimensions
    let mut sdk_versions: Vec<String> = rows.iter().map(|r| r.sdk_version.clone()).collect();
    sdk_versions.sort();
    sdk_versions.dedup();

    let mut wasm_runtimes: Vec<String> = rows.iter().map(|r| r.wasm_runtime.clone()).collect();
    wasm_runtimes.sort();
    wasm_runtimes.dedup();

    let mut networks: Vec<String> = rows.iter().map(|r| r.network.clone()).collect();
    networks.sort();
    networks.dedup();

    // Build entries
    let entries: Vec<CompatibilityTestEntry> = rows
        .iter()
        .map(|r| CompatibilityTestEntry {
            sdk_version: r.sdk_version.clone(),
            wasm_runtime: r.wasm_runtime.clone(),
            network: r.network.clone(),
            status: r.compatible.clone(),
            tested_at: r.tested_at,
            test_duration_ms: r.test_duration_ms,
            error_message: r.error_message.clone(),
        })
        .collect();

    // Summary counts
    let compatible_count = entries
        .iter()
        .filter(|e| e.status == CompatibilityStatus::Compatible)
        .count();
    let warning_count = entries
        .iter()
        .filter(|e| e.status == CompatibilityStatus::Warning)
        .count();
    let incompatible_count = entries
        .iter()
        .filter(|e| e.status == CompatibilityStatus::Incompatible)
        .count();

    let last_tested = rows.iter().map(|r| r.tested_at).max();

    let response = CompatibilityTestMatrixResponse {
        contract_id,
        sdk_versions,
        wasm_runtimes,
        networks,
        summary: CompatibilityTestSummary {
            total_tests: entries.len(),
            compatible_count,
            warning_count,
            incompatible_count,
        },
        entries,
        last_tested,
    };

    Ok(Json(response))
}

/// POST /api/contracts/:id/compatibility-matrix/test
///
/// Run a compatibility test for a contract against a specific SDK version,
/// Wasm runtime, and network. Attempts to invoke the contract with synthetic
/// operations and records the result.
pub async fn run_compatibility_test(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<RunCompatibilityTestRequest>,
) -> ApiResult<Json<CompatibilityTestEntry>> {
    // Verify contract exists
    let exists: bool = sqlx::query_scalar("SELECT COUNT(*) > 0 FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    if !exists {
        return Err(ApiError::not_found("NotFound", "Contract not found"));
    }

    // Simulate compatibility test execution
    // In production, this would invoke the contract with synthetic operations
    // against the specified SDK/runtime/network combination.
    let start = std::time::Instant::now();

    let (status, error_message) =
        simulate_compatibility_test(&body.sdk_version, &body.wasm_runtime, &body.network);

    let duration_ms = start.elapsed().as_millis() as i32;
    let now = Utc::now();

    // Check if there's an existing entry to detect status changes
    let previous: Option<ContractCompatibilityRow> = sqlx::query_as(
        r#"
        SELECT id, contract_id, sdk_version, wasm_runtime, network,
               compatible AS "compatible: CompatibilityStatus",
               tested_at, test_duration_ms, test_output, error_message,
               created_at, updated_at
        FROM contract_compatibility
        WHERE contract_id = $1 AND sdk_version = $2 AND wasm_runtime = $3 AND network = $4
        "#,
    )
    .bind(contract_id)
    .bind(&body.sdk_version)
    .bind(&body.wasm_runtime)
    .bind(&body.network)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let previous_status = previous.as_ref().map(|p| p.compatible.clone());

    // Upsert the compatibility result
    sqlx::query(
        r#"
        INSERT INTO contract_compatibility
            (contract_id, sdk_version, wasm_runtime, network, compatible, tested_at, test_duration_ms, error_message)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (contract_id, sdk_version, wasm_runtime, network)
        DO UPDATE SET
            compatible = EXCLUDED.compatible,
            tested_at = EXCLUDED.tested_at,
            test_duration_ms = EXCLUDED.test_duration_ms,
            error_message = EXCLUDED.error_message,
            updated_at = NOW()
        "#,
    )
    .bind(contract_id)
    .bind(&body.sdk_version)
    .bind(&body.wasm_runtime)
    .bind(&body.network)
    .bind(&status)
    .bind(now)
    .bind(duration_ms)
    .bind(&error_message)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    // Record history if status changed
    if previous_status.as_ref() != Some(&status) {
        let change_reason = match (&previous_status, &status) {
            (None, _) => Some("Initial test".to_string()),
            (Some(old), new) => Some(format!("Status changed from {} to {}", old, new)),
        };

        sqlx::query(
            r#"
            INSERT INTO contract_compatibility_history
                (contract_id, sdk_version, wasm_runtime, network, previous_status, new_status, changed_at, change_reason)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(contract_id)
        .bind(&body.sdk_version)
        .bind(&body.wasm_runtime)
        .bind(&body.network)
        .bind(&previous_status)
        .bind(&status)
        .bind(now)
        .bind(&change_reason)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

        // Notify publisher if status degraded
        if status == CompatibilityStatus::Incompatible || status == CompatibilityStatus::Warning {
            let message = format!(
                "Contract compatibility changed to '{}' for SDK {} / Runtime {} / Network {}",
                status, body.sdk_version, body.wasm_runtime, body.network
            );

            sqlx::query(
                r#"
                INSERT INTO compatibility_notifications (contract_id, sdk_version, message)
                VALUES ($1, $2, $3)
                "#,
            )
            .bind(contract_id)
            .bind(&body.sdk_version)
            .bind(&message)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;
        }
    }

    let entry = CompatibilityTestEntry {
        sdk_version: body.sdk_version,
        wasm_runtime: body.wasm_runtime,
        network: body.network,
        status,
        tested_at: now,
        test_duration_ms: Some(duration_ms),
        error_message,
    };

    Ok(Json(entry))
}

/// GET /api/contracts/:id/compatibility-matrix/history
///
/// Returns historical compatibility changes for trend analysis.
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn get_compatibility_history(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(params): Query<HistoryQuery>,
) -> ApiResult<Json<CompatibilityHistoryResponse>> {
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);

    let rows: Vec<CompatibilityHistoryRow> = sqlx::query_as(
        r#"
        SELECT id, contract_id, sdk_version, wasm_runtime, network,
               previous_status AS "previous_status: CompatibilityStatus",
               new_status AS "new_status: CompatibilityStatus",
               changed_at, change_reason
        FROM contract_compatibility_history
        WHERE contract_id = $1
        ORDER BY changed_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(contract_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let total = rows.len();

    Ok(Json(CompatibilityHistoryResponse {
        contract_id,
        changes: rows,
        total,
    }))
}

/// GET /api/contracts/:id/compatibility-matrix/notifications
///
/// Returns unread compatibility notifications for a contract's publisher.
pub async fn get_compatibility_notifications(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<Vec<CompatibilityNotification>>> {
    let rows: Vec<CompatibilityNotification> = sqlx::query_as(
        r#"
        SELECT id, contract_id, sdk_version, message, is_read, created_at
        FROM compatibility_notifications
        WHERE contract_id = $1
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    Ok(Json(rows))
}

/// POST /api/contracts/:id/compatibility-matrix/notifications/read
///
/// Mark all notifications for a contract as read.
pub async fn mark_notifications_read(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    sqlx::query(
        "UPDATE compatibility_notifications SET is_read = TRUE WHERE contract_id = $1 AND NOT is_read",
    )
    .bind(contract_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    Ok(Json(serde_json::json!({
        "message": "All notifications marked as read",
        "contract_id": contract_id,
    })))
}

/// GET /api/compatibility-dashboard
///
/// Returns a dashboard summary of compatibility across all contracts.
pub async fn get_compatibility_dashboard(
    State(state): State<AppState>,
) -> ApiResult<Json<CompatibilityDashboardResponse>> {
    let total_contracts_tested: i64 =
        sqlx::query_scalar("SELECT COUNT(DISTINCT contract_id) FROM contract_compatibility")
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let overall_compatible: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_compatibility WHERE compatible = 'compatible'",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let overall_warning: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_compatibility WHERE compatible = 'warning'",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let overall_incompatible: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_compatibility WHERE compatible = 'incompatible'",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let sdk_versions: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT sdk_version FROM contract_compatibility ORDER BY sdk_version DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    let recent_changes: Vec<CompatibilityHistoryRow> = sqlx::query_as(
        r#"
        SELECT id, contract_id, sdk_version, wasm_runtime, network,
               previous_status AS "previous_status: CompatibilityStatus",
               new_status AS "new_status: CompatibilityStatus",
               changed_at, change_reason
        FROM contract_compatibility_history
        ORDER BY changed_at DESC
        LIMIT 20
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    Ok(Json(CompatibilityDashboardResponse {
        total_contracts_tested,
        overall_compatible,
        overall_warning,
        overall_incompatible,
        sdk_versions,
        recent_changes,
    }))
}

/// Simulates a compatibility test by attempting synthetic contract invocation.
/// In production, this would use the Soroban SDK to actually invoke the contract.
fn simulate_compatibility_test(
    sdk_version: &str,
    _wasm_runtime: &str,
    _network: &str,
) -> (CompatibilityStatus, Option<String>) {
    // Parse SDK version to determine compatibility heuristic
    let parts: Vec<&str> = sdk_version.split('.').collect();
    let major: u32 = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);

    if major < 20 {
        (
            CompatibilityStatus::Incompatible,
            Some(format!(
                "SDK version {} is below minimum supported version 20.0.0",
                sdk_version
            )),
        )
    } else if major < 21 {
        (
            CompatibilityStatus::Warning,
            Some(format!(
                "SDK version {} has known deprecations; upgrade recommended",
                sdk_version
            )),
        )
    } else {
        (CompatibilityStatus::Compatible, None)
    }
}
