// tests/dependency_graph_tests.rs
//
// Issue #1147 — Contract dependency graphs and transitive risk analysis.
//
// These are live-HTTP tests: they seed a known graph directly in Postgres, then
// drive the real endpoints. That combination is deliberate. The interesting
// behaviour of this feature lives in a recursive CTE and in SQL's three-valued
// logic around the tenancy predicate, and neither is exercised by a pure test.
// The pure rules -- severity table, combinator, dedup, ordering, version
// conflicts -- are covered by `cargo test -p shared`, where they run with no
// database at all.
//
// To run:
//   docker run -d --name sr-test-pg -e POSTGRES_PASSWORD=postgres \
//     -e POSTGRES_DB=soroban_registry -p 5432:5432 postgres:16
//   export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/soroban_registry"
//   export JWT_SECRET="test-jwt-secret-at-least-32-characters-long"
//   export ELASTICSEARCH_URL="http://localhost:9200"
//   cargo run --bin api &            # migrations run on startup
//   export TEST_API_BASE_URL="http://localhost:3001"
//   cargo test --test dependency_graph_tests -- --ignored --test-threads=1
//
// The suite issues enough requests to trip the default per-IP rate limit. Start
// the API with a raised limit to keep runs clean:
//   export RATE_LIMIT_IP_PER_MINUTE=100000
//   export RATE_LIMIT_ANON_PER_MINUTE=100000
//
// `--test-threads=1` is required: every test shares one seeded fixture keyed by
// the `sr1147-` slug prefix, and they mutate visibility and edges.

use std::env;

use reqwest::StatusCode;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

