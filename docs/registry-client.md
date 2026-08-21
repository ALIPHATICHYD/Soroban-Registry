# `soroban-registry-client`

A typed Rust client for the Soroban Registry HTTP API, in
[`backend/registry_client`](../backend/registry_client). Request construction,
authentication, retries, pagination, response decoding and error mapping live
here rather than in any one consumer, so the CLI, CI integrations, publisher
automation and third-party tools all share one interpretation of pagination
tokens, authentication failures, conflict responses and structured API errors.

The crate is named `soroban-registry-client`; the library it exposes is
`registry_client`:

```toml
[dependencies]
soroban-registry-client = { path = "../backend/registry_client" }
```

```rust
use registry_client::RegistryClient;
```

## Configuring a client

```rust
use std::time::Duration;
use registry_client::{Auth, ClientConfig, RegistryClient, RetryPolicy};

let client = RegistryClient::from_config(
    ClientConfig::new("https://registry.example")
        .with_auth(Auth::bearer(std::env::var("REGISTRY_TOKEN")?))
        .with_timeout(Duration::from_secs(15))
        .with_user_agent("my-tool/1.0")
        .with_retry_policy(RetryPolicy::attempts(5)),
)?;
```

| Knob | Default | Notes |
| --- | --- | --- |
| `base_url` | — | Trailing slashes are trimmed. |
| `auth` | `Auth::None` | `Auth::bearer(token)` or `Auth::api_key(header, value)`. |
| `timeout` | 30s | Applied per attempt, not per call. |
| `user_agent` | `soroban-registry-client/<version>` | Identifies the SDK to the registry. |
| `retry` | 3 attempts, 250 ms initial backoff, x3 growth, honours `Retry-After` | See [Retries](#retries). |

`RegistryClient::with_http_client` takes your own `reqwest::Client` when you
need custom TLS, proxies or pooling. Cloning a client is cheap and shares the
connection pool; `with_*` on a clone does not disturb the original.

## Searching

```rust
use registry_client::{ContractSearchRequest, PageLimits, RegistryClient};

let client = RegistryClient::new("https://registry.example")?;

// One page.
let page = client
    .search_page(&ContractSearchRequest::offset("swap"), None, 20)
    .await?;
println!("{} of {:?} matches", page.items.len(), page.total);

// Every page, bounded at 1000 items.
let mut walk = client.search_paginator(
    ContractSearchRequest::cursor("swap")
        .with_networks(["testnet"])
        .with_categories(["DeFi"])
        .with_verified_only(true),
    PageLimits::default().with_page_size(50).with_max_items(Some(1_000)),
)?;
while let Some(page) = walk.next_page().await? {
    for hit in page.items {
        println!("{} — {}", hit.name, hit.contract_id);
    }
}
```

Listing works the same way, with the API's own filter type and full `Contract`
models:

```rust
use registry_client::{ContractSearchParams, PageLimits, PaginationMode};

let page = client
    .list_contracts(&ContractSearchParams {
        limit: Some(20),
        verified_only: Some(true),
        ..Default::default()
    })
    .await?;

let mut walk = client.list_paginator(
    ContractSearchParams::default(),
    PaginationMode::Cursor,
    PageLimits::default().with_max_items(Some(5_000)),
)?;
let all = walk.collect_all().await?;
```

Cursor and offset pagination are distinct typed representations
(`PageCursor::Cursor` vs `PageCursor::Offset`), cursors stay opaque, and the two
can never be combined — see [pagination.md](./pagination.md) for the full set of
guarantees.

## Publishing

```rust
use registry_client::{Network, PublishRequest};

let request = PublishRequest {
    contract_id: "CDLZ…".to_string(),
    wasm_hash: "9f86d081…".to_string(),
    wasm_artifact_base64: None,
    name: "swap-router".to_string(),
    slug: None,
    description: Some("AMM router".to_string()),
    network: Network::Testnet,
    category: Some("DeFi".to_string()),
    tags: vec!["amm".to_string()],
    source_url: None,
    publisher_address: "GDLZ…".to_string(),
    dependencies: Vec::new(),
    is_cicd: true,
};

// The key makes a lost response safe to retry: the registry replays the
// original result instead of registering the contract twice.
let contract = client
    .publish_contract(&request, Some("release-1.4.2".to_string()))
    .await?;
println!("published {}", contract.contract_id);
```

Other mutating calls take the same optional key: `update_contract_metadata`,
`submit_contract_verification`, `deprecate_contract`, `trigger_dependency_scan`,
`create_ownership_transfer`, `confirm_ownership_transfer`, and the webhook
methods.

## Endpoint groups

| Group | Methods |
| --- | --- |
| Contracts | `list_contracts`, `list_paginator`, `get_contract`, `get_contract_metadata`, `update_contract_metadata`, `publish_contract`, `search_page`, `search_paginator` |
| Verification | `submit_contract_verification`, `contract_verification_status`, `contract_verification_history` |
| Deprecation | `deprecation_info`, `deprecate_contract`, `undeprecate_contract` |
| Vulnerabilities | `dependency_scan_report`, `trigger_dependency_scan`, `vulnerability_assessment` |
| Ownership transfer | `create_ownership_transfer`, `list_ownership_transfers`, `get_ownership_transfer`, `confirm_ownership_transfer`, `ownership_transfer_logs` (provenance chain) |
| Webhooks | `list_webhooks`, `create_webhook`, `delete_webhook`, `webhook_deliveries`, `test_webhook`, `retry_webhook_delivery` |
| Snapshots | `contract_snapshot`, `registry_signing_key` |

Request and response types come from `backend/shared` wherever the backend
defines them (`Contract`, `PublishRequest`, `PaginatedResponse<T>`,
`DeprecationInfo`, `OwnershipTransfer`, `ContractSnapshot`, …). Where a type
lives in the API crate instead, the client mirrors it with the same wire shape
(`ContractVerifyRequest`, `WebhookDelivery`, `RegistrySigningKey`).

For an endpoint the crate does not wrap yet, the escape hatch shares all of the
same behaviour:

```rust
use registry_client::RequestSpec;

let tags: Vec<String> = client.get_json("/api/contracts/tags").await?;
let report: serde_json::Value = client
    .send_json(RequestSpec::get("/api/contracts/trending").query_pair("limit", "10"))
    .await?;
```

## Errors

Failure modes stay distinct instead of collapsing into "request failed":

| Variant | When | `is_transient()` |
| --- | --- | --- |
| `Unauthorized` | 401 — expired or missing credentials | no |
| `Forbidden` | 403 — authenticated but not allowed | no |
| `NotFound` | 404 | no |
| `Validation` | 400 / 422 | no |
| `Conflict` | 409 | no |
| `IdempotencyInProgress` | 409 `IdempotencyKeyInProgress` — same key still running | yes |
| `RateLimited` | 429, with `retry_after` | yes |
| `Upstream` | 5xx | yes |
| `Timeout` | the attempt exceeded `timeout` | yes |
| `Transport` | connect/send failure | yes |
| `MalformedResponse` | 2xx body the client cannot decode | no |
| `Cancelled` | the caller's `CancelToken` fired | no |
| `Pagination` | a walk could not continue safely | no |

Every response-derived variant carries the server's `code`, `message`,
structured `details`, request id and `Retry-After`:

```rust
match client.publish_contract(&request, None).await {
    Ok(contract) => println!("published {}", contract.contract_id),
    Err(err) if err.is_auth() => eprintln!("log in again: {err}"),
    Err(err) if err.code() == Some("ContractAlreadyExists") => {
        eprintln!("already registered: {:?}", err.error_details());
    }
    Err(err) if err.is_transient() => {
        eprintln!("try later{}", match err.retry_after() {
            Some(after) => format!(" (after {}s)", after.as_secs()),
            None => String::new(),
        });
    }
    Err(err) => return Err(err.into()),
}
```

## Retries

`RetryPolicy` is explicit and configurable:

- **Safe methods** (`GET`/`HEAD`/`OPTIONS`) are retried on transport faults,
  timeouts, 408, 429 and 5xx.
- **Mutations are retried only when the call supplies an idempotency key.** A
  keyless publish is attempted exactly once, so it can never register twice.
  `RetryPolicy::with_retry_idempotent_mutations(false)` disables even that.
- `Retry-After` wins over the computed backoff, up to `max_retry_after`
  (default 60s). A longer one is surfaced as `Error::RateLimited` rather than
  blocking the call.
- `RetryPolicy::none()` disables retrying entirely.

## Cancellation

```rust
use registry_client::CancelToken;

let cancel = CancelToken::new();
let client = client.with_cancel_token(cancel.clone());

tokio::spawn(async move {
    tokio::signal::ctrl_c().await.ok();
    cancel.cancel();
});
```

In-flight requests fail with `Error::Cancelled`; a `Paginator` stops between
pages and keeps what it already emitted.

## Secrets

Bearer tokens, API keys and webhook signing secrets are held in `Secret`, whose
`Debug`/`Display` print `<redacted>`. Errors never carry request headers. So
logging a client, a config, a paginator or an error cannot leak a credential:

```rust
let client = RegistryClient::new(url)?.with_bearer_token(Some(token));
println!("{client:?}"); // …auth: Bearer(<redacted>)…
```

A newly created webhook's signing secret is moved out of the loggable model into
`CreatedWebhook::secret`; reach it deliberately with `Secret::expose`.

## Response cache hook

Consumers with their own cache can serve safe reads from it:

```rust
use registry_client::ResponseCache;

struct MyCache;
impl ResponseCache for MyCache {
    fn get(&self, key: &str) -> Option<String> { /* … */ None }
    fn put(&self, key: &str, body: &str) { /* … */ }
}

let client = client.with_response_cache(std::sync::Arc::new(MyCache));
```

Only cacheable `GET`s are looked up or stored; mutations always go to the
network. The CLI plugs its on-disk HTTP cache in this way, which is what keeps
`--no-cache` working across commands.
