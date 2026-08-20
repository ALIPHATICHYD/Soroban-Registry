//! Request execution: authentication, retries, idempotency, cancellation, and
//! the mapping from HTTP responses onto the typed error taxonomy.
//!
//! Credentials are attached here and nowhere else, and never copied into an
//! error, a cache key, or a log line.

use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{HeaderMap, RETRY_AFTER};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::{Auth, ClientConfig};
use crate::error::{ApiErrorDetails, Error, Result, TransportKind};
use crate::pagination::CancelToken;

/// Header carrying the idempotency key for mutating requests.
pub const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";
/// Longest idempotency key the registry accepts.
pub const IDEMPOTENCY_KEY_MAX_LEN: usize = 255;

/// A response body cache the client may consult for safe reads.
///
/// The CLI plugs its on-disk HTTP cache in here; other consumers can ignore it.
/// Only `GET`s marked cacheable are ever looked up or stored, and authenticated
/// responses are cached exactly as any other — supply a cache that is scoped to
/// the credential if that matters to you.
pub trait ResponseCache: Send + Sync {
    /// Cached body for `key`, if any is still fresh.
    fn get(&self, key: &str) -> Option<String>;
    /// Store a successful response body.
    fn put(&self, key: &str, body: &str);
}

/// One HTTP call, described independently of how it is executed.
#[derive(Debug, Clone)]
pub struct RequestSpec {
    pub method: Method,
    /// API path such as `/api/contracts`, or an absolute URL.
    pub path: String,
    pub query: Vec<(String, String)>,
    pub body: Option<serde_json::Value>,
    /// Sent as `Idempotency-Key`. Also the only thing that makes a mutation
    /// eligible for retrying.
    pub idempotency_key: Option<String>,
    /// Allow the [`ResponseCache`] to serve this request (`GET` only).
    pub cacheable: bool,
}

impl RequestSpec {
    pub fn new(method: Method, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            query: Vec::new(),
            body: None,
            idempotency_key: None,
            cacheable: false,
        }
    }

    /// A cacheable `GET`.
    pub fn get(path: impl Into<String>) -> Self {
        Self::new(Method::GET, path).cacheable(true)
    }

    pub fn post(path: impl Into<String>) -> Self {
        Self::new(Method::POST, path)
    }

    pub fn patch(path: impl Into<String>) -> Self {
        Self::new(Method::PATCH, path)
    }

    pub fn delete(path: impl Into<String>) -> Self {
        Self::new(Method::DELETE, path)
    }

    pub fn query_pair(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query.push((key.into(), value.into()));
        self
    }

    /// Add a query pair only when the value is `Some`.
    pub fn maybe_query<T: ToString>(self, key: &str, value: Option<T>) -> Self {
        match value {
            Some(value) => self.query_pair(key, value.to_string()),
            None => self,
        }
    }

    pub fn json_body<T: Serialize>(mut self, body: &T) -> Result<Self> {
        self.body = Some(serde_json::to_value(body).map_err(|err| {
            Error::InvalidRequest(format!("request body could not be serialized: {err}"))
        })?);
        Ok(self)
    }

    pub fn raw_body(mut self, body: serde_json::Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Attach an idempotency key, making a safe retry of this mutation possible.
    pub fn idempotency_key(mut self, key: Option<impl Into<String>>) -> Result<Self> {
        self.idempotency_key = match key {
            Some(key) => {
                let key = key.into();
                if key.is_empty() || key.len() > IDEMPOTENCY_KEY_MAX_LEN {
                    return Err(Error::InvalidRequest(format!(
                        "idempotency key must be 1..={IDEMPOTENCY_KEY_MAX_LEN} characters"
                    )));
                }
                Some(key)
            }
            None => None,
        };
        Ok(self)
    }

    pub fn cacheable(mut self, cacheable: bool) -> Self {
        self.cacheable = cacheable;
        self
    }

    /// `GET`/`HEAD`/`OPTIONS` — free of side effects, so always safe to repeat.
    pub fn is_safe(&self) -> bool {
        matches!(self.method, Method::GET | Method::HEAD | Method::OPTIONS)
    }
}

