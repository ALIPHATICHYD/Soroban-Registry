//! Typed Rust client for the Soroban Registry HTTP API.
//!
//! The registry's HTTP surface — request construction, authentication, retries,
//! pagination, response decoding, and error mapping — lives here rather than in
//! any one consumer. The CLI, CI integrations, publisher automation, and
//! third-party tools then share one interpretation of pagination tokens,
//! authentication failures, conflict responses, and structured API errors.
//!
//! # Searching
//!
//! ```no_run
//! use registry_client::{ContractSearchRequest, PageLimits, RegistryClient};
//!
//! # async fn example() -> registry_client::Result<()> {
//! let client = RegistryClient::new("https://registry.example")?;
//!
//! // Walk every page of a search, bounded at 1000 items.
//! let mut walk = client.search_paginator(
//!     ContractSearchRequest::cursor("swap").with_networks(["testnet"]),
//!     PageLimits::default().with_max_items(Some(1_000)),
//! )?;
//! while let Some(page) = walk.next_page().await? {
//!     for hit in page.items {
//!         println!("{} — {}", hit.name, hit.contract_id);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Publishing
//!
//! ```no_run
//! use registry_client::{Auth, ClientConfig, Network, PublishRequest, RegistryClient};
//!
//! # async fn example() -> registry_client::Result<()> {
//! let client = RegistryClient::from_config(
//!     ClientConfig::new("https://registry.example").with_auth(Auth::bearer("…token…")),
//! )?;
//!
//! let request = PublishRequest {
//!     contract_id: "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
//!     wasm_hash: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".to_string(),
//!     wasm_artifact_base64: None,
//!     name: "swap-router".to_string(),
//!     slug: None,
//!     description: Some("AMM router".to_string()),
//!     network: Network::Testnet,
//!     category: Some("DeFi".to_string()),
//!     tags: vec!["amm".to_string()],
//!     source_url: None,
//!     publisher_address: "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC".to_string(),
//!     dependencies: Vec::new(),
//!     is_cicd: true,
//! };
//!
//! // With an idempotency key a lost response can be retried safely: the
//! // registry replays the original result instead of publishing twice.
//! match client
//!     .publish_contract(&request, Some("release-1.4.2".to_string()))
//!     .await
//! {
//!     Ok(contract) => println!("published {}", contract.contract_id),
//!     Err(err) if err.is_auth() => eprintln!("log in again: {err}"),
//!     Err(err) => return Err(err),
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Errors
//!
//! Failure modes stay distinct — [`Error::Unauthorized`], [`Error::Forbidden`],
//! [`Error::Validation`], [`Error::Conflict`], [`Error::IdempotencyInProgress`],
//! [`Error::RateLimited`], [`Error::Upstream`], [`Error::Timeout`],
//! [`Error::MalformedResponse`] — and each carries the server's `code`,
//! `message`, structured `details`, request id, and `Retry-After` where present.
//!
//! # Retries
//!
//! [`RetryPolicy`] is explicit and configurable. Safe methods may be retried;
//! a mutation is retried **only** when the call supplies an idempotency key, so
//! a publish is never silently repeated. `Retry-After` is honoured up to a
//! configurable ceiling.
//!
//! # Secrets
//!
//! Bearer tokens, API keys, and webhook signing secrets are held in [`Secret`],
//! whose `Debug`/`Display` print `<redacted>`. Errors carry no headers, so
//! logging a client, a config, or an error cannot leak a credential.
//!
//! # Cancellation
//!
//! [`RegistryClient::with_cancel_token`] aborts in-flight requests, and a
//! [`Paginator`] stops between pages, keeping what it already emitted.

pub mod api;
pub mod client;
pub mod config;
pub mod error;
pub mod http;
pub mod models;
pub mod pagination;

pub use api::contracts::{ContractListFetcher, ContractSearchFetcher};
pub use api::snapshots::RegistrySigningKey;
pub use api::verification::{
    ContractVerifyRequest, VerificationHistoryEntry, VerificationHistoryResponse,
    VerificationStatusResponse, VerificationSubmitResponse,
};
pub use api::webhooks::{CreatedWebhook, WebhookDelivery};
pub use client::RegistryClient;
pub use config::{
    default_user_agent, Auth, ClientConfig, RetryPolicy, Secret, DEFAULT_MAX_ATTEMPTS,
    DEFAULT_TIMEOUT, REDACTED,
};
pub use error::{ApiErrorDetails, Error, PaginationError, Result, TransportKind};
pub use http::{query_pairs_from, RequestSpec, ResponseCache, IDEMPOTENCY_KEY_HEADER};
pub use models::{
    ConfirmOwnershipTransferRequest, Contract, ContractGetResponse, ContractHit,
    ContractSearchParams, ContractSearchRequest, ContractSnapshot, CreateOwnershipTransferRequest,
    CreateWebhookRequest, DependencyScanReport, DeprecateContractRequest, DeprecationInfo, Network,
    OwnershipTransfer, OwnershipTransferLog, PaginatedResponse, PublishRequest, SearchEndpoint,
    UpdateContractMetadataRequest, WebhookConfiguration,
};
pub use pagination::{
    CancelToken, PageCollection, PageCursor, PageFetcher, PageLimits, PageRequest, PaginationMode,
    Paginator, RegistryPage, StopReason, DEFAULT_MAX_PAGES, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE,
};
