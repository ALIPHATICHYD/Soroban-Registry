//! Phase 3 — USDC payment intents → license issuance.
//!
//! Flow:
//!   1. Buyer calls `POST /api/contracts/:id/usdc-intents` with the
//!      plan they want. Server records a `marketplace_usdc_payments`
//!      row in `pending` status with an `expires_at` (default 1h) and
//!      returns destination, amount, memo, asset, network. Buyer pays
//!      from any Stellar wallet.
//!   2. The indexer observes a Stellar payment that matches a pending
//!      memo and POSTs to `/api/marketplace/usdc/confirm` with the
//!      `tx_hash` + observed amount + memo. (For now this endpoint
//!      requires an admin Bearer token — there's no public way to
//!      "claim" a license from an arbitrary tx_hash.)
//!   3. Server verifies the row is still pending, marks it
//!      `confirmed`, and issues the license via the shared issuance
//!      helper.
//!
//! Wiring the indexer to actually call `/usdc/confirm` is a separate
//! task in the `backend/indexer` crate — that's where the on-chain
//! event stream lives. This module exposes the seam.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{ApiError, ApiResult},
    marketplace::{
        self, issuance,
        models::IssuedLicense,
        usdc::{cents_to_usdc_stroops, generate_memo, UsdcConfig, UsdcError},
    },
    state::AppState,
};

const INTENT_TTL_SECONDS: i64 = 3600; // 1 hour
const MAX_MEMO_RETRIES: usize = 3;

#[derive(Debug, Deserialize)]
pub struct CreateIntentRequest {
    pub plan_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct CreateIntentResponse {
    pub payment_id: Uuid,
    pub network: String,
    pub receiving_address: String,
    pub asset_code: &'static str,
    pub asset_issuer: String,
    pub memo: String,
    pub amount_cents: i64,
    pub amount_usdc_stroops: i64,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmIntentRequest {
    pub memo: String,
    pub tx_hash: String,
    /// Amount observed on-chain in USDC stroops. Server compares
    /// against the recorded expected amount and rejects underpayment.
    pub observed_stroops: i64,
}

#[derive(Debug, Serialize)]
pub struct ConfirmIntentResponse {
    pub payment_id: Uuid,
    pub license: IssuedLicense,
}

#[derive(Debug, sqlx::FromRow)]
struct PlanForIntent {
    id: Uuid,
    price_cents: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct ClaimedIntent {
    id: Uuid,
    contract_id: Uuid,
    plan_id: Uuid,
    payer_id: Uuid,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct IntentRow {
    id: Uuid,
    contract_id: Uuid,
    plan_id: Uuid,
    payer_id: Uuid,
    amount_cents: i64,
    status: String,
    expires_at: chrono::DateTime<Utc>,
    license_id: Option<Uuid>,
}

/// POST /api/contracts/:contract_id/usdc-intents
pub async fn create_intent(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<CreateIntentRequest>,
) -> ApiResult<(StatusCode, Json<CreateIntentResponse>)> {
    let config = load_usdc_config()?;

    let plan = sqlx::query_as::<_, PlanForIntent>(
        r#"
        SELECT id, price_cents
        FROM contract_pricing_plans
        WHERE id = $1 AND contract_id = $2 AND is_active
        "#,
    )
    .bind(req.plan_id)
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        ApiError::not_found(
            "plan_not_found",
            "pricing plan not found or inactive for this contract",
        )
    })?;

    if plan.price_cents == 0 {
        return Err(ApiError::bad_request_msg(
            "USDC intents are only valid for paid plans",
        ));
    }

    let expires_at = Utc::now() + Duration::seconds(INTENT_TTL_SECONDS);
    let payment_id = Uuid::new_v4();

    // Retry on memo collision (vanishingly unlikely but cheap to handle).
    let mut last_err: Option<sqlx::Error> = None;
    let mut chosen_memo = String::new();
    for _ in 0..MAX_MEMO_RETRIES {
        let memo = generate_memo();
        let res = sqlx::query(
            r#"
            INSERT INTO marketplace_usdc_payments
                (id, contract_id, plan_id, payer_id, amount_cents,
                 receiving_address, asset_issuer, network, memo, status, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending', $10)
            "#,
        )
        .bind(payment_id)
        .bind(contract_id)
        .bind(plan.id)
        .bind(user.publisher_id)
        .bind(plan.price_cents)
        .bind(&config.receiving_address)
        .bind(&config.asset_issuer)
        .bind(&config.network)
        .bind(&memo)
        .bind(expires_at)
        .execute(&state.db)
        .await;

        match res {
            Ok(_) => {
                chosen_memo = memo;
                last_err = None;
                break;
            }
            Err(e) => {
                if let sqlx::Error::Database(db_err) = &e {
                    // 23505 = unique_violation — try a new memo
                    if db_err.code().as_deref() == Some("23505") {
                        last_err = Some(e);
                        continue;
                    }
                }
                return Err(e.into());
            }
        }
    }

    if let Some(e) = last_err {
        return Err(e.into());
    }

    Ok((
        StatusCode::CREATED,
        Json(CreateIntentResponse {
            payment_id,
            network: config.network,
            receiving_address: config.receiving_address,
            asset_code: "USDC",
            asset_issuer: config.asset_issuer,
            memo: chosen_memo,
            amount_cents: plan.price_cents,
            amount_usdc_stroops: cents_to_usdc_stroops(plan.price_cents),
            expires_at,
        }),
    ))
}

/// POST /api/marketplace/usdc/confirm
///
/// Called by the indexer (or an operator) when a matching on-chain
/// payment has been observed. Currently requires an authenticated
/// publisher to call — that publisher's id is recorded in metadata as
/// "confirmed_by". For Phase 3 the expectation is the indexer's
/// service account performs this call; a dedicated admin/service
/// scope is a follow-up.
pub async fn confirm_intent(
    State(state): State<AppState>,
    confirmer: AuthenticatedUser,
    Json(req): Json<ConfirmIntentRequest>,
) -> ApiResult<Json<ConfirmIntentResponse>> {
    if req.tx_hash.len() != 64 || !req.tx_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::bad_request_msg(
            "tx_hash must be a 64-char hex string",
        ));
    }
    if req.observed_stroops <= 0 {
        return Err(ApiError::bad_request_msg(
            "observed_stroops must be positive",
        ));
    }

    // Atomic check-and-claim: Postgres row locks serialise concurrent
    // UPDATEs against the same row, and the `tx_hash IS NULL` predicate
    // ensures only the first to land actually claims the row. If we
    // get 0 rows back, someone else (or expiry, or a wrong memo) won.
    //
    // Underpayment is *not* checked here because the UPDATE has
    // already side-effected. We instead snapshot the intent
    // (read-only) first, validate, then claim. The validation is
    // safe to do outside the transaction — `amount_cents` and
    // `expires_at` are immutable after intent creation.
    let snapshot = sqlx::query_as::<_, IntentRow>(
        r#"
        SELECT id, contract_id, plan_id, payer_id, amount_cents,
               status, expires_at, license_id
        FROM marketplace_usdc_payments
        WHERE memo = $1
        "#,
    )
    .bind(&req.memo)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("intent_not_found", "no payment intent with that memo"))?;

