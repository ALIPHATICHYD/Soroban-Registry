//! Integration tests for local offline contract spec inspection (#1142)

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use stellar_xdr::curr::{
    Limits, ScSpecEntry, ScSpecEventDataFormat, ScSpecEventParamLocationV0, ScSpecEventParamV0,
    ScSpecEventV0, ScSpecFunctionInputV0, ScSpecFunctionV0, ScSpecTypeUdt,
    ScSpecUdtEnumCaseV0, ScSpecUdtEnumV0, ScSpecUdtErrorEnumCaseV0, ScSpecUdtErrorEnumV0,
    ScSpecUdtStructV0, ScSpecUdtUnionCaseTupleV0, ScSpecUdtUnionCaseV0, ScSpecUdtUnionV0,
    ScSpecTypeDef, ScSymbol, StringM, VecM, WriteXdr,
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
    // LEB128 encode the section size
    let mut size = body.len();
    loop {
        let mut byte = (size & 0x7F) as u8;
        size >>= 7;
        if size != 0 {
            byte |= 0x80;
        }
        wasm.push(byte);
        if size == 0 {
            break;
        }
    }
    wasm.extend_from_slice(&body);
    wasm
}

// ---------------------------------------------------------------------------
// Existing tests (preserved)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// New tests for issue #1142 acceptance criteria
// ---------------------------------------------------------------------------

#[test]
fn inspect_spec_duplicate_function_names() {
    let dir = tempdir().unwrap();
    let wasm_path = dir.path().join("dup_func.wasm");

    // Two functions with the same name "transfer"
    let func1 = ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
        doc: StringM::try_from("First").unwrap(),
        name: ScSymbol::try_from("transfer").unwrap(),
        inputs: VecM::default(),
        outputs: VecM::default(),
    });
    let func2 = ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
        doc: StringM::try_from("Second").unwrap(),
        name: ScSymbol::try_from("transfer").unwrap(),
        inputs: VecM::default(),
        outputs: VecM::default(),
    });

    let mut spec_bytes = func1.to_xdr(Limits::none()).unwrap();
    spec_bytes.extend_from_slice(&func2.to_xdr(Limits::none()).unwrap());

    let wasm_bytes = append_custom_section(MINIMAL_WASM.to_vec(), "contractspecv0", &spec_bytes);
    fs::write(&wasm_path, wasm_bytes).unwrap();

    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("inspect-spec")
        .arg(wasm_path.to_str().unwrap())
        .arg("--json")
        .output()
        .expect("run inspect-spec");

    assert!(!output.status.success(), "expected non-zero exit for duplicate names");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    assert_eq!(json["status"], "invalid");
    assert_eq!(json["counts"]["functions"], 2);
    assert!(
        json["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["message"].as_str().unwrap().contains("Duplicate function name")),
        "expected duplicate function name diagnostic"
    );
}

#[test]
fn inspect_spec_duplicate_type_names() {
    let dir = tempdir().unwrap();
    let wasm_path = dir.path().join("dup_type.wasm");

    // Two structs with the same name "Token"
    let struct1 = ScSpecEntry::UdtStructV0(ScSpecUdtStructV0 {
        doc: StringM::try_from("First").unwrap(),
        lib: StringM::default(),
        name: StringM::try_from("Token").unwrap(),
        fields: VecM::default(),
    });
    let struct2 = ScSpecEntry::UdtStructV0(ScSpecUdtStructV0 {
        doc: StringM::try_from("Second").unwrap(),
        lib: StringM::default(),
        name: StringM::try_from("Token").unwrap(),
        fields: VecM::default(),
    });

    let mut spec_bytes = struct1.to_xdr(Limits::none()).unwrap();
    spec_bytes.extend_from_slice(&struct2.to_xdr(Limits::none()).unwrap());

    let wasm_bytes = append_custom_section(MINIMAL_WASM.to_vec(), "contractspecv0", &spec_bytes);
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
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    assert_eq!(json["status"], "invalid");
    assert_eq!(json["counts"]["types"], 2);
    assert!(
        json["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["message"].as_str().unwrap().contains("Duplicate type name")),
        "expected duplicate type name diagnostic"
    );
}

#[test]
fn inspect_spec_unresolved_type_reference() {
    let dir = tempdir().unwrap();
    let wasm_path = dir.path().join("unresolved.wasm");

    // Function referencing undefined UDT "MissingType"
    let func = ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
        doc: StringM::try_from("Uses unknown type").unwrap(),
        name: ScSymbol::try_from("do_thing").unwrap(),
        inputs: VecM::try_from(vec![ScSpecFunctionInputV0 {
            doc: StringM::default(),
            name: StringM::try_from("val").unwrap(),
            type_: ScSpecTypeDef::Udt(ScSpecTypeUdt {
                name: StringM::try_from("MissingType").unwrap(),
            }),
        }])
        .unwrap(),
        outputs: VecM::default(),
    });

    let spec_bytes = func.to_xdr(Limits::none()).unwrap();
    let wasm_bytes = append_custom_section(MINIMAL_WASM.to_vec(), "contractspecv0", &spec_bytes);
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
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    assert_eq!(json["status"], "invalid");
    assert!(
        json["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["message"].as_str().unwrap().contains("Unresolved type reference")),
        "expected unresolved type reference diagnostic"
    );
}

