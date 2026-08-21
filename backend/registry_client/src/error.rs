//! Error types for the registry client.
//!
//! The registry's failure modes are kept apart rather than collapsed into one
//! "request failed": callers need to tell an expired session (401) from a
//! permission problem (403), a duplicate publish (409) from a validation
//! failure (422), and a rate limit (429) from a temporary upstream fault (5xx),
//! because the correct response differs in every case.
//!
//! Nothing in here carries request headers or credentials, so an error is safe
//! to log or bubble up verbatim.

use std::fmt;
use std::time::Duration;

use crate::config::REDACTED;
use crate::pagination::PaginationMode;

pub type Result<T> = std::result::Result<T, Error>;

/// The structured part of an API error response.
///
/// The registry answers failures with `{code, message, details, request_id, …}`;
/// all of it is preserved so callers can branch on `code` and surface `details`
/// to users without re-parsing the body.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApiErrorDetails {
    pub method: String,
    pub url: String,
    pub status: u16,
    /// Server error code, e.g. `INVALID_CURSOR`, `ContractAlreadyExists`.
    pub code: Option<String>,
    pub message: Option<String>,
    /// Structured `details` payload, preserved verbatim.
    pub details: Option<serde_json::Value>,
    /// Correlation id from the body or the `x-request-id` header.
    pub request_id: Option<String>,
    /// Parsed `Retry-After`, when the response carried one.
    pub retry_after: Option<Duration>,
}

impl ApiErrorDetails {
    pub fn new(method: impl Into<String>, url: impl Into<String>, status: u16) -> Self {
        Self {
            method: method.into(),
            url: url.into(),
            status,
            ..Self::default()
        }
    }

    /// `code`, or the HTTP status as a string when the server sent none.
    pub fn code_or_status(&self) -> String {
        self.code.clone().unwrap_or_else(|| self.status.to_string())
    }
}

impl fmt::Display for ApiErrorDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} returned {}", self.method, self.url, self.status)?;
        match (self.code.as_deref(), self.message.as_deref()) {
            (Some(code), Some(message)) => write!(f, " ({code}: {message})")?,
            (Some(code), None) => write!(f, " ({code})")?,
            (None, Some(message)) => write!(f, " ({message})")?,
            (None, None) => {}
        }
        if let Some(request_id) = &self.request_id {
            write!(f, " [request_id={request_id}]")?;
        }
        Ok(())
    }
}

/// Why a transport-level attempt failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    /// The connection could not be established.
    Connect,
    /// The request could not be sent or built.
    Request,
    /// The response body could not be read.
    Body,
    /// Something else reqwest reported.
    Other,
}

impl fmt::Display for TransportKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TransportKind::Connect => "could not connect to the registry",
            TransportKind::Request => "the request could not be sent",
            TransportKind::Body => "the response body could not be read",
            TransportKind::Other => "the network request failed",
        })
    }
}

/// Anything that can go wrong while talking to the registry.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The request never produced a response, after any retries.
    #[error("request to {url} failed after {attempts} attempt(s): {kind}. Check the API URL and your network connection.")]
    Transport {
        url: String,
        attempts: usize,
        kind: TransportKind,
        /// Transport-level detail. Never contains headers or credentials.
        reason: String,
    },

    /// The request exceeded the configured timeout.
    #[error("request to {url} timed out after {}s ({attempts} attempt(s))", timeout.as_secs())]
    Timeout {
        url: String,
        attempts: usize,
        timeout: Duration,
    },

    /// 401 — missing, expired, or invalid credentials.
    ///
    /// The details are boxed so that `Result<T, Error>` stays cheap to return.
    #[error("authentication failed: {0}")]
    Unauthorized(Box<ApiErrorDetails>),

    /// 403 — authenticated, but not allowed to do this.
    #[error("not authorized: {0}")]
    Forbidden(Box<ApiErrorDetails>),

    /// 404 — no such resource.
    #[error("not found: {0}")]
    NotFound(Box<ApiErrorDetails>),

    /// 400/422 — the request was rejected as invalid.
    #[error("invalid request: {0}")]
    Validation(Box<ApiErrorDetails>),

    /// 409 — conflicting state, e.g. a contract already registered.
    #[error("conflict: {0}")]
    Conflict(Box<ApiErrorDetails>),

    /// 409 with `IdempotencyKeyInProgress` — an identical request carrying the
    /// same idempotency key is still running. Retry it later; do not re-send
    /// with a fresh key, which would duplicate the effect.
    #[error("idempotent request already in flight: {0}")]
    IdempotencyInProgress(Box<ApiErrorDetails>),

    /// 429 — rate limited. `retry_after` is the server's own advice, when given.
    #[error("rate limited: {details}{}", retry_after_suffix(retry_after))]
    RateLimited {
        details: Box<ApiErrorDetails>,
        retry_after: Option<Duration>,
    },

    /// 5xx — the registry or something behind it is temporarily unhealthy.
    #[error("registry unavailable: {0}")]
    Upstream(Box<ApiErrorDetails>),

    /// A non-success status that is none of the above.
    #[error("{0}")]
    Api(Box<ApiErrorDetails>),

    /// A 2xx response whose body was not what this client expects.
    #[error("could not decode the response from {url}: {reason}")]
    MalformedResponse { url: String, reason: String },

    /// The client was asked to build an impossible request.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// The caller cancelled the request.
    #[error("request to {url} was cancelled")]
    Cancelled { url: String },

    /// A pagination walk could not continue safely.
    #[error(transparent)]
    Pagination(#[from] PaginationError),
}

