#![allow(dead_code)]

use anyhow::{Context, Result};
use serde_json::Value;

/// Extracts a string from a JSON Value.
pub fn as_str(val: &Value, field: &str) -> Result<String> {
    val.as_str()
        .map(str_to_string)
        .context(format!("Missing or invalid string field: {}", field))
}

/// Extracts a boolean from a JSON Value.
pub fn as_bool(val: &Value, field: &str) -> Result<bool> {
    val.as_bool()
        .context(format!("Missing or invalid boolean field: {}", field))
}

/// Extracts an f64 from a JSON Value.
pub fn as_f64(val: &Value, field: &str) -> Result<f64> {
    val.as_f64()
        .context(format!("Missing or invalid f64 field: {}", field))
}

/// Extracts an i64 from a JSON Value.
pub fn as_i64(val: &Value, field: &str) -> Result<i64> {
    val.as_i64()
        .context(format!("Missing or invalid i64 field: {}", field))
}

/// Extracts a u64 from a JSON Value, correctly handling f64 boundaries.
pub fn as_u64(val: &Value, field: &str) -> Result<u64> {
    if let Some(num) = val.as_u64() {
        Ok(num)
    } else if let Some(f) = val.as_f64() {
        f64_to_u64(f).context(format!(
            "Invalid f64 to u64 conversion for field: {}",
            field
        ))
    } else {
        anyhow::bail!("Missing or invalid numeric field: {}", field)
    }
}

/// Extracts a usize from a JSON Value.
pub fn as_usize(val: &Value, field: &str) -> Result<usize> {
    let u = as_u64(val, field)?;
    usize::try_from(u)
        .map_err(|_| anyhow::anyhow!("Value exceeds usize capacity for field: {}", field))
}

/// Converts an f64 to a u64 securely.
pub fn f64_to_u64(f: f64) -> Result<u64> {
    if f.is_nan() || f.is_infinite() {
        anyhow::bail!("Cannot convert NaN or Infinity to u64");
    }
    if f < 0.0 {
        anyhow::bail!("Cannot convert negative f64 to u64");
    }
    if f.fract() != 0.0 {
        anyhow::bail!("Cannot convert fractional f64 to u64 without precision loss");
    }
    if f > u64::MAX as f64 {
        anyhow::bail!("Value exceeds u64 maximum");
    }
    Ok(f as u64)
}

/// Extracts an array from a JSON Value.
pub fn as_array<'a>(val: &'a Value, field: &str) -> Result<&'a Vec<Value>> {
    val.as_array()
        .context(format!("Missing or invalid array field: {}", field))
}

/// Extracts an object from a JSON Value.
pub fn as_object<'a>(val: &'a Value, field: &str) -> Result<&'a serde_json::Map<String, Value>> {
    val.as_object()
        .context(format!("Missing or invalid object field: {}", field))
}

/// Unified &str to String explicitly.
pub fn str_to_string(s: &str) -> String {
    s.to_string()
}

/// Unified String to &str explicitly (though typically implicit via Deref).
pub fn string_to_str(s: &str) -> &str {
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_as_str() {
        assert_eq!(as_str(&json!("hello"), "test").unwrap(), "hello");
        assert_eq!(as_str(&json!(""), "test").unwrap(), "");
        assert!(as_str(&json!(42), "test").is_err());
        assert!(as_str(&json!(null), "test").is_err());
        assert!(as_str(&json!([1, 2]), "test").is_err());
    }

    #[test]
    fn test_as_bool() {
        assert!(as_bool(&json!(true), "test").unwrap());
        assert!(!as_bool(&json!(false), "test").unwrap());
        assert!(as_bool(&json!("true"), "test").is_err());
        assert!(as_bool(&json!(1), "test").is_err());
        assert!(as_bool(&json!(null), "test").is_err());
    }

    #[test]
    fn test_as_f64() {
        assert_eq!(as_f64(&json!(42.5), "test").unwrap(), 42.5);
        assert_eq!(as_f64(&json!(-10.25), "test").unwrap(), -10.25);
        assert_eq!(as_f64(&json!(0.0), "test").unwrap(), 0.0);
        assert_eq!(as_f64(&json!(100), "test").unwrap(), 100.0);
        assert!(as_f64(&json!("42.5"), "test").is_err());
        assert!(as_f64(&json!(null), "test").is_err());
    }

    #[test]
    fn test_as_i64() {
        assert_eq!(as_i64(&json!(42), "test").unwrap(), 42);
        assert_eq!(as_i64(&json!(-42), "test").unwrap(), -42);
        assert_eq!(as_i64(&json!(0), "test").unwrap(), 0);
        assert!(as_i64(&json!(42.5), "test").is_err());
        assert!(as_i64(&json!("42"), "test").is_err());
        assert!(as_i64(&json!(null), "test").is_err());
    }

    #[test]
    fn test_as_u64() {
        assert_eq!(as_u64(&json!(42), "test").unwrap(), 42);
        assert_eq!(as_u64(&json!(0), "test").unwrap(), 0);
        assert_eq!(as_u64(&json!(u64::MAX), "test").unwrap(), u64::MAX);
        assert_eq!(as_u64(&json!(42.0), "test").unwrap(), 42);
        assert!(as_u64(&json!(-1), "test").is_err());
        assert!(as_u64(&json!(42.5), "test").is_err());
        assert!(as_u64(&json!("42"), "test").is_err());
        assert!(as_u64(&json!(null), "test").is_err());
    }

    #[test]
    fn test_as_usize() {
        assert_eq!(as_usize(&json!(42), "test").unwrap(), 42);
        assert_eq!(as_usize(&json!(0), "test").unwrap(), 0);
        assert_eq!(as_usize(&json!(42.0), "test").unwrap(), 42);
        assert!(as_usize(&json!(-1), "test").is_err());
        assert!(as_usize(&json!(42.5), "test").is_err());
        assert!(as_usize(&json!(null), "test").is_err());
    }

    #[test]
    fn test_f64_to_u64() {
        assert_eq!(f64_to_u64(0.0).unwrap(), 0);
        assert_eq!(f64_to_u64(42.0).unwrap(), 42);

        assert!(f64_to_u64(-1.0).is_err());
        assert!(f64_to_u64(3.5).is_err());
        assert!(f64_to_u64(f64::NAN).is_err());
        assert!(f64_to_u64(f64::INFINITY).is_err());
        assert!(f64_to_u64(f64::NEG_INFINITY).is_err());
        assert!(f64_to_u64((u64::MAX as f64) * 2.0).is_err());
    }

    #[test]
    fn test_as_array() {
        let v = json!([1, 2, 3]);
        let arr = as_array(&v, "test").unwrap();
        assert_eq!(arr.len(), 3);
        assert!(as_array(&json!({}), "test").is_err());
        assert!(as_array(&json!("[]"), "test").is_err());
        assert!(as_array(&json!(null), "test").is_err());
    }

    #[test]
    fn test_as_object() {
        let v = json!({"a": 1});
        let obj = as_object(&v, "test").unwrap();
        assert_eq!(obj.get("a").unwrap(), &json!(1));
        assert!(as_object(&json!([]), "test").is_err());
        assert!(as_object(&json!("{}"), "test").is_err());
        assert!(as_object(&json!(null), "test").is_err());
    }

    #[test]
    fn test_str_bindings() {
        let s = "hello";
        let s_val = str_to_string(s);
        assert_eq!(s_val, "hello");
        assert_eq!(string_to_str(&s_val), "hello");
    }
}