#[test]
fn inspect_spec_events_and_errors_counting() {
    let dir = tempdir().unwrap();
    let wasm_path = dir.path().join("events_errors.wasm");

    let event = ScSpecEntry::EventV0(ScSpecEventV0 {
        doc: StringM::try_from("Transfer event").unwrap(),
        lib: StringM::default(),
        name: ScSymbol::try_from("Transfer").unwrap(),
        prefix_topics: VecM::default(),
        params: VecM::try_from(vec![ScSpecEventParamV0 {
            doc: StringM::default(),
            name: StringM::try_from("amount").unwrap(),
            type_: ScSpecTypeDef::U64,
            location: ScSpecEventParamLocationV0::Data,
        }])
        .unwrap(),
        data_format: ScSpecEventDataFormat::SingleValue,
    });

    let err_enum = ScSpecEntry::UdtErrorEnumV0(ScSpecUdtErrorEnumV0 {
        doc: StringM::try_from("Contract errors").unwrap(),
        lib: StringM::default(),
        name: StringM::try_from("Error").unwrap(),
        cases: VecM::try_from(vec![
            ScSpecUdtErrorEnumCaseV0 {
                doc: StringM::default(),
                name: StringM::try_from("Unauthorized").unwrap(),
                value: 1,
            },
            ScSpecUdtErrorEnumCaseV0 {
                doc: StringM::default(),
                name: StringM::try_from("NotFound").unwrap(),
                value: 2,
            },
        ])
        .unwrap(),
    });

    let mut spec_bytes = event.to_xdr(Limits::none()).unwrap();
    spec_bytes.extend_from_slice(&err_enum.to_xdr(Limits::none()).unwrap());

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
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    assert_eq!(json["status"], "valid");
    assert_eq!(json["counts"]["events"], 1);
    assert_eq!(json["counts"]["errors"], 2);
}

#[test]
fn inspect_spec_unions_and_enums_counting() {
    let dir = tempdir().unwrap();
    let wasm_path = dir.path().join("udts.wasm");

    let union = ScSpecEntry::UdtUnionV0(ScSpecUdtUnionV0 {
        doc: StringM::default(),
        lib: StringM::default(),
        name: StringM::try_from("Status").unwrap(),
        cases: VecM::try_from(vec![ScSpecUdtUnionCaseV0::TupleV0(
            ScSpecUdtUnionCaseTupleV0 {
                doc: StringM::default(),
                name: StringM::try_from("Active").unwrap(),
                type_: VecM::try_from(vec![ScSpecTypeDef::Bool]).unwrap(),
            },
        )])
        .unwrap(),
    });

    let enum_entry = ScSpecEntry::UdtEnumV0(ScSpecUdtEnumV0 {
        doc: StringM::default(),
        lib: StringM::default(),
        name: StringM::try_from("Direction").unwrap(),
        cases: VecM::try_from(vec![
            ScSpecUdtEnumCaseV0 {
                doc: StringM::default(),
                name: StringM::try_from("Up").unwrap(),
                value: 0,
            },
            ScSpecUdtEnumCaseV0 {
                doc: StringM::default(),
                name: StringM::try_from("Down").unwrap(),
                value: 1,
            },
        ])
        .unwrap(),
    });

    let mut spec_bytes = union.to_xdr(Limits::none()).unwrap();
    spec_bytes.extend_from_slice(&enum_entry.to_xdr(Limits::none()).unwrap());

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
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    assert_eq!(json["status"], "valid");
    assert_eq!(json["counts"]["types"], 2);
}

