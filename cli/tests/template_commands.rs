use std::env;
use std::path::PathBuf;
use std::process::Command;

fn binary() -> PathBuf {
    let name = "soroban-registry";
    let env_candidates = [
        format!("CARGO_BIN_EXE_{}", name),
        "CARGO_BIN_EXE_soroban_registry".to_string(),
    ];
    for var in env_candidates {
        if let Ok(path) = env::var(var) {
            return PathBuf::from(path);
        }
    }

    let exe_name = if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };
    // Fallback: look for the binary in target/debug
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let binary_path = PathBuf::from(&manifest_dir)
        .join("target")
        .join("debug")
        .join(&exe_name);
    if binary_path.exists() {
        return binary_path;
    }
    // Try workspace target directory
    PathBuf::from(&manifest_dir)
        .parent()
        .map(|p| p.join("target").join("debug").join(&exe_name))
        .filter(|p| p.exists())
        .unwrap_or_else(|| panic!("Could not find {} binary. Run `cargo build` first.", name))
}

#[test]
#[ignore = "template command not yet implemented"]
fn template_list_help() {
    let out = Command::new(binary())
        .args(["template", "list", "--help"])
        .output()
        .expect("failed to run binary");

    assert!(out.status.success(), "exit status: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("list"), "expected 'list' in help output");
}

#[test]
#[ignore = "template command not yet implemented"]
fn template_clone_help() {
    let out = Command::new(binary())
        .args(["template", "clone", "--help"])
        .output()
        .expect("failed to run binary");

    assert!(out.status.success(), "exit status: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("clone"), "expected 'clone' in help output");
}

#[test]
#[ignore = "template command not yet implemented"]
fn template_list_fails_gracefully_without_api() {
    let out = Command::new(binary())
        .args(["--api-url", "http://127.0.0.1:19999", "template", "list"])
        .output()
        .expect("failed to run binary");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("Invalid network"),
        "should not be a network parse error"
    );
}

#[test]
#[ignore = "template command not yet implemented"]
fn template_clone_fails_gracefully_without_api() {
    let out = Command::new(binary())
        .args([
            "--api-url",
            "http://127.0.0.1:19999",
            "template",
            "clone",
            "token",
            "my-token",
        ])
        .output()
        .expect("failed to run binary");

    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("unexpected argument"),
        "arg parsing should succeed"
    );
}
