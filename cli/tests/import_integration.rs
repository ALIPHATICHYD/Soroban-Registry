use std::env;
use std::fs;
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
fn test_contract_import_help() {
    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("import")
        .arg("--help")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--format") || stdout.contains("-f"));
    assert!(stdout.contains("--on-duplicate"));
    assert!(stdout.contains("--network-map"));
    assert!(stdout.contains("--dry-run"));
    assert!(stdout.contains("--validate"));
    assert!(stdout.contains("--atomic"));
}

#[test]
fn test_contract_import_validation_empty_fields() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("invalid_contracts.json");

    // contract_id is empty, network is invalid
    let bad_json = r#"[
        {
            "contract_id": "",
            "name": "Invalid Contract",
            "network": "unknownnet",
            "publisher_address": "Unknown"
        }
    ]"#;
    fs::write(&file_path, bad_json).unwrap();

    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("import")
        .arg(file_path.to_str().unwrap())
        .arg("--validate")
        .arg("--dry-run")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Validation failed"));
    assert!(stderr.contains("contract_id is empty") || stderr.contains("invalid network"));
}

#[test]
fn test_contract_import_dry_run_json() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("valid_contracts.json");

    let good_json = r#"[
        {
            "contract_id": "CDD7D...somevalidid",
            "name": "MyContract",
            "network": "testnet",
            "publisher_address": "GD3P..."
        }
    ]"#;
    fs::write(&file_path, good_json).unwrap();

    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("import")
        .arg(file_path.to_str().unwrap())
        .arg("--dry-run")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dry-run complete"));
    assert!(stdout.contains("CDD7D...somevalidid"));
}

#[test]
fn test_contract_import_dry_run_jsonl() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("valid_contracts.jsonl");

    let good_jsonl = "{\"contract_id\": \"CDD7D...somevalidid\", \"name\": \"MyContract\", \"network\": \"testnet\", \"publisher_address\": \"GD3P...\"}\n";
    fs::write(&file_path, good_jsonl).unwrap();

    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("import")
        .arg(file_path.to_str().unwrap())
        .arg("--dry-run")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dry-run complete"));
    assert!(stdout.contains("CDD7D...somevalidid"));
}

#[test]
fn test_contract_import_dry_run_csv() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("valid_contracts.csv");

    let csv_content = "contract_id,name,network,publisher_address\nCDD7D...somevalidid,MyContract,testnet,GD3P...\n";
    fs::write(&file_path, csv_content).unwrap();

    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("import")
        .arg(file_path.to_str().unwrap())
        .arg("--dry-run")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Dry-run complete"));
    assert!(stdout.contains("CDD7D...somevalidid"));
}
