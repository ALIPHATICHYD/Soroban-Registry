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
fn test_test_help_includes_issue_527_flags() {
    let output = Command::new(get_binary_path())
        .arg("test")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--test-command"));
    assert!(stdout.contains("--require-coverage"));
    assert!(stdout.contains("--coverage-threshold"));
}

#[test]
fn test_test_command_runs_without_scenario_file_with_custom_command() {
    let test_command = if cfg!(windows) { "echo ok" } else { "true" };
    let output = Command::new(get_binary_path())
        .arg("test")
        .arg("--contract-path")
        .arg(".")
        .arg("--test-command")
        .arg(test_command)
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
