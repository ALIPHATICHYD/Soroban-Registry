use serde::Serialize;
use std::backtrace::Backtrace;
use std::fmt;

/// Standardized error codes for machine-readable error identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
    DatabaseError,
    ServiceUnavailable,
    VerificationFailed,
    InvalidInput,
    ExternalServiceError,
    NetworkError,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::BadRequest => "BAD_REQUEST",
            ErrorCode::Unauthorized => "UNAUTHORIZED",
            ErrorCode::Forbidden => "FORBIDDEN",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::Conflict => "CONFLICT",
            ErrorCode::UnprocessableEntity => "UNPROCESSABLE_ENTITY",
            ErrorCode::PayloadTooLarge => "PAYLOAD_TOO_LARGE",
            ErrorCode::RateLimited => "RATE_LIMITED",
            ErrorCode::InternalError => "INTERNAL_ERROR",
            ErrorCode::DatabaseError => "DATABASE_ERROR",
            ErrorCode::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            ErrorCode::VerificationFailed => "VERIFICATION_FAILED",
            ErrorCode::InvalidInput => "INVALID_INPUT",
            ErrorCode::ExternalServiceError => "EXTERNAL_SERVICE_ERROR",
            ErrorCode::NetworkError => "NETWORK_ERROR",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Categorization of registry errors for monitoring and analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    Database,
    NotFound,
    InvalidInput,
    VerificationFailed,
    StellarRpc,
    Internal,
    S3,
    Network,
    Authentication,
    Conflict,
    ExternalService,
    RateLimit,
    Validation,
}

impl ErrorCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCategory::Database => "database",
            ErrorCategory::NotFound => "not_found",
            ErrorCategory::InvalidInput => "invalid_input",
            ErrorCategory::VerificationFailed => "verification_failed",
            ErrorCategory::StellarRpc => "stellar_rpc",
            ErrorCategory::Internal => "internal",
            ErrorCategory::S3 => "s3",
            ErrorCategory::Network => "network",
            ErrorCategory::Authentication => "authentication",
            ErrorCategory::Conflict => "conflict",
            ErrorCategory::ExternalService => "external_service",
            ErrorCategory::RateLimit => "rate_limit",
            ErrorCategory::Validation => "validation",
        }
    }
}

/// Custom error types for the registry with structured context.
#[derive(Debug)]
pub struct RegistryError {
    kind: RegistryErrorKind,
    backtrace: Option<String>,
    category: ErrorCategory,
    error_code: ErrorCode,
    source_module: Option<String>,
}

#[derive(Debug)]
enum RegistryErrorKind {
    Database(sqlx::Error),
    NotFound(String),
    InvalidInput(String),
    VerificationFailed(String),
    StellarRpc(String),
    Internal(String),
    S3(String),
    Io(std::io::Error),
    Serde(serde_json::Error),
    Anyhow(anyhow::Error),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.error_code,
            self.category.as_str(),
            self.kind
        )
    }
}

impl fmt::Display for RegistryErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryErrorKind::Database(e) => write!(f, "Database error: {}", e),
            RegistryErrorKind::NotFound(msg) => write!(f, "Not found: {}", msg),
            RegistryErrorKind::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            RegistryErrorKind::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            RegistryErrorKind::StellarRpc(msg) => write!(f, "Stellar RPC error: {}", msg),
            RegistryErrorKind::Internal(msg) => write!(f, "Internal error: {}", msg),
            RegistryErrorKind::S3(msg) => write!(f, "S3 error: {}", msg),
            RegistryErrorKind::Io(e) => write!(f, "IO error: {}", e),
            RegistryErrorKind::Serde(e) => write!(f, "Serialization error: {}", e),
            RegistryErrorKind::Anyhow(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for RegistryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            RegistryErrorKind::Database(e) => Some(e),
            RegistryErrorKind::Io(e) => Some(e),
            RegistryErrorKind::Serde(e) => Some(e),
            _ => None,
        }
    }
}

impl RegistryError {
    fn new(kind: RegistryErrorKind, category: ErrorCategory) -> Self {
        let error_code = Self::category_to_code(category);
        let backtrace = capture_backtrace();
        Self {
            kind,
            backtrace,
            category,
            error_code,
            source_module: None,
        }
    }