fn api_base_url() -> String {
    env::var("TEST_API_BASE_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

fn database_url() -> String {
    env::var("TEST_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set for dependency graph tests")
}

async fn pool() -> PgPool {
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url())
        .await
        .expect("failed to connect to the test database")
}

// ── Fixture ─────────────────────────────────────────────────────────────────

const PUBLISHER: Uuid = Uuid::from_u128(0x1147_0000_0000_0000_0000_0000_0000_0001);

/// Distinctive fixture address. `publishers.stellar_address` is UNIQUE, so a
/// generic all-A address would collide with whatever else happens to be in a
/// shared test database.
fn publisher_address() -> String {
    format!("GSR1147{}", "0".repeat(49))
}

fn contract_uuid(tag: u8) -> Uuid {
    Uuid::from_u128(0x1147_0000_0000_0000_0000_0000_0000_0100 + tag as u128)
}

/// Deterministic 56-character `C...` strkey-shaped id. Not a real strkey (no
/// checksum), but `validate_contract_id` only checks shape, and using real
/// addresses would make the fixtures unreadable.
fn address(letter: char) -> String {
    format!("C{}", letter.to_string().repeat(55).to_uppercase())
}

/// `contracts.wasm_hash` is CHECK-constrained to 64 hex characters, so the
/// fixture letter is folded into the hex alphabet rather than used directly.
fn wasm_hash(letter: char) -> String {
    let hex = match letter {
        'a'..='f' => letter,
        other => char::from_digit((other as u32 - 'a' as u32) % 10, 10).unwrap_or('0'),
    };
    hex.to_string().repeat(64)
}

/// Seed the graph these tests share:
///
/// ```text
///   A -> B -> C -> D -> A      (a 4-cycle)
///   A -> D                     (diamond: D is reachable at depth 1 and depth 3)
///   A -> "some-library"        (unresolved, not an address)
///   C -> CZZZ...               (unresolved, a well-formed but unregistered address)
///   B also exists on mainnet   (so a bare address for B is ambiguous)
///   E                          (isolated, signed, no findings)
/// ```
async fn seed(pool: &PgPool) {
    // Clear child rows first. `contract_dependency_edges` cascades with its
    // contracts, but `contract_versions` and the signature/scan tables use
    // RESTRICT, so a previous run's rows would block the delete.
    //
    // Matched by the same slug pattern as the contracts themselves rather than
    // by this run's ids: a fixture left behind by an interrupted run has
    // different ids but the same slugs, and would otherwise wedge every
    // subsequent run.
    for table in [
        "security_issues",
        "security_scans",
        "package_signatures",
        "contract_versions",
        "contract_deprecations",
    ] {
        sqlx::query(&format!(
            "DELETE FROM {table} WHERE contract_id IN
                (SELECT id FROM contracts WHERE slug LIKE 'sr1147-%')"
        ))
        .execute(pool)
        .await
        .unwrap_or_else(|e| panic!("clear {table}: {e}"));
    }

    sqlx::query("DELETE FROM contracts WHERE slug LIKE 'sr1147-%'")
        .execute(pool)
        .await
        .expect("clear fixture contracts");
    // By id and by address: the address is UNIQUE, so a row left over from an
    // interrupted run under a different id would still block the insert.
    sqlx::query("DELETE FROM publishers WHERE id = $1 OR stellar_address = $2")
        .bind(PUBLISHER)
        .bind(publisher_address())
        .execute(pool)
        .await
        .expect("clear fixture publisher");

    sqlx::query("INSERT INTO publishers (id, stellar_address, username) VALUES ($1, $2, 'sr1147')")
        .bind(PUBLISHER)
        .bind(publisher_address())
        .execute(pool)
        .await
        .expect("create fixture publisher");

    for (tag, letter, name) in [
        (1u8, 'a', "A"),
        (2, 'b', "B"),
        (3, 'c', "C"),
        (4, 'd', "D"),
        (5, 'e', "E"),
    ] {
        sqlx::query(
            "INSERT INTO contracts (id, contract_id, wasm_hash, name, slug, publisher_id, network, interface_id, interface_algorithm)
             VALUES ($1, $2, $3, $4, $5, $6, 'testnet', $7, 'soroban-interface-v1')",
        )
        .bind(contract_uuid(tag))
        .bind(address(letter))
        .bind(wasm_hash(letter))
        .bind(name)
        .bind(format!("sr1147-{}", name.to_lowercase()))
        .bind(PUBLISHER)
        .bind(format!("iface-{letter}"))
        .execute(pool)
        .await
        .expect("create fixture contract");
    }

    // B's mainnet homograph: same address, different network. This is what makes
    // a bare address ambiguous and is the reason the routes take ?network=.
    sqlx::query(
        "INSERT INTO contracts (id, contract_id, wasm_hash, name, slug, publisher_id, network)
         VALUES ($1, $2, $3, 'B-mainnet', 'sr1147-b-mainnet', $4, 'mainnet')",
    )
    .bind(contract_uuid(20))
    .bind(address('b'))
    .bind(wasm_hash('z'))
    .bind(PUBLISHER)
    .execute(pool)
    .await
    .expect("create mainnet homograph");

    let edges: &[(u8, Option<u8>, String, &str, &str)] = &[
        (1, Some(2), address('b'), "resolved", "^1.0.0"),
        (2, Some(3), address('c'), "resolved", "^1.0.0"),
        (3, Some(4), address('d'), "resolved", "^1.0.0"),
        (4, Some(1), address('a'), "resolved", "^1.0.0"),
        (1, Some(4), address('d'), "resolved", "^2.0.0"),
        (1, None, "some-library".to_string(), "unresolved", "*"),
        (3, None, address('z'), "unresolved", "*"),
    ];

    for (source, target, target_ref, state, constraint) in edges {
        sqlx::query(
            "INSERT INTO contract_dependency_edges
                (source_contract_id, target_contract_id, target_ref, network, edge_source,
                 edge_state, version_constraint, expected_interface_id)
             VALUES ($1, $2, $3, 'testnet', 'declared', $4::dependency_edge_state, $5, $6)",
        )
        .bind(contract_uuid(*source))
        .bind(target.map(contract_uuid))
        .bind(target_ref)
        .bind(state)
        .bind(constraint)
        .bind(target.map(|t| format!("iface-{}", (b'a' + t - 1) as char)))
        .execute(pool)
        .await
        .expect("create fixture edge");
    }

    // E is fully signed, so it has no findings at all -- the case that proves
    // "zero findings" is reachable.
    sqlx::query(
        "INSERT INTO package_signatures
            (contract_id, version, wasm_hash, signature, signing_address, public_key, status)
         VALUES ($1, '1.0.0', $2, 'sig', $3, 'pk', 'valid')",
    )
    .bind(contract_uuid(5))
    .bind(wasm_hash('e'))
    .bind(publisher_address())
    .execute(pool)
    .await
    .expect("sign contract E");
}

/// Number of 429s to ride out before giving up.
///
/// The suite makes enough requests to trip the default per-IP limit, and a 429
/// says nothing about the behaviour under test -- letting one fail a test would
/// make the whole suite flaky for a reason unrelated to dependency graphs.
const RATE_LIMIT_RETRIES: usize = 8;

async fn get_response(path: &str) -> reqwest::Response {
    let url = format!("{}{}", api_base_url(), path);
    for attempt in 0..=RATE_LIMIT_RETRIES {
        let response = reqwest::get(&url).await.expect("request the API");
        if response.status() != StatusCode::TOO_MANY_REQUESTS {
            return response;
        }
        if attempt < RATE_LIMIT_RETRIES {
            tokio::time::sleep(std::time::Duration::from_millis(250 * (attempt as u64 + 1))).await;
        }
    }
    panic!("still rate limited after {RATE_LIMIT_RETRIES} retries: {url}. Raise RATE_LIMIT_IP_PER_MINUTE on the API under test.");
}

async fn get(path: &str) -> (StatusCode, Value) {
    let response = get_response(path).await;
    let status = response.status();
    let body = response.json().await.unwrap_or(Value::Null);
    (status, body)
}

async fn get_text(path: &str) -> String {
    get_response(path).await.text().await.expect("read body")
}

/// Every contract id appearing anywhere in a nested dependency tree.
fn flatten(node: &Value, out: &mut Vec<String>) {
    if let Some(id) = node.get("contract_id").and_then(Value::as_str) {
        out.push(id.to_string());
    }
    if let Some(children) = node.get("dependencies").and_then(Value::as_array) {
        for child in children {
            flatten(child, out);
        }
    }
}

fn diagnostics_of(body: &Value) -> Vec<String> {
    body.get("root")
        .and_then(|r| r.get("visualization_hints"))
        .and_then(|h| h.get("diagnostics"))
        .or_else(|| body.get("diagnostics"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|d| d.get("kind").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

// ── Path resolution ─────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn strkey_with_network_resolves() {
    // The original defect: the route declared Path<Uuid> while the CLI sent a
    // C... address, so axum rejected with 400 before any SQL ran.
    seed(&pool().await).await;

    let (status, body) = get(&format!(
        "/api/contracts/{}/dependencies?network=testnet",
        address('a')
    ))
    .await;

    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(
        body["root"]["contract_id"].as_str(),
        Some(address('a').as_str())
    );
}

#[tokio::test]
#[ignore]
async fn uuid_resolves() {
    seed(&pool().await).await;
    let (status, body) = get(&format!(
        "/api/contracts/{}/dependencies?transitive=true",
        contract_uuid(1)
    ))
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
}

#[tokio::test]
#[ignore]
async fn ambiguous_address_is_a_conflict_with_candidates() {
    // Silently picking one network would answer a question about the wrong
    // contract, invisibly. 409 with the candidates lets the caller choose.
    seed(&pool().await).await;

    let (status, body) = get(&format!("/api/contracts/{}/dependencies", address('b'))).await;

    assert_eq!(status, StatusCode::CONFLICT, "body: {body}");
    let candidates = body["details"]["candidates"]
        .as_array()
        .expect("candidates listed");
    let networks: Vec<&str> = candidates
        .iter()
        .filter_map(|c| c["network"].as_str())
        .collect();
    assert!(networks.contains(&"mainnet"));
    assert!(networks.contains(&"testnet"));
}

#[tokio::test]
#[ignore]
async fn ambiguous_address_resolves_once_scoped() {
    seed(&pool().await).await;
    let (status, _) = get(&format!(
        "/api/contracts/{}/dependencies?network=testnet",
        address('b')
    ))
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
#[ignore]
async fn a_non_address_non_uuid_is_a_bad_request_not_a_not_found() {
    // 404 would imply the contract might exist somewhere; 400 says the input is
    // malformed, which is the actionable message.
    seed(&pool().await).await;
    let (status, _) = get("/api/contracts/not-a-contract/dependencies").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
#[ignore]
async fn a_well_formed_unregistered_address_is_not_found() {
    seed(&pool().await).await;
    let (status, _) = get(&format!(
        "/api/contracts/{}/dependencies?network=testnet",
        address('q')
    ))
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore]
async fn uuid_with_a_contradicting_network_is_rejected() {
    // A UUID names exactly one row, so a mismatched ?network= is a contradiction
    // in the request rather than an ambiguity to resolve.
    seed(&pool().await).await;
    let (status, _) = get(&format!(
        "/api/contracts/{}/dependencies?network=mainnet",
        contract_uuid(1)
    ))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// ── Traversal shape ─────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn empty_graph_returns_a_root_with_no_dependencies() {
    seed(&pool().await).await;
    let (status, body) = get(&format!(
        "/api/contracts/{}/dependencies?transitive=true",
        contract_uuid(5)
    ))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total_dependencies"].as_u64(), Some(0));
    assert_eq!(body["has_circular"].as_bool(), Some(false));
    assert_eq!(
        body["root"]["dependencies"].as_array().map(Vec::len),
        Some(0)
    );
}

#[tokio::test]
#[ignore]
async fn a_cycle_terminates_and_is_reported() {
    // The guard must stop the walk *and* leave evidence. Terminating silently
    // would be indistinguishable from depth truncation.
    seed(&pool().await).await;
    let (status, body) = get(&format!(
        "/api/contracts/{}/dependencies?transitive=true",
        contract_uuid(1)
    ))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["has_circular"].as_bool(), Some(true));
    assert!(diagnostics_of(&body).contains(&"cycle".to_string()));
}

#[tokio::test]
#[ignore]
async fn a_diamond_does_not_duplicate_the_shared_subtree() {
    // D is reachable at depth 1 (A->D) and depth 3 (A->B->C->D). Keying tree
    // nesting on the parent contract id rather than the parent path attached
    // D's children once per incoming path and rendered them under every
    // occurrence, multiplying the subtree.
    seed(&pool().await).await;
    let (_, body) = get(&format!(
        "/api/contracts/{}/dependencies?transitive=true",
        contract_uuid(1)
    ))
    .await;

    let mut ids = Vec::new();
    flatten(&body["root"], &mut ids);

    // A appears as the root and once per cycle closure (two distinct cycles),
    // never more.
    let a_occurrences = ids.iter().filter(|id| **id == address('a')).count();
    assert_eq!(
        a_occurrences, 3,
        "root plus exactly two cycle closures; got {ids:?}"
    );
}

#[tokio::test]
#[ignore]
async fn direct_only_does_not_walk_the_closure() {
    seed(&pool().await).await;
    let (_, body) = get(&format!(
        "/api/contracts/{}/dependencies?transitive=false",
        contract_uuid(1)
    ))
    .await;

    assert_eq!(body["max_depth"].as_u64(), Some(1));
    for child in body["root"]["dependencies"].as_array().unwrap() {
        assert_eq!(
            child["dependencies"].as_array().map(Vec::len),
            Some(0),
            "a direct-only walk must not expand children"
        );
    }
}

#[tokio::test]
#[ignore]
async fn depth_limit_truncates_and_says_so() {
    seed(&pool().await).await;
    let (_, body) = get(&format!(
        "/api/contracts/{}/dependency-graph?depth=2",
        contract_uuid(1)
    ))
    .await;

    assert_eq!(body["truncated"].as_bool(), Some(true));
    assert_eq!(body["truncation_reason"].as_str(), Some("depth_limit"));
    assert!(diagnostics_of(&body).contains(&"truncated".to_string()));
}

#[tokio::test]
#[ignore]
async fn node_budget_truncates_and_says_so() {
    seed(&pool().await).await;
    let (_, body) = get(&format!(
        "/api/contracts/{}/dependency-graph?max_nodes=1",
        contract_uuid(1)
    ))
    .await;

    assert_eq!(body["truncated"].as_bool(), Some(true));
    assert_eq!(body["truncation_reason"].as_str(), Some("node_limit"));
    assert_eq!(body["total_dependencies"].as_u64(), Some(1));
}

#[tokio::test]
#[ignore]
async fn unresolved_edges_are_retained_and_split_by_shape() {
    // A well-formed address that is not registered is a real gap in the graph.
    // A free-form name is an undeclared library and merely informational. A
    // blanket severity over both would bury the signal in noise.
    seed(&pool().await).await;
    let (_, body) = get(&format!(
        "/api/contracts/{}/dependency-graph",
        contract_uuid(1)
    ))
    .await;

    assert_eq!(body["unresolved"].as_u64(), Some(2));

    let details: Vec<&str> = body["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["kind"] == "unresolved_edge")
        .filter_map(|d| d["detail"].as_str())
        .collect();

    assert!(
        details.iter().any(|d| d.contains("not registered")),
        "a valid address that is unregistered is a gap: {details:?}"
    );
    assert!(
        details.iter().any(|d| d.contains("undeclared library")),
        "a free-form name is informational: {details:?}"
    );
}

#[tokio::test]
#[ignore]
async fn dependents_walks_the_reverse_edges() {
    seed(&pool().await).await;
    let (status, body) = get(&format!(
        "/api/contracts/{}/dependents?transitive=true",
        contract_uuid(4)
    ))
    .await;

    assert_eq!(status, StatusCode::OK);
    let mut ids = Vec::new();
    flatten(&body["root"], &mut ids);
    // Both routes into D: directly from A, and from C.
    assert!(ids.contains(&address('a')));
    assert!(ids.contains(&address('c')));
}

// ── Tenancy ─────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn a_private_node_is_counted_but_never_named() {
    let pool = pool().await;
    seed(&pool).await;

    sqlx::query("UPDATE contracts SET visibility = 'private' WHERE id = $1")
        .bind(contract_uuid(3))
        .execute(&pool)
        .await
        .expect("hide C");

    let (_, body) = get(&format!(
        "/api/contracts/{}/dependencies?transitive=true",
        contract_uuid(1)
    ))
    .await;

    let mut ids = Vec::new();
    flatten(&body["root"], &mut ids);

    assert!(
        !ids.contains(&address('c')),
        "a private contract's address must not appear: {ids:?}"
    );
    assert!(
        ids.iter().any(|id| id == "[redacted]"),
        "the node must still be counted: {ids:?}"
    );

    sqlx::query("UPDATE contracts SET visibility = 'public' WHERE id = $1")
        .bind(contract_uuid(3))
        .execute(&pool)
        .await
        .expect("restore C");
}

#[tokio::test]
#[ignore]
async fn a_private_node_is_not_a_stepping_stone() {
    // C is the only route to D's own dependencies via B. With C hidden, the
    // walk must stop there rather than enumerating what a private contract
    // depends on.
    let pool = pool().await;
    seed(&pool).await;

    // Remove the A->D shortcut so C is the only path onward.
    sqlx::query("DELETE FROM contract_dependency_edges WHERE source_contract_id = $1 AND target_contract_id = $2")
        .bind(contract_uuid(1))
        .bind(contract_uuid(4))
        .execute(&pool)
        .await
        .expect("drop the diamond shortcut");

    sqlx::query("UPDATE contracts SET visibility = 'private' WHERE id = $1")
        .bind(contract_uuid(3))
        .execute(&pool)
        .await
        .expect("hide C");

    let (_, body) = get(&format!(
        "/api/contracts/{}/dependencies?transitive=true",
        contract_uuid(1)
    ))
    .await;

    let mut ids = Vec::new();
    flatten(&body["root"], &mut ids);
    assert!(
        !ids.contains(&address('d')),
        "D sits behind a private node and must not be enumerated: {ids:?}"
    );

    sqlx::query("UPDATE contracts SET visibility = 'public' WHERE id = $1")
        .bind(contract_uuid(3))
        .execute(&pool)
        .await
        .expect("restore C");
}

// ── Risk ────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn a_clean_contract_has_zero_findings() {
    // Reachable only because diagnostics are kept out of `findings`. If cycles
    // or unresolved edges carried a severity, no graph could ever be clean.
    seed(&pool().await).await;
    let (status, body) = get(&format!(
        "/api/contracts/{}/dependency-risk",
        contract_uuid(5)
    ))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["direct_findings"].as_array().map(Vec::len), Some(0));
    assert_eq!(body["inherited_findings"].as_array().map(Vec::len), Some(0));
    assert!(body["overall_risk"]["effective_severity"].is_null());
}

#[tokio::test]
#[ignore]
async fn a_vulnerability_propagates_with_its_path_and_severity() {
    let pool = pool().await;
    seed(&pool).await;

    let scan_id = Uuid::new_v4();
    // The notify_security_issue trigger has a pre-existing bug (it references
    // `sub.` inside the query that defines `sub`), so it is disabled for the
    // seed. Unrelated to this feature.
    sqlx::query("ALTER TABLE security_issues DISABLE TRIGGER USER")
        .execute(&pool)
        .await
        .expect("disable notify trigger");
    sqlx::query(
        "INSERT INTO security_scans (id, contract_id, status, scan_type)
         VALUES ($1, $2, 'completed', 'static')",
    )
    .bind(scan_id)
    .bind(contract_uuid(4))
    .execute(&pool)
    .await
    .expect("create scan");
    sqlx::query(
        "INSERT INTO security_issues (scan_id, contract_id, title, description, severity, status)
         VALUES ($1, $2, 'Reentrancy', 'external call before state write', 'critical', 'open')",
    )
    .bind(scan_id)
    .bind(contract_uuid(4))
    .execute(&pool)
    .await
    .expect("create issue");

    let (_, body) = get(&format!(
        "/api/contracts/{}/dependency-risk",
        contract_uuid(1)
    ))
    .await;

    let inherited = body["inherited_findings"].as_array().unwrap();
    let vuln: Vec<&Value> = inherited
        .iter()
        .filter(|f| f["rule_id"] == "open_vulnerability")
        .collect();

    assert_eq!(
        vuln.len(),
        1,
        "the same finding reached by two paths must dedupe to one"
    );
    assert_eq!(
        vuln[0]["severity"].as_str(),
        Some("Critical"),
        "a vulnerable dependency is as exploitable through the caller"
    );
    // Shortest path wins: A->D, not A->B->C->D.
    assert_eq!(vuln[0]["inherited_via_depth"].as_u64(), Some(1));
    assert_eq!(vuln[0]["path"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        body["overall_risk"]["effective_severity"].as_str(),
        Some("Critical")
    );

    sqlx::query("DELETE FROM security_issues WHERE scan_id = $1")
        .bind(scan_id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("ALTER TABLE security_issues ENABLE TRIGGER USER")
        .execute(&pool)
        .await
        .ok();
}

#[tokio::test]
#[ignore]
async fn a_revoked_signature_on_a_dependency_is_inherited() {
    let pool = pool().await;
    seed(&pool).await;

    sqlx::query(
        "INSERT INTO package_signatures
            (contract_id, version, wasm_hash, signature, signing_address, public_key, status, revoked_reason)
         VALUES ($1, '1.0.0', $2, 'sig', $3, 'pk', 'revoked', 'key compromise')",
    )
    .bind(contract_uuid(2))
    .bind(wasm_hash('b'))
    .bind(publisher_address())
    .execute(&pool)
    .await
    .expect("revoke B's signature");

    let (_, body) = get(&format!(
        "/api/contracts/{}/dependency-risk",
        contract_uuid(1)
    ))
    .await;

    let revoked: Vec<&Value> = body["inherited_findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule_id"] == "signature_revoked")
        .collect();

    assert_eq!(revoked.len(), 1, "body: {body}");
    assert_eq!(revoked[0]["severity"].as_str(), Some("High"));
}

#[tokio::test]
#[ignore]
async fn conflicting_constraints_are_a_diagnostic_not_a_finding() {
    // A version conflict has no defensible severity, so it must never inflate
    // the risk level of an otherwise clean graph.
    let pool = pool().await;
    seed(&pool).await;

    for version in ["1.0.0", "2.0.0"] {
        sqlx::query(
            "INSERT INTO contract_versions (contract_id, version, wasm_hash) VALUES ($1, $2, $3)",
        )
        .bind(contract_uuid(4))
        .bind(version)
        .bind(wasm_hash(if version == "1.0.0" { 'd' } else { 'e' }))
        .execute(&pool)
        .await
        .ok();
    }

    let (_, body) = get(&format!(
        "/api/contracts/{}/dependency-risk",
        contract_uuid(1)
    ))
    .await;

    assert!(
        diagnostics_of(&body).contains(&"version_conflict".to_string()),
        "^1.0.0 and ^2.0.0 on D have no commonly satisfying version: {body}"
    );
    assert!(
        !body["inherited_findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["rule_id"] == "version_conflict"),
        "a conflict must not appear as a severity-bearing finding"
    );
}

#[tokio::test]
#[ignore]
async fn an_interface_change_is_reported_against_the_recorded_expectation() {
    let pool = pool().await;
    seed(&pool).await;

    sqlx::query("UPDATE contracts SET interface_id = 'iface-b-v2' WHERE id = $1")
        .bind(contract_uuid(2))
        .execute(&pool)
        .await
        .expect("change B's interface");

    let (_, body) = get(&format!(
        "/api/contracts/{}/dependency-risk",
        contract_uuid(1)
    ))
    .await;

    let all: Vec<&Value> = body["direct_findings"]
        .as_array()
        .unwrap()
        .iter()
        .chain(body["inherited_findings"].as_array().unwrap())
        .filter(|f| f["rule_id"] == "interface_incompatibility")
        .collect();

    assert_eq!(all.len(), 1, "body: {body}");
    assert!(all[0]["detail"]
        .as_str()
        .unwrap()
        .contains("interface changed"));

    sqlx::query("UPDATE contracts SET interface_id = 'iface-b' WHERE id = $1")
        .bind(contract_uuid(2))
        .execute(&pool)
        .await
        .expect("restore B's interface");
}

#[tokio::test]
#[ignore]
async fn a_null_interface_id_is_unknown_not_incompatible() {
    // Every artifact-less contract has a NULL interface id. Treating that as
    // drift would flag most of the registry as incompatible.
    let pool = pool().await;
    seed(&pool).await;

    sqlx::query("UPDATE contracts SET interface_id = NULL WHERE id = $1")
        .bind(contract_uuid(2))
        .execute(&pool)
        .await
        .expect("clear B's interface");

    let (_, body) = get(&format!(
        "/api/contracts/{}/dependency-risk",
        contract_uuid(1)
    ))
    .await;

    assert!(
        !body["direct_findings"]
            .as_array()
            .unwrap()
            .iter()
            .chain(body["inherited_findings"].as_array().unwrap())
            .any(|f| f["rule_id"] == "interface_incompatibility"),
        "unknown must not read as different: {body}"
    );

    sqlx::query("UPDATE contracts SET interface_id = 'iface-b' WHERE id = $1")
        .bind(contract_uuid(2))
        .execute(&pool)
        .await
        .expect("restore B's interface");
}

// ── Pagination ──────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn the_flat_node_list_is_paginated() {
    seed(&pool().await).await;
    let (status, body) = get(&format!(
        "/api/contracts/{}/dependency-graph?per_page=2&page=1",
        contract_uuid(1)
    ))
    .await;

    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["nodes"]["items"].as_array().map(Vec::len), Some(2));
    assert_eq!(body["nodes"]["per_page"].as_i64(), Some(2));
    assert_eq!(body["nodes"]["page"].as_i64(), Some(1));
    // The total counts the whole reachable set, not the page.
    assert_eq!(
        body["nodes"]["total"].as_i64(),
        body["total_dependencies"].as_i64()
    );
}

#[tokio::test]
#[ignore]
async fn pages_partition_the_node_set_without_gaps_or_duplicates() {
    // The property that makes offset pagination usable here: the traversal
    // emits a total order, so walking every page reconstructs the set exactly.
    seed(&pool().await).await;

    let (_, first) = get(&format!(
        "/api/contracts/{}/dependency-graph?per_page=100",
        contract_uuid(1)
    ))
    .await;
    let total = first["nodes"]["total"].as_i64().expect("total") as usize;

    let mut walked: Vec<String> = Vec::new();
    for page in 1..=((total / 2) + 2) {
        let (_, body) = get(&format!(
            "/api/contracts/{}/dependency-graph?per_page=2&page={page}",
            contract_uuid(1)
        ))
        .await;
        let items = body["nodes"]["items"].as_array().expect("items");
        if items.is_empty() {
            break;
        }
        for item in items {
            walked.push(serde_json::to_string(item).expect("serialize node"));
        }
    }

    assert_eq!(walked.len(), total, "every node appears exactly once");
    let mut deduped = walked.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(deduped.len(), walked.len(), "no node is returned twice");
}

#[tokio::test]
#[ignore]
async fn a_page_boundary_is_stable_across_requests() {
    seed(&pool().await).await;
    let path = format!(
        "/api/contracts/{}/dependency-graph?per_page=2&page=2",
        contract_uuid(1)
    );
    assert_eq!(get_text(&path).await, get_text(&path).await);
}

#[tokio::test]
#[ignore]
async fn a_page_past_the_end_is_empty_not_an_error() {
    seed(&pool().await).await;
    let (status, body) = get(&format!(
        "/api/contracts/{}/dependency-graph?per_page=2&page=9999",
        contract_uuid(1)
    ))
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["nodes"]["items"].as_array().map(Vec::len), Some(0));
    assert!(
        body["nodes"]["total"].as_i64().unwrap() > 0,
        "total is unaffected"
    );
}

#[tokio::test]
#[ignore]
async fn per_page_is_capped_server_side() {
    // A caller must not be able to page in the whole node cap at once.
    seed(&pool().await).await;
    let (_, body) = get(&format!(
        "/api/contracts/{}/dependency-graph?per_page=100000",
        contract_uuid(1)
    ))
    .await;
    assert_eq!(body["nodes"]["per_page"].as_i64(), Some(200));
}

#[tokio::test]
#[ignore]
async fn a_zero_or_negative_page_is_clamped_not_rejected() {
    seed(&pool().await).await;
    for query in ["page=0", "page=-5", "per_page=0"] {
        let (status, body) = get(&format!(
            "/api/contracts/{}/dependency-graph?{query}",
            contract_uuid(1)
        ))
        .await;
        assert_eq!(status, StatusCode::OK, "{query} -> {body}");
        assert!(body["nodes"]["page"].as_i64().unwrap() >= 1);
        assert!(body["nodes"]["per_page"].as_i64().unwrap() >= 1);
    }
}

// ── Determinism ─────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn repeated_traversals_are_byte_identical() {
    // The acceptance criterion. Order comes from a total SQL ORDER BY with no
    // floats, and dedup keeps the shortest path with a lexicographic tie-break,
    // so the output is a function of the graph rather than of row arrival order.
    seed(&pool().await).await;
    let path = format!(
        "/api/contracts/{}/dependencies?transitive=true",
        contract_uuid(1)
    );

    let first = get_text(&path).await;
    let second = get_text(&path).await;
    assert_eq!(first, second);
}