/// Executes [`RequestSpec`]s against one registry.
#[derive(Clone)]
pub(crate) struct Transport {
    http: reqwest::Client,
    config: Arc<ClientConfig>,
    cache: Option<Arc<dyn ResponseCache>>,
    cancel: Option<CancelToken>,
}

/// Redacts the credential-bearing config; see [`ClientConfig`]'s own `Debug`.
impl std::fmt::Debug for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transport")
            .field("config", &self.config)
            .field("cache", &self.cache.is_some())
            .field("cancellable", &self.cancel.is_some())
            .finish()
    }
}

impl Transport {
    pub(crate) fn new(config: Arc<ClientConfig>, http: reqwest::Client) -> Self {
        Self {
            http,
            config,
            cache: None,
            cancel: None,
        }
    }

    pub(crate) fn config(&self) -> &ClientConfig {
        &self.config
    }

    pub(crate) fn set_cache(&mut self, cache: Option<Arc<dyn ResponseCache>>) {
        self.cache = cache;
    }

    /// Mutate the shared configuration, leaving clones of this transport that
    /// were made earlier untouched.
    pub(crate) fn map_config(&mut self, apply: impl FnOnce(&mut ClientConfig)) {
        apply(Arc::make_mut(&mut self.config));
    }

    pub(crate) fn set_cancel_token(&mut self, cancel: Option<CancelToken>) {
        self.cancel = cancel;
    }

    pub(crate) fn cancel_token(&self) -> Option<&CancelToken> {
        self.cancel.as_ref()
    }

    /// Execute and decode a JSON response.
    pub(crate) async fn send_json<T: DeserializeOwned>(&self, spec: RequestSpec) -> Result<T> {
        let url = self.url_for(&spec.path);
        let body = self.send(spec).await?;
        serde_json::from_str(&body).map_err(|err| Error::MalformedResponse {
            url,
            reason: err.to_string(),
        })
    }

    /// Execute a request whose response body is irrelevant (204 and friends).
    pub(crate) async fn send_discard(&self, spec: RequestSpec) -> Result<()> {
        self.send(spec).await.map(|_| ())
    }

