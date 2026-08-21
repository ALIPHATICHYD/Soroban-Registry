//! Pagination decisions for `contract search`.
//!
//! Turning command-line flags into a `registry_client` request and a set of
//! safety bounds is pure logic, kept separate from the command's HTTP and
//! rendering code in [`crate::contract_search`] so it can be tested directly.

use anyhow::{anyhow, Result};
use registry_client::{ContractSearchRequest, PageLimits, PaginationMode};
use std::collections::HashSet;
use std::str::FromStr;

/// Item budget for `--all` when the caller does not pick one. `--all` is always
/// bounded: an unbounded walk over a registry that is still being written to
/// need never terminate.
pub const DEFAULT_ALL_MAX_ITEMS: u64 = 1_000;
/// Page budget for `--all` when the caller does not pick one.
pub const DEFAULT_ALL_MAX_PAGES: u64 = 100;

/// Networks the registry accepts as filter values.
const KNOWN_NETWORKS: [&str; 3] = ["mainnet", "testnet", "futurenet"];

/// Everything `contract search` accepts.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub query: String,
    pub networks: Option<String>,
    pub category: Option<String>,
    pub tags: Option<String>,
    pub verified_only: bool,
    /// Items per page.
    pub limit: u32,
    /// Starting offset — offset pagination only.
    pub offset: Option<u64>,
    /// Continuation token — cursor pagination only.
    pub cursor: Option<String>,
    /// `cursor` or `offset`; inferred when absent.
    pub pagination: Option<String>,
    pub all: bool,
    pub max_items: Option<u64>,
    pub max_pages: Option<u64>,
    pub json: bool,
}

impl SearchOptions {
    /// The `--max-items` bound a `--all` walk runs under.
    pub fn effective_max_items(&self) -> u64 {
        self.max_items.unwrap_or(DEFAULT_ALL_MAX_ITEMS)
    }

    /// The `--max-pages` bound a `--all` walk runs under.
    pub fn effective_max_pages(&self) -> u64 {
        self.max_pages.unwrap_or(DEFAULT_ALL_MAX_PAGES)
    }
}

/// Pick the pagination scheme.
///
/// An explicit `--pagination` always wins. Otherwise: a `--cursor` implies a
/// cursor walk, an `--offset` implies an offset walk, `--all` defaults to cursor
/// pagination (stable under concurrent writes, so a full walk cannot skip or
/// repeat rows), and a single page defaults to offset pagination (relevance
/// ordered, which is what a one-shot search should show).
pub fn resolve_mode(options: &SearchOptions) -> Result<PaginationMode> {
    if let Some(mode) = options.pagination.as_deref() {
        return PaginationMode::from_str(mode).map_err(|err| anyhow!("{err}"));
    }
    if options.cursor.is_some() {
        return Ok(PaginationMode::Cursor);
    }
    if options.offset.is_some() {
        return Ok(PaginationMode::Offset);
    }
    Ok(if options.all {
        PaginationMode::Cursor
    } else {
        PaginationMode::Offset
    })
}

/// Build the client request, rejecting anything the registry could only serve
/// by silently dropping a filter or a page boundary.
pub fn build_request(
    options: &SearchOptions,
    mode: PaginationMode,
) -> Result<ContractSearchRequest> {
    let request = ContractSearchRequest::new(options.query.clone(), mode)
        .with_networks(normalize_networks(options.networks.as_deref())?)
        .with_categories(split_filter(options.category.as_deref()))
        .with_tags(split_filter(options.tags.as_deref()))
        .with_verified_only(options.verified_only)
        .with_cursor(options.cursor.clone())
        .with_offset(options.offset);

    // Surfaces mixed cursor/offset pagination (and an empty query) before any
    // request is sent.
    request
        .validate()
        .map_err(|err| anyhow!("Failed to search contracts: {err}"))?;

    Ok(request)
}

/// Safety bounds for the walk. Without `--all` this is exactly one page, which
/// keeps single-page behaviour identical to a plain search.
pub fn build_limits(options: &SearchOptions) -> PageLimits {
    if !options.all {
        return PageLimits::single_page(options.limit);
    }

    PageLimits::default()
        .with_page_size(options.limit)
        .with_max_items(Some(options.effective_max_items()))
        .with_max_pages(Some(options.effective_max_pages()))
}

/// Canonicalise and de-duplicate network filters, rejecting unknown values here
/// rather than letting the server silently drop them.
pub fn normalize_networks(networks: Option<&str>) -> Result<Vec<String>> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for value in split_filter(networks) {
        let canonical = value.to_lowercase();
        if !KNOWN_NETWORKS.contains(&canonical.as_str()) {
            anyhow::bail!(
                "Invalid network: {value}. Allowed values: {}",
                KNOWN_NETWORKS.join(", ")
            );
        }
        if seen.insert(canonical.clone()) {
            normalized.push(canonical);
        }
    }

    Ok(normalized)
}

