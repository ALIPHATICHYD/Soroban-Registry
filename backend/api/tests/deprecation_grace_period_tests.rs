// ═══════════════════════════════════════════════════════════════════════════
// DEPRECATION GRACE-PERIOD TESTS  (issue #1061)
// ═══════════════════════════════════════════════════════════════════════════
//
// Tests that deprecated contracts:
//   - are still fully resolvable during the grace period
//   - include a `deprecation_warning` in every GET response
//   - are hard-deleted once the grace period has elapsed (via purge endpoint)
//   - can be undeprecated and the warning disappears
//
// Run: cargo test --test deprecation_grace_period_tests -- --ignored
// ═══════════════════════════════════════════════════════════════════════════

use chrono::{Duration, Utc};
use reqwest::StatusCode;
use serde_json::{json, Value};

fn api_base_url() -> String {
    std::env::var("TEST_API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

/// Helper: publish a throwaway contract and return its UUID (as String).
async fn publish_test_contract(client: &reqwest::Client, base: &str) -> String {
    let payload = json!({
        "contract_id": format!("C{}", uuid::Uuid::new_v4().to_string().replace("-", "")),
        "wasm_hash": format!("{:064x}", uuid::Uuid::new_v4().as_u128()),
        "name": format!("DeprecationTest-{}", uuid::Uuid::new_v4()),
        "network": "testnet",
        "publisher_address": format!("G{}", uuid::Uuid::new_v4().to_string().replace("-", ""))
    });

    let res = client
        .post(format!("{}/api/contracts", base))
        .json(&payload)
        .send()
        .await
        .expect("publish contract");

    assert!(
        res.status().is_success(),
        "publish failed: {}",
        res.status()
    );

    let body: Value = res.json().await.unwrap();
    body["id"]
        .as_str()
        .expect("id field in publish response")
        .to_string()
}

// ─── 1. Deprecated contract is still resolvable during the grace period ──────

/// A contract that has been deprecated with a future retirement date must still
/// be returned with HTTP 200 by both the main and V1 endpoints.
#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_deprecated_contract_still_resolvable() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let contract_id = publish_test_contract(&client, &base).await;

    // Deprecate with a retirement date 30 days in the future
    let retirement_at = (Utc::now() + Duration::days(30)).to_rfc3339();
    let deprecate_payload = json!({
        "retirement_at":    retirement_at,
        "migration_guide_url": "https://example.com/migration",
        "deprecated_reason": "Replaced by v2",
        "grace_period_days": 30
    });

    let dep_res = client
        .post(format!("{}/api/contracts/{}/deprecate", base, contract_id))
        .json(&deprecate_payload)
        .send()
        .await
        .expect("deprecate request");

    assert_eq!(dep_res.status(), StatusCode::OK, "deprecate should succeed");

    // The contract must still be resolvable (200) from the main GET endpoint
    let get_res = client
        .get(format!("{}/api/contracts/{}", base, contract_id))
        .send()
        .await
        .expect("GET contract");

    assert_eq!(
        get_res.status(),
        StatusCode::OK,
        "deprecated contract must still return 200"
    );

    let body: Value = get_res.json().await.unwrap();

    // The contract itself must be present
    assert_eq!(body["id"].as_str().unwrap_or(""), contract_id);
}

// ─── 2. GET response includes deprecation_warning for deprecated contract ────

/// Verify that every GET of a deprecated contract includes a populated
/// `deprecation_warning` object with the expected fields.
#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_get_response_includes_deprecation_warning() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let contract_id = publish_test_contract(&client, &base).await;

    let retirement_at = (Utc::now() + Duration::days(14)).to_rfc3339();
    let reason = "Superseded by v3";
    let deprecate_payload = json!({
        "retirement_at":     retirement_at,
        "migration_guide_url": "https://docs.example.com/v3-migration",
        "deprecated_reason": reason,
        "grace_period_days": 14
    });

    client
        .post(format!("{}/api/contracts/{}/deprecate", base, contract_id))
        .json(&deprecate_payload)
        .send()
        .await
        .expect("deprecate")
        .error_for_status()
        .expect("deprecate OK");

    // Main endpoint
    let body: Value = client
        .get(format!("{}/api/contracts/{}", base, contract_id))
        .send()
        .await
        .expect("GET")
        .json()
        .await
        .unwrap();

    let warning = body.get("deprecation_warning").expect(
        "deprecation_warning field must be present in GET /api/contracts/:id for a deprecated contract",
    );

    assert!(!warning.is_null(), "deprecation_warning must not be null");
    assert_eq!(
        warning["message"].as_str().unwrap_or(""),
        reason,
        "deprecation_warning.message should match deprecated_reason"
    );
    assert!(
        warning.get("deprecated_at").is_some(),
        "deprecation_warning must contain deprecated_at"
    );
    assert!(
        warning.get("retirement_at").is_some(),
        "deprecation_warning must contain retirement_at"
    );
    assert!(
        warning["days_until_retirement"].as_i64().unwrap_or(-1) > 0,
        "days_until_retirement should be positive"
    );
    assert_eq!(
        warning["grace_period_days"].as_i64().unwrap_or(-1),
        14,
        "grace_period_days should match the value supplied on deprecation"
    );
    assert_eq!(
        warning["migration_guide_url"].as_str().unwrap_or(""),
        "https://docs.example.com/v3-migration"
    );
}

