use std::env;
use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> PathBuf {
    // When running tests via cargo test, CARGO_BIN_EXE_<name> is set
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
    // Fallback: look for the binary in target/debug
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let binary_path = PathBuf::from(&manifest_dir)
        .join("target")
        .join("debug")
        .join(&exe_name);
    if binary_path.exists() {
        return binary_path;
    }
    // Try workspace target directory
    PathBuf::from(&manifest_dir)
        .parent()
        .map(|p| p.join("target").join("debug").join(&exe_name))
        .filter(|p| p.exists())
        .unwrap_or_else(|| panic!("Could not find {} binary. Run `cargo build` first.", name))
}

#[test]
fn test_help_command() {
    let output = Command::new(get_binary_path())
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("soroban-registry"));
    // Verify global flag is mentioned
    assert!(stdout.contains("--network"));
}

#[test]
fn test_invalid_network_flag() {
    let output = Command::new(get_binary_path())
        .arg("--network")
        .arg("invalid_value")
        .arg("list")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The error message comes from `config::resolve_network` which calls `Network::from_str`
    // which returns "Invalid network: invalid_value. Allowed values: mainnet, testnet, futurenet"
    assert!(stderr.contains("Invalid network"));
}

#[test]
fn test_network_flag_global() {
    // This test ensures the flag is accepted globally even if command fails network call
    let output = Command::new(get_binary_path())
        .arg("--network")
        .arg("testnet")
        .arg("list")
        .arg("--limit")
        .arg("0") // minimize output/effect
        .output()
        .expect("Failed to execute command");

    // Even if it fails to connect to API, it should have parsed args successfully.
    // If API connection fails (likely in test env), output.status will be exit code 1 due to anyhow context "Failed to list contracts".
    // But stderr should NOT contain "Invalid network".
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("Invalid network"));
    assert!(!stderr.contains("unexpected argument"));
}