    fn category_to_code(category: ErrorCategory) -> ErrorCode {
        match category {
            ErrorCategory::Database => ErrorCode::DatabaseError,
            ErrorCategory::NotFound => ErrorCode::NotFound,
            ErrorCategory::InvalidInput => ErrorCode::InvalidInput,
            ErrorCategory::VerificationFailed => ErrorCode::VerificationFailed,
            ErrorCategory::StellarRpc => ErrorCode::ExternalServiceError,
            ErrorCategory::Internal => ErrorCode::InternalError,
            ErrorCategory::S3 => ErrorCode::ExternalServiceError,
            ErrorCategory::Network => ErrorCode::NetworkError,
            ErrorCategory::Authentication => ErrorCode::Unauthorized,
            ErrorCategory::Conflict => ErrorCode::Conflict,
            ErrorCategory::ExternalService => ErrorCode::ExternalServiceError,
            ErrorCategory::RateLimit => ErrorCode::RateLimited,
            ErrorCategory::Validation => ErrorCode::BadRequest,
        }
    }

    pub fn category(&self) -> ErrorCategory {
        self.category
    }

    pub fn error_code(&self) -> ErrorCode {
        self.error_code
    }

    pub fn backtrace(&self) -> Option<&str> {
        self.backtrace.as_deref()
    }

    pub fn source_module(&self) -> Option<&str> {
        self.source_module.as_deref()
    }

    pub fn with_source_module(mut self, module: impl Into<String>) -> Self {
        self.source_module = Some(module.into());
        self
    }

    pub fn database(err: sqlx::Error) -> Self {
        let err_clone = format!("{}", &err);
        let error = Self::new(RegistryErrorKind::Database(err), ErrorCategory::Database);
        tracing::error!(
            category = "database",
            error_code = %error.error_code,
            error = %err_clone,
            "database_error"
        );
        error
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        let msg_str = msg.into();
        let error = Self::new(
            RegistryErrorKind::NotFound(msg_str.clone()),
            ErrorCategory::NotFound,
        );
        tracing::warn!(
            category = "not_found",
            error_code = %error.error_code,
            message = %msg_str,
            "resource_not_found"
        );
        error
    }

    pub fn invalid_input(msg: impl Into<String>) -> Self {
        let msg_str = msg.into();
        let error = Self::new(
            RegistryErrorKind::InvalidInput(msg_str.clone()),
            ErrorCategory::InvalidInput,
        );
        tracing::warn!(
            category = "invalid_input",
            error_code = %error.error_code,
            message = %msg_str,
            "invalid_input"
        );
        error
    }

    pub fn verification_failed(msg: impl Into<String>) -> Self {
        let msg_str = msg.into();
        let error = Self::new(
            RegistryErrorKind::VerificationFailed(msg_str.clone()),
            ErrorCategory::VerificationFailed,
        );
        tracing::error!(
            category = "verification_failed",
            error_code = %error.error_code,
            message = %msg_str,
            "verification_failed"
        );
        error
    }

    pub fn stellar_rpc(msg: impl Into<String>) -> Self {
        let msg_str = msg.into();
        let error = Self::new(
            RegistryErrorKind::StellarRpc(msg_str.clone()),
            ErrorCategory::StellarRpc,
        );
        tracing::error!(
            category = "stellar_rpc",
            error_code = %error.error_code,
            message = %msg_str,
            "stellar_rpc_error"
        );
        error
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        let msg_str = msg.into();
        let backtrace = capture_backtrace();
        let error = Self {
            kind: RegistryErrorKind::Internal(msg_str.clone()),
            backtrace,
            category: ErrorCategory::Internal,
            error_code: ErrorCode::InternalError,
            source_module: None,
        };
        tracing::error!(
            category = "internal",
            error_code = %error.error_code,
            message = %msg_str,
            backtrace = %error.backtrace.as_deref().unwrap_or("none"),
            "internal_error"
        );
        error
    }

    pub fn s3(msg: impl Into<String>) -> Self {
        let msg_str = msg.into();
        let error = Self::new(RegistryErrorKind::S3(msg_str.clone()), ErrorCategory::S3);
        tracing::error!(
            category = "s3",
            error_code = %error.error_code,
            message = %msg_str,
            "s3_error"
        );
        error
    }

    pub fn io(err: std::io::Error) -> Self {
        let err_str = format!("{}", &err);
        let error = Self::new(RegistryErrorKind::Io(err), ErrorCategory::Internal);
        tracing::error!(
            category = "io",
            error_code = %error.error_code,
            error = %err_str,
            "io_error"
        );
        error
    }

