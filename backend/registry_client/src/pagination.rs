//! Backend-agnostic pagination.
//!
//! The registry exposes two pagination strategies (see `docs/pagination.md`):
//! stable keyset **cursor** pagination served by PostgreSQL, and **offset**
//! pagination served by Elasticsearch (and by the PostgreSQL fallback). Both are
//! wrapped here behind one type so consumers never manage continuation state,
//! and never mix parameters from the two schemes.
//!
//! The guarantees this module enforces, all of them bounded so a misbehaving
//! server cannot spin a consumer forever:
//!
//! * Cursors stay **opaque** — they are compared and echoed back, never decoded.
//! * Cursor pagination and offset parameters can never be combined.
//! * A repeated cursor, or an offset that fails to advance, is a hard error.
//! * `offset + page_len` is checked for overflow on every page.
//! * Empty pages that still carry a continuation token are bounded.
//! * A failed fetch leaves the walk untouched, so a retry re-requests the same
//!   page and can never duplicate or skip already-emitted items.
//! * Server-supplied ordering is preserved exactly; items are never re-sorted.

use std::collections::HashSet;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;

use futures_util::stream::{self, Stream, TryStreamExt};
use tokio::sync::watch;

use crate::error::{Error, PaginationError, Result};

/// Page size used when the caller does not pick one.
pub const DEFAULT_PAGE_SIZE: u32 = 50;
/// Largest page size the API honours (`limit` is clamped to `1..=100` server side).
pub const MAX_PAGE_SIZE: u32 = 100;
/// Safety bound on a full walk when the caller does not pick one.
pub const DEFAULT_MAX_PAGES: u64 = 200;
/// Consecutive empty-but-continuable pages tolerated before giving up.
pub const DEFAULT_MAX_EMPTY_PAGES: u32 = 3;

/// Which pagination scheme a walk uses. Chosen explicitly by the caller — the
/// client never guesses, because the two schemes hit different endpoints and
/// give different ordering guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaginationMode {
    /// Stable keyset pagination: `(created_at DESC, id DESC)`, no skips or
    /// duplicates under concurrent writes.
    Cursor,
    /// `limit`/`offset` pagination: relevance ordering, may drift under
    /// concurrent writes.
    Offset,
}

impl PaginationMode {
    pub fn as_str(self) -> &'static str {
        match self {
            PaginationMode::Cursor => "cursor",
            PaginationMode::Offset => "offset",
        }
    }
}

impl fmt::Display for PaginationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PaginationMode {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "cursor" | "keyset" => Ok(PaginationMode::Cursor),
            "offset" | "limit-offset" => Ok(PaginationMode::Offset),
            other => Err(Error::InvalidRequest(format!(
                "unknown pagination mode `{other}`; expected `cursor` or `offset`"
            ))),
        }
    }
}

/// Where the next page starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageCursor {
    /// An opaque server token. The client compares it for repetition and sends
    /// it back verbatim; it never parses, decodes or synthesises one.
    Cursor(String),
    /// A row offset into the result set.
    Offset { offset: u64 },
}

impl PageCursor {
    /// The pagination scheme this continuation belongs to.
    pub fn mode(&self) -> PaginationMode {
        match self {
            PageCursor::Cursor(_) => PaginationMode::Cursor,
            PageCursor::Offset { .. } => PaginationMode::Offset,
        }
    }

    /// The opaque token, for cursor continuations.
    pub fn as_cursor(&self) -> Option<&str> {
        match self {
            PageCursor::Cursor(token) => Some(token),
            PageCursor::Offset { .. } => None,
        }
    }

    /// The row offset, for offset continuations.
    pub fn as_offset(&self) -> Option<u64> {
        match self {
            PageCursor::Offset { offset } => Some(*offset),
            PageCursor::Cursor(_) => None,
        }
    }
}

impl fmt::Display for PageCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PageCursor::Cursor(token) => f.write_str(token),
            PageCursor::Offset { offset } => write!(f, "{offset}"),
        }
    }
}

/// Advance an offset by a page length, refusing to wrap.
pub fn advance_offset(offset: u64, page_len: u64) -> Result<u64> {
    offset
        .checked_add(page_len)
        .ok_or(PaginationError::OffsetOverflow { offset, page_len })
        .map_err(Error::from)
}

/// One page of results, in the order the server returned them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryPage<T> {
    /// Items in server order. Never re-sorted by this client, so whatever
    /// ordering guarantee the endpoint provides is the ordering you observe.
    pub items: Vec<T>,
    /// Continuation for the following page, or `None` on the last page.
    pub next: Option<PageCursor>,
    /// Total matches reported by the server, preserved verbatim when supplied.
    pub total: Option<u64>,
}

impl<T> RegistryPage<T> {
    pub fn new(items: Vec<T>, next: Option<PageCursor>, total: Option<u64>) -> Self {
        Self { items, next, total }
    }

    /// A terminal page carrying nothing.
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            next: None,
            total: None,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Convert the items while keeping the pagination metadata.
    pub fn map<U, F: FnMut(T) -> U>(self, f: F) -> RegistryPage<U> {
        RegistryPage {
            items: self.items.into_iter().map(f).collect(),
            next: self.next,
            total: self.total,
        }
    }
}

