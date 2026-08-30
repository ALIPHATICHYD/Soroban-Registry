// tests/migration_versioning_tests.rs
//
// Unit tests for the database migration versioning and rollback system (Issue #252).
// Tests cover checksum computation, version gap detection, validation logic,
// and rollback state management.

use sha2::{Digest, Sha256};
use std::path::Path;

/// Compute SHA-256 hex checksum (mirrors the handler logic).
fn compute_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Detect gaps in a sorted list of version numbers.
fn detect_version_gaps(versions: &[i32]) -> Vec<i32> {
    let mut gaps = Vec::new();
    if let (Some(&min), Some(&max)) = (versions.first(), versions.last()) {
        for v in min..=max {
            if !versions.contains(&v) {
                gaps.push(v);
            }
        }
    }
    gaps
}

/// Simulate migration state for testing.
#[derive(Debug, Clone)]
struct FakeMigration {
    version: i32,
    description: String,
    filename: String,
    checksum: String,
    rolled_back: bool,
}

fn make_migration(version: i32, sql: &str, rolled_back: bool) -> FakeMigration {
    FakeMigration {
        version,
        description: format!("Migration v{}", version),
        filename: format!("{:03}_migration.sql", version),
        checksum: compute_checksum(sql),
        rolled_back,
    }
}

fn get_current_version(migrations: &[FakeMigration]) -> Option<i32> {
    migrations
        .iter()
        .filter(|m| !m.rolled_back)
        .map(|m| m.version)
        .max()
}

fn count_applied(migrations: &[FakeMigration]) -> usize {
    migrations.iter().filter(|m| !m.rolled_back).count()
}

fn count_rolled_back(migrations: &[FakeMigration]) -> usize {
    migrations.iter().filter(|m| m.rolled_back).count()
}

fn is_healthy(migrations: &[FakeMigration]) -> bool {
    let active: Vec<i32> = migrations
        .iter()
        .filter(|m| !m.rolled_back)
        .map(|m| m.version)
        .collect();
    detect_version_gaps(&active).is_empty()
}

// ─── Checksum Tests ──────────────────────────────────────────────────────────

#[test]
fn test_checksum_deterministic() {
    let sql = "CREATE TABLE foo (id INT);";
    let c1 = compute_checksum(sql);
    let c2 = compute_checksum(sql);
    assert_eq!(c1, c2, "Same content should produce same checksum");
}

#[test]
fn test_checksum_different_content() {
    let c1 = compute_checksum("CREATE TABLE foo (id INT);");
    let c2 = compute_checksum("CREATE TABLE bar (id INT);");
    assert_ne!(
        c1, c2,
        "Different content should produce different checksums"
    );
}

#[test]
fn test_checksum_is_sha256_hex() {
    let checksum = compute_checksum("test");
    assert_eq!(checksum.len(), 64, "SHA-256 hex should be 64 chars");
    assert!(
        checksum.chars().all(|c| c.is_ascii_hexdigit()),
        "Should be valid hex"
    );
}

#[test]
fn test_checksum_empty_string() {
    let checksum = compute_checksum("");
    // SHA-256 of empty string is well-known
    assert_eq!(
        checksum,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_checksum_whitespace_sensitive() {
    let c1 = compute_checksum("SELECT 1;");
    let c2 = compute_checksum("SELECT  1;");
    assert_ne!(
        c1, c2,
        "Whitespace differences should produce different checksums"
    );
}

#[test]
fn test_migration_checksum_stability_for_sample_migrations() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let migrations_dir = manifest_dir.join("../../database/migrations");
    let sample_files = [
        "001_initial.sql",
        "002_add_abi.sql",
        "005_create_migrations_table.sql",
    ];

    let checksums: Vec<String> = sample_files
        .iter()
        .map(|file_name| {
            let path = migrations_dir.join(file_name);
            let sql = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("Failed to read migration file {}", path.display()));
            compute_checksum(&sql)
        })
        .collect();

    let repeat_checksums: Vec<String> = sample_files
        .iter()
        .map(|file_name| {
            let path = migrations_dir.join(file_name);
            let sql = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("Failed to read migration file {}", path.display()));
            compute_checksum(&sql)
        })
        .collect();

    assert_eq!(checksums, repeat_checksums);
    assert_eq!(checksums.len(), sample_files.len());
    assert!(checksums.iter().all(|c| c.len() == 64));
}

// ─── Version Gap Detection ───────────────────────────────────────────────────

#[test]
fn test_no_gaps_sequential() {
    let versions = vec![1, 2, 3, 4, 5];
    assert!(detect_version_gaps(&versions).is_empty());
}

#[test]
fn test_gap_detected() {
    let versions = vec![1, 2, 4, 5];
    let gaps = detect_version_gaps(&versions);
    assert_eq!(gaps, vec![3]);
}

