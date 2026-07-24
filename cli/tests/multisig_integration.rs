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

    let exe_name = if cfg!(windows) {
        format!("{}.exe", name_hyphen)
    } else {
        name_hyphen.to_string()
    };

    // Fallback: look in target/debug relative to the crate
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let binary_path = PathBuf::from(&manifest_dir)
        .join("target")
        .join("debug")
        .join(&exe_name);
    if binary_path.exists() {
        return binary_path;
    }

    if let Some(workspace_target) = PathBuf::from(&manifest_dir)
        .parent()
        .map(|p| p.join("target").join("debug").join(&exe_name))
        .filter(|p| p.exists())
    {
        return workspace_target;
    }

    panic!(
        "Could not find {} binary. Run `cargo build` first.",
        name_hyphen
    )
}

#[test]
fn test_multisig_help() {
    let output = Command::new(get_binary_path())
        .arg("multisig")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("create-policy"));
    assert!(stdout.contains("create-proposal"));
    assert!(stdout.contains("sign"));
    assert!(stdout.contains("execute"));
    assert!(stdout.contains("list-proposals"));
}

#[test]
fn test_create_policy_missing_args() {
    let output = Command::new(get_binary_path())
        .arg("multisig")
        .arg("create-policy")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("required arguments were not provided"));
}

#[test]
fn test_create_proposal_help() {
    let output = Command::new(get_binary_path())
        .arg("multisig")
        .arg("create-proposal")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--contract-id"));
    assert!(stdout.contains("--wasm-hash"));
    assert!(stdout.contains("--policy-id"));
}
