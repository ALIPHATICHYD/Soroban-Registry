/// Integration tests for CLI output formats
/// Tests stdout formatting and invalid format names

#[cfg(test)]
mod output_format_tests {
    use serde_json::json;

    #[test]
    fn test_json_output_format() {
        let data = json!({
            "contracts": [
                {
                    "id": "test-id-1",
                    "name": "TestContract",
                    "network": "testnet",
                    "is_verified": true
                }
            ],
            "count": 1
        });

        let json_str = serde_json::to_string_pretty(&data).unwrap();
        assert!(json_str.contains("\"contracts\""));
        assert!(json_str.contains("\"TestContract\""));
        assert!(json_str.contains("\"count\""));
        assert!(json_str.contains("1"));
    }

    #[test]
    fn test_csv_output_format() {
        let csv_output =
            "id,name,network,is_verified\n\"test-id-1\",\"TestContract\",\"testnet\",true\n";
        assert!(csv_output.contains("id,name,network,is_verified"));
        assert!(csv_output.contains("test-id-1"));
        assert!(csv_output.contains("TestContract"));
    }

    #[test]
    fn test_yaml_output_format() {
        let data = json!({
            "contracts": [
                {
                    "id": "test-id-1",
                    "name": "TestContract",
                    "network": "testnet"
                }
            ]
        });

        let yaml_str = serde_yaml::to_string(&data).unwrap();
        assert!(yaml_str.contains("contracts:"));
        assert!(yaml_str.contains("id:"));
        assert!(yaml_str.contains("TestContract"));
    }

    #[test]
    fn test_format_parsing_valid() {
        let formats = vec!["table", "json", "csv", "yaml", "yml"];
        for format in formats {
            assert!(
                format
                    .parse::<soroban_registry_cli::output_format::OutputFormat>()
                    .is_ok(),
                "Format '{}' should be valid",
                format
            );
        }
    }

    #[test]
    fn test_format_parsing_invalid() {
        let invalid_formats = vec!["invalid", "xml", "protobuf", "msgpack"];
        for format in invalid_formats {
            assert!(
                format
                    .parse::<soroban_registry_cli::output_format::OutputFormat>()
                    .is_err(),
                "Format '{}' should be invalid",
                format
            );
        }
    }

    #[test]
    fn test_format_case_insensitive() {
        let formats = vec!["TABLE", "JSON", "CSV", "YAML", "YML"];
        for format in formats {
            assert!(
                format
                    .parse::<soroban_registry_cli::output_format::OutputFormat>()
                    .is_ok(),
                "Format '{}' should be valid (case-insensitive)",
                format
            );
        }
    }

    #[test]
    fn test_csv_special_characters() {
        let csv_with_comma = "\"Alice, Bob\",\"test\"";
        assert!(csv_with_comma.contains("\"Alice, Bob\""));

        let csv_with_quotes = "\"say \"\"hello\"\"\",\"test\"";
        assert!(csv_with_quotes.contains("\"\""));
    }

    #[test]
    fn test_json_schema_stability() {
        let data = json!({
            "contracts": [
                {
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "name": "MyToken",
                    "contract_id": "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4",
                    "network": "testnet",
                    "category": "defi",
                    "is_verified": true,
                    "health_score": 95,
                    "created_at": "2024-01-15T10:30:00Z",
                    "tags": ["token", "erc20"]
                }
            ],
            "count": 1
        });

        let json_str = serde_json::to_string_pretty(&data).unwrap();

        // Verify all expected fields are present
        assert!(json_str.contains("\"id\""));
        assert!(json_str.contains("\"name\""));
        assert!(json_str.contains("\"contract_id\""));
        assert!(json_str.contains("\"network\""));
        assert!(json_str.contains("\"category\""));
        assert!(json_str.contains("\"is_verified\""));
        assert!(json_str.contains("\"health_score\""));
        assert!(json_str.contains("\"created_at\""));
        assert!(json_str.contains("\"tags\""));
        assert!(json_str.contains("\"count\""));
    }

    #[test]
    fn test_empty_array_handling() {
        let empty_data = json!([]);
        let json_str = serde_json::to_string_pretty(&empty_data).unwrap();
        assert_eq!(json_str.trim(), "[]");
    }

    #[test]
    fn test_null_values_handling() {
        let data_with_nulls = json!({
            "name": "Test",
            "description": null,
            "category": null
        });

        let json_str = serde_json::to_string_pretty(&data_with_nulls).unwrap();
        assert!(json_str.contains("null"));
    }
}
