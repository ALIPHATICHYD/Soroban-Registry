// tests/database_validation_constraints_tests.rs
//
// Lightweight assertions that the database validation migration exists and
// contains the expected integrity rules.

#[test]
fn test_database_validation_migration_exists() {
    let migration_path =
        "../../database/migrations/20260601000000_database_validation_constraints.sql";
    assert!(
        std::path::Path::new(migration_path).exists(),
        "Migration file should exist at {}",
        migration_path
    );
}

#[test]
fn test_database_validation_migration_contains_core_constraints() {
    let migration_path =
        "../../database/migrations/20260601000000_database_validation_constraints.sql";
    let content =
        std::fs::read_to_string(migration_path).expect("Should be able to read migration file");

    assert!(
        content.contains("chk_contracts_contract_id_format"),
        "Contracts should validate Stellar contract IDs"
    );
    assert!(
        content.contains("chk_contracts_wasm_hash_format"),
        "Contracts should validate WASM hash format"
    );
    assert!(
        content.contains("validate_contract_version_integrity"),
        "Contract version trigger should exist"
    );
    assert!(
        content.contains("validate_verification_integrity"),
        "Verification trigger should exist"
    );
    assert!(
        content.contains("chk_organization_invitations_email_format"),
        "Organization invitations should validate email format"
    );
    assert!(
        content.contains("chk_user_preferences_theme"),
        "User preferences should validate theme values"
    );
}

#[test]
fn test_database_validation_migration_uses_clear_constraint_names() {
    let migration_path =
        "../../database/migrations/20260601000000_database_validation_constraints.sql";
    let content =
        std::fs::read_to_string(migration_path).expect("Should be able to read migration file");

    assert!(
        content.contains("chk_publishers_stellar_address_format"),
        "Publisher address constraint should be clearly named"
    );
    assert!(
        content.contains("chk_contract_versions_signature_algorithm"),
        "Signature algorithm constraint should be clearly named"
    );
    assert!(
        content.contains("chk_notification_queue_status"),
        "Notification queue status constraint should be clearly named"
    );
}
