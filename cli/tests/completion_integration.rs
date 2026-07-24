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
fn completion_bash_smoke_test() {
    let output = Command::new(get_binary_path())
        .arg("completion")
        .arg("bash")
        .output()
        .expect("run completion bash");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("soroban-registry"));
    assert!(stdout.contains("_soroban-registry"));
}

#[test]
fn completion_help_lists_shells() {
    let output = Command::new(get_binary_path())
        .arg("completion")
        .arg("--help")
        .output()
        .expect("run completion help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bash"));
    assert!(stdout.contains("zsh"));
    assert!(stdout.contains("fish"));
}
