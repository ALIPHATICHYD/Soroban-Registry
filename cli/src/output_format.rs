/// Centralized output formatting module for machine-readable and human-readable output formats.
/// Supports: Table (human-readable), JSON, CSV, and YAML (machine-readable).
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::str::FromStr;

/// Supported output formats for CLI commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable table format (default)
    Table,
    /// Machine-readable JSON format
    Json,
    /// Machine-readable CSV format
    Csv,
    /// Machine-readable YAML format
    Yaml,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" | "text" => Ok(OutputFormat::Table),
            "json" => Ok(OutputFormat::Json),
            "csv" => Ok(OutputFormat::Csv),
            "yaml" | "yml" => Ok(OutputFormat::Yaml),
            _ => Err(format!(
                "Invalid output format '{}'. Supported formats: table, json, csv, yaml",
                s
            )),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Csv => write!(f, "csv"),
            OutputFormat::Yaml => write!(f, "yaml"),
        }
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Table
    }
}

/// Infers output format from file extension
pub fn infer_format_from_extension(path: &str) -> Option<OutputFormat> {
    let lower = path.to_lowercase();
    if lower.ends_with(".json") {
        Some(OutputFormat::Json)
    } else if lower.ends_with(".csv") {
        Some(OutputFormat::Csv)
    } else if lower.ends_with(".yaml") || lower.ends_with(".yml") {
        Some(OutputFormat::Yaml)
    } else if lower.ends_with(".txt") || lower.ends_with(".table") {
        Some(OutputFormat::Table)
    } else {
        None
    }
}

/// Validates that a format string is a supported format
pub fn validate_format(format: &str) -> Result<OutputFormat> {
    format.parse::<OutputFormat>().map_err(|e| anyhow!(e))
}

/// Renders JSON output with pretty printing
pub fn render_json(data: &Value) -> Result<String> {
    serde_json::to_string_pretty(data).map_err(|e| anyhow!("Failed to serialize JSON: {}", e))
}

/// Renders YAML output
pub fn render_yaml(data: &Value) -> Result<String> {
    serde_yaml::to_string(data).map_err(|e| anyhow!("Failed to serialize YAML: {}", e))
}

/// Renders CSV output from a JSON array of objects
pub fn render_csv(data: &Value) -> Result<String> {
    let array = data
        .as_array()
        .ok_or_else(|| anyhow!("CSV format requires an array of objects"))?;

    if array.is_empty() {
        return Ok(String::new());
    }

    // Extract headers from first object
    let first_obj = array[0]
        .as_object()
        .ok_or_else(|| anyhow!("CSV format requires an array of objects"))?;

    let headers: Vec<String> = first_obj.keys().cloned().collect();
    let mut output = String::new();

    // Write header
    output.push_str(&headers.join(","));
    output.push('\n');

    // Write rows
    for item in array {
        let obj = item
            .as_object()
            .ok_or_else(|| anyhow!("CSV format requires an array of objects"))?;

        let row: Vec<String> = headers
            .iter()
            .map(|header| {
                let value = obj.get(header).unwrap_or(&Value::Null);
                csv_escape_value(value)
            })
            .collect();

        output.push_str(&row.join(","));
        output.push('\n');
    }

    Ok(output)
}

