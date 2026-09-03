//! Contracts: list, search, read, metadata, and publish.

use shared::models::{
    Contract, ContractGetResponse, ContractSearchParams, PaginatedResponse, PublishRequest,
    UpdateContractMetadataRequest,
};

use crate::error::{Error, Result};
use crate::http::{query_pairs_from, RequestSpec};
use crate::models::{ContractHit, ContractSearchRequest, RawSearchResponse};
use crate::pagination::{
    advance_offset, PageCursor, PageFetcher, PageFuture, PageLimits, PageRequest, PaginationMode,
    Paginator, RegistryPage,
};
use crate::RegistryClient;

/// `GET /api/contracts` — the list endpoint.
pub const CONTRACTS_PATH: &str = "/api/contracts";

impl RegistryClient {
    // ── List ─────────────────────────────────────────────────────────────────

    /// One page of `GET /api/contracts`.
    ///
    /// `params` is the API's own filter type, so every documented filter is
    /// available without a parallel struct to keep in sync.
    pub async fn list_contracts(
        &self,
        params: &ContractSearchParams,
    ) -> Result<PaginatedResponse<Contract>> {
        let mut spec = RequestSpec::get(CONTRACTS_PATH);
        spec.query = query_pairs_from(params)?;
        self.transport.send_json(spec).await
    }

    /// A paginator that walks `GET /api/contracts`.
    ///
    /// Cursor mode follows `next_cursor`; offset mode advances `offset` and
    /// stops at the reported total. Mixing the two is rejected up front.
    pub fn list_paginator(
        &self,
        params: ContractSearchParams,
        mode: PaginationMode,
        limits: PageLimits,
    ) -> Result<Paginator<ContractListFetcher>> {
        let start = match mode {
            PaginationMode::Cursor => {
                if let Some(offset) = params.offset {
                    if offset > 0 {
                        return Err(crate::PaginationError::MixedPagination {
                            offset: offset as u64,
                        }
                        .into());
                    }
                }
                params
                    .cursor
                    .as_ref()
                    .filter(|token| !token.is_empty())
                    .map(|token| PageCursor::Cursor(token.clone()))
            }
            PaginationMode::Offset => {
                if params.cursor.is_some() {
                    return Err(crate::PaginationError::ModeMismatch {
                        expected: PaginationMode::Offset,
                        returned: PaginationMode::Cursor,
                    }
                    .into());
                }
                params
                    .offset
                    .filter(|offset| *offset > 0)
                    .map(|offset| PageCursor::Offset {
                        offset: offset as u64,
                    })
            }
        };

        let fetcher = ContractListFetcher {
            client: self.clone(),
            params,
            mode,
        };
        Paginator::new(fetcher, mode)
            .with_limits(limits)
            .start_at(start)
    }

    // ── Read ─────────────────────────────────────────────────────────────────

    /// `GET /api/contracts/{id}` — by registry UUID, contract address, or slug.
    pub async fn get_contract(&self, id: &str) -> Result<ContractGetResponse> {
        self.transport
            .send_json(RequestSpec::get(contract_path(id, "")))
            .await
    }

    /// `GET /api/v1/contracts/{id}/metadata`.
    ///
    /// The endpoint composes fields from several tables and has no single
    /// backend type, so the document is returned as-is.
    pub async fn get_contract_metadata(&self, id: &str) -> Result<serde_json::Value> {
        self.transport
            .send_json(RequestSpec::get(format!("/api/v1/contracts/{id}/metadata")))
            .await
    }

    // ── Write ────────────────────────────────────────────────────────────────

    /// `PATCH /api/contracts/{id}/metadata`.
    ///
    /// A metadata update is not retried unless `idempotency_key` is supplied:
    /// repeating it would append a spurious version-history entry.
    pub async fn update_contract_metadata(
        &self,
        id: &str,
        request: &UpdateContractMetadataRequest,
        idempotency_key: Option<String>,
    ) -> Result<Contract> {
        let spec = RequestSpec::patch(contract_path(id, "/metadata"))
            .json_body(request)?
            .idempotency_key(idempotency_key)?;
        self.transport.send_json(spec).await
    }

