//! Field validators for input validation
//!
//! This module provides reusable validation functions for common field types
//! in the Soroban Registry API.

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Stellar contract ID pattern: 56 characters starting with 'C'
    static ref CONTRACT_ID_REGEX: Regex = Regex::new(r"^C[A-Z0-9]{55}$").unwrap();

    /// Stellar address pattern: 56 characters starting with 'G'
    static ref STELLAR_ADDRESS_REGEX: Regex = Regex::new(r"^G[A-Z0-9]{55}$").unwrap();

    /// Semver pattern: major.minor.patch with optional pre-release
    static ref SEMVER_REGEX: Regex = Regex::new(
        r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$"
    ).unwrap();

    /// URL pattern for source URLs
    static ref URL_REGEX: Regex = Regex::new(
        r"^https?://[^\s/$.?#].[^\s]*$"
    ).unwrap();

    /// HTML tag detection pattern
    static ref HTML_TAG_REGEX: Regex = Regex::new(r"<[^>]+>").unwrap();

    /// Script/event handler pattern for XSS detection
    static ref XSS_PATTERN_REGEX: Regex = Regex::new(
        r"(?i)(javascript:|on\w+\s*=|<script|<iframe|<object|<embed)"
    ).unwrap();

    /// WASM hash pattern: 64 hexadecimal characters
    static ref WASM_HASH_REGEX: Regex = Regex::new(r"^[a-fA-F0-9]{64}$").unwrap();

    /// Contract name pattern: Alphanumeric, spaces, hyphens, and underscores
    static ref NAME_FORMAT_REGEX: Regex = Regex::new(r"^[a-zA-Z0-9\s\-_]+$").unwrap();
}

/// Validate that a string is not empty after trimming
pub fn validate_required(value: &str, field_name: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{} is required", field_name));
    }
    Ok(())
}

/// Validate string length within bounds
pub fn validate_length(value: &str, min: usize, max: usize) -> Result<(), String> {
    let len = value.chars().count();
    if len < min {
        return Err(format!("must be at least {} characters", min));
    }
    if len > max {
        return Err(format!("must be at most {} characters", max));
    }
    Ok(())
}

/// Validate that a string length is within bounds, returning field-specific error
pub fn validate_length_with_field(
    value: &str,
    min: usize,
    max: usize,
    field_name: &str,
) -> Result<(), String> {
    let len = value.chars().count();
    if len < min {
        return Err(format!(
            "{} must be at least {} characters",
            field_name, min
        ));
    }
    if len > max {
        return Err(format!("{} must be at most {} characters", field_name, max));
    }
    Ok(())
}

/// Validate Stellar contract ID format
/// Must be 56 characters starting with 'C'
pub fn validate_contract_id(contract_id: &str) -> Result<(), String> {
    let trimmed = contract_id.trim();

    if trimmed.is_empty() {
        return Err("contract_id is required".to_string());
    }

    if !CONTRACT_ID_REGEX.is_match(trimmed) {
        return Err(
            "must be a valid Stellar contract ID (56 characters starting with 'C')".to_string(),
        );
    }

    Ok(())
}

/// Validate Stellar address format
/// Must be 56 characters starting with 'G'
pub fn validate_stellar_address(address: &str) -> Result<(), String> {
    let trimmed = address.trim();

    if trimmed.is_empty() {
        return Err("stellar address is required".to_string());
    }

    if !STELLAR_ADDRESS_REGEX.is_match(trimmed) {
        return Err(
            "must be a valid Stellar address (56 characters starting with 'G')".to_string(),
        );
    }

    Ok(())
}

/// Validate optional Stellar address (only validates if Some)
pub fn validate_stellar_address_optional(address: &Option<String>) -> Result<(), String> {
    match address {
        Some(addr) if !addr.trim().is_empty() => validate_stellar_address(addr),
        _ => Ok(()),
    }
}

/// Validate semver version string
pub fn validate_semver(version: &str) -> Result<(), String> {
    let trimmed = version.trim();

    if trimmed.is_empty() {
        return Err("version is required".to_string());
    }

    if !SEMVER_REGEX.is_match(trimmed) {
        return Err("must be a valid semantic version (e.g., 1.0.0)".to_string());
    }

    Ok(())
}

