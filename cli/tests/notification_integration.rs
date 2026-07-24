use std::env;
use std::path::PathBuf;
use std::process::Command;

fn get_binary_path() -> PathBuf {
    let name = "soroban-registry";
    for var in [
        format!("CARGO_BIN_EXE_{}", name),
        "CARGO_BIN_EXE_soroban_registry".to_string(),
    ] {
        if let Ok(path) = env::var(var) {
            return PathBuf::from(path);
        }
    }
    let exe = if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let p = PathBuf::from(&manifest)
        .join("target")
        .join("debug")
        .join(&exe);
    if p.exists() {
        return p;
    }
    PathBuf::from(&manifest)
        .parent()
        .map(|d| d.join("target").join("debug").join(&exe))
        .filter(|p| p.exists())
        .unwrap_or_else(|| panic!("Binary not found. Run `cargo build` first."))
}

#[test]
fn test_notification_help() {
    let out = Command::new(get_binary_path())
        .args(["contract", "notification", "--help"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("subscribe") || stdout.contains("notification"));
}

#[test]
fn test_notification_subscribe_help() {
    let out = Command::new(get_binary_path())
        .args(["contract", "notification", "subscribe", "--help"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("address") || stdout.contains("alerts"));
}

#[test]
fn test_notification_subscribe_list_unsubscribe() {
    let addr = "CTEST_NOTIF_INTEGRATION_001";

    // Subscribe
    let sub = Command::new(get_binary_path())
        .args([
            "contract",
            "notification",
            "subscribe",
            addr,
            "--alerts",
            "updates,security",
            "--channels",
            "cli",
            "--frequency",
            "instant",
        ])
        .output()
        .unwrap();
    assert!(
        sub.status.success(),
        "subscribe failed: {}",
        String::from_utf8_lossy(&sub.stderr)
    );
    assert!(String::from_utf8_lossy(&sub.stdout).contains("Subscribed"));

    // List (JSON)
    let list = Command::new(get_binary_path())
        .args(["contract", "notification", "list", "--json"])
        .output()
        .unwrap();
    assert!(
        list.status.success(),
        "list failed: {}",
        String::from_utf8_lossy(&list.stderr)
    );
    assert!(String::from_utf8_lossy(&list.stdout).contains(addr));

    // Unsubscribe (cleanup)
    let unsub = Command::new(get_binary_path())
        .args(["contract", "notification", "unsubscribe", addr])
        .output()
        .unwrap();
    assert!(
        unsub.status.success(),
        "unsubscribe failed: {}",
        String::from_utf8_lossy(&unsub.stderr)
    );
    assert!(String::from_utf8_lossy(&unsub.stdout).contains("Unsubscribed"));
}

#[test]
fn test_notification_test_requires_subscription() {
    let out = Command::new(get_binary_path())
        .args(["contract", "notification", "test", "CNONEXISTENT_XYZ_999"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("No subscription found") || stderr.contains("subscribe"));
}

#[test]
fn test_notification_invalid_alert_type() {
    let out = Command::new(get_binary_path())
        .args([
            "contract",
            "notification",
            "subscribe",
            "CSOME_ADDR",
            "--alerts",
            "invalid_type",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Unknown alert type") || stderr.contains("invalid_type"));
}

#[test]
fn test_notification_invalid_frequency() {
    let out = Command::new(get_binary_path())
        .args([
            "contract",
            "notification",
            "subscribe",
            "CSOME_ADDR",
            "--frequency",
            "hourly",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Unknown frequency") || stderr.contains("hourly"));
}