impl Error {
    /// The HTTP status, for errors that came from a response.
    pub fn status(&self) -> Option<u16> {
        self.details().map(|details| details.status)
    }

    /// The server's error code, when one was supplied.
    pub fn code(&self) -> Option<&str> {
        self.details().and_then(|details| details.code.as_deref())
    }

    /// The server's structured `details` payload, when one was supplied.
    pub fn error_details(&self) -> Option<&serde_json::Value> {
        self.details().and_then(|details| details.details.as_ref())
    }

    /// The correlation id, for quoting in a bug report.
    pub fn request_id(&self) -> Option<&str> {
        self.details()
            .and_then(|details| details.request_id.as_deref())
    }

    /// `Retry-After`, when the server sent one.
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::RateLimited { retry_after, .. } => *retry_after,
            other => other.details().and_then(|details| details.retry_after),
        }
    }

    /// The structured API error, for the variants that carry one.
    pub fn details(&self) -> Option<&ApiErrorDetails> {
        match self {
            Error::Unauthorized(details)
            | Error::Forbidden(details)
            | Error::NotFound(details)
            | Error::Validation(details)
            | Error::Conflict(details)
            | Error::IdempotencyInProgress(details)
            | Error::Upstream(details)
            | Error::Api(details) => Some(details),
            Error::RateLimited { details, .. } => Some(details),
            _ => None,
        }
    }

    /// True when repeating the request could plausibly succeed — a transport
    /// fault, a timeout, a rate limit, or a 5xx. Says nothing about whether the
    /// request is *safe* to repeat; that is [`crate::RetryPolicy::allows_retrying`].
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Error::Transport { .. }
                | Error::Timeout { .. }
                | Error::RateLimited { .. }
                | Error::Upstream { .. }
                | Error::IdempotencyInProgress(_)
        )
    }

    /// True for 401/403 — the caller needs different credentials, not a retry.
    pub fn is_auth(&self) -> bool {
        matches!(self, Error::Unauthorized(_) | Error::Forbidden(_))
    }

    /// True when the registry rejected the continuation token we sent. Restart
    /// the walk rather than retrying with the same cursor.
    pub fn is_invalid_cursor(&self) -> bool {
        matches!(self.status(), Some(400) | Some(422))
            && self.code().is_some_and(|code| {
                code.eq_ignore_ascii_case("INVALID_CURSOR") || code == "InvalidPaginationCursor"
            })
    }

    /// Classify a non-success response into the taxonomy above.
    pub fn from_response(details: ApiErrorDetails) -> Self {
        let retry_after = details.retry_after;
        let status = details.status;
        let is_idempotency_replay = details
            .code
            .as_deref()
            .is_some_and(|code| code.eq_ignore_ascii_case("IdempotencyKeyInProgress"));
        let details = Box::new(details);

        match status {
            401 => Error::Unauthorized(details),
            403 => Error::Forbidden(details),
            404 => Error::NotFound(details),
            409 if is_idempotency_replay => Error::IdempotencyInProgress(details),
            409 => Error::Conflict(details),
            400 | 422 => Error::Validation(details),
            429 => Error::RateLimited {
                details,
                retry_after,
            },
            status if (500..600).contains(&status) => Error::Upstream(details),
            _ => Error::Api(details),
        }
    }
}

