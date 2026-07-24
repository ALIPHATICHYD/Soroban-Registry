//! POST /api/v1/contracts/{id}/report — contract issue reporting (issue #873)

use axum::{
    extract::{ConnectInfo, Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContractReportRequest {
    /// Category of the report
    #[schema(example = "security")]
    pub r#type: ReportType,
    /// Human-readable description of the problem
    #[schema(example = "Potential reentrancy issue in withdraw function")]
    pub description: String,
    /// Optional contact address so maintainers can follow up
    pub contact_info: Option<String>,
    /// When true the reporter identity is not stored
    #[serde(default)]
    pub anonymous: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, sqlx::Type, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "report_type", rename_all = "snake_case")]
pub enum ReportType {
    Security,
    Abuse,
    Invalid,
    Deprecated,
    Other,
}

impl std::fmt::Display for ReportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Security => "security",
            Self::Abuse => "abuse",
            Self::Invalid => "invalid",
            Self::Deprecated => "deprecated",
            Self::Other => "other",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContractReportResponse {
    pub success: bool,
    pub report_id: String,
    pub status: ReportStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Submitted,
    Reviewing,
    Resolved,
}

// ── Rate limiting constants ───────────────────────────────────────────────────

const REPORT_RATE_LIMIT_PER_DAY: u32 = 10;
const REPORT_RATE_WINDOW_SECS: u64 = 86_400; // 24 hours

// ── Handlers ─────────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/v1/contracts/{id}/report",
    params(
        ("id" = String, Path, description = "Contract identifier (UUID or string ID)")
    ),
    request_body = ContractReportRequest,
    responses(
        (status = 201, description = "Report submitted successfully", body = ContractReportResponse),
        (status = 400, description = "Invalid request body"),
        (status = 404, description = "Contract not found"),
        (status = 429, description = "Rate limit exceeded — 10 reports per IP per day"),
    ),
    tag = "Contracts"
)]
pub async fn report_contract(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(payload): Json<ContractReportRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate description length
    if payload.description.trim().is_empty() {
        return Err(ApiError::bad_request(
            "MISSING_DESCRIPTION",
            "description must not be empty",
        ));
    }
    if payload.description.len() > 2_000 {
        return Err(ApiError::bad_request(
            "DESCRIPTION_TOO_LONG",
            "description must be 2000 characters or fewer",
        ));
    }

    // Resolve contract
    let contract_uuid = resolve_contract_id(&state, &id).await?;

    // IP-based rate limit: 10 per day
    let ip_str = addr.ip().to_string();
    enforce_rate_limit(
        &state,
        &ip_str,
        REPORT_RATE_LIMIT_PER_DAY,
        REPORT_RATE_WINDOW_SECS,
    )
    .await?;

    // Persist the report
    let report_id = Uuid::new_v4();
    let contact = if payload.anonymous {
        None
    } else {
        payload.contact_info.as_deref()
    };
    let type_str = payload.r#type.to_string();

    sqlx::query(
        r#"
        INSERT INTO contract_reports
            (id, contract_id, report_type, description, contact_info, anonymous, status, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, 'submitted', $7)
        "#,
    )
    .bind(report_id)
    .bind(contract_uuid)
    .bind(&type_str)
    .bind(&payload.description)
    .bind(contact)
    .bind(payload.anonymous)
    .bind(Utc::now())
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to store report: {e}")))?;

    let response = ContractReportResponse {
        success: true,
        report_id: format!("rep_{}", report_id.as_simple()),
        status: ReportStatus::Submitted,
        message: "Report received. Thank you for helping keep the registry safe.".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/v1/contracts/{id}/report/{report_id}/status
#[utoipa::path(
    get,
    path = "/api/v1/contracts/{id}/report/{report_id}/status",
    params(
        ("id" = String, Path, description = "Contract identifier"),
        ("report_id" = String, Path, description = "Report ID (rep_... format)")
    ),
    responses(
        (status = 200, description = "Report status"),
        (status = 404, description = "Report not found"),
    ),
    tag = "Contracts"
)]
pub async fn get_report_status(
    State(state): State<AppState>,
    Path((id, report_id_param)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    // strip "rep_" prefix if present
    let raw_id = report_id_param
        .strip_prefix("rep_")
        .unwrap_or(&report_id_param);

    let report_uuid = Uuid::parse_str(raw_id).map_err(|_| {
        ApiError::bad_request(
            "INVALID_REPORT_ID",
            "report_id must be a valid UUID (rep_...)",
        )
    })?;

    let contract_uuid = resolve_contract_id(&state, &id).await?;

    let row: Option<(String, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT status, created_at FROM contract_reports WHERE id = $1 AND contract_id = $2",
    )
    .bind(report_uuid)
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("db error: {e}")))?;

    let (status_str, created_at) = row.ok_or_else(|| {
        ApiError::not_found("REPORT_NOT_FOUND", "no report found with the given id")
    })?;

    Ok(Json(serde_json::json!({
        "reportId": format!("rep_{}", raw_id),
        "status": status_str,
        "createdAt": created_at,
    })))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Resolve a contract by UUID string or human-readable contract_id string.
async fn resolve_contract_id(state: &AppState, id: &str) -> ApiResult<Uuid> {
    if let Ok(uuid) = Uuid::parse_str(id) {
        let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM contracts WHERE id = $1")
            .bind(uuid)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("db error: {e}")))?;
        return exists.ok_or_else(|| {
            ApiError::not_found("CONTRACT_NOT_FOUND", format!("contract {} not found", id))
        });
    }

    let uuid: Option<Uuid> = sqlx::query_scalar("SELECT id FROM contracts WHERE contract_id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("db error: {e}")))?;

    uuid.ok_or_else(|| {
        ApiError::not_found("CONTRACT_NOT_FOUND", format!("contract {} not found", id))
    })
}

/// Cache-backed per-IP rate limit for the report endpoint.
/// Stores a hit counter keyed to `report_rl:<ip>:<day>` with a 24-hour TTL.
async fn enforce_rate_limit(
    state: &AppState,
    ip: &str,
    limit: u32,
    window_secs: u64,
) -> ApiResult<()> {
    use std::time::Duration;

    let day_bucket = Utc::now().timestamp() / window_secs as i64;
    let cache_key = format!("{}:{}", ip, day_bucket);

    let (cached, _hit) = state.cache.get("report_rl", &cache_key).await;
    let count: u32 = cached.as_deref().and_then(|s| s.parse().ok()).unwrap_or(0);

    if count >= limit {
        return Err(ApiError::new(
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMITED",
            format!(
                "You may submit at most {} reports per day. Try again later.",
                limit
            ),
        ));
    }

    state
        .cache
        .put(
            "report_rl",
            &cache_key,
            (count + 1).to_string(),
            Some(Duration::from_secs(window_secs + 60)),
        )
        .await;

    Ok(())
}