/// What the paginator asks a fetcher for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRequest {
    /// `None` for the first page of a walk.
    pub cursor: Option<PageCursor>,
    /// Maximum items wanted in this page.
    pub limit: u32,
    pub mode: PaginationMode,
    /// 0-based index of the page being fetched, for logging and diagnostics.
    pub page_index: u64,
}

pub type PageFuture<'a, T> = Pin<Box<dyn Future<Output = Result<RegistryPage<T>>> + Send + 'a>>;

/// Fetches one page. Implementors own the transport concerns — including
/// retries, which must happen *inside* a single `fetch_page` call so the
/// paginator never re-emits items it has already handed out.
pub trait PageFetcher: Send + Sync {
    type Item: Send;

    fn fetch_page(&self, request: PageRequest) -> PageFuture<'_, Self::Item>;
}

/// Safety bounds for a walk. The defaults are deliberately finite: an
/// unbounded "fetch everything" is never the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageLimits {
    /// Items requested per page (clamped to `1..=MAX_PAGE_SIZE`).
    pub page_size: u32,
    /// Hard cap on pages fetched. `None` means unbounded, which only the
    /// caller can ask for explicitly.
    pub max_pages: Option<u64>,
    /// Hard cap on items emitted. `None` means unbounded.
    pub max_items: Option<u64>,
    /// Consecutive empty-but-continuable pages tolerated (minimum 1).
    pub max_empty_pages: u32,
}

impl Default for PageLimits {
    fn default() -> Self {
        Self {
            page_size: DEFAULT_PAGE_SIZE,
            max_pages: Some(DEFAULT_MAX_PAGES),
            max_items: None,
            max_empty_pages: DEFAULT_MAX_EMPTY_PAGES,
        }
    }
}

impl PageLimits {
    /// Limits for fetching exactly one page of `page_size` items.
    pub fn single_page(page_size: u32) -> Self {
        Self {
            page_size,
            max_pages: Some(1),
            max_items: None,
            max_empty_pages: DEFAULT_MAX_EMPTY_PAGES,
        }
    }

    pub fn with_page_size(mut self, page_size: u32) -> Self {
        self.page_size = page_size;
        self
    }

    pub fn with_max_pages(mut self, max_pages: Option<u64>) -> Self {
        self.max_pages = max_pages;
        self
    }

    pub fn with_max_items(mut self, max_items: Option<u64>) -> Self {
        self.max_items = max_items;
        self
    }

    pub fn with_max_empty_pages(mut self, max_empty_pages: u32) -> Self {
        self.max_empty_pages = max_empty_pages;
        self
    }

    /// Page size to request, never asking for more than the remaining item
    /// budget so `--max-items` is respected without over-fetching.
    fn effective_page_size(&self, remaining_items: Option<u64>) -> u32 {
        let size = self.page_size.clamp(1, MAX_PAGE_SIZE);
        match remaining_items {
            Some(remaining) => size.min(remaining.clamp(1, MAX_PAGE_SIZE as u64) as u32),
            None => size,
        }
    }

    fn empty_page_budget(&self) -> u32 {
        self.max_empty_pages.max(1)
    }
}

/// Why a walk stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// The server reported no further pages.
    Exhausted,
    /// The `max_items` bound was reached.
    MaxItems,
    /// The `max_pages` bound was reached.
    MaxPages,
    /// The caller cancelled the walk.
    Cancelled,
}

impl StopReason {
    pub fn as_str(self) -> &'static str {
        match self {
            StopReason::Exhausted => "exhausted",
            StopReason::MaxItems => "max_items",
            StopReason::MaxPages => "max_pages",
            StopReason::Cancelled => "cancelled",
        }
    }

    /// True when the walk really did reach the end of the result set.
    pub fn is_complete(self) -> bool {
        matches!(self, StopReason::Exhausted)
    }
}

impl fmt::Display for StopReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Cooperative cancellation for a walk. Cloneable, so a signal handler can hold
/// one while the walk holds another.
#[derive(Debug, Clone)]
pub struct CancelToken {
    sender: Arc<watch::Sender<bool>>,
}

impl Default for CancelToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancelToken {
    pub fn new() -> Self {
        Self {
            sender: Arc::new(watch::channel(false).0),
        }
    }

    /// Request cancellation. Idempotent, and safe from any task.
    pub fn cancel(&self) {
        // `send_replace`, not `send`: the flag must be set even when nobody is
        // currently awaiting `cancelled()`.
        self.sender.send_replace(true);
    }

    pub fn is_cancelled(&self) -> bool {
        *self.sender.borrow()
    }

    /// Resolves once cancelled. Used to abandon an in-flight page fetch.
    pub async fn cancelled(&self) {
        let mut receiver = self.sender.subscribe();
        while !*receiver.borrow_and_update() {
            // The sender is held in this `Arc`, so `changed()` cannot fail while
            // `self` is alive; treat an error as "never cancelled" regardless.
            if receiver.changed().await.is_err() {
                std::future::pending::<()>().await;
            }
        }
    }
}

