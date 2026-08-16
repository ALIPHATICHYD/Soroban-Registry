use std::env;
use std::path::PathBuf;
use std::process::Command;

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

#[test]
fn describe_root_command() {
    let output = Command::new(get_binary_path())
        .arg("--describe")
        .output()
        .expect("run soroban-registry --describe");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    assert_eq!(json["version"], "1.0");
    assert_eq!(json["command"], "soroban-registry");
    assert!(json["subcommands"].as_array().unwrap().contains(&serde_json::json!("contract")));
    assert!(json["subcommands"].as_array().unwrap().contains(&serde_json::json!("completion")));
}

#[test]
fn describe_contract_verify_snapshot_nested_command() {
    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("verify-snapshot")
        .arg("--describe")
        .output()
        .expect("run soroban-registry contract verify-snapshot --describe");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    assert_eq!(json["version"], "1.0");
    assert_eq!(json["command"], "soroban-registry contract verify-snapshot");
    assert_eq!(json["arguments"]["file"]["type"], "path");
    assert_eq!(json["arguments"]["file"]["required"], true);
    assert_eq!(json["arguments"]["expect-key"]["type"], "string");
    assert_eq!(json["arguments"]["expect-key"]["required"], false);
    assert_eq!(json["arguments"]["expect-key"]["secret"], false);
    assert!(json["output"]["formats"].as_array().unwrap().contains(&serde_json::json!("json")));
}

#[test]
fn describe_secret_argument_protection() {
    let output = Command::new(get_binary_path())
        .arg("contract")
        .arg("deprecate")
        .arg("--describe")
        .output()
        .expect("run soroban-registry contract deprecate --describe");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");

    assert_eq!(json["arguments"]["private-key"]["secret"], true);
    assert_eq!(json["arguments"]["private-key"]["default_value"], serde_json::Value::Null);
}

#[test]
fn generate_artifacts_creates_and_verifies_files() {
    let temp_dir = tempfile::tempdir().expect("tempdir");

    // Generate artifacts
    let gen_output = Command::new(get_binary_path())
        .arg("generate-artifacts")
        .arg("--output-dir")
        .arg(temp_dir.path().to_str().unwrap())
        .output()
        .expect("run generate-artifacts");

    assert!(gen_output.status.success(), "generate-artifacts stderr: {}", String::from_utf8_lossy(&gen_output.stderr));
    assert!(temp_dir.path().join("schema.json").exists());
    assert!(temp_dir.path().join("completions/soroban-registry.bash").exists());
    assert!(temp_dir.path().join("completions/_soroban-registry").exists());
    assert!(temp_dir.path().join("completions/soroban-registry.fish").exists());
    assert!(temp_dir.path().join("completions/soroban-registry.ps1").exists());

    // Check artifacts (--check mode should pass)
    let check_output = Command::new(get_binary_path())
        .arg("generate-artifacts")
        .arg("--check")
        .arg("--output-dir")
        .arg(temp_dir.path().to_str().unwrap())
        .output()
        .expect("run generate-artifacts --check");

    assert!(check_output.status.success(), "generate-artifacts --check stderr: {}", String::from_utf8_lossy(&check_output.stderr));
}
