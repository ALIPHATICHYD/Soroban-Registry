//! HTTP client for the registry search endpoints.

use std::time::Duration;

use reqwest::{Method, StatusCode};

use crate::error::{Error, Result};
use crate::models::{ContractHit, ContractSearchRequest, RawSearchResponse};
use crate::pagination::{
    PageFetcher, PageFuture, PageLimits, PageRequest, Paginator, RegistryPage,
};

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_ATTEMPTS: usize = 3;
const BACKOFFS_MS: [u64; 2] = [250, 750];

/// Client for the Soroban Registry HTTP API.
///
/// Cloning is cheap; the underlying `reqwest::Client` shares its connection pool.
#[derive(Debug, Clone)]
pub struct RegistryClient {
    http: reqwest::Client,
    base_url: String,
    bearer_token: Option<String>,
    max_attempts: usize,
}

impl RegistryClient {
    /// A client for `base_url` (e.g. `http://localhost:3001`).
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|err| {
                Error::InvalidRequest(format!("could not build an HTTP client: {err}"))
            })?;
        Ok(Self::with_http_client(base_url, http))
    }

    /// A client using a caller-supplied `reqwest::Client`, for callers that
    /// already configure timeouts, proxies or TLS.
    pub fn with_http_client(base_url: impl Into<String>, http: reqwest::Client) -> Self {
        Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            bearer_token: None,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
        }
    }

    /// Send `Authorization: Bearer …` with every request.
    pub fn with_bearer_token(mut self, token: Option<String>) -> Self {
        self.bearer_token = token;
        self
    }

    /// Attempts per page fetch, retries included (minimum 1).
    ///
    /// Retries happen inside a single page fetch, so they can never cause a
    /// paginated walk to emit an item twice.
    pub fn with_max_attempts(mut self, max_attempts: usize) -> Self {
        self.max_attempts = max_attempts.max(1);
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Fetch a single page of search results.
    ///
    /// `continuation` is `None` for the first page. Most callers want
    /// [`RegistryClient::search_paginator`] instead, which manages continuations.
    pub async fn search_page(
        &self,
        request: &ContractSearchRequest,
        continuation: Option<crate::pagination::PageCursor>,
        limit: u32,
    ) -> Result<RegistryPage<ContractHit>> {
        request.validate()?;

        let mode = request.mode;
        let endpoint = request.effective_endpoint();
        let url = format!("{}{}", self.base_url, endpoint.path());

        let mut params: Vec<(&str, String)> =
            vec![("q", request.query.clone()), ("limit", limit.to_string())];

        // Offset walks report where they are; cursor walks send the opaque token
        // (empty on the first page, which is how the API is asked for a cursor
        // walk at all). The two are never sent together.
        let mut current_offset = 0_u64;
        match mode {
            crate::pagination::PaginationMode::Cursor => {
                let token = continuation
                    .as_ref()
                    .and_then(|cursor| cursor.as_cursor())
                    .unwrap_or("");
                params.push(("cursor", token.to_string()));
            }
            crate::pagination::PaginationMode::Offset => {
                current_offset = continuation
                    .as_ref()
                    .and_then(|cursor| cursor.as_offset())
                    .or(request.offset)
                    .unwrap_or(0);
                params.push(("offset", current_offset.to_string()));
            }
        }

        if !request.networks.is_empty() {
            params.push(("networks", request.networks.join(",")));
        }
        if !request.categories.is_empty() {
            params.push(("categories", request.categories.join(",")));
        }
        if !request.tags.is_empty() {
            params.push(("tags", request.tags.join(",")));
        }
        if request.verified_only {
            params.push(("verified_only", "true".to_string()));
        }

        let body = self.get(&url, &params).await?;
        let raw: RawSearchResponse = serde_json::from_str(&body).map_err(|err| Error::Decode {
            url: url.clone(),
            reason: err.to_string(),
        })?;

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

    /// GET with bounded retries for transient failures. Returns the body of the
    /// first non-retryable response, or an error for a non-success status.
    async fn get(&self, url: &str, params: &[(&str, String)]) -> Result<String> {
        let mut last_transport_error: Option<String> = None;

        for attempt in 1..=self.max_attempts {
            let mut builder = self.http.get(url).query(params);
            if let Some(token) = &self.bearer_token {
                builder = builder.bearer_auth(token);
            }

            match builder.send().await {
                Ok(response) => {
                    let status = response.status();
                    if is_retryable_status(status) && attempt < self.max_attempts {
                        sleep_before_retry(attempt).await;
                        continue;
                    }

                    let body = response.text().await.map_err(|err| Error::Decode {
                        url: url.to_string(),
                        reason: err.to_string(),
                    })?;

                    if !status.is_success() {
                        let (code, message) = extract_api_error(&body);
                        return Err(Error::Api {
                            method: Method::GET.to_string(),
                            url: url.to_string(),
                            status: status.as_u16(),
                            code,
                            message,
                        });
                    }

                    return Ok(body);
                }
                Err(err) => {
                    let reason = describe_transport_error(&err);
                    if is_transient(&err) && attempt < self.max_attempts {
                        last_transport_error = Some(reason);
                        sleep_before_retry(attempt).await;
                        continue;
                    }
                    return Err(Error::Transport {
                        url: url.to_string(),
                        attempts: attempt,
                        reason,
                    });
                }
            }
        }

        Err(Error::Transport {
            url: url.to_string(),
            attempts: self.max_attempts,
            reason: last_transport_error
                .unwrap_or_else(|| "the registry kept returning a retryable status".to_string()),
        })
    }
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

fn is_retryable_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::REQUEST_TIMEOUT
            | StatusCode::TOO_MANY_REQUESTS
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

/// Search requests are GETs, so retrying any send failure is safe.
fn is_transient(err: &reqwest::Error) -> bool {
    err.is_connect() || err.is_timeout() || err.is_request()
}

fn describe_transport_error(err: &reqwest::Error) -> String {
    if err.is_timeout() {
        "the request timed out".to_string()
    } else if err.is_connect() {
        "could not connect to the registry".to_string()
    } else if err.is_request() {
        "the request could not be sent".to_string()
    } else {
        err.to_string()
    }
}

/// Pull `code`/`message` out of an API error body, tolerating both the flat
/// shape the API emits and a nested `{"error": {…}}` shape.
fn extract_api_error(body: &str) -> (Option<String>, Option<String>) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return (None, None);
    };
    let scope = value.get("error").unwrap_or(&value);
    let field = |name: &str| {
        scope
            .get(name)
            .and_then(|value| value.as_str())
            .map(str::to_string)
    };
    (
        field("code").or_else(|| field("error_code")),
        field("message"),
    )
}

