// Publisher profile aggregation endpoint tests.
//
// Requires a running API and database:
//   cargo test --test publisher_profile_aggregation_tests -- --ignored

use chrono::{Duration, Utc};
use reqwest::StatusCode;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

fn api_base_url() -> String {
    std::env::var("TEST_API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

fn database_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set for publisher profile tests")
}

async fn pool() -> PgPool {
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url())
        .await
        .expect("failed to connect to the test database")
}

fn random_strkey(prefix: char) -> String {
    let raw = format!(
        "{:032X}{:032X}",
        Uuid::new_v4().as_u128(),
        Uuid::new_v4().as_u128()
    );
    format!("{}{}", prefix, &raw[..55])
}

fn contract_id_from(id: Uuid) -> String {
    let hex = id.simple().to_string().to_uppercase();
    format!("C{hex:0<55}")
}

fn wasm_hash_from(id: Uuid) -> String {
    format!("{:064x}", id.as_u128())
}

async fn create_publisher(pool: &PgPool) -> Uuid {
    sqlx::query_scalar("INSERT INTO publishers (stellar_address) VALUES ($1) RETURNING id")
        .bind(random_strkey('G'))
        .fetch_one(pool)
        .await
        .expect("create publisher")
}

async fn create_contract(
    pool: &PgPool,
    publisher_id: Uuid,
    slug_prefix: &str,
    idx: usize,
    created_at: chrono::DateTime<Utc>,
    network: &str,
    category: Option<&str>,
    verified: bool,
) -> Uuid {
    let id = Uuid::new_v4();
    let contract_id = contract_id_from(id);
    let wasm_hash = wasm_hash_from(id);
    let slug = format!("{slug_prefix}-{idx}-{}", id.simple());
    let name = format!("{slug_prefix} contract {idx}");

    sqlx::query(
        r#"
        INSERT INTO contracts
            (id, contract_id, wasm_hash, name, slug, description, publisher_id,
             network, is_verified, verification_status, category, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12)
        "#,
    )
    .bind(id)
    .bind(contract_id)
    .bind(wasm_hash.clone())
    .bind(name)
    .bind(slug)
    .bind(Some(format!("fixture {idx}")))
    .bind(publisher_id)
    .bind(network)
    .bind(verified)
    .bind(if verified { "verified" } else { "unverified" })
    .bind(category)
    .bind(created_at)
    .execute(pool)
    .await
    .expect("insert contract fixture");

    sqlx::query(
        r#"
        INSERT INTO contract_versions (contract_id, version, wasm_hash, source_url, commit_hash, release_notes)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id)
    .bind("1.0.0")
    .bind(&wasm_hash)
    .bind(Some("https://example.com/source"))
    .bind(Some(format!("{:040x}", idx + 1)))
    .bind(Some("initial release"))
    .execute(pool)
    .await
    .expect("insert version 1");

    sqlx::query(
        r#"
        INSERT INTO contract_versions (contract_id, version, wasm_hash, source_url, commit_hash, release_notes)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id)
    .bind("1.0.1")
    .bind(&wasm_hash)
    .bind(Some("https://example.com/source"))
    .bind(Some(format!("{:040x}", idx + 1001)))
    .bind(Some("patch release"))
    .execute(pool)
    .await
    .expect("insert version 2");

    id
}

