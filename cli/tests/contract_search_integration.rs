// tests/contract_search_integration.rs
//
// End-to-end tests for `soroban-registry contract search`: the real binary is
// run against a mock registry, so these cover the whole path — flag parsing,
// pagination mode selection, continuation handling, safety bounds and the JSON
// pagination metadata.

use std::process::{Command, Output};

use serde_json::{json, Value};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const BINARY: &str = env!("CARGO_BIN_EXE_soroban-registry");

/// Run the CLI against `api_url` and return its output.
async fn run_cli(api_url: &str, args: &[&str]) -> Output {
    let mut command = Command::new(BINARY);
    command
        .arg("--api-url")
        .arg(api_url)
        .arg("contract")
        .arg("search");
    command.args(args);

    // The mock server needs the runtime while the child process runs, so do the
    // blocking wait off the async worker.
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

fn hit(name: &str) -> Value {
    json!({
        "id": format!("11111111-1111-1111-1111-1111111111{:02}", name.len()),
        "contract_id": format!("CA{}", name.to_uppercase()),
        "name": name,
        "description": format!("{name} contract"),
        "category": "DeFi",
        "network": "testnet",
        "is_verified": true,
        "is_deprecated": false,
        "deprecation_status": "active",
        "relevance_score": 0.5
    })
}

// ── Flag surface ──────────────────────────────────────────────────────────────

#[test]
fn help_documents_the_pagination_flags() {
    let output = Command::new(BINARY)
        .args(["contract", "search", "--help"])
        .output()
        .expect("failed to run the CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for flag in [
        "--all",
        "--max-items",
        "--max-pages",
        "--cursor",
        "--offset",
        "--pagination",
        "--json",
    ] {
        assert!(stdout.contains(flag), "help should document {flag}");
    }
}

#[test]
fn a_cursor_and_an_offset_cannot_be_passed_together() {
    let output = Command::new(BINARY)
        .args([
            "contract", "search", "swap", "--cursor", "token", "--offset", "20",
        ])
        .output()
        .expect("failed to run the CLI");

    assert!(
        !output.status.success(),
        "mixed pagination must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with"),
        "unexpected stderr: {stderr}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unknown_network_filter_fails_before_any_request() {
    let server = MockServer::start().await;

    let output = run_cli(
        &server.uri(),
        &["swap", "--networks", "testnet,not-a-network", "--json"],
    )
    .await;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not-a-network"),
        "unexpected stderr: {stderr}"
    );
    assert!(
        server
            .received_requests()
            .await
            .unwrap_or_default()
            .is_empty(),
        "no request should reach the registry"
    );
}

