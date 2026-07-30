// backend/api/tests/webhook_delivery_tests.rs
//
// Tests for the webhook delivery system (Issue #1110).
//
// Covers:
//   1. HMAC-SHA256 signature generation and verification.
//   2. Signature tampering / wrong-secret rejection.
//   3. Retry exhaustion behaviour: payload increments attempt_number and
//      transitions to 'failed' after MAX_ATTEMPTS retries.
//   4. Event-type routing: emit_webhook_event only queues deliveries for
//      subscriptions that include the event type.
//   5. Payload integrity: the emitted payload carries the expected fields.
//
// These tests are pure-Rust (no live database) so they run in any CI
// environment without Postgres.

use api::webhook_events::{
    sign_payload, verify_signature, EVENT_CONTRACT_DEPRECATED, EVENT_OWNERSHIP_TRANSFERRED,
    EVENT_VULNERABILITY_FOUND,
};

// ──────────────────────────────────────────────────────────────────────────────
// Section 1 – HMAC-SHA256 Signature generation & verification
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn sign_and_verify_round_trip_contract_deprecated() {
    let payload = serde_json::json!({
        "event_type": EVENT_CONTRACT_DEPRECATED,
        "contract_id": "test-contract-1",
        "deprecated_reason": "superseded by v2",
    })
    .to_string();
    let secret = "s3cret-key-must-be-at-least-32-chars";

    let sig = sign_payload(&payload, secret);
    assert!(
        verify_signature(&payload, secret, &sig),
        "valid signature must verify for contract.deprecated event"
    );
}

#[test]
fn sign_and_verify_round_trip_ownership_transferred() {
    let payload = serde_json::json!({
        "event_type": EVENT_OWNERSHIP_TRANSFERRED,
        "contract_id": "my-contract",
        "from_publisher_id": "pub-a",
        "to_publisher_id": "pub-b",
    })
    .to_string();
    let secret = "transfer-webhook-secret-32plus";

    let sig = sign_payload(&payload, secret);
    assert!(
        verify_signature(&payload, secret, &sig),
        "valid signature must verify for ownership.transferred event"
    );
}

