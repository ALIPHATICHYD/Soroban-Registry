// tests/error_taxonomy_tests.rs
//
// Every failure mode the registry can produce, over real HTTP, asserted through
// the client's public API: success, malformed JSON, timeouts, 401, 403, 409,
// 422, 429 and 5xx — plus what the retry policy does with each.
//
// The fake registry is a few lines of tokio rather than a mocking framework, so
// these tests stay dependency-free and run offline.

mod support;

use std::time::Duration;

use registry_client::{
    ClientConfig, ContractSearchParams, Error, PublishRequest, RegistryClient, RetryPolicy,
};
use support::{FakeRegistry, Reply};

fn publish_request() -> PublishRequest {
    PublishRequest {
        contract_id: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
        wasm_hash: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".to_string(),
        wasm_artifact_base64: None,
        name: "swap-router".to_string(),
        slug: None,
        description: None,
        network: registry_client::Network::Testnet,
        category: None,
        tags: Vec::new(),
        source_url: None,
        publisher_address: "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
        dependencies: Vec::new(),
        is_cicd: false,
    }
}

/// A client with retries off, so one request means one attempt.
fn single_attempt_client(registry: &FakeRegistry) -> RegistryClient {
    RegistryClient::from_config(
        ClientConfig::new(registry.base_url()).with_retry_policy(RetryPolicy::none()),
    )
    .expect("client")
}

// ── Success ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_successful_list_decodes_into_typed_models() {
    let registry = FakeRegistry::start(vec![Reply::ok(&support::contract_list_body(2, 7))]).await;
    let client = single_attempt_client(&registry);

    let page = client
        .list_contracts(&ContractSearchParams {
            limit: Some(2),
            ..Default::default()
        })
        .await
        .expect("list should succeed");

    assert_eq!(page.items.len(), 2);
    assert_eq!(page.total, 7);
    assert_eq!(page.items[0].name, "contract-0");
    let target = registry.requests()[0].target().to_string();
    assert!(target.starts_with("/api/contracts?"), "{target}");
    assert!(target.contains("limit=2"), "{target}");
}

// ── Malformed responses ───────────────────────────────────────────────────────

#[tokio::test]
async fn malformed_json_is_reported_as_a_decode_failure() {
    let registry = FakeRegistry::start(vec![Reply::ok("{ this is not json")]).await;
    let client = single_attempt_client(&registry);

    let err = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect_err("a malformed body must not be swallowed");

    assert!(
        matches!(err, Error::MalformedResponse { .. }),
        "unexpected error: {err:?}"
    );
    assert!(err.to_string().contains("could not decode"), "{err}");
}