// ── Single page (unchanged one-page behaviour) ─────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_single_page_search_makes_one_offset_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/contracts/search"))
        .and(query_param("offset", "0"))
        .and(query_param("limit", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "query": "swap",
            "total": 5,
            "limit": 2,
            "offset": 0,
            "took_ms": 3,
            "backend": "elasticsearch",
            "results": [hit("aa"), hit("bb")],
            "facets": {"categories": [], "networks": [], "tags": []}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let output = run_cli(&server.uri(), &["swap", "--limit", "2", "--json"]).await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = stdout_json(&output);
    assert_eq!(body["count"], 2);
    assert_eq!(body["pagination"]["mode"], "offset");
    assert_eq!(body["pagination"]["pages_fetched"], 1);
    assert_eq!(body["pagination"]["total"], 5);
    assert_eq!(body["pagination"]["complete"], false);
    assert_eq!(body["pagination"]["stop_reason"], "single_page");
    assert_eq!(body["pagination"]["next_offset"], 2);
    assert!(body["pagination"]["next_cursor"].is_null());
    assert!(
        body["pagination"]["max_items"].is_null(),
        "bounds only apply to --all"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cursor_page_can_be_resumed_from_a_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("cursor", "tok-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [hit("cc")],
            "total": 3,
            "took_ms": 1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let output = run_cli(&server.uri(), &["swap", "--cursor", "tok-2", "--json"]).await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = stdout_json(&output);
    assert_eq!(body["count"], 1);
    assert_eq!(body["pagination"]["mode"], "cursor");
    assert_eq!(body["pagination"]["complete"], true);
    assert_eq!(body["pagination"]["stop_reason"], "exhausted");
}

// ── `--all` walks ─────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn all_walks_every_page_to_the_end_of_the_result_set() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("cursor", ""))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [hit("aa"), hit("bb")],
            "total": 3,
            "next_cursor": "tok-2"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("cursor", "tok-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [hit("cc")],
            "total": 3
        })))
        .mount(&server)
        .await;

    let output = run_cli(&server.uri(), &["swap", "--all", "--limit", "2", "--json"]).await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = stdout_json(&output);
    let names: Vec<&str> = body["contracts"]
        .as_array()
        .expect("contracts array")
        .iter()
        .map(|hit| hit["name"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        names,
        vec!["aa", "bb", "cc"],
        "pages arrive in server order"
    );
    assert_eq!(body["count"], 3);
    assert_eq!(
        body["pagination"]["mode"], "cursor",
        "--all defaults to cursor pagination"
    );
    assert_eq!(body["pagination"]["pages_fetched"], 2);
    assert_eq!(body["pagination"]["total"], 3);
    assert_eq!(body["pagination"]["complete"], true);
    assert_eq!(body["pagination"]["stop_reason"], "exhausted");
    assert!(body["pagination"]["next_cursor"].is_null());
    assert_eq!(
        body["pagination"]["max_items"], 1000,
        "--all is bounded by default"
    );
    assert_eq!(body["pagination"]["max_pages"], 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn all_stops_at_max_items_and_reports_where_to_resume() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("cursor", ""))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [hit("aa"), hit("bb")],
            "total": 500,
            "next_cursor": "tok-2"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("cursor", "tok-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [hit("cc")],
            "total": 500,
            "next_cursor": "tok-3"
        })))
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &[
            "swap",
            "--all",
            "--limit",
            "2",
            "--max-items",
            "3",
            "--json",
        ],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = stdout_json(&output);
    assert_eq!(body["count"], 3, "the item bound is respected exactly");
    assert_eq!(body["pagination"]["stop_reason"], "max_items");
    assert_eq!(body["pagination"]["complete"], false);
    assert_eq!(body["pagination"]["total"], 500);
    assert_eq!(body["pagination"]["max_items"], 3);
    assert_eq!(
        body["pagination"]["next_cursor"], "tok-3",
        "a bounded walk says where to resume"
    );
    assert_eq!(
        server.received_requests().await.unwrap_or_default().len(),
        2,
        "no page is fetched beyond the bound"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn all_stops_at_max_pages() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("cursor", ""))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [hit("aa")],
            "total": 500,
            "next_cursor": "tok-2"
        })))
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &[
            "swap",
            "--all",
            "--limit",
            "1",
            "--max-pages",
            "1",
            "--json",
        ],
    )
    .await;

    assert!(output.status.success());
    let body = stdout_json(&output);
    assert_eq!(body["count"], 1);
    assert_eq!(body["pagination"]["pages_fetched"], 1);
    assert_eq!(body["pagination"]["stop_reason"], "max_pages");
    assert_eq!(body["pagination"]["max_pages"], 1);
    assert_eq!(body["pagination"]["next_cursor"], "tok-2");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_repeated_cursor_fails_instead_of_looping() {
    let server = MockServer::start().await;
    // Every request answers with the same continuation token.
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [hit("aa")],
            "total": 500,
            "next_cursor": "stuck"
        })))
        .mount(&server)
        .await;

    let output = run_cli(&server.uri(), &["swap", "--all", "--limit", "1", "--json"]).await;

    assert!(
        !output.status.success(),
        "a looping server must fail loudly"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("repeated a pagination cursor"),
        "unexpected stderr: {stderr}"
    );
    // Two requests: the first page, then the one that revealed the repeat.
    assert_eq!(
        server.received_requests().await.unwrap_or_default().len(),
        2
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_malformed_cursor_reports_the_registry_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "code": "INVALID_CURSOR",
            "message": "The provided pagination cursor is invalid"
        })))
        .mount(&server)
        .await;

    let output = run_cli(&server.uri(), &["swap", "--cursor", "garbage", "--json"]).await;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("INVALID_CURSOR"),
        "unexpected stderr: {stderr}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn empty_pages_with_a_continuation_token_do_not_loop_forever() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("cursor", ""))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [],
            "total": 9,
            "next_cursor": "tok-a"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("cursor", "tok-a"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [],
            "total": 9,
            "next_cursor": "tok-b"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("cursor", "tok-b"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [],
            "total": 9,
            "next_cursor": "tok-c"
        })))
        .mount(&server)
        .await;

    let output = run_cli(&server.uri(), &["swap", "--all", "--limit", "2", "--json"]).await;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("consecutive empty page"),
        "unexpected stderr: {stderr}"
    );
    assert_eq!(
        server.received_requests().await.unwrap_or_default().len(),
        3,
        "the walk stops rather than requesting more empty pages"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn filters_are_normalised_and_repeated_on_every_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("networks", "testnet,mainnet"))
        .and(query_param("categories", "DeFi,NFT"))
        .and(query_param("tags", "amm"))
        .and(query_param("verified_only", "true"))
        .and(query_param("cursor", ""))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [hit("aa")],
            "total": 2,
            "next_cursor": "tok-2"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("networks", "testnet,mainnet"))
        .and(query_param("categories", "DeFi,NFT"))
        .and(query_param("tags", "amm"))
        .and(query_param("verified_only", "true"))
        .and(query_param("cursor", "tok-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [hit("bb")],
            "total": 2
        })))
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &[
            "swap",
            "--all",
            "--limit",
            "1",
            "--networks",
            " TESTNET , mainnet ,testnet",
            "--category",
            "DeFi, NFT",
            "--tags",
            "amm",
            "--verified-only",
            "--json",
        ],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body = stdout_json(&output);
    assert_eq!(body["count"], 2);
    assert_eq!(body["pagination"]["complete"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_explicit_pagination_mode_overrides_the_default() {
    let server = MockServer::start().await;
    // Cursor mode forced for a single page: the full-text endpoint is used and an
    // empty cursor opens the walk.
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("cursor", ""))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [hit("aa")],
            "total": 1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &["swap", "--pagination", "cursor", "--limit", "1", "--json"],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout_json(&output)["pagination"]["mode"], "cursor");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn human_output_shows_pagination_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .and(query_param("cursor", ""))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contracts": [hit("aa")],
            "total": 42,
            "next_cursor": "tok-2"
        })))
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &["swap", "--all", "--limit", "1", "--max-items", "1"],
    )
    .await;

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("of 42 match(es)"), "stdout: {stdout}");
    assert!(stdout.contains("cursor pagination"), "stdout: {stdout}");
    assert!(stdout.contains("--max-items bound (1)"), "stdout: {stdout}");
    assert!(stdout.contains("--cursor tok-2"), "stdout: {stdout}");
}