/// Everything a completed (or bounded) walk produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageCollection<T> {
    /// Items in server order, across all pages.
    pub items: Vec<T>,
    /// Server-reported total for the query, when the API supplied one.
    pub total: Option<u64>,
    pub pages_fetched: u64,
    pub stop_reason: StopReason,
    /// Continuation to resume from, when the walk stopped at a bound rather
    /// than at the end of the result set.
    pub next: Option<PageCursor>,
}

impl<T> PageCollection<T> {
    /// True when the whole result set was read.
    pub fn is_complete(&self) -> bool {
        self.stop_reason.is_complete()
    }
}

/// Walks pages for one query, in one pagination mode.
///
/// Construct it from a [`PageFetcher`] (the HTTP client builds one for you via
/// [`crate::RegistryClient::search_paginator`]) and drive it with
/// [`Paginator::next_page`], [`Paginator::collect_all`], or the
/// [`Paginator::pages`] / [`Paginator::items`] streams.
pub struct Paginator<F: PageFetcher> {
    fetcher: F,
    mode: PaginationMode,
    limits: PageLimits,
    cancel: CancelToken,
    /// Continuation for the next fetch; `None` before the first page.
    next: Option<PageCursor>,
    /// Current position in offset mode, tracked so a stalled or overflowing
    /// advance is detected against what the client actually walked.
    offset_position: u64,
    /// Every cursor already requested or offered, for repeat detection. Holds
    /// the tokens verbatim — they are compared, never decoded.
    seen_cursors: HashSet<String>,
    pages_fetched: u64,
    items_emitted: u64,
    total: Option<u64>,
    consecutive_empty_pages: u32,
    stop_reason: Option<StopReason>,
}

impl<F: PageFetcher> fmt::Debug for Paginator<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Paginator")
            .field("mode", &self.mode)
            .field("limits", &self.limits)
            .field("next", &self.next)
            .field("pages_fetched", &self.pages_fetched)
            .field("items_emitted", &self.items_emitted)
            .field("total", &self.total)
            .field("stop_reason", &self.stop_reason)
            .finish_non_exhaustive()
    }
}

impl<F: PageFetcher> Paginator<F> {
    /// A walk over `fetcher` in `mode`, with default safety bounds.
    pub fn new(fetcher: F, mode: PaginationMode) -> Self {
        Self {
            fetcher,
            mode,
            limits: PageLimits::default(),
            cancel: CancelToken::new(),
            next: None,
            offset_position: 0,
            seen_cursors: HashSet::new(),
            pages_fetched: 0,
            items_emitted: 0,
            total: None,
            consecutive_empty_pages: 0,
            stop_reason: None,
        }
    }

