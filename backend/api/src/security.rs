use axum::{
    body::Body,
    extract::State,
    http::{
        header::{self, HeaderName, HeaderValue},
        HeaderMap, Method, Request, StatusCode,
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use std::{collections::HashSet, time::Duration};
use tower_http::cors::CorsLayer;

use crate::error::ApiError;

pub const CSRF_COOKIE_NAME: &str = "sr_csrf";
pub const CSRF_HEADER_NAME: &str = "x-csrf-token";

const DEFAULT_ALLOWED_ORIGINS: &str = "http://localhost:3000,https://soroban-registry.vercel.app";

#[derive(Debug, Clone)]
pub struct WebSecurityConfig {
    allowed_origins: HashSet<String>,
    csrf_cookie_secure: bool,
    csrf_same_site: SameSiteMode,
}

#[derive(Debug, Clone, Copy)]
enum SameSiteMode {
    Lax,
    Strict,
    None,
}

impl SameSiteMode {
    fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "strict" => Self::Strict,
            "none" => Self::None,
            _ => Self::Lax,
        }
    }

    fn as_cookie_value(self) -> &'static str {
        match self {
            Self::Lax => "Lax",
            Self::Strict => "Strict",
            Self::None => "None",
        }
    }
}

impl WebSecurityConfig {
    pub fn from_env() -> Self {
        let raw_origins = std::env::var("CORS_ALLOWED_ORIGINS")
            .or_else(|_| std::env::var("ALLOWED_ORIGINS"))
            .unwrap_or_else(|_| DEFAULT_ALLOWED_ORIGINS.to_string());

        let allowed_origins = parse_allowed_origins(&raw_origins);
        let csrf_cookie_secure = std::env::var("CSRF_COOKIE_SECURE")
            .map(|value| !matches!(value.as_str(), "0" | "false" | "FALSE"))
            .unwrap_or(true);
        let csrf_same_site = std::env::var("CSRF_COOKIE_SAMESITE")
            .map(|value| SameSiteMode::from_env_value(&value))
            .unwrap_or(SameSiteMode::Lax);

        Self {
            allowed_origins,
            csrf_cookie_secure,
            csrf_same_site,
        }
    }

    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        self.allowed_origins.contains(origin)
    }

    pub fn build_cors_layer(&self) -> CorsLayer {
        let origins: Vec<HeaderValue> = self
            .allowed_origins
            .iter()
            .filter_map(|origin| HeaderValue::from_str(origin).ok())
            .collect();

        CorsLayer::new()
            .allow_origin(origins)
            .allow_credentials(true)
            .allow_methods([
                Method::GET,
                Method::HEAD,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                HeaderName::from_static("x-api-key"),
                HeaderName::from_static("x-api-plan"),
                HeaderName::from_static("x-mfa-token"),
                HeaderName::from_static(CSRF_HEADER_NAME),
                crate::request_tracing::X_REQUEST_ID.clone(),
                crate::request_tracing::X_CORRELATION_ID.clone(),
            ])
            .expose_headers([
                crate::request_tracing::X_REQUEST_ID.clone(),
                crate::request_tracing::X_CORRELATION_ID.clone(),
                header::RETRY_AFTER,
                header::SET_COOKIE,
                HeaderName::from_static("x-ratelimit-limit"),
                HeaderName::from_static("x-ratelimit-remaining"),
                HeaderName::from_static("x-ratelimit-reset"),
                HeaderName::from_static("x-ratelimit-tier"),
                HeaderName::from_static(CSRF_HEADER_NAME),
            ])
            .max_age(Duration::from_secs(3600))
    }

    pub fn csrf_cookie(&self, token: &str) -> String {
        let secure = if self.csrf_cookie_secure {
            "; Secure"
        } else {
            ""
        };
        format!(
            "{CSRF_COOKIE_NAME}={token}; Path=/; Max-Age=7200; SameSite={}{}; HttpOnly",
            self.csrf_same_site.as_cookie_value(),
            secure
        )
    }
}

