// ═══════════════════════════════════════════════════════════════════════════
// IDEMPOTENCY-KEY TESTS FOR CONTRACT PUBLISH (Issue #1055)
// ═══════════════════════════════════════════════════════════════════════════
//
// Tests for the optional `Idempotency-Key` header on POST /api/contracts:
// - A repeated request with the same key replays the first response instead
//   of re-running publish logic (proven by sending a *different* payload on
//   the retry and asserting the original, cached contract comes back).
// - Concurrent requests with the same key never create more than one
//   contract: exactly one succeeds with 200, the rest either see the same
//   cached contract back or a 409 "in progress" while the first is still
//   running.
// - The header is validated at the boundary: empty or oversized keys are
//   rejected with 400 before any publish logic runs.
// - Omitting the header entirely leaves existing behavior untouched.
//
// To run: cargo test --test idempotency_publish_tests -- --ignored
// ═══════════════════════════════════════════════════════════════════════════

use reqwest::StatusCode;
use serde_json::{json, Value};

fn api_base_url() -> String {
    std::env::var("TEST_API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

/// Generates a random 56-character Stellar-format StrKey (prefix + 55
/// uppercase alphanumeric characters), matching CONTRACT_ID_REGEX /
/// STELLAR_ADDRESS_REGEX in validation/validators.rs.
fn random_strkey(prefix: char) -> String {
    let raw = format!(
        "{:032X}{:032X}",
        uuid::Uuid::new_v4().as_u128(),
        uuid::Uuid::new_v4().as_u128()
    );
    format!("{}{}", prefix, &raw[..55])
}

fn publish_payload(contract_id: &str, wasm_hash: &str, network: &str) -> Value {
    json!({
        "contract_id": contract_id,
        "wasm_hash": wasm_hash,
        "name": format!("Test Contract {}", uuid::Uuid::new_v4()),
        "network": network,
        "publisher_address": random_strkey('G'),
        "tags": []
    })
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_repeated_publish_with_same_key_replays_cached_response() {
    let base = api_base_url();
    let client = reqwest::Client::new();
    let idempotency_key = uuid::Uuid::new_v4().to_string();

    let contract_id = random_strkey('C');
    let wasm_hash = format!("{:064x}", uuid::Uuid::new_v4().as_u128());
    let payload = publish_payload(&contract_id, &wasm_hash, "testnet");

    let res1 = client
        .post(format!("{}/api/contracts", base))
        .header("Idempotency-Key", &idempotency_key)
        .json(&payload)
        .send()
        .await
        .expect("first publish request failed");
    assert_eq!(res1.status(), StatusCode::OK);
    let first: Value = res1.json().await.unwrap();

    // Retry with the SAME key but a materially different payload (a new
    // contract_id and wasm_hash). If the idempotency cache is actually being
    // consulted before publish logic runs, this must still return the first
    // response untouched — not a second contract, and not a validation
    // error about the second payload.
    let different_contract_id = random_strkey('C');
    let different_wasm_hash = format!("{:064x}", uuid::Uuid::new_v4().as_u128());
    let different_payload =
        publish_payload(&different_contract_id, &different_wasm_hash, "testnet");

    let res2 = client
        .post(format!("{}/api/contracts", base))
        .header("Idempotency-Key", &idempotency_key)
        .json(&different_payload)
        .send()
        .await
        .expect("replayed publish request failed");
    assert_eq!(res2.status(), StatusCode::OK);
    let second: Value = res2.json().await.unwrap();

    assert_eq!(first["id"], second["id"]);
    assert_eq!(second["contract_id"], contract_id);
    assert_eq!(second["wasm_hash"], wasm_hash);
    assert_ne!(second["contract_id"], different_contract_id);
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_concurrent_requests_with_same_key_create_at_most_one_contract() {
    let base = api_base_url();
    let idempotency_key = uuid::Uuid::new_v4().to_string();

    let contract_id = random_strkey('C');
    let wasm_hash = format!("{:064x}", uuid::Uuid::new_v4().as_u128());
    let payload = publish_payload(&contract_id, &wasm_hash, "testnet");

    let mut handles = Vec::new();
    for _ in 0..10 {
        let base = base.clone();
        let key = idempotency_key.clone();
        let payload = payload.clone();
        handles.push(tokio::spawn(async move {
            reqwest::Client::new()
                .post(format!("{}/api/contracts", base))
                .header("Idempotency-Key", &key)
                .json(&payload)
                .send()
                .await
        }));
    }

    let mut ok_ids = Vec::new();
    let mut conflict_count = 0;
    for handle in handles {
        let response = handle
            .await
            .expect("task panicked")
            .expect("request failed");
        match response.status() {
            StatusCode::OK => {
                let body: Value = response.json().await.unwrap();
                ok_ids.push(body["id"].clone());
            }
            StatusCode::CONFLICT => {
                conflict_count += 1;
            }
            other => panic!("unexpected status from concurrent publish: {}", other),
        }
    }

    // Every 200 response must reference the exact same contract row — never
    // two different ids, which would mean the same key raced past the lock
    // and published twice.
    assert!(!ok_ids.is_empty(), "at least one request should succeed");
    for id in &ok_ids {
        assert_eq!(
            id, &ok_ids[0],
            "concurrent requests created different contracts"
        );
    }
    assert_eq!(
        ok_ids.len() + conflict_count,
        10,
        "every request must resolve to either 200 (cached/created) or 409 (in progress)"
    );
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_empty_idempotency_key_is_rejected() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let contract_id = random_strkey('C');
    let wasm_hash = format!("{:064x}", uuid::Uuid::new_v4().as_u128());
    let payload = publish_payload(&contract_id, &wasm_hash, "testnet");

    let res = client
        .post(format!("{}/api/contracts", base))
        .header("Idempotency-Key", "")
        .json(&payload)
        .send()
        .await
        .expect("request failed");
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_oversized_idempotency_key_is_rejected() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let contract_id = random_strkey('C');
    let wasm_hash = format!("{:064x}", uuid::Uuid::new_v4().as_u128());
    let payload = publish_payload(&contract_id, &wasm_hash, "testnet");
    let oversized_key = "a".repeat(256);

    let res = client
        .post(format!("{}/api/contracts", base))
        .header("Idempotency-Key", oversized_key)
        .json(&payload)
        .send()
        .await
        .expect("request failed");
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_missing_idempotency_key_publishes_normally() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let contract_id = random_strkey('C');
    let wasm_hash = format!("{:064x}", uuid::Uuid::new_v4().as_u128());
    let payload = publish_payload(&contract_id, &wasm_hash, "testnet");

    // No Idempotency-Key header at all — existing behavior must be unaffected.
    let res = client
        .post(format!("{}/api/contracts", base))
        .json(&payload)
        .send()
        .await
        .expect("request failed");
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["contract_id"], contract_id);
}
