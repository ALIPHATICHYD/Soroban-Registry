//! Structured request tracing middleware.
//!
//! Every incoming HTTP request gets a unique UUID (`X-Request-ID`), and a
//! JSON-structured log line is emitted after the response is sent.
//!
//! Health-check endpoints are intentionally skipped so they don't pollute
//! the log stream.
//!
//! Log fields:
//!   timestamp, request_id, method, path, status, duration_ms, user_ip

use axum::{
    body::to_bytes,
    body::Body,
    extract::ConnectInfo,
    http::{HeaderMap, HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use opentelemetry::global;
use opentelemetry::propagation::{Extractor, Injector};
use opentelemetry::trace::TraceContextExt;
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::Resource;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::time::Instant;
use tracing::Instrument;
use tracing::Level;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use uuid::Uuid;

/// Paths that should never be logged (health checks, readiness probes, etc.)
const SKIP_LOG_PATHS: &[&str] = &["/health", "/healthz", "/ready", "/ping", "/metrics"];
const DEFAULT_REQUEST_BODY_LIMIT: usize = 16 * 1024;
const MAX_CAPTURE_BODY_BYTES: usize = 256 * 1024;
const REDACTED: &str = "[REDACTED]";

static ENDPOINT_LOG_LEVELS: Lazy<Vec<(String, Level)>> =
    Lazy::new(|| parse_endpoint_log_levels(std::env::var("REQUEST_LOG_LEVELS").ok()));

static REQUEST_BODY_LIMIT: Lazy<usize> = Lazy::new(|| {
    std::env::var("REQUEST_LOG_BODY_LIMIT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_REQUEST_BODY_LIMIT)
});

static LOG_GUARD: OnceCell<tracing_appender::non_blocking::WorkerGuard> = OnceCell::new();

/// The response header name carrying the request ID back to the caller.
pub static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");
pub static X_CORRELATION_ID: HeaderName = HeaderName::from_static("x-correlation-id");

tokio::task_local! {
    static CURRENT_REQUEST_ID: String;
}

/// Axum middleware: attach a request ID, log the completed request as JSON,
/// and add the `X-Request-ID` header to the response.
pub async fn tracing_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let (parts, body) = req.into_parts();
    let mut req_body_bytes = Vec::new();
    if let Ok(bytes) = to_bytes(body, MAX_CAPTURE_BODY_BYTES).await {
        req_body_bytes = bytes.to_vec();
    }

    let mut req = Request::from_parts(parts, Body::from(req_body_bytes.clone()));
    let request_id = request_id_from_headers(req.headers()).unwrap_or_else(generate_request_id);
    let method = req.method().to_string();
    let path = req.uri().path().to_owned();
    let query = req.uri().query().unwrap_or_default().to_string();
    let user_ip = addr.ip().to_string();
    let request_headers = sanitize_headers(req.headers());
    let request_body = sanitize_body(
        &req_body_bytes,
        req.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        *REQUEST_BODY_LIMIT,
    );
    let user_info = extract_user_identity(req.headers());

    // Inject the request ID into extensions so handlers / DB layers can read it
    req.extensions_mut().insert(RequestId(request_id.clone()));

    let start = Instant::now();
    let parent_context = extract_remote_context(req.headers());
    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id,
        method = %method,
        path = %path,
        user_ip = %user_ip,
        user_id = tracing::field::Empty,
        contract_id = tracing::field::Empty,
        organization_id = tracing::field::Empty,
    );
    span.set_parent(parent_context);
    let mut response = CURRENT_REQUEST_ID
        .scope(request_id.clone(), next.run(req).instrument(span.clone()))
        .await;
    let duration_ms = start.elapsed().as_millis() as u64;
    let response_size = response_body_size(&response);

    attach_request_id_headers(response.headers_mut(), &request_id);

    // Skip noisy health-check paths
    if SKIP_LOG_PATHS.iter().any(|p| path.starts_with(p)) {
        return response;
    }

    let status = response.status().as_u16();
    let level = endpoint_log_level(&path);

    log_request(
        level,
        json!({
            "request_id": request_id,
            "method": method,
            "path": path,
            "query": query,
            "status": status,
            "duration_ms": duration_ms,
            "response_size": response_size,
            "user_info": user_info,
            "request_headers": request_headers,
            "request_body": request_body,
            "user_ip": user_ip,
        }),
    );

    response
}

fn endpoint_log_level(path: &str) -> Level {
    let mut selected = Level::INFO;
    let mut selected_len = 0usize;

    for (prefix, level) in ENDPOINT_LOG_LEVELS.iter() {
        if prefix == "*" {
            if selected_len == 0 {
                selected = *level;
            }
            continue;
        }

        if path.starts_with(prefix) && prefix.len() >= selected_len {
            selected = *level;
            selected_len = prefix.len();
        }
    }

    selected
}

fn parse_endpoint_log_levels(value: Option<String>) -> Vec<(String, Level)> {
    let Some(raw) = value else {
        return vec![("*".to_string(), Level::INFO)];
    };

    let mut pairs = Vec::new();
    for segment in raw.split(',') {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut split = trimmed.splitn(2, '=');
        let prefix = split.next().unwrap_or("*").trim();
        let level_str = split.next().unwrap_or("info").trim().to_ascii_lowercase();
        let level = match level_str.as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        };

        pairs.push((
            if prefix.is_empty() { "*" } else { prefix }.to_string(),
            level,
        ));
    }

    if pairs.is_empty() {
        vec![("*".to_string(), Level::INFO)]
    } else {
        pairs
    }
}

