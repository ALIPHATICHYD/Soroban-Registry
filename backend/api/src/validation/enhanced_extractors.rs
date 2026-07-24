//! Enhanced request extraction with security logging and rate limiting
//!
//! This module provides middleware and utilities to integrate validation failure
//! logging and rate limiting into the request/response pipeline.

use axum::{
    body::Body,
    extract::{connect_info::ConnectInfo, MatchedPath},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;

use crate::security_log;

/// Middleware that tracks validation failures for rate limiting
///
/// Add this middleware to your router to enable validation failure rate limiting:
///
/// ```ignore
/// let app = Router::new()
///     // ... routes ...
///     .layer(middleware::from_fn(request_tracing::tracing_middleware))
///     .layer(middleware::from_fn(validation_failure_tracking_middleware))
/// ```
pub async fn validation_failure_tracking_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    matched_path: Option<MatchedPath>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let correlation_id = crate::request_tracing::get_or_create_request_id(&req);

    let response = next.run(req).await;

    // Log validation failures (400 responses)
    if response.status() == axum::http::StatusCode::BAD_REQUEST {
        let client_ip = addr.ip();
        let path = matched_path
            .as_ref()
            .map(|p| p.as_str())
            .unwrap_or("unknown");

        security_log::log_validation_failure(
            client_ip,
            "request",
            "Request validation failed",
            path,
            "POST",
            &correlation_id,
            1,
        );
    }

    response
}

/// A validation rule applied by `validate_request` middleware.
///
/// Each rule names a source (e.g. `"query.contract_id"`) and a validator closure
/// returning `Ok(())` or an error message.
pub struct ValidationRule {
    pub field: &'static str,
    pub validator: Box<dyn Fn(&Request<Body>) -> Result<(), String> + Send + Sync>,
}

impl ValidationRule {
    pub fn new(
        field: &'static str,
        validator: impl Fn(&Request<Body>) -> Result<(), String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            field,
            validator: Box::new(validator),
        }
    }
}

/// Middleware factory that applies a static list of `ValidationRule`s to every
/// request passing through the route it wraps.
///
/// Validation failures are collected and returned as a single 400 JSON response
/// in the standard `ApiError` envelope (see `docs/ERROR_CODES.md`).
///
/// # Example
///
/// ```ignore
/// use axum::{Router, middleware};
/// use crate::validation::enhanced_extractors::{validate_request, ValidationRule};
/// use crate::validation::validators::validate_contract_id;
///
/// let app = Router::new()
///     .route("/api/contracts/:id", get(handler))
///     .layer(middleware::from_fn(validate_request(vec![
///         ValidationRule::new("path.id", |req| {
///             let id = req.uri().path().split('/').last().unwrap_or("");
///             validate_contract_id(id)
///         }),
///     ])));
/// ```
pub fn validate_request(
    rules: Vec<ValidationRule>,
) -> impl Fn(
    Request<Body>,
    Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>>
       + Clone
       + Send
       + 'static {
    use std::sync::Arc;
    let rules = Arc::new(rules);

    move |req: Request<Body>, next: Next| {
        let rules = rules.clone();
        Box::pin(async move {
            let correlation_id = crate::request_tracing::get_or_create_request_id(&req);

            let mut errors = vec![];
            for rule in rules.iter() {
                if let Err(msg) = (rule.validator)(&req) {
                    errors.push(super::extractors::FieldError::new(rule.field, msg));
                }
            }

            if !errors.is_empty() {
                let _ = correlation_id; // tracing id is attached by ApiError::into_response
                return axum::response::IntoResponse::into_response(
                    super::extractors::ValidationError::new(errors),
                );
            }

            next.run(req).await
        })
    }
}

/// Development-only middleware that validates response bodies are non-empty
/// JSON when the response status is 2xx and Content-Type is application/json.
///
/// Only active when compiled in debug mode (`cfg(debug_assertions)`).
/// In release builds this middleware is a no-op pass-through.
pub async fn validate_response_schema_dev(req: Request<Body>, next: Next) -> Response {
    let response = next.run(req).await;

    #[cfg(debug_assertions)]
    {
        let status = response.status();
        let is_json = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false);

        if status.is_success() && is_json {
            let (parts, body) = response.into_parts();
            let bytes = match axum::body::to_bytes(body, 1024 * 1024).await {
                Ok(b) => b,
                Err(_) => {
                    tracing::warn!(
                        "[dev] validate_response_schema_dev: failed to buffer response body"
                    );
                    return Response::from_parts(parts, axum::body::Body::empty());
                }
            };

            if bytes.is_empty() {
                tracing::warn!(
                    status = status.as_u16(),
                    "[dev] Response has empty body for JSON content-type"
                );
            } else if serde_json::from_slice::<serde_json::Value>(&bytes).is_err() {
                tracing::warn!(
                    status = status.as_u16(),
                    "[dev] Response body is not valid JSON despite application/json content-type"
                );
            }

            return Response::from_parts(parts, axum::body::Body::from(bytes));
        }
    }

    response
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_correlation_id_generation() {
        let id = crate::request_tracing::generate_request_id();
        assert!(!id.is_empty());
        assert_eq!(id.len(), 36); // UUID v4 format
    }
}
