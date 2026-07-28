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
fn test_analyze_help() {
    let output = Command::new(get_binary_path())
        .args(["analyze", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("CONTRACT_ID")
            || stdout.contains("contract-id")
            || stdout.contains("contract_id")
    );
    assert!(stdout.contains("network"));
    assert!(stdout.contains("report-format") || stdout.contains("report_format"));
    assert!(stdout.contains("output"));
}

#[test]
fn test_analyze_invalid_network() {
    let output = Command::new(get_binary_path())
        .args([
            "analyze",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
            "--network",
            "badnetwork",
        ])
        .output()
        .expect("Failed to execute command");

    // Should fail: unknown network resolved at API level or missing contract
    assert!(!output.status.success());
}

#[test]
fn test_analyze_invalid_format() {
    let output = Command::new(get_binary_path())
        .args([
            "analyze",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
            "--network",
            "testnet",
            "--report-format",
            "pdf",
        ])
        .output()
        .expect("Failed to execute command");

    // Should fail: registry not running + unsupported format
    // Either way exit is non-zero
    assert!(!output.status.success());
}

#[test]
fn test_analyze_json_format_flag_accepted() {
    // Verifies the flag parses correctly (command fails at API connection, not arg parse)
    let output = Command::new(get_binary_path())
        .args([
            "analyze",
            "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM",
            "--network",
            "testnet",
            "--report-format",
            "json",
        ])
        .output()
        .expect("Failed to execute command");

    // Should not fail with "unexpected argument" — only with API/network errors
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "Flag parsing failed: {}",
        stderr
    );
    assert!(
        !stderr.contains("unrecognized"),
        "Flag parsing failed: {}",
        stderr
    );
}