/// Validate WASM hash format
/// Must be 64 hexadecimal characters (SHA-256 length)
pub fn validate_wasm_hash(hash: &str) -> Result<(), String> {
    let trimmed = hash.trim();

    if trimmed.is_empty() {
        return Err("wasm_hash is required".to_string());
    }

    if !WASM_HASH_REGEX.is_match(trimmed) {
        return Err("must be a valid 64-character hexadecimal SHA-256 hash".to_string());
    }

    Ok(())
}

/// Validate contract name format
/// Alphanumeric, spaces, hyphens, and underscores only
pub fn validate_name_format(name: &str) -> Result<(), String> {
    if !NAME_FORMAT_REGEX.is_match(name) {
        return Err(
            "name can only contain alphanumeric characters, spaces, hyphens, and underscores"
                .to_string(),
        );
    }
    Ok(())
}

/// Validate category against a whitelist
pub fn validate_category_whitelist(category: &str, whitelist: &[&str]) -> Result<(), String> {
    if !whitelist.contains(&category) {
        return Err(format!(
            "invalid category '{}'. allowed: [{}]",
            category,
            whitelist.join(", ")
        ));
    }
    Ok(())
}

/// Validate URL format
pub fn validate_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();

    if trimmed.is_empty() {
        return Ok(()); // Empty URLs are allowed (optional field)
    }

    if !URL_REGEX.is_match(trimmed) {
        return Err("must be a valid URL (starting with http:// or https://)".to_string());
    }

    Ok(())
}

/// Validate optional URL
pub fn validate_url_optional(url: &Option<String>) -> Result<(), String> {
    match url {
        Some(u) if !u.trim().is_empty() => validate_url(u),
        _ => Ok(()),
    }
}

/// Validate that a string contains no HTML tags
pub fn validate_no_html(value: &str) -> Result<(), String> {
    if HTML_TAG_REGEX.is_match(value) {
        return Err("HTML tags are not allowed".to_string());
    }
    Ok(())
}

/// Validate that a string contains no potential XSS patterns
pub fn validate_no_xss(value: &str) -> Result<(), String> {
    if XSS_PATTERN_REGEX.is_match(value) {
        return Err("potentially unsafe content detected".to_string());
    }
    Ok(())
}

/// Validate a list of tags
pub fn validate_tags(
    tags: &[String],
    max_tags: usize,
    max_tag_length: usize,
) -> Result<(), String> {
    if tags.len() > max_tags {
        return Err(format!("at most {} tags are allowed", max_tags));
    }

    for (i, tag) in tags.iter().enumerate() {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            return Err(format!("tag at index {} cannot be empty", i));
        }
        if trimmed.len() > max_tag_length {
            return Err(format!(
                "tag '{}' exceeds maximum length of {} characters",
                trimmed, max_tag_length
            ));
        }
        if let Err(e) = validate_no_xss(trimmed) {
            return Err(format!("tag '{}': {}", trimmed, e));
        }
    }

    Ok(())
}

/// Validate source code size
pub fn validate_source_code_size(source: &str, max_bytes: usize) -> Result<(), String> {
    let size = source.len();
    if size > max_bytes {
        let max_mb = max_bytes as f64 / (1024.0 * 1024.0);
        let actual_mb = size as f64 / (1024.0 * 1024.0);
        return Err(format!(
            "source code size ({:.2} MB) exceeds maximum allowed ({:.2} MB)",
            actual_mb, max_mb
        ));
    }
    Ok(())
}