/// HTTPS/TLS enforcement settings (#895).
///
/// The API is typically deployed behind a TLS-terminating load balancer or
/// reverse proxy that forwards the original scheme in `X-Forwarded-Proto`. This
/// layer adds HSTS to responses and (optionally) redirects plaintext requests to
/// HTTPS so all client communication is encrypted in transit.
#[derive(Debug, Clone)]
pub struct HttpsConfig {
    hsts_enabled: bool,
    hsts_max_age_secs: u64,
    hsts_include_subdomains: bool,
    redirect_to_https: bool,
}

impl Default for HttpsConfig {
    fn default() -> Self {
        Self {
            hsts_enabled: true,
            hsts_max_age_secs: 63_072_000, // 2 years (HSTS preload minimum is 1 year)
            hsts_include_subdomains: true,
            redirect_to_https: false,
        }
    }
}

impl HttpsConfig {
    pub fn from_env() -> Self {
        let defaults = Self::default();
        let bool_env = |key: &str, default: bool| {
            std::env::var(key)
                .map(|v| !matches!(v.trim().to_ascii_lowercase().as_str(), "0" | "false" | "no"))
                .unwrap_or(default)
        };

        Self {
            hsts_enabled: bool_env("HSTS_ENABLED", defaults.hsts_enabled),
            hsts_max_age_secs: std::env::var("HSTS_MAX_AGE_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.hsts_max_age_secs),
            hsts_include_subdomains: bool_env(
                "HSTS_INCLUDE_SUBDOMAINS",
                defaults.hsts_include_subdomains,
            ),
            redirect_to_https: bool_env("FORCE_HTTPS", defaults.redirect_to_https),
        }
    }

    fn hsts_header_value(&self) -> String {
        if self.hsts_include_subdomains {
            format!("max-age={}; includeSubDomains", self.hsts_max_age_secs)
        } else {
            format!("max-age={}", self.hsts_max_age_secs)
        }
    }
}

/// True when the request reached us over plaintext HTTP, per the proxy's
/// `X-Forwarded-Proto` header. Absent the header we assume HTTPS (the common
/// case for direct TLS) and do not redirect.
fn is_insecure_request(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|proto| {
            proto
                .split(',')
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("http")
        })
        .unwrap_or(false)
}

/// Enforce HTTPS in transit: optionally redirect plaintext requests and attach
/// an HSTS header to every response (#895).
pub async fn https_enforcement_middleware(
    State(config): State<HttpsConfig>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if config.redirect_to_https && is_insecure_request(request.headers()) {
        if let Some(location) = https_redirect_location(request.headers(), request.uri()) {
            if let Ok(value) = HeaderValue::from_str(&location) {
                let mut response = StatusCode::PERMANENT_REDIRECT.into_response();
                response.headers_mut().insert(header::LOCATION, value);
                return response;
            }
        }
    }

    let mut response = next.run(request).await;

    if config.hsts_enabled {
        if let Ok(value) = HeaderValue::from_str(&config.hsts_header_value()) {
            response
                .headers_mut()
                .insert(header::STRICT_TRANSPORT_SECURITY, value);
        }
    }

    response
}

fn https_redirect_location(headers: &HeaderMap, uri: &axum::http::Uri) -> Option<String> {
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok())?;
    let path_and_query = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    Some(format!("https://{host}{path_and_query}"))
}

fn parse_allowed_origins(raw: &str) -> HashSet<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty() && *origin != "*")
        .map(ToOwned::to_owned)
        .collect()
}

pub fn generate_csrf_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub async fn csrf_and_origin_middleware(
    State(config): State<WebSecurityConfig>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if is_safe_method(request.method()) {
        return next.run(request).await;
    }

    if let Err(error) = validate_origin(&config, request.headers()) {
        return error.into_response();
    }

    if csrf_required(request.headers()) {
        if let Err(error) = validate_csrf_token(request.headers()) {
            return error.into_response();
        }
    }

    next.run(request).await
}

