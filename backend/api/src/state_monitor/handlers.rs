use crate::validation::extractors::ValidatedJson;
use crate::{
    error::ApiError,
    state::AppState,
    state_monitor::{
        point_in_time::{self, Anchor, StateDiff, StateSnapshot},
        AnomalyInfo, StateChangeEntry,
    },
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct StateHistoryResponse {
    pub contract_id: String,
    pub changes: Vec<StateChangeEntry>,
    pub total: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnomaliesResponse {
    pub anomalies: Vec<AnomalyInfo>,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct StateHistoryQuery {
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AnomaliesQuery {
    pub severity: Option<String>,
    pub limit: Option<i32>,
}

/// Get state change history for a contract
pub async fn get_state_history_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<StateHistoryQuery>,
) -> Result<Json<StateHistoryResponse>, ApiError> {
    let monitor = state.state_monitor.as_ref().ok_or_else(|| {
        ApiError::service_unavailable_with(
            "STATE_MONITOR_DISABLED",
            "State monitor service is not enabled",
        )
    })?;

    let limit = params.limit.unwrap_or(50).clamp(1, 1000);

    let changes = monitor
        .get_state_history(&contract_id, limit)
        .await
        .map_err(|e| ApiError::internal_error("STATE_HISTORY_ERROR", e.to_string()))?;

    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM contract_state_history WHERE contract_id = $1")
            .bind(
                Uuid::parse_str(&contract_id)
                    .map_err(|_| ApiError::bad_request_with("INVALID_ID", "Invalid UUID"))?,
            )
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal_error("DB_ERROR", e.to_string()))?;

    Ok(Json(StateHistoryResponse {
        contract_id,
        changes,
        total,
    }))
}

/// Get anomalies for a specific contract or all contracts
pub async fn get_anomalies_handler(
    State(state): State<AppState>,
    Query(params): Query<AnomaliesQuery>,
) -> Result<Json<AnomaliesResponse>, ApiError> {
    let monitor = state.state_monitor.as_ref().ok_or_else(|| {
        ApiError::service_unavailable_with(
            "STATE_MONITOR_DISABLED",
            "State monitor service is not enabled",
        )
    })?;

    let limit = params.limit.unwrap_or(50).clamp(1, 1000);

    let anomalies = monitor
        .get_anomalies(None, params.severity.as_deref(), limit)
        .await
        .map_err(|e| ApiError::internal_error("ANOMALY_ERROR", e.to_string()))?;

    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM state_anomalies WHERE is_resolved = FALSE")
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal_error("DB_ERROR", e.to_string()))?;

    Ok(Json(AnomaliesResponse { anomalies, total }))
}

/// Get anomalies for a specific contract
pub async fn get_contract_anomalies_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<AnomaliesQuery>,
) -> Result<Json<AnomaliesResponse>, ApiError> {
    let monitor = state.state_monitor.as_ref().ok_or_else(|| {
        ApiError::service_unavailable_with(
            "STATE_MONITOR_DISABLED",
            "State monitor service is not enabled",
        )
    })?;

    let limit = params.limit.unwrap_or(50).clamp(1, 1000);

    let anomalies = monitor
        .get_anomalies(Some(&contract_id), params.severity.as_deref(), limit)
        .await
        .map_err(|e| ApiError::internal_error("ANOMALY_ERROR", e.to_string()))?;

    let contract_uuid = Uuid::parse_str(&contract_id)
        .map_err(|_| ApiError::bad_request_with("INVALID_ID", "Invalid UUID"))?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM state_anomalies 
         WHERE contract_id = $1 AND is_resolved = FALSE",
    )
    .bind(contract_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("DB_ERROR", e.to_string()))?;

    Ok(Json(AnomaliesResponse { anomalies, total }))
}

