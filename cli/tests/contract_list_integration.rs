// tests/contract_list_integration.rs
//
// End-to-end tests for `soroban-registry contract list`: the real binary runs
// against a mock registry, covering each output format, pagination, filters,
// error handling and the performance target.

use std::process::{Command, Output};
use std::time::Instant;

use serde_json::{json, Value};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const BINARY: &str = env!("CARGO_BIN_EXE_soroban-registry");

async fn run_cli(api_url: &str, args: &[&str]) -> Output {
    let mut command = Command::new(BINARY);
    command
        .arg("--api-url")
        .arg(api_url)
        .arg("--no-cache")
        .arg("contract")
        .arg("list");
    command.args(args);

    tokio::task::spawn_blocking(move || command.output().expect("failed to run the CLI"))
        .await
        .expect("CLI task panicked")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_str(&stdout_of(output)).unwrap_or_else(|err| {
        panic!(
            "expected JSON on stdout ({err}).\nstdout: {}\nstderr: {}",
            stdout_of(output),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// A `Contract` as the registry serializes it.
fn contract(index: usize, name: &str, category: Option<&str>) -> Value {
    json!({
        "id": format!("11111111-1111-1111-1111-{:012}", index),
        "contract_id": format!("CA{:054}", index),
        "wasm_hash": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
        "name": name,
        "slug": format!("slug-{index}"),
        "description": "a contract",
        "publisher_id": "22222222-2222-2222-2222-222222222222",
        "network": "testnet",
        "is_verified": true,
        "verification_status": "Verified",
        "category": category,
        "tags": [],
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-02-03T04:05:06Z",
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

fn page(count: usize, total: i64) -> Value {
    let items: Vec<Value> = (0..count)
        .map(|index| contract(index, &format!("contract-{index}"), Some("DeFi")))
        .collect();
    json!({
        "items": items,
        "total": total,
        "page": 1,
        "per_page": count.max(1),
        "pages": 1
    })
}

async fn registry_returning(body: Value) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/contracts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    server
}

// ── Table (default) ───────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn table_is_the_default_and_shows_the_promised_columns() {
    let server = registry_returning(page(3, 3)).await;

    let output = run_cli(&server.uri(), &[]).await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = stdout_of(&output);
    for header in ["ADDRESS", "NAME", "NETWORK", "CATEGORY", "LAST UPDATE"] {
        assert!(
            stdout.contains(header),
            "missing column {header}:\n{stdout}"
        );
    }
    assert!(stdout.contains("contract-0"), "{stdout}");
    assert!(stdout.contains("testnet"), "{stdout}");
    assert!(stdout.contains("DeFi"), "{stdout}");
    assert!(
        stdout.contains("2026-02-03 04:05:06"),
        "last update:\n{stdout}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn table_reports_the_current_page_and_total() {
    let server = registry_returning(page(10, 95)).await;

    let output = run_cli(&server.uri(), &["--limit", "10", "--offset", "20"]).await;

    let stdout = stdout_of(&output);
    assert!(
        stdout.contains("Page 3 of 10"),
        "offset 20 with limit 10 is page 3 of 10:\n{stdout}"
    );
    assert!(stdout.contains("showing 10 of 95 contract(s)"), "{stdout}");
    assert!(
        stdout.contains("--offset 30"),
        "it should show how to get the next page:\n{stdout}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_empty_registry_says_so_without_an_empty_table() {
    let server = registry_returning(page(0, 0)).await;

    let output = run_cli(&server.uri(), &[]).await;

    assert!(output.status.success());
    let stdout = stdout_of(&output);
    assert!(stdout.contains("No contracts found"), "{stdout}");
    assert!(!stdout.contains("ADDRESS"), "no header for an empty result");
}

// ── JSON ──────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_carries_the_columns_and_pagination_metadata() {
    let server = registry_returning(page(2, 42)).await;

    let output = run_cli(&server.uri(), &["--format", "json", "--limit", "2"]).await;

    let body = stdout_json(&output);
    let first = &body["contracts"][0];
    assert_eq!(first["name"], "contract-0");
    assert_eq!(first["network"], "testnet");
    assert_eq!(first["category"], "DeFi");
    assert_eq!(first["last_update"], "2026-02-03T04:05:06+00:00");
    assert!(
        first["address"].as_str().unwrap().starts_with("CA"),
        "address is the on-chain contract id"
    );

    let pagination = &body["pagination"];
    assert_eq!(pagination["count"], 2);
    assert_eq!(pagination["total"], 42);
    assert_eq!(pagination["page"], 1);
    assert_eq!(pagination["total_pages"], 21);
    assert_eq!(pagination["has_more"], true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_output_is_pipeable() {
    let server = registry_returning(page(3, 3)).await;

    let output = run_cli(&server.uri(), &["--format", "json"]).await;

    let stdout = stdout_of(&output);
    // Nothing but JSON on stdout: no banner, no ANSI escapes.
    assert!(stdout.trim_start().starts_with('{'), "{stdout}");
    assert!(
        !stdout.contains('\u{1b}'),
        "piped output must not be coloured"
    );
    serde_json::from_str::<Value>(&stdout).expect("stdout parses as one JSON document");
}

// ── CSV ───────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn csv_has_a_header_and_one_row_per_contract() {
    let server = registry_returning(page(3, 3)).await;

    let output = run_cli(&server.uri(), &["--format", "csv"]).await;

    let stdout = stdout_of(&output);
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(lines[0], "address,name,network,category,last_update");
    assert_eq!(lines.len(), 4, "header plus three rows:\n{stdout}");
    assert!(
        lines[1].contains(",contract-0,testnet,DeFi,"),
        "{}",
        lines[1]
    );
    assert!(!stdout.contains('\u{1b}'), "csv must not be coloured");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn csv_quotes_fields_that_contain_separators() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/contracts"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "items": [contract(1, "Comma, Inc \"quoted\"", Some("DeFi"))],
            "total": 1, "page": 1, "per_page": 1, "pages": 1
        })))
        .mount(&server)
        .await;

    let output = run_cli(&server.uri(), &["--format", "csv"]).await;

    let stdout = stdout_of(&output);
    assert!(
        stdout.contains("\"Comma, Inc \"\"quoted\"\"\""),
        "commas and quotes must be escaped:\n{stdout}"
    );
}

// ── Pagination and filters on the wire ────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn limit_and_offset_reach_the_registry() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/contracts"))
        .and(query_param("limit", "25"))
        .and(query_param("offset", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(25, 200)))
        .expect(1)
        .mount(&server)
        .await;

    let output = run_cli(&server.uri(), &["--limit", "25", "--offset", "50"]).await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn filters_are_normalised_before_they_are_sent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/contracts"))
        .and(query_param("networks", "testnet,mainnet"))
        .and(query_param("categories", "DeFi,NFT"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page(1, 1)))
        .expect(1)
        .mount(&server)
        .await;

    let output = run_cli(
        &server.uri(),
        &[
            "--networks",
            " TESTNET , mainnet ,testnet",
            "--category",
            "DeFi, NFT",
        ],
    )
    .await;

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_oversized_limit_is_rejected_with_a_way_forward() {
    let server = MockServer::start().await;

    let output = run_cli(&server.uri(), &["--limit", "500"]).await;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("capped at 100"), "{stderr}");
    assert!(
        stderr.contains("--offset"),
        "suggests paging instead: {stderr}"
    );
    assert!(
        server
            .received_requests()
            .await
            .unwrap_or_default()
            .is_empty(),
        "nothing should reach the registry"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unknown_format_lists_the_supported_ones() {
    let server = MockServer::start().await;

    let output = run_cli(&server.uri(), &["--format", "xml"]).await;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("table, json, csv"), "{stderr}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unknown_network_fails_before_any_request() {
    let server = MockServer::start().await;

    let output = run_cli(&server.uri(), &["--networks", "bogusnet"]).await;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid network"), "{stderr}");
    assert!(
        server
            .received_requests()
            .await
            .unwrap_or_default()
            .is_empty(),
        "nothing should reach the registry"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_registry_error_explains_what_to_do_next() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/contracts"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "code": "Unauthorized",
            "message": "Session expired"
        })))
        .mount(&server)
        .await;

    let output = run_cli(&server.uri(), &[]).await;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to list contracts"), "{stderr}");
    assert!(stderr.contains("auth login"), "suggests a fix: {stderr}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unreachable_registry_names_the_url() {
    let output = run_cli("http://127.0.0.1:9", &[]).await;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Could not reach the registry"), "{stderr}");
    assert!(stderr.contains("--api-url"), "{stderr}");
}

