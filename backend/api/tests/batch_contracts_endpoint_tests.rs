use reqwest::StatusCode;
use serde_json::{json, Value};
use std::time::Instant;

fn api_base_url() -> String {
    std::env::var("TEST_API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

fn performance_threshold_ms() -> u128 {
    std::env::var("BATCH_PERF_THRESHOLD_MS")
        .ok()
        .and_then(|v| v.parse::<u128>().ok())
        .unwrap_or(500)
}

#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn batch_endpoint_returns_ordered_results_under_threshold() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let list_res = client
        .get(format!("{}/api/contracts?limit=50", base))
        .send()
        .await
        .expect("failed to call list contracts endpoint");

    assert_eq!(
        list_res.status(),
        StatusCode::OK,
        "list contracts must return 200"
    );

    let list_body: Value = list_res
        .json()
        .await
        .expect("failed to deserialize list contracts response");

    let items = list_body
        .get("items")
        .and_then(Value::as_array)
        .expect("list contracts response missing items array");

    assert!(
        items.len() >= 50,
        "expected at least 50 contracts in test dataset, found {}",
        items.len()
    );

    let mut requested_ids: Vec<String> = items
        .iter()
        .take(50)
        .map(|item| {
            item.get("id")
                .and_then(Value::as_str)
                .expect("contract item missing id")
                .to_string()
        })
        .collect();

    requested_ids.push("00000000-0000-0000-0000-000000000000".to_string());

    let started = Instant::now();
    let batch_res = client
        .post(format!(
            "{}/api/contracts/batch?fields=id,name,address",
            base
        ))
        .json(&requested_ids)
        .send()
        .await
        .expect("failed to call batch contracts endpoint");
    let elapsed_ms = started.elapsed().as_millis();

    assert_eq!(
        batch_res.status(),
        StatusCode::OK,
        "batch endpoint must return 200"
    );

    let threshold = performance_threshold_ms();
    assert!(
        elapsed_ms < threshold,
        "batch request took {}ms (threshold={}ms)",
        elapsed_ms,
        threshold
    );

    let results: Vec<Option<Value>> = batch_res
        .json()
        .await
        .expect("failed to deserialize batch contracts response");

    assert_eq!(
        results.len(),
        requested_ids.len(),
        "response must match request length"
    );

    for i in 0..50 {
        let row = results[i]
            .as_ref()
            .expect("existing contract id should not yield null result");

        assert_eq!(
            row.get("id").and_then(Value::as_str),
            Some(requested_ids[i].as_str()),
            "response order mismatch at index {}",
            i
        );

        assert!(
            row.get("name").is_some(),
            "field filtering should include requested field 'name'"
        );
        assert!(
            row.get("address").is_some(),
            "field filtering should include requested alias field 'address'"
        );
        assert!(
            row.get("contract_id").is_none(),
            "field filtering should exclude non-requested fields"
        );
    }

    assert!(
        results.last().and_then(|v| v.as_ref()).is_none(),
        "missing contract should return null in response array"
    );
}

#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn batch_endpoint_rejects_more_than_100_ids() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let body: Vec<String> = (0..101)
        .map(|i| format!("00000000-0000-0000-0000-{:012}", i))
        .collect();

    let res = client
        .post(format!("{}/api/contracts/batch", base))
        .json(&body)
        .send()
        .await
        .expect("failed to call batch contracts endpoint");

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let err: Value = res
        .json()
        .await
        .unwrap_or_else(|_| json!({ "message": "missing" }));

    let msg = err
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    assert!(
        msg.contains("100") || msg.contains("batch"),
        "unexpected validation error payload: {}",
        err
    );
}