/// Resolve an anomaly
pub async fn resolve_anomaly_handler(
    State(state): State<AppState>,
    Path(anomaly_id): Path<String>,
    ValidatedJson(payload): ValidatedJson<ResolveAnomalyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let monitor = state.state_monitor.as_ref().ok_or_else(|| {
        ApiError::service_unavailable_with(
            "STATE_MONITOR_DISABLED",
            "State monitor service is not enabled",
        )
    })?;

    monitor
        .resolve_anomaly(&anomaly_id, payload.resolution_notes.as_deref())
        .await
        .map_err(|e| ApiError::internal_error("RESOLVE_ERROR", e.to_string()))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Anomaly resolved successfully",
        "anomaly_id": anomaly_id,
    })))
}

#[derive(Debug, Deserialize)]
pub struct ResolveAnomalyRequest {
    pub resolution_notes: Option<String>,
}

// ── Point-in-time + diff endpoints (state analysis) ─────────────────

#[derive(Debug, Deserialize)]
pub struct StateAtQuery {
    pub ledger: Option<i64>,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct StateDiffQuery {
    pub from_ledger: Option<i64>,
    pub to_ledger: Option<i64>,
    pub from_timestamp: Option<DateTime<Utc>>,
    pub to_timestamp: Option<DateTime<Utc>>,
}

/// GET /api/contracts/:id/state-at?ledger=N | ?timestamp=T
///
/// Returns the derived state of every key for `contract_id` as of the
/// requested point. Exactly one of `ledger` or `timestamp` is
/// required; supplying both is a 400.
pub async fn get_state_at_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<StateAtQuery>,
) -> Result<Json<StateSnapshot>, ApiError> {
    let contract_uuid = parse_contract_id(&contract_id)?;
    let anchor = parse_anchor(params.ledger, params.timestamp)?;

    let snapshot = point_in_time::snapshot_at(&state.db, contract_uuid, anchor)
        .await
        .map_err(|e| ApiError::internal_error("STATE_QUERY_FAILED", e.to_string()))?;

    Ok(Json(snapshot))
}

/// GET /api/contracts/:id/state-diff?from_ledger=N&to_ledger=M
/// (or analogous `_timestamp` variants on either side).
///
/// Returns the per-key {added, removed, changed} diff between two
/// derived snapshots. Mixing ledger/timestamp across sides is allowed
/// but discouraged.
pub async fn get_state_diff_handler(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<StateDiffQuery>,
) -> Result<Json<StateDiff>, ApiError> {
    let contract_uuid = parse_contract_id(&contract_id)?;
    let from = parse_anchor(params.from_ledger, params.from_timestamp)
        .map_err(|e| prefix_param_error(e, "from_"))?;
    let to = parse_anchor(params.to_ledger, params.to_timestamp)
        .map_err(|e| prefix_param_error(e, "to_"))?;

    let diff = point_in_time::diff(&state.db, contract_uuid, from, to)
        .await
        .map_err(|e| ApiError::internal_error("STATE_DIFF_FAILED", e.to_string()))?;

    Ok(Json(diff))
}

fn parse_contract_id(s: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(s).map_err(|_| ApiError::bad_request_msg("contract_id must be a valid UUID"))
}

/// Validate exactly-one-of(ledger, timestamp). Returns 400 with a
/// clear message otherwise.
fn parse_anchor(ledger: Option<i64>, timestamp: Option<DateTime<Utc>>) -> Result<Anchor, ApiError> {
    match (ledger, timestamp) {
        (Some(_), Some(_)) => Err(ApiError::bad_request_msg(
            "supply exactly one of `ledger` or `timestamp`, not both",
        )),
        (None, None) => Err(ApiError::bad_request_msg(
            "supply one of `ledger` or `timestamp`",
        )),
        (Some(n), None) if n < 0 => Err(ApiError::bad_request_msg("`ledger` must be non-negative")),
        (Some(n), None) => Ok(Anchor::Ledger(n)),
        (None, Some(t)) => Ok(Anchor::Timestamp(t)),
    }
}

fn prefix_param_error(err: ApiError, prefix: &str) -> ApiError {
    // The diff endpoint takes two anchors with `from_`/`to_` prefixes
    // — rewrite the inner error so the user sees which side is wrong.
    let msg = err
        .to_string()
        .replace("`ledger`", &format!("`{prefix}ledger`"))
        .replace("`timestamp`", &format!("`{prefix}timestamp`"));
    ApiError::bad_request_msg(msg)
}
