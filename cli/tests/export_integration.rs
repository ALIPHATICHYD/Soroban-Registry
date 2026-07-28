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
fn test_export_help_lists_format_and_filter_flags() {
    let output = Command::new(get_binary_path())
        .arg("export")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--format"));
    assert!(stdout.contains("--filter"));
    assert!(stdout.contains("--page-size"));
    assert!(stdout.contains("json"));
    assert!(stdout.contains("csv"));
    assert!(stdout.contains("markdown"));
}

#[test]
fn test_export_rejects_invalid_format_before_api_request() {
    let output = Command::new(get_binary_path())
        .arg("--api-url")
        .arg("http://127.0.0.1:9999")
        .arg("export")
        .arg("--format")
        .arg("xml")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unsupported export format"));
}
