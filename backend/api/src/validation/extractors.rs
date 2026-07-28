//! Custom Axum extractors for validated input
//!
//! This module provides `ValidatedJson<T>` - a drop-in replacement for `Json<T>`
//! that automatically sanitizes and validates incoming JSON payloads.

use axum::{
    async_trait,
    extract::{FromRequest, FromRequestParts, Request},
    http::{request::Parts, StatusCode},
    Json,
};
use chrono::{SecondsFormat, Utc};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::json;

/// A field-level validation error
#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl FieldError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Internal helper preserved for unit tests that assert the legacy intermediate
/// shape used to be produced for validation failures. New code MUST NOT rely on
/// this struct — HTTP responses are produced by [`ApiError`] (see
/// `docs/ERROR_CODES.md`).
#[derive(Debug, Serialize)]
pub struct ValidationErrorResponse {
    pub error_code: String,
    pub message: String,
    pub details: serde_json::Value,
    pub timestamp: String,
    pub correlation_id: String,
}

impl ValidationErrorResponse {
    pub fn new(errors: Vec<FieldError>, correlation_id: String) -> Self {
        let error_summary = validation_summary(&errors);
        Self {
            error_code: "BAD_REQUEST".to_string(),
            message: error_summary,
            details: json!({
                "reason": "VALIDATION_ERROR",
                "field_errors": errors,
                "correlation_id": correlation_id
            }),
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            correlation_id,
        }
    }
}

fn validation_summary(errors: &[FieldError]) -> String {
    if errors.len() == 1 {
        format!("Validation failed for field '{}'", errors[0].field)
    } else {
        format!("Validation failed for {} fields", errors.len())
    }
}

/// Validation error that converts to an HTTP response.
///
/// Renders as a standard [`crate::error::ApiError`] envelope (HTTP 400,
/// `code = "VALIDATION_ERROR"`) so every error response in the API shares the
/// same JSON shape documented in `docs/ERROR_CODES.md`.
#[derive(Debug)]
pub struct ValidationError {
    pub errors: Vec<FieldError>,
}

impl ValidationError {
    pub fn new(errors: Vec<FieldError>) -> Self {
        Self { errors }
    }

    pub fn single(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            errors: vec![FieldError::new(field, message)],
        }
    }
}

impl From<ValidationError> for crate::error::ApiError {
    fn from(err: ValidationError) -> Self {
        let summary = validation_summary(&err.errors);
        crate::error::ApiError::bad_request_with("VALIDATION_ERROR", summary)
            .with_details(json!({ "field_errors": err.errors }))
    }
}

impl axum::response::IntoResponse for ValidationError {
    fn into_response(self) -> axum::response::Response {
        crate::error::ApiError::from(self).into_response()
    }
}

/// Trait for types that can be validated and sanitized
///
/// Implement this trait for your request types to enable automatic
/// validation when using `ValidatedJson<T>`.
pub trait Validatable: Sized {
    /// Sanitize the data in-place (trim whitespace, strip HTML, etc.)
    fn sanitize(&mut self);

    /// Validate the data and return any field errors
    fn validate(&self) -> Result<(), Vec<FieldError>>;
}