fn log_request(level: Level, payload: Value) {
    match level {
        Level::TRACE => tracing::trace!(request = %payload, "request"),
        Level::DEBUG => tracing::debug!(request = %payload, "request"),
        Level::WARN => tracing::warn!(request = %payload, "request"),
        Level::ERROR => tracing::error!(request = %payload, "request"),
        Level::INFO => tracing::info!(request = %payload, "request"),
    }
}

fn response_body_size(response: &Response) -> usize {
    response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
}

fn extract_user_identity(headers: &HeaderMap) -> Value {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .map(mask_secret);
    let authorization = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(mask_authorization);

    json!({
        "api_key": api_key,
        "authorization": authorization,
    })
}

fn sanitize_headers(headers: &HeaderMap) -> Value {
    let mut out = serde_json::Map::new();
    for (name, value) in headers.iter() {
        let key = name.as_str().to_ascii_lowercase();
        let value = value.to_str().unwrap_or_default();

        let sanitized = if is_sensitive_key(&key) {
            REDACTED.to_string()
        } else {
            truncate(value, 256)
        };

        out.insert(key, Value::String(sanitized));
    }
    Value::Object(out)
}

fn sanitize_body(bytes: &[u8], content_type: Option<&str>, limit: usize) -> Value {
    if bytes.is_empty() {
        return json!({ "kind": "empty" });
    }

    if content_type
        .unwrap_or_default()
        .contains("application/json")
    {
        if let Ok(mut value) = serde_json::from_slice::<Value>(bytes) {
            redact_value(&mut value);
            return json!({
                "kind": "json",
                "size": bytes.len(),
                "body": value,
            });
        }
    }

    let text = String::from_utf8_lossy(bytes);
    json!({
        "kind": "text",
        "size": bytes.len(),
        "body": truncate(&text, limit),
    })
}

fn redact_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if is_sensitive_key(key) {
                    *val = Value::String(REDACTED.to_string());
                } else {
                    redact_value(val);
                }
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                redact_value(item);
            }
        }
        _ => {}
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let lowered = key.to_ascii_lowercase();
    lowered.contains("password")
        || lowered.contains("token")
        || lowered.contains("secret")
        || lowered.contains("private_key")
        || lowered.contains("api_key")
        || lowered.contains("authorization")
}

fn truncate(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        return value.to_string();
    }
    format!("{}...(truncated)", &value[..limit])
}

fn mask_secret(value: &str) -> String {
    if value.len() <= 8 {
        return REDACTED.to_string();
    }
    format!("{}...{}", &value[..4], &value[value.len() - 4..])
}

fn mask_authorization(value: &str) -> String {
    let mut split = value.split_whitespace();
    let scheme = split.next().unwrap_or("Bearer");
    let token = split.next().unwrap_or_default();
    if token.is_empty() {
        return REDACTED.to_string();
    }
    format!("{} {}", scheme, mask_secret(token))
}

