// Integration tests for `soroban-registry contract verify --wasm <path>`,
// the local pre-publish verification mode.
//
// Covers valid, invalid, and edge-case artifacts, plus argument validation.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn binary_path() -> PathBuf {
    for var in ["CARGO_BIN_EXE_soroban_registry", "CARGO_BIN_EXE_soroban-registry"] {
        if let Ok(path) = env::var(var) {
            return PathBuf::from(path);
        }
    }
    let mut path = env::current_dir().expect("cwd");
    path.push("target");
    path.push("debug");
    path.push("soroban-registry");
    if path.exists() {
        return path;
    }
    panic!("could not locate soroban-registry binary; run `cargo build` first");
}

/// A minimal but structurally valid WASM module that exports one function `f`.
/// (header + type + function + export + code sections)
fn wasm_with_function() -> Vec<u8> {
    vec![
        0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // header (\0asm v1)
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00, // type section: one () -> ()
        0x03, 0x02, 0x01, 0x00, // function section: one func of type 0
        0x07, 0x05, 0x01, 0x01, b'f', 0x00, 0x00, // export "f" as func 0
        0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b, // code: one empty body
    ]
}

/// Append a custom section named `name` (empty payload) to a WASM module.
fn add_custom_section(mut wasm: Vec<u8>, name: &str) -> Vec<u8> {
    let mut body = vec![name.len() as u8];
    body.extend_from_slice(name.as_bytes());
    wasm.push(0x00); // custom section id
    wasm.push(body.len() as u8);
    wasm.extend_from_slice(&body);
    wasm
}

/// A valid Soroban contract: a function-bearing module carrying the spec section.
fn soroban_contract_wasm() -> Vec<u8> {
    add_custom_section(wasm_with_function(), "contractspecv0")
}

fn write_temp(bytes: &[u8]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("contract.wasm");
    std::fs::write(&path, bytes).expect("write wasm");
    (dir, path)
}

fn run_verify_wasm(path: &std::path::Path, extra: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(binary_path());
    cmd.arg("contract").arg("verify").arg("--wasm").arg(path);
    for a in extra {
        cmd.arg(a);
    }
    cmd.output().expect("run contract verify --wasm")
}

#[test]
fn help_lists_wasm_and_verbose_flags() {
    let output = Command::new(binary_path())
        .args(["contract", "verify", "--help"])
        .output()
        .expect("run help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--wasm"), "help should mention --wasm:\n{stdout}");
    assert!(stdout.contains("--verbose"), "help should mention --verbose");
}

#[test]
fn valid_soroban_contract_passes() {
    let (_dir, path) = write_temp(&soroban_contract_wasm());
    let output = run_verify_wasm(&path, &["--json"]);
    assert!(
        output.status.success(),
        "valid contract should exit 0; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"passed\": true"), "expected passed=true:\n{stdout}");
    assert!(stdout.contains("\"is_soroban_contract\": true"));
}

#[test]
fn valid_wasm_without_spec_is_not_a_soroban_contract() {
    // Structurally valid WASM, but no contractspecv0 section → must fail with a
    // clear "not a Soroban contract" style error.
    let (_dir, path) = write_temp(&wasm_with_function());
    let output = run_verify_wasm(&path, &[]);
    assert!(!output.status.success(), "non-Soroban wasm should exit non-zero");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("contractspecv0") || combined.to_lowercase().contains("soroban contract"),
        "error should explain the missing contract spec:\n{combined}"
    );
}

#[test]
fn invalid_magic_bytes_fail() {
    let (_dir, path) = write_temp(&[0xff, 0xff, 0xff, 0xff, 0x00, 0x01]);
    let output = run_verify_wasm(&path, &["--json"]);
    assert!(!output.status.success(), "invalid WASM should exit non-zero");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"passed\": false"), "expected passed=false:\n{stdout}");
}

#[test]
fn empty_file_fails() {
    let (_dir, path) = write_temp(&[]);
    let output = run_verify_wasm(&path, &[]);
    assert!(!output.status.success(), "empty file should exit non-zero");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.to_lowercase().contains("empty"), "should mention empty file:\n{combined}");
}

#[test]
fn missing_file_fails_clearly() {
    let output = Command::new(binary_path())
        .args(["contract", "verify", "--wasm", "/no/such/contract.wasm"])
        .output()
        .expect("run");
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.to_lowercase().contains("not found"), "should report file not found:\n{combined}");
}

#[test]
fn verbose_shows_diagnostics() {
    let (_dir, path) = write_temp(&soroban_contract_wasm());
    let output = run_verify_wasm(&path, &["--verbose"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Diagnostics"), "verbose should print diagnostics:\n{stdout}");
    assert!(stdout.contains("Exported functions"), "verbose should list exports");
}

#[test]
fn requires_address_or_wasm() {
    let output = Command::new(binary_path())
        .args(["contract", "verify"])
        .output()
        .expect("run");
    assert!(!output.status.success(), "no address and no --wasm should error");
}

#[test]
fn rejects_both_address_and_wasm() {
    let (_dir, path) = write_temp(&soroban_contract_wasm());
    let output = Command::new(binary_path())
        .args(["contract", "verify", "SOMEADDRESS"])
        .arg("--wasm")
        .arg(&path)
        .output()
        .expect("run");
    assert!(!output.status.success(), "address + --wasm should be rejected");
}