    /// `POST /api/contracts` — publish a contract.
    ///
    /// Pass an `idempotency_key` for anything that may be retried (CI, a flaky
    /// link): the registry replays the original response instead of registering
    /// a second time, and only then will this client retry the request itself.
    /// Without a key the request is sent exactly once.
    ///
    /// A key that is still being processed surfaces as
    /// [`Error::IdempotencyInProgress`] rather than a generic conflict.
    pub async fn publish_contract(
        &self,
        request: &PublishRequest,
        idempotency_key: Option<String>,
    ) -> Result<Contract> {
        let spec = RequestSpec::post(CONTRACTS_PATH)
            .json_body(request)?
            .idempotency_key(idempotency_key)?;
        self.transport.send_json(spec).await
    }

    // ── Search ───────────────────────────────────────────────────────────────

    /// Fetch a single page of search results.
    ///
    /// `continuation` is `None` for the first page. Most callers want
    /// [`RegistryClient::search_paginator`] instead, which manages continuations.
    pub async fn search_page(
        &self,
        request: &ContractSearchRequest,
        continuation: Option<PageCursor>,
        limit: u32,
    ) -> Result<RegistryPage<ContractHit>> {
        request.validate()?;

        let mode = request.mode;
        let endpoint = request.effective_endpoint();
        let mut spec = RequestSpec::get(endpoint.path())
            .query_pair("q", request.query.clone())
            .query_pair("limit", limit.to_string());

        // Offset walks report where they are; cursor walks send the opaque token
        // (empty on the first page, which is how the API is asked for a cursor
        // walk at all). The two are never sent together.
        let mut current_offset = 0_u64;
        match mode {
            PaginationMode::Cursor => {
                let token = continuation
                    .as_ref()
                    .and_then(|cursor| cursor.as_cursor())
                    .unwrap_or("");
                spec = spec.query_pair("cursor", token);
            }
            PaginationMode::Offset => {
                current_offset = continuation
                    .as_ref()
                    .and_then(|cursor| cursor.as_offset())
                    .or(request.offset)
                    .unwrap_or(0);
                spec = spec.query_pair("offset", current_offset.to_string());
            }
        }

        if !request.networks.is_empty() {
            spec = spec.query_pair("networks", request.networks.join(","));
        }
        if !request.categories.is_empty() {
            spec = spec.query_pair("categories", request.categories.join(","));
        }
        if !request.tags.is_empty() {
            spec = spec.query_pair("tags", request.tags.join(","));
        }
        if request.verified_only {
            spec = spec.query_pair("verified_only", "true");
        }

        let raw: RawSearchResponse = self.transport.send_json(spec).await?;
        raw.into_page(mode, current_offset, limit)
    }

    /// A paginator that walks the whole result set for `request`, subject to
    /// `limits`.
    ///
    /// Validates the request up front, so mixing a cursor with an offset fails
    /// here rather than part-way through a walk.
    pub fn search_paginator(
        &self,
        request: ContractSearchRequest,
        limits: PageLimits,
    ) -> Result<Paginator<ContractSearchFetcher>> {
        let start = request.start_continuation()?;
        let mode = request.mode;
        let fetcher = ContractSearchFetcher {
            client: self.clone(),
            request,
        };

        Paginator::new(fetcher, mode)
            .with_limits(limits)
            .start_at(start)
    }
}

/// Path for a contract sub-resource, with the id percent-safe enough for the
/// identifier shapes the registry accepts (UUID, `C…` address, or slug).
fn contract_path(id: &str, suffix: &str) -> String {
    format!("{CONTRACTS_PATH}/{}{suffix}", id.trim())
}

/// Fetches search pages for one request. Built by
/// [`RegistryClient::search_paginator`].
pub struct ContractSearchFetcher {
    client: RegistryClient,
    request: ContractSearchRequest,
}

impl ContractSearchFetcher {
    pub fn request(&self) -> &ContractSearchRequest {
        &self.request
    }
}

impl PageFetcher for ContractSearchFetcher {
    type Item = ContractHit;

    fn fetch_page(&self, request: PageRequest) -> PageFuture<'_, ContractHit> {
        Box::pin(async move {
            self.client
                .search_page(&self.request, request.cursor, request.limit)
                .await
        })
    }
}

