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
fn compare_help_includes_exit_code_and_fields() {
    let output = Command::new(get_binary_path())
        .arg("compare")
        .arg("--help")
        .output()
        .expect("run compare help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--exit-code"));
    assert!(stdout.contains("--diff"));
    assert!(stdout.contains("--fields"));
}

#[test]
fn global_no_cache_flag_is_available() {
    let output = Command::new(get_binary_path())
        .arg("--help")
        .output()
        .expect("run root help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--no-cache"));
}