pub fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn current_request_id() -> Option<String> {
    CURRENT_REQUEST_ID
        .try_with(|request_id| request_id.clone())
        .ok()
}

pub fn inject_business_context(
    user_id: Option<&str>,
    contract_id: Option<&str>,
    org_id: Option<&str>,
) {
    let span = tracing::Span::current();
    if let Some(uid) = user_id {
        span.record("user_id", &uid);
    }
    if let Some(cid) = contract_id {
        span.record("contract_id", &cid);
    }
    if let Some(oid) = org_id {
        span.record("organization_id", &oid);
    }
}

pub fn request_id_from_headers(headers: &HeaderMap) -> Option<String> {
    [X_REQUEST_ID.as_str(), X_CORRELATION_ID.as_str()]
        .iter()
        .find_map(|header_name| {
            headers
                .get(*header_name)
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
}

pub fn request_id_from_request<B>(request: &Request<B>) -> Option<String> {
    request
        .extensions()
        .get::<RequestId>()
        .map(|request_id| request_id.0.clone())
        .or_else(|| request_id_from_headers(request.headers()))
}

pub fn get_or_create_request_id<B>(request: &Request<B>) -> String {
    request_id_from_request(request)
        .or_else(current_request_id)
        .unwrap_or_else(generate_request_id)
}

pub fn attach_request_id_headers(headers: &mut HeaderMap, request_id: &str) {
    if let Ok(value) = HeaderValue::from_str(request_id) {
        headers.insert(X_REQUEST_ID.clone(), value.clone());
        headers.insert(X_CORRELATION_ID.clone(), value);
    }
}

pub fn inject_current_trace_context(headers: &mut reqwest::header::HeaderMap) {
    let context = opentelemetry::Context::current();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&context, &mut ReqwestHeaderInjector(headers));
    });
}

fn extract_remote_context(headers: &HeaderMap) -> opentelemetry::Context {
    global::get_text_map_propagator(|propagator| propagator.extract(&AxumHeaderExtractor(headers)))
}

struct AxumHeaderExtractor<'a>(&'a HeaderMap);

impl Extractor for AxumHeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|value| value.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|key| key.as_str()).collect()
    }
}

struct ReqwestHeaderInjector<'a>(&'a mut reqwest::header::HeaderMap);

impl Injector for ReqwestHeaderInjector<'_> {
    fn set(&mut self, key: &str, value: String) {
        if let (Ok(header_name), Ok(header_value)) = (
            reqwest::header::HeaderName::from_bytes(key.as_bytes()),
            reqwest::header::HeaderValue::from_str(&value),
        ) {
            self.0.insert(header_name, header_value);
        }
    }
}

// ── Request ID extractor ──────────────────────────────────────────────────────

/// A newtype wrapper stored in request extensions so downstream code can
/// cheaply retrieve the current request ID without re-parsing headers.
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl RequestId {
    /// Retrieve the request ID from Axum request extensions, if present.
    pub fn from_request(req: &Request<Body>) -> Option<&str> {
        req.extensions().get::<RequestId>().map(|r| r.0.as_str())
    }
}

// ── JSON tracing subscriber initialiser ──────────────────────────────────────