    pub fn with_limits(mut self, limits: PageLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn with_cancel_token(mut self, cancel: CancelToken) -> Self {
        self.cancel = cancel;
        self
    }

    /// Resume from a continuation obtained earlier.
    ///
    /// Errors when the continuation belongs to the other pagination scheme:
    /// resuming a cursor walk from an offset would silently skip rows, so it is
    /// rejected rather than reinterpreted.
    pub fn start_at(mut self, start: Option<PageCursor>) -> Result<Self> {
        match (&start, self.mode) {
            (Some(PageCursor::Offset { offset }), PaginationMode::Cursor) => {
                return Err(PaginationError::MixedPagination { offset: *offset }.into());
            }
            (Some(PageCursor::Cursor(_)), PaginationMode::Offset) => {
                return Err(PaginationError::ModeMismatch {
                    expected: PaginationMode::Offset,
                    returned: PaginationMode::Cursor,
                }
                .into());
            }
            _ => {}
        }

        if let Some(PageCursor::Cursor(token)) = &start {
            // The starting token counts as seen: a server that hands it straight
            // back is looping, and that must be an error, not a re-read.
            if !token.is_empty() {
                self.seen_cursors.insert(token.clone());
            }
        }
        if let Some(PageCursor::Offset { offset }) = &start {
            self.offset_position = *offset;
        }

        self.next = start;
        Ok(self)
    }

    pub fn mode(&self) -> PaginationMode {
        self.mode
    }

    pub fn limits(&self) -> PageLimits {
        self.limits
    }

    pub fn cancel_token(&self) -> CancelToken {
        self.cancel.clone()
    }

    /// Latest server-reported total, preserved across the walk.
    pub fn total(&self) -> Option<u64> {
        self.total
    }

    pub fn pages_fetched(&self) -> u64 {
        self.pages_fetched
    }

    pub fn items_emitted(&self) -> u64 {
        self.items_emitted
    }

    /// Continuation for the page that has not been fetched yet, if any. Use it
    /// to resume a walk that stopped at a bound.
    pub fn next_continuation(&self) -> Option<&PageCursor> {
        self.next.as_ref()
    }

    /// Why the walk finished, once it has.
    pub fn stop_reason(&self) -> Option<StopReason> {
        self.stop_reason
    }

    /// Fetch the next page, or `Ok(None)` once the walk is over — because the
    /// result set ran out, a bound was hit, or the walk was cancelled. Check
    /// [`Paginator::stop_reason`] to tell those apart.
    ///
    /// On error the walk state is left exactly as it was, so calling again
    /// re-requests the *same* page: a retry can neither duplicate nor skip
    /// items that were already emitted.
    pub async fn next_page(&mut self) -> Result<Option<RegistryPage<F::Item>>> {
        if self.stop_reason.is_some() {
            return Ok(None);
        }
        if self.cancel.is_cancelled() {
            return Ok(self.stop(StopReason::Cancelled));
        }
        if self
            .limits
            .max_pages
            .is_some_and(|max| self.pages_fetched >= max)
        {
            return Ok(self.stop(StopReason::MaxPages));
        }

        let remaining_items = match self.limits.max_items {
            Some(max) => {
                let remaining = max.saturating_sub(self.items_emitted);
                if remaining == 0 {
                    return Ok(self.stop(StopReason::MaxItems));
                }
                Some(remaining)
            }
            None => None,
        };

        let request = PageRequest {
            cursor: self.next.clone(),
            limit: self.limits.effective_page_size(remaining_items),
            mode: self.mode,
            page_index: self.pages_fetched,
        };
        // Fetch, abandoning the in-flight request if cancellation arrives. The
        // cancel token is cloned so the select does not borrow `self` twice.
        let cancel = self.cancel.clone();
        let fetched = {
            let fetch = self.fetcher.fetch_page(request);
            tokio::select! {
                biased;
                _ = cancel.cancelled() => None,
                result = fetch => Some(result),
            }
        };
        let Some(result) = fetched else {
            return Ok(self.stop(StopReason::Cancelled));
        };
        // `?` here leaves every field untouched — that is what makes a caller's
        // retry safe.
        let mut page = result?;

        // ── Validate the continuation before committing anything ──────────────
        let page_len = page.items.len() as u64;
        let next = self.validate_continuation(&page, page_len)?;

        if page.items.is_empty() {
            let empties = self.consecutive_empty_pages.saturating_add(1);
            if next.is_some() && empties >= self.limits.empty_page_budget() {
                return Err(PaginationError::EmptyPageLoop { pages: empties }.into());
            }
        }

        // A well-behaved server already honoured `limit`, but clamp regardless so
        // `max_items` is an upper bound on what a consumer ever sees.
        let mut truncated = false;
        if let Some(remaining) = remaining_items {
            if page_len > remaining {
                page.items.truncate(remaining as usize);
                truncated = true;
            }
        }

        // ── Commit ────────────────────────────────────────────────────────────
        self.pages_fetched += 1;
        self.items_emitted += page.items.len() as u64;
        if let Some(total) = page.total {
            self.total = Some(total);
        }
        if page.items.is_empty() {
            self.consecutive_empty_pages = self.consecutive_empty_pages.saturating_add(1);
        } else {
            self.consecutive_empty_pages = 0;
        }
        if let Some(PageCursor::Cursor(token)) = &next {
            self.seen_cursors.insert(token.clone());
        }
        if let Some(PageCursor::Offset { offset }) = &next {
            self.offset_position = *offset;
        }

        if truncated {
            // Resuming from `next` would skip the rows just dropped, so the walk
            // ends here without a resume point.
            self.next = None;
            self.stop_reason = Some(StopReason::MaxItems);
        } else {
            self.next = next;
            if self.next.is_none() {
                self.stop_reason = Some(StopReason::Exhausted);
            } else if self
                .limits
                .max_items
                .is_some_and(|max| self.items_emitted >= max)
            {
                // Bound reached exactly on a page boundary: keep `next` so the
                // caller can resume, and stop without another request.
                self.stop_reason = Some(StopReason::MaxItems);
            }
            // A short page that still carries a continuation is followed, not
            // second-guessed: dropping the server's token here would silently
            // lose results.
        }

        Ok(Some(page))
    }

    /// Validate the server's continuation against the walk so far, returning the
    /// continuation to use next (`None` ends the walk).
    fn validate_continuation(
        &self,
        page: &RegistryPage<F::Item>,
        page_len: u64,
    ) -> Result<Option<PageCursor>> {
        let page_number = self.pages_fetched + 1;

        let Some(next) = page.next.clone() else {
            return Ok(None);
        };
        if next.mode() != self.mode {
            return Err(PaginationError::ModeMismatch {
                expected: self.mode,
                returned: next.mode(),
            }
            .into());
        }

        match next {
            PageCursor::Cursor(token) => {
                if token.is_empty() {
                    // Carries no position; treat as the end of the walk rather
                    // than restarting from page one.
                    return Ok(None);
                }
                if self.seen_cursors.contains(&token) {
                    return Err(PaginationError::RepeatedCursor { page: page_number }.into());
                }
                Ok(Some(PageCursor::Cursor(token)))
            }
            PageCursor::Offset { offset } => {
                // Always validate the walk's own advance, so a position that runs
                // off the end of `u64` fails loudly instead of wrapping.
                advance_offset(self.offset_position, page_len)?;
                if offset <= self.offset_position {
                    return Err(PaginationError::StalledOffset {
                        offset: self.offset_position,
                        page: page_number,
                    }
                    .into());
                }
                Ok(Some(PageCursor::Offset { offset }))
            }
        }
    }

    fn stop(&mut self, reason: StopReason) -> Option<RegistryPage<F::Item>> {
        self.stop_reason = Some(reason);
        None
    }

    /// Walk every remaining page, subject to the configured bounds.
    pub async fn collect_all(&mut self) -> Result<PageCollection<F::Item>> {
        let mut items = Vec::new();
        while let Some(page) = self.next_page().await? {
            items.extend(page.items);
        }

        Ok(PageCollection {
            items,
            total: self.total,
            pages_fetched: self.pages_fetched,
            stop_reason: self.stop_reason.unwrap_or(StopReason::Exhausted),
            next: self.next.clone(),
        })
    }

    /// The walk as a stream of pages.
    pub fn pages(self) -> impl Stream<Item = Result<RegistryPage<F::Item>>> {
        stream::try_unfold(self, |mut paginator| async move {
            match paginator.next_page().await? {
                Some(page) => Ok(Some((page, paginator))),
                None => Ok(None),
            }
        })
    }

    /// The walk as a flat stream of items, in server order.
    pub fn items(self) -> impl Stream<Item = Result<F::Item>> {
        self.pages()
            .map_ok(|page| stream::iter(page.items.into_iter().map(Ok)))
            .try_flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use std::sync::Mutex;

    /// A scripted fetcher: each entry is the page returned for the matching
    /// request index. Records the continuation it was handed so tests can assert
    /// what the paginator actually sent.
    struct ScriptedFetcher {
        pages: Mutex<Vec<Result<RegistryPage<u32>>>>,
        seen_requests: Mutex<Vec<PageRequest>>,
    }

    impl ScriptedFetcher {
        fn new(pages: Vec<Result<RegistryPage<u32>>>) -> Self {
            Self {
                pages: Mutex::new(pages),
                seen_requests: Mutex::new(Vec::new()),
            }
        }

        fn ok(pages: Vec<RegistryPage<u32>>) -> Self {
            Self::new(pages.into_iter().map(Ok).collect())
        }

        fn requests(&self) -> Vec<PageRequest> {
            self.seen_requests.lock().unwrap().clone()
        }
    }

    impl PageFetcher for ScriptedFetcher {
        type Item = u32;

        fn fetch_page(&self, request: PageRequest) -> PageFuture<'_, u32> {
            self.seen_requests.lock().unwrap().push(request);
            let mut pages = self.pages.lock().unwrap();
            let page = if pages.is_empty() {
                Ok(RegistryPage::empty())
            } else {
                pages.remove(0)
            };
            Box::pin(async move { page })
        }
    }

    fn cursor_page(items: &[u32], next: Option<&str>, total: Option<u64>) -> RegistryPage<u32> {
        RegistryPage::new(
            items.to_vec(),
            next.map(|token| PageCursor::Cursor(token.to_string())),
            total,
        )
    }

    fn offset_page(items: &[u32], next: Option<u64>, total: Option<u64>) -> RegistryPage<u32> {
        RegistryPage::new(
            items.to_vec(),
            next.map(|offset| PageCursor::Offset { offset }),
            total,
        )
    }

    fn paginator(fetcher: ScriptedFetcher, mode: PaginationMode) -> Paginator<ScriptedFetcher> {
        Paginator::new(fetcher, mode).with_limits(PageLimits::default().with_page_size(2))
    }

    #[tokio::test]
    async fn cursor_walk_collects_every_page_in_order() {
        let fetcher = ScriptedFetcher::ok(vec![
            cursor_page(&[1, 2], Some("c1"), Some(5)),
            cursor_page(&[3, 4], Some("c2"), Some(5)),
            cursor_page(&[5], None, Some(5)),
        ]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor);

        let collected = walk.collect_all().await.expect("walk should succeed");

        assert_eq!(collected.items, vec![1, 2, 3, 4, 5]);
        assert_eq!(collected.total, Some(5));
        assert_eq!(collected.pages_fetched, 3);
        assert_eq!(collected.stop_reason, StopReason::Exhausted);
        assert!(collected.is_complete());
        assert!(collected.next.is_none());
    }

    #[tokio::test]
    async fn cursor_tokens_are_echoed_back_verbatim_and_never_decoded() {
        // Deliberately not valid base64 or JSON: the client must treat the token
        // as an opaque string.
        let opaque = "!!not-base64!!";
        let fetcher = ScriptedFetcher::ok(vec![
            cursor_page(&[1, 2], Some(opaque), None),
            cursor_page(&[3], None, None),
        ]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor);

        walk.next_page().await.unwrap();
        walk.next_page().await.unwrap();

        let requests = walk.fetcher.requests();
        assert_eq!(requests[0].cursor, None);
        assert_eq!(
            requests[1].cursor,
            Some(PageCursor::Cursor(opaque.to_string()))
        );
    }

    #[tokio::test]
    async fn repeated_cursor_terminates_with_an_error() {
        let fetcher = ScriptedFetcher::ok(vec![
            cursor_page(&[1, 2], Some("same"), None),
            cursor_page(&[3, 4], Some("same"), None),
        ]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor);

        walk.next_page().await.expect("first page");
        let err = walk.next_page().await.expect_err("repeat must be rejected");

        assert!(matches!(
            err,
            Error::Pagination(PaginationError::RepeatedCursor { page: 2 })
        ));
        assert!(err.to_string().contains("repeated a pagination cursor"));
    }

    #[tokio::test]
    async fn cursor_echoed_from_the_starting_position_is_a_repeat() {
        let fetcher = ScriptedFetcher::ok(vec![cursor_page(&[1, 2], Some("resume"), None)]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor)
            .start_at(Some(PageCursor::Cursor("resume".to_string())))
            .expect("cursor start is valid for a cursor walk");

        let err = walk.next_page().await.expect_err("echoed cursor is a loop");

        assert!(matches!(
            err,
            Error::Pagination(PaginationError::RepeatedCursor { page: 1 })
        ));
    }

    #[tokio::test]
    async fn empty_continuable_pages_are_bounded() {
        let fetcher = ScriptedFetcher::ok(vec![
            cursor_page(&[], Some("c1"), Some(9)),
            cursor_page(&[], Some("c2"), Some(9)),
            cursor_page(&[], Some("c3"), Some(9)),
            cursor_page(&[], Some("c4"), Some(9)),
        ]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor).with_limits(
            PageLimits::default()
                .with_page_size(2)
                .with_max_empty_pages(3),
        );

        assert!(walk.next_page().await.unwrap().is_some());
        assert!(walk.next_page().await.unwrap().is_some());
        let err = walk
            .next_page()
            .await
            .expect_err("a third empty page must stop the walk");

        assert!(matches!(
            err,
            Error::Pagination(PaginationError::EmptyPageLoop { pages: 3 })
        ));
    }

    #[tokio::test]
    async fn empty_final_page_without_continuation_ends_cleanly() {
        let fetcher = ScriptedFetcher::ok(vec![
            cursor_page(&[1, 2], Some("c1"), Some(2)),
            cursor_page(&[], None, Some(2)),
        ]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor);

        let collected = walk.collect_all().await.expect("walk should succeed");

        assert_eq!(collected.items, vec![1, 2]);
        assert_eq!(collected.stop_reason, StopReason::Exhausted);
    }

    #[tokio::test]
    async fn offset_walk_advances_and_stops_at_the_total() {
        let fetcher = ScriptedFetcher::ok(vec![
            offset_page(&[1, 2], Some(2), Some(5)),
            offset_page(&[3, 4], Some(4), Some(5)),
            offset_page(&[5], None, Some(5)),
        ]);
        let mut walk = paginator(fetcher, PaginationMode::Offset);

        let collected = walk.collect_all().await.expect("walk should succeed");

        assert_eq!(collected.items, vec![1, 2, 3, 4, 5]);
        assert_eq!(collected.total, Some(5));
        let offsets: Vec<Option<u64>> = walk
            .fetcher
            .requests()
            .iter()
            .map(|request| request.cursor.as_ref().and_then(PageCursor::as_offset))
            .collect();
        assert_eq!(offsets, vec![None, Some(2), Some(4)]);
    }

    #[tokio::test]
    async fn unchanged_offset_terminates_with_an_error() {
        let fetcher = ScriptedFetcher::ok(vec![
            offset_page(&[1, 2], Some(2), Some(9)),
            // Server hands back the offset we are already at.
            offset_page(&[3, 4], Some(2), Some(9)),
        ]);
        let mut walk = paginator(fetcher, PaginationMode::Offset);

        walk.next_page().await.expect("first page");
        let err = walk.next_page().await.expect_err("stalled offset");

        assert!(matches!(
            err,
            Error::Pagination(PaginationError::StalledOffset { offset: 2, page: 2 })
        ));
    }

    #[tokio::test]
    async fn offset_overflow_is_detected() {
        let fetcher = ScriptedFetcher::ok(vec![offset_page(&[1, 2], Some(u64::MAX), None)]);
        let mut walk = paginator(fetcher, PaginationMode::Offset)
            .start_at(Some(PageCursor::Offset {
                offset: u64::MAX - 1,
            }))
            .expect("offset start is valid for an offset walk");

        let err = walk.next_page().await.expect_err("overflowing advance");

        assert!(matches!(
            err,
            Error::Pagination(PaginationError::OffsetOverflow {
                offset,
                page_len: 2
            }) if offset == u64::MAX - 1
        ));
    }

    #[test]
    fn advance_offset_rejects_wrapping() {
        assert_eq!(advance_offset(10, 5).unwrap(), 15);
        assert!(advance_offset(u64::MAX, 1).is_err());
    }

    #[tokio::test]
    async fn cursor_walk_rejects_an_offset_continuation() {
        let fetcher = ScriptedFetcher::ok(vec![offset_page(&[1, 2], Some(2), None)]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor);

        let err = walk.next_page().await.expect_err("mode mismatch");

        assert!(matches!(
            err,
            Error::Pagination(PaginationError::ModeMismatch {
                expected: PaginationMode::Cursor,
                returned: PaginationMode::Offset
            })
        ));
    }

    #[test]
    fn cursor_mode_rejects_an_offset_start() {
        let fetcher = ScriptedFetcher::ok(vec![]);
        let err = Paginator::new(fetcher, PaginationMode::Cursor)
            .start_at(Some(PageCursor::Offset { offset: 40 }))
            .expect_err("cursor + offset must be rejected");

        assert!(matches!(
            err,
            Error::Pagination(PaginationError::MixedPagination { offset: 40 })
        ));
    }

    #[test]
    fn offset_mode_rejects_a_cursor_start() {
        let fetcher = ScriptedFetcher::ok(vec![]);
        let err = Paginator::new(fetcher, PaginationMode::Offset)
            .start_at(Some(PageCursor::Cursor("abc".into())))
            .expect_err("offset walk cannot resume from a cursor");

        assert!(matches!(
            err,
            Error::Pagination(PaginationError::ModeMismatch { .. })
        ));
    }

    #[tokio::test]
    async fn max_items_on_a_page_boundary_keeps_a_resume_point() {
        let fetcher = ScriptedFetcher::ok(vec![
            cursor_page(&[1, 2], Some("c1"), Some(100)),
            cursor_page(&[3, 4], Some("c2"), Some(100)),
            cursor_page(&[5, 6], Some("c3"), Some(100)),
        ]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor).with_limits(
            PageLimits::default()
                .with_page_size(2)
                .with_max_items(Some(4)),
        );

        let collected = walk.collect_all().await.expect("walk should succeed");

        assert_eq!(collected.items, vec![1, 2, 3, 4]);
        assert_eq!(collected.stop_reason, StopReason::MaxItems);
        assert!(!collected.is_complete());
        assert_eq!(collected.total, Some(100));
        // Third page was never requested.
        assert_eq!(walk.fetcher.requests().len(), 2);
        assert_eq!(
            collected.next,
            Some(PageCursor::Cursor("c2".to_string())),
            "stopping at a bound leaves a usable resume point"
        );
    }

    #[tokio::test]
    async fn max_items_trims_the_last_requested_page_size() {
        let fetcher = ScriptedFetcher::ok(vec![
            cursor_page(&[1, 2], Some("c1"), Some(100)),
            cursor_page(&[3], Some("c2"), Some(100)),
        ]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor).with_limits(
            PageLimits::default()
                .with_page_size(2)
                .with_max_items(Some(3)),
        );

        let collected = walk.collect_all().await.expect("walk should succeed");

        assert_eq!(collected.items, vec![1, 2, 3]);
        assert_eq!(collected.stop_reason, StopReason::MaxItems);
        // The final request asks only for the remaining item budget.
        assert_eq!(walk.fetcher.requests()[1].limit, 1);
    }

    #[tokio::test]
    async fn max_items_truncates_an_oversized_page() {
        // Server ignores `limit` and returns more than the remaining budget.
        let fetcher = ScriptedFetcher::ok(vec![cursor_page(&[1, 2, 3, 4, 5], Some("c1"), None)]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor).with_limits(
            PageLimits::default()
                .with_page_size(2)
                .with_max_items(Some(2)),
        );

        let collected = walk.collect_all().await.expect("walk should succeed");

        assert_eq!(collected.items, vec![1, 2]);
        assert_eq!(collected.stop_reason, StopReason::MaxItems);
    }

    #[tokio::test]
    async fn max_pages_bounds_the_walk() {
        let fetcher = ScriptedFetcher::ok(vec![
            cursor_page(&[1, 2], Some("c1"), Some(100)),
            cursor_page(&[3, 4], Some("c2"), Some(100)),
            cursor_page(&[5, 6], Some("c3"), Some(100)),
        ]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor).with_limits(
            PageLimits::default()
                .with_page_size(2)
                .with_max_pages(Some(2)),
        );

        let collected = walk.collect_all().await.expect("walk should succeed");

        assert_eq!(collected.items, vec![1, 2, 3, 4]);
        assert_eq!(collected.pages_fetched, 2);
        assert_eq!(collected.stop_reason, StopReason::MaxPages);
        assert_eq!(
            collected.next,
            Some(PageCursor::Cursor("c2".to_string())),
            "a bounded walk exposes where to resume"
        );
    }

    #[tokio::test]
    async fn single_page_limits_fetch_exactly_one_page() {
        let fetcher = ScriptedFetcher::ok(vec![
            cursor_page(&[1, 2], Some("c1"), Some(50)),
            cursor_page(&[3, 4], Some("c2"), Some(50)),
        ]);
        let mut walk =
            Paginator::new(fetcher, PaginationMode::Cursor).with_limits(PageLimits::single_page(2));

        let collected = walk.collect_all().await.expect("walk should succeed");

        assert_eq!(collected.items, vec![1, 2]);
        assert_eq!(collected.pages_fetched, 1);
        assert_eq!(collected.total, Some(50));
    }

    #[tokio::test]
    async fn cancellation_before_the_walk_yields_nothing() {
        let fetcher = ScriptedFetcher::ok(vec![cursor_page(&[1, 2], Some("c1"), None)]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor);
        walk.cancel_token().cancel();

        let collected = walk
            .collect_all()
            .await
            .expect("cancellation is not an error");

        assert!(collected.items.is_empty());
        assert_eq!(collected.stop_reason, StopReason::Cancelled);
        assert!(walk.fetcher.requests().is_empty(), "nothing was requested");
    }

    #[tokio::test]
    async fn cancellation_mid_walk_keeps_the_pages_already_emitted() {
        let fetcher = ScriptedFetcher::ok(vec![
            cursor_page(&[1, 2], Some("c1"), Some(100)),
            cursor_page(&[3, 4], Some("c2"), Some(100)),
        ]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor);
        let cancel = walk.cancel_token();

        let first = walk.next_page().await.unwrap().expect("first page");
        assert_eq!(first.items, vec![1, 2]);

        cancel.cancel();
        assert!(walk.next_page().await.unwrap().is_none());
        assert_eq!(walk.stop_reason(), Some(StopReason::Cancelled));
        assert_eq!(walk.items_emitted(), 2);
        assert_eq!(
            walk.next_continuation(),
            Some(&PageCursor::Cursor("c1".to_string())),
            "a cancelled walk can be resumed"
        );
    }

    #[tokio::test]
    async fn cancellation_interrupts_an_in_flight_fetch() {
        struct HangingFetcher;

        impl PageFetcher for HangingFetcher {
            type Item = u32;

            fn fetch_page(&self, _request: PageRequest) -> PageFuture<'_, u32> {
                Box::pin(async {
                    std::future::pending::<()>().await;
                    unreachable!("the fetch never completes")
                })
            }
        }

        let mut walk = Paginator::new(HangingFetcher, PaginationMode::Cursor);
        let cancel = walk.cancel_token();
        tokio::spawn(async move {
            tokio::task::yield_now().await;
            cancel.cancel();
        });

        let page = walk
            .next_page()
            .await
            .expect("cancellation is not an error");

        assert!(page.is_none());
        assert_eq!(walk.stop_reason(), Some(StopReason::Cancelled));
    }

