use axum::{
    http::{header::RETRY_AFTER, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{SecondsFormat, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use std::backtrace::Backtrace;

/// Standardized error types, payload normalization, and HTTP response handling for the API layer.
///
/// This module avoids specific external crate dependencies (e.g., `uuid::Uuid`) for request
/// tracking to minimize bloat, instead delegating correlation ID logic to the `request_tracing`
/// module which handles generation and lifecycle natively.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    UnprocessableEntity,
    PayloadTooLarge,
    RateLimited,
    InternalError,
}

/// Categorization of errors for monitoring and analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Input validation or user-caused errors
    Validation,
    /// Authentication / authorization failures
    Authentication,
    /// Requested resource not found
    NotFound,
    /// Resource conflict (duplicate, version mismatch)
    Conflict,
    /// Database connectivity or query errors
    Database,
    /// Errors from Stellar RPC calls
    StellarRpc,
    /// Errors from external service dependencies (S3, Elasticsearch, etc.)
    ExternalService,
    /// Rate limiting
    RateLimit,
    /// Internal / unexpected errors
    Internal,
    /// Network / I/O errors
    Network,
}

impl ErrorCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCategory::Validation => "validation",
            ErrorCategory::Authentication => "authentication",
            ErrorCategory::NotFound => "not_found",
            ErrorCategory::Conflict => "conflict",
            ErrorCategory::Database => "database",
            ErrorCategory::StellarRpc => "stellar_rpc",
            ErrorCategory::ExternalService => "external_service",
            ErrorCategory::RateLimit => "rate_limit",
            ErrorCategory::Internal => "internal",
            ErrorCategory::Network => "network",
        }
    }
}

impl ErrorCode {
    fn from_status(status: StatusCode) -> Self {
        match status {
            StatusCode::BAD_REQUEST => Self::BadRequest,
            StatusCode::UNAUTHORIZED => Self::Unauthorized,
            StatusCode::FORBIDDEN => Self::Forbidden,
            StatusCode::NOT_FOUND => Self::NotFound,
            StatusCode::CONFLICT => Self::Conflict,
            StatusCode::UNPROCESSABLE_ENTITY => Self::UnprocessableEntity,
            StatusCode::PAYLOAD_TOO_LARGE => Self::PayloadTooLarge,
            StatusCode::TOO_MANY_REQUESTS => Self::RateLimited,
            _ => Self::InternalError,
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    error_code: ErrorCode,
    code: String,
    message: String,
    details: Option<Value>,
    backtrace: Option<String>,
    category: ErrorCategory,
    /// Seconds until the caller may retry. When set, `into_response` attaches
    /// a `Retry-After` header — used by rate limiting and cooldown checks.
    retry_after_seconds: Option<u64>,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.error_code, self.message)
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    code: String,
    request_id: String,
    error_code: ErrorCode,
    message: String,
    details: Value,
    timestamp: String,
    correlation_id: String,
}

fn capture_backtrace() -> Option<String> {
    let backtrace = Backtrace::capture();
    if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
        Some(format!("{:#}", backtrace))
    } else {
        None
    }
}

impl ErrorCategory {
    fn from_status(status: StatusCode) -> Self {
        match status {
            StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => ErrorCategory::Validation,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ErrorCategory::Authentication,
            StatusCode::NOT_FOUND => ErrorCategory::NotFound,
            StatusCode::CONFLICT => ErrorCategory::Conflict,
            StatusCode::TOO_MANY_REQUESTS => ErrorCategory::RateLimit,
            StatusCode::PAYLOAD_TOO_LARGE => ErrorCategory::Validation,
            StatusCode::SERVICE_UNAVAILABLE => ErrorCategory::ExternalService,
            _ => ErrorCategory::Internal,
        }
    }
}

fn normalize_error_code(code: impl Into<String>) -> String {
    let raw = code.into();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "INTERNAL_ERROR".to_string();
    }

    let mut normalized = String::with_capacity(trimmed.len() + 8);
    for (idx, ch) in trimmed.chars().enumerate() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase()
                && idx > 0
                && !normalized.ends_with('_')
                && normalized
                    .chars()
                    .last()
                    .is_some_and(|prev| prev.is_ascii_lowercase())
            {
                normalized.push('_');
            }
            normalized.push(ch.to_ascii_uppercase());
        } else if !normalized.ends_with('_') {
            normalized.push('_');
        }
    }

    let normalized = normalized.trim_matches('_').to_string();
    if normalized.is_empty() {
        "INTERNAL_ERROR".to_string()
    } else {
        normalized
    }
}

impl ApiError {
    pub fn new(status: StatusCode, error: impl Into<String>, message: impl Into<String>) -> Self {
        let reason = error.into();
        let code = normalize_error_code(reason.clone());
        Self {
            status,
            error_code: ErrorCode::from_status(status),
            code,
            message: message.into(),
            details: if reason.is_empty() {
                None
            } else {
                Some(json!({ "reason": normalize_error_code(reason) }))
            },
            backtrace: capture_backtrace(),
            category: ErrorCategory::from_status(status),
            retry_after_seconds: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<Value>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Attach a `Retry-After: <seconds>` header to the response. Used by rate
    /// limiting and cooldown checks so callers know exactly when to retry.
    pub fn with_retry_after(mut self, seconds: u64) -> Self {
        self.retry_after_seconds = Some(seconds);
        self
    }

    pub fn bad_request(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, error, message)
    }

    pub fn bad_request_with(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, error, message)
    }

