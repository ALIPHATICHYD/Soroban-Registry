//! Phase 2 — Stripe checkout + webhook → license issuance.
//!
//! Two endpoints:
//!
//!   * `POST /api/contracts/:contract_id/checkout` (authed)
//!       Creates a Stripe Checkout Session for the given plan and
//!       records a row in `marketplace_stripe_payments` so the webhook
//!       can find it on completion.
//!
//!   * `POST /api/marketplace/stripe/webhook` (public, HMAC-verified)
//!       Receives Stripe webhook events. Verifies the `Stripe-Signature`
//!       header, dedupes by event id (Stripe retries on 5xx), and
//!       issues the license on `checkout.session.completed`.
//!
//! The webhook handler accepts the raw request body bytes so the HMAC
//! is computed over the exact payload Stripe signed; do NOT route this
//! through axum's `Json` extractor — that re-serialises and breaks the
//! signature.

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{ApiError, ApiResult},
    marketplace::{
        self, issuance,
        models::IssuedLicense,
        stripe::{
            verify_webhook_signature, CheckoutSession, CheckoutSessionParams,
            CheckoutSessionPayload, StripeClient, StripeError, WebhookEvent,
        },
    },
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateCheckoutRequest {
    pub plan_id: Uuid,
    pub success_url: String,
    pub cancel_url: String,
    #[serde(default)]
    pub customer_email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateCheckoutResponse {
    pub payment_id: Uuid,
    pub checkout_url: String,
    pub session_id: String,
}

#[derive(Debug, sqlx::FromRow)]
struct PlanForCheckout {
    id: Uuid,
    name: String,
    price_cents: i64,
    currency: String,
    billing_period: String,
}

#[derive(Debug, sqlx::FromRow)]
struct PaymentRow {
    id: Uuid,
    contract_id: Uuid,
    plan_id: Uuid,
    payer_id: Uuid,
    license_id: Option<Uuid>,
}

/// POST /api/contracts/:contract_id/checkout
pub async fn create_checkout(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<CreateCheckoutRequest>,
) -> ApiResult<(StatusCode, Json<CreateCheckoutResponse>)> {
    let plan = sqlx::query_as::<_, PlanForCheckout>(
        r#"
        SELECT id, name, price_cents, currency, billing_period
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
            "checkout is only valid for paid plans; use POST /licenses for free plans",
        ));
    }

    let client = load_stripe_client()?;

    // Generate payment_id up-front so we can put it in Stripe metadata.
    // We only insert after Stripe confirms — if Stripe rejects, we
    // don't leave a dangling row; if the subsequent insert fails, we
    // leave an orphan Stripe session whose webhook will land on a
    // payment we don't recognise (logged + 400, Stripe will give up).
    let payment_id = Uuid::new_v4();
    let session: CheckoutSession = client
        .create_checkout_session(CheckoutSessionParams {
            payment_id: &payment_id.to_string(),
            contract_id: &contract_id.to_string(),
            plan_id: &plan.id.to_string(),
            payer_id: &user.publisher_id.to_string(),
            amount_cents: plan.price_cents,
            currency: &plan.currency,
            billing_period: &plan.billing_period,
            product_name: &format!("{} access", plan.name),
            success_url: &req.success_url,
            cancel_url: &req.cancel_url,
            customer_email: req.customer_email.as_deref(),
        })
        .await
        .map_err(map_stripe_error)?;

    let url = session.url.clone().ok_or_else(|| {
        ApiError::internal_error(
            "stripe_no_url",
            "Stripe returned a session without a redirect URL",
        )
    })?;

    sqlx::query(
        r#"
        INSERT INTO marketplace_stripe_payments
            (id, contract_id, plan_id, payer_id, stripe_checkout_session,
             stripe_payment_intent, stripe_customer,
             amount_cents, currency, status, checkout_url)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending', $10)
        "#,
    )
    .bind(payment_id)
    .bind(contract_id)
    .bind(plan.id)
    .bind(user.publisher_id)
    .bind(&session.id)
    .bind(&session.payment_intent)
    .bind(&session.customer)
    .bind(plan.price_cents)
    .bind(&plan.currency)
    .bind(&url)
    .execute(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(CreateCheckoutResponse {
            payment_id,
            checkout_url: url,
            session_id: session.id,
        }),
    ))
}

