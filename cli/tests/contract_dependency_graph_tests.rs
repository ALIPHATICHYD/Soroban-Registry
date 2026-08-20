// tests/contract_dependency_graph_tests.rs
//
// Issue #1147 — CLI surface for dependency graphs and transitive risk.
//
// Black-box subprocess tests. Everything here runs without a registry: help
// text, argument parsing, and the failure path when the API is unreachable.
// The behaviour that needs a live registry -- tree shape, risk propagation,
// --fail-on exit codes against real data -- is covered by
// `backend/api/tests/dependency_graph_tests.rs`, which drives the same code
// paths through HTTP.
//
// To run:
//   cd cli && cargo test --test contract_dependency_graph_tests

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

fn run(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(get_binary_path())
        .args(args)
        .output()
        .expect("run the CLI");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// A port nothing is listening on, so the request fails fast and locally.
const UNREACHABLE: &str = "http://127.0.0.1:1";

const VALID_ADDRESS: &str = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

// ── Help and discoverability ────────────────────────────────────────────────

#[test]
fn dependencies_help_documents_the_network_flag() {
    // --network is not optional decoration: a bare address registered on more
    // than one network is a 409, and the help is where a user learns the fix.
    let (ok, stdout, _) = run(&["contract", "dependencies", "--help"]);
    assert!(ok);
    assert!(stdout.contains("--network"), "{stdout}");
    assert!(stdout.contains("--transitive"), "{stdout}");
    assert!(stdout.contains("--json"), "{stdout}");
}

#[test]
fn dependents_help_is_available() {
    let (ok, stdout, _) = run(&["contract", "dependents", "--help"]);
    assert!(ok);
    assert!(stdout.contains("--network"), "{stdout}");
}

#[test]
fn dependency_risk_help_documents_fail_on() {
    let (ok, stdout, _) = run(&["contract", "dependency-risk", "--help"]);
    assert!(ok);
    assert!(stdout.contains("--fail-on"), "{stdout}");
}

#[test]
fn the_three_commands_appear_in_the_contract_group() {
    let (ok, stdout, _) = run(&["contract", "--help"]);
    assert!(ok);
    for command in ["dependencies", "dependents", "dependency-risk"] {
        assert!(stdout.contains(command), "missing {command} in:\n{stdout}");
    }
}

#[test]
fn the_address_is_positional() {
    // Matching the other contract subcommands. An `--id` flag would be the odd
    // one out across the whole `contract` group.
    let (ok, stdout, _) = run(&["contract", "dependencies", "--help"]);
    assert!(ok);
    assert!(
        stdout.contains("<ADDRESS>"),
        "address should be positional:\n{stdout}"
    );
}

// ── Argument validation ─────────────────────────────────────────────────────

#[test]
fn a_missing_address_is_rejected() {
    let (ok, _, stderr) = run(&["contract", "dependencies"]);
    assert!(!ok);
    assert!(
        stderr.contains("required") || stderr.contains("ADDRESS"),
        "{stderr}"
    );
}

#[test]
fn an_invalid_fail_on_level_is_rejected_before_any_request() {
    // Validated client-side so a typo fails immediately with the valid values,
    // rather than after a round trip.
    let (ok, _, stderr) = run(&[
        "--api-url",
        UNREACHABLE,
        "contract",
        "dependency-risk",
        VALID_ADDRESS,
        "--fail-on",
        "catastrophic",
    ]);
    assert!(!ok);
    assert!(stderr.contains("invalid severity"), "{stderr}");
    assert!(
        stderr.contains("low, medium, high, critical"),
        "the error should name the valid values: {stderr}"
    );
}

#[test]
fn every_documented_fail_on_level_parses() {
    for level in ["low", "medium", "high", "critical", "HIGH", " critical "] {
        let (_, _, stderr) = run(&[
            "--api-url",
            UNREACHABLE,
            "contract",
            "dependency-risk",
            VALID_ADDRESS,
            "--fail-on",
            level,
        ]);
        assert!(
            !stderr.contains("invalid severity"),
            "'{level}' should parse: {stderr}"
        );
    }
}

// ── Failure paths ───────────────────────────────────────────────────────────

#[test]
fn an_unreachable_registry_fails_with_an_actionable_message() {
    let (ok, _, stderr) = run(&[
        "--api-url",
        UNREACHABLE,
        "contract",
        "dependencies",
        VALID_ADDRESS,
    ]);
    assert!(!ok);
    assert!(
        stderr.contains("registry") || stderr.contains("Network request failed"),
        "{stderr}"
    );
}

#[test]
fn an_unreachable_registry_does_not_report_a_risk_verdict() {
    // Exiting 0 on a failed request would let a CI gate pass silently when the
    // registry is down -- the exact scenario --fail-on exists to catch.
    let (ok, stdout, _) = run(&[
        "--api-url",
        UNREACHABLE,
        "contract",
        "dependency-risk",
        VALID_ADDRESS,
        "--fail-on",
        "low",
    ]);
    assert!(!ok, "a failed request must not exit 0");
    assert!(
        !stdout.contains("Overall:"),
        "no verdict should be printed: {stdout}"
    );
}