fn retry_after_suffix(retry_after: &Option<Duration>) -> String {
    match retry_after {
        Some(after) => format!(" (retry after {}s)", after.as_secs()),
        None => String::new(),
    }
}

/// Assert-friendly helper for tests and callers that log errors: confirms a
/// rendered message carries no credential material.
pub fn is_redacted(rendered: &str, secret: &str) -> bool {
    !rendered.contains(secret) || rendered.contains(REDACTED)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn details(status: u16, code: Option<&str>) -> ApiErrorDetails {
        ApiErrorDetails {
            method: "POST".to_string(),
            url: "http://registry.test/api/contracts".to_string(),
            status,
            code: code.map(str::to_string),
            message: Some("boom".to_string()),
            details: None,
            request_id: Some("req-1".to_string()),
            retry_after: None,
        }
    }

    #[test]
    fn statuses_map_to_distinct_variants() {
        assert!(matches!(
            Error::from_response(details(401, None)),
            Error::Unauthorized(_)
        ));
        assert!(matches!(
            Error::from_response(details(403, None)),
            Error::Forbidden(_)
        ));
        assert!(matches!(
            Error::from_response(details(404, None)),
            Error::NotFound(_)
        ));
        assert!(matches!(
            Error::from_response(details(409, Some("ContractAlreadyExists"))),
            Error::Conflict(_)
        ));
        assert!(matches!(
            Error::from_response(details(422, None)),
            Error::Validation(_)
        ));
        assert!(matches!(
            Error::from_response(details(400, None)),
            Error::Validation(_)
        ));
        assert!(matches!(
            Error::from_response(details(429, None)),
            Error::RateLimited { .. }
        ));
        assert!(matches!(
            Error::from_response(details(503, None)),
            Error::Upstream(_)
        ));
        assert!(matches!(
            Error::from_response(details(418, None)),
            Error::Api(_)
        ));
    }

    #[test]
    fn an_in_flight_idempotent_replay_is_its_own_variant() {
        let error = Error::from_response(details(409, Some("IdempotencyKeyInProgress")));
        assert!(matches!(error, Error::IdempotencyInProgress(_)));
        assert!(error.is_transient(), "the caller should retry it later");
    }

    #[test]
    fn structured_details_are_preserved() {
        let mut raw = details(422, Some("ValidationError"));
        raw.details = Some(serde_json::json!({"field": "name", "reason": "too long"}));
        raw.retry_after = Some(Duration::from_secs(4));
        let error = Error::from_response(raw);

        assert_eq!(error.status(), Some(422));
        assert_eq!(error.code(), Some("ValidationError"));
        assert_eq!(error.request_id(), Some("req-1"));
        assert_eq!(
            error
                .error_details()
                .and_then(|value| value["field"].as_str()),
            Some("name")
        );
        assert_eq!(error.retry_after(), Some(Duration::from_secs(4)));
    }

    #[test]
    fn rate_limit_errors_surface_retry_after() {
        let mut raw = details(429, Some("RateLimited"));
        raw.retry_after = Some(Duration::from_secs(30));
        let error = Error::from_response(raw);

        assert_eq!(error.retry_after(), Some(Duration::from_secs(30)));
        assert!(error.is_transient());
        assert!(error.to_string().contains("retry after 30s"), "{error}");
    }

    #[test]
    fn transient_and_auth_errors_are_distinguishable() {
        assert!(Error::from_response(details(503, None)).is_transient());
        assert!(!Error::from_response(details(422, None)).is_transient());
        assert!(Error::from_response(details(401, None)).is_auth());
        assert!(Error::from_response(details(403, None)).is_auth());
        assert!(!Error::from_response(details(409, None)).is_auth());
    }

    #[test]
    fn display_includes_code_message_and_request_id() {
        let rendered =
            Error::from_response(details(409, Some("ContractAlreadyExists"))).to_string();
        assert!(rendered.contains("ContractAlreadyExists"), "{rendered}");
        assert!(rendered.contains("boom"), "{rendered}");
        assert!(rendered.contains("req-1"), "{rendered}");
    }

    #[test]
    fn invalid_cursor_is_recognised_from_either_code_spelling() {
        assert!(Error::from_response(details(400, Some("INVALID_CURSOR"))).is_invalid_cursor());
        assert!(
            Error::from_response(details(400, Some("InvalidPaginationCursor"))).is_invalid_cursor()
        );
        assert!(!Error::from_response(details(400, Some("EMPTY_QUERY"))).is_invalid_cursor());
    }
}
