// tests/api_surface_tests.rs
//
// The typed endpoint groups, over real HTTP: what each method sends (method,
// path, query, body, headers) and what it decodes. Also covers the retry rules
// for mutations, the response-cache hook, and cancellation.

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use registry_client::{
    Auth, CancelToken, ClientConfig, ContractSearchParams, ContractVerifyRequest,
    CreateWebhookRequest, Error, Network, PageLimits, PaginationMode, PublishRequest,
    RegistryClient, ResponseCache, RetryPolicy, StopReason, UpdateContractMetadataRequest,
    IDEMPOTENCY_KEY_HEADER,
};
use serde_json::json;
use support::{FakeRegistry, Reply};

fn publish_request() -> PublishRequest {
    PublishRequest {
        contract_id: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
        wasm_hash: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".to_string(),
        wasm_artifact_base64: None,
        name: "swap-router".to_string(),
        slug: None,
        description: Some("AMM router".to_string()),
        network: Network::Testnet,
        category: Some("DeFi".to_string()),
        tags: vec!["amm".to_string()],
        source_url: None,
        publisher_address: "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
        dependencies: Vec::new(),
        is_cicd: true,
    }
}

// ── Publish ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn publish_posts_the_typed_request_and_decodes_the_contract() {
    let registry =
        FakeRegistry::start(vec![Reply::ok(&support::contract_body(1).to_string())]).await;

    let contract = registry
        .client()
        .publish_contract(&publish_request(), None)
        .await
        .expect("publish should succeed");

    assert_eq!(contract.name, "contract-1");

    let request = &registry.requests()[0];
    assert_eq!(request.method(), "POST");
    assert_eq!(request.target(), "/api/contracts");
    let body = request.json();
    assert_eq!(body["contract_id"], publish_request().contract_id);
    assert_eq!(body["network"], "testnet");
    assert_eq!(body["is_cicd"], true);
    assert!(
        request.header(IDEMPOTENCY_KEY_HEADER).is_none(),
        "no key was asked for, so none is sent"
    );
}

#[tokio::test]
async fn publish_sends_the_idempotency_key_when_given_one() {
    let registry =
        FakeRegistry::start(vec![Reply::ok(&support::contract_body(1).to_string())]).await;

    registry
        .client()
        .publish_contract(&publish_request(), Some("release-1.4.2".to_string()))
        .await
        .expect("publish should succeed");

    assert_eq!(
        registry.requests()[0].header(IDEMPOTENCY_KEY_HEADER),
        Some("release-1.4.2".to_string())
    );
}