/// Validate JSON value is not deeply nested (prevent DoS)
pub fn validate_json_depth(value: &serde_json::Value, max_depth: usize) -> Result<(), String> {
    fn check_depth(v: &serde_json::Value, current: usize, max: usize) -> Result<(), String> {
        if current > max {
            return Err(format!("JSON exceeds maximum nesting depth of {}", max));
        }
        match v {
            serde_json::Value::Array(arr) => {
                for item in arr {
                    check_depth(item, current + 1, max)?;
                }
            }
            serde_json::Value::Object(obj) => {
                for (_, val) in obj {
                    check_depth(val, current + 1, max)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    check_depth(value, 0, max_depth)
}

/// Validate that a value is one of the allowed strings.
pub fn validate_one_of(value: &str, allowed: &[&str]) -> Result<(), String> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(format!("must be one of: {}", allowed.join(", ")))
    }
}

/// Validate optional string against allowed values when present and non-empty.
pub fn validate_one_of_optional(value: &Option<String>, allowed: &[&str]) -> Result<(), String> {
    match value {
        Some(v) if !v.trim().is_empty() => validate_one_of(v.trim(), allowed),
        _ => Ok(()),
    }
}

/// Validate a slug: lowercase alphanumeric and hyphens.
pub fn validate_slug(slug: &str) -> Result<(), String> {
    lazy_static! {
        static ref SLUG_REGEX: Regex = Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").unwrap();
    }
    if slug.is_empty() {
        return Err("slug is required".to_string());
    }
    if slug.len() > 100 {
        return Err("slug must be at most 100 characters".to_string());
    }
    if !SLUG_REGEX.is_match(slug) {
        return Err("slug must contain only lowercase letters, numbers, and hyphens".to_string());
    }
    Ok(())
}

/// Validate a basic email format.
pub fn validate_email(email: &str) -> Result<(), String> {
    lazy_static! {
        static ref EMAIL_REGEX: Regex = Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap();
    }
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return Err("email is required".to_string());
    }
    if trimmed.len() > 320 {
        return Err("email must be at most 320 characters".to_string());
    }
    if !EMAIL_REGEX.is_match(trimmed) {
        return Err("must be a valid email address".to_string());
    }
    Ok(())
}

/// Validate collection size is within bounds.
pub fn validate_collection_size(count: usize, min: usize, max: usize) -> Result<(), String> {
    if count < min {
        return Err(format!("at least {} items are required", min));
    }
    if count > max {
        return Err(format!("at most {} items are allowed", max));
    }
    Ok(())
}

/// Validate a numeric rating (e.g. 0.0–5.0).
pub fn validate_rating(value: f64, min: f64, max: f64) -> Result<(), String> {
    if !value.is_finite() {
        return Err("rating must be a finite number".to_string());
    }
    if value < min || value > max {
        return Err(format!("rating must be between {} and {}", min, max));
    }
    Ok(())
}

/// Validate a percentage value (0.0–100.0).
pub fn validate_percentage(value: f64) -> Result<(), String> {
    validate_rating(value, 0.0, 100.0)
        .map_err(|_| "percentage must be between 0 and 100".to_string())
}

/// Validate hex string of exact length.
pub fn validate_hex_length(value: &str, len: usize) -> Result<(), String> {
    lazy_static! {
        static ref HEX_REGEX: Regex = Regex::new(r"^[a-fA-F0-9]+$").unwrap();
    }
    let trimmed = value.trim();
    if trimmed.len() != len {
        return Err(format!("must be exactly {} hexadecimal characters", len));
    }
    if !HEX_REGEX.is_match(trimmed) {
        return Err("must contain only hexadecimal characters".to_string());
    }
    Ok(())
}

/// Validate base64-encoded content size.
pub fn validate_base64_size(value: &str, max_decoded_bytes: usize) -> Result<(), String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("base64 content is required".to_string());
    }
    if trimmed.len() > max_decoded_bytes * 4 / 3 + 4 {
        return Err(format!(
            "base64 payload exceeds maximum allowed size ({} bytes decoded)",
            max_decoded_bytes
        ));
    }
    let decoded = STANDARD
        .decode(trimmed)
        .map_err(|_| "must be valid base64-encoded data".to_string())?;
    if decoded.len() > max_decoded_bytes {
        return Err(format!(
            "decoded content exceeds maximum allowed size ({} bytes)",
            max_decoded_bytes
        ));
    }
    Ok(())
}

