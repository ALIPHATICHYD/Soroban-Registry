//! Request and response models for the registry search endpoints.

use serde::{Deserialize, Serialize};

use crate::error::{Error, PaginationError, Result};
use crate::pagination::{advance_offset, PageCursor, PaginationMode};

// Request and response types the registry already defines are re-exported
// rather than redeclared, so a consumer never has to keep a parallel copy in
// sync with the backend.
pub use shared::models::{
    ConfirmOwnershipTransferRequest, Contract, ContractGetResponse, ContractSearchParams,
    CreateOwnershipTransferRequest, CreateWebhookRequest, DependencyScanReport,
    DeprecateContractRequest, DeprecationInfo, Network, OwnershipTransfer, OwnershipTransferLog,
    PaginatedResponse, PublishRequest, UpdateContractMetadataRequest, WebhookConfiguration,
};
pub use shared::snapshot::ContractSnapshot;

/// One search result, normalised across the registry's search endpoints.
///
/// `GET /api/search` names the array `contracts` and `GET /api/v1/contracts/search`
/// names it `results`; both carry the same fields, so consumers see one type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractHit {
    /// Registry UUID.
    #[serde(default)]
    pub id: String,
    /// On-chain contract address.
    #[serde(default)]
    pub contract_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub is_verified: bool,
    #[serde(default)]
    pub is_deprecated: bool,
    #[serde(default)]
    pub deprecation_status: Option<String>,
    #[serde(default)]
    pub replacement_contract_id: Option<String>,
    #[serde(default)]
    pub relevance_score: Option<f64>,
}

/// Which registry endpoint serves a search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchEndpoint {
    /// `GET /api/search` — PostgreSQL full-text search. Serves cursor pagination
    /// and is the only search endpoint that filters by tag.
    FullText,
    /// `GET /api/v1/contracts/search` — advanced search, Elasticsearch-backed
    /// with a PostgreSQL fallback. Offset pagination.
    AdvancedV1,
}

impl SearchEndpoint {
    pub fn path(self) -> &'static str {
        match self {
            SearchEndpoint::FullText => "/api/search",
            SearchEndpoint::AdvancedV1 => "/api/v1/contracts/search",
        }
    }
}

/// A contract search, independent of how it is paginated.
///
/// The pagination mode is explicit: the client never guesses, because cursor and
/// offset pagination are served by different backends with different ordering
/// guarantees (see `docs/pagination.md`).
#[derive(Debug, Clone)]
pub struct ContractSearchRequest {
    /// Full-text query. Required — the API rejects an empty one.
    pub query: String,
    /// Network filters (`mainnet`, `testnet`, `futurenet`).
    pub networks: Vec<String>,
    pub categories: Vec<String>,
    /// Tag filters. Only `GET /api/search` supports these.
    pub tags: Vec<String>,
    pub verified_only: bool,
    pub mode: PaginationMode,
    /// Opaque continuation token to resume a cursor walk from. Cursor mode only.
    pub cursor: Option<String>,
    /// Starting row offset. Offset mode only.
    pub offset: Option<u64>,
    /// Force a specific endpoint. Defaults to the one that natively serves the
    /// chosen pagination mode.
    pub endpoint: Option<SearchEndpoint>,
}

impl ContractSearchRequest {
    /// A cursor-paginated search — stable under concurrent writes, and the right
    /// default for walking a whole result set.
    pub fn cursor(query: impl Into<String>) -> Self {
        Self::new(query, PaginationMode::Cursor)
    }

    /// An offset-paginated search — relevance ordered, jump-to-page capable.
    pub fn offset(query: impl Into<String>) -> Self {
        Self::new(query, PaginationMode::Offset)
    }

    pub fn new(query: impl Into<String>, mode: PaginationMode) -> Self {
        Self {
            query: query.into(),
            networks: Vec::new(),
            categories: Vec::new(),
            tags: Vec::new(),
            verified_only: false,
            mode,
            cursor: None,
            offset: None,
            endpoint: None,
        }
    }