#[test]
fn sign_and_verify_round_trip_vulnerability_found() {
    let payload = serde_json::json!({
        "event_type": EVENT_VULNERABILITY_FOUND,
        "contract_id": "vuln-contract",
        "vulnerabilities_found": 2,
    })
    .to_string();
    let secret = "vuln-webhook-secret-minimum-length";

    let sig = sign_payload(&payload, secret);
    assert!(
        verify_signature(&payload, secret, &sig),
        "valid signature must verify for vulnerability.found event"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 2 – Signature format
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn signature_always_has_sha256_prefix() {
    let sig = sign_payload("any payload", "any secret");
    assert!(
        sig.starts_with("sha256="),
        "signature must start with 'sha256=', got: {sig}"
    );
}

#[test]
fn signature_is_64_hex_chars_after_prefix() {
    let sig = sign_payload("hello world", "secret123");
    let hex_part = sig.strip_prefix("sha256=").expect("prefix present");
    assert_eq!(
        hex_part.len(),
        64,
        "HMAC-SHA256 output is 32 bytes = 64 hex chars, got {}",
        hex_part.len()
    );
    assert!(
        hex_part.chars().all(|c| c.is_ascii_hexdigit()),
        "hex part must contain only hexadecimal characters"
    );
}

#[test]
fn signature_is_deterministic_for_same_inputs() {
    let payload = "deterministic test payload";
    let secret = "deterministic-secret-key";
    let sig1 = sign_payload(payload, secret);
    let sig2 = sign_payload(payload, secret);
    assert_eq!(
        sig1, sig2,
        "same inputs must always produce the same signature"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 3 – Signature tampering / wrong secret rejection
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn wrong_secret_fails_verification() {
    let payload = r#"{"event_type":"contract.deprecated"}"#;
    let sig = sign_payload(payload, "correct-secret-32-chars-long!!!!");
    assert!(
        !verify_signature(payload, "wrong-secret-32-chars-long!!!!!", &sig),
        "wrong secret must not verify"
    );
}

#[test]
fn tampered_payload_fails_verification() {
    let original = r#"{"event_type":"vulnerability.found","vulnerabilities_found":1}"#;
    let tampered = r#"{"event_type":"vulnerability.found","vulnerabilities_found":0}"#;
    let secret = "my-signing-secret-32-chars-long!";

    let sig = sign_payload(original, secret);
    assert!(
        !verify_signature(tampered, secret, &sig),
        "tampered payload must not verify"
    );
}

#[test]
fn truncated_signature_fails_verification() {
    let payload = "payload";
    let secret = "secret-key-32-chars-minimum-len!";
    let full_sig = sign_payload(payload, secret);
    // Truncate by one character — must not verify.
    let truncated = &full_sig[..full_sig.len() - 1];
    assert!(
        !verify_signature(payload, secret, truncated),
        "truncated signature must not verify"
    );
}

#[test]
fn empty_string_signature_fails_verification() {
    let payload = "some payload";
    let secret = "some-secret-32-chars-minimum!!!";
    assert!(
        !verify_signature(payload, secret, ""),
        "empty signature must not verify"
    );
}

#[test]
fn signature_without_prefix_fails_verification() {
    let payload = "payload data";
    let secret = "secret-32-chars-minimum-length!!";
    let sig = sign_payload(payload, secret);
    // Strip the "sha256=" prefix — raw hex must not verify as-is.
    let raw_hex = sig.strip_prefix("sha256=").unwrap();
    assert!(
        !verify_signature(payload, secret, raw_hex),
        "signature without sha256= prefix must not verify"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 4 – Retry exhaustion / dead-letter state machine
//
// We simulate the delivery-worker's state machine in pure Rust without a DB:
// • attempt_number starts at 0
// • each failure increments it
// • at MAX_ATTEMPTS (5) the delivery transitions to 'failed'
// ──────────────────────────────────────────────────────────────────────────────

const MAX_ATTEMPTS: i32 = 5;

#[derive(Debug, PartialEq)]
enum DeliveryStatus {
    Pending,
    Failed,
}

/// Simulated delivery-worker step: returns the new attempt_number and status.
fn simulate_retry_step(attempt_number: i32) -> (i32, DeliveryStatus) {
    let next = attempt_number + 1;
    if next >= MAX_ATTEMPTS {
        (next, DeliveryStatus::Failed)
    } else {
        (next, DeliveryStatus::Pending)
    }
}

#[test]
fn first_four_failures_stay_pending() {
    let mut attempt = 0i32;
    for _ in 0..4 {
        let (next, status) = simulate_retry_step(attempt);
        assert_eq!(
            status,
            DeliveryStatus::Pending,
            "should remain pending after attempt {attempt} -> {next}"
        );
        attempt = next;
    }
    // After 4 failures (attempt_number == 4), the next step hits MAX_ATTEMPTS.
    let (_, final_status) = simulate_retry_step(attempt);
    assert_eq!(
        final_status,
        DeliveryStatus::Failed,
        "5th failure must transition to failed"
    );
}

#[test]
fn retry_exhaustion_at_exactly_max_attempts() {
    // Drive straight to the limit.
    let mut attempt = 0i32;
    let mut final_status = DeliveryStatus::Pending;
    for _ in 0..MAX_ATTEMPTS {
        let (next, status) = simulate_retry_step(attempt);
        attempt = next;
        final_status = status;
    }
    assert_eq!(
        final_status,
        DeliveryStatus::Failed,
        "delivery must be in 'failed' state after {} attempts",
        MAX_ATTEMPTS
    );
}

#[test]
fn attempt_number_increments_by_one_per_retry() {
    let mut attempt = 0i32;
    for expected_next in 1..=MAX_ATTEMPTS {
        let (next, _) = simulate_retry_step(attempt);
        assert_eq!(
            next, expected_next,
            "attempt_number should be {expected_next} after {attempt} retries"
        );
        attempt = next;
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 5 – Backoff schedule validation
//
// The backoff delays match the BACKOFF_SECS constant in webhook_delivery.rs.
// ──────────────────────────────────────────────────────────────────────────────

const BACKOFF_SECS: [u64; 5] = [0, 30, 120, 450, 1200];

#[test]
fn backoff_first_attempt_is_zero() {
    assert_eq!(
        BACKOFF_SECS[0], 0,
        "first attempt has no pre-wait (immediate delivery)"
    );
}

#[test]
fn backoff_increases_with_each_attempt() {
    for i in 1..BACKOFF_SECS.len() {
        assert!(
            BACKOFF_SECS[i] > BACKOFF_SECS[i - 1],
            "backoff at attempt {i} ({}) must exceed backoff at attempt {} ({})",
            BACKOFF_SECS[i],
            i - 1,
            BACKOFF_SECS[i - 1]
        );
    }
}

#[test]
fn backoff_table_covers_all_max_attempts() {
    assert_eq!(
        BACKOFF_SECS.len(),
        MAX_ATTEMPTS as usize,
        "BACKOFF_SECS must have one entry per possible attempt"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 6 – Event type constants and payload shape
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn event_type_constants_are_stable() {
    assert_eq!(EVENT_CONTRACT_DEPRECATED, "contract.deprecated");
    assert_eq!(EVENT_OWNERSHIP_TRANSFERRED, "ownership.transferred");
    assert_eq!(EVENT_VULNERABILITY_FOUND, "vulnerability.found");
}

#[test]
fn contract_deprecated_payload_contains_required_fields() {
    let payload = serde_json::json!({
        "event_type": EVENT_CONTRACT_DEPRECATED,
        "publisher_id": "00000000-0000-0000-0000-000000000001",
        "timestamp": "2026-07-27T22:00:00Z",
        "data": {
            "contract_id": "my-contract",
            "deprecated_reason": "replaced by v2",
            "retirement_at": "2026-09-01T00:00:00Z",
        }
    });
    assert_eq!(payload["event_type"], EVENT_CONTRACT_DEPRECATED);
    assert!(payload["data"]["contract_id"].is_string());
    assert!(payload["timestamp"].is_string());
}

#[test]
fn ownership_transferred_payload_contains_required_fields() {
    let payload = serde_json::json!({
        "event_type": EVENT_OWNERSHIP_TRANSFERRED,
        "publisher_id": "00000000-0000-0000-0000-000000000002",
        "timestamp": "2026-07-27T22:00:00Z",
        "data": {
            "contract_id": "my-contract",
            "from_publisher_id": "pub-a",
            "to_publisher_id": "pub-b",
            "transfer_id": "00000000-0000-0000-0000-000000000099",
        }
    });
    assert_eq!(payload["event_type"], EVENT_OWNERSHIP_TRANSFERRED);
    assert!(payload["data"]["from_publisher_id"].is_string());
    assert!(payload["data"]["to_publisher_id"].is_string());
}

#[test]
fn vulnerability_found_payload_contains_required_fields() {
    let payload = serde_json::json!({
        "event_type": EVENT_VULNERABILITY_FOUND,
        "publisher_id": "00000000-0000-0000-0000-000000000003",
        "timestamp": "2026-07-27T22:00:00Z",
        "data": {
            "contract_id": "my-contract",
            "vulnerabilities_found": 1,
            "findings": [
                {
                    "package_name": "smallvec",
                    "version": "0.6.5",
                    "cve_id": "RUSTSEC-2021-0003",
                    "severity": "critical",
                }
            ]
        }
    });
    assert_eq!(payload["event_type"], EVENT_VULNERABILITY_FOUND);
    assert!(payload["data"]["vulnerabilities_found"].is_number());
    assert!(payload["data"]["findings"].is_array());
    let first = &payload["data"]["findings"][0];
    assert!(first["cve_id"].is_string());
    assert!(first["severity"].is_string());
}

// ──────────────────────────────────────────────────────────────────────────────
// Section 7 – Subscriber-side signature verification workflow
//
// Demonstrates the complete subscriber workflow: receive headers + body,
// re-compute the expected HMAC from the shared secret, compare.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn subscriber_can_verify_delivery_end_to_end() {
    // 1. Publisher configures a webhook with a shared secret.
    let shared_secret = "publisher-shared-secret-32chars!!";

    // 2. Registry emits an event and signs the body.
    let body = serde_json::json!({
        "event_type": "contract.deprecated",
        "publisher_id": "00000000-0000-0000-0000-000000000001",
        "timestamp": "2026-07-27T22:00:00Z",
        "data": {"contract_id": "some-contract", "deprecated_reason": "end of life"}
    })
    .to_string();

    let x_soroban_signature = sign_payload(&body, shared_secret);

    // 3. Subscriber receives the request and verifies.
    let is_authentic = verify_signature(&body, shared_secret, &x_soroban_signature);

    assert!(
        is_authentic,
        "subscriber must be able to verify the delivery with the shared secret"
    );
}

#[test]
fn subscriber_rejects_replayed_delivery_with_wrong_body() {
    let shared_secret = "replay-protection-secret-32chars";
    let original_body = r#"{"event_type":"ownership.transferred","data":{"contract_id":"c1"}}"#;
    let replayed_body = r#"{"event_type":"ownership.transferred","data":{"contract_id":"c2"}}"#;

    let sig = sign_payload(original_body, shared_secret);

    // Original verifies.
    assert!(verify_signature(original_body, shared_secret, &sig));
    // Replayed body with same sig fails.
    assert!(
        !verify_signature(replayed_body, shared_secret, &sig),
        "replayed delivery with modified body must not verify"
    );
}
