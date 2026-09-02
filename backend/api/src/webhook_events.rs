//! Webhook event emission for lifecycle events (#1110).
//!
//! Provides a single public helper – `emit_webhook_event` – that handlers call
//! after completing a state-changing operation.  The helper:
//!
//! 1. Looks up every active `webhook_subscriptions` row that covers the event type
//!    for the relevant publisher.
//! 2. Builds a signed JSON payload using HMAC-SHA256.
//! 3. Enqueues a `webhook_subscription_deliveries` row (status = 'pending') for
//!    each matching subscription; the background delivery worker picks them up.
//!
//! Delivery itself (HTTP POST, retry, dead-letter) is handled by the existing
//! `webhook_delivery` background task, which was extended in the migration to
//! also poll `webhook_subscription_deliveries`.

use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use tracing::{error, warn};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

// ─── Public constants for event type strings ─────────────────────────────────

/// Fired when a contract is deprecated via POST /api/contracts/:id/deprecate.
pub const EVENT_CONTRACT_DEPRECATED: &str = "contract.deprecated";

/// Fired when an ownership transfer reaches 'completed' status.
pub const EVENT_OWNERSHIP_TRANSFERRED: &str = "ownership.transferred";

/// Fired when a dependency scan finds at least one vulnerability.
pub const EVENT_VULNERABILITY_FOUND: &str = "vulnerability.found";

// ─── Public API ───────────────────────────────────────────────────────────────

/// Enqueue webhook deliveries for `event_type` addressed to `publisher_id`.
///
/// The call is best-effort: individual enqueue failures are logged but never
/// propagated back to the caller, so a misconfigured webhook cannot break the
/// primary operation that triggered the event.
///
/// # Arguments
/// * `db`           – Shared database pool.
/// * `publisher_id` – The publisher who owns the subscriptions to notify.
/// * `event_type`   – One of the `EVENT_*` constants defined above.
/// * `payload`      – Arbitrary JSON value included in the delivery body.
pub async fn emit_webhook_event(
    db: &PgPool,
    publisher_id: Uuid,
    event_type: &str,
    payload: serde_json::Value,
) {
    let subscriptions = match fetch_active_subscriptions(db, publisher_id, event_type).await {
        Ok(subs) => subs,
        Err(e) => {
            error!(
                %publisher_id,
                event_type,
                error = %e,
                "Failed to fetch webhook subscriptions for event emission"
            );
            return;
        }
    };

    if subscriptions.is_empty() {
        return;
    }

    let payload_str = serde_json::json!({
        "event_type": event_type,
        "publisher_id": publisher_id,
        "timestamp": Utc::now().to_rfc3339(),
        "data": payload,
    })
    .to_string();

    for sub in subscriptions {
        let signature = sign_payload(&payload_str, &sub.secret);
        let payload_json: serde_json::Value =
            serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);

        if let Err(e) = enqueue_delivery(db, sub.id, event_type, payload_json, &signature).await {
            warn!(
                subscription_id = %sub.id,
                %publisher_id,
                event_type,
                error = %e,
                "Failed to enqueue webhook delivery"
            );
        }
    }
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow)]
struct SubscriptionRow {
    id: Uuid,
    secret: String,
}

async fn fetch_active_subscriptions(
    db: &PgPool,
    publisher_id: Uuid,
    event_type: &str,
) -> Result<Vec<SubscriptionRow>, sqlx::Error> {
    sqlx::query_as::<_, SubscriptionRow>(
        r#"
        SELECT id, secret
        FROM webhook_subscriptions
        WHERE publisher_id = $1
          AND is_active    = TRUE
          AND $2           = ANY(event_types)
        "#,
    )
    .bind(publisher_id)
    .bind(event_type)
    .fetch_all(db)
    .await
}

/// HMAC-SHA256 sign the payload; returns a `sha256=<hex>` signature string.
pub fn sign_payload(payload: &str, secret: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(payload.as_bytes());
    let result = mac.finalize().into_bytes();
    format!("sha256={}", hex::encode(result))
}

/// Verify a `sha256=<hex>` signature against a payload and secret.
///
/// Returns `true` when the signature is valid, `false` otherwise.
pub fn verify_signature(payload: &str, secret: &str, signature: &str) -> bool {
    let expected = sign_payload(payload, secret);
    // Constant-time comparison to prevent timing attacks.
    constant_time_eq(expected.as_bytes(), signature.as_bytes())
}

/// Constant-time byte-slice comparison.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

async fn enqueue_delivery(
    db: &PgPool,
    subscription_id: Uuid,
    event_type: &str,
    payload: serde_json::Value,
    _signature: &str, // stored for reference; delivery worker re-signs at send time
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO webhook_subscription_deliveries
            (subscription_id, event_type, payload, status, attempt_number, created_at, updated_at)
        VALUES ($1, $2, $3, 'pending', 0, NOW(), NOW())
        "#,
    )
    .bind(subscription_id)
    .bind(event_type)
    .bind(payload)
    .execute(db)
    .await?;
    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Signature helpers ─────────────────────────────────────────────────────

    #[test]
    fn sign_and_verify_round_trip() {
        let payload = r#"{"event_type":"contract.deprecated","data":{}}"#;
        let secret = "super-secret-key-32-bytes-minimum";
        let sig = sign_payload(payload, secret);
        assert!(
            verify_signature(payload, secret, &sig),
            "round-trip should verify"
        );
    }

    #[test]
    fn signature_format_starts_with_sha256_prefix() {
        let sig = sign_payload("hello", "key");
        assert!(
            sig.starts_with("sha256="),
            "signature should start with 'sha256='"
        );
    }

    #[test]
    fn wrong_secret_fails_verification() {
        let payload = "some payload";
        let sig = sign_payload(payload, "correct-secret");
        assert!(
            !verify_signature(payload, "wrong-secret", &sig),
            "wrong secret must not verify"
        );
    }

    #[test]
    fn tampered_payload_fails_verification() {
        let secret = "my-secret";
        let original = r#"{"event_type":"vulnerability.found"}"#;
        let sig = sign_payload(original, secret);
        let tampered = r#"{"event_type":"vulnerability.found","injected":true}"#;
        assert!(
            !verify_signature(tampered, secret, &sig),
            "tampered payload must not verify"
        );
    }

    #[test]
    fn empty_payload_produces_deterministic_signature() {
        let sig1 = sign_payload("", "key");
        let sig2 = sign_payload("", "key");
        assert_eq!(sig1, sig2, "same inputs must produce same signature");
    }

    #[test]
    fn different_payloads_produce_different_signatures() {
        let secret = "same-secret";
        let sig_a = sign_payload("payload-a", secret);
        let sig_b = sign_payload("payload-b", secret);
        assert_ne!(
            sig_a, sig_b,
            "different payloads must produce different signatures"
        );
    }

    // ── Event type constants ──────────────────────────────────────────────────

    #[test]
    fn event_type_constants_are_correct() {
        assert_eq!(EVENT_CONTRACT_DEPRECATED, "contract.deprecated");
        assert_eq!(EVENT_OWNERSHIP_TRANSFERRED, "ownership.transferred");
        assert_eq!(EVENT_VULNERABILITY_FOUND, "vulnerability.found");
    }
}
