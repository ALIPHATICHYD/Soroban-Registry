use reqwest::StatusCode;
use serde_json::{json, Value};

fn api_base_url() -> String {
    std::env::var("TEST_API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn batch_info_v1_endpoint_success_and_fields_filtering() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let list_res = client
        .get(format!("{}/api/contracts?limit=10", base))
        .send()
        .await
        .expect("failed to call list contracts endpoint");

    assert_eq!(list_res.status(), StatusCode::OK);
    let list_body: Value = list_res
        .json()
        .await
        .expect("failed to parse list response");
    let items = list_body
        .get("items")
        .and_then(Value::as_array)
        .expect("missing items");

    assert!(
        !items.is_empty(),
        "expected at least one contract in database"
    );

    let mut requested_ids: Vec<String> = items
        .iter()
        .take(5)
        .map(|item| {
            item.get("id")
                .and_then(Value::as_str)
                .expect("missing id")
                .to_string()
        })
        .collect();

    let missing_uuid = "11111111-1111-1111-1111-111111111111".to_string();
    let missing_address = "CAS3JOSJHDBZ22222222222222222222222222222222222222222222".to_string();
    requested_ids.push(missing_uuid.clone());
    requested_ids.push(missing_address.clone());

    let batch_res = client
        .post(format!(
            "{}/api/v1/contracts/batch-info?fields=id,name,address",
            base
        ))
        .json(&requested_ids)
        .send()
        .await
        .expect("failed to call batch info endpoint");

    assert_eq!(batch_res.status(), StatusCode::OK);

    let res_body: Value = batch_res
        .json()
        .await
        .expect("failed to parse response body");

    let contracts = res_body
        .get("contracts")
        .and_then(Value::as_array)
        .expect("missing contracts array");
    let missing = res_body
        .get("missing")
        .and_then(Value::as_array)
        .expect("missing missing array");

    let missing_str_vec: Vec<String> = missing
        .iter()
        .map(|v| v.as_str().expect("missing item not a string").to_string())
        .collect();

    assert!(missing_str_vec.contains(&missing_uuid));
    assert!(missing_str_vec.contains(&missing_address));

    assert!(!contracts.is_empty());
    for contract in contracts {
        assert!(contract.get("id").is_some(), "id field should be present");
        assert!(
            contract.get("name").is_some(),
            "name field should be present"
        );
        assert!(
            contract.get("address").is_some(),
            "address field should be present"
        );
        assert!(contract.get("status").is_none());
    }
}

#[tokio::test]
#[ignore = "requires running API + database with contract data"]
async fn batch_info_v1_endpoint_rejects_more_than_100_ids() {
    let base = api_base_url();
    let client = reqwest::Client::new();

    let body: Vec<String> = (0..101)
        .map(|i| format!("00000000-0000-0000-0000-{:012}", i))
        .collect();

    let res = client
        .post(format!("{}/api/v1/contracts/batch-info", base))
        .json(&body)
        .send()
        .await
        .expect("failed to call batch-info endpoint");

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