/// POST /api/marketplace/stripe/webhook
///
/// Receives raw request bytes so we can HMAC-verify against exactly
/// what Stripe signed. Returns 200 quickly (Stripe retries on 5xx),
/// 400 on signature mismatch, 503 if not configured.
pub async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<StatusCode> {
    let webhook_secret = std::env::var("MARKETPLACE_STRIPE_WEBHOOK_SECRET").map_err(|_| {
        ApiError::service_unavailable_with(
            "stripe_webhook_unconfigured",
            "MARKETPLACE_STRIPE_WEBHOOK_SECRET is not set",
        )
    })?;

    let sig_header = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            ApiError::bad_request("missing_signature", "Stripe-Signature header is required")
        })?;

    verify_webhook_signature(&body, sig_header, &webhook_secret, Utc::now()).map_err(|e| {
        // 400 (not 401) — matches Stripe's own docs guidance: signature
        // failures are client errors from Stripe's POV.
        ApiError::bad_request("invalid_signature", e.to_string())
    })?;

    let event: WebhookEvent = serde_json::from_slice(&body)
        .map_err(|e| ApiError::bad_request("malformed_event", e.to_string()))?;

    // Idempotency: if we've already processed this event id, return 200
    // without doing anything else.
    let inserted = sqlx::query(
        r#"
        INSERT INTO marketplace_stripe_webhook_events (event_id, event_type, payload)
        VALUES ($1, $2, $3)
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(&event.id)
    .bind(&event.event_type)
    .bind(serde_json::to_value(&event).unwrap_or(serde_json::Value::Null))
    .execute(&state.db)
    .await?
    .rows_affected();

    if inserted == 0 {
        tracing::info!(event_id = %event.id, "stripe webhook duplicate; skipping");
        return Ok(StatusCode::OK);
    }

    if event.event_type == "checkout.session.completed" {
        handle_checkout_completed(&state, &event).await?;
    }
    // All other event types are recorded but otherwise ignored for v1.

    Ok(StatusCode::OK)
}

async fn handle_checkout_completed(state: &AppState, event: &WebhookEvent) -> ApiResult<()> {
    let session: CheckoutSessionPayload = serde_json::from_value(event.data.object.clone())
        .map_err(|e| {
            ApiError::bad_request("malformed_session", format!("checkout session decode: {e}"))
        })?;

    // The metadata fields are the same ones we stamped in `create_checkout`.
    let payment_id_str = session
        .metadata
        .get("payment_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ApiError::bad_request(
                "missing_metadata",
                "checkout session missing payment_id metadata",
            )
        })?;
    let payment_id: Uuid = payment_id_str
        .parse()
        .map_err(|_| ApiError::bad_request("bad_metadata", "payment_id metadata is not a uuid"))?;

    // Lookup the pending payment row.
    let payment = sqlx::query_as::<_, PaymentRow>(
        r#"
        SELECT id, contract_id, plan_id, payer_id, license_id
        FROM marketplace_stripe_payments
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(payment_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("payment_not_found", "stripe payment not found"))?;

    // Defence in depth: if we already issued a license for this row,
    // we're done. Should be unreachable thanks to event-id idempotency
    // above, but Stripe occasionally sends "duplicate-ish" events for
    // the same session and the cost of a double-check is zero.
    if payment.license_id.is_some() {
        return Ok(());
    }

    let signer = marketplace::load_signer()?;
    let issued: IssuedLicense = issuance::issue_for_owner(
        &state.db,
        &signer,
        payment.contract_id,
        payment.plan_id,
        payment.payer_id,
        serde_json::json!({
            "source": "stripe",
            "stripe_session_id": session.id,
            "stripe_payment_intent": session.payment_intent,
            "stripe_event_id": event.id,
        }),
    )
    .await?;

    sqlx::query(
        r#"
        UPDATE marketplace_stripe_payments
        SET status = 'completed',
            license_id = $2,
            completed_at = NOW(),
            stripe_payment_intent = COALESCE($3, stripe_payment_intent),
            stripe_customer       = COALESCE($4, stripe_customer)
        WHERE id = $1
        "#,
    )
    .bind(payment.id)
    .bind(issued.license.id)
    .bind(&session.payment_intent)
    .bind(&session.customer)
    .execute(&state.db)
    .await?;

    sqlx::query("UPDATE marketplace_stripe_webhook_events SET payment_id = $2 WHERE event_id = $1")
        .bind(&event.id)
        .bind(payment.id)
        .execute(&state.db)
        .await?;

    tracing::info!(
        payment_id = %payment.id,
        license_id = %issued.license.id,
        stripe_session = %session.id,
        "stripe checkout completed; license issued"
    );

    Ok(())
}

fn load_stripe_client() -> ApiResult<StripeClient> {
    StripeClient::from_env().map_err(|e| {
        ApiError::service_unavailable_with(
            "stripe_unconfigured",
            format!("Stripe is not configured: {e}"),
        )
    })
}

fn map_stripe_error(e: StripeError) -> ApiError {
    match e {
        StripeError::SecretKeyMissing | StripeError::WebhookSecretMissing => {
            ApiError::service_unavailable_with("stripe_unconfigured", e.to_string())
        }
        StripeError::Api { status, body } if (400..500).contains(&status) => {
            ApiError::bad_request("stripe_rejected", format!("stripe {status}: {body}"))
        }
        _ => ApiError::internal_error("stripe_error", e.to_string()),
    }
}
