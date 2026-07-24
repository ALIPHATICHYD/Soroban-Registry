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

#[test]
fn test_list_help() {
    let output = Command::new(get_binary_path())
        .arg("list")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--network"));
    assert!(stdout.contains("--networks"));
    assert!(stdout.contains("--category"));
    assert!(stdout.contains("--limit"));
    assert!(stdout.contains("--offset"));
    assert!(stdout.contains("--format"));
}

#[test]
fn test_list_fails_gracefully_without_api() {
    let output = Command::new(get_binary_path())
        .arg("--api-url")
        .arg("http://127.0.0.1:9999")
        .arg("list")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to list contracts"));
    assert!(!stderr.contains("unexpected argument"));
}

#[test]
fn test_list_with_multiple_networks_parses_correctly() {
    let output = Command::new(get_binary_path())
        .arg("--api-url")
        .arg("http://127.0.0.1:9999")
        .arg("list")
        .arg("--networks")
        .arg("mainnet,testnet,futurenet")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to list contracts"));
    assert!(!stderr.contains("unexpected argument"));
}

#[test]
fn test_list_with_comma_separated_categories_parses_correctly() {
    let output = Command::new(get_binary_path())
        .arg("--api-url")
        .arg("http://127.0.0.1:9999")
        .arg("list")
        .arg("--category")
        .arg("DeFi,NFT")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to list contracts"));
    assert!(!stderr.contains("unexpected argument"));
}

#[test]
fn test_list_with_combined_network_and_category_filters() {
    let output = Command::new(get_binary_path())
        .arg("--api-url")
        .arg("http://127.0.0.1:9999")
        .arg("list")
        .arg("--networks")
        .arg("mainnet,testnet")
        .arg("--category")
        .arg("dex")
        .arg("--limit")
        .arg("10")
        .arg("--offset")
        .arg("20")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to list contracts"));
    assert!(!stderr.contains("unexpected argument"));
}

#[test]
fn test_list_with_global_network_flag_still_works() {
    // `list` has no subcommand-local `--network` (it would collide with the
    // global `--network` arg id and panic — see the comment on `Commands::List`).
    // A single network is instead selected via the global flag, before the
    // subcommand name.
    let output = Command::new(get_binary_path())
        .arg("--api-url")
        .arg("http://127.0.0.1:9999")
        .arg("--network")
        .arg("testnet")
        .arg("list")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to list contracts"));
    assert!(!stderr.contains("unexpected argument"));
    assert!(!stderr.contains("panicked"));
}

#[test]
fn test_list_rejects_invalid_network_filter_clearly() {
    let output = Command::new(get_binary_path())
        .arg("--api-url")
        .arg("http://127.0.0.1:9999")
        .arg("list")
        .arg("--networks")
        .arg("bogusnet")
        .output()
        .expect("Failed to execute command");

    // Invalid network values should fail locally, with a clear error, before
    // any request is attempted — not surface as a generic connection failure.
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid network"),
        "expected a clear invalid-network error, got: {stderr}"
    );
    assert!(!stderr.contains("Failed to list contracts"));
}
