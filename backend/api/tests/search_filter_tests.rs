// tests/search_filter_tests.rs
//
// Issue #989: Fix contract search filter parsing for network and category.
// Verifies mixed filter combinations, normalized comma-separated filters,
// and consistent param naming on /api/contracts.

use reqwest::StatusCode;
use serde_json::Value;

fn api_base_url() -> String {
    std::env::var("TEST_API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn test_mixed_filter_combinations_and_empty_results() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    // 1. Test a mixed filter combination (category + network + verified_only + query)
    //    Uses plural param names with comma-separated values
    let mixed_url = format!(
        "{}/api/contracts?networks=testnet&categories=DeFi&verified_only=true&query=token",
        base
    );
    let res = client
        .get(&mixed_url)
        .send()
        .await
        .expect("Failed to call contracts list with mixed filters");

    assert_eq!(
        res.status(),
        StatusCode::OK,
        "Mixed filter request should return 200 OK"
    );

    let body: Value = res
        .json()
        .await
        .expect("Failed to deserialize response body");

    // Check pagination metadata structure
    assert!(body.get("items").is_some(), "Response must include items");
    assert!(
        body.get("total").is_some(),
        "Response must include total count"
    );

    // 2. Test empty result set (filtering with something that doesn't exist)
    let empty_url = format!(
        "{}/api/contracts?networks=mainnet&query=nonexistent_contract_name_search_12345",
        base
    );
    let res_empty = client
        .get(&empty_url)
        .send()
        .await
        .expect("Failed to call contracts list with empty criteria");

    assert_eq!(
        res_empty.status(),
        StatusCode::OK,
        "Request resulting in empty set should return 200 OK"
    );

    let body_empty: Value = res_empty
        .json()
        .await
        .expect("Failed to deserialize empty response body");

    let items = body_empty
        .get("items")
        .and_then(Value::as_array)
        .expect("Response must include items array");

    let total = body_empty
        .get("total")
        .and_then(Value::as_i64)
        .expect("Response must include total count");

    assert_eq!(
        items.len(),
        0,
        "Expected empty result set items array length to be 0"
    );
    assert_eq!(total, 0, "Expected empty result set total count to be 0");
}

#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn test_network_filter_singular_and_plural() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    // Singular network param (backward compatibility)
    let singular = client
        .get(format!("{}/api/contracts?network=testnet", base))
        .send()
        .await
        .expect("Singular network param request failed");
    assert_eq!(singular.status(), StatusCode::OK);

    // Plural networks param with comma-separated
    let plural_comma = client
        .get(format!("{}/api/contracts?networks=testnet,mainnet", base))
        .send()
        .await
        .expect("Plural comma-separated networks request failed");
    assert_eq!(plural_comma.status(), StatusCode::OK);

    // Plural networks param with repeated params
    let plural_repeated = client
        .get(format!("{}/api/contracts?networks=testnet&networks=mainnet", base))
        .send()
        .await
        .expect("Plural repeated networks request failed");
    assert_eq!(plural_repeated.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn test_category_filter_singular_and_plural() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    // Singular category param (backward compatibility)
    let singular = client
        .get(format!("{}/api/contracts?category=DeFi", base))
        .send()
        .await
        .expect("Singular category param request failed");
    assert_eq!(singular.status(), StatusCode::OK);

    // Plural categories param with comma-separated values
    let plural_comma = client
        .get(format!("{}/api/contracts?categories=DeFi,NFT", base))
        .send()
        .await
        .expect("Plural comma-separated categories request failed");
    assert_eq!(plural_comma.status(), StatusCode::OK);

    // Plural categories param with repeated params
    let plural_repeated = client
        .get(format!("{}/api/contracts?categories=DeFi&categories=NFT", base))
        .send()
        .await
        .expect("Plural repeated categories request failed");
    assert_eq!(plural_repeated.status(), StatusCode::OK);
}

#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn test_combined_network_and_category_filters_normalized() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    // Combined: network singular + category comma-separated
    let mixed = client
        .get(format!(
            "{}/api/contracts?network=testnet&categories=DeFi,Lending",
            base
        ))
        .send()
        .await
        .expect("Mixed singular network + comma categories request failed");
    assert_eq!(mixed.status(), StatusCode::OK);

    // Combined: networks comma-separated + category singular
    let mixed2 = client
        .get(format!(
            "{}/api/contracts?networks=testnet,mainnet&category=NFT",
            base
        ))
        .send()
        .await
        .expect("Mixed comma networks + singular category request failed");
    assert_eq!(mixed2.status(), StatusCode::OK);

    // Combined: All plural with filters + pagination
    let mixed3 = client
        .get(format!(
            "{}/api/contracts?networks=testnet&categories=DeFi&verified_only=true&limit=10&offset=0",
            base
        ))
        .send()
        .await
        .expect("Combined filters with pagination request failed");
    assert_eq!(mixed3.status(), StatusCode::OK);

    let body3: Value = mixed3
        .json()
        .await
        .expect("Failed to deserialize combined filter response");
    assert!(body3.get("items").is_some(), "Response must include items");
    assert!(
        body3.get("total").is_some(),
        "Response must include total count"
    );
}

#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn test_invalid_filter_values_fail_clearly() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    // Invalid network value should return 400 Bad Request
    let invalid_network = client
        .get(format!("{}/api/contracts?networks=unknown_network_xyz", base))
        .send()
        .await
        .expect("Invalid network request failed");

    assert!(
        invalid_network.status().is_client_error(),
        "Invalid network value should produce a client error, got {}",
        invalid_network.status()
    );
}

#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn test_filtered_pagination_remains_stable() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    // Fetch first page with network filter
    let page1 = client
        .get(format!(
            "{}/api/contracts?networks=testnet&limit=5&offset=0",
            base
        ))
        .send()
        .await
        .expect("Page 1 request failed");
    assert_eq!(page1.status(), StatusCode::OK);

    let body1: Value = page1.json().await.expect("Failed to parse page 1");
    let total1 = body1["total"].as_i64().unwrap_or(0);
    let items1 = body1["items"].as_array().map(|a| a.len()).unwrap_or(0);

    // Fetch second page
    let page2 = client
        .get(format!(
            "{}/api/contracts?networks=testnet&limit=5&offset=5",
            base
        ))
        .send()
        .await
        .expect("Page 2 request failed");
    assert_eq!(page2.status(), StatusCode::OK);

    let body2: Value = page2.json().await.expect("Failed to parse page 2");
    let total2 = body2["total"].as_i64().unwrap_or(0);

    // Total count should be consistent across pages
    assert_eq!(
        total1, total2,
        "Total contract count should be stable across paginated requests"
    );

    // Total must be >= items returned
    assert!(
        total1 >= items1 as i64,
        "Total should be >= items returned on page"
    );
}