/// Split a comma-separated filter, trimming blanks.
pub fn split_filter(value: Option<&str>) -> Vec<String> {
    value
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> SearchOptions {
        SearchOptions {
            query: "swap".to_string(),
            networks: None,
            category: None,
            tags: None,
            verified_only: false,
            limit: 20,
            offset: None,
            cursor: None,
            pagination: None,
            all: false,
            max_items: None,
            max_pages: None,
            json: false,
        }
    }

    #[test]
    fn a_single_page_search_defaults_to_offset_pagination() {
        assert_eq!(resolve_mode(&options()).unwrap(), PaginationMode::Offset);
    }

    #[test]
    fn all_defaults_to_stable_cursor_pagination() {
        let mut opts = options();
        opts.all = true;
        assert_eq!(resolve_mode(&opts).unwrap(), PaginationMode::Cursor);
    }

    #[test]
    fn an_explicit_mode_wins() {
        let mut opts = options();
        opts.all = true;
        opts.pagination = Some("offset".to_string());
        assert_eq!(resolve_mode(&opts).unwrap(), PaginationMode::Offset);

        opts.pagination = Some("nonsense".to_string());
        assert!(resolve_mode(&opts).is_err());
    }

    #[test]
    fn a_cursor_or_offset_flag_selects_the_matching_mode() {
        let mut opts = options();
        opts.cursor = Some("token".to_string());
        assert_eq!(resolve_mode(&opts).unwrap(), PaginationMode::Cursor);

        let mut opts = options();
        opts.offset = Some(40);
        assert_eq!(resolve_mode(&opts).unwrap(), PaginationMode::Offset);
    }

    #[test]
    fn a_cursor_cannot_be_combined_with_an_offset() {
        let mut opts = options();
        opts.cursor = Some("token".to_string());
        opts.offset = Some(40);

        let err = build_request(&opts, PaginationMode::Cursor)
            .expect_err("mixed pagination must be rejected");
        assert!(err.to_string().contains("cannot be combined"), "{err}");
    }

    #[test]
    fn cursor_mode_rejects_an_offset_even_without_a_cursor() {
        let mut opts = options();
        opts.pagination = Some("cursor".to_string());
        opts.offset = Some(40);

        let mode = resolve_mode(&opts).unwrap();
        assert!(build_request(&opts, mode).is_err());
    }

    #[test]
    fn a_single_page_search_fetches_exactly_one_page() {
        let limits = build_limits(&options());
        assert_eq!(limits.max_pages, Some(1));
        assert_eq!(limits.max_items, None);
        assert_eq!(limits.page_size, 20);
    }

    #[test]
    fn all_is_always_bounded() {
        let mut opts = options();
        opts.all = true;
        let limits = build_limits(&opts);
        assert_eq!(limits.max_items, Some(DEFAULT_ALL_MAX_ITEMS));
        assert_eq!(limits.max_pages, Some(DEFAULT_ALL_MAX_PAGES));

        opts.max_items = Some(25);
        opts.max_pages = Some(3);
        let limits = build_limits(&opts);
        assert_eq!(limits.max_items, Some(25));
        assert_eq!(limits.max_pages, Some(3));
    }

    #[test]
    fn network_filters_are_canonicalised_and_deduplicated() {
        let networks = normalize_networks(Some(" testnet, MAINNET ,testnet")).unwrap();
        assert_eq!(networks, vec!["testnet", "mainnet"]);
    }

    #[test]
    fn an_unknown_network_filter_is_rejected_locally() {
        let err = normalize_networks(Some("testnet,not-a-network"))
            .expect_err("unknown networks must fail before the request");
        assert!(err.to_string().contains("not-a-network"));
    }

    #[test]
    fn filter_lists_are_split_and_trimmed() {
        assert_eq!(
            split_filter(Some("DeFi, NFT ,,lending")),
            vec!["DeFi", "NFT", "lending"]
        );
        assert!(split_filter(None).is_empty());
    }

    #[test]
    fn an_empty_query_is_rejected_before_any_request() {
        let mut opts = options();
        opts.query = "  ".to_string();
        assert!(build_request(&opts, PaginationMode::Offset).is_err());
    }

    #[test]
    fn tag_filters_route_to_the_full_text_endpoint() {
        let mut opts = options();
        opts.tags = Some("defi,amm".to_string());
        let request = build_request(&opts, PaginationMode::Offset).expect("valid request");
        assert_eq!(
            request.effective_endpoint(),
            registry_client::SearchEndpoint::FullText,
            "only /api/search filters by tag"
        );
        assert_eq!(request.tags, vec!["defi", "amm"]);
    }
}