fn validate_origin(config: &WebSecurityConfig, headers: &HeaderMap) -> Result<(), ApiError> {
    if let Some(fetch_site) = headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok())
    {
        if fetch_site.eq_ignore_ascii_case("cross-site") {
            return Err(ApiError::forbidden_with_error(
                "CORS_ORIGIN_DENIED",
                "Cross-site mutation requests are not allowed",
            ));
        }
    }

    if let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    {
        if !config.is_origin_allowed(origin) {
            return Err(ApiError::forbidden_with_error(
                "CORS_ORIGIN_DENIED",
                "Request origin is not allowed by the API CORS policy",
            ));
        }
    }

    Ok(())
}

fn csrf_required(headers: &HeaderMap) -> bool {
    headers.contains_key(header::COOKIE) || headers.contains_key(header::ORIGIN)
}

fn validate_csrf_token(headers: &HeaderMap) -> Result<(), ApiError> {
    let header_token = headers
        .get(CSRF_HEADER_NAME)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            ApiError::forbidden_with_error(
                "CSRF_TOKEN_MISSING",
                "CSRF token is required for browser mutation requests",
            )
        })?;

    let cookie_token = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookie| cookie_value(cookie, CSRF_COOKIE_NAME))
        .ok_or_else(|| {
            ApiError::forbidden_with_error(
                "CSRF_COOKIE_MISSING",
                "CSRF cookie is required for browser mutation requests",
            )
        })?;

    if header_token != cookie_token {
        return Err(ApiError::forbidden_with_error(
            "CSRF_TOKEN_INVALID",
            "CSRF token validation failed",
        ));
    }

    Ok(())
}

fn cookie_value(cookie_header: &str, name: &str) -> Option<String> {
    cookie_header.split(';').find_map(|part| {
        let mut pieces = part.trim().splitn(2, '=');
        let key = pieces.next()?.trim();
        let value = pieces.next()?.trim();
        (key == name).then(|| value.to_string())
    })
}

fn is_safe_method(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{middleware, routing::post, Router};
    use tower::ServiceExt;

    fn test_config() -> WebSecurityConfig {
        WebSecurityConfig {
            allowed_origins: ["http://localhost:3000".to_string()].into_iter().collect(),
            csrf_cookie_secure: false,
            csrf_same_site: SameSiteMode::Lax,
        }
    }

    #[tokio::test]
    async fn unsafe_browser_request_requires_matching_csrf_token() {
        let config = test_config();
        let app = Router::new()
            .route("/mutate", post(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                config,
                csrf_and_origin_middleware,
            ));

        let rejected = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/mutate")
                    .header(header::ORIGIN, "http://localhost:3000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(rejected.status(), StatusCode::FORBIDDEN);

        let accepted = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/mutate")
                    .header(header::ORIGIN, "http://localhost:3000")
                    .header(header::COOKIE, "sr_csrf=test-token")
                    .header(CSRF_HEADER_NAME, "test-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(accepted.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn hsts_header_is_added_to_responses() {
        let config = HttpsConfig::default();
        let app = Router::new()
            .route("/health", axum::routing::get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                config,
                https_enforcement_middleware,
            ));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response
            .headers()
            .get(header::STRICT_TRANSPORT_SECURITY)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("max-age="));
    }

    #[tokio::test]
    async fn insecure_request_is_redirected_when_forced() {
        let config = HttpsConfig {
            redirect_to_https: true,
            ..HttpsConfig::default()
        };
        let app = Router::new()
            .route("/data", axum::routing::get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                config,
                https_enforcement_middleware,
            ));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/data?x=1")
                    .header(header::HOST, "api.example.com")
                    .header("x-forwarded-proto", "http")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
        assert_eq!(
            response.headers().get(header::LOCATION).unwrap(),
            "https://api.example.com/data?x=1"
        );
    }

    #[tokio::test]
    async fn unsafe_cross_site_origin_is_rejected() {
        let error = validate_origin(
            &test_config(),
            &Request::builder()
                .method(Method::POST)
                .uri("/")
                .header(header::ORIGIN, "https://evil.example")
                .body(Body::empty())
                .unwrap()
                .headers()
                .clone(),
        )
        .unwrap_err();
        assert_eq!(error.into_response().status(), StatusCode::FORBIDDEN);
    }
}
