// tests/registry_client_integration.rs
//
// The CLI call sites migrated onto `soroban-registry-client`: list, search,
// metadata read/update, and publish. The real binary runs against a mock
// registry, so these cover what the client sends (path, query, body, headers)
// and how the typed error taxonomy reaches the user.

use std::process::{Command, Output};

use serde_json::{json, Value};
use wiremock::matchers::{body_json_string, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const BINARY: &str = env!("CARGO_BIN_EXE_soroban-registry");

/// Run the CLI against `api_url`, always bypassing the on-disk cache so one run
/// means one request.
async fn run_cli(api_url: &str, args: &[&str]) -> Output {
    let mut command = Command::new(BINARY);
    command.arg("--api-url").arg(api_url).arg("--no-cache");
    command.args(args);

    tokio::task::spawn_blocking(move || command.output().expect("failed to run the CLI"))
        .await
        .expect("CLI task panicked")
}

fn stdout_json(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).unwrap_or_else(|err| {
        panic!(
            "expected JSON on stdout ({err}).\nstdout: {stdout}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// A `Contract` as the registry serializes it.
fn contract(index: usize) -> Value {
    json!({
        "id": format!("11111111-1111-1111-1111-{:012}", index),
        "contract_id": format!("CA{:054}", index),
        "wasm_hash": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
        "name": format!("contract-{index}"),
        "slug": format!("contract-{index}"),
        "description": "a contract",
        "publisher_id": "22222222-2222-2222-2222-222222222222",
        "network": "testnet",
        "is_verified": index % 2 == 0,
        "verification_status": "unverified",
        "category": "DeFi",
        "tags": [{"id": "33333333-3333-3333-3333-333333333333", "name": "amm", "color": "#3b82f6"}],
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-02T00:00:00Z",
        "verified_at": null,
        "deployed_at": null,
        "verified_by": null,
        "verification_notes": null,
        "last_accessed_at": null,
        "health_score": 80,
        "is_maintenance": false,
        "logical_id": null,
        "network_configs": null,
        "organization_id": null,
        "visibility": "public",
        "usage_count": 3
    })
}

fn contract_page(count: usize, total: i64) -> Value {
    json!({
        "items": (0..count).map(contract).collect::<Vec<_>>(),
        "total": total,
        "page": 1,
        "per_page": count.max(1),
        "pages": 1
    })
}

// ── list ──────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_json_output_is_built_from_typed_models() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/contracts"))
        .and(query_param("limit", "2"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(contract_page(2, 9)))
        .expect(1)
        .mount(&server)
        .await;

    let output = run_cli(&server.uri(), &["list", "--limit", "2", "--format", "json"]).await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = stdout_json(&output);
    assert_eq!(body["count"], 2);
    assert_eq!(body["contracts"][0]["name"], "contract-0");
    assert_eq!(body["contracts"][0]["network"], "testnet");
    assert_eq!(body["contracts"][0]["tags"][0], "amm");
    assert_eq!(body["contracts"][0]["health_score"], 80);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_sends_normalised_filters_as_typed_query_params() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/contracts"))
        .and(query_param("networks", "testnet,mainnet"))
        .and(query_param("categories", "DeFi"))
        .respond_with(ResponseTemplate::new(200).set_body_json(contract_page(1, 1)))
        .expect(1)
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &[
            "list",
            "--networks",
            " TESTNET , mainnet ",
            "--category",
            "DeFi",
            "--format",
            "json",
        ],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_table_output_reports_the_server_total() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/contracts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(contract_page(2, 41)))
        .mount(&server)
        .await;

    let output = run_cli(&server.uri(), &["list", "--limit", "2"]).await;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Contract Registry"), "{stdout}");
    assert!(stdout.contains("of 41 contracts"), "{stdout}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_surfaces_an_unauthorized_response_clearly() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/contracts"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "code": "Unauthorized",
            "message": "Session expired",
            "request_id": "req-77"
        })))
        .mount(&server)
        .await;

    let output = run_cli(&server.uri(), &["list"]).await;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to list contracts"), "{stderr}");
    assert!(
        stderr.contains("Unauthorized") || stderr.contains("Session expired"),
        "the server's own error should reach the user: {stderr}"
    );
    assert!(
        stderr.contains("req-77"),
        "request id aids support: {stderr}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_still_rejects_an_invalid_network_locally() {
    let server = MockServer::start().await;

    let output = run_cli(&server.uri(), &["list", "--networks", "bogusnet"]).await;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid network"), "{stderr}");
    assert!(!stderr.contains("Failed to list contracts"), "{stderr}");
    assert!(
        server
            .received_requests()
            .await
            .unwrap_or_default()
            .is_empty(),
        "nothing should reach the registry"
    );
}

// ── search ────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_sends_the_query_and_filters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/contracts"))
        .and(query_param("query", "swap"))
        .and(query_param("verified_only", "true"))
        .and(query_param("categories", "DeFi"))
        .respond_with(ResponseTemplate::new(200).set_body_json(contract_page(2, 2)))
        .expect(1)
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &[
            "search",
            "swap",
            "--verified-only",
            "--category",
            "DeFi",
            "--json",
        ],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout_json(&output)["count"], 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_maps_sort_updated_onto_the_api_sort_field() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/contracts"))
        .and(query_param("sort_by", "updated_at"))
        .respond_with(ResponseTemplate::new(200).set_body_json(contract_page(1, 1)))
        .expect(1)
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &["search", "swap", "--sort", "updated", "--json"],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_reports_a_server_fault_without_losing_its_context() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/contracts"))
        .respond_with(ResponseTemplate::new(503).set_body_json(json!({
            "code": "ServiceUnavailable",
            "message": "database down"
        })))
        .mount(&server)
        .await;

    let output = run_cli(&server.uri(), &["search", "swap"]).await;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to search contracts"), "{stderr}");
    assert!(
        stderr.contains("registry unavailable") || stderr.contains("503"),
        "{stderr}"
    );
}