    #[tokio::test]
    async fn a_failed_fetch_does_not_advance_the_walk_or_duplicate_items() {
        let fetcher = ScriptedFetcher::new(vec![
            Ok(cursor_page(&[1, 2], Some("c1"), Some(4))),
            Err(Error::Transport {
                url: "http://registry.test/api/search".to_string(),
                attempts: 3,
                kind: crate::error::TransportKind::Connect,
                reason: "could not connect to the registry".to_string(),
            }),
            Ok(cursor_page(&[3, 4], None, Some(4))),
        ]);
        let mut walk = paginator(fetcher, PaginationMode::Cursor);

        let mut items = Vec::new();
        items.extend(walk.next_page().await.unwrap().unwrap().items);
        assert!(walk.next_page().await.is_err(), "second fetch fails");
        // Retrying re-requests the same cursor rather than skipping ahead.
        items.extend(walk.next_page().await.unwrap().unwrap().items);
        assert!(walk.next_page().await.unwrap().is_none());

        assert_eq!(items, vec![1, 2, 3, 4], "no duplicates, no skips");
        let cursors: Vec<Option<PageCursor>> = walk
            .fetcher
            .requests()
            .iter()
            .map(|request| request.cursor.clone())
            .collect();
        assert_eq!(
            cursors,
            vec![
                None,
                Some(PageCursor::Cursor("c1".to_string())),
                Some(PageCursor::Cursor("c1".to_string())),
            ]
        );
    }

