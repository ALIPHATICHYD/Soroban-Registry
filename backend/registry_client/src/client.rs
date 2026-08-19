//! The client type: configuration, transport wiring, and the escape hatch for
//! endpoints the typed API does not cover yet.
//!
//! Endpoint methods live next door in [`crate::api`], grouped by domain.

use std::sync::Arc;
use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::config::{Auth, ClientConfig, RetryPolicy};
use crate::error::{Error, Result};
use crate::http::{RequestSpec, ResponseCache, Transport};
use crate::pagination::CancelToken;

/// Client for the Soroban Registry HTTP API.
///
/// Cloning is cheap: the configuration is shared and the underlying
/// `reqwest::Client` keeps its connection pool.
///
/// ```no_run
/// use registry_client::{Auth, ClientConfig, RegistryClient, RetryPolicy};
/// # fn main() -> registry_client::Result<()> {
/// let client = RegistryClient::from_config(
///     ClientConfig::new("https://registry.example")
///         .with_auth(Auth::bearer(std::env::var("REGISTRY_TOKEN").unwrap_or_default()))
///         .with_timeout(std::time::Duration::from_secs(15))
///         .with_user_agent("my-tool/1.0")
///         .with_retry_policy(RetryPolicy::attempts(5)),
/// )?;
/// # let _ = client;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct RegistryClient {
    pub(crate) transport: Transport,
}

impl RegistryClient {
    /// A client for `base_url` (e.g. `http://localhost:3001`) with defaults.
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        Self::from_config(ClientConfig::new(base_url))
    }

    /// A client from a full configuration.
    pub fn from_config(config: ClientConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|err| {
                Error::InvalidRequest(format!("could not build an HTTP client: {err}"))
            })?;
        Ok(Self::with_http_client(config, http))
    }

    /// A client using a caller-supplied `reqwest::Client`, for consumers that
    /// already configure proxies, TLS, or connection pooling.
    pub fn with_http_client(config: ClientConfig, http: reqwest::Client) -> Self {
        Self {
            transport: Transport::new(Arc::new(config), http),
        }
    }

    pub fn config(&self) -> &ClientConfig {
        self.transport.config()
    }

    pub fn base_url(&self) -> &str {
        &self.transport.config().base_url
    }

    /// Replace the authentication scheme.
    pub fn with_auth(mut self, auth: Auth) -> Self {
        self.map_config(|config| config.auth = auth);
        self
    }

    /// Bearer authentication, or anonymous when `token` is `None`.
    pub fn with_bearer_token(mut self, token: Option<impl Into<String>>) -> Self {
        let auth = match token {
            Some(token) => Auth::bearer(token),
            None => Auth::None,
        };
        self.map_config(|config| config.auth = auth);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.map_config(|config| config.timeout = timeout);
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        let user_agent = user_agent.into();
        self.map_config(|config| config.user_agent = user_agent);
        self
    }

    /// Replace the retry policy. [`RetryPolicy::none`] disables retrying.
    pub fn with_retry_policy(mut self, retry: RetryPolicy) -> Self {
        self.map_config(|config| config.retry = retry);
        self
    }

    /// Serve cacheable `GET`s from `cache` before hitting the network.
    pub fn with_response_cache(mut self, cache: Arc<dyn ResponseCache>) -> Self {
        self.transport.set_cache(Some(cache));
        self
    }

    /// Abort in-flight requests when `cancel` fires.
    pub fn with_cancel_token(mut self, cancel: CancelToken) -> Self {
        self.transport.set_cancel_token(Some(cancel));
        self
    }

    /// The cancellation token this client honours, if any.
    pub fn cancel_token(&self) -> Option<CancelToken> {
        self.transport.cancel_token().cloned()
    }

    // ── Escape hatch ─────────────────────────────────────────────────────────

    /// Execute an arbitrary request and decode the JSON response, for endpoints
    /// this crate does not wrap yet. Retry, auth, cancellation, and error
    /// classification behave exactly as for the typed methods.
    pub async fn send_json<T: DeserializeOwned>(&self, spec: RequestSpec) -> Result<T> {
        self.transport.send_json(spec).await
    }

    /// Execute an arbitrary request, returning the raw response body.
    pub async fn send_raw(&self, spec: RequestSpec) -> Result<String> {
        self.transport.send(spec).await
    }

    /// `GET` a JSON resource by path, e.g. `/api/contracts/tags`.
    pub async fn get_json<T: DeserializeOwned>(&self, path: impl Into<String>) -> Result<T> {
        self.transport.send_json(RequestSpec::get(path)).await
    }

    fn map_config(&mut self, apply: impl FnOnce(&mut ClientConfig)) {
        self.transport.map_config(apply);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::REDACTED;

    #[test]
    fn base_url_trailing_slash_is_normalised() {
        let client = RegistryClient::new("http://registry.test/").expect("client");
        assert_eq!(client.base_url(), "http://registry.test");
    }

    #[test]
    fn builder_methods_update_the_shared_config() {
        let client = RegistryClient::new("http://registry.test")
            .expect("client")
            .with_user_agent("my-tool/2.0")
            .with_timeout(Duration::from_secs(3))
            .with_retry_policy(RetryPolicy::none())
            .with_bearer_token(Some("token-abc"));

        assert_eq!(client.config().user_agent, "my-tool/2.0");
        assert_eq!(client.config().timeout, Duration::from_secs(3));
        assert_eq!(client.config().retry.max_attempts, 1);
        assert!(matches!(client.config().auth, Auth::Bearer(_)));
    }

    #[test]
    fn cloned_clients_do_not_share_later_config_edits() {
        let anonymous = RegistryClient::new("http://registry.test").expect("client");
        let authenticated = anonymous.clone().with_bearer_token(Some("token-abc"));

        assert!(anonymous.config().auth.is_anonymous());
        assert!(!authenticated.config().auth.is_anonymous());
    }

    #[test]
    fn debug_output_never_contains_the_token() {
        let client = RegistryClient::new("http://registry.test")
            .expect("client")
            .with_bearer_token(Some("token-abc"));
        let rendered = format!("{client:?}");
        assert!(!rendered.contains("token-abc"), "{rendered}");
        assert!(rendered.contains(REDACTED), "{rendered}");
    }
}