#[test]
fn inspect_spec_interface_fingerprint_is_stable() {
    let dir = tempdir().unwrap();
    let wasm_path = dir.path().join("fingerprint.wasm");

    let func = ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
        doc: StringM::try_from("Get balance").unwrap(),
        name: ScSymbol::try_from("balance").unwrap(),
        inputs: VecM::try_from(vec![ScSpecFunctionInputV0 {
            doc: StringM::default(),
            name: StringM::try_from("account").unwrap(),
            type_: ScSpecTypeDef::Address,
        }])
        .unwrap(),
        outputs: VecM::try_from(vec![ScSpecTypeDef::U64]).unwrap(),
    });

    let spec_bytes = func.to_xdr(Limits::none()).unwrap();
    let wasm_bytes = append_custom_section(MINIMAL_WASM.to_vec(), "contractspecv0", &spec_bytes);
    fs::write(&wasm_path, wasm_bytes).unwrap();

    // Run twice to verify deterministic output
    let run = || -> String {
        let output = Command::new(get_binary_path())
            .arg("contract")
            .arg("inspect-spec")
            .arg(wasm_path.to_str().unwrap())
            .arg("--json")
            .output()
            .expect("run inspect-spec");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
        json["interface"]["interface_id"]
            .as_str()
            .unwrap()
            .to_string()
    };

    let id1 = run();
    let id2 = run();
    assert_eq!(id1, id2, "interface fingerprint must be deterministic");
    assert!(id1.len() == 64, "fingerprint should be 64-char hex SHA-256");
}

#[test]
fn inspect_spec_human_readable_output() {
    let dir = tempdir().unwrap();
    let wasm_path = dir.path().join("human.wasm");

    let func = ScSpecEntry::FunctionV0(ScSpecFunctionV0 {
        doc: StringM::try_from("Transfer").unwrap(),
        name: ScSymbol::try_from("transfer").unwrap(),
        inputs: VecM::try_from(vec![ScSpecFunctionInputV0 {
            doc: StringM::default(),
            name: StringM::try_from("amount").unwrap(),
            type_: ScSpecTypeDef::U64,
        }])
        .unwrap(),
        outputs: VecM::try_from(vec![ScSpecTypeDef::Bool]).unwrap(),
    });

    let spec_bytes = func.to_xdr(Limits::none()).unwrap();
    let wasm_bytes = append_custom_section(MINIMAL_WASM.to_vec(), "contractspecv0", &spec_bytes);
    fs::write(&wasm_path, wasm_bytes).unwrap();

    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("inspect-spec")
        .arg(wasm_path.to_str().unwrap())
        .output()
        .expect("run inspect-spec");

    assert!(output.status.success(), "inspect-spec stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Human-readable output should contain key sections
    assert!(stdout.contains("Soroban Contract Specification Inspection"), "missing header");
    assert!(stdout.contains("SHA-256:"), "missing SHA-256");
    assert!(stdout.contains("VALID"), "missing VALID status");
    assert!(stdout.contains("Functions:"), "missing Functions section");
    assert!(stdout.contains("transfer"), "missing function name");
}

#[test]
fn inspect_spec_human_readable_invalid_exits_nonzero() {
    let dir = tempdir().unwrap();
    let wasm_path = dir.path().join("human_invalid.wasm");
    fs::write(&wasm_path, MINIMAL_WASM).unwrap();

    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("inspect-spec")
        .arg(wasm_path.to_str().unwrap())
        .output()
        .expect("run inspect-spec");

    assert!(!output.status.success(), "expected non-zero exit for missing spec");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("INVALID"), "should show INVALID in human-readable mode");
}

#[test]
fn inspect_spec_wasm_sha256_matches_actual_hash() {
    let dir = tempdir().unwrap();
    let wasm_path = dir.path().join("hash_check.wasm");
    fs::write(&wasm_path, MINIMAL_WASM).unwrap();

    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("inspect-spec")
        .arg(wasm_path.to_str().unwrap())
        .arg("--json")
        .output()
        .expect("run inspect-spec");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    let reported_hash = json["wasm_sha256"].as_str().expect("wasm_sha256 should be a string");
    assert_eq!(reported_hash.len(), 64, "SHA-256 should be 64 hex chars");

    // Verify independently using sha2 crate
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(MINIMAL_WASM);
    let expected = hex::encode(hasher.finalize());
    assert_eq!(reported_hash, expected, "reported hash should match computed hash");
}
