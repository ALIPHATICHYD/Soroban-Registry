// tests/pagination_cursor_tests.rs
//
// Cursor (keyset) pagination for the search endpoints.
//
// Offset pagination skips or duplicates rows when contracts are inserted between
// paginated requests. These tests seed a known set of rows, walk them via cursor
// pagination, and insert MORE rows midway through the walk to simulate concurrent
// publishing — then assert that every pre-existing row is returned exactly once,
// with no skips and no duplicates.
//
// They need both a running API (TEST_API_BASE_URL, default http://localhost:3001)
// and a direct database connection (TEST_DATABASE_URL or DATABASE_URL) to perform
// the concurrent writes, so they are #[ignore] by default.

use std::env;

use chrono::{Duration, Utc};
use reqwest::StatusCode;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

fn api_base_url() -> String {
    env::var("TEST_API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

fn database_url() -> String {
    env::var("TEST_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set for pagination tests")
}

async fn pool() -> PgPool {
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url())
        .await
        .expect("failed to connect to the test database")
}

/// Reuse any existing publisher, or create one — contracts.publisher_id is a
/// RESTRICT foreign key, so a valid publisher must exist to insert contracts.
async fn ensure_publisher(pool: &PgPool) -> Uuid {
    if let Some(row) = sqlx::query("SELECT id FROM publishers LIMIT 1")
        .fetch_optional(pool)
        .await
        .expect("query publishers")
    {
        return row.get("id");
    }

    // stellar_address must match ^G[A-Z0-9]{55}$
    let address = format!("G{}", "A".repeat(55));
    sqlx::query_scalar("INSERT INTO publishers (stellar_address) VALUES ($1) RETURNING id")
        .bind(address)
        .fetch_one(pool)
        .await
        .expect("create publisher")
}

/// A 56-char Stellar-style contract id (`^C[A-Z0-9]{55}$`) derived from a UUID.
fn contract_id_from(id: Uuid) -> String {
    let hex = id.simple().to_string().to_uppercase(); // 32 chars, [A-F0-9]
    format!("C{hex:0<55}")
}

/// Insert one contract whose name contains `marker` (so a full-text search for
/// `marker` returns exactly this test's rows), at an explicit created_at.
async fn insert_contract(
    pool: &PgPool,
    publisher: Uuid,
    marker: &str,
    idx: usize,
    created_at: chrono::DateTime<Utc>,
) -> Uuid {
    let id = Uuid::new_v4();
    let contract_id = contract_id_from(id);
    // to_string() first: uuid's Simple Display ignores format width, so padding
    // must be applied to an owned String to reach the required 64 hex chars.
    let wasm_hash = format!("{:0<64}", id.simple().to_string());
    let slug = format!("{}-{}", marker, id.simple()); // ^[a-z0-9]+(-[a-z0-9]+)*$
    let name = format!("{marker} contract {idx}");

    sqlx::query(
        r#"
        INSERT INTO contracts
            (id, contract_id, wasm_hash, name, publisher_id, network, slug,
             category, description, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, 'testnet', $6, 'DeFi', $7, $8, $8)
        "#,
    )
    .bind(id)
    .bind(contract_id)
    .bind(wasm_hash)
    .bind(name)
    .bind(publisher)
    .bind(slug)
    .bind(format!("{marker} pagination fixture"))
    .bind(created_at)
    .execute(pool)
    .await
    .expect("insert contract fixture");

    id
}

/// Remove every fixture row this test created (identified by the marker slug).
async fn cleanup(pool: &PgPool, marker: &str) {
    sqlx::query("DELETE FROM contracts WHERE slug LIKE $1")
        .bind(format!("{marker}-%"))
        .execute(pool)
        .await
        .expect("cleanup fixtures");
}

/// A per-run marker word (lowercase alphanumeric, safe as a search term and slug
/// segment) so concurrent runs and the seed data never collide.
fn unique_marker() -> String {
    format!("zzpag{}", Uuid::new_v4().simple())
}

/// Extract the result array from either endpoint's response shape:
/// `/api/search` uses `contracts`, `/api/v1/contracts/search` uses `results`.
fn results_of<'a>(body: &'a Value, key: &str) -> &'a Vec<Value> {
    body.get(key)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("response missing `{key}` array: {body}"))
}