#[test]
fn test_multiple_gaps() {
    let versions = vec![1, 3, 5, 7];
    let gaps = detect_version_gaps(&versions);
    assert_eq!(gaps, vec![2, 4, 6]);
}

#[test]
fn test_no_gaps_single_version() {
    let versions = vec![5];
    assert!(detect_version_gaps(&versions).is_empty());
}

#[test]
fn test_no_gaps_empty() {
    let versions: Vec<i32> = vec![];
    assert!(detect_version_gaps(&versions).is_empty());
}

// ─── Migration State Tests ───────────────────────────────────────────────────

#[test]
fn test_current_version_with_active_migrations() {
    let migrations = vec![
        make_migration(1, "CREATE TABLE a;", false),
        make_migration(2, "CREATE TABLE b;", false),
        make_migration(3, "CREATE TABLE c;", false),
    ];
    assert_eq!(get_current_version(&migrations), Some(3));
}

#[test]
fn test_current_version_with_rollback() {
    let migrations = vec![
        make_migration(1, "CREATE TABLE a;", false),
        make_migration(2, "CREATE TABLE b;", false),
        make_migration(3, "CREATE TABLE c;", true), // rolled back
    ];
    assert_eq!(get_current_version(&migrations), Some(2));
}

#[test]
fn test_current_version_all_rolled_back() {
    let migrations = vec![
        make_migration(1, "CREATE TABLE a;", true),
        make_migration(2, "CREATE TABLE b;", true),
    ];
    assert_eq!(get_current_version(&migrations), None);
}

#[test]
fn test_current_version_empty() {
    let migrations: Vec<FakeMigration> = vec![];
    assert_eq!(get_current_version(&migrations), None);
}

#[test]
fn test_count_applied() {
    let migrations = vec![
        make_migration(1, "a", false),
        make_migration(2, "b", false),
        make_migration(3, "c", true),
    ];
    assert_eq!(count_applied(&migrations), 2);
    assert_eq!(count_rolled_back(&migrations), 1);
}

#[test]
fn test_healthy_no_gaps() {
    let migrations = vec![
        make_migration(1, "a", false),
        make_migration(2, "b", false),
        make_migration(3, "c", false),
    ];
    assert!(is_healthy(&migrations));
}

#[test]
fn test_unhealthy_with_gap() {
    let migrations = vec![
        make_migration(1, "a", false),
        make_migration(2, "b", true), // rolled back creates gap
        make_migration(3, "c", false),
    ];
    assert!(!is_healthy(&migrations));
}

// ─── Rollback Validation Tests ───────────────────────────────────────────────

#[test]
fn test_rollback_checksum_matches() {
    let down_sql = "DROP TABLE foo;";
    let stored_checksum = compute_checksum(down_sql);
    let actual_checksum = compute_checksum(down_sql);
    assert_eq!(
        stored_checksum, actual_checksum,
        "Rollback checksum should match"
    );
}

#[test]
fn test_rollback_checksum_tampered() {
    let original = "DROP TABLE foo;";
    let tampered = "DROP TABLE foo; DROP TABLE bar;";
    let stored_checksum = compute_checksum(original);
    let actual_checksum = compute_checksum(tampered);
    assert_ne!(
        stored_checksum, actual_checksum,
        "Tampered rollback should not match"
    );
}

#[test]
fn test_migration_filename_format() {
    let m = make_migration(1, "sql", false);
    assert_eq!(m.filename, "001_migration.sql");
    assert_eq!(m.description, "Migration v1");
    assert_eq!(m.checksum.len(), 64);

    let m = make_migration(42, "sql", false);
    assert_eq!(m.filename, "042_migration.sql");
}

// ─── Duplicate Version Detection ─────────────────────────────────────────────

#[test]
fn test_duplicate_version_detection() {
    let migrations = [
        make_migration(1, "a", false),
        make_migration(2, "b", false),
        make_migration(2, "c", false), // duplicate
    ];
    let versions: Vec<i32> = migrations.iter().map(|m| m.version).collect();
    let mut deduped = versions.clone();
    deduped.sort();
    deduped.dedup();
    assert_ne!(
        versions.len(),
        deduped.len(),
        "Duplicate versions should be detected"
    );
}

// ─── Advisory Lock Simulation ────────────────────────────────────────────────

#[test]
fn test_lock_prevents_concurrent_operation() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let lock = AtomicBool::new(false);

    // First acquire succeeds
    let acquired = lock
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok();
    assert!(acquired, "First lock acquisition should succeed");

    // Second acquire fails
    let acquired = lock
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok();
    assert!(!acquired, "Second lock acquisition should fail");

    // Release
    lock.store(false, Ordering::SeqCst);

    // Third acquire succeeds after release
    let acquired = lock
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok();
    assert!(acquired, "Lock acquisition after release should succeed");
}