/// Validate per-network config version range (Issue #43).
/// Ensures min_version and max_version are valid semver and min <= max when both present.
pub fn validate_network_config_versions(
    min_version: Option<&str>,
    max_version: Option<&str>,
) -> Result<(), String> {
    if let Some(min) = min_version.filter(|s| !s.trim().is_empty()) {
        validate_semver(min).map_err(|e| format!("min_version: {}", e))?;
    }
    if let Some(max) = max_version.filter(|s| !s.trim().is_empty()) {
        validate_semver(max).map_err(|e| format!("max_version: {}", e))?;
    }
    if let (Some(min), Some(max)) = (
        min_version.filter(|s| !s.trim().is_empty()),
        max_version.filter(|s| !s.trim().is_empty()),
    ) {
        let a = shared::SemVer::parse(min.trim())
            .ok_or_else(|| "min_version must be a valid semver (e.g. 1.0.0)".to_string())?;
        let b = shared::SemVer::parse(max.trim())
            .ok_or_else(|| "max_version must be a valid semver (e.g. 1.0.0)".to_string())?;
        if a > b {
            return Err("min_version must be less than or equal to max_version".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_contract_id() {
        // Valid contract ID
        let valid_id = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
        assert!(validate_contract_id(valid_id).is_ok());

        // Invalid: starts with G
        let invalid_g = "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
        assert!(validate_contract_id(invalid_g).is_err());

        // Invalid: too short
        assert!(validate_contract_id("CABC123").is_err());

        // Invalid: empty
        assert!(validate_contract_id("").is_err());
    }

    #[test]
    fn test_validate_stellar_address() {
        // Valid address
        let valid = "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
        assert!(validate_stellar_address(valid).is_ok());

        // Invalid: starts with C
        let invalid_c = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
        assert!(validate_stellar_address(invalid_c).is_err());
    }

    #[test]
    fn test_validate_length() {
        assert!(validate_length("hello", 1, 10).is_ok());
        assert!(validate_length("", 1, 10).is_err());
        assert!(validate_length("hello world!", 1, 5).is_err());
    }

    #[test]
    fn test_validate_no_html() {
        assert!(validate_no_html("plain text").is_ok());
        assert!(validate_no_html("<script>alert('xss')</script>").is_err());
        assert!(validate_no_html("<b>bold</b>").is_err());
    }

    #[test]
    fn test_validate_no_xss() {
        assert!(validate_no_xss("normal text").is_ok());
        assert!(validate_no_xss("javascript:alert(1)").is_err());
        assert!(validate_no_xss("onclick=alert(1)").is_err());
    }

    #[test]
    fn test_validate_tags() {
        let valid_tags = vec!["defi".to_string(), "token".to_string()];
        assert!(validate_tags(&valid_tags, 10, 50).is_ok());

        // Too many tags
        let many_tags: Vec<String> = (0..15).map(|i| format!("tag{}", i)).collect();
        assert!(validate_tags(&many_tags, 10, 50).is_err());
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://github.com/user/repo").is_ok());
        assert!(validate_url("http://example.com").is_ok());
        assert!(validate_url("not-a-url").is_err());
        assert!(validate_url("ftp://invalid.com").is_err());
    }

    #[test]
    fn test_validate_semver() {
        assert!(validate_semver("1.0.0").is_ok());
        assert!(validate_semver("0.1.0-alpha").is_ok());
        assert!(validate_semver("2.0.0-rc.1+build.123").is_ok());
        assert!(validate_semver("not-a-version").is_err());
    }

    #[test]
    fn test_validate_wasm_hash() {
        let valid = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert!(validate_wasm_hash(valid).is_ok());

        assert!(validate_wasm_hash("abc").is_err());
        assert!(validate_wasm_hash("not-hex").is_err());
    }

    #[test]
    fn test_validate_name_format() {
        assert!(validate_name_format("My Contract").is_ok());
        assert!(validate_name_format("My-Contract_123").is_ok());
        assert!(validate_name_format("Contract!").is_err());
        assert!(validate_name_format("<b>HTML</b>").is_err());
    }

    #[test]
    fn test_validate_category_whitelist() {
        let whitelist = vec!["DEX", "Lending"];
        assert!(validate_category_whitelist("DEX", &whitelist).is_ok());
        assert!(validate_category_whitelist("Bridge", &whitelist).is_err());
    }
}