    /// Execute a request, returning the raw response body.
    ///
    /// Retries follow the configured [`crate::RetryPolicy`]: safe methods and
    /// mutations carrying an idempotency key may be repeated, nothing else is.
    pub(crate) async fn send(&self, spec: RequestSpec) -> Result<String> {
        match self.cancel.clone() {
            Some(cancel) => {
                let url = self.url_for(&spec.path);
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => Err(Error::Cancelled { url }),
                    result = self.send_inner(spec) => result,
                }
            }
            None => self.send_inner(spec).await,
        }
    }

    async fn send_inner(&self, spec: RequestSpec) -> Result<String> {
        let url = self.url_for(&spec.path);
        let policy = &self.config.retry;
        let may_retry = policy.allows_retrying(spec.is_safe(), spec.idempotency_key.is_some());
        let max_attempts = if may_retry {
            policy.max_attempts.max(1)
        } else {
            1
        };

        let cache_key = (spec.cacheable && spec.method == Method::GET)
            .then(|| cache_key(&url, &spec.query))
            .filter(|_| self.cache.is_some());
        if let (Some(cache), Some(key)) = (&self.cache, &cache_key) {
            if let Some(body) = cache.get(key) {
                return Ok(body);
            }
        }

        let mut attempt = 0;
        loop {
            attempt += 1;

            let outcome = self.attempt(&spec, &url).await;
            let error = match outcome {
                Ok(body) => {
                    if let (Some(cache), Some(key)) = (&self.cache, &cache_key) {
                        cache.put(key, &body);
                    }
                    return Ok(body);
                }
                Err(error) => error,
            };

            if attempt >= max_attempts || !error.is_transient() {
                return Err(self.finalize_error(error, attempt));
            }

            // A server-supplied Retry-After wins over the computed backoff, but
            // only up to the configured ceiling: a caller should not silently
            // block for minutes inside one call.
            let delay = match (policy.respect_retry_after, error.retry_after()) {
                (true, Some(after)) if after > policy.max_retry_after => {
                    return Err(self.finalize_error(error, attempt));
                }
                (true, Some(after)) => after,
                _ => policy.backoff_for(attempt),
            };
            if matches!(error, Error::Timeout { .. }) && !policy.retry_on_timeout {
                return Err(self.finalize_error(error, attempt));
            }

            tokio::time::sleep(delay).await;
        }
    }

    /// One attempt: send, read, classify.
    async fn attempt(&self, spec: &RequestSpec, url: &str) -> Result<String> {
        let mut builder = self
            .http
            .request(spec.method.clone(), url)
            .timeout(self.config.timeout)
            .header(reqwest::header::USER_AGENT, &self.config.user_agent);

        if !spec.query.is_empty() {
            builder = builder.query(&spec.query);
        }
        if let Some(body) = &spec.body {
            builder = builder.json(body);
        }
        if let Some(key) = &spec.idempotency_key {
            builder = builder.header(IDEMPOTENCY_KEY_HEADER, key);
        }
        // The only place a credential is ever attached.
        builder = match &self.config.auth {
            Auth::None => builder,
            Auth::Bearer(token) => builder.bearer_auth(token.expose()),
            Auth::ApiKey { header, value } => builder.header(header.as_str(), value.expose()),
        };

        let response = match builder.send().await {
            Ok(response) => response,
            Err(err) => return Err(self.transport_error(err, url)),
        };

        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await.map_err(|err| Error::Transport {
            url: url.to_string(),
            attempts: 1,
            kind: TransportKind::Body,
            reason: err.to_string(),
        })?;

        if status.is_success() {
            return Ok(body);
        }

        Err(Error::from_response(build_details(
            &spec.method,
            url,
            status,
            &headers,
            &body,
        )))
    }

    fn transport_error(&self, err: reqwest::Error, url: &str) -> Error {
        if err.is_timeout() {
            return Error::Timeout {
                url: url.to_string(),
                attempts: 1,
                timeout: self.config.timeout,
            };
        }
        let kind = if err.is_connect() {
            TransportKind::Connect
        } else if err.is_request() {
            TransportKind::Request
        } else {
            TransportKind::Other
        };
        Error::Transport {
            url: url.to_string(),
            attempts: 1,
            kind,
            // reqwest's Display carries the URL and cause, never headers.
            reason: err.to_string(),
        }
    }

    /// Stamp the real attempt count onto transport-level errors, which are
    /// constructed one attempt at a time.
    fn finalize_error(&self, error: Error, attempts: usize) -> Error {
        match error {
            Error::Transport {
                url, kind, reason, ..
            } => Error::Transport {
                url,
                attempts,
                kind,
                reason,
            },
            Error::Timeout { url, timeout, .. } => Error::Timeout {
                url,
                attempts,
                timeout,
            },
            other => other,
        }
    }

    fn url_for(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            self.config.url_for(path)
        }
    }
}

