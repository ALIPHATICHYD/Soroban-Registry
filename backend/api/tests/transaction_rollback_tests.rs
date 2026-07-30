// ═══════════════════════════════════════════════════════════════════════════
// TRANSACTION ROLLBACK TESTS (Issue #1109)
// ═══════════════════════════════════════════════════════════════════════════
//
// Verifies that multi-table writes properly roll back if a failure occurs
// mid-request, leaving the database in a consistent state without orphaned
// or partial records.
//
// To run: cargo test --test transaction_rollback_tests -- --ignored
// ═══════════════════════════════════════════════════════════════════════════

use reqwest::StatusCode;
use serde_json::{json, Value};

fn api_base_url() -> String {
    std::env::var("TEST_API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

/// Generates a random 56-character Stellar-format StrKey
fn random_strkey(prefix: char) -> String {
    let raw = format!(
        "{:032X}{:032X}",
        uuid::Uuid::new_v4().as_u128(),
        uuid::Uuid::new_v4().as_u128()
    );
    format!("{}{}", prefix, &raw[..55])
}

#[tokio::test]
#[ignore]
async fn test_publish_contract_transaction_rollback() {
    let client = reqwest::Client::new();
    let base_url = api_base_url();

    let contract_id = random_strkey('C');
    let wasm_hash = format!("{:064x}", uuid::Uuid::new_v4().as_u128());
    
    // Create a payload that will succeed on the initial `contracts` insert,
    // but fail later during `save_dependencies` due to a VARCHAR(255) constraint
    // violation on `dependency_name`.
    let long_dependency_name = "A".repeat(300);
    
    let payload = json!({
        "contract_id": contract_id,
        "name": "Rollback Test Contract",
        "description": "This contract should not persist",
        "network": "testnet",
        "wasm_hash": wasm_hash,
        "slug": "rollback-test",
        "category": "defi",
        "tags": ["test"],
        "dependencies": [
            {
                "name": long_dependency_name,
                "version_constraint": "^1.0.0"
            }
        ]
    });

    let res = client
        .post(format!("{}/api/contracts", base_url))
        .header("Content-Type", "application/json")
        .header("X-Publisher-Address", random_strkey('G'))
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    // The request should fail due to internal server error (DB error: value too long for type character varying(255))
    // Or it might be a 400/500 depending on how errors map, but it definitely shouldn't be 200 OK.
    assert_ne!(res.status(), StatusCode::OK);

    // Verify the contract was NOT inserted (rolled back)
    let get_res = client
        .get(format!("{}/api/contracts/{}", base_url, contract_id))
        .send()
        .await
        .expect("Failed to execute get request");
        
    // It should not exist
    assert_eq!(get_res.status(), StatusCode::NOT_FOUND);
}