/// Initialise `tracing-subscriber` with a JSON formatter suitable for
/// ELK / Splunk / Datadog ingestion.
///
/// Call this **once** at application startup, replacing the plain-text
/// subscriber currently set up in `main.rs`.
///
/// Log rotation (daily, 7-day retention) is handled by the deployment
/// environment (e.g. logrotate, Docker log driver, or a dedicated log
/// shipper). The subscriber itself writes to stdout so the runtime can
/// redirect / rotate as needed.
pub fn init_json_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "soroban-registry-api".to_string());
    let otlp_endpoint = std::env::var("OTLP_ENDPOINT")
        .or_else(|_| std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT"))
        .ok();

    global::set_text_map_propagator(TraceContextPropagator::new());

    // Per-layer filtering (issue #887): the user-facing fmt/OTEL layers keep the
    // configured RUST_LOG directives, while the sqlx query-capture layer attaches
    // its own filter so it always sees every query regardless of RUST_LOG and
    // without spamming the console. EnvFilter is not Clone, so we resolve the
    // directive string once and build a fresh filter per layer.
    let filter_directives =
        std::env::var("RUST_LOG").unwrap_or_else(|_| "api=info,tower_http=info".to_string());
    let make_env = || tracing_subscriber::EnvFilter::new(filter_directives.clone());

    let writer = if let Ok(log_dir) = std::env::var("API_LOG_DIR") {
        let file_appender = tracing_appender::rolling::daily(log_dir, "api.log");
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        let _ = LOG_GUARD.set(guard);
        tracing_subscriber::fmt::writer::BoxMakeWriter::new(non_blocking)
    } else {
        tracing_subscriber::fmt::writer::BoxMakeWriter::new(std::io::stdout)
    };

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_writer(writer)
        .with_filter(make_env());

    if let Some(endpoint) = otlp_endpoint {
        // Retrieve sampling rate from env (default to 1.0 which is 100%)
        let sample_rate = std::env::var("OTEL_TRACES_SAMPLER_ARG")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(1.0);

        let sampler = opentelemetry_sdk::trace::Sampler::ParentBased(Box::new(
            opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(sample_rate),
        ));

        let trace_config = opentelemetry_sdk::trace::Config::default()
            .with_sampler(sampler)
            .with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                service_name.clone(),
            )]));

        match opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_trace_config(trace_config)
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .http()
                    .with_endpoint(endpoint),
            )
            .install_batch(opentelemetry_sdk::runtime::Tokio)
        {
            Ok(provider) => {
                let tracer = provider.tracer(service_name);
                tracing_subscriber::registry()
                    .with(fmt_layer)
                    .with(
                        tracing_opentelemetry::layer()
                            .with_tracer(tracer)
                            .with_filter(make_env()),
                    )
                    .with(crate::query_analysis::capture_layer())
                    .init();
                return;
            }
            Err(err) => {
                tracing::warn!(error = %err, "Failed to initialize OTLP exporter; falling back to log-only tracing");
            }
        }
    }

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(crate::query_analysis::capture_layer())
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        extract::ConnectInfo,
        http::{Request, StatusCode},
        middleware,
        response::IntoResponse,
        routing::get,
        Json, Router,
    };
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use tower::ServiceExt;

    async fn ok_handler(req: Request<Body>) -> impl IntoResponse {
        let request_id = request_id_from_request(&req).unwrap_or_default();
        Json(serde_json::json!({ "request_id": request_id }))
    }

    async fn error_handler() -> crate::error::ApiResult<Json<serde_json::Value>> {
        Err(crate::error::ApiError::bad_request(
            "BadRequest",
            "broken request",
        ))
    }

    fn app() -> Router {
        Router::new()
            .route("/ok", get(ok_handler))
            .route("/error", get(error_handler))
            .layer(middleware::from_fn(tracing_middleware))
    }

    async fn call(app: Router, request: Request<Body>) -> axum::response::Response {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4000);
        let mut request = request;
        request.extensions_mut().insert(ConnectInfo(addr));
        app.oneshot(request).await.unwrap()
    }

    #[tokio::test]
    async fn preserves_incoming_request_id_header() {
        let response = call(
            app(),
            Request::builder()
                .uri("/ok")
                .method("GET")
                .header(X_REQUEST_ID.as_str(), "req-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(X_REQUEST_ID.as_str())
                .and_then(|v| v.to_str().ok()),
            Some("req-123")
        );
    }

    #[tokio::test]
    async fn generates_request_id_when_missing() {
        let response = call(
            app(),
            Request::builder()
                .uri("/ok")
                .method("GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        let request_id = response
            .headers()
            .get(X_REQUEST_ID.as_str())
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();

        assert_eq!(request_id.len(), 36);
        assert_eq!(
            response
                .headers()
                .get(X_CORRELATION_ID.as_str())
                .and_then(|v| v.to_str().ok()),
            Some(request_id.as_str())
        );
    }

    #[tokio::test]
    async fn error_responses_reuse_request_id() {
        let response = call(
            app(),
            Request::builder()
                .uri("/error")
                .method("GET")
                .header(X_REQUEST_ID.as_str(), "req-error-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response
                .headers()
                .get(X_REQUEST_ID.as_str())
                .and_then(|v| v.to_str().ok()),
            Some("req-error-1")
        );

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(json["correlation_id"], "req-error-1");
    }
}
