//! Error types for the registry client.

use crate::pagination::PaginationMode;

pub type Result<T> = std::result::Result<T, Error>;

/// Anything that can go wrong while talking to the registry.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request never produced a response (DNS, connect, timeout, …), after
    /// any internal retries were exhausted.
    #[error("request to {url} failed after {attempts} attempt(s): {reason}. Check the API URL and your network connection.")]
    Transport {
        url: String,
        attempts: usize,
        reason: String,
    },

    /// The registry answered with a non-success status.
    #[error("{method} {url} returned {status}{}", detail_suffix(code, message))]
    Api {
        method: String,
        url: String,
        status: u16,
        /// Machine-readable error code when the API supplied one
        /// (e.g. `INVALID_CURSOR`, `InvalidPaginationCursor`).
        code: Option<String>,
        message: Option<String>,
    },

    /// The response body was not the shape this client expects.
    #[error("could not decode the response from {url}: {reason}")]
    Decode { url: String, reason: String },

    /// The client was asked to build an impossible request.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// A pagination walk could not continue safely.
    #[error(transparent)]
    Pagination(#[from] PaginationError),
}

impl Error {
    /// The API error code, when this is an API error that carried one.
    pub fn api_code(&self) -> Option<&str> {
        match self {
            Error::Api { code, .. } => code.as_deref(),
            _ => None,
        }
    }

    /// The HTTP status, when this error came from a response.
    pub fn status(&self) -> Option<u16> {
        match self {
            Error::Api { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// True when the registry rejected the continuation token we sent. Callers
    /// walking a saved cursor should restart the walk rather than retry.
    pub fn is_invalid_cursor(&self) -> bool {
        matches!(self.status(), Some(400))
            && self.api_code().is_some_and(|code| {
                code.eq_ignore_ascii_case("INVALID_CURSOR") || code == "InvalidPaginationCursor"
            })
    }
}

fn detail_suffix(code: &Option<String>, message: &Option<String>) -> String {
    match (code.as_deref(), message.as_deref()) {
        (Some(code), Some(message)) => format!(" ({code}: {message})"),
        (Some(code), None) => format!(" ({code})"),
        (None, Some(message)) => format!(" ({message})"),
        (None, None) => String::new(),
    }
}

/// Reasons a pagination walk stops with an error instead of simply ending.
///
/// Every variant here is a *bounded failure*: the client refuses to keep
/// requesting pages when the continuation state it was handed cannot make
/// progress, so a misbehaving server can never spin a consumer forever.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PaginationError {
    /// Cursor pagination and offset parameters are mutually exclusive: the
    /// cursor already encodes the position, and an offset applied on top of it
    /// silently skips rows.
    #[error("cursor pagination cannot be combined with an offset (offset={offset}); drop the offset or switch to offset pagination")]
    MixedPagination { offset: u64 },

    /// A continuation token of the wrong kind for the active walk.
    #[error("{expected} pagination cannot continue from a{} {returned} continuation", article(*returned))]
    ModeMismatch {
        expected: PaginationMode,
        returned: PaginationMode,
    },

    /// The server handed back a cursor it had already handed back. Following it
    /// would re-fetch a page we have already emitted, forever.
    #[error("the registry repeated a pagination cursor after page {page}; stopping instead of looping over the same page")]
    RepeatedCursor { page: u64 },

    /// The offset did not move forward, so the next request would return the
    /// same rows.
    #[error("offset pagination did not advance past {offset} after page {page}; stopping instead of re-reading the same rows")]
    StalledOffset { offset: u64, page: u64 },

    /// `offset + page_len` does not fit in a `u64`.
    #[error("offset pagination overflowed: {offset} + {page_len} exceeds the largest representable offset")]
    OffsetOverflow { offset: u64, page_len: u64 },

    /// Repeated empty pages that still carry a continuation token. A server
    /// doing this cannot be distinguished from one looping, so stop.
    #[error("the registry returned {pages} consecutive empty page(s) while still offering a continuation token; stopping to avoid an endless walk")]
    EmptyPageLoop { pages: u32 },
}

/// `a`/`an` for the mode name, so messages read naturally.
fn article(mode: PaginationMode) -> &'static str {
    match mode {
        PaginationMode::Offset => "n",
        PaginationMode::Cursor => "",
    }
}