    pub fn with_networks<I, S>(mut self, networks: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.networks = networks.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_categories<I, S>(mut self, categories: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.categories = categories.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_verified_only(mut self, verified_only: bool) -> Self {
        self.verified_only = verified_only;
        self
    }

    /// Resume a cursor walk. The token stays opaque — it is sent back exactly as
    /// received.
    pub fn with_cursor(mut self, cursor: Option<String>) -> Self {
        self.cursor = cursor;
        self
    }

    pub fn with_offset(mut self, offset: Option<u64>) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_endpoint(mut self, endpoint: Option<SearchEndpoint>) -> Self {
        self.endpoint = endpoint;
        self
    }

    /// The endpoint this request will hit.
    pub fn effective_endpoint(&self) -> SearchEndpoint {
        if let Some(endpoint) = self.endpoint {
            return endpoint;
        }
        if !self.tags.is_empty() {
            // Only the full-text endpoint filters by tag.
            return SearchEndpoint::FullText;
        }
        match self.mode {
            PaginationMode::Cursor => SearchEndpoint::FullText,
            PaginationMode::Offset => SearchEndpoint::AdvancedV1,
        }
    }

    /// Reject requests that cannot be served faithfully, rather than sending
    /// them and silently losing a filter or a page boundary.
    pub fn validate(&self) -> Result<()> {
        if self.query.trim().is_empty() {
            return Err(Error::InvalidRequest(
                "a search query is required and cannot be empty".to_string(),
            ));
        }

        // Cursor and offset pagination can never be combined: the cursor already
        // encodes a position, and an offset on top of it skips rows.
        if let Some(offset) = self.offset {
            if self.mode == PaginationMode::Cursor || self.cursor.is_some() {
                return Err(PaginationError::MixedPagination { offset }.into());
            }
        }
        if self.cursor.is_some() && self.mode == PaginationMode::Offset {
            return Err(PaginationError::ModeMismatch {
                expected: PaginationMode::Offset,
                returned: PaginationMode::Cursor,
            }
            .into());
        }

        if !self.tags.is_empty() && self.effective_endpoint() == SearchEndpoint::AdvancedV1 {
            return Err(Error::InvalidRequest(
                "tag filters are only supported by GET /api/search; drop --endpoint or drop the tag filter".to_string(),
            ));
        }

        Ok(())
    }

    /// Where a walk of this request starts, as a continuation.
    pub fn start_continuation(&self) -> Result<Option<PageCursor>> {
        self.validate()?;
        Ok(match self.mode {
            PaginationMode::Cursor => self
                .cursor
                .as_ref()
                .filter(|token| !token.is_empty())
                .map(|token| PageCursor::Cursor(token.clone())),
            PaginationMode::Offset => self
                .offset
                .filter(|offset| *offset > 0)
                .map(|offset| PageCursor::Offset { offset }),
        })
    }
}

/// The wire shape of a search response, tolerant of both endpoints.
#[derive(Debug, Deserialize)]
pub(crate) struct RawSearchResponse {
    /// `GET /api/search` calls this `contracts`; the v1 endpoint calls it `results`.
    #[serde(default, alias = "results")]
    pub contracts: Vec<ContractHit>,
    #[serde(default)]
    pub total: Option<i64>,
    /// Present only for cursor-paginated responses, and only while more rows remain.
    #[serde(default)]
    pub next_cursor: Option<String>,
}

impl RawSearchResponse {
    /// Convert to a page, deriving the continuation for the active mode.
    ///
    /// `current_offset` is the offset this response was fetched at and
    /// `requested_limit` the page size that was asked for; both are needed to
    /// decide whether an offset walk has more rows.
    pub(crate) fn into_page(
        self,
        mode: PaginationMode,
        current_offset: u64,
        requested_limit: u32,
    ) -> Result<crate::pagination::RegistryPage<ContractHit>> {
        // A negative total is meaningless; treat it as "not reported" rather than
        // inventing a value.
        let total = self.total.and_then(|total| u64::try_from(total).ok());
        let items = self.contracts;
        let page_len = items.len() as u64;

        let next = match mode {
            PaginationMode::Cursor => self
                .next_cursor
                .filter(|token| !token.is_empty())
                .map(PageCursor::Cursor),
            PaginationMode::Offset => {
                if page_len == 0 {
                    None
                } else {
                    let next_offset = advance_offset(current_offset, page_len)?;
                    let has_more = match total {
                        Some(total) => next_offset < total,
                        // Without a total, a full page implies there may be more.
                        None => page_len >= u64::from(requested_limit),
                    };
                    has_more.then_some(PageCursor::Offset {
                        offset: next_offset,
                    })
                }
            }
        };

        Ok(crate::pagination::RegistryPage::new(items, next, total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_text_response_shape_parses() {
        let raw: RawSearchResponse = serde_json::from_str(
            r#"{
                "contracts": [{
                    "id": "11111111-1111-1111-1111-111111111111",
                    "contract_id": "CA...",
                    "name": "swap",
                    "description": null,
                    "category": "DeFi",
                    "network": "testnet",
                    "is_verified": true,
                    "deprecation_status": "active",
                    "is_deprecated": false,
                    "relevance_score": 0.75,
                    "matched_terms": null,
                    "highlighted": null
                }],
                "total": 42,
                "took_ms": 3,
                "next_cursor": "opaque-token"
            }"#,
        )
        .expect("full-text response should parse");

        assert_eq!(raw.contracts.len(), 1);
        assert_eq!(raw.contracts[0].name, "swap");
        assert_eq!(raw.total, Some(42));

        let page = raw
            .into_page(PaginationMode::Cursor, 0, 20)
            .expect("page conversion");
        assert_eq!(
            page.next,
            Some(PageCursor::Cursor("opaque-token".to_string()))
        );
        assert_eq!(page.total, Some(42));
    }

    #[test]
    fn advanced_v1_response_shape_parses() {
        let raw: RawSearchResponse = serde_json::from_str(
            r#"{
                "query": "swap",
                "total": 5,
                "limit": 2,
                "offset": 2,
                "took_ms": 7,
                "backend": "elasticsearch",
                "results": [
                    {"id": "a", "contract_id": "CA1", "name": "one", "network": "mainnet", "is_verified": false, "relevance_score": 1.0},
                    {"id": "b", "contract_id": "CA2", "name": "two", "network": "mainnet", "is_verified": false, "relevance_score": 0.5}
                ],
                "facets": {"categories": [], "networks": [], "tags": []}
            }"#,
        )
        .expect("advanced response should parse");

        let page = raw
            .into_page(PaginationMode::Offset, 2, 2)
            .expect("page conversion");
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.next, Some(PageCursor::Offset { offset: 4 }));
        assert_eq!(page.total, Some(5));
    }

    #[test]
    fn offset_page_at_the_total_has_no_continuation() {
        let raw = RawSearchResponse {
            contracts: vec![ContractHit {
                id: "a".into(),
                contract_id: "CA1".into(),
                name: "one".into(),
                description: None,
                category: None,
                network: "mainnet".into(),
                is_verified: false,
                is_deprecated: false,
                deprecation_status: None,
                replacement_contract_id: None,
                relevance_score: None,
            }],
            total: Some(3),
            next_cursor: None,
        };

        let page = raw
            .into_page(PaginationMode::Offset, 2, 2)
            .expect("page conversion");
        assert!(page.next.is_none(), "offset 2 + 1 item reaches total 3");
    }

    #[test]
    fn offset_conversion_detects_overflow() {
        let raw = RawSearchResponse {
            contracts: vec![ContractHit {
                id: "a".into(),
                contract_id: "CA1".into(),
                name: "one".into(),
                description: None,
                category: None,
                network: "mainnet".into(),
                is_verified: false,
                is_deprecated: false,
                deprecation_status: None,
                replacement_contract_id: None,
                relevance_score: None,
            }],
            total: None,
            next_cursor: None,
        };

        let err = raw
            .into_page(PaginationMode::Offset, u64::MAX, 1)
            .expect_err("advancing past u64::MAX must fail");
        assert!(matches!(
            err,
            Error::Pagination(PaginationError::OffsetOverflow { .. })
        ));
    }

    #[test]
    fn empty_next_cursor_ends_the_walk() {
        let raw = RawSearchResponse {
            contracts: Vec::new(),
            total: Some(0),
            next_cursor: Some(String::new()),
        };

        let page = raw
            .into_page(PaginationMode::Cursor, 0, 20)
            .expect("page conversion");
        assert!(page.next.is_none());
    }

    #[test]
    fn negative_total_is_treated_as_unreported() {
        let raw = RawSearchResponse {
            contracts: Vec::new(),
            total: Some(-1),
            next_cursor: None,
        };

        let page = raw
            .into_page(PaginationMode::Cursor, 0, 20)
            .expect("page conversion");
        assert!(page.total.is_none());
    }

    #[test]
    fn cursor_mode_rejects_an_offset_parameter() {
        let err = ContractSearchRequest::cursor("swap")
            .with_offset(Some(40))
            .validate()
            .expect_err("cursor + offset must be rejected");

        assert!(matches!(
            err,
            Error::Pagination(PaginationError::MixedPagination { offset: 40 })
        ));
        assert!(err.to_string().contains("cannot be combined"));
    }

    #[test]
    fn offset_mode_rejects_a_cursor_parameter() {
        let err = ContractSearchRequest::offset("swap")
            .with_cursor(Some("token".to_string()))
            .validate()
            .expect_err("offset + cursor must be rejected");

        assert!(matches!(
            err,
            Error::Pagination(PaginationError::ModeMismatch { .. })
        ));
    }

    #[test]
    fn empty_query_is_rejected_before_a_request_is_made() {
        assert!(ContractSearchRequest::cursor("   ").validate().is_err());
    }

    #[test]
    fn endpoint_defaults_follow_the_pagination_mode() {
        assert_eq!(
            ContractSearchRequest::cursor("swap").effective_endpoint(),
            SearchEndpoint::FullText
        );
        assert_eq!(
            ContractSearchRequest::offset("swap").effective_endpoint(),
            SearchEndpoint::AdvancedV1
        );
        // Tag filters are only served by the full-text endpoint.
        assert_eq!(
            ContractSearchRequest::offset("swap")
                .with_tags(["defi"])
                .effective_endpoint(),
            SearchEndpoint::FullText
        );
    }

    #[test]
    fn forcing_the_advanced_endpoint_with_tags_is_rejected() {
        let err = ContractSearchRequest::offset("swap")
            .with_tags(["defi"])
            .with_endpoint(Some(SearchEndpoint::AdvancedV1))
            .validate()
            .expect_err("tags are unsupported there");

        assert!(matches!(err, Error::InvalidRequest(_)));
    }

    #[test]
    fn start_continuation_reflects_the_mode() {
        assert_eq!(
            ContractSearchRequest::cursor("swap")
                .with_cursor(Some("token".to_string()))
                .start_continuation()
                .unwrap(),
            Some(PageCursor::Cursor("token".to_string()))
        );
        assert_eq!(
            ContractSearchRequest::offset("swap")
                .with_offset(Some(40))
                .start_continuation()
                .unwrap(),
            Some(PageCursor::Offset { offset: 40 })
        );
        assert_eq!(
            ContractSearchRequest::cursor("swap")
                .start_continuation()
                .unwrap(),
            None
        );
    }
}

#[cfg(test)]
mod wire_compat_tests {
    use super::*;

    /// The registry omits `next_cursor` when there is no next page, so the
    /// paginated envelope must decode without it.
    #[test]
    fn a_paginated_envelope_decodes_without_its_optional_fields() {
        let page: PaginatedResponse<serde_json::Value> =
            serde_json::from_str(r#"{"items":[],"total":0,"page":1,"per_page":20,"pages":0}"#)
                .expect("a response without optional fields must decode");
        assert!(page.next_cursor.is_none());
        assert!(page.filters.is_none());
    }
}