// ── metadata ──────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_metadata_update_reads_then_patches_with_an_idempotency_key() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/api/contracts/CA000000000000000000000000000000000000000000000000000001",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(contract(1)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PATCH"))
        .and(path(
            "/api/contracts/11111111-1111-1111-1111-000000000001/metadata",
        ))
        // The client sends the API's own request type, so unchanged fields go
        // out as explicit nulls; the handler treats null and absent alike.
        .and(body_json_string(
            r#"{"name":"renamed","description":null,"category":null,"tags":null,"user_id":null}"#,
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(contract(1)))
        .expect(1)
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &[
            "contract",
            "update",
            "CA000000000000000000000000000000000000000000000000000001",
            "--name",
            "renamed",
            "--yes",
            "--json",
        ],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let patch = server
        .received_requests()
        .await
        .expect("requests")
        .into_iter()
        .find(|request: &Request| request.method.as_str() == "PATCH")
        .expect("a PATCH was sent");
    let key = patch
        .headers
        .get("idempotency-key")
        .expect("metadata updates carry an idempotency key")
        .to_str()
        .expect("ascii");
    assert!(key.starts_with("cli-metadata-"), "{key}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_missing_contract_is_reported_as_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/contracts/CAMISSING"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "code": "NotFound",
            "message": "no such contract"
        })))
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &["contract", "update", "CAMISSING", "--name", "x", "--yes"],
    )
    .await;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Contract not found: CAMISSING"), "{stderr}");
}

// ── publish ───────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publish_posts_the_typed_request_with_an_idempotency_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/contracts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(contract(4)))
        .expect(1)
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &[
            "publish",
            "--contract-id",
            "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
            "--name",
            "swap-router",
            "--publisher",
            "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
            "--network",
            "testnet",
            "--category",
            "DeFi",
            "--tags",
            "amm,dex",
            "--skip-tests",
        ],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("published successfully"), "{stdout}");

    let request = server
        .received_requests()
        .await
        .expect("requests")
        .into_iter()
        .next()
        .expect("a POST was sent");
    let body: Value = serde_json::from_slice(&request.body).expect("json body");
    assert_eq!(
        body["contract_id"],
        "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"
    );
    assert_eq!(body["network"], "testnet");
    assert_eq!(body["tags"], json!(["amm", "dex"]));

    let key = request
        .headers
        .get("idempotency-key")
        .expect("a publish must carry an idempotency key so a retry cannot double-register")
        .to_str()
        .expect("ascii");
    assert!(key.starts_with("cli-publish-"), "{key}");
    assert!(request.headers.get("user-agent").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publish_reports_a_conflict_with_the_server_code() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/contracts"))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({
            "code": "ContractAlreadyExists",
            "message": "already registered"
        })))
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &[
            "publish",
            "--contract-id",
            "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
            "--name",
            "swap-router",
            "--publisher",
            "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
            "--skip-tests",
        ],
    )
    .await;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to publish"), "{stderr}");
    assert!(stderr.contains("ContractAlreadyExists"), "{stderr}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_failing_publish_is_attempted_once_per_run() {
    let server = MockServer::start().await;
    // Every attempt fails with a retryable status; the publish must not be
    // repeated within a single run beyond the idempotency-key allowance.
    Mock::given(method("POST"))
        .and(path("/api/contracts"))
        .respond_with(
            ResponseTemplate::new(503).set_body_json(json!({"code": "ServiceUnavailable"})),
        )
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &[
            "publish",
            "--contract-id",
            "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
            "--name",
            "swap-router",
            "--publisher",
            "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
            "--skip-tests",
        ],
    )
    .await;

    assert!(!output.status.success());
    let attempts = server.received_requests().await.unwrap_or_default().len();
    assert!(
        (1..=3).contains(&attempts),
        "a keyed publish may retry, but stays bounded: {attempts}"
    );
    let keys: Vec<String> = server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter_map(|request| {
            request
                .headers
                .get("idempotency-key")
                .and_then(|value| value.to_str().ok())
                .map(str::to_string)
        })
        .collect();
    assert_eq!(
        keys.len(),
        attempts,
        "every attempt carries the key, so the registry can dedupe"
    );
    assert!(
        keys.windows(2).all(|pair| pair[0] == pair[1]),
        "the key must not change between attempts: {keys:?}"
    );
}
