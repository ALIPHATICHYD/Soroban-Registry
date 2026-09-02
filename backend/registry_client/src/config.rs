//! Client configuration: base URL, authentication, timeouts, user agent, and
//! retry policy.
//!
//! Credentials are wrapped in [`Secret`], whose `Debug` and `Display` print a
//! placeholder. Config and error values are therefore safe to log: a bearer
//! token, API key, or signing secret never reaches a log line through them.

use std::fmt;
use std::time::Duration;

/// Default per-request timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
/// Default number of attempts for a retryable request (1 try + 2 retries).
pub const DEFAULT_MAX_ATTEMPTS: usize = 3;
/// Longest `Retry-After` this client will wait for by default.
pub const DEFAULT_MAX_RETRY_AFTER: Duration = Duration::from_secs(60);

/// A credential that must never be printed.
///
/// The inner value is reachable only through [`Secret::expose`], which is
/// deliberately noisy at call sites.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

/// What every redacted value renders as.
pub const REDACTED: &str = "<redacted>";

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The raw credential. Only for putting it on the wire.
    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(REDACTED)
    }
}

impl<T: Into<String>> From<T> for Secret {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

/// How requests authenticate.
#[derive(Clone, Default, PartialEq, Eq)]
pub enum Auth {
    /// Anonymous — public read endpoints only.
    #[default]
    None,
    /// `Authorization: Bearer <token>`.
    Bearer(Secret),
    /// A custom header, e.g. `X-API-Key: <key>`.
    ApiKey { header: String, value: Secret },
}

impl Auth {
    pub fn bearer(token: impl Into<String>) -> Self {
        Auth::Bearer(Secret::new(token))
    }

    pub fn api_key(header: impl Into<String>, value: impl Into<String>) -> Self {
        Auth::ApiKey {
            header: header.into(),
            value: Secret::new(value),
        }
    }

    pub fn is_anonymous(&self) -> bool {
        matches!(self, Auth::None)
    }
}

/// Redacts the credential; only the *scheme* is identifiable.
impl fmt::Debug for Auth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Auth::None => f.write_str("None"),
            Auth::Bearer(_) => write!(f, "Bearer({REDACTED})"),
            Auth::ApiKey { header, .. } => {
                write!(f, "ApiKey {{ header: {header:?}, value: {REDACTED} }}")
            }
        }
    }
}

/// When and how often a failed request is retried.
///
/// Retries are opt-out, explicit, and never silent about *what* is retried:
/// a request is only ever retried when it is safe to repeat — see
/// [`RetryPolicy::allows_retrying`].
#[derive(Debug, Clone, PartialEq)]
pub struct RetryPolicy {
    /// Total attempts, retries included. `1` disables retrying.
    pub max_attempts: usize,
    /// Backoff before the first retry.
    pub initial_backoff: Duration,
    /// Upper bound on the computed backoff.
    pub max_backoff: Duration,
    /// Growth factor applied per retry.
    pub backoff_multiplier: f64,
    /// Honour a `Retry-After` header instead of the computed backoff.
    pub respect_retry_after: bool,
    /// Refuse to wait longer than this for a `Retry-After`; a longer one is
    /// surfaced as a [`crate::Error::RateLimited`] instead of blocking.
    pub max_retry_after: Duration,
    /// Retry requests that timed out.
    pub retry_on_timeout: bool,
    /// Retry mutating requests (POST/PATCH) when they carry an idempotency
    /// key. With no key, a mutation is never retried, whatever this says.
    pub retry_idempotent_mutations: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            initial_backoff: Duration::from_millis(250),
            max_backoff: Duration::from_secs(5),
            backoff_multiplier: 3.0,
            respect_retry_after: true,
            max_retry_after: DEFAULT_MAX_RETRY_AFTER,
            retry_on_timeout: true,
            retry_idempotent_mutations: true,
        }
    }
}

impl RetryPolicy {
    /// No retries: every request is attempted exactly once.
    pub fn none() -> Self {
        Self {
            max_attempts: 1,
            ..Self::default()
        }
    }

    /// `attempts` tries in total (clamped to at least 1).
    pub fn attempts(attempts: usize) -> Self {
        Self {
            max_attempts: attempts.max(1),
            ..Self::default()
        }
    }

    pub fn with_initial_backoff(mut self, backoff: Duration) -> Self {
        self.initial_backoff = backoff;
        self
    }

    pub fn with_max_backoff(mut self, backoff: Duration) -> Self {
        self.max_backoff = backoff;
        self
    }

    pub fn with_respect_retry_after(mut self, respect: bool) -> Self {
        self.respect_retry_after = respect;
        self
    }

    pub fn with_max_retry_after(mut self, max: Duration) -> Self {
        self.max_retry_after = max;
        self
    }

    pub fn with_retry_on_timeout(mut self, retry: bool) -> Self {
        self.retry_on_timeout = retry;
        self
    }

    pub fn with_retry_idempotent_mutations(mut self, retry: bool) -> Self {
        self.retry_idempotent_mutations = retry;
        self
    }

    /// Whether a request may be repeated at all.
    ///
    /// Safe methods (GET/HEAD/OPTIONS) always may. A mutation may only when the
    /// caller supplied an idempotency key *and* the policy allows it — repeating
    /// a keyless publish would risk registering a contract twice.
    pub fn allows_retrying(&self, safe_method: bool, has_idempotency_key: bool) -> bool {
        if self.max_attempts <= 1 {
            return false;
        }
        safe_method || (has_idempotency_key && self.retry_idempotent_mutations)
    }

