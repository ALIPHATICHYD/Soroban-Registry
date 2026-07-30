use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

fn get_binary_path() -> PathBuf {
    let name = "soroban-registry";
    if let Ok(path) = env::var(format!("CARGO_BIN_EXE_{}", name)) {
        return PathBuf::from(path);
    }
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let binary_path = PathBuf::from(&manifest_dir)
        .join("target")
        .join("debug")
        .join(name);
    if binary_path.exists() {
        return binary_path;
    }
    PathBuf::from(&manifest_dir)
        .parent()
        .map(|p| p.join("target").join("debug").join(name))
        .filter(|p| p.exists())
        .unwrap_or_else(|| panic!("Could not find {} binary. Run `cargo build` first.", name))
}

#[test]
fn test_doctor_missing_config() {
    let dir = tempdir().unwrap();
    let output = Command::new(get_binary_path())
        .env("HOME", dir.path())
        .arg("publisher")
        .arg("doctor")
        .arg("--json")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(report["config_file_exists"].as_bool().unwrap(), false);
    assert_eq!(report["overall_status"].as_bool().unwrap(), false);
}

#[test]
fn test_doctor_missing_auth() {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".soroban-registry");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(&config_path, "[defaults]\nnetwork = \"testnet\"\n").unwrap();

    let output = Command::new(get_binary_path())
        .env("HOME", dir.path())
        .arg("publisher")
        .arg("doctor")
        .arg("--json")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(report["config_file_exists"].as_bool().unwrap(), true);
    assert_eq!(report["session_valid"].as_bool().unwrap(), false);
    assert_eq!(report["overall_status"].as_bool().unwrap(), false);
}

#[test]
fn test_doctor_missing_key_file() {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".soroban-registry");
    fs::create_dir_all(&config_dir).unwrap();
    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        "[auth]\nsession_token = \"fake_token\"\nsigning_key_path = \"/does/not/exist/key.pem\"\n",
    )
    .unwrap();

    let output = Command::new(get_binary_path())
        .env("HOME", dir.path())
        .arg("publisher")
        .arg("doctor")
        .arg("--json")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(report["config_file_exists"].as_bool().unwrap(), true);
    assert_eq!(report["session_valid"].as_bool().unwrap(), true);
    assert_eq!(report["signing_key_present"].as_bool().unwrap(), false);
    assert_eq!(report["overall_status"].as_bool().unwrap(), false);
}

#[test]
fn test_doctor_network_failure() {
    let dir = tempdir().unwrap();
    let config_dir = dir.path().join(".soroban-registry");
    fs::create_dir_all(&config_dir).unwrap();

    // Create a dummy key file
    let key_path = config_dir.join("key.pem");
    fs::write(&key_path, "dummy key").unwrap();

    let config_path = config_dir.join("config.toml");
    let conf = format!(
        "[auth]\nsession_token = \"fake_token\"\nsigning_key_path = \"{}\"\n",
        key_path.display()
    );
    fs::write(&config_path, conf).unwrap();

    let output = Command::new(get_binary_path())
        .env("HOME", dir.path())
        .arg("--api-url")
        .arg("http://127.0.0.1:0") // Guaranteed to fail connecting
        .arg("publisher")
        .arg("doctor")
        .arg("--json")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(report["config_file_exists"].as_bool().unwrap(), true);
    assert_eq!(report["session_valid"].as_bool().unwrap(), true);
    assert_eq!(report["signing_key_present"].as_bool().unwrap(), true);
    assert_eq!(report["registry_reachable"].as_bool().unwrap(), false);
    assert_eq!(report["overall_status"].as_bool().unwrap(), false);
}