#[tokio::test]
async fn a_well_formed_body_of_the_wrong_shape_is_a_decode_failure() {
    // Valid JSON, but not a paginated contract list.
    let registry = FakeRegistry::start(vec![Reply::ok(r#"{"unexpected": true}"#)]).await;
    let client = single_attempt_client(&registry);

    let err = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect_err("shape mismatch must surface");

    assert!(matches!(err, Error::MalformedResponse { .. }), "{err:?}");
}

// ── Timeouts ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn a_slow_endpoint_produces_a_timeout_error() {
    let registry = FakeRegistry::start(vec![Reply::hang()]).await;
    let client = RegistryClient::from_config(
        ClientConfig::new(registry.base_url())
            .with_timeout(Duration::from_millis(150))
            .with_retry_policy(RetryPolicy::none()),
    )
    .expect("client");

    let err = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect_err("the request must time out");

    assert!(matches!(err, Error::Timeout { .. }), "unexpected: {err:?}");
    assert!(err.is_transient(), "a timeout is worth retrying");
    assert!(err.to_string().contains("timed out"), "{err}");
}

#[tokio::test]
async fn timeouts_are_retried_when_the_policy_allows_it() {
    let registry = FakeRegistry::start(vec![
        Reply::hang(),
        Reply::ok(&support::contract_list_body(1, 1)),
    ])
    .await;
    let client = RegistryClient::from_config(
        ClientConfig::new(registry.base_url())
            .with_timeout(Duration::from_millis(150))
            .with_retry_policy(
                RetryPolicy::attempts(2).with_initial_backoff(Duration::from_millis(1)),
            ),
    )
    .expect("client");

    let page = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect("the retry should succeed");

    assert_eq!(page.items.len(), 1);
    assert_eq!(registry.requests().len(), 2);
}

#[tokio::test]
async fn timeout_retries_can_be_switched_off() {
    let registry = FakeRegistry::start(vec![
        Reply::hang(),
        Reply::ok(&support::contract_list_body(1, 1)),
    ])
    .await;
    let client = RegistryClient::from_config(
        ClientConfig::new(registry.base_url())
            .with_timeout(Duration::from_millis(150))
            .with_retry_policy(RetryPolicy::attempts(3).with_retry_on_timeout(false)),
    )
    .expect("client");

    let err = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect_err("timeouts must not be retried");

    assert!(matches!(err, Error::Timeout { .. }), "{err:?}");
    assert_eq!(registry.requests().len(), 1);
}

// ── Status classification ─────────────────────────────────────────────────────

#[tokio::test]
async fn unauthorized_responses_are_distinct_from_forbidden_ones() {
    let registry = FakeRegistry::start(vec![
        Reply::error(
            401,
            r#"{"code":"Unauthorized","message":"session expired","request_id":"req-1"}"#,
        ),
        Reply::error(
            403,
            r#"{"code":"Forbidden","message":"not the publisher","request_id":"req-2"}"#,
        ),
    ])
    .await;
    let client = single_attempt_client(&registry);

    let unauthorized = client
        .get_contract("abc")
        .await
        .expect_err("401 must surface");
    assert!(
        matches!(unauthorized, Error::Unauthorized(_)),
        "{unauthorized:?}"
    );
    assert!(unauthorized.is_auth());
    assert!(
        !unauthorized.is_transient(),
        "retrying will not fix credentials"
    );
    assert_eq!(unauthorized.code(), Some("Unauthorized"));
    assert_eq!(unauthorized.request_id(), Some("req-1"));

    let forbidden = client
        .get_contract("abc")
        .await
        .expect_err("403 must surface");
    assert!(matches!(forbidden, Error::Forbidden(_)), "{forbidden:?}");
    assert!(forbidden.is_auth());
    assert_eq!(forbidden.status(), Some(403));
}

#[tokio::test]
async fn a_missing_contract_is_a_not_found_error() {
    let registry = FakeRegistry::start(vec![Reply::error(
        404,
        r#"{"code":"NotFound","message":"no such contract"}"#,
    )])
    .await;
    let client = single_attempt_client(&registry);

    let err = client.get_contract("missing").await.expect_err("404");

    assert!(matches!(err, Error::NotFound(_)), "{err:?}");
    assert!(!err.is_transient());
}

#[tokio::test]
async fn a_duplicate_publish_is_a_conflict_with_the_server_code() {
    let registry = FakeRegistry::start(vec![Reply::error(
        409,
        r#"{"code":"ContractAlreadyExists","message":"already registered","details":{"contract_id":"CDLZ"}}"#,
    )])
    .await;
    let client = single_attempt_client(&registry);

    let err = client
        .publish_contract(&publish_request(), None)
        .await
        .expect_err("409");

    assert!(matches!(err, Error::Conflict(_)), "{err:?}");
    assert_eq!(err.code(), Some("ContractAlreadyExists"));
    assert_eq!(
        err.error_details()
            .and_then(|details| details["contract_id"].as_str()),
        Some("CDLZ"),
        "structured details are preserved"
    );
}

#[tokio::test]
async fn an_in_flight_idempotent_publish_is_its_own_error() {
    let registry = FakeRegistry::start(vec![Reply::error(
        409,
        r#"{"code":"IdempotencyKeyInProgress","message":"already processing"}"#,
    )])
    .await;
    let client = single_attempt_client(&registry);

    let err = client
        .publish_contract(&publish_request(), Some("release-1".to_string()))
        .await
        .expect_err("409 in-progress");

    assert!(
        matches!(err, Error::IdempotencyInProgress(_)),
        "an in-flight replay must be distinguishable from a duplicate: {err:?}"
    );
    assert!(err.is_transient(), "the caller should try again shortly");
}

#[tokio::test]
async fn validation_failures_carry_field_details() {
    let registry = FakeRegistry::start(vec![Reply::error(
        422,
        r#"{"code":"ValidationError","message":"invalid metadata","details":{"errors":[{"field":"name","reason":"too long"}]}}"#,
    )])
    .await;
    let client = single_attempt_client(&registry);

    let err = client
        .publish_contract(&publish_request(), None)
        .await
        .expect_err("422");

    assert!(matches!(err, Error::Validation(_)), "{err:?}");
    assert!(!err.is_transient(), "a bad request will not fix itself");
    let details = err.error_details().expect("structured details");
    assert_eq!(details["errors"][0]["field"], "name");
}

#[tokio::test]
async fn a_400_is_also_a_validation_failure() {
    let registry = FakeRegistry::start(vec![Reply::error(
        400,
        r#"{"code":"EMPTY_QUERY","message":"Search query cannot be empty"}"#,
    )])
    .await;
    let client = single_attempt_client(&registry);

    let err = client.get_contract("abc").await.expect_err("400");

    assert!(matches!(err, Error::Validation(_)), "{err:?}");
    assert_eq!(err.code(), Some("EMPTY_QUERY"));
}

#[tokio::test]
async fn rate_limits_preserve_retry_after() {
    let registry = FakeRegistry::start(vec![Reply::error(
        429,
        r#"{"code":"RateLimited","message":"slow down"}"#,
    )
    .with_header("retry-after", "42")])
    .await;
    let client = single_attempt_client(&registry);

    let err = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect_err("429");

    assert!(matches!(err, Error::RateLimited { .. }), "{err:?}");
    assert_eq!(err.retry_after(), Some(Duration::from_secs(42)));
    assert!(err.is_transient());
    assert!(err.to_string().contains("retry after 42s"), "{err}");
}

#[tokio::test]
async fn a_retry_after_beyond_the_ceiling_is_surfaced_instead_of_slept_through() {
    let registry = FakeRegistry::start(vec![
        Reply::error(429, r#"{"code":"RateLimited"}"#).with_header("retry-after", "3600"),
        Reply::ok(&support::contract_list_body(1, 1)),
    ])
    .await;
    let client =
        RegistryClient::from_config(ClientConfig::new(registry.base_url()).with_retry_policy(
            RetryPolicy::attempts(3).with_max_retry_after(Duration::from_secs(5)),
        ))
        .expect("client");

    let err = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect_err("an hour-long wait must not block the call");

    assert!(matches!(err, Error::RateLimited { .. }), "{err:?}");
    assert_eq!(err.retry_after(), Some(Duration::from_secs(3600)));
    assert_eq!(
        registry.requests().len(),
        1,
        "the client stopped instead of sleeping"
    );
}

#[tokio::test]
async fn a_short_retry_after_is_honoured_before_retrying() {
    let registry = FakeRegistry::start(vec![
        Reply::error(429, r#"{"code":"RateLimited"}"#).with_header("retry-after", "0"),
        Reply::ok(&support::contract_list_body(1, 1)),
    ])
    .await;
    let client = RegistryClient::from_config(
        ClientConfig::new(registry.base_url()).with_retry_policy(RetryPolicy::attempts(2)),
    )
    .expect("client");

    let page = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect("the retry should succeed");

    assert_eq!(page.items.len(), 1);
    assert_eq!(registry.requests().len(), 2);
}

#[tokio::test]
async fn server_faults_are_temporary_upstream_errors() {
    let registry = FakeRegistry::start(vec![Reply::error(
        503,
        r#"{"code":"ServiceUnavailable","message":"database down"}"#,
    )])
    .await;
    let client = single_attempt_client(&registry);

    let err = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect_err("503");

    assert!(matches!(err, Error::Upstream(_)), "{err:?}");
    assert!(err.is_transient());
    assert_eq!(err.status(), Some(503));
}

#[tokio::test]
async fn a_5xx_is_retried_for_safe_reads() {
    let registry = FakeRegistry::start(vec![
        Reply::error(500, r#"{"code":"Internal"}"#),
        Reply::error(502, r#"{"code":"BadGateway"}"#),
        Reply::ok(&support::contract_list_body(1, 1)),
    ])
    .await;
    let client =
        RegistryClient::from_config(ClientConfig::new(registry.base_url()).with_retry_policy(
            RetryPolicy::attempts(3).with_initial_backoff(Duration::from_millis(1)),
        ))
        .expect("client");

    let page = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect("the third attempt should succeed");

    assert_eq!(page.items.len(), 1);
    assert_eq!(registry.requests().len(), 3);
}

#[tokio::test]
async fn retries_are_bounded_by_the_policy() {
    let registry = FakeRegistry::start(vec![
        Reply::error(500, r#"{"code":"Internal"}"#),
        Reply::error(500, r#"{"code":"Internal"}"#),
        Reply::error(500, r#"{"code":"Internal"}"#),
        Reply::error(500, r#"{"code":"Internal"}"#),
    ])
    .await;
    let client =
        RegistryClient::from_config(ClientConfig::new(registry.base_url()).with_retry_policy(
            RetryPolicy::attempts(2).with_initial_backoff(Duration::from_millis(1)),
        ))
        .expect("client");

    let err = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect_err("all attempts fail");

    assert!(matches!(err, Error::Upstream(_)), "{err:?}");
    assert_eq!(registry.requests().len(), 2, "exactly max_attempts tries");
}

#[tokio::test]
async fn an_unreachable_registry_is_a_transport_error() {
    // Port 1 is reserved and refuses connections.
    let client = RegistryClient::from_config(
        ClientConfig::new("http://127.0.0.1:1").with_retry_policy(RetryPolicy::none()),
    )
    .expect("client");

    let err = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect_err("nothing is listening");

    assert!(matches!(err, Error::Transport { .. }), "{err:?}");
    assert!(err.is_transient());
    assert!(err.to_string().contains("attempt(s)"), "{err}");
}

#[tokio::test]
async fn an_unrecognised_status_stays_a_generic_api_error() {
    let registry = FakeRegistry::start(vec![Reply::error(
        418,
        r#"{"code":"Teapot","message":"nope"}"#,
    )])
    .await;
    let client = single_attempt_client(&registry);

    let err = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect_err("418");

    assert!(matches!(err, Error::Api(_)), "{err:?}");
    assert_eq!(err.status(), Some(418));
    assert_eq!(err.code(), Some("Teapot"));
}
