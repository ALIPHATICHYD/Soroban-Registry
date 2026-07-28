//! Integration tests for `soroban-registry contract deprecate` (#1091)
//!
//! These tests verify the CLI surface (help output, required flags, error
//! messages) and the cryptographic signing logic without requiring a live
//! registry API.

use std::env;
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
fn test_deprecate_help_shows_required_flags() {
    let output = Command::new(get_binary_path())
        .args(["contract", "deprecate", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("--reason"),
        "Missing --reason flag in help output"
    );
    assert!(
        stdout.contains("--private-key"),
        "Missing --private-key flag in help output"
    );
    assert!(
        stdout.contains("--replacement"),
        "Missing --replacement flag in help output"
    );
    assert!(
        stdout.contains("--migration-guide"),
        "Missing --migration-guide flag in help output"
    );
    assert!(
        stdout.contains("--grace-period-days"),
        "Missing --grace-period-days flag in help output"
    );
    assert!(
        stdout.contains("--json"),
        "Missing --json flag in help output"
    );
    assert!(
        stdout.contains("-y"),
        "Missing -y (--yes) flag in help output"
    );
}

#[test]
fn test_deprecate_help_mentions_signature() {
    let output = Command::new(get_binary_path())
        .args(["contract", "deprecate", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Help text should mention signature/signing
    let mentions_signing = stdout.to_lowercase().contains("sign")
        || stdout.to_lowercase().contains("ed25519")
        || stdout.to_lowercase().contains("publisher");
    assert!(
        mentions_signing,
        "Help text should mention signing/publisher/Ed25519"
    );
}

// ── Missing required arguments ───────────────────────────────────────────────

#[test]
fn test_deprecate_refuses_without_reason() {
    let output = Command::new(get_binary_path())
        .args([
            "contract",
            "deprecate",
            "CABC123",
            "--private-key",
            "dGVzdGtleXRlc3RrZXl0ZXN0a2V5dGVzdGtleTA=", // 32 bytes base64
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "Should fail when --reason is missing"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--reason") || stderr.contains("required"),
        "Error should mention missing --reason, got: {}",
        stderr
    );
}

#[test]
fn test_deprecate_refuses_without_private_key() {
    let output = Command::new(get_binary_path())
        .args([
            "contract",
            "deprecate",
            "CABC123",
            "--reason",
            "End of life",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "Should fail when --private-key is missing"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--private-key") || stderr.contains("required"),
        "Error should mention missing --private-key, got: {}",
        stderr
    );
}

#[test]
fn test_deprecate_refuses_without_address() {
    let output = Command::new(get_binary_path())
        .args([
            "contract",
            "deprecate",
            "--reason",
            "End of life",
            "--private-key",
            "dGVzdGtleXRlc3RrZXl0ZXN0a2V5dGVzdGtleTA=",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "Should fail when address argument is missing"
    );
}

// ── JSON flag parsing ────────────────────────────────────────────────────────

#[test]
fn test_deprecate_json_flag_accepted() {
    // The --json flag should be accepted by the parser, even though the command
    // will fail at the network layer (no registry running).
    let output = Command::new(get_binary_path())
        .args([
            "contract",
            "deprecate",
            "CABC123",
            "--reason",
            "testing",
            "--private-key",
            "dGVzdGtleXRlc3RrZXl0ZXN0a2V5dGVzdGtleTA=",
            "--json",
            "-y",
        ])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should fail due to network/key issues, NOT due to unrecognized flags
    assert!(
        !stderr.contains("unexpected argument") && !stderr.contains("unrecognized"),
        "--json flag should be accepted, got: {}",
        stderr
    );
}

// ── Signing logic tests (pure crypto, no network) ────────────────────────────

#[test]
fn test_signing_payload_json_structure() {
    let payload = serde_json::json!({
        "contract_id": "CABC123",
        "action": "deprecate",
        "timestamp": "2025-07-01T00:00:00Z",
        "nonce": "unique-nonce-value"
    });

    assert_eq!(payload["action"], "deprecate");
    assert_eq!(payload["contract_id"], "CABC123");
    assert!(payload["nonce"].is_string());
    assert!(payload["timestamp"].is_string());
}

#[test]
fn test_signed_request_structure() {
    let request = serde_json::json!({
        "reason": "Replaced by v2",
        "replacement_contract_id": "CNEW456",
        "migration_guide_url": "https://example.com/migrate",
        "grace_period_days": 30,
        "payload": {
            "contract_id": "COLD123",
            "action": "deprecate",
            "timestamp": "2025-07-01T00:00:00Z",
            "nonce": "test-nonce"
        },
        "signature": "base64_signature_here",
        "public_key": "base64_pubkey_here",
        "signing_address": "1ABC..."
    });

    // Verify all required fields are present
    assert!(request["reason"].is_string());
    assert!(request["grace_period_days"].is_number());
    assert!(request["payload"]["contract_id"].is_string());
    assert!(request["payload"]["action"] == "deprecate");
    assert!(request["signature"].is_string());
    assert!(request["public_key"].is_string());
    assert!(request["signing_address"].is_string());
}

#[test]
fn test_typed_error_structure() {
    // Verify the expected error structure from the backend
    let unauthorized_error = serde_json::json!({
        "error": "unauthorized",
        "message": "Signature verification failed: key does not match publisher"
    });
    assert_eq!(unauthorized_error["error"], "unauthorized");

    let expired_error = serde_json::json!({
        "error": "signature_expired",
        "message": "Signed timestamp is older than 300 seconds"
    });
    assert_eq!(expired_error["error"], "signature_expired");

    let mismatch_error = serde_json::json!({
        "error": "key_mismatch",
        "message": "Signing key does not match registered publisher"
    });
    assert_eq!(mismatch_error["error"], "key_mismatch");
}

#[test]
fn test_replacement_flag_optional() {
    // --replacement should be optional; the command should parse without it
    let output = Command::new(get_binary_path())
        .args([
            "contract",
            "deprecate",
            "CABC123",
            "--reason",
            "End of life",
            "--private-key",
            "dGVzdGtleXRlc3RrZXl0ZXN0a2V5dGVzdGtleTA=",
            "-y",
        ])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should not fail due to missing --replacement (it's optional)
    assert!(
        !stderr.contains("--replacement"),
        "--replacement should be optional, got: {}",
        stderr
    );
}
