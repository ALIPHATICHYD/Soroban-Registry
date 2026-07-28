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
fn cache_status_runs_locally() {
    let output = Command::new(get_binary_path())
        .arg("cache")
        .arg("status")
        .output()
        .expect("run cache status");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Cache Status") || stdout.contains("total_entries"));
}

#[test]
fn cache_clear_accepts_disk_level() {
    let output = Command::new(get_binary_path())
        .arg("cache")
        .arg("clear")
        .arg("--level")
        .arg("disk")
        .output()
        .expect("run cache clear");

    assert!(output.status.success());
}