// ── Performance ───────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hundred_contracts_render_quickly() {
    let server = registry_returning(page(100, 100)).await;

    let started = Instant::now();
    let output = run_cli(&server.uri(), &["--limit", "100"]).await;
    let elapsed = started.elapsed();

    assert!(output.status.success());
    let stdout = stdout_of(&output);
    assert_eq!(
        stdout.lines().count(),
        103,
        "header, 100 rows, blank line, summary:\n{stdout}"
    );
    // The target is 500ms. The ceiling here is deliberately looser because this
    // measures process startup on a possibly cold, shared CI runner as well as
    // the work itself; it is a regression guard, not the benchmark.
    assert!(
        elapsed.as_millis() < 2_000,
        "listing 100 contracts took {elapsed:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closing_the_pipe_early_is_not_a_crash() {
    // `contract list --format csv | head -3` is the most ordinary thing a user
    // will do with this command. `println!` panics on a closed pipe, so without
    // explicit handling this exits 101 with "failed printing to stdout".
    let server = registry_returning(page(100, 100)).await;
    let url = server.uri();

    let output = tokio::task::spawn_blocking(move || {
        let mut child = Command::new(BINARY)
            .args(["--api-url", &url, "--no-cache", "contract", "list"])
            .args(["--limit", "100", "--format", "csv"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn the CLI");

        // Read a few lines, then drop the pipe while the CLI is still writing.
        {
            use std::io::{BufRead, BufReader};
            let stdout = child.stdout.take().expect("stdout");
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            for _ in 0..3 {
                line.clear();
                let _ = reader.read_line(&mut line);
            }
        }

        child.wait_with_output().expect("wait for the CLI")
    })
    .await
    .expect("CLI task panicked");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked"),
        "a closed pipe must not panic: {stderr}"
    );
    assert!(
        output.status.success(),
        "a closed pipe is a normal end, got {:?}: {stderr}",
        output.status.code()
    );
}
