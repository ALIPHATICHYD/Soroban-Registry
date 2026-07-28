//! Usage metering — append-only event ledger + summary endpoint.
//!
//! Metering is gated by license possession: callers must POST the
//! license JWT they want to meter against. We verify signature + DB
//! status before writing an event, so a revoked license cannot rack up
//! metered usage.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{ApiError, ApiResult},
    marketplace::{
        license::LicenseSigner,
        models::{LicenseRecord, RecordUsageRequest, UsageSummary},
    },
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct UsageWindow {
    /// ISO-8601 window start; defaults to 30 days ago.
    #[serde(default)]
    pub since: Option<DateTime<Utc>>,
}

/// Body for POST .../usage — the license JWT is the auth, not a
/// publisher Bearer token. Whoever holds the license token can meter
/// against it; the server cross-checks `claims.jti` against the URL.
#[derive(Debug, Deserialize)]
pub struct RecordUsageBody {
    pub token: String,
    #[serde(flatten)]
    pub event: RecordUsageRequest,
}

/// POST /api/marketplace/licenses/{jti}/usage
pub async fn record_usage(
    State(state): State<AppState>,
    Path(jti): Path<Uuid>,
    Json(body): Json<RecordUsageBody>,
) -> ApiResult<(StatusCode, Json<UsageSummary>)> {
    // Verify the license token before touching the DB so a forged/expired
    // token bails out fast.
    let signer = license_handlers_signer()?;
    let claims = signer
        .verify(&body.token)
        .map_err(|e| ApiError::unauthorized(format!("license token invalid: {e}")))?;
    if claims.jti != jti {
        return Err(ApiError::unauthorized(
            "license token does not match path jti",
        ));
    }

    let license = sqlx::query_as::<_, LicenseRecord>(
        r#"SELECT id, jti, contract_id, plan_id, owner_id, issued_at, expires_at,
                  revoked_at, status, metadata, created_at
           FROM contract_licenses WHERE jti = $1"#,
    )
    .bind(jti)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("license_not_found", "license not found"))?;

    if license.status != "active" {
        return Err(ApiError::forbidden(format!(
            "license is {} and cannot record usage",
            license.status
        )));
    }
    if let Some(exp) = license.expires_at {
        if exp <= Utc::now() {
            return Err(ApiError::forbidden("license has expired"));
        }
    }

    if body.event.call_count <= 0 {
        return Err(ApiError::bad_request_msg("call_count must be positive"));
    }

    let metadata = body.event.metadata.unwrap_or_else(|| serde_json::json!({}));

    sqlx::query(
        r#"
        INSERT INTO contract_usage_events (license_id, contract_id, call_count, metadata)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(license.id)
    .bind(license.contract_id)
    .bind(body.event.call_count)
    .bind(metadata)
    .execute(&state.db)
    .await?;

    let summary = summarize(&state, &license, None).await?;
    Ok((StatusCode::CREATED, Json(summary)))
}

/// GET /api/marketplace/licenses/{jti}/usage?since=<rfc3339>
pub async fn get_usage(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(jti): Path<Uuid>,
    Query(window): Query<UsageWindow>,
) -> ApiResult<Json<UsageSummary>> {
    let license = sqlx::query_as::<_, LicenseRecord>(
        r#"SELECT id, jti, contract_id, plan_id, owner_id, issued_at, expires_at,
                  revoked_at, status, metadata, created_at
           FROM contract_licenses WHERE jti = $1"#,
    )
    .bind(jti)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("license_not_found", "license not found"))?;

    if license.owner_id != user.publisher_id {
        // Allow the contract owner to inspect too.
        let owns_contract: Option<Uuid> = sqlx::query_scalar(
            "SELECT publisher_id FROM contracts WHERE id = $1 AND publisher_id = $2",
        )
        .bind(license.contract_id)
        .bind(user.publisher_id)
        .fetch_optional(&state.db)
        .await?;
        if owns_contract.is_none() {
            return Err(ApiError::forbidden(
                "not authorized to view usage for this license",
            ));
        }
    }

    let summary = summarize(&state, &license, window.since).await?;
    Ok(Json(summary))
}

async fn summarize(
    state: &AppState,
    license: &LicenseRecord,
    since: Option<DateTime<Utc>>,
) -> ApiResult<UsageSummary> {
    let now = Utc::now();
    let period_start = since.unwrap_or_else(|| now - Duration::days(30));

    let row: (Option<i64>, i64) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(call_count), 0)::BIGINT, COUNT(*)::BIGINT
        FROM contract_usage_events
        WHERE license_id = $1 AND ts >= $2
        "#,
    )
    .bind(license.id)
    .bind(period_start)
    .fetch_one(&state.db)
    .await?;

    let total_calls = row.0.unwrap_or(0);
    let event_count = row.1;

    // Pull plan quota for the response (informational). The column is
    // nullable (NULL = unlimited), so the scalar type is Option<i64> and
    // fetch_optional yields Option<Option<i64>> — flatten to one layer.
    let quota: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT call_quota FROM contract_pricing_plans WHERE id = $1",
    )
    .bind(license.plan_id)
    .fetch_optional(&state.db)
    .await?
    .flatten();

    let quota_exceeded = quota.map(|q| total_calls > q).unwrap_or(false);

    Ok(UsageSummary {
        license_id: license.id,
        period_start,
        period_end: now,
        total_calls,
        event_count,
        call_quota: quota,
        quota_exceeded,
    })
}

fn license_handlers_signer() -> ApiResult<LicenseSigner> {
    LicenseSigner::from_env().map_err(|e| {
        ApiError::service_unavailable_with(
            "license_signing_unavailable",
            format!("marketplace license signing key is not configured: {e}"),
        )
    })
}