/// Escapes a JSON value for CSV output
fn csv_escape_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            // Escape quotes and wrap in quotes if needed
            if s.contains(',') || s.contains('"') || s.contains('\n') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.clone()
            }
        }
        Value::Array(arr) => {
            // Join array elements with pipe separator
            let items: Vec<String> = arr
                .iter()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    _ => v.to_string(),
                })
                .collect();
            format!("\"{}\"", items.join("|"))
        }
        Value::Object(_) => {
            // Serialize nested objects as JSON string
            format!("\"{}\"", value.to_string().replace('"', "\"\""))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_parsing() {
        assert_eq!(
            "table".parse::<OutputFormat>().unwrap(),
            OutputFormat::Table
        );
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("csv".parse::<OutputFormat>().unwrap(), OutputFormat::Csv);
        assert_eq!("yaml".parse::<OutputFormat>().unwrap(), OutputFormat::Yaml);
        assert_eq!("yml".parse::<OutputFormat>().unwrap(), OutputFormat::Yaml);
        assert!("invalid".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_format_case_insensitive() {
        assert_eq!(
            "TABLE".parse::<OutputFormat>().unwrap(),
            OutputFormat::Table
        );
        assert_eq!("JSON".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("CSV".parse::<OutputFormat>().unwrap(), OutputFormat::Csv);
        assert_eq!("YAML".parse::<OutputFormat>().unwrap(), OutputFormat::Yaml);
    }

    #[test]
    fn test_infer_format_from_extension() {
        assert_eq!(
            infer_format_from_extension("data.json"),
            Some(OutputFormat::Json)
        );
        assert_eq!(
            infer_format_from_extension("data.csv"),
            Some(OutputFormat::Csv)
        );
        assert_eq!(
            infer_format_from_extension("data.yaml"),
            Some(OutputFormat::Yaml)
        );
        assert_eq!(
            infer_format_from_extension("data.yml"),
            Some(OutputFormat::Yaml)
        );
        assert_eq!(
            infer_format_from_extension("data.txt"),
            Some(OutputFormat::Table)
        );
        assert_eq!(infer_format_from_extension("data.unknown"), None);
    }

    #[test]
    fn test_validate_format() {
        assert!(validate_format("json").is_ok());
        assert!(validate_format("csv").is_ok());
        assert!(validate_format("yaml").is_ok());
        assert!(validate_format("table").is_ok());
        assert!(validate_format("invalid").is_err());
    }

    #[test]
    fn test_render_json() {
        let data = serde_json::json!({ "name": "test", "value": 42 });
        let result = render_json(&data).unwrap();
        assert!(result.contains("\"name\""));
        assert!(result.contains("\"test\""));
        assert!(result.contains("\"value\""));
        assert!(result.contains("42"));
    }

    #[test]
    fn test_render_yaml() {
        let data = serde_json::json!({ "name": "test", "value": 42 });
        let result = render_yaml(&data).unwrap();
        assert!(result.contains("name:"));
        assert!(result.contains("test"));
        assert!(result.contains("value:"));
        assert!(result.contains("42"));
    }

    #[test]
    fn test_render_csv_basic() {
        let data = serde_json::json!([
            { "name": "Alice", "age": 30 },
            { "name": "Bob", "age": 25 }
        ]);
        let result = render_csv(&data).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 rows
        assert!(lines[0].contains("name") || lines[0].contains("age"));
    }

    #[test]
    fn test_render_csv_with_special_chars() {
        let data = serde_json::json!([
            { "name": "Alice, Bob", "description": "Has \"quotes\"" }
        ]);
        let result = render_csv(&data).unwrap();
        assert!(result.contains("\"Alice, Bob\""));
        assert!(result.contains("\"Has \"\"quotes\"\"\""));
    }

    #[test]
    fn test_csv_escape_value_string_with_comma() {
        let value = Value::String("hello, world".to_string());
        let result = csv_escape_value(&value);
        assert_eq!(result, "\"hello, world\"");
    }

    #[test]
    fn test_csv_escape_value_string_with_quotes() {
        let value = Value::String("say \"hello\"".to_string());
        let result = csv_escape_value(&value);
        assert_eq!(result, "\"say \"\"hello\"\"\"");
    }

    #[test]
    fn test_csv_escape_value_number() {
        let value = Value::Number(serde_json::Number::from(42));
        let result = csv_escape_value(&value);
        assert_eq!(result, "42");
    }

    #[test]
    fn test_csv_escape_value_bool() {
        let value = Value::Bool(true);
        let result = csv_escape_value(&value);
        assert_eq!(result, "true");
    }

    #[test]
    fn test_csv_escape_value_null() {
        let value = Value::Null;
        let result = csv_escape_value(&value);
        assert_eq!(result, "");
    }

    #[test]
    fn test_csv_escape_value_array() {
        let value = Value::Array(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
        ]);
        let result = csv_escape_value(&value);
        assert_eq!(result, "\"a|b\"");
    }

    #[test]
    fn test_format_display() {
        assert_eq!(OutputFormat::Table.to_string(), "table");
        assert_eq!(OutputFormat::Json.to_string(), "json");
        assert_eq!(OutputFormat::Csv.to_string(), "csv");
        assert_eq!(OutputFormat::Yaml.to_string(), "yaml");
    }

    #[test]
    fn test_format_default() {
        assert_eq!(OutputFormat::default(), OutputFormat::Table);
    }
}
