use serde_json::{Map, Value};
use std::backtrace::Backtrace;

/// Log a fatal error and abort the process.
///
/// This macro logs a message at `error` level with `fatal = true`, captures
/// a backtrace, and then aborts the process.
#[macro_export]
macro_rules! fatal {
    ($($arg:tt)+) => {{
        let backtrace = $crate::logging::capture_backtrace_string();
        tracing::error!(
            fatal = true,
            backtrace = %backtrace.as_deref().unwrap_or("none"),
            $($arg)+
        );
        std::process::abort();
    }};
}

/// Log a structured error with full context, category, and backtrace.
pub fn log_error(message: &str, category: &str, status: u16, request_id: Option<&str>) {
    let backtrace = capture_backtrace_string();
    tracing::error!(
        category = category,
        status = status,
        request_id = request_id,
        backtrace = %backtrace.as_deref().unwrap_or("none"),
        "{message}",
    );
}

/// Log a structured warning with context.
pub fn log_warn(message: &str, category: &str, request_id: Option<&str>) {
    tracing::warn!(category = category, request_id = request_id, "{message}",);
}

/// Log a structured info message with context.
pub fn log_info(message: &str, category: &str, request_id: Option<&str>) {
    tracing::info!(category = category, request_id = request_id, "{message}",);
}

/// Log a structured debug message with context.
pub fn log_debug(message: &str, category: &str, request_id: Option<&str>) {
    tracing::debug!(category = category, request_id = request_id, "{message}",);
}

/// Capture a full backtrace as a formatted string, if available.
pub fn capture_backtrace_string() -> Option<String> {
    let backtrace = Backtrace::capture();
    if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
        Some(format!("{:#}", backtrace))
    } else {
        None
    }
}

/// Redact sensitive fields from a JSON value before logging.
///
/// Recursively walks the value and replaces values for known sensitive keys
/// with `[REDACTED]`.
pub fn redact_sensitive_data(value: &mut Value) {
    match value {
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                if is_sensitive_key(&key) {
                    map.insert(key, Value::String("[REDACTED]".to_string()));
                } else if let Some(inner) = map.get_mut(&key) {
                    redact_sensitive_data(inner);
                }
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                redact_sensitive_data(item);
            }
        }
        _ => {}
    }
}

/// Check if a key is considered sensitive and should be redacted.
pub fn is_sensitive_key(key: &str) -> bool {
    let lowered = key.to_ascii_lowercase();
    lowered.contains("password")
        || lowered.contains("secret")
        || lowered.contains("token")
        || lowered.contains("api_key")
        || lowered.contains("private_key")
        || lowered.contains("authorization")
        || lowered.contains("cookie")
        || lowered.contains("jwt")
        || lowered.contains("session")
        || lowered.contains("credit_card")
        || lowered.contains("ssn")
        || lowered.contains("access_key")
}

/// Sanitize a JSON value by redacting sensitive fields.
///
/// Returns a new sanitized value without modifying the original.
pub fn sanitize_value(value: &Value) -> Value {
    match value {
        Value::Object(obj) => {
            let mut sanitized = Map::new();
            for (key, val) in obj {
                if is_sensitive_key(key) {
                    sanitized.insert(key.clone(), Value::String("[REDACTED]".to_string()));
                } else {
                    sanitized.insert(key.clone(), sanitize_value(val));
                }
            }
            Value::Object(sanitized)
        }
        Value::Array(items) => Value::Array(items.iter().map(sanitize_value).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_capture_backtrace_string() {
        let bt = capture_backtrace_string();
        // May be None if RUST_BACKTRACE is not set, but should not panic
        if let Some(backtrace) = bt {
            assert!(backtrace.contains("capture_backtrace_string"));
        }
    }

    #[test]
    fn test_redact_sensitive_data() {
        let mut data = json!({
            "username": "alice",
            "password": "supersecret",
            "token": "abc123",
            "nested": {
                "api_key": "key123",
                "normal_field": "hello",
            }
        });

        redact_sensitive_data(&mut data);

        assert_eq!(data["password"], "[REDACTED]");
        assert_eq!(data["token"], "[REDACTED]");
        assert_eq!(data["nested"]["api_key"], "[REDACTED]");
        assert_eq!(data["nested"]["normal_field"], "hello");
        assert_eq!(data["username"], "alice");
    }

    #[test]
    fn test_sanitize_value_does_not_modify_original() {
        let original = json!({
            "password": "secret123",
        });

        let sanitized = sanitize_value(&original);

        assert_eq!(original["password"], "secret123");
        assert_eq!(sanitized["password"], "[REDACTED]");
    }

    #[test]
    fn test_is_sensitive_key() {
        assert!(is_sensitive_key("password"));
        assert!(is_sensitive_key("PASSWORD"));
        assert!(is_sensitive_key("api_key"));
        assert!(is_sensitive_key("authorization"));
        assert!(is_sensitive_key("private_key"));
        assert!(is_sensitive_key("credit_card"));
        assert!(!is_sensitive_key("username"));
        assert!(!is_sensitive_key("email"));
        assert!(!is_sensitive_key("contract_id"));
    }

    #[test]
    fn test_log_functions_do_not_panic() {
        log_error("test error", "test", 500, Some("req-1"));
        log_warn("test warning", "test", Some("req-1"));
        log_info("test info", "test", Some("req-1"));
        log_debug("test debug", "test", Some("req-1"));
    }
}
