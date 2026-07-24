//! Input Validation Module
//!
//! This module provides comprehensive input validation and sanitization
//! for the Soroban Registry API.
//!
//! # Overview
//!
//! The validation system consists of three main components:
//!
//! 1. **Extractors** - Custom Axum extractors like `ValidatedJson<T>`
//! 2. **Validators** - Reusable validation functions for common patterns
//! 3. **Sanitizers** - Functions to clean and normalize input data
//!
//! # Usage
//!
//! ## Using ValidatedJson in Handlers
//!
//! ```ignore
//! use crate::validation::{ValidatedJson, Validatable, FieldError, ValidationBuilder};
//!
//! // Implement Validatable for your request type
//! impl Validatable for MyRequest {
//!     fn sanitize(&mut self) {
//!         self.name = sanitizers::sanitize_name(&self.name);
//!     }
//!
//!     fn validate(&self) -> Result<(), Vec<FieldError>> {
//!         ValidationBuilder::new()
//!             .check("name", || validators::validate_length(&self.name, 1, 255))
//!             .build()
//!     }
//! }
//!
//! // Use in handler - validation happens automatically
//! pub async fn create_item(
//!     ValidatedJson(req): ValidatedJson<MyRequest>,
//! ) -> impl IntoResponse {
//!     // req is sanitized and validated
//! }
//! ```
//!
//! ## Validation Error Response
//!
//! When validation fails, a 400 Bad Request is returned:
//!
//! ```json
//! {
//!   "error_code": "BAD_REQUEST",
//!   "message": "Validation failed for 2 fields",
//!   "details": {
//!     "reason": "VALIDATION_ERROR",
//!     "field_errors": [
//!       {"field": "contract_id", "message": "must be a valid Stellar contract ID"},
//!       {"field": "name", "message": "must be at least 1 character"}
//!     ],
//!     "correlation_id": "uuid-here"
//!   },
//!   "timestamp": "2026-02-20T10:30:00Z"
//! }
//! ```

#[allow(dead_code)]
pub mod enhanced_extractors;
#[allow(dead_code)]
pub mod extractors;
#[allow(dead_code)]
pub mod handler_requests;
#[allow(dead_code)]
pub mod payload_size;
#[allow(dead_code)]
pub mod requests;
#[allow(dead_code)]
pub mod sanitizers;
#[allow(dead_code)]
pub mod url_validation;
#[allow(dead_code)]
pub mod validation_rate_limit;
#[allow(dead_code)]
pub mod validators;

#[cfg(test)]
mod comprehensive_tests;

#[cfg(test)]
mod integration_guide;

// Re-export commonly used items
#[allow(unused_imports)]
pub use enhanced_extractors::{validate_request, validate_response_schema_dev, ValidationRule};
#[allow(unused_imports)]
pub use extractors::{
    FieldError, Validatable, ValidatedJson, ValidatedPath, ValidatedQuery, ValidationBuilder,
    ValidationError,
};
#[allow(unused_imports)]
pub use handler_requests::{BatchContractIdsRequest, RecordMetricsBatchRequest, ValidatedJsonBody};
#[allow(unused_imports)]
pub use sanitizers::{
    normalize_contract_id, normalize_stellar_address, sanitize_description,
    sanitize_description_optional, sanitize_name, sanitize_tags, sanitize_url_optional, strip_html,
    trim, trim_optional,
};
#[allow(unused_imports)]
pub use url_validation::{
    get_domain_whitelist, parse_url_components, validate_https_url_only,
    validate_url_https_only_with_whitelist, UrlComponents,
};
#[allow(unused_imports)]
pub use validators::{
    validate_category_whitelist, validate_contract_id, validate_length, validate_name_format,
    validate_network_config_versions, validate_no_html, validate_no_xss, validate_required,
    validate_semver, validate_source_code_size, validate_stellar_address,
    validate_stellar_address_optional, validate_tags, validate_url, validate_url_optional,
    validate_wasm_hash,
};