/// Flatten a serializable request into query pairs.
///
/// Lets a typed params struct from `shared` (e.g. `ContractSearchParams`) be
/// used as-is: `None` fields are dropped, scalars stringify, and sequences join
/// with commas — the shape every list filter on the API accepts.
pub fn query_pairs_from<T: Serialize>(params: &T) -> Result<Vec<(String, String)>> {
    let value = serde_json::to_value(params).map_err(|err| {
        Error::InvalidRequest(format!("query parameters could not be serialized: {err}"))
    })?;
    let serde_json::Value::Object(fields) = value else {
        return Err(Error::InvalidRequest(
            "query parameters must serialize to an object".to_string(),
        ));
    };

    let mut pairs = Vec::new();
    for (key, value) in fields {
        match value {
            serde_json::Value::Null => {}
            serde_json::Value::String(text) => pairs.push((key, text)),
            serde_json::Value::Bool(flag) => pairs.push((key, flag.to_string())),
            serde_json::Value::Number(number) => pairs.push((key, number.to_string())),
            serde_json::Value::Array(items) => {
                let joined: Vec<String> = items
                    .iter()
                    .filter_map(|item| match item {
                        serde_json::Value::String(text) => Some(text.clone()),
                        serde_json::Value::Null => None,
                        other => Some(other.to_string()),
                    })
                    .collect();
                if !joined.is_empty() {
                    pairs.push((key, joined.join(",")));
                }
            }
            // Nested objects have no query-string representation; a caller that
            // needs one should pass the field explicitly.
            serde_json::Value::Object(_) => {}
        }
    }
    pairs.sort();
    Ok(pairs)
}

/// Cache key for a safe read. Built from the URL and query only — never from a
/// header — so no credential can leak into a cache file name.
fn cache_key(url: &str, query: &[(String, String)]) -> String {
    if query.is_empty() {
        return url.to_string();
    }
    let mut pairs: Vec<String> = query
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect();
    pairs.sort();
    format!("{url}?{}", pairs.join("&"))
}

/// Pull the structured error out of a failure response.
fn build_details(
    method: &Method,
    url: &str,
    status: StatusCode,
    headers: &HeaderMap,
    body: &str,
) -> ApiErrorDetails {
    let mut details = ApiErrorDetails::new(method.as_str(), url, status.as_u16());
    details.retry_after = parse_retry_after(headers);
    details.request_id = headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        // Not JSON: keep a bounded snippet so the caller can see *something*.
        if !body.trim().is_empty() {
            details.message = Some(truncate(body.trim(), 500));
        }
        return details;
    };

    // The API answers with a flat `{code, message, details, request_id}`; a
    // nested `{"error": {…}}` shape is accepted too.
    let scope = value.get("error").unwrap_or(&value);
    let string_field = |name: &str| {
        scope
            .get(name)
            .and_then(|value| value.as_str())
            .map(str::to_string)
    };

    details.code = string_field("code").or_else(|| string_field("error_code"));
    details.message = string_field("message").or_else(|| string_field("error"));
    details.details = scope
        .get("details")
        .filter(|value| !value.is_null())
        .cloned();
    details.request_id = details
        .request_id
        .or_else(|| string_field("request_id"))
        .or_else(|| string_field("correlation_id"));

    details
}