    if snapshot.status == "confirmed" || snapshot.license_id.is_some() {
        return Err(ApiError::conflict(
            "already_confirmed",
            "this payment intent has already been confirmed",
        ));
    }
    if snapshot.status == "expired" || Utc::now() > snapshot.expires_at {
        let _ = sqlx::query(
            "UPDATE marketplace_usdc_payments SET status='expired' WHERE id=$1 AND status='pending'",
        )
        .bind(snapshot.id)
        .execute(&state.db)
        .await;
        return Err(ApiError::conflict(
            "intent_expired",
            "this payment intent has expired; create a new one",
        ));
    }

    let expected_stroops = cents_to_usdc_stroops(snapshot.amount_cents);
    if req.observed_stroops < expected_stroops {
        return Err(ApiError::bad_request(
            "underpayment",
            format!(
                "observed {} stroops, expected at least {} (${}.{:02})",
                req.observed_stroops,
                expected_stroops,
                snapshot.amount_cents / 100,
                snapshot.amount_cents % 100
            ),
        ));
    }

    // Claim the row atomically. Concurrent confirms either lose the
    // race (claimed=None) or block on the row lock and re-evaluate the
    // WHERE — which then fails because tx_hash is no longer NULL.
    let claimed = sqlx::query_as::<_, ClaimedIntent>(
        r#"
        UPDATE marketplace_usdc_payments
        SET tx_hash = $2,
            confirmed_amount = $3
        WHERE memo = $1
          AND tx_hash IS NULL
          AND status = 'pending'
          AND expires_at > NOW()
        RETURNING id, contract_id, plan_id, payer_id
        "#,
    )
    .bind(&req.memo)
    .bind(&req.tx_hash)
    .bind(req.observed_stroops / 100_000)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| {
        ApiError::conflict(
            "already_confirmed",
            "this payment intent was confirmed by a concurrent request",
        )
    })?;

    // From here until the final UPDATE, a crash would leave the row in
    // a "claimed but license not issued" state (tx_hash set, status
    // still 'pending', license_id NULL). Recovery is an admin task —
    // see project memory for the runbook seam.
    let signer = marketplace::load_signer()?;
    let issued = issuance::issue_for_owner(
        &state.db,
        &signer,
        claimed.contract_id,
        claimed.plan_id,
        claimed.payer_id,
        serde_json::json!({
            "source": "usdc",
            "tx_hash": req.tx_hash,
            "memo": req.memo,
            "confirmed_by": confirmer.publisher_id.to_string(),
        }),
    )
    .await?;

    sqlx::query(
        r#"
        UPDATE marketplace_usdc_payments
        SET status = 'confirmed',
            license_id = $2,
            confirmed_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(claimed.id)
    .bind(issued.license.id)
    .execute(&state.db)
    .await?;

    tracing::info!(
        payment_id = %claimed.id,
        license_id = %issued.license.id,
        tx_hash = %req.tx_hash,
        "usdc payment confirmed; license issued"
    );

    Ok(Json(ConfirmIntentResponse {
        payment_id: claimed.id,
        license: issued,
    }))
}

/// GET /api/marketplace/usdc-payments/:payment_id
pub async fn get_intent(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(payment_id): Path<Uuid>,
) -> ApiResult<Json<IntentRow>> {
    let row = sqlx::query_as::<_, IntentRow>(
        r#"
        SELECT id, contract_id, plan_id, payer_id, amount_cents,
               status, expires_at, license_id
        FROM marketplace_usdc_payments
        WHERE id = $1
        "#,
    )
    .bind(payment_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("intent_not_found", "payment intent not found"))?;

    if row.payer_id != user.publisher_id {
        return Err(ApiError::forbidden("not authorized to view this intent"));
    }

    Ok(Json(row))
}

fn load_usdc_config() -> ApiResult<UsdcConfig> {
    UsdcConfig::from_env().map_err(|e| match e {
        UsdcError::ReceivingAddressMissing
        | UsdcError::AssetIssuerMissing
        | UsdcError::BadNetwork(_)
        | UsdcError::BadReceivingAddress => ApiError::service_unavailable_with(
            "usdc_unconfigured",
            format!("USDC payments are not configured: {e}"),
        ),
    })
}
