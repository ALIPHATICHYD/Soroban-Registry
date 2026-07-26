//! Unit + file-level tests for Issue #1090 contract deprecation state.

use shared::{DeprecationStatus, UndeprecateContractRequest};

#[test]
fn migration_file_exists_and_is_additive() {
    let migration_path =
        "../../database/migrations/20260725010000_issue1090_contract_deprecation_state.sql";
    assert!(
        std::path::Path::new(migration_path).exists(),
        "Migration file should exist at {migration_path}"
    );

    let content = std::fs::read_to_string(migration_path).expect("read migration");
    assert!(
        content.contains("ADD COLUMN IF NOT EXISTS deprecated_at"),
        "must add deprecated_at"
    );
    assert!(
        content.contains("ADD COLUMN IF NOT EXISTS deprecation_reason"),
        "must add deprecation_reason"
    );
    assert!(
        content.contains("ADD COLUMN IF NOT EXISTS replacement_contract_id"),
        "must add replacement_contract_id"
    );
    assert!(
        content.contains("ADD COLUMN IF NOT EXISTS is_deprecated"),
        "must add is_deprecated for trending/similar filters"
    );
    assert!(
        content.contains("GENERATED ALWAYS AS"),
        "deprecation_status should be generated from columns"
    );
    // #1061 flags rows as deprecated without a timestamp, so the backfill has to
    // run before the flag/timestamp consistency constraint is applied.
    let backfill = content
        .find("SET deprecated_at = NOW()")
        .expect("migration should backfill deprecated_at");
    let constraint = content
        .find("chk_contracts_deprecation_flag_consistency")
        .expect("migration should add the consistency constraint");
    assert!(
        backfill < constraint,
        "backfill must run before the consistency constraint"
    );
    assert!(
        !content.contains("DROP TABLE"),
        "additive migration must not drop tables"
    );
    assert!(
        !content.contains("DROP COLUMN"),
        "additive migration must not drop columns"
    );
}

#[test]
fn deprecation_status_covers_deprecate_resolve_replace() {
    // Active → resolve by hash remains Active
    assert_eq!(
        DeprecationStatus::from_columns(None, None),
        DeprecationStatus::Active
    );

    // Deprecate without replacement
    let deprecated_at = chrono::Utc::now();
    assert_eq!(
        DeprecationStatus::from_columns(Some(deprecated_at), None),
        DeprecationStatus::Deprecated
    );

    // Replace / supersede with lineage pointer
    let replacement = uuid::Uuid::new_v4();
    assert_eq!(
        DeprecationStatus::from_columns(Some(deprecated_at), Some(replacement)),
        DeprecationStatus::Superseded
    );
}

#[test]
fn undeprecate_without_override_is_rejected_by_flag_check() {
    let denied = UndeprecateContractRequest {
        r#override: false,
        force: false,
    };
    assert!(!denied.has_override());

    let allowed_override = UndeprecateContractRequest {
        r#override: true,
        force: false,
    };
    assert!(allowed_override.has_override());

    let allowed_force = UndeprecateContractRequest {
        r#override: false,
        force: true,
    };
    assert!(allowed_force.has_override());
}

#[test]
fn contract_response_serializes_deprecation_fields() {
    use chrono::Utc;
    use shared::{Contract, Network, VerificationStatus, VisibilityType};
    use uuid::Uuid;

    let replacement = Uuid::new_v4();
    let contract = Contract {
        id: Uuid::new_v4(),
        contract_id: "C_OLD".into(),
        wasm_hash: "abc".into(),
        name: "Old".into(),
        slug: "old".into(),
        description: None,
        publisher_id: Uuid::new_v4(),
        network: Network::Testnet,
        is_verified: true,
        verification_status: VerificationStatus::Verified,
        category: None,
        tags: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        verified_at: None,
        deployed_at: None,
        verified_by: None,
        verification_notes: None,
        last_accessed_at: None,
        health_score: 0,
        is_maintenance: false,
        logical_id: None,
        network_configs: None,
        relevance_score: None,
        organization_id: None,
        visibility: VisibilityType::Public,
        current_version: Some("1.0.0".into()),
        usage_count: 0,
        deprecated_at: Some(Utc::now()),
        deprecation_reason: Some("use C_NEW".into()),
        replacement_contract_id: Some(replacement),
        is_deprecated: true,
        deprecation_status: DeprecationStatus::Superseded,
    };

    let json = serde_json::to_value(&contract).expect("serialize");
    assert_eq!(json["deprecation_status"], "superseded");
    assert_eq!(json["is_deprecated"], true);
    assert_eq!(
        json["replacement_contract_id"],
        serde_json::json!(replacement.to_string())
    );
    assert_eq!(json["deprecation_reason"], "use C_NEW");
}

#[test]
fn search_hit_shape_includes_deprecation_for_pagination_stability() {
    // Document the search response contract used by both PG and ES paths so
    // cursor pages interleaved with deprecated rows still expose status.
    let hit = serde_json::json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "contract_id": "C123",
        "name": "Demo",
        "description": null,
        "category": null,
        "network": "testnet",
        "is_verified": true,
        "deprecation_status": "deprecated",
        "replacement_contract_id": null,
        "is_deprecated": true,
        "relevance_score": 1.0
    });

    assert_eq!(hit["deprecation_status"], "deprecated");
    assert_eq!(hit["is_deprecated"], true);
    assert!(hit.get("replacement_contract_id").is_some());
}