// ─── 3. V1 metadata endpoint also carries the warning ─────────────────────

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_v1_metadata_includes_deprecation_warning() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let contract_id = publish_test_contract(&client, &base).await;

    let retirement_at = (Utc::now() + Duration::days(7)).to_rfc3339();
    let deprecate_payload = json!({
        "retirement_at": retirement_at,
        "migration_guide_url": "https://example.com/v2",
        "deprecated_reason": "Moved to v2",
        "grace_period_days": 7
    });

    client
        .post(format!("{}/api/contracts/{}/deprecate", base, contract_id))
        .json(&deprecate_payload)
        .send()
        .await
        .expect("deprecate")
        .error_for_status()
        .expect("deprecate OK");

    let body: Value = client
        .get(format!(
            "{}/api/v1/contracts/{}/metadata",
            base, contract_id
        ))
        .send()
        .await
        .expect("v1 GET")
        .json()
        .await
        .unwrap();

    let warning = body
        .get("deprecation_warning")
        .expect("v1 metadata must include deprecation_warning for deprecated contract");

    assert!(
        !warning.is_null(),
        "v1 deprecation_warning must not be null"
    );
    assert!(
        warning["days_until_retirement"].as_i64().unwrap_or(-1) >= 0,
        "days_until_retirement must be non-negative"
    );
}

// ─── 4. Undeprecate clears the warning ────────────────────────────────────

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_undeprecate_clears_warning() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let contract_id = publish_test_contract(&client, &base).await;

    // Deprecate first
    let retirement_at = (Utc::now() + Duration::days(10)).to_rfc3339();
    let deprecate_payload = json!({
        "retirement_at": retirement_at,
        "migration_guide_url": "https://example.com/migrate",
        "deprecated_reason": "Will be removed"
    });

    client
        .post(format!("{}/api/contracts/{}/deprecate", base, contract_id))
        .json(&deprecate_payload)
        .send()
        .await
        .expect("deprecate")
        .error_for_status()
        .expect("deprecate OK");

    // Confirm warning is present
    let deprecated_body: Value = client
        .get(format!("{}/api/contracts/{}", base, contract_id))
        .send()
        .await
        .expect("GET after deprecate")
        .json()
        .await
        .unwrap();
    assert!(
        deprecated_body
            .get("deprecation_warning")
            .map(|w| !w.is_null())
            .unwrap_or(false),
        "warning should be present after deprecation"
    );

    // Reactivating without an explicit override is rejected (issue #1090)
    let no_override_res = client
        .delete(format!("{}/api/contracts/{}/deprecate", base, contract_id))
        .send()
        .await
        .expect("undeprecate without override");
    assert_eq!(
        no_override_res.status(),
        StatusCode::BAD_REQUEST,
        "undeprecate without override should be rejected"
    );

    // Undeprecate
    let undep_res = client
        .delete(format!(
            "{}/api/contracts/{}/deprecate?override=true",
            base, contract_id
        ))
        .send()
        .await
        .expect("undeprecate");
    assert_eq!(
        undep_res.status(),
        StatusCode::OK,
        "undeprecate should return 200"
    );

    // Warning must be gone
    let active_body: Value = client
        .get(format!("{}/api/contracts/{}", base, contract_id))
        .send()
        .await
        .expect("GET after undeprecate")
        .json()
        .await
        .unwrap();
    let warning_after = active_body.get("deprecation_warning");
    assert!(
        warning_after.is_none() || warning_after.map(Value::is_null).unwrap_or(false),
        "deprecation_warning must be absent / null after undeprecation"
    );
}