async fn sleep_before_retry(attempt: usize) {
    let index = attempt.saturating_sub(1).min(BACKOFFS_MS.len() - 1);
    tokio::time::sleep(Duration::from_millis(BACKOFFS_MS[index])).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pagination::PaginationMode;

    #[test]
    fn base_url_trailing_slash_is_normalised() {
        let client = RegistryClient::new("http://registry.test/").expect("client");
        assert_eq!(client.base_url(), "http://registry.test");
    }

    #[test]
    fn api_error_bodies_are_parsed() {
        let (code, message) = extract_api_error(
            r#"{"code":"INVALID_CURSOR","message":"The provided pagination cursor is invalid"}"#,
        );
        assert_eq!(code.as_deref(), Some("INVALID_CURSOR"));
        assert_eq!(
            message.as_deref(),
            Some("The provided pagination cursor is invalid")
        );

        let (nested_code, _) =
            extract_api_error(r#"{"error":{"code":"InvalidPaginationCursor","message":"nope"}}"#);
        assert_eq!(nested_code.as_deref(), Some("InvalidPaginationCursor"));

        assert_eq!(extract_api_error("not json"), (None, None));
    }

    #[test]
    fn invalid_cursor_errors_are_recognisable() {
        let err = Error::Api {
            method: "GET".to_string(),
            url: "http://registry.test/api/search".to_string(),
            status: 400,
            code: Some("INVALID_CURSOR".to_string()),
            message: Some("The provided pagination cursor is invalid".to_string()),
        };
        assert!(err.is_invalid_cursor());
        assert_eq!(err.status(), Some(400));
    }

    #[test]
    fn mixing_a_cursor_and_an_offset_fails_before_any_request() {
        let client = RegistryClient::new("http://registry.test").expect("client");
        let request = crate::models::ContractSearchRequest::new("swap", PaginationMode::Cursor)
            .with_offset(Some(20));

        assert!(client
            .search_paginator(request, PageLimits::default())
            .is_err());
    }
}
