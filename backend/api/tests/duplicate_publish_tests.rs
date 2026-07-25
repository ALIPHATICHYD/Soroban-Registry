// ═══════════════════════════════════════════════════════════════════════════
// DUPLICATE CONTRACT DETECTION ON PUBLISH TESTS (Issue #953)
// ═══════════════════════════════════════════════════════════════════════════
//
// Tests for duplicate-contract detection on POST /api/contracts covering:
// - Idempotent replay of a prior successful publish (same contract_id,
//   network, and wasm_hash) returns the existing record instead of erroring.
// - Republishing an already-registered contract_id/network with different
//   source code returns a 409 conflict referencing the existing record.
// - Reusing the same wasm_hash under a different contract_id is allowed
//   (identical bytecode is legitimately deployed as separate instances).
//
// To run: cargo test --test duplicate_publish_tests -- --ignored
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
async fn test_repeated_publish_with_same_source_is_idempotent() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let contract_id = random_strkey('C');
    let wasm_hash = format!("{:064x}", uuid::Uuid::new_v4().as_u128());
    let payload = publish_payload(&contract_id, &wasm_hash, "testnet");

    let res1 = client
        .post(format!("{}/api/contracts", base))
        .json(&payload)
        .send()
        .await
        .expect("first publish request failed");
    assert_eq!(res1.status(), StatusCode::OK);
    let first: Value = res1.json().await.unwrap();

    // Retry the exact same publish (e.g. a client retrying after a dropped
    // response). This must not create a second row or return an error.
    let res2 = client
        .post(format!("{}/api/contracts", base))
        .json(&payload)
        .send()
        .await
        .expect("second publish request failed");
    assert_eq!(res2.status(), StatusCode::OK);
    let second: Value = res2.json().await.unwrap();

    assert_eq!(first["id"], second["id"]);
    assert_eq!(second["contract_id"], contract_id);
    assert_eq!(second["wasm_hash"], wasm_hash);
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_republish_with_different_source_returns_conflict() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let contract_id = random_strkey('C');
    let wasm_hash = format!("{:064x}", uuid::Uuid::new_v4().as_u128());
    let payload = publish_payload(&contract_id, &wasm_hash, "testnet");

    let res1 = client
        .post(format!("{}/api/contracts", base))
        .json(&payload)
        .send()
        .await
        .expect("initial publish request failed");
    assert_eq!(res1.status(), StatusCode::OK);
    let existing: Value = res1.json().await.unwrap();

    // Same contract_id + network, but different wasm_hash: a genuine attempt
    // to overwrite an already-registered contract's source.
    let other_wasm_hash = format!("{:064x}", uuid::Uuid::new_v4().as_u128());
    let conflicting_payload = publish_payload(&contract_id, &other_wasm_hash, "testnet");

    let res2 = client
        .post(format!("{}/api/contracts", base))
        .json(&conflicting_payload)
        .send()
        .await
        .expect("conflicting publish request failed");
    assert_eq!(res2.status(), StatusCode::CONFLICT);

    let body: Value = res2.json().await.unwrap();
    let reference = &body["details"]["existing_contract"];
    assert_eq!(reference["id"], existing["id"]);
    assert_eq!(reference["contract_id"], contract_id);
    assert_eq!(reference["wasm_hash"], wasm_hash);
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_same_source_hash_under_different_contract_id_is_allowed() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let wasm_hash = format!("{:064x}", uuid::Uuid::new_v4().as_u128());

    let contract_id_a = random_strkey('C');
    let payload_a = publish_payload(&contract_id_a, &wasm_hash, "testnet");
    let res_a = client
        .post(format!("{}/api/contracts", base))
        .json(&payload_a)
        .send()
        .await
        .unwrap();
    assert_eq!(res_a.status(), StatusCode::OK);

    // Identical bytecode published under a different contract_id is a
    // legitimate separate deployment, not a duplicate.
    let contract_id_b = random_strkey('C');
    let payload_b = publish_payload(&contract_id_b, &wasm_hash, "testnet");
    let res_b = client
        .post(format!("{}/api/contracts", base))
        .json(&payload_b)
        .send()
        .await
        .unwrap();
    assert_eq!(res_b.status(), StatusCode::OK);

    let a: Value = res_a.json().await.unwrap();
    let b: Value = res_b.json().await.unwrap();
    assert_ne!(a["id"], b["id"]);
    assert_eq!(a["wasm_hash"], b["wasm_hash"]);
}