/// Custom JSON extractor that validates and sanitizes input
///
/// Use this instead of `Json<T>` to automatically:
/// 1. Parse JSON from the request body
/// 2. Sanitize all string fields (trim, strip HTML, normalize)
/// 3. Validate fields against defined rules
/// 4. Log validation failures for security monitoring
/// 5. Return detailed 400 errors for validation failures
///
/// # Example
///
/// ```ignore
/// use crate::validation::{ValidatedJson, Validatable, FieldError};
///
/// pub async fn create_item(
///     ValidatedJson(req): ValidatedJson<CreateRequest>,
/// ) -> impl IntoResponse {
///     // req is already sanitized and validated
///     // ...
/// }
/// ```
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validatable + Send,
    S: Send + Sync,
{
    type Rejection = ValidationError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // Extract client IP and correlation ID for logging
        let client_ip = req
            .extensions()
            .get::<std::net::SocketAddr>()
            .map(|addr| addr.ip())
            .unwrap_or_else(|| std::net::IpAddr::from([127, 0, 0, 1]));

        let correlation_id = crate::request_tracing::get_or_create_request_id(&req);

        let path = req
            .extensions()
            .get::<axum::extract::MatchedPath>()
            .map(|p| p.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Step 1: Parse JSON
        let Json(mut data) = Json::<T>::from_request(req, state).await.map_err(|err| {
            // Convert JSON parsing errors to validation errors
            let message = match err {
                axum::extract::rejection::JsonRejection::JsonDataError(e) => {
                    format!("Invalid JSON data: {}", e.body_text())
                }
                axum::extract::rejection::JsonRejection::JsonSyntaxError(e) => {
                    format!("JSON syntax error: {}", e.body_text())
                }
                axum::extract::rejection::JsonRejection::MissingJsonContentType(_) => {
                    "Content-Type must be application/json".to_string()
                }
                axum::extract::rejection::JsonRejection::BytesRejection(_) => {
                    "Failed to read request body".to_string()
                }
                _ => "Invalid JSON payload".to_string(),
            };

            // Log JSON parsing error
            crate::security_log::log_validation_failure(
                client_ip,
                "body",
                &message,
                &path,
                "POST",
                &correlation_id,
                1,
            );

            ValidationError::single("body", message)
        })?;

        // Step 2: Sanitize the data
        data.sanitize();

        // Step 3: Validate the data
        data.validate().map_err(|errors| {
            // Log validation failures with field details
            for error in &errors {
                crate::security_log::log_validation_failure(
                    client_ip,
                    &error.field,
                    &error.message,
                    &path,
                    "POST",
                    &correlation_id,
                    1,
                );
            }
            ValidationError::new(errors)
        })?;

        Ok(ValidatedJson(data))
    }
}

// Implement Deref for ergonomic access
impl<T> std::ops::Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for ValidatedJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Custom Path extractor that validates path parameters implementing `Validatable`.
pub struct ValidatedPath<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for ValidatedPath<T>
where
    T: serde::de::DeserializeOwned + Validatable + Send,
    S: Send + Sync,
{
    type Rejection = ValidationError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let correlation_id = parts
            .extensions
            .get::<crate::request_tracing::RequestId>()
            .map(|r| r.0.clone())
            .unwrap_or_else(crate::request_tracing::generate_request_id);
        let path = parts
            .extensions
            .get::<axum::extract::MatchedPath>()
            .map(|p| p.as_str())
            .unwrap_or("unknown")
            .to_string();

        let axum::extract::Path(mut data) =
            axum::extract::Path::<T>::from_request_parts(parts, state)
                .await
                .map_err(|err| {
                    let message = format!("Invalid path parameters: {}", err);
                    crate::security_log::log_validation_failure(
                        std::net::IpAddr::from([127, 0, 0, 1]),
                        "path",
                        &message,
                        &path,
                        parts.method.as_str(),
                        &correlation_id,
                        1,
                    );
                    ValidationError::single("path", message)
                })?;

        data.sanitize();
        data.validate().map_err(|errors| {
            for error in &errors {
                crate::security_log::log_validation_failure(
                    std::net::IpAddr::from([127, 0, 0, 1]),
                    &error.field,
                    &error.message,
                    &path,
                    parts.method.as_str(),
                    &correlation_id,
                    1,
                );
            }
            ValidationError::new(errors)
        })?;

        Ok(ValidatedPath(data))
    }
}

impl<T> std::ops::Deref for ValidatedPath<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Custom Query extractor that validates query parameters implementing `Validatable`.
pub struct ValidatedQuery<T>(pub T);