/// Fetches list pages. Built by [`RegistryClient::list_paginator`].
pub struct ContractListFetcher {
    client: RegistryClient,
    params: ContractSearchParams,
    mode: PaginationMode,
}

impl PageFetcher for ContractListFetcher {
    type Item = Contract;

    fn fetch_page(&self, request: PageRequest) -> PageFuture<'_, Contract> {
        Box::pin(async move {
            let mut params = self.params.clone();
            params.limit = Some(i64::from(request.limit));

            let current_offset = match self.mode {
                PaginationMode::Cursor => {
                    params.offset = None;
                    params.page = None;
                    // An empty cursor opts the endpoint into a keyset walk.
                    params.cursor = Some(
                        request
                            .cursor
                            .as_ref()
                            .and_then(PageCursor::as_cursor)
                            .unwrap_or("")
                            .to_string(),
                    );
                    0
                }
                PaginationMode::Offset => {
                    params.cursor = None;
                    let offset = request
                        .cursor
                        .as_ref()
                        .and_then(PageCursor::as_offset)
                        .or(params.offset.map(|offset| offset.max(0) as u64))
                        .unwrap_or(0);
                    params.offset = Some(i64::try_from(offset).map_err(|_| {
                        Error::InvalidRequest(format!("offset {offset} exceeds the API's range"))
                    })?);
                    offset
                }
            };

            let page = self.client.list_contracts(&params).await?;
            let total = u64::try_from(page.total).ok();
            let page_len = page.items.len() as u64;

            let next = match self.mode {
                PaginationMode::Cursor => page
                    .next_cursor
                    .clone()
                    .filter(|token| !token.is_empty())
                    .map(PageCursor::Cursor),
                PaginationMode::Offset => {
                    if page_len == 0 {
                        None
                    } else {
                        let next_offset = advance_offset(current_offset, page_len)?;
                        let has_more = match total {
                            Some(total) => next_offset < total,
                            None => page_len >= u64::from(request.limit),
                        };
                        has_more.then_some(PageCursor::Offset {
                            offset: next_offset,
                        })
                    }
                }
            };

            Ok(RegistryPage::new(page.items, next, total))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PaginationError;

    fn client() -> RegistryClient {
        RegistryClient::new("http://registry.test").expect("client")
    }

    #[test]
    fn contract_sub_resource_paths_are_built_from_the_id() {
        assert_eq!(
            contract_path("11111111-1111-1111-1111-111111111111", "/metadata"),
            "/api/contracts/11111111-1111-1111-1111-111111111111/metadata"
        );
        assert_eq!(contract_path(" CA123 ", ""), "/api/contracts/CA123");
    }

    #[test]
    fn a_cursor_list_walk_rejects_an_offset() {
        let params = ContractSearchParams {
            offset: Some(40),
            ..Default::default()
        };
        let err = client()
            .list_paginator(params, PaginationMode::Cursor, PageLimits::default())
            .expect_err("cursor + offset must be rejected");
        assert!(matches!(
            err,
            Error::Pagination(PaginationError::MixedPagination { offset: 40 })
        ));
    }

    #[test]
    fn an_offset_list_walk_rejects_a_cursor() {
        let params = ContractSearchParams {
            cursor: Some("token".to_string()),
            ..Default::default()
        };
        let err = client()
            .list_paginator(params, PaginationMode::Offset, PageLimits::default())
            .expect_err("offset + cursor must be rejected");
        assert!(matches!(
            err,
            Error::Pagination(PaginationError::ModeMismatch { .. })
        ));
    }

    #[test]
    fn list_params_flatten_into_the_documented_query_shape() {
        let params = ContractSearchParams {
            query: Some("swap".to_string()),
            limit: Some(20),
            offset: Some(40),
            verified_only: Some(true),
            ..Default::default()
        };
        let pairs = query_pairs_from(&params).expect("flatten");

        assert!(pairs.contains(&("query".to_string(), "swap".to_string())));
        assert!(pairs.contains(&("limit".to_string(), "20".to_string())));
        assert!(pairs.contains(&("offset".to_string(), "40".to_string())));
        assert!(pairs.contains(&("verified_only".to_string(), "true".to_string())));
        assert!(
            !pairs.iter().any(|(key, _)| key == "cursor"),
            "absent filters are not sent"
        );
    }
}