#[tokio::test]
#[ignore]
async fn repeated_risk_reports_are_byte_identical() {
    seed(&pool().await).await;
    let path = format!("/api/contracts/{}/dependency-risk", contract_uuid(1));

    let first = get_text(&path).await;
    let second = get_text(&path).await;
    assert_eq!(first, second);
}

// ── Compatibility ───────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn the_compatibility_view_exposes_declared_edges() {
    // Eight of the eleven phantom-table call sites read `contract_dependencies`
    // in the legacy static shape. The view is what makes them work unchanged.
    let pool = pool().await;
    seed(&pool).await;

    let rows: Vec<(Uuid, String, Option<Uuid>)> = sqlx::query_as(
        "SELECT contract_id, dependency_name, dependency_contract_id
         FROM contract_dependencies WHERE contract_id = $1 ORDER BY dependency_name",
    )
    .bind(contract_uuid(1))
    .fetch_all(&pool)
    .await
    .expect("read the compatibility view");

    assert_eq!(rows.len(), 3, "two resolved plus one unresolved");
    assert!(rows
        .iter()
        .any(|(_, name, id)| name == &address('b') && id.is_some()));
    assert!(rows
        .iter()
        .any(|(_, name, id)| name == "some-library" && id.is_none()));
}

#[tokio::test]
#[ignore]
async fn a_superseded_edge_leaves_the_current_view() {
    let pool = pool().await;
    seed(&pool).await;

    sqlx::query(
        "UPDATE contract_dependency_edges SET superseded_at = NOW()
         WHERE source_contract_id = $1 AND target_ref = $2",
    )
    .bind(contract_uuid(1))
    .bind(address('b'))
    .execute(&pool)
    .await
    .expect("supersede an edge");

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_dependencies WHERE contract_id = $1 AND dependency_name = $2",
    )
    .bind(contract_uuid(1))
    .bind(address('b'))
    .fetch_one(&pool)
    .await
    .expect("count current edges");

    assert_eq!(count, 0, "the view must show current state only");
}

