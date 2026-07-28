// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT SLUG GENERATION TESTS
// ═══════════════════════════════════════════════════════════════════════════
//
// Tests for contract slug generation covering:
// - Auto-generation from name
// - Duplicate handling (numeric suffix)
// - Manual override
// - Fetching by slug
// - Immutability
//
// To run: cargo test --test slug_tests -- --ignored
// ═══════════════════════════════════════════════════════════════════════════

use reqwest::StatusCode;
use serde_json::{json, Value};

fn api_base_url() -> String {
    std::env::var("TEST_API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_slug_auto_generation() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let name = format!("My Awesome Contract {}", uuid::Uuid::new_v4());
    let expected_slug = name.to_lowercase().replace(" ", "-");

    let payload = json!({
        "contract_id": format!("C{}", uuid::Uuid::new_v4().to_string().replace("-", "")),
        "wasm_hash": format!("{:064x}", uuid::Uuid::new_v4().as_u128()),
        "name": &name,
        "network": "testnet",
        "publisher_address": format!("G{}", uuid::Uuid::new_v4().to_string().replace("-", ""))
    });

    let res = client
        .post(format!("{}/api/contracts", base))
        .json(&payload)
        .send()
        .await
        .expect("failed to create contract");

    assert_eq!(res.status(), StatusCode::CREATED);
    let contract: Value = res.json().await.unwrap();

    let slug = contract.get("slug").and_then(Value::as_str).unwrap();
    assert_eq!(slug, expected_slug);

    // Fetch by slug
    let get_res = client
        .get(format!("{}/api/contracts/{}?network=testnet", base, slug))
        .send()
        .await
        .unwrap();

    assert_eq!(get_res.status(), StatusCode::OK);
    let fetched: Value = get_res.json().await.unwrap();
    assert_eq!(fetched["contract"]["id"], contract["id"]);
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_duplicate_slug_handling() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let shared_name = format!("Duplicate Name {}", uuid::Uuid::new_v4());
    let base_slug = shared_name.to_lowercase().replace(" ", "-");

    // First contract
    let payload1 = json!({
        "contract_id": format!("C{}", uuid::Uuid::new_v4().to_string().replace("-", "")),
        "wasm_hash": format!("{:064x}", uuid::Uuid::new_v4().as_u128()),
        "name": &shared_name,
        "network": "testnet",
        "publisher_address": format!("G{}", uuid::Uuid::new_v4().to_string().replace("-", ""))
    });

    let res1 = client
        .post(format!("{}/api/contracts", base))
        .json(&payload1)
        .send()
        .await
        .unwrap();
    assert_eq!(res1.status(), StatusCode::CREATED);
    let c1: Value = res1.json().await.unwrap();
    assert_eq!(c1["slug"], base_slug);

    // Second contract with same name on same network
    let payload2 = json!({
        "contract_id": format!("C{}", uuid::Uuid::new_v4().to_string().replace("-", "")),
        "wasm_hash": format!("{:064x}", uuid::Uuid::new_v4().as_u128()),
        "name": &shared_name,
        "network": "testnet",
        "publisher_address": format!("G{}", uuid::Uuid::new_v4().to_string().replace("-", ""))
    });

    let res2 = client
        .post(format!("{}/api/contracts", base))
        .json(&payload2)
        .send()
        .await
        .unwrap();
    assert_eq!(res2.status(), StatusCode::CREATED);
    let c2: Value = res2.json().await.unwrap();
    assert_eq!(c2["slug"], format!("{}-1", base_slug));

    // Third contract with same name on DIFFERENT network (should NOT have suffix)
    let payload3 = json!({
        "contract_id": format!("C{}", uuid::Uuid::new_v4().to_string().replace("-", "")),
        "wasm_hash": format!("{:064x}", uuid::Uuid::new_v4().as_u128()),
        "name": &shared_name,
        "network": "mainnet",
        "publisher_address": format!("G{}", uuid::Uuid::new_v4().to_string().replace("-", ""))
    });

    let res3 = client
        .post(format!("{}/api/contracts", base))
        .json(&payload3)
        .send()
        .await
        .unwrap();
    assert_eq!(res3.status(), StatusCode::CREATED);
    let c3: Value = res3.json().await.unwrap();
    assert_eq!(c3["slug"], base_slug);
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_manual_slug_override() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let name = format!("Override Me {}", uuid::Uuid::new_v4());
    let manual_slug = format!("custom-slug-{}", uuid::Uuid::new_v4());

    let payload = json!({
        "contract_id": format!("C{}", uuid::Uuid::new_v4().to_string().replace("-", "")),
        "wasm_hash": format!("{:064x}", uuid::Uuid::new_v4().as_u128()),
        "name": &name,
        "slug": &manual_slug,
        "network": "testnet",
        "publisher_address": format!("G{}", uuid::Uuid::new_v4().to_string().replace("-", ""))
    });

    let res = client
        .post(format!("{}/api/contracts", base))
        .json(&payload)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let contract: Value = res.json().await.unwrap();
    assert_eq!(contract["slug"], manual_slug);
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_slug_immutability() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let name = format!("Immutable Slug {}", uuid::Uuid::new_v4());
    let payload = json!({
        "contract_id": format!("C{}", uuid::Uuid::new_v4().to_string().replace("-", "")),
        "wasm_hash": format!("{:064x}", uuid::Uuid::new_v4().as_u128()),
        "name": &name,
        "network": "testnet",
        "publisher_address": format!("G{}", uuid::Uuid::new_v4().to_string().replace("-", ""))
    });

    let res = client
        .post(format!("{}/api/contracts", base))
        .json(&payload)
        .send()
        .await
        .unwrap();
    let contract: Value = res.json().await.unwrap();
    let slug = contract["slug"].as_str().unwrap().to_string();
    let id = contract["id"].as_str().unwrap();

    // Update metadata (name) - should NOT change slug
    let update_payload = json!({
        "name": "New Name That Should Not Affect Slug"
    });

    let update_res = client
        .patch(format!("{}/api/contracts/{}", base, id))
        .json(&update_payload)
        .send()
        .await
        .unwrap();

    assert_eq!(update_res.status(), StatusCode::OK);
    let updated: Value = update_res.json().await.unwrap();
    assert_eq!(updated["slug"], slug);
    assert_eq!(updated["name"], "New Name That Should Not Affect Slug");
}
