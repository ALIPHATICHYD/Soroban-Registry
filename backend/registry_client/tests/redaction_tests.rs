// tests/redaction_tests.rs
//
// Credentials must never reach a log line or an error message. These tests hunt
// for the token in everything a consumer might plausibly print: the client, its
// config, every error variant, and the pagination types.

mod support;

use std::time::Duration;

use registry_client::{
    Auth, ClientConfig, ContractSearchParams, ContractSearchRequest, PageLimits, PublishRequest,
    RegistryClient, RetryPolicy, Secret, REDACTED,
};
use support::{FakeRegistry, Reply};

const TOKEN: &str = "tok_live_51H8xQzZZZsecretZZZ";
const API_KEY: &str = "key_live_do_not_log_me";

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

fn assert_no_secret(label: &str, rendered: &str) {
    assert!(
        !rendered.contains(TOKEN) && !rendered.contains(API_KEY),
        "{label} leaked a credential: {rendered}"
    );
}

#[test]
fn a_secret_never_prints_itself() {
    let secret = Secret::new(TOKEN);
    assert_no_secret("Secret Debug", &format!("{secret:?}"));
    assert_no_secret("Secret Display", &format!("{secret}"));
    assert_eq!(format!("{secret:?}"), REDACTED);
}

#[test]
fn config_and_client_debug_are_safe_to_log() {
    let config = ClientConfig::new("http://registry.test").with_auth(Auth::bearer(TOKEN));
    assert_no_secret("ClientConfig Debug", &format!("{config:?}"));

    let client = RegistryClient::from_config(config).expect("client");
    assert_no_secret("RegistryClient Debug", &format!("{client:?}"));
    assert_no_secret("RegistryClient alt Debug", &format!("{client:#?}"));

    let keyed = RegistryClient::new("http://registry.test")
        .expect("client")
        .with_auth(Auth::api_key("X-API-Key", API_KEY));
    assert_no_secret("api-key client Debug", &format!("{keyed:?}"));
}

#[tokio::test]
async fn api_errors_do_not_echo_the_credential() {
    let registry = FakeRegistry::start(vec![
        Reply::error(401, r#"{"code":"Unauthorized","message":"token expired"}"#),
        Reply::error(403, r#"{"code":"Forbidden","message":"not yours"}"#),
        Reply::error(409, r#"{"code":"ContractAlreadyExists"}"#),
        Reply::error(
            422,
            r#"{"code":"ValidationError","details":{"field":"name"}}"#,
        ),
        Reply::error(429, r#"{"code":"RateLimited"}"#),
        Reply::error(500, r#"{"code":"Internal"}"#),
        Reply::ok("{ not json"),
    ])
    .await;
    let client = RegistryClient::from_config(
        ClientConfig::new(registry.base_url())
            .with_auth(Auth::bearer(TOKEN))
            .with_retry_policy(RetryPolicy::none()),
    )
    .expect("client");

    for expected in ["401", "403", "409", "422", "429", "500", "malformed"] {
        let err = client
            .list_contracts(&ContractSearchParams::default())
            .await
            .expect_err("every scripted reply is a failure");

        assert_no_secret(&format!("{expected} Display"), &err.to_string());
        assert_no_secret(&format!("{expected} Debug"), &format!("{err:?}"));
    }
}

#[tokio::test]
async fn transport_and_timeout_errors_do_not_echo_the_credential() {
    // Nothing listening on port 1: a connect failure.
    let unreachable = RegistryClient::from_config(
        ClientConfig::new("http://127.0.0.1:1")
            .with_auth(Auth::bearer(TOKEN))
            .with_retry_policy(RetryPolicy::none()),
    )
    .expect("client");
    let err = unreachable
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect_err("connection refused");
    assert_no_secret("transport Display", &err.to_string());
    assert_no_secret("transport Debug", &format!("{err:?}"));

    let registry = FakeRegistry::start(vec![Reply::hang()]).await;
    let slow = RegistryClient::from_config(
        ClientConfig::new(registry.base_url())
            .with_auth(Auth::api_key("X-API-Key", API_KEY))
            .with_timeout(Duration::from_millis(120))
            .with_retry_policy(RetryPolicy::none()),
    )
    .expect("client");
    let err = slow
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect_err("timeout");
    assert_no_secret("timeout Display", &err.to_string());
    assert_no_secret("timeout Debug", &format!("{err:?}"));
}

#[tokio::test]
async fn a_paginator_does_not_carry_the_credential_into_debug_output() {
    let registry = FakeRegistry::start(Vec::new()).await;
    let client = RegistryClient::from_config(
        ClientConfig::new(registry.base_url()).with_auth(Auth::bearer(TOKEN)),
    )
    .expect("client");

    let walk = client
        .search_paginator(
            ContractSearchRequest::cursor("swap"),
            PageLimits::default().with_page_size(5),
        )
        .expect("paginator");

    assert_no_secret("Paginator Debug", &format!("{walk:?}"));
}

#[tokio::test]
async fn a_failed_publish_does_not_echo_the_credential() {
    let registry = FakeRegistry::start(vec![Reply::error(
        409,
        r#"{"code":"ContractAlreadyExists","message":"already registered"}"#,
    )])
    .await;
    let client = RegistryClient::from_config(
        ClientConfig::new(registry.base_url())
            .with_auth(Auth::bearer(TOKEN))
            .with_retry_policy(RetryPolicy::none()),
    )
    .expect("client");

    let err = client
        .publish_contract(&publish_request(), Some("release-1".to_string()))
        .await
        .expect_err("409");

    assert_no_secret("publish Display", &err.to_string());
    assert_no_secret("publish Debug", &format!("{err:?}"));
}