/// Walk every page via cursor pagination, invoking `after_first_page` once (to
/// simulate a concurrent write) after the first page is fetched. Returns the ids
/// seen, in order.
async fn walk<F, Fut>(
    client: &reqwest::Client,
    endpoint: &str,
    results_key: &str,
    marker: &str,
    limit: usize,
    mut after_first_page: F,
) -> Vec<String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let base = api_base_url();
    let mut cursor = String::new(); // empty cursor = "first page" in cursor mode
    let mut seen = Vec::new();
    let mut first = true;

    // Bound the loop defensively so a pagination bug can't hang the suite.
    for _ in 0..100 {
        let url =
            format!("{base}{endpoint}?q={marker}&limit={limit}&cursor={cursor}");
        let res = client.get(&url).send().await.expect("send paginated request");
        assert_eq!(res.status(), StatusCode::OK, "page request {url} should be 200");
        let body: Value = res.json().await.expect("deserialize page body");

        for item in results_of(&body, results_key) {
            seen.push(
                item.get("id")
                    .and_then(Value::as_str)
                    .expect("result item must have a string id")
                    .to_string(),
            );
        }

        if first {
            after_first_page().await;
            first = false;
        }

        match body.get("next_cursor").and_then(Value::as_str) {
            Some(next) if !next.is_empty() => cursor = next.to_string(),
            _ => break,
        }
    }

    seen
}

fn assert_no_skips_or_duplicates(seen: &[String], expected: &[Uuid]) {
    let mut unique = seen.to_vec();
    unique.sort();
    unique.dedup();
    assert_eq!(
        unique.len(),
        seen.len(),
        "cursor pagination returned duplicate rows: {seen:?}"
    );

    for id in expected {
        assert!(
            seen.iter().any(|s| s == &id.to_string()),
            "pre-existing row {id} was skipped across paginated requests"
        );
    }
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_cursor_pagination_covers_all_rows_without_gaps() {
    let pool = pool().await;
    let marker = unique_marker();
    let publisher = ensure_publisher(&pool).await;

    // 7 rows with strictly-decreasing, distinct created_at (all in the past).
    let base = Utc::now() - Duration::hours(1);
    let mut ids = Vec::new();
    for i in 0..7 {
        ids.push(insert_contract(&pool, publisher, &marker, i, base - Duration::seconds(i as i64)).await);
    }

    let client = reqwest::Client::new();
    let seen = walk(&client, "/api/search", "contracts", &marker, 2, || async {}).await;

    assert_no_skips_or_duplicates(&seen, &ids);
    assert_eq!(seen.len(), ids.len(), "expected exactly the 7 seeded rows");

    cleanup(&pool, &marker).await;
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_cursor_pagination_stable_under_concurrent_inserts() {
    let pool = pool().await;
    let marker = unique_marker();
    let publisher = ensure_publisher(&pool).await;

    let base = Utc::now() - Duration::hours(1);
    let mut original_ids = Vec::new();
    for i in 0..6 {
        original_ids
            .push(insert_contract(&pool, publisher, &marker, i, base - Duration::seconds(i as i64)).await);
    }

    // After the first page is read, insert 3 more rows dated "now" — newer than
    // every original and newer than the first page's cursor. Under offset
    // pagination this shifts the window and duplicates/skips rows; under keyset
    // pagination the already-passed cursor is unaffected.
    let write_pool = pool.clone();
    let write_marker = marker.clone();
    let client = reqwest::Client::new();
    let seen = walk(&client, "/api/search", "contracts", &marker, 2, || {
        let pool = write_pool.clone();
        let marker = write_marker.clone();
        async move {
            let now = Utc::now();
            for i in 100..103 {
                insert_contract(&pool, publisher, &marker, i, now).await;
            }
        }
    })
    .await;

    // Every pre-existing row appears exactly once; nothing duplicated.
    assert_no_skips_or_duplicates(&seen, &original_ids);

    cleanup(&pool, &marker).await;
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_v1_search_cursor_stable_under_concurrent_inserts() {
    let pool = pool().await;
    let marker = unique_marker();
    let publisher = ensure_publisher(&pool).await;

    let base = Utc::now() - Duration::hours(1);
    let mut original_ids = Vec::new();
    for i in 0..6 {
        original_ids
            .push(insert_contract(&pool, publisher, &marker, i, base - Duration::seconds(i as i64)).await);
    }

    let write_pool = pool.clone();
    let write_marker = marker.clone();
    let client = reqwest::Client::new();
    let seen = walk(
        &client,
        "/api/v1/contracts/search",
        "results",
        &marker,
        2,
        || {
            let pool = write_pool.clone();
            let marker = write_marker.clone();
            async move {
                let now = Utc::now();
                for i in 100..103 {
                    insert_contract(&pool, publisher, &marker, i, now).await;
                }
            }
        },
    )
    .await;

    assert_no_skips_or_duplicates(&seen, &original_ids);

    cleanup(&pool, &marker).await;
}

#[tokio::test]
#[ignore = "requires running API + database"]
async fn test_invalid_cursor_is_rejected() {
    let client = reqwest::Client::new();
    let base = api_base_url();

    for endpoint in ["/api/search", "/api/v1/contracts/search"] {
        let url = format!("{base}{endpoint}?q=token&cursor=not_valid_base64%21%21");
        let res = client.get(&url).send().await.expect("send request");
        assert_eq!(
            res.status(),
            StatusCode::BAD_REQUEST,
            "{endpoint} should reject a malformed cursor with 400"
        );
    }
}
