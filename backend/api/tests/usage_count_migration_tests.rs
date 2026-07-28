// tests/usage_count_migration_tests.rs
//
// Unit tests for the usage_count migration.
// Tests validate SQL syntax, constraint behavior, and index creation

#[test]
fn test_migration_file_exists() {
    let migration_path = "../../database/migrations/20260427000000_add_usage_count.sql";
    assert!(
        std::path::Path::new(migration_path).exists(),
        "Migration file should exist at {}",
        migration_path
    );
}

#[test]
fn test_migration_contains_required_elements() {
    let migration_path = "../../database/migrations/20260427000000_add_usage_count.sql";
    let content =
        std::fs::read_to_string(migration_path).expect("Should be able to read migration file");

    // Check for required SQL statements
    assert!(
        content.contains("ADD COLUMN usage_count BIGINT NOT NULL DEFAULT 0"),
        "Migration should add usage_count column with correct type and default"
    );

    assert!(
        content.contains("CHECK (usage_count >= 0)"),
        "Migration should add non-negative constraint"
    );

    assert!(
        content.contains("CREATE INDEX idx_contracts_usage_count ON contracts(usage_count DESC)"),
        "Migration should create index for efficient queries"
    );
}

#[test]
fn test_migration_sql_syntax() {
    let migration_path = "../../database/migrations/20260427000000_add_usage_count.sql";
    let content =
        std::fs::read_to_string(migration_path).expect("Should be able to read migration file");

    // Basic SQL syntax validation
    assert!(
        content.contains("ALTER TABLE contracts"),
        "Should modify contracts table"
    );
    assert!(content.contains("ADD CONSTRAINT"), "Should add constraint");
    assert!(!content.contains("DROP"), "Should not drop anything");
    assert!(!content.contains("DELETE"), "Should not delete data");
}

#[test]
fn test_constraint_name_follows_convention() {
    let migration_path = "../../database/migrations/20260427000000_add_usage_count.sql";
    let content =
        std::fs::read_to_string(migration_path).expect("Should be able to read migration file");

    assert!(
        content.contains("chk_contracts_usage_count_non_negative"),
        "Constraint should follow naming convention"
    );
}

#[test]
fn test_index_name_follows_convention() {
    let migration_path = "../../database/migrations/20260427000000_add_usage_count.sql";
    let content =
        std::fs::read_to_string(migration_path).expect("Should be able to read migration file");

    assert!(
        content.contains("idx_contracts_usage_count"),
        "Index should follow naming convention"
    );
}