    /// Backoff before the retry that follows `attempt` (1-based).
    pub fn backoff_for(&self, attempt: usize) -> Duration {
        let exponent = attempt.saturating_sub(1).min(16) as i32;
        let scaled = self.initial_backoff.as_secs_f64() * self.backoff_multiplier.powi(exponent);
        let capped = scaled.min(self.max_backoff.as_secs_f64()).max(0.0);
        Duration::from_secs_f64(capped)
    }
}

/// Everything a [`crate::RegistryClient`] needs, independent of transport.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Registry base URL, without a trailing slash.
    pub base_url: String,
    pub auth: Auth,
    /// Per-request timeout, applied to each attempt.
    pub timeout: Duration,
    /// `User-Agent` sent with every request.
    pub user_agent: String,
    pub retry: RetryPolicy,
}

impl ClientConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: normalize_base_url(base_url),
            auth: Auth::None,
            timeout: DEFAULT_TIMEOUT,
            user_agent: default_user_agent(),
            retry: RetryPolicy::default(),
        }
    }

    pub fn with_auth(mut self, auth: Auth) -> Self {
        self.auth = auth;
        self
    }

    /// Bearer authentication, or anonymous when `token` is `None`.
    pub fn with_bearer_token(mut self, token: Option<impl Into<String>>) -> Self {
        self.auth = match token {
            Some(token) => Auth::bearer(token),
            None => Auth::None,
        };
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    pub fn with_retry_policy(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Absolute URL for an API path such as `/api/contracts`.
    pub fn url_for(&self, path: &str) -> String {
        if path.starts_with('/') {
            format!("{}{}", self.base_url, path)
        } else {
            format!("{}/{}", self.base_url, path)
        }
    }
}

fn normalize_base_url(base_url: impl Into<String>) -> String {
    base_url.into().trim_end_matches('/').to_string()
}

/// `soroban-registry-client/<version>` — identifies the SDK to the registry.
pub fn default_user_agent() -> String {
    format!("soroban-registry-client/{}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secrets_are_redacted_in_debug_and_display() {
        let secret = Secret::new("super-secret-token");
        assert_eq!(format!("{secret:?}"), REDACTED);
        assert_eq!(format!("{secret}"), REDACTED);
        assert!(!format!("{secret:?}").contains("super-secret"));
        assert_eq!(secret.expose(), "super-secret-token");
    }

    #[test]
    fn auth_debug_keeps_the_scheme_but_drops_the_credential() {
        let bearer = format!("{:?}", Auth::bearer("token-abc"));
        assert!(bearer.contains("Bearer"));
        assert!(!bearer.contains("token-abc"));

        let api_key = format!("{:?}", Auth::api_key("X-API-Key", "key-xyz"));
        assert!(
            api_key.contains("X-API-Key"),
            "the header name is not secret"
        );
        assert!(!api_key.contains("key-xyz"));
    }

    #[test]
    fn config_debug_never_contains_the_token() {
        let config = ClientConfig::new("http://registry.test").with_auth(Auth::bearer("token-abc"));
        let rendered = format!("{config:?}");
        assert!(!rendered.contains("token-abc"), "{rendered}");
        assert!(rendered.contains(REDACTED), "{rendered}");
    }

    #[test]
    fn base_urls_are_normalised_and_joined() {
        let config = ClientConfig::new("http://registry.test/");
        assert_eq!(config.base_url, "http://registry.test");
        assert_eq!(
            config.url_for("/api/contracts"),
            "http://registry.test/api/contracts"
        );
        assert_eq!(
            config.url_for("api/contracts"),
            "http://registry.test/api/contracts"
        );
    }

    #[test]
    fn mutations_are_not_retried_without_an_idempotency_key() {
        let policy = RetryPolicy::default();
        assert!(policy.allows_retrying(true, false), "GET is safe to repeat");
        assert!(
            !policy.allows_retrying(false, false),
            "a keyless mutation must never be repeated"
        );
        assert!(
            policy.allows_retrying(false, true),
            "a keyed mutation may be"
        );
    }

    #[test]
    fn a_disabled_policy_never_retries() {
        let policy = RetryPolicy::none();
        assert!(!policy.allows_retrying(true, true));
    }

    #[test]
    fn opting_out_of_mutation_retries_is_honoured() {
        let policy = RetryPolicy::default().with_retry_idempotent_mutations(false);
        assert!(!policy.allows_retrying(false, true));
        assert!(policy.allows_retrying(true, false));
    }

    #[test]
    fn backoff_grows_and_is_capped() {
        let policy = RetryPolicy::default()
            .with_initial_backoff(Duration::from_millis(100))
            .with_max_backoff(Duration::from_millis(500));
        assert_eq!(policy.backoff_for(1), Duration::from_millis(100));
        assert_eq!(policy.backoff_for(2), Duration::from_millis(300));
        assert_eq!(policy.backoff_for(3), Duration::from_millis(500));
        assert_eq!(policy.backoff_for(99), Duration::from_millis(500));
    }

    #[test]
    fn the_default_user_agent_names_the_sdk() {
        assert!(default_user_agent().starts_with("soroban-registry-client/"));
    }
}