#[async_trait]
impl<S, T> FromRequestParts<S> for ValidatedQuery<T>
where
    T: serde::de::DeserializeOwned + Validatable + Send,
    S: Send + Sync,
{
    type Rejection = ValidationError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let correlation_id_q = parts
            .extensions
            .get::<crate::request_tracing::RequestId>()
            .map(|r| r.0.clone())
            .unwrap_or_else(crate::request_tracing::generate_request_id);
        let path_q = parts
            .extensions
            .get::<axum::extract::MatchedPath>()
            .map(|p| p.as_str())
            .unwrap_or("unknown")
            .to_string();

        let axum::extract::Query(mut data) =
            axum::extract::Query::<T>::from_request_parts(parts, state)
                .await
                .map_err(|err| {
                    let message = format!("Invalid query parameters: {}", err);
                    crate::security_log::log_validation_failure(
                        std::net::IpAddr::from([127, 0, 0, 1]),
                        "query",
                        &message,
                        &path_q,
                        parts.method.as_str(),
                        &correlation_id_q,
                        1,
                    );
                    ValidationError::single("query", message)
                })?;

        data.sanitize();
        data.validate().map_err(|errors| {
            for error in &errors {
                crate::security_log::log_validation_failure(
                    std::net::IpAddr::from([127, 0, 0, 1]),
                    &error.field,
                    &error.message,
                    &path_q,
                    parts.method.as_str(),
                    &correlation_id_q,
                    1,
                );
            }
            ValidationError::new(errors)
        })?;

        Ok(ValidatedQuery(data))
    }
}

impl<T> std::ops::Deref for ValidatedQuery<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Builder for accumulating validation errors
#[derive(Debug, Default)]
pub struct ValidationBuilder {
    errors: Vec<FieldError>,
}

impl ValidationBuilder {
    pub fn new() -> Self {
        Self { errors: vec![] }
    }

    /// Add an error if the result is Err
    pub fn check<F>(&mut self, field: &str, validator: F) -> &mut Self
    where
        F: FnOnce() -> Result<(), String>,
    {
        if let Err(message) = validator() {
            self.errors.push(FieldError::new(field, message));
        }
        self
    }

    /// Add an error directly
    pub fn add_error(&mut self, field: impl Into<String>, message: impl Into<String>) -> &mut Self {
        self.errors.push(FieldError::new(field, message));
        self
    }

    /// Add error if condition is true
    pub fn check_condition(
        &mut self,
        condition: bool,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> &mut Self {
        if condition {
            self.errors.push(FieldError::new(field, message));
        }
        self
    }

    /// Finish building and return Result
    pub fn build(self) -> Result<(), Vec<FieldError>> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
        }
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get current error count
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_error() {
        let error = FieldError::new("name", "is required");
        assert_eq!(error.field, "name");
        assert_eq!(error.message, "is required");
    }

    #[test]
    fn test_validation_builder() {
        let mut builder = ValidationBuilder::new();

        builder
            .check("name", || Err("is required".to_string()))
            .check("email", || Ok(()))
            .check_condition(true, "age", "must be positive");

        assert!(builder.has_errors());
        assert_eq!(builder.error_count(), 2);

        let result = builder.build();
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].field, "name");
        assert_eq!(errors[1].field, "age");
    }

    #[test]
    fn test_validation_error_response() {
        let errors = vec![
            FieldError::new("contract_id", "is required"),
            FieldError::new("name", "must be at least 1 character"),
        ];

        let response = ValidationErrorResponse::new(errors, "test-correlation-id".to_string());

        assert_eq!(response.error_code, "BAD_REQUEST");
        assert_eq!(response.details["reason"], "VALIDATION_ERROR");
        assert_eq!(
            response.details["field_errors"].as_array().unwrap().len(),
            2
        );
        assert!(response.message.contains("2 fields"));
    }

    #[test]
    fn test_single_error_response() {
        let errors = vec![FieldError::new("name", "is required")];
        let response = ValidationErrorResponse::new(errors, "test-correlation-id".to_string());

        assert!(response.message.contains("field 'name'"));
    }

    #[tokio::test]
    async fn validation_error_renders_standard_envelope() {
        use axum::response::IntoResponse;

        let err = ValidationError::new(vec![
            FieldError::new("contract_id", "is required"),
            FieldError::new("name", "must be at least 1 character"),
        ]);
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body readable");
        let value: serde_json::Value = serde_json::from_slice(&body).expect("body is json");

        // Standard ApiError envelope (docs/ERROR_CODES.md)
        assert_eq!(value["code"], "VALIDATION_ERROR");
        assert_eq!(value["error_code"], "BAD_REQUEST");
        assert!(value["request_id"].is_string());
        assert!(value["correlation_id"].is_string());
        assert!(value["timestamp"].is_string());
        assert!(value["message"].as_str().unwrap().contains("2 fields"));
        assert_eq!(
            value["details"]["field_errors"].as_array().unwrap().len(),
            2
        );
    }
}