async fn delete_publisher(pool: &PgPool, publisher_id: Uuid) {
    sqlx::query("DELETE FROM publishers WHERE id = $1")
        .bind(publisher_id)
        .execute(pool)
        .await
        .expect("cleanup publisher fixtures");
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn publisher_summary_handles_publishers_with_no_contracts() {
    let pool = pool().await;
    let api = reqwest::Client::new();
    let base = api_base_url();
    let publisher_id = create_publisher(&pool).await;

    let summary_res = api
        .get(format!("{base}/api/publishers/{publisher_id}/summary"))
        .send()
        .await
        .expect("call publisher summary endpoint");
    assert_eq!(summary_res.status(), StatusCode::OK);

    let summary: Value = summary_res.json().await.expect("deserialize summary");
    assert_eq!(summary["publisher"]["id"], json!(publisher_id.to_string()));
    assert_eq!(summary["contract_count"], 0);
    assert_eq!(summary["version_count"], 0);
    assert_eq!(summary["verified_contract_count"], 0);
    assert_eq!(summary["network_breakdown"], json!([]));
    assert_eq!(summary["category_breakdown"], json!([]));
    assert_eq!(summary["verification_status_breakdown"], json!([]));

    let contracts_res = api
        .get(format!("{base}/api/publishers/{publisher_id}/contracts?limit=10"))
        .send()
        .await
        .expect("call publisher contracts endpoint");
    assert_eq!(contracts_res.status(), StatusCode::OK);

    let contracts: Value = contracts_res.json().await.expect("deserialize contracts");
    assert_eq!(contracts["items"], json!([]));
    assert_eq!(contracts["total"], 0);
    assert_eq!(contracts["has_more"], false);
    assert!(contracts.get("next_cursor").is_none() || contracts["next_cursor"].is_null());

    delete_publisher(&pool, publisher_id).await;
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn publisher_summary_and_contract_pagination_scale_to_many_contracts() {
    let pool = pool().await;
    let api = reqwest::Client::new();
    let base = api_base_url();
    let publisher_id = create_publisher(&pool).await;

    let started = Utc::now();
    let mut expected = Vec::new();
    let networks = ["mainnet", "testnet", "futurenet"];
    let categories = [Some("DeFi"), Some("NFT"), Some("Infra"), None];

    for idx in 0..120usize {
        let created_at = started - Duration::seconds((idx / 3) as i64);
        let network = networks[idx % networks.len()];
        let category = categories[idx % categories.len()];
        let verified = idx % 10 == 0;
        let id = create_contract(
            &pool,
            publisher_id,
            "publisher-profile",
            idx,
            created_at,
            network,
            category,
            verified,
        )
        .await;
        expected.push((created_at, id));
    }

    expected.sort_by(|left, right| right.0.cmp(&left.0).then(right.1.cmp(&left.1)));

    let summary_res = api
        .get(format!("{base}/api/publishers/{publisher_id}/summary"))
        .send()
        .await
        .expect("call publisher summary endpoint");
    assert_eq!(summary_res.status(), StatusCode::OK);
    let summary: Value = summary_res.json().await.expect("deserialize summary");

    assert_eq!(summary["contract_count"], 120);
    assert_eq!(summary["version_count"], 240);
    assert_eq!(summary["verified_contract_count"], 12);

    let network_counts = summary["network_breakdown"]
        .as_array()
        .expect("network breakdown should be an array");
    let mut seen_networks = std::collections::BTreeMap::new();
    for row in network_counts {
        seen_networks.insert(
            row["network"].as_str().unwrap().to_string(),
            (row["contract_count"].as_i64().unwrap(), row["version_count"].as_i64().unwrap()),
        );
    }
    assert_eq!(seen_networks.get("mainnet"), Some(&(40, 80)));
    assert_eq!(seen_networks.get("testnet"), Some(&(40, 80)));
    assert_eq!(seen_networks.get("futurenet"), Some(&(40, 80)));

    let category_counts = summary["category_breakdown"]
        .as_array()
        .expect("category breakdown should be an array");
    let mut seen_categories = std::collections::BTreeMap::new();
    for row in category_counts {
        let key = row["category"].as_str().unwrap_or("null").to_string();
        seen_categories.insert(
            key,
            (row["contract_count"].as_i64().unwrap(), row["version_count"].as_i64().unwrap()),
        );
    }
    assert_eq!(seen_categories.get("DeFi"), Some(&(30, 60)));
    assert_eq!(seen_categories.get("NFT"), Some(&(30, 60)));
    assert_eq!(seen_categories.get("Infra"), Some(&(30, 60)));
    assert_eq!(seen_categories.get("null"), Some(&(30, 60)));

    let verification_status = summary["verification_status_breakdown"]
        .as_array()
        .expect("verification status breakdown should be an array");
    assert_eq!(verification_status.len(), 2);
    let verified_row = verification_status
        .iter()
        .find(|row| row["verification_status"] == "verified")
        .expect("verified row should exist");
    let unverified_row = verification_status
        .iter()
        .find(|row| row["verification_status"] == "unverified")
        .expect("unverified row should exist");
    assert_eq!(verified_row["contract_count"], 12);
    assert_eq!(unverified_row["contract_count"], 108);

    let mut seen = Vec::new();
    let mut cursor: Option<String> = None;
    let limit = 25;

    for _ in 0..10 {
        let mut request = api.get(format!("{base}/api/publishers/{publisher_id}/contracts"));
        request = request.query(&[("limit", limit.to_string())]);
        if let Some(current) = &cursor {
            request = request.query(&[("cursor", current.as_str())]);
        }

        let res = request.send().await.expect("call contracts endpoint");
        assert_eq!(res.status(), StatusCode::OK);

        let body: Value = res.json().await.expect("deserialize contracts response");
        let items = body["items"].as_array().expect("items must be an array");

        for item in items {
            seen.push((
                item["created_at"].as_str().unwrap().to_string(),
                Uuid::parse_str(item["id"].as_str().unwrap()).unwrap(),
            ));
        }

        if body["has_more"].as_bool().unwrap_or(false) {
            cursor = body["next_cursor"]
                .as_str()
                .map(|value| value.to_string());
            assert!(cursor.is_some(), "has_more pages must include a next_cursor");
        } else {
            assert!(body["next_cursor"].is_null() || body.get("next_cursor").is_none());
            break;
        }
    }

    assert_eq!(seen.len(), expected.len(), "cursor pagination must return every contract");

    let seen_ids: Vec<Uuid> = seen
        .iter()
        .map(|(_, id)| *id)
        .collect();
    let mut unique_ids = seen_ids.clone();
    unique_ids.sort();
    unique_ids.dedup();
    assert_eq!(unique_ids.len(), seen_ids.len(), "pagination returned duplicates");

    let actual_order: Vec<Uuid> = seen_ids;
    let expected_order: Vec<Uuid> = expected.into_iter().map(|(_, id)| id).collect();
    assert_eq!(actual_order, expected_order, "publisher contracts must be stable and deterministic");

    delete_publisher(&pool, publisher_id).await;
}