// ── Snapshot schema ─────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn a_snapshot_without_a_graph_stays_on_schema_1_0() {
    // Requires REGISTRY_SIGNING_KEY on the API. Skipped otherwise, because a
    // 503 here says nothing about the schema rule under test.
    seed(&pool().await).await;
    let (status, body) = get(&format!("/api/contracts/{}/snapshot", contract_uuid(1))).await;
    if status == StatusCode::SERVICE_UNAVAILABLE {
        eprintln!("skipping: REGISTRY_SIGNING_KEY is not configured on the API");
        return;
    }

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["payload"]["schema_version"].as_str(), Some("1.0"));
    assert!(body["payload"].get("dependency_graph").is_none());
}

#[tokio::test]
#[ignore]
async fn a_graph_bearing_snapshot_declares_schema_1_1() {
    // An old verifier must get UnsupportedSchema("1.1"), not a signature
    // failure. That is only possible if the version moves with the content.
    seed(&pool().await).await;
    let (status, body) = get(&format!(
        "/api/contracts/{}/snapshot?include_dependency_graph=true",
        contract_uuid(1)
    ))
    .await;
    if status == StatusCode::SERVICE_UNAVAILABLE {
        eprintln!("skipping: REGISTRY_SIGNING_KEY is not configured on the API");
        return;
    }

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["payload"]["schema_version"].as_str(), Some("1.1"));
    assert!(body["payload"]["dependency_graph"]["overall_risk"].is_object());
}
