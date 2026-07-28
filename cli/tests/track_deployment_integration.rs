use std::env;
use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> PathBuf {
    let name = "soroban-registry";
    let env_candidates = [
        format!("CARGO_BIN_EXE_{}", name),
        "CARGO_BIN_EXE_soroban_registry".to_string(),
    ];
    for var in env_candidates {
        if let Ok(path) = env::var(var) {
            return PathBuf::from(path);
        }
    }

    let exe_name = if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let binary_path = PathBuf::from(&manifest_dir)
        .join("target")
        .join("debug")
        .join(&exe_name);
    if binary_path.exists() {
        return binary_path;
    }
    PathBuf::from(&manifest_dir)
        .parent()
        .map(|p| p.join("target").join("debug").join(&exe_name))
        .filter(|p| p.exists())
        .unwrap_or_else(|| panic!("Could not find {} binary. Run `cargo build` first.", name))
}

#[test]
fn test_track_deployment_help() {
    let output = Command::new(get_binary_path())
        .args(["track-deployment", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("contract-id"));
    assert!(stdout.contains("network"));
    assert!(stdout.contains("wait-timeout"));
    assert!(stdout.contains("tx-hash"));
}

#[test]
fn test_track_deployment_invalid_network() {
    // Should fail immediately with an unknown network — no polling occurs.
    let output = Command::new(get_binary_path())
        .args([
            "track-deployment",
            "--contract-id",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
            "--network",
            "badnetwork",
            "--wait-timeout",
            "1",
        ])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Unknown network") || stderr.contains("badnetwork"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn test_track_deployment_timeout_exits_code_2() {
    // Use a non-existent contract on testnet with a very short timeout.
    // The registry API is not running in CI, so the poll will find nothing
    // and the timeout should trigger exit code 2.
    let output = Command::new(get_binary_path())
        .args([
            "track-deployment",
            "--contract-id",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
            "--network",
            "testnet",
            "--wait-timeout",
            "6", // just over one poll interval (5s)
        ])
        .output()
        .expect("Failed to execute command");

    // Exit code 2 = timeout (process::exit(2) in track_deployment.rs)
    assert_eq!(
        output.status.code(),
        Some(2),
        "Expected exit code 2 on timeout, got {:?}",
        output.status.code()
    );
}

#[test]
fn test_track_deployment_timeout_json_output() {
    let output = Command::new(get_binary_path())
        .args([
            "track-deployment",
            "--contract-id",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
            "--network",
            "testnet",
            "--wait-timeout",
            "6",
            "--json",
        ])
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(2));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON on timeout");

    assert_eq!(parsed["status"], "timeout");
    assert_eq!(parsed["network"], "testnet");
    assert_eq!(
        parsed["contract_id"],
        "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM"
    );
    assert_eq!(parsed["registered_in_registry"], false);
}
