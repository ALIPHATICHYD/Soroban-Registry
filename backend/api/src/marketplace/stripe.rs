//! Thin Stripe client — Checkout Sessions and webhook verification.
//!
//! Why not `async-stripe`: we use a tiny slice of Stripe (one Checkout
//! Sessions create call + webhook HMAC verification). Pulling in the
//! full crate would multiply dependency surface for no benefit and
//! make security review harder.
//!
//! The Stripe secret key (`MARKETPLACE_STRIPE_SECRET_KEY`) and the
//! webhook signing secret (`MARKETPLACE_STRIPE_WEBHOOK_SECRET`) are
//! loaded from env per request — same lazy-load posture as the license
//! signing key, so an unconfigured deploy returns 503 instead of 500.
//!
//! All endpoints work against both `sk_test_…` and `sk_live_…` keys
//! transparently; this module never inspects the key prefix.

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

const STRIPE_API_BASE: &str = "https://api.stripe.com/v1";
/// Stripe rejects timestamps that drift more than 5 minutes from
/// `now()` in webhook signature checks; we match that tolerance.
const WEBHOOK_TIMESTAMP_TOLERANCE: i64 = 300;

#[derive(Debug, thiserror::Error)]
pub enum StripeError {
    #[error("MARKETPLACE_STRIPE_SECRET_KEY is not set")]
    SecretKeyMissing,
    #[error("MARKETPLACE_STRIPE_WEBHOOK_SECRET is not set")]
    WebhookSecretMissing,
    #[error("stripe http error: {0}")]
    Http(String),
    #[error("stripe returned {status}: {body}")]
    Api { status: u16, body: String },
    #[error("malformed stripe response: {0}")]
    Decode(String),
    #[error("webhook signature header missing or malformed")]
    BadSignatureHeader,
    #[error("webhook signature did not match payload")]
    SignatureMismatch,
    #[error("webhook timestamp outside tolerance window")]
    TimestampSkew,
}

pub struct StripeClient {
    secret_key: String,
    http: reqwest::Client,
}

impl StripeClient {
    pub fn from_env() -> Result<Self, StripeError> {
        let secret_key = std::env::var("MARKETPLACE_STRIPE_SECRET_KEY")
            .map_err(|_| StripeError::SecretKeyMissing)?;
        Ok(Self {
            secret_key,
            http: reqwest::Client::new(),
        })
    }

    /// Create a Checkout Session. `mode=subscription` if the plan is
    /// monthly, otherwise `payment` for one-time. We don't manage
    /// Stripe Products/Prices in this codebase — we use `price_data`
    /// inline so a single env-configured Stripe account works without
    /// extra console setup.
    pub async fn create_checkout_session(
        &self,
        params: CheckoutSessionParams<'_>,
    ) -> Result<CheckoutSession, StripeError> {
        let mode = match params.billing_period {
            "monthly" => "subscription",
            _ => "payment",
        };

        let mut form: Vec<(String, String)> = vec![
            ("mode".into(), mode.into()),
            ("success_url".into(), params.success_url.into()),
            ("cancel_url".into(), params.cancel_url.into()),
            ("line_items[0][quantity]".into(), "1".into()),
            (
                "line_items[0][price_data][currency]".into(),
                params.currency.to_lowercase(),
            ),
            (
                "line_items[0][price_data][unit_amount]".into(),
                params.amount_cents.to_string(),
            ),
            (
                "line_items[0][price_data][product_data][name]".into(),
                params.product_name.into(),
            ),
        ];
        if mode == "subscription" {
            form.push((
                "line_items[0][price_data][recurring][interval]".into(),
                "month".into(),
            ));
        }
        if let Some(email) = params.customer_email {
            form.push(("customer_email".into(), email.into()));
        }
        // Metadata that survives the round-trip via the webhook payload.
        form.push(("metadata[payment_id]".into(), params.payment_id.into()));
        form.push(("metadata[contract_id]".into(), params.contract_id.into()));
        form.push(("metadata[plan_id]".into(), params.plan_id.into()));
        form.push(("metadata[payer_id]".into(), params.payer_id.into()));

        let resp = self
            .http
            .post(format!("{STRIPE_API_BASE}/checkout/sessions"))
            .basic_auth(&self.secret_key, Some(""))
            .form(&form)
            .send()
            .await
            .map_err(|e| StripeError::Http(e.to_string()))?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| StripeError::Http(e.to_string()))?;

        if !(200..300).contains(&status) {
            return Err(StripeError::Api { status, body });
        }

        serde_json::from_str(&body).map_err(|e| StripeError::Decode(e.to_string()))
    }
}

#[derive(Debug)]
pub struct CheckoutSessionParams<'a> {
    pub payment_id: &'a str,
    pub contract_id: &'a str,
    pub plan_id: &'a str,
    pub payer_id: &'a str,
    pub amount_cents: i64,
    pub currency: &'a str,
    pub billing_period: &'a str,
    pub product_name: &'a str,
    pub success_url: &'a str,
    pub cancel_url: &'a str,
    pub customer_email: Option<&'a str>,
}

