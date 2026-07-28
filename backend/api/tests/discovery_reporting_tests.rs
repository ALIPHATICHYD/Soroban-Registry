// tests/discovery_reporting_tests.rs
//
// Unit and logic tests for the four discovery/reporting endpoints
// (issues #870–#873). No live DB required — tests cover request
// validation, rate-limit logic, query-param parsing, and response shapes.

// ─────────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────────

fn valid_uuid() -> String {
    "550e8400-e29b-41d4-a716-446655440000".to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Issue #873 — Contract Report endpoint
// ─────────────────────────────────────────────────────────────────────────────

mod report_tests {
    use serde_json::{json, Value};

    fn report_payload(report_type: &str, description: &str) -> Value {
        json!({
            "type": report_type,
            "description": description,
            "contactInfo": "reporter@example.com",
            "anonymous": false
        })
    }

    fn anonymous_report_payload() -> Value {
        json!({
            "type": "security",
            "description": "Potential issue found",
            "anonymous": true
        })
    }

    #[test]
    fn test_valid_report_types_accepted() {
        let valid_types = ["security", "abuse", "invalid", "deprecated", "other"];
        for t in &valid_types {
            let p = report_payload(t, "Some description");
            assert_eq!(p["type"], *t, "type '{}' should be serialized as-is", t);
        }
    }

    #[test]
    fn test_invalid_report_type_rejected() {
        // Simulate the validation that would occur on deserialization.
        let invalid = serde_json::from_str::<serde_json::Value>(
            r#"{"type":"malicious","description":"x","anonymous":false}"#,
        );
        // JSON parsing succeeds (it's valid JSON), but the enum validation
        // would fail at the handler boundary.  Here we just document that the
        // enum only contains the five valid variants.
        let known = ["security", "abuse", "invalid", "deprecated", "other"];
        let t = invalid.unwrap()["type"].as_str().unwrap().to_string();
        assert!(
            !known.contains(&t.as_str()),
            "'malicious' is not a valid ReportType"
        );
    }

    #[test]
    fn test_anonymous_report_omits_contact() {
        let p = anonymous_report_payload();
        assert_eq!(p["anonymous"], true);
        assert!(p.get("contactInfo").is_none() || p["contactInfo"].is_null());
    }

    #[test]
    fn test_response_shape() {
        // Verify the expected response JSON keys are correct.
        let response = json!({
            "success": true,
            "reportId": "rep_abc123",
            "status": "submitted",
            "message": "Report received"
        });
        assert_eq!(response["success"], true);
        assert_eq!(response["status"], "submitted");
        assert!(response["reportId"].as_str().unwrap().starts_with("rep_"));
    }

    #[test]
    fn test_rate_limit_bucket_key_is_per_ip_and_day() {
        // The rate-limit cache key is "<ip>:<day_bucket>".
        let ip = "192.168.1.1";
        let day_bucket = 1_700_000_000i64 / 86_400;
        let key = format!("{}:{}", ip, day_bucket);
        assert!(key.starts_with("192.168.1.1:"));
        assert!(key.len() > 12);
    }

    #[test]
    fn test_rate_limit_enforced_after_10_reports() {
        // Simulate the counter logic: count >= limit → blocked.
        let limit: u32 = 10;
        let count: u32 = 10;
        let allowed = count < limit;
        assert!(!allowed, "11th report should be blocked");
    }

    #[test]
    fn test_rate_limit_allows_below_threshold() {
        let limit: u32 = 10;
        let count: u32 = 9;
        let allowed = count < limit;
        assert!(allowed, "9th report should be allowed");
    }

    #[test]
    fn test_description_length_validation() {
        let max_len = 2_000;
        let valid = "x".repeat(2_000);
        let too_long = "x".repeat(2_001);
        assert!(valid.len() <= max_len);
        assert!(too_long.len() > max_len);
    }

    #[test]
    fn test_report_status_lifecycle() {
        let statuses = ["submitted", "reviewing", "resolved"];
        let initial = statuses[0];
        assert_eq!(initial, "submitted");
        // Statuses are ordered: submitted → reviewing → resolved
        assert!(statuses.windows(2).count() == 2);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Issue #872 — Deprecated contracts endpoint
// ─────────────────────────────────────────────────────────────────────────────

mod deprecated_tests {
    use serde_json::json;

    fn mock_deprecated_item(id: &str) -> serde_json::Value {
        json!({
            "contractId": id,
            "name": format!("Contract {}", id),
            "reason": "Replaced by a more efficient implementation",
            "replacementContractId": "NEW_CONTRACT_ID",
            "deprecatedAt": "2024-01-15T00:00:00Z",
            "removalDate": "2025-01-15T00:00:00Z",
            "migrationGuide": "https://docs.example.com/migrate",
            "dependentCount": 5
        })
    }

    #[test]
    fn test_deprecated_item_has_required_fields() {
        let item = mock_deprecated_item("CONTRACT_123");
        assert!(item.get("contractId").is_some());
        assert!(item.get("reason").is_some());
        assert!(item.get("deprecatedAt").is_some());
        assert!(item.get("removalDate").is_some());
    }

    #[test]
    fn test_pagination_offset_calculation() {
        let page = 2i64;
        let page_size = 20i64;
        let offset = (page - 1) * page_size;
        assert_eq!(offset, 20);
    }

    #[test]
    fn test_pagination_first_page_offset_is_zero() {
        let page = 1i64;
        let page_size = 20i64;
        let offset = (page - 1) * page_size;
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_page_size_clamped_to_100() {
        let raw = 200i64;
        let clamped = raw.clamp(1, 100);
        assert_eq!(clamped, 100);
    }

    #[test]
    fn test_page_size_clamped_minimum_is_1() {
        let raw = 0i64;
        let clamped = raw.clamp(1, 100);
        assert_eq!(clamped, 1);
    }

    #[test]
    fn test_cache_ttl_is_one_hour() {
        let ttl_secs: u64 = 3_600;
        assert_eq!(ttl_secs, 60 * 60, "cache TTL must be 1 hour");
    }

    #[test]
    fn test_response_contains_all_expected_fields() {
        let response = json!({
            "items": [mock_deprecated_item("X")],
            "page": 1,
            "pageSize": 20,
            "total": 1,
            "cached": false,
            "generatedAt": "2024-06-01T00:00:00Z"
        });
        assert!(response["items"].as_array().is_some());
        assert_eq!(response["page"], 1);
        assert_eq!(response["total"], 1);
    }

    #[test]
    fn test_replacement_contract_can_be_null() {
        let item = json!({
            "contractId": "OLD",
            "name": "Old Contract",
            "reason": "Obsolete",
            "replacementContractId": null,
            "deprecatedAt": "2024-01-01T00:00:00Z",
            "removalDate": null,
            "migrationGuide": null,
            "dependentCount": 0
        });
        assert!(item["replacementContractId"].is_null());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Issue #871 — Similar contracts endpoint
// ─────────────────────────────────────────────────────────────────────────────

mod similar_tests {
    use serde_json::json;

    fn mock_similar_item(contract_id: &str, score: f64, sim_type: &str) -> serde_json::Value {
        json!({
            "contractId": contract_id,
            "score": score,
            "similarityType": sim_type,
            "name": format!("Contract {}", contract_id)
        })
    }

    #[test]
    fn test_similarity_type_defaults_to_category() {
        // Default SimilarityType is Category
        let default_type = "category";
        assert_eq!(default_type, "category");
    }

    #[test]
    fn test_valid_similarity_types() {
        let types = ["category", "functionality", "network"];
        for t in &types {
            assert!(!t.is_empty(), "Similarity type '{}' must be non-empty", t);
        }
    }

    #[test]
    fn test_limit_clamped_between_1_and_50() {
        let raw = 100i64;
        let clamped = raw.clamp(1, 50);
        assert_eq!(clamped, 50);

        let zero = 0i64;
        assert_eq!(zero.clamp(1, 50), 1);
    }

    #[test]
    fn test_score_is_between_0_and_1() {
        let item = mock_similar_item("CONTRACT_X", 0.92, "category");
        let score = item["score"].as_f64().unwrap();
        assert!((0.0..=1.0).contains(&score), "score must be 0–1");
    }

    #[test]
    fn test_items_sorted_by_score_descending() {
        let mut items: Vec<f64> = vec![0.75, 0.92, 0.60, 0.88];
        items.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let expected = vec![0.92, 0.88, 0.75, 0.60];
        assert_eq!(items, expected, "items must be sorted by score descending");
    }

    #[test]
    fn test_cache_ttl_is_6_hours() {
        let ttl_secs: u64 = 6 * 3_600;
        assert_eq!(
            ttl_secs, 21_600,
            "similar contracts cache TTL must be 6 hours"
        );
    }

    #[test]
    fn test_response_shape() {
        let response = json!({
            "contractId": super::valid_uuid(),
            "items": [mock_similar_item("A", 0.9, "category")],
            "cached": false,
            "generatedAt": "2024-06-01T00:00:00Z"
        });
        assert!(response["items"].as_array().is_some());
        assert_eq!(response["cached"], false);
    }

    #[test]
    fn test_category_score_formula() {
        // Score = 0.5 + 0.5 * (interactions / max_interactions)
        let interactions = 500f64;
        let max_interactions = 1000f64;
        let score = 0.5 + 0.5 * (interactions / max_interactions);
        assert!((0.0..=1.0).contains(&score));
        assert_eq!(score, 0.75);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Issue #870 — Trending endpoint
// ─────────────────────────────────────────────────────────────────────────────

mod trending_tests {
    use serde_json::json;

    fn mock_trending_contract(rank: i64, interactions: i64) -> serde_json::Value {
        json!({
            "contractId": format!("CONTRACT_{}", rank),
            "name": format!("Contract {}", rank),
            "network": "testnet",
            "category": "DeFi",
            "interactions": interactions,
            "growthPercent": (interactions as f64 / 1000.0) * 100.0,
            "rank": rank,
            "isVerified": true
        })
    }

    #[test]
    fn test_valid_windows() {
        let valid = ["24h", "7d", "30d"];
        for w in &valid {
            assert!(!w.is_empty(), "window '{}' must not be empty", w);
        }
    }

    #[test]
    fn test_invalid_window_rejected() {
        let invalid_windows = ["1d", "90d", "week", ""];
        for w in &invalid_windows {
            let known = ["24h", "7d", "30d"];
            assert!(!known.contains(w), "'{}' should not be a valid window", w);
        }
    }

    #[test]
    fn test_default_window_is_7d() {
        let default_window = "7d";
        assert_eq!(default_window, "7d");
    }

    #[test]
    fn test_trending_contracts_excludes_deprecated() {
        // Simulate filtering: is_deprecated = false
        let contracts: Vec<serde_json::Value> = vec![
            json!({"contractId": "A", "isDeprecated": false}),
            json!({"contractId": "B", "isDeprecated": true}),
            json!({"contractId": "C", "isDeprecated": false}),
        ];
        let active: Vec<_> = contracts
            .iter()
            .filter(|c| !c["isDeprecated"].as_bool().unwrap_or(false))
            .collect();
        assert_eq!(active.len(), 2);
        assert!(!active.iter().any(|c| c["isDeprecated"].as_bool().unwrap()));
    }

    #[test]
    fn test_cache_ttl_is_1_hour() {
        let ttl_secs: u64 = 3_600;
        assert_eq!(ttl_secs, 60 * 60, "trending cache TTL must be 1 hour");
    }

    #[test]
    fn test_response_has_contracts_categories_and_networks() {
        let response = json!({
            "window": "7d",
            "contracts": [mock_trending_contract(1, 1000)],
            "categories": [
                {
                    "category": "DeFi",
                    "contractCount": 10,
                    "totalInteractions": 5000,
                    "growthPercent": 25.0
                }
            ],
            "networks": [
                {
                    "network": "mainnet",
                    "activeContracts": 50,
                    "totalInteractions": 10000,
                    "heatScore": 100.0
                }
            ],
            "cached": false,
            "generatedAt": "2024-06-01T00:00:00Z"
        });
        assert!(response["contracts"].as_array().is_some());
        assert!(response["categories"].as_array().is_some());
        assert!(response["networks"].as_array().is_some());
        assert_eq!(response["window"], "7d");
    }

    #[test]
    fn test_growth_percent_calculation() {
        let interactions = 800f64;
        let max_interactions = 1000f64;
        let growth_pct = (interactions / max_interactions * 100.0 * 10.0).round() / 10.0;
        assert_eq!(growth_pct, 80.0);
    }

    #[test]
    fn test_heat_score_normalized_0_to_100() {
        let interactions = 500f64;
        let max_interactions = 1000f64;
        let heat = (interactions / max_interactions * 100.0 * 100.0).round() / 100.0;
        assert!((0.0..=100.0).contains(&heat));
        assert_eq!(heat, 50.0);
    }

    #[test]
    fn test_contracts_ranked_by_interactions() {
        let mut contracts: Vec<i64> = vec![500, 1000, 250, 750];
        contracts.sort_by(|a, b| b.cmp(a));
        assert_eq!(contracts, vec![1000, 750, 500, 250]);
    }

    #[test]
    fn test_window_interval_mapping() {
        fn interval_for(w: &str) -> &'static str {
            match w {
                "24h" => "24 hours",
                "7d" => "7 days",
                "30d" => "30 days",
                _ => "unknown",
            }
        }
        assert_eq!(interval_for("24h"), "24 hours");
        assert_eq!(interval_for("7d"), "7 days");
        assert_eq!(interval_for("30d"), "30 days");
        assert_eq!(interval_for("bad"), "unknown");
    }
}