    pub fn bad_request_msg(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "BAD_REQUEST", message)
    }

    pub fn not_found(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, error, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error_code: ErrorCode::InternalError,
            code: "INTERNAL_ERROR".to_string(),
            message: message.into(),
            details: None,
            backtrace: capture_backtrace(),
            category: ErrorCategory::Internal,
            retry_after_seconds: None,
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "FORBIDDEN", message)
    }

    pub fn forbidden_with_error(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, error, message)
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED", message)
    }

    pub fn unprocessable(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, error, message)
    }

    pub fn conflict(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, error, message)
    }

    pub fn db_error(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error_code: ErrorCode::InternalError,
            code: "DATABASE_ERROR".to_string(),
            message: message.into(),
            details: None,
            backtrace: capture_backtrace(),
            category: ErrorCategory::Database,
            retry_after_seconds: None,
        }
    }

    pub fn internal_error(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error_code: ErrorCode::InternalError,
            code: normalize_error_code(error.into()),
            message: message.into(),
            details: None,
            backtrace: capture_backtrace(),
            category: ErrorCategory::Internal,
            retry_after_seconds: None,
        }
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "SERVICE_UNAVAILABLE",
            message,
        )
    }

    pub fn service_unavailable_with(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, error, message)
    }

    pub fn payload_too_large(message: impl Into<String>) -> Self {
        Self::new(StatusCode::PAYLOAD_TOO_LARGE, "PAYLOAD_TOO_LARGE", message)
    }

    pub fn category(&self) -> ErrorCategory {
        self.category
    }

    pub fn backtrace(&self) -> Option<&str> {
        self.backtrace.as_deref()
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let correlation_id = crate::request_tracing::current_request_id()
            .unwrap_or_else(crate::request_tracing::generate_request_id);
        let retry_after_seconds = self.retry_after_seconds;
        let details = self.details.unwrap_or_else(|| json!({}));
        let category_str = self.category.as_str();

        tracing::error!(
            request_id = %correlation_id,
            status = self.status.as_u16(),
            code = %self.code,
            error_code = ?self.error_code,
            category = %category_str,
            backtrace = %self.backtrace.as_deref().unwrap_or("none"),
            details = %details,
            message = %self.message,
            "api_error"
        );

        let payload = ErrorResponse {
            code: self.code,
            request_id: correlation_id.clone(),
            error_code: self.error_code,
            message: self.message,
            details,
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            correlation_id: correlation_id.clone(),
        };

        let mut response = (self.status, Json(payload)).into_response();
        crate::request_tracing::attach_request_id_headers(response.headers_mut(), &correlation_id);
        if let Some(seconds) = retry_after_seconds {
            if let Ok(value) = HeaderValue::from_str(&seconds.to_string()) {
                response.headers_mut().insert(RETRY_AFTER, value);
            } else {
                tracing::warn!("Failed to encode Retry-After header");
            }
        }
        response
    }
}

pub type ApiResult<T> = std::result::Result<T, ApiError>;
pub type AppError = ApiError;

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        tracing::error!(err = %e, category = "database", "database error");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error_code: ErrorCode::InternalError,
            code: "DATABASE_ERROR".to_string(),
            message: "Database error".to_string(),
            details: Some(json!({ "reason": e.to_string() })),
            backtrace: capture_backtrace(),
            category: ErrorCategory::Database,
            retry_after_seconds: None,
        }
    }
}

impl From<StatusCode> for ApiError {
    fn from(status: StatusCode) -> Self {
        Self::new(
            status,
            format!("{}", ErrorCode::from_status(status)),
            status.canonical_reason().unwrap_or("Unknown Error"),
        )
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        let backtrace = capture_backtrace();
        tracing::error!(err = %e, category = "internal", "internal_error");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error_code: ErrorCode::InternalError,
            code: "INTERNAL_ERROR".to_string(),
            message: e.to_string(),
            details: Some(json!({ "reason": format!("{:#}", e) })),
            backtrace,
            category: ErrorCategory::Internal,
            retry_after_seconds: None,
        }
    }
}

impl From<String> for ApiError {
    fn from(message: String) -> Self {
        Self::internal(message)
    }
}

impl From<&str> for ApiError {
    fn from(message: &str) -> Self {
        Self::internal(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn api_error_uses_standard_response_shape() {
        let response = ApiError::bad_request("INVALID_INPUT", "Invalid request payload")
            .with_details(json!({ "field": "name" }))
            .into_response();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let value: serde_json::Value =
            serde_json::from_slice(&body).expect("response body should be valid json");

        assert_eq!(value["code"], "INVALID_INPUT");
        assert_eq!(value["error_code"], "BAD_REQUEST");
        assert_eq!(value["message"], "Invalid request payload");
        assert_eq!(value["details"]["field"], "name");
        assert!(value["request_id"].is_string());
        assert!(value["timestamp"].is_string());
    }

    #[tokio::test]
    async fn with_retry_after_attaches_header_only_when_set() {
        let without = ApiError::rate_limited("Too many requests").into_response();
        assert!(!without.headers().contains_key(RETRY_AFTER));

        let with = ApiError::rate_limited("Cooldown active")
            .with_retry_after(42)
            .into_response();
        assert_eq!(
            with.headers().get(RETRY_AFTER).and_then(|v| v.to_str().ok()),
            Some("42")
        );
    }

    #[tokio::test]
    async fn rate_limited_errors_use_rate_limited_code() {
        let response = ApiError::rate_limited("Too many requests").into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let value: serde_json::Value =
            serde_json::from_slice(&body).expect("response body should be valid json");

        assert_eq!(value["error_code"], "RATE_LIMITED");
    }
}