#[tokio::test]
async fn a_keyless_publish_is_never_retried() {
    let registry = FakeRegistry::start(vec![
        Reply::error(503, r#"{"code":"ServiceUnavailable"}"#),
        Reply::ok(&support::contract_body(1).to_string()),
    ])
    .await;
    let client =
        RegistryClient::from_config(ClientConfig::new(registry.base_url()).with_retry_policy(
            RetryPolicy::attempts(3).with_initial_backoff(Duration::from_millis(1)),
        ))
        .expect("client");

    let err = client
        .publish_contract(&publish_request(), None)
        .await
        .expect_err("a keyless mutation must not be repeated");

    assert!(matches!(err, Error::Upstream(_)), "{err:?}");
    assert_eq!(
        registry.requests().len(),
        1,
        "publishing twice could register the contract twice"
    );
}

#[tokio::test]
async fn a_keyed_publish_is_retried_and_replays_the_original_response() {
    let registry = FakeRegistry::start(vec![
        Reply::error(503, r#"{"code":"ServiceUnavailable"}"#),
        Reply::ok(&support::contract_body(1).to_string()),
    ])
    .await;
    let client =
        RegistryClient::from_config(ClientConfig::new(registry.base_url()).with_retry_policy(
            RetryPolicy::attempts(3).with_initial_backoff(Duration::from_millis(1)),
        ))
        .expect("client");

    let contract = client
        .publish_contract(&publish_request(), Some("release-1".to_string()))
        .await
        .expect("the retry should succeed");

    assert_eq!(contract.name, "contract-1");
    let requests = registry.requests();
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert_eq!(
            request.header(IDEMPOTENCY_KEY_HEADER),
            Some("release-1".to_string()),
            "every attempt carries the same key, so the registry can dedupe"
        );
    }
}

#[tokio::test]
async fn mutation_retries_can_be_disabled_even_with_a_key() {
    let registry = FakeRegistry::start(vec![
        Reply::error(503, r#"{"code":"ServiceUnavailable"}"#),
        Reply::ok(&support::contract_body(1).to_string()),
    ])
    .await;
    let client = RegistryClient::from_config(
        ClientConfig::new(registry.base_url())
            .with_retry_policy(RetryPolicy::attempts(3).with_retry_idempotent_mutations(false)),
    )
    .expect("client");

    let err = client
        .publish_contract(&publish_request(), Some("release-1".to_string()))
        .await
        .expect_err("retrying was switched off");

    assert!(matches!(err, Error::Upstream(_)), "{err:?}");
    assert_eq!(registry.requests().len(), 1);
}

#[tokio::test]
async fn an_invalid_idempotency_key_is_rejected_before_sending() {
    let registry = FakeRegistry::start(Vec::new()).await;

    let err = registry
        .client()
        .publish_contract(&publish_request(), Some(String::new()))
        .await
        .expect_err("an empty key is invalid");

    assert!(matches!(err, Error::InvalidRequest(_)), "{err:?}");
    assert!(
        registry.requests().is_empty(),
        "nothing should reach the network"
    );
}

// ── Metadata ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn metadata_updates_patch_the_metadata_path() {
    let registry =
        FakeRegistry::start(vec![Reply::ok(&support::contract_body(2).to_string())]).await;

    let update = UpdateContractMetadataRequest {
        name: Some("renamed".to_string()),
        description: None,
        category: Some("NFT".to_string()),
        tags: Some(vec!["art".to_string()]),
        user_id: None,
    };
    let contract = registry
        .client()
        .update_contract_metadata("11111111-1111-1111-1111-111111111111", &update, None)
        .await
        .expect("update should succeed");

    assert_eq!(contract.name, "contract-2");
    let request = &registry.requests()[0];
    assert_eq!(request.method(), "PATCH");
    assert_eq!(
        request.target(),
        "/api/contracts/11111111-1111-1111-1111-111111111111/metadata"
    );
    assert_eq!(request.json()["name"], "renamed");
    assert_eq!(request.json()["category"], "NFT");
}

#[tokio::test]
async fn contract_metadata_reads_use_the_v1_endpoint() {
    let registry = FakeRegistry::start(vec![Reply::ok(r#"{"contract_id":"CA1","abi":{}}"#)]).await;

    let metadata = registry
        .client()
        .get_contract_metadata("CA1")
        .await
        .expect("metadata should decode");

    assert_eq!(metadata["contract_id"], "CA1");
    assert_eq!(
        registry.requests()[0].target(),
        "/api/v1/contracts/CA1/metadata"
    );
}

#[tokio::test]
async fn get_contract_decodes_the_flattened_response() {
    let mut body = support::contract_body(3);
    body["current_network"] = json!("testnet");
    let registry = FakeRegistry::start(vec![Reply::ok(&body.to_string())]).await;

    let response = registry
        .client()
        .get_contract("contract-3")
        .await
        .expect("contract should decode");

    assert_eq!(response.contract.name, "contract-3");
    assert!(matches!(response.current_network, Some(Network::Testnet)));
    assert_eq!(registry.requests()[0].target(), "/api/contracts/contract-3");
}

// ── List pagination ───────────────────────────────────────────────────────────

#[tokio::test]
async fn the_list_paginator_walks_cursor_pages() {
    let registry = FakeRegistry::start(vec![
        Reply::ok(&support::contract_list_body_with_cursor(
            2,
            3,
            Some("tok-2"),
        )),
        Reply::ok(&support::contract_list_body(1, 3)),
    ])
    .await;

    let mut walk = registry
        .client()
        .list_paginator(
            ContractSearchParams::default(),
            PaginationMode::Cursor,
            PageLimits::default().with_page_size(2),
        )
        .expect("paginator");

    let collected = walk.collect_all().await.expect("walk should succeed");

    assert_eq!(collected.items.len(), 3);
    assert_eq!(collected.total, Some(3));
    assert_eq!(collected.stop_reason, StopReason::Exhausted);

    let targets = registry
        .requests()
        .iter()
        .map(|request| request.target().to_string())
        .collect::<Vec<_>>();
    assert!(
        targets[0].contains("cursor=&") || targets[0].ends_with("cursor="),
        "{:?}",
        targets[0]
    );
    assert!(targets[1].contains("cursor=tok-2"), "{:?}", targets[1]);
}

#[tokio::test]
async fn the_list_paginator_walks_offset_pages_and_stops_at_the_total() {
    let registry = FakeRegistry::start(vec![
        Reply::ok(&support::contract_list_body(2, 3)),
        Reply::ok(&support::contract_list_body(1, 3)),
    ])
    .await;

    let mut walk = registry
        .client()
        .list_paginator(
            ContractSearchParams::default(),
            PaginationMode::Offset,
            PageLimits::default().with_page_size(2),
        )
        .expect("paginator");

    let collected = walk.collect_all().await.expect("walk should succeed");

    assert_eq!(collected.items.len(), 3);
    assert_eq!(collected.stop_reason, StopReason::Exhausted);
    let targets = registry
        .requests()
        .iter()
        .map(|request| request.target().to_string())
        .collect::<Vec<_>>();
    assert_eq!(targets.len(), 2, "the total ends the walk");
    assert!(targets[0].contains("offset=0"), "{:?}", targets[0]);
    assert!(targets[1].contains("offset=2"), "{:?}", targets[1]);
    assert!(
        !targets[1].contains("cursor="),
        "an offset walk never sends a cursor: {:?}",
        targets[1]
    );
}

// ── Verification, deprecation, vulnerabilities ─────────────────────────────────

#[tokio::test]
async fn verification_submission_posts_to_the_contract_verify_path() {
    let registry = FakeRegistry::start(vec![Reply::ok(
        r#"{"verification_id":"33333333-3333-3333-3333-333333333333","contract_id":"CA1","status":"pending","message":"queued","submitted_at":"2026-01-01T00:00:00Z"}"#,
    )])
    .await;

    let request =
        ContractVerifyRequest::new("fn main() {}", "1.81.0", json!({"profile": "release"}))
            .with_notes(Some("first attempt".to_string()));
    let response = registry
        .client()
        .submit_contract_verification("CA1", &request, Some("verify-1".to_string()))
        .await
        .expect("submission should succeed");

    assert_eq!(response.status, "pending");
    let recorded = &registry.requests()[0];
    assert_eq!(recorded.method(), "POST");
    assert_eq!(recorded.target(), "/api/contracts/CA1/verify");
    assert_eq!(recorded.json()["compiler_version"], "1.81.0");
    assert_eq!(recorded.json()["notes"], "first attempt");
    assert_eq!(
        recorded.header(IDEMPOTENCY_KEY_HEADER),
        Some("verify-1".to_string())
    );
}

#[tokio::test]
async fn verification_status_and_history_decode() {
    let registry = FakeRegistry::start(vec![
        Reply::ok(
            r#"{"contract_id":"CA1","verification_status":"verified","is_verified":true,"verified_at":"2026-01-01T00:00:00Z","verification_method":"source","auditor":null,"report_url":null,"verification_notes":null,"cached":true}"#,
        ),
        Reply::ok(
            r#"{"contract_id":"CA1","total":1,"history":[{"id":"44444444-4444-4444-4444-444444444444","from_status":"pending","to_status":"verified","changed_by":null,"notes":null,"created_at":"2026-01-01T00:00:00Z"}]}"#,
        ),
    ])
    .await;
    let client = registry.client();

    let status = client
        .contract_verification_status("CA1")
        .await
        .expect("status should decode");
    assert!(status.is_verified);
    assert!(status.cached);

    let history = client
        .contract_verification_history("CA1")
        .await
        .expect("history should decode");
    assert_eq!(history.total, 1);
    assert_eq!(history.history[0].to_status, "verified");

    assert_eq!(
        registry
            .requests()
            .iter()
            .map(|request| request.target().to_string())
            .collect::<Vec<_>>(),
        vec![
            "/api/contracts/CA1/verification-status",
            "/api/contracts/CA1/verification-history"
        ]
    );
}

#[tokio::test]
async fn deprecation_calls_hit_the_documented_paths() {
    let info = r#"{"contract_id":"11111111-1111-1111-1111-111111111111","is_deprecated":true,"deprecation_status":"deprecated","deprecated_at":"2026-01-01T00:00:00Z","retirement_at":"2026-06-01T00:00:00Z","replacement_contract_id":null,"migration_guide_url":null,"notes":null,"deprecated_reason":"superseded","grace_period_days":null}"#;
    let registry = FakeRegistry::start(vec![Reply::ok(info), Reply::ok(info)]).await;
    let client = registry.client();

    client
        .deprecation_info("11111111-1111-1111-1111-111111111111")
        .await
        .ok();
    client
        .undeprecate_contract("11111111-1111-1111-1111-111111111111", None)
        .await
        .ok();

    let requests = registry.requests();
    assert_eq!(requests[0].method(), "GET");
    assert!(requests[0].target().ends_with("/deprecation-info"));
    assert_eq!(requests[1].method(), "DELETE");
    assert!(requests[1].target().ends_with("/deprecate"));
}

#[tokio::test]
async fn dependency_scan_reports_decode_into_the_shared_model() {
    let registry = FakeRegistry::start(vec![Reply::ok(
        r#"{"contract_id":"11111111-1111-1111-1111-111111111111","status":"vulnerable","dependencies_scanned":12,"vulnerable_dependency_count":1,"last_scanned_at":"2026-01-01T00:00:00Z","findings":[]}"#,
    )])
    .await;

    let report = registry
        .client()
        .dependency_scan_report("11111111-1111-1111-1111-111111111111")
        .await
        .expect("report should decode");

    assert_eq!(report.status, "vulnerable");
    assert_eq!(report.dependencies_scanned, 12);
    assert!(registry.requests()[0]
        .target()
        .ends_with("/dependency-scan"));
}

// ── Webhooks ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn creating_a_webhook_lifts_the_signing_secret_out_of_the_model() {
    let registry = FakeRegistry::start(vec![Reply::ok(
        r#"{"id":"55555555-5555-5555-5555-555555555555","user_id":null,"organization_id":null,"name":"ci","url":"https://ci.example/hook","notification_types":[],"is_active":true,"verify_ssl":true,"custom_headers":null,"rate_limit_per_minute":null,"total_deliveries":0,"failed_deliveries":0,"last_delivery_at":null,"last_success_at":null,"last_failure_at":null,"consecutive_failures":0,"secret":"whsec_super_secret","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z"}"#,
    )])
    .await;

    let created = registry
        .client()
        .create_webhook(
            &CreateWebhookRequest {
                name: "ci".to_string(),
                url: "https://ci.example/hook".to_string(),
                notification_types: Vec::new(),
                secret: None,
                verify_ssl: Some(true),
                custom_headers: None,
            },
            None,
        )
        .await
        .expect("creation should succeed");

    assert_eq!(created.webhook.name, "ci");
    assert!(
        created.webhook.secret.is_none(),
        "the secret is moved out of the loggable model"
    );
    assert_eq!(
        created.secret.as_ref().map(|secret| secret.expose()),
        Some("whsec_super_secret")
    );
    let rendered = format!("{created:?}");
    assert!(
        !rendered.contains("whsec_super_secret"),
        "debug output must not print the signing secret: {rendered}"
    );
}

#[tokio::test]
async fn webhook_deletes_and_test_sends_tolerate_empty_bodies() {
    let registry = FakeRegistry::start(vec![Reply::no_content(), Reply::no_content()]).await;
    let client = registry.client();

    client
        .delete_webhook("55555555-5555-5555-5555-555555555555", None)
        .await
        .expect("delete should succeed");
    client
        .test_webhook("55555555-5555-5555-5555-555555555555", None)
        .await
        .expect("test send should succeed");

    let requests = registry.requests();
    assert_eq!(requests[0].method(), "DELETE");
    assert_eq!(requests[1].method(), "POST");
    assert!(requests[1].target().ends_with("/test"));
}

// ── Snapshots ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn the_registry_signing_key_decodes() {
    let registry = FakeRegistry::start(vec![Reply::ok(
        r#"{"algorithm":"ed25519","public_key":"BASE64KEY","key_fingerprint":"ab12"}"#,
    )])
    .await;

    let key = registry
        .client()
        .registry_signing_key()
        .await
        .expect("key should decode");

    assert_eq!(key.algorithm, "ed25519");
    assert_eq!(key.key_fingerprint, "ab12");
    assert_eq!(registry.requests()[0].target(), "/api/registry/signing-key");
}

// ── Cross-cutting behaviour ───────────────────────────────────────────────────

#[tokio::test]
async fn every_request_carries_the_configured_user_agent_and_credential() {
    let registry = FakeRegistry::start(vec![Reply::ok(&support::contract_list_body(1, 1))]).await;
    let client = RegistryClient::from_config(
        ClientConfig::new(registry.base_url())
            .with_user_agent("my-tool/9.9")
            .with_auth(Auth::bearer("token-abc")),
    )
    .expect("client");

    client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect("list should succeed");

    let request = &registry.requests()[0];
    assert_eq!(
        request.header("user-agent"),
        Some("my-tool/9.9".to_string())
    );
    assert_eq!(
        request.header("authorization"),
        Some("Bearer token-abc".to_string()),
        "the credential must reach the wire even though it is redacted in logs"
    );
}

#[tokio::test]
async fn an_api_key_scheme_uses_its_own_header() {
    let registry = FakeRegistry::start(vec![Reply::ok(&support::contract_list_body(1, 1))]).await;
    let client = RegistryClient::from_config(
        ClientConfig::new(registry.base_url()).with_auth(Auth::api_key("X-API-Key", "key-xyz")),
    )
    .expect("client");

    client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect("list should succeed");

    let request = &registry.requests()[0];
    assert_eq!(request.header("x-api-key"), Some("key-xyz".to_string()));
    assert!(request.header("authorization").is_none());
}

#[tokio::test]
async fn the_response_cache_hook_serves_repeat_reads() {
    #[derive(Default)]
    struct CountingCache {
        entries: Mutex<Vec<(String, String)>>,
        hits: AtomicUsize,
    }

    impl ResponseCache for CountingCache {
        fn get(&self, key: &str) -> Option<String> {
            let entries = self.entries.lock().expect("cache");
            let hit = entries
                .iter()
                .find(|(cached, _)| cached == key)
                .map(|(_, body)| body.clone());
            if hit.is_some() {
                self.hits.fetch_add(1, Ordering::SeqCst);
            }
            hit
        }

        fn put(&self, key: &str, body: &str) {
            self.entries
                .lock()
                .expect("cache")
                .push((key.to_string(), body.to_string()));
        }
    }

    let registry = FakeRegistry::start(vec![Reply::ok(&support::contract_list_body(1, 1))]).await;
    let cache = Arc::new(CountingCache::default());
    let client = RegistryClient::from_config(ClientConfig::new(registry.base_url()))
        .expect("client")
        .with_response_cache(cache.clone());

    let params = ContractSearchParams {
        limit: Some(1),
        ..Default::default()
    };
    client.list_contracts(&params).await.expect("first read");
    client.list_contracts(&params).await.expect("second read");

    assert_eq!(
        registry.requests().len(),
        1,
        "the second read came from the cache"
    );
    assert_eq!(cache.hits.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn mutations_are_never_served_from_the_cache() {
    struct AlwaysHit;

    impl ResponseCache for AlwaysHit {
        fn get(&self, _key: &str) -> Option<String> {
            Some(support::contract_body(99).to_string())
        }
        fn put(&self, _key: &str, _body: &str) {}
    }

    let registry =
        FakeRegistry::start(vec![Reply::ok(&support::contract_body(1).to_string())]).await;
    let client = RegistryClient::from_config(ClientConfig::new(registry.base_url()))
        .expect("client")
        .with_response_cache(Arc::new(AlwaysHit));

    let contract = client
        .publish_contract(&publish_request(), None)
        .await
        .expect("publish should succeed");

    assert_eq!(contract.name, "contract-1", "the cache was bypassed");
    assert_eq!(registry.requests().len(), 1);
}

#[tokio::test]
async fn a_cancelled_client_abandons_the_request() {
    let registry = FakeRegistry::start(vec![Reply::hang()]).await;
    let cancel = CancelToken::new();
    let client = RegistryClient::from_config(ClientConfig::new(registry.base_url()))
        .expect("client")
        .with_cancel_token(cancel.clone());

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel.cancel();
    });

    let err = client
        .list_contracts(&ContractSearchParams::default())
        .await
        .expect_err("cancellation must surface");

    assert!(matches!(err, Error::Cancelled { .. }), "{err:?}");
}

#[tokio::test]
async fn the_escape_hatch_shares_the_clients_behaviour() {
    let registry = FakeRegistry::start(vec![Reply::ok(r#"["defi","amm"]"#)]).await;
    let client = RegistryClient::from_config(
        ClientConfig::new(registry.base_url()).with_auth(Auth::bearer("token-abc")),
    )
    .expect("client");

    let tags: Vec<String> = client
        .get_json("/api/contracts/tags")
        .await
        .expect("tags should decode");

    assert_eq!(tags, vec!["defi", "amm"]);
    assert_eq!(
        registry.requests()[0].header("authorization"),
        Some("Bearer token-abc".to_string())
    );
}
