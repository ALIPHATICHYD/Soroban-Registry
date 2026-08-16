//! Integration tests for local offline contract spec inspection (#1142)

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use stellar_xdr::curr::{
    Limits, ScSpecEntry, ScSpecFunctionInputV0, ScSpecFunctionV0, ScSpecTypeDef, ScSymbol,
    StringM, VecM, WriteXdr,
};
use tempfile::tempdir;

fn get_binary_path() -> PathBuf {
    let name = "soroban-registry";
    if let Ok(path) = env::var(format!("CARGO_BIN_EXE_{}", name.replace('-', "_"))) {
        return PathBuf::from(path);
    }
    if let Ok(path) = env::var(format!("CARGO_BIN_EXE_{name}")) {
        return PathBuf::from(path);
    }
    let mut path = env::current_dir().expect("cwd");
    path.push("target/debug/soroban-registry");
    path
}

/// Helper: Minimal valid WASM binary exporting function `f`.
const MINIMAL_WASM: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // header (\0asm v1)
    0x01, 0x04, 0x01, 0x60, 0x00, 0x00, // type section: () -> ()
    0x03, 0x02, 0x01, 0x00, // function section
    0x07, 0x05, 0x01, 0x01, b'f', 0x00, 0x00, // export "f"
    0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b, // code section
];

/// Helper: append custom section to WASM module bytes.
fn append_custom_section(mut wasm: Vec<u8>, section_name: &str, data: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.push(section_name.len() as u8);
    body.extend_from_slice(section_name.as_bytes());
    body.extend_from_slice(data);

    wasm.push(0x00); // custom section identifier
    wasm.push(body.len() as u8); // section length (LEB128 single byte)
    wasm.extend_from_slice(&body);
    wasm
}

#[test]
fn inspect_spec_missing_file_returns_error() {
    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("inspect-spec")
        .arg("non_existent_file_12345.wasm")
        .arg("--json")
        .output()
        .expect("run inspect-spec");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON response");

    assert_eq!(json["status"], "invalid");
    assert_eq!(json["spec_section"]["present"], false);
    assert!(json["diagnostics"][0]["message"].as_str().unwrap().contains("not found"));
}

#[test]
fn inspect_spec_missing_spec_section() {
    let dir = tempdir().unwrap();
    let wasm_path = dir.path().join("plain.wasm");
    fs::write(&wasm_path, MINIMAL_WASM).unwrap();

    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("inspect-spec")
        .arg(wasm_path.to_str().unwrap())
        .arg("--json")
        .output()
        .expect("run inspect-spec");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON response");

    assert_eq!(json["status"], "invalid");
    assert_eq!(json["spec_section"]["present"], false);
    assert_eq!(json["spec_section"]["bytes"], 0);
    assert!(json["diagnostics"][0]["message"].as_str().unwrap().contains("Missing 'contractspecv0'"));
}

#[test]
fn inspect_spec_malformed_xdr_section() {
    let dir = tempdir().unwrap();
    let wasm_path = dir.path().join("malformed.wasm");

    let malformed_data = vec![0xFF, 0xFF, 0xFF, 0xFF, 0x12, 0x34];
    let wasm_bytes = append_custom_section(MINIMAL_WASM.to_vec(), "contractspecv0", &malformed_data);
    fs::write(&wasm_path, wasm_bytes).unwrap();

    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("inspect-spec")
        .arg(wasm_path.to_str().unwrap())
        .arg("--json")
        .output()
        .expect("run inspect-spec");

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON response");

    assert_eq!(json["status"], "invalid");
    assert_eq!(json["spec_section"]["present"], true);
    assert!(json["diagnostics"].as_array().unwrap().iter().any(|d| d["message"].as_str().unwrap().contains("Malformed XDR")));
}

#[test]
fn inspect_spec_valid_spec_section() {
    let dir = tempdir().unwrap();
    let wasm_path = dir.path().join("valid.wasm");

    let func_entry = ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
        doc: StringM::try_from("Transfer tokens").unwrap(),
        name: ScSymbol::try_from("transfer").unwrap(),
        inputs: VecM::try_from(vec![ScSpecFunctionInputV0 {
            doc: StringM::try_from("Recipient address").unwrap(),
            name: StringM::try_from("to").unwrap(),
            type_: ScSpecTypeDef::Address,
        }]).unwrap(),
        outputs: VecM::try_from(vec![ScSpecTypeDef::Bool]).unwrap(),
    });

    let spec_bytes = func_entry.to_xdr(Limits::none()).unwrap();

    let wasm_bytes = append_custom_section(MINIMAL_WASM.to_vec(), "contractspecv0", &spec_bytes);
    fs::write(&wasm_path, wasm_bytes).unwrap();

    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("inspect-spec")
        .arg(wasm_path.to_str().unwrap())
        .arg("--json")
        .output()
        .expect("run inspect-spec");

    assert!(output.status.success(), "inspect-spec stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON response");

    assert_eq!(json["status"], "valid");
    assert_eq!(json["spec_section"]["present"], true);
    assert!(json["spec_section"]["bytes"].as_u64().unwrap() > 0);
    assert_eq!(json["interface"]["algorithm"], "soroban-interface-v1");
    assert!(json["interface"]["interface_id"].is_string());
    assert_eq!(json["counts"]["functions"], 1);
    assert_eq!(json["counts"]["types"], 0);
    assert_eq!(json["counts"]["events"], 0);
    assert_eq!(json["counts"]["errors"], 0);
}
