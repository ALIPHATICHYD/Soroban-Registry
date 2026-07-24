// tests/search_filter_tests.rs
//
// Issue #948: Add unified contract search filters for network, category, and verification.
// Verifies mixed filter combinations and empty result sets on /api/contracts.

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

    // Check that response filters metadata is populated correctly
    let filters = body
        .get("filters")
        .expect("Response must include active filter metadata");
    assert!(
        filters
            .get("verified_only")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "verified_only should be true in response metadata"
    );
    assert_eq!(
        filters.get("query").and_then(Value::as_str),
        Some("token"),
        "query should match search term in response metadata"
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

// ─────────────────────────────────────────────────────────────────────────────
// Consistency between /api/contracts (list) and /api/v1/contracts/search.
//
// Regression coverage for network/category filters drifting between callers:
// the CLI sends networks comma-joined, which the search endpoint previously
// parsed as one literal value, silently dropping the filter and returning
// unfiltered results.
// ─────────────────────────────────────────────────────────────────────────────

/// The list and search endpoints must accept the same filter spellings. The CLI
/// sends networks comma-joined, so both must parse that form (and tolerate
/// surrounding whitespace and duplicate values) rather than one silently
/// treating `"mainnet,testnet"` as a single unknown value.
#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn test_both_endpoints_accept_the_same_filter_spellings() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let spellings = [
        "mainnet,testnet",
        // whitespace around separators
        "mainnet, testnet",
        // repeated values collapse rather than erroring
        "mainnet,mainnet,testnet",
        // trailing separator / blank entry
        "mainnet,testnet,",
    ];

    for spelling in spellings {
        for url in [
            format!("{}/api/contracts?networks={}", base, spelling),
            format!(
                "{}/api/v1/contracts/search?q=token&networks={}",
                base, spelling
            ),
        ] {
            let res = client
                .get(&url)
                .send()
                .await
                .expect("filter request failed");

            assert_eq!(
                res.status(),
                StatusCode::OK,
                "network filter `{spelling}` should be accepted by {url}"
            );
        }
    }

    // Categories must be split the same way on both endpoints.
    for url in [
        format!("{}/api/contracts?categories=DeFi,NFT", base),
        format!(
            "{}/api/v1/contracts/search?q=token&categories=DeFi,NFT",
            base
        ),
    ] {
        let res = client
            .get(&url)
            .send()
            .await
            .expect("category filter request failed");

        assert_eq!(
            res.status(),
            StatusCode::OK,
            "comma-separated categories should be accepted by {url}"
        );
    }
}

/// Search results must respect network and category filters simultaneously.
#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn test_search_respects_both_network_and_category_filters() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let url = format!(
        "{}/api/v1/contracts/search?q=token&networks=testnet&categories=DeFi",
        base
    );

    let res = client
        .get(&url)
        .send()
        .await
        .expect("combined filter search request failed");

    assert_eq!(
        res.status(),
        StatusCode::OK,
        "combined network + category search should return 200 OK"
    );

    let body: Value = res
        .json()
        .await
        .expect("failed to deserialize search response");

    let results = body
        .get("results")
        .and_then(Value::as_array)
        .expect("search response must include a results array");

    // Vacuously true on an empty dataset, but pins the contract once data exists.
    for hit in results {
        assert_eq!(
            hit.get("network").and_then(Value::as_str),
            Some("testnet"),
            "every hit must match the requested network filter: {hit}"
        );
        assert_eq!(
            hit.get("category").and_then(Value::as_str),
            Some("DeFi"),
            "every hit must match the requested category filter: {hit}"
        );
    }
}

/// Invalid filter values must fail clearly instead of being silently dropped.
/// Previously an unparseable network left the filter empty, which returned
/// every contract rather than reporting the bad input.
#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn test_invalid_network_filter_fails_clearly() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    for bad in ["not_a_network", "mainnet,not_a_network"] {
        // Both endpoints must reject the same bad input the same way.
        for url in [
            format!("{}/api/contracts?networks={}", base, bad),
            format!("{}/api/v1/contracts/search?q=token&networks={}", base, bad),
        ] {
            let res = client
                .get(&url)
                .send()
                .await
                .expect("invalid network filter request failed");

            assert_eq!(
                res.status(),
                StatusCode::BAD_REQUEST,
                "invalid network filter `{bad}` must be rejected by {url}, not ignored"
            );

            let body = res.text().await.unwrap_or_default();
            assert!(
                body.contains("not_a_network"),
                "error from {url} should name the offending value, got: {body}"
            );
        }
    }
}

/// Pagination must stay stable while filters are applied: the total should not
/// move between pages and no contract should appear on two pages.
#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn test_filtered_pagination_is_stable() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let page = |offset: i64| {
        let url = format!(
            "{}/api/v1/contracts/search?q=token&networks=mainnet,testnet&categories=DeFi&limit=5&offset={}",
            base, offset
        );
        let client = client.clone();
        async move {
            client
                .get(&url)
                .send()
                .await
                .expect("paginated search request failed")
                .json::<Value>()
                .await
                .expect("failed to deserialize paginated response")
        }
    };

    let first = page(0).await;
    let second = page(5).await;

    assert_eq!(
        first.get("total").and_then(Value::as_i64),
        second.get("total").and_then(Value::as_i64),
        "total must not change between pages of the same filtered query"
    );

    let ids = |body: &Value| -> Vec<String> {
        body.get("results")
            .and_then(Value::as_array)
            .map(|results| {
                results
                    .iter()
                    .filter_map(|hit| hit.get("id").and_then(Value::as_str))
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };

    let first_ids = ids(&first);
    for id in ids(&second) {
        assert!(
            !first_ids.contains(&id),
            "contract {id} appeared on two pages of the same filtered query"
        );
    }
}