    #[tokio::test]
    async fn items_stream_yields_every_item_in_order() {
        let fetcher = ScriptedFetcher::ok(vec![
            cursor_page(&[1, 2], Some("c1"), Some(5)),
            cursor_page(&[3, 4], Some("c2"), Some(5)),
            cursor_page(&[5], None, Some(5)),
        ]);
        let walk = paginator(fetcher, PaginationMode::Cursor);

        let items: Vec<u32> = walk
            .items()
            .map(|item| item.expect("stream item"))
            .collect::<Vec<_>>()
            .await;

        assert_eq!(items, vec![1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    async fn pages_stream_surfaces_pagination_errors() {
        let fetcher = ScriptedFetcher::ok(vec![
            cursor_page(&[1], Some("same"), None),
            cursor_page(&[2], Some("same"), None),
        ]);
        let walk = paginator(fetcher, PaginationMode::Cursor);

        let results: Vec<Result<RegistryPage<u32>>> = walk.pages().collect::<Vec<_>>().await;

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(matches!(
            results[1],
            Err(Error::Pagination(PaginationError::RepeatedCursor { .. }))
        ));
    }

    #[test]
    fn pagination_mode_parses_the_documented_spellings() {
        assert_eq!(
            "cursor".parse::<PaginationMode>().unwrap(),
            PaginationMode::Cursor
        );
        assert_eq!(
            " OFFSET ".parse::<PaginationMode>().unwrap(),
            PaginationMode::Offset
        );
        assert!("pages".parse::<PaginationMode>().is_err());
    }

    #[test]
    fn effective_page_size_respects_the_remaining_budget_and_server_cap() {
        let limits = PageLimits::default().with_page_size(50);
        assert_eq!(limits.effective_page_size(None), 50);
        assert_eq!(limits.effective_page_size(Some(7)), 7);
        assert_eq!(limits.effective_page_size(Some(1_000)), 50);
        assert_eq!(
            PageLimits::default()
                .with_page_size(5_000)
                .effective_page_size(None),
            MAX_PAGE_SIZE
        );
    }
}