/// Subset of the Stripe Checkout Session resource we care about.
#[derive(Debug, Clone, Deserialize)]
pub struct CheckoutSession {
    pub id: String,
    pub url: Option<String>,
    pub payment_intent: Option<String>,
    pub customer: Option<String>,
}

// ── Webhooks ─────────────────────────────────────────────────────────

/// Verify a Stripe webhook signature header per the spec:
///   <https://stripe.com/docs/webhooks/signatures>
///
/// The `Stripe-Signature` header has the form:
///   `t=<ts>,v1=<hex>[,v1=<hex>...]`
///
/// We require:
///   * `|now - t| <= WEBHOOK_TIMESTAMP_TOLERANCE`
///   * At least one `v1=` digest matches HMAC-SHA256("<t>.<payload>", secret)
///   * Comparison is constant-time.
pub fn verify_webhook_signature(
    payload: &[u8],
    signature_header: &str,
    webhook_secret: &str,
    now: DateTime<Utc>,
) -> Result<(), StripeError> {
    let mut ts: Option<i64> = None;
    let mut sigs: Vec<&str> = Vec::new();
    for part in signature_header.split(',') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("t=") {
            ts = v.parse().ok();
        } else if let Some(v) = part.strip_prefix("v1=") {
            sigs.push(v);
        }
    }
    let Some(ts) = ts else {
        return Err(StripeError::BadSignatureHeader);
    };
    if sigs.is_empty() {
        return Err(StripeError::BadSignatureHeader);
    }
    let drift = (now.timestamp() - ts).abs();
    if drift > WEBHOOK_TIMESTAMP_TOLERANCE {
        return Err(StripeError::TimestampSkew);
    }

    let mut mac = Hmac::<Sha256>::new_from_slice(webhook_secret.as_bytes())
        .map_err(|e| StripeError::Decode(e.to_string()))?;
    mac.update(ts.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);
    let expected = mac.finalize().into_bytes();

    for sig_hex in sigs {
        let Ok(sig_bytes) = hex_decode(sig_hex) else {
            continue;
        };
        if sig_bytes.len() == expected.len() && constant_time_eq(&sig_bytes, &expected) {
            return Ok(());
        }
    }
    Err(StripeError::SignatureMismatch)
}

fn hex_decode(s: &str) -> Result<Vec<u8>, ()> {
    if s.len() % 2 != 0 {
        return Err(());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Subset of the Event resource we decode from verified webhook payloads.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebhookEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: WebhookEventData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebhookEventData {
    pub object: serde_json::Value,
}

/// Convenience: minimal projection of a `checkout.session.completed`
/// event payload — only the fields we use to issue the license.
#[derive(Debug, Clone, Deserialize)]
pub struct CheckoutSessionPayload {
    pub id: String,
    pub payment_intent: Option<String>,
    pub customer: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hmac_hex(secret: &str, msg: &str) -> String {
        use hmac::Mac;
        let mut m = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        m.update(msg.as_bytes());
        m.finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    #[test]
    fn webhook_signature_accepts_valid() {
        let payload = br#"{"id":"evt_test"}"#;
        let secret = "whsec_test";
        let now = Utc::now();
        let ts = now.timestamp();
        let signed = format!("{ts}.{}", std::str::from_utf8(payload).unwrap());
        let sig = hmac_hex(secret, &signed);
        let header = format!("t={ts},v1={sig}");
        assert!(verify_webhook_signature(payload, &header, secret, now).is_ok());
    }

    #[test]
    fn webhook_signature_rejects_tampered_payload() {
        let payload = br#"{"id":"evt_test"}"#;
        let secret = "whsec_test";
        let now = Utc::now();
        let ts = now.timestamp();
        let signed = format!("{ts}.{}", std::str::from_utf8(payload).unwrap());
        let sig = hmac_hex(secret, &signed);
        let header = format!("t={ts},v1={sig}");
        let tampered = br#"{"id":"evt_evil"}"#;
        assert!(matches!(
            verify_webhook_signature(tampered, &header, secret, now),
            Err(StripeError::SignatureMismatch)
        ));
    }

    #[test]
    fn webhook_signature_rejects_stale_timestamp() {
        let payload = br#"{"id":"evt_test"}"#;
        let secret = "whsec_test";
        let now = Utc::now();
        let stale_ts = now.timestamp() - 3600;
        let signed = format!("{stale_ts}.{}", std::str::from_utf8(payload).unwrap());
        let sig = hmac_hex(secret, &signed);
        let header = format!("t={stale_ts},v1={sig}");
        assert!(matches!(
            verify_webhook_signature(payload, &header, secret, now),
            Err(StripeError::TimestampSkew)
        ));
    }

    #[test]
    fn malformed_header_rejected() {
        let payload = b"x";
        let now = Utc::now();
        assert!(matches!(
            verify_webhook_signature(payload, "garbage", "s", now),
            Err(StripeError::BadSignatureHeader)
        ));
    }
}