    pub fn log_with_context(&self) {
        match &self.kind {
            RegistryErrorKind::Database(e) => {
                tracing::error!(
                    category = %self.category.as_str(),
                    error_code = %self.error_code,
                    error = %e,
                    backtrace = %self.backtrace.as_deref().unwrap_or("none"),
                    source_module = %self.source_module.as_deref().unwrap_or("unknown"),
                    "database_error"
                );
            }
            RegistryErrorKind::StellarRpc(msg) => {
                tracing::error!(
                    category = %self.category.as_str(),
                    error_code = %self.error_code,
                    message = %msg,
                    backtrace = %self.backtrace.as_deref().unwrap_or("none"),
                    source_module = %self.source_module.as_deref().unwrap_or("unknown"),
                    "stellar_rpc_error"
                );
            }
            _ => {
                tracing::error!(
                    category = %self.category.as_str(),
                    error_code = %self.error_code,
                    error = %self,
                    backtrace = %self.backtrace.as_deref().unwrap_or("none"),
                    source_module = %self.source_module.as_deref().unwrap_or("unknown"),
                    "registry_error"
                );
            }
        }
    }
}

fn capture_backtrace() -> Option<String> {
    let backtrace = Backtrace::capture();
    if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
        Some(format!("{:#}", backtrace))
    } else {
        None
    }
}

impl From<sqlx::Error> for RegistryError {
    fn from(err: sqlx::Error) -> Self {
        RegistryError::database(err)
    }
}

impl From<serde_json::Error> for RegistryError {
    fn from(err: serde_json::Error) -> Self {
        let err_str = format!("{}", &err);
        let error = Self::new(RegistryErrorKind::Serde(err), ErrorCategory::Internal);
        tracing::error!(
            category = "serialization",
            error_code = %error.error_code,
            error = %err_str,
            "json_error"
        );
        error
    }
}

impl From<std::io::Error> for RegistryError {
    fn from(err: std::io::Error) -> Self {
        RegistryError::io(err)
    }
}

impl From<anyhow::Error> for RegistryError {
    fn from(err: anyhow::Error) -> Self {
        let err_str = format!("{}", &err);
        let backtrace = capture_backtrace();
        let error = Self {
            kind: RegistryErrorKind::Anyhow(err),
            backtrace,
            category: ErrorCategory::Internal,
            error_code: ErrorCode::InternalError,
            source_module: None,
        };
        tracing::error!(
            category = "internal",
            error_code = %error.error_code,
            error = %err_str,
            backtrace = %error.backtrace.as_deref().unwrap_or("none"),
            "anyhow_error"
        );
        error
    }
}

impl From<s3::error::S3Error> for RegistryError {
    fn from(err: s3::error::S3Error) -> Self {
        RegistryError::s3(format!("{}", err))
    }
}

impl From<s3::creds::error::CredentialsError> for RegistryError {
    fn from(err: s3::creds::error::CredentialsError) -> Self {
        RegistryError::s3(format!("Credentials error: {}", err))
    }
}

pub type Result<T> = std::result::Result<T, RegistryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::BadRequest.as_str(), "BAD_REQUEST");
        assert_eq!(ErrorCode::InternalError.as_str(), "INTERNAL_ERROR");
        assert_eq!(ErrorCode::NotFound.to_string(), "NOT_FOUND");
    }

    #[test]
    fn test_error_category_as_str() {
        assert_eq!(ErrorCategory::Database.as_str(), "database");
        assert_eq!(ErrorCategory::Internal.as_str(), "internal");
        assert_eq!(
            ErrorCategory::VerificationFailed.as_str(),
            "verification_failed"
        );
    }

    #[test]
    fn test_registry_error_creation() {
        let err = RegistryError::not_found("Contract not found");
        assert_eq!(err.category(), ErrorCategory::NotFound);
        assert_eq!(err.error_code(), ErrorCode::NotFound);
        assert!(err.to_string().contains("NOT_FOUND"));
    }

    #[test]
    fn test_invalid_input_error() {
        let err = RegistryError::invalid_input("Empty source code");
        assert_eq!(err.category(), ErrorCategory::InvalidInput);
        assert_eq!(err.error_code(), ErrorCode::InvalidInput);
    }

    #[test]
    fn test_internal_error_captures_backtrace() {
        let err = RegistryError::internal("Something went wrong");
        assert_eq!(err.category(), ErrorCategory::Internal);
        assert_eq!(err.error_code(), ErrorCode::InternalError);
    }

    #[test]
    fn test_source_module() {
        let err = RegistryError::internal("test").with_source_module("verifier");
        assert_eq!(err.source_module(), Some("verifier"));
    }

    #[test]
    fn test_error_display_format() {
        let err = RegistryError::not_found("missing resource");
        let display = format!("{}", err);
        assert!(display.contains("NOT_FOUND"));
        assert!(display.contains("not_found"));
        assert!(display.contains("missing resource"));
    }
}