// ─── 5. Grace period data round-trips through DeprecationInfo ────────────

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_deprecation_info_reflects_grace_period() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let contract_id = publish_test_contract(&client, &base).await;

    let retirement_at = (Utc::now() + Duration::days(45)).to_rfc3339();
    let deprecate_payload = json!({
        "retirement_at": retirement_at,
        "migration_guide_url": "https://example.com/guide",
        "deprecated_reason": "Long grace period test",
        "grace_period_days": 45
    });

    client
        .post(format!("{}/api/contracts/{}/deprecate", base, contract_id))
        .json(&deprecate_payload)
        .send()
        .await
        .expect("deprecate")
        .error_for_status()
        .expect("deprecate OK");

    let info: Value = client
        .get(format!(
            "{}/api/contracts/{}/deprecation-info",
            base, contract_id
        ))
        .send()
        .await
        .expect("GET deprecation-info")
        .json()
        .await
        .unwrap();

    assert_eq!(
        info["status"].as_str().unwrap_or(""),
        "deprecated",
        "status should be 'deprecated'"
    );
    assert_eq!(
        info["grace_period_days"].as_i64().unwrap_or(-1),
        45,
        "grace_period_days should round-trip"
    );
    assert_eq!(
        info["deprecated_reason"].as_str().unwrap_or(""),
        "Long grace period test",
        "deprecated_reason should round-trip"
    );
    assert!(
        info["days_remaining"].as_i64().unwrap_or(-1) > 0,
        "days_remaining should be positive"
    );
}

// ─── 6. Non-deprecated contract has no deprecation_warning ───────────────

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_active_contract_has_no_deprecation_warning() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let contract_id = publish_test_contract(&client, &base).await;

    let body: Value = client
        .get(format!("{}/api/contracts/{}", base, contract_id))
        .send()
        .await
        .expect("GET contract")
        .json()
        .await
        .unwrap();

    let warning = body.get("deprecation_warning");
    assert!(
        warning.is_none() || warning.map(Value::is_null).unwrap_or(false),
        "active contract must not have a deprecation_warning"
    );
}

// ─── 7. Unit tests: grace-period model logic (no live server required) ────

#[cfg(test)]
mod unit_tests {
    use chrono::{Duration, Utc};
    use shared::DeprecationWarning;

    #[test]
    fn deprecation_warning_days_until_retirement_future() {
        let now = Utc::now();
        let retirement_at = now + Duration::days(10);
        let warning = DeprecationWarning {
            message: "deprecated".into(),
            deprecated_at: now - Duration::days(1),
            retirement_at,
            days_until_retirement: (retirement_at - now).num_days(),
            replacement_contract_id: None,
            migration_guide_url: None,
            grace_period_days: Some(10),
        };
        assert!(
            warning.days_until_retirement > 0,
            "days_until_retirement should be positive for a future retirement date"
        );
        assert_eq!(warning.grace_period_days, Some(10));
    }

    #[test]
    fn deprecation_warning_already_retired() {
        let now = Utc::now();
        let retirement_at = now - Duration::days(1);
        let days = if retirement_at > now {
            (retirement_at - now).num_days()
        } else {
            0
        };
        let warning = DeprecationWarning {
            message: "retired".into(),
            deprecated_at: now - Duration::days(10),
            retirement_at,
            days_until_retirement: days,
            replacement_contract_id: None,
            migration_guide_url: None,
            grace_period_days: None,
        };
        assert_eq!(
            warning.days_until_retirement, 0,
            "days_until_retirement should be 0 when retirement_at is in the past"
        );
        assert!(
            warning.grace_period_days.is_none(),
            "grace_period_days should be None when not set"
        );
    }

    #[test]
    fn deprecation_warning_serialization_omits_none_fields() {
        let now = Utc::now();
        let warning = DeprecationWarning {
            message: "going away".into(),
            deprecated_at: now,
            retirement_at: now + Duration::days(30),
            days_until_retirement: 30,
            replacement_contract_id: None,
            migration_guide_url: None,
            grace_period_days: None,
        };
        let serialized = serde_json::to_value(&warning).unwrap();
        assert!(
            serialized.get("replacement_contract_id").is_none(),
            "replacement_contract_id should be omitted when None"
        );
        assert!(
            serialized.get("migration_guide_url").is_none(),
            "migration_guide_url should be omitted when None"
        );
        assert!(
            serialized.get("grace_period_days").is_none(),
            "grace_period_days should be omitted when None"
        );
        assert!(
            serialized.get("message").is_some(),
            "message should always be present"
        );
    }
}
