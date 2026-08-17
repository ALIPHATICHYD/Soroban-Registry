//! Typed client for the Soroban Registry HTTP API.
//!
//! The registry paginates in two ways — stable keyset **cursor** pagination
//! served by PostgreSQL, and **offset** pagination served by Elasticsearch (see
//! `docs/pagination.md`). Consumers used to have to know which endpoint uses
//! which, and manage continuation state by hand: easy to mix a cursor with an
//! offset, lose results, re-read a page, or loop forever on a bad token.
//!
//! This crate wraps both behind one abstraction. Pick a mode, get a
//! [`Paginator`], and iterate:
//!
//! ```no_run
//! use futures_util::StreamExt;
//! use registry_client::{ContractSearchRequest, PageLimits, RegistryClient};
//!
//! # async fn example() -> registry_client::Result<()> {
//! let client = RegistryClient::new("http://localhost:3001")?;
//! let request = ContractSearchRequest::cursor("swap").with_networks(["testnet"]);
//!
//! // Page at a time…
//! let mut walk = client.search_paginator(
//!     request.clone(),
//!     PageLimits::default().with_max_items(Some(1_000)),
//! )?;
//! while let Some(page) = walk.next_page().await? {
//!     println!("{} result(s) of {:?}", page.items.len(), page.total);
//! }
//!
//! // …or as a stream of items. The stream is not `Unpin`, so pin it to poll it.
//! let mut items = Box::pin(
//!     client
//!         .search_paginator(request, PageLimits::default().with_max_items(Some(1_000)))?
//!         .items(),
//! );
//! while let Some(hit) = items.next().await {
//!     println!("{}", hit?.name);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Every walk is bounded, cancellable, and refuses to follow a continuation that
//! cannot make progress — see [`pagination`] for the full list of guarantees.

pub mod client;
pub mod error;
pub mod models;
pub mod pagination;

pub use client::{ContractSearchFetcher, RegistryClient};
pub use error::{Error, PaginationError, Result};
pub use models::{ContractHit, ContractSearchRequest, SearchEndpoint};
pub use pagination::{
    CancelToken, PageCollection, PageCursor, PageFetcher, PageLimits, PageRequest, PaginationMode,
    Paginator, RegistryPage, StopReason, DEFAULT_MAX_PAGES, DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE,
};