/// `Retry-After` in delta-seconds, which is what the registry emits. An
/// HTTP-date form is ignored rather than guessed at.
fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    let raw = headers.get(RETRY_AFTER)?.to_str().ok()?;
    raw.trim().parse::<u64>().ok().map(Duration::from_secs)
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let kept: String = value.chars().take(max).collect();
    format!("{kept}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(
                reqwest::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
        headers
    }

    #[test]
    fn api_error_bodies_are_parsed_into_details() {
        let details = build_details(
            &Method::POST,
            "http://registry.test/api/contracts",
            StatusCode::UNPROCESSABLE_ENTITY,
            &headers_with(&[("x-request-id", "req-42")]),
            r#"{"code":"ValidationError","message":"name too long","details":{"field":"name"}}"#,
        );

        assert_eq!(details.status, 422);
        assert_eq!(details.code.as_deref(), Some("ValidationError"));
        assert_eq!(details.message.as_deref(), Some("name too long"));
        assert_eq!(details.request_id.as_deref(), Some("req-42"));
        assert_eq!(details.details.unwrap()["field"], "name");
    }

    #[test]
    fn nested_error_bodies_are_accepted() {
        let details = build_details(
            &Method::GET,
            "http://registry.test/api/contracts/x",
            StatusCode::NOT_FOUND,
            &HeaderMap::new(),
            r#"{"error":{"code":"NotFound","message":"no such contract"}}"#,
        );
        assert_eq!(details.code.as_deref(), Some("NotFound"));
        assert_eq!(details.message.as_deref(), Some("no such contract"));
    }

    #[test]
    fn non_json_error_bodies_keep_a_bounded_snippet() {
        let details = build_details(
            &Method::GET,
            "http://registry.test/api/contracts",
            StatusCode::BAD_GATEWAY,
            &HeaderMap::new(),
            &"upstream exploded ".repeat(100),
        );
        let message = details.message.expect("snippet");
        assert!(message.starts_with("upstream exploded"));
        assert!(message.chars().count() <= 501, "snippet must be bounded");
    }

    #[test]
    fn retry_after_seconds_are_parsed() {
        assert_eq!(
            parse_retry_after(&headers_with(&[("retry-after", "30")])),
            Some(Duration::from_secs(30))
        );
        // HTTP-date form is not guessed at.
        assert_eq!(
            parse_retry_after(&headers_with(&[(
                "retry-after",
                "Wed, 21 Oct 2026 07:28:00 GMT"
            )])),
            None
        );
        assert_eq!(parse_retry_after(&HeaderMap::new()), None);
    }

    #[test]
    fn cache_keys_are_order_independent_and_credential_free() {
        let a = cache_key(
            "http://registry.test/api/contracts",
            &[
                ("limit".to_string(), "20".to_string()),
                ("network".to_string(), "testnet".to_string()),
            ],
        );
        let b = cache_key(
            "http://registry.test/api/contracts",
            &[
                ("network".to_string(), "testnet".to_string()),
                ("limit".to_string(), "20".to_string()),
            ],
        );
        assert_eq!(a, b);
        assert!(!a.contains("Bearer"));
    }

    #[test]
    fn typed_params_flatten_into_query_pairs() {
        #[derive(serde::Serialize)]
        struct Params {
            query: Option<String>,
            limit: Option<i64>,
            verified_only: Option<bool>,
            networks: Option<Vec<String>>,
            missing: Option<String>,
        }

        let pairs = query_pairs_from(&Params {
            query: Some("swap".to_string()),
            limit: Some(20),
            verified_only: Some(true),
            networks: Some(vec!["testnet".to_string(), "mainnet".to_string()]),
            missing: None,
        })
        .expect("params flatten");

        assert_eq!(
            pairs,
            vec![
                ("limit".to_string(), "20".to_string()),
                ("networks".to_string(), "testnet,mainnet".to_string()),
                ("query".to_string(), "swap".to_string()),
                ("verified_only".to_string(), "true".to_string()),
            ]
        );
    }

    #[test]
    fn safe_methods_are_recognised() {
        assert!(RequestSpec::get("/api/contracts").is_safe());
        assert!(!RequestSpec::post("/api/contracts").is_safe());
        assert!(!RequestSpec::patch("/api/contracts/x/metadata").is_safe());
        assert!(!RequestSpec::delete("/api/webhooks/x").is_safe());
    }

    #[test]
    fn idempotency_keys_are_validated() {
        assert!(RequestSpec::post("/api/contracts")
            .idempotency_key(Some("key-1"))
            .is_ok());
        assert!(RequestSpec::post("/api/contracts")
            .idempotency_key(Some(""))
            .is_err());
        assert!(RequestSpec::post("/api/contracts")
            .idempotency_key(Some("k".repeat(IDEMPOTENCY_KEY_MAX_LEN + 1)))
            .is_err());
        assert!(RequestSpec::post("/api/contracts")
            .idempotency_key(None::<String>)
            .is_ok());
    }

    #[test]
    fn optional_query_pairs_are_skipped_when_absent() {
        let spec = RequestSpec::get("/api/contracts")
            .maybe_query("limit", Some(20))
            .maybe_query("cursor", None::<String>);
        assert_eq!(spec.query, vec![("limit".to_string(), "20".to_string())]);
    }
}
