//! Internal license-issuance helper.
//!
//! Both the authed `POST /api/contracts/:id/licenses` endpoint and the
//! payment webhook handlers (Phase 2 Stripe, Phase 3 USDC) need to mint
//! a license. This module is the single place where:
//!
//!   1. The pricing plan is re-validated against the contract.
//!   2. Expiry is derived from `billing_period`.
//!   3. The DB row is inserted.
//!   4. The Ed25519 JWT is signed.
//!
//! Keep payment-specific logic OUT of here — callers stamp their own
//! provenance in `metadata` (e.g. `{"source":"stripe","session_id":...}`).

use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::marketplace::{
    license::{LicenseClaims, LicenseSigner},
    models::{IssuedLicense, LicenseRecord},
};

#[derive(Debug, sqlx::FromRow)]
struct PlanForIssue {
    id: Uuid,
    name: String,
    billing_period: String,
    call_quota: Option<i64>,
}

/// Issue a license to `owner_id` for `plan_id` on `contract_id`.
///
/// Returns the DB record, the signed JWT, and the public key (so HTTP
/// callers can return it verbatim — see [`crate::marketplace::license_handlers`]).
pub async fn issue_for_owner(
    db: &PgPool,
    signer: &LicenseSigner,
    contract_id: Uuid,
    plan_id: Uuid,
    owner_id: Uuid,
    metadata: serde_json::Value,
) -> ApiResult<IssuedLicense> {
    let plan = sqlx::query_as::<_, PlanForIssue>(
        r#"
        SELECT id, name, billing_period, call_quota
        FROM contract_pricing_plans
        WHERE id = $1 AND contract_id = $2 AND is_active
        "#,
    )
    .bind(plan_id)
    .bind(contract_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        ApiError::not_found(
            "plan_not_found",
            "pricing plan not found or inactive for this contract",
        )
    })?;

    let now = Utc::now();
    let expires_at = match plan.billing_period.as_str() {
        "monthly" => Some(now + Duration::days(30)),
        _ => None,
    };

    let jti = Uuid::new_v4();
    let record = sqlx::query_as::<_, LicenseRecord>(
        r#"
        INSERT INTO contract_licenses
            (jti, contract_id, plan_id, owner_id, issued_at, expires_at, status, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, 'active', $7)
        RETURNING id, jti, contract_id, plan_id, owner_id, issued_at, expires_at,
                  revoked_at, status, metadata, created_at
        "#,
    )
    .bind(jti)
    .bind(contract_id)
    .bind(plan.id)
    .bind(owner_id)
    .bind(now)
    .bind(expires_at)
    .bind(&metadata)
    .fetch_one(db)
    .await?;

    let claims = LicenseClaims {
        jti: record.jti,
        sub: record.owner_id,
        aud: record.contract_id,
        plan_id: record.plan_id,
        plan_name: plan.name,
        iat: now.timestamp(),
        exp: expires_at.map(|t| t.timestamp()),
        quota: plan.call_quota,
    };
    let token = signer
        .sign(&claims)
        .map_err(|e| ApiError::internal_error("license_sign_failed", e.to_string()))?;

    Ok(IssuedLicense {
        license: record,
        token,
        public_key_b64: signer.public_key_b64(),
    })
}
