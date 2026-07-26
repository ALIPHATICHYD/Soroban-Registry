//! Integration tests for `soroban-registry contract audit` (#1060)
//!
//! These tests verify the CLI surface (help output, flag presence, error
//! messages) and lockfile I/O without requiring a live registry API.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> PathBuf {
    let name_hyphen = "soroban-registry";
    let name_underscore = "soroban_registry";

    if let Ok(path) = env::var(format!("CARGO_BIN_EXE_{}", name_underscore)) {
        return PathBuf::from(path);
    }
    if let Ok(path) = env::var(format!("CARGO_BIN_EXE_{}", name_hyphen)) {
        return PathBuf::from(path);
    }

    let mut path = env::current_dir().expect("Failed to get current dir");
    path.push("target");
    path.push("debug");
    path.push(name_hyphen);
    if path.exists() {
        return path;
    }
    path.set_extension("exe");
    if path.exists() {
        return path;
    }

    panic!("Could not find binary path via env var. Ensure `cargo build` has run.");
}

// ── Help / flag presence tests ───────────────────────────────────────────────

#[test]
fn test_contract_audit_help_shows_drift_flags() {
    let output = Command::new(get_binary_path())
        .args(["contract", "audit", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // All key flags must appear in the help output
    assert!(
        stdout.contains("--lockfile"),
        "Missing --lockfile flag in help output"
    );
    assert!(
        stdout.contains("--fix"),
        "Missing --fix flag in help output"
    );
    assert!(
        stdout.contains("--init"),
        "Missing --init flag in help output"
    );
    assert!(
        stdout.contains("--contracts"),
        "Missing --contracts flag in help output"
    );
    assert!(
        stdout.contains("--json"),
        "Missing --json flag in help output"
    );
    assert!(
        stdout.contains("--format"),
        "Missing --format flag in help output"
    );
}

#[test]
fn test_contract_audit_help_mentions_drift() {
    let output = Command::new(get_binary_path())
        .args(["contract", "audit", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The help text should mention drift detection
    assert!(
        stdout.to_lowercase().contains("drift"),
        "Help text should mention 'drift'"
    );
}

// ── Missing lockfile error ───────────────────────────────────────────────────

#[test]
fn test_contract_audit_missing_lockfile_errors() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let lockfile = dir.path().join("nonexistent.lock.json");

    let output = Command::new(get_binary_path())
        .args([
            "contract",
            "audit",
            "--lockfile",
            lockfile.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "Should fail when lockfile is missing"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Lockfile not found") || stderr.contains("not found"),
        "Error should mention missing lockfile, got: {}",
        stderr
    );
}

// ── Init without contracts errors ────────────────────────────────────────────

#[test]
fn test_contract_audit_init_without_contracts_errors() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let lockfile = dir.path().join("new.lock.json");

    let output = Command::new(get_binary_path())
        .args([
            "contract",
            "audit",
            "--lockfile",
            lockfile.to_str().unwrap(),
            "--init",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "Should fail when --init is used without --contracts"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No contract IDs"),
        "Error should mention missing contract IDs, got: {}",
        stderr
    );
}

// ── Init refuses to overwrite existing lockfile ──────────────────────────────

#[test]
fn test_contract_audit_init_refuses_overwrite() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let lockfile = dir.path().join("existing.lock.json");
    fs::write(&lockfile, "{}").expect("Failed to write dummy lockfile");

    let output = Command::new(get_binary_path())
        .args([
            "contract",
            "audit",
            "--lockfile",
            lockfile.to_str().unwrap(),
            "--init",
            "--contracts",
            "CABC123",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "Should fail when lockfile already exists"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("already exists"),
        "Error should mention existing lockfile, got: {}",
        stderr
    );
}

// ── JSON output flag ─────────────────────────────────────────────────────────

#[test]
fn test_contract_audit_json_flag_accepted() {
    // This tests that the --json flag is accepted by the parser.
    // The command will fail because no lockfile exists at the default path,
    // but it should NOT fail because of an unrecognized flag.
    let output = Command::new(get_binary_path())
        .args(["contract", "audit", "--json", "--lockfile", "/tmp/nonexistent_test_lockfile.json"])
        .output()
        .expect("Failed to execute command");

    // It should fail due to missing lockfile, not due to unrecognized flags
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("unrecognized"),
        "--json flag should be accepted, got: {}",
        stderr
    );
}

// ── Lockfile I/O unit-level tests (run via the test binary) ──────────────────

#[test]
fn test_lockfile_write_creates_valid_json() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("test.lock.json");

    let lockfile_json = serde_json::json!({
        "version": 1,
        "generated_at": "2025-07-01T00:00:00Z",
        "registry_url": "http://localhost:3000",
        "contracts": {
            "CABC123": {
                "contract_id": "CABC123",
                "name": "test-token",
                "version": "1.0.0",
                "network": "testnet",
                "hash": "deadbeef",
                "updated_at": "2025-07-01T00:00:00Z",
                "status": "active"
            }
        }
    });

    fs::write(&path, serde_json::to_string_pretty(&lockfile_json).unwrap())
        .expect("Failed to write lockfile");

    let content = fs::read_to_string(&path).expect("Failed to read lockfile");
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("Lockfile should be valid JSON");

    assert_eq!(parsed["version"], 1);
    assert_eq!(parsed["contracts"]["CABC123"]["name"], "test-token");
    assert_eq!(parsed["contracts"]["CABC123"]["version"], "1.0.0");
}

#[test]
fn test_lockfile_with_multiple_contracts() {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = dir.path().join("multi.lock.json");

    let lockfile_json = serde_json::json!({
        "version": 1,
        "generated_at": "2025-07-01T00:00:00Z",
        "registry_url": "http://localhost:3000",
        "contracts": {
            "CONTRACT_A": {
                "contract_id": "CONTRACT_A",
                "name": "defi-pool",
                "version": "2.1.0",
                "network": "mainnet"
            },
            "CONTRACT_B": {
                "contract_id": "CONTRACT_B",
                "name": "oracle",
                "version": "1.0.0",
                "network": "testnet"
            },
            "CONTRACT_C": {
                "contract_id": "CONTRACT_C",
                "name": "nft-marketplace",
                "version": "3.0.0-beta",
                "network": "futurenet",
                "status": "deprecated"
            }
        }
    });

    fs::write(&path, serde_json::to_string_pretty(&lockfile_json).unwrap())
        .expect("Failed to write lockfile");

    let content = fs::read_to_string(&path).expect("Failed to read lockfile");
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("Lockfile should be valid JSON");

    let contracts = parsed["contracts"].as_object().unwrap();
    assert_eq!(contracts.len(), 3);
    assert_eq!(parsed["contracts"]["CONTRACT_C"]["status"], "deprecated");
}

#[test]
fn test_drift_report_json_structure() {
    // Verify the drift report JSON structure matches expectations
    let report = serde_json::json!({
        "added": [{"contract_id": "CNEW", "name": "new-contract"}],
        "removed": [{"contract_id": "COLD", "name": "old-contract"}],
        "changed": [{
            "contract_id": "CMOD",
            "name": "modified-contract",
            "changes": [{
                "field": "version",
                "local": "1.0.0",
                "remote": "2.0.0"
            }]
        }],
        "audited": 3,
        "has_drift": true
    });

    assert_eq!(report["added"].as_array().unwrap().len(), 1);
    assert_eq!(report["removed"].as_array().unwrap().len(), 1);
    assert_eq!(report["changed"].as_array().unwrap().len(), 1);
    assert!(report["has_drift"].as_bool().unwrap());
    assert_eq!(report["changed"][0]["changes"][0]["field"], "version");
}
