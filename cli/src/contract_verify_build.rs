//! contract_verify_build.rs — `soroban-registry contract verify-build` (#1140)
//!
//! Attempts to independently reproduce a contract's published WASM artifact
//! from source, using the recorded build-provenance metadata as a hint, and
//! compares the rebuilt artifact's SHA-256 hash against the expected hash.
//!
//! Provenance is treated as metadata, not proof: a "reproduced" verdict is
//! only ever reached by actually rebuilding and matching hashes here, never
//! by the presence or plausibility of the recorded metadata alone. This
//! command only builds source the caller already has locally; it never
//! fetches a repository and never runs on the backend.

use crate::contract_provenance::load_provenance;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;
use sha2::{Digest, Sha256};
use shared::provenance::BuildProvenance;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

const BUILD_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_CAPTURED_OUTPUT: usize = 4000;

/// Distinct outcomes the issue requires callers to be able to tell apart.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum VerifyBuildOutcome {
    /// `source_dir` does not exist or has no `Cargo.toml`.
    MissingSource { detail: String },
    /// The locally installed toolchain does not match what provenance
    /// recorded, and `--allow-toolchain-mismatch` was not passed.
    ToolchainMismatch { recorded: String, installed: String },
    /// The build command ran but did not produce a usable WASM artifact.
    BuildFailed { detail: String },
    /// The rebuild succeeded but its hash differs from the expected hash.
    HashMismatch { expected: String, actual: String },
    /// The rebuild succeeded and its hash matches the expected hash.
    Reproduced { hash: String },
}

impl VerifyBuildOutcome {
    fn is_success(&self) -> bool {
        matches!(self, VerifyBuildOutcome::Reproduced { .. })
    }
}

fn normalize_hash(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let stripped = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if stripped.len() != 64 || !stripped.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(stripped.to_ascii_lowercase())
}

fn truncate(s: &str) -> String {
    if s.len() > MAX_CAPTURED_OUTPUT {
        format!("{}... (truncated)", &s[..MAX_CAPTURED_OUTPUT])
    } else {
        s.to_string()
    }
}

async fn installed_rustc_version() -> Option<String> {
    let output = Command::new("rustc").arg("--version").output().await.ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// `rustc --version` output looks like `rustc 1.79.0 (129f3b996 2024-06-10)`;
/// provenance records just the version number (`1.79.0`). A substring check
/// avoids requiring an exact string match against the full banner.
fn toolchain_matches(recorded: &str, installed: &str) -> bool {
    installed.contains(recorded)
}

fn find_wasm_artifact(source_dir: &Path) -> Option<PathBuf> {
    let candidates = [
        source_dir.join("target/wasm32v1-none/release"),
        source_dir.join("target/wasm32-unknown-unknown/release"),
    ];
    for dir in candidates {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("wasm") {
                return Some(path);
            }
        }
    }
    None
}

async fn run_build(source_dir: &Path) -> Result<Vec<u8>, VerifyBuildOutcome> {
    let mut command = Command::new("stellar");
    command
        .args(["contract", "build"])
        .current_dir(source_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = timeout(BUILD_TIMEOUT, command.output())
        .await
        .map_err(|_| VerifyBuildOutcome::BuildFailed {
            detail: format!("Build timed out after {}s", BUILD_TIMEOUT.as_secs()),
        })?
        .map_err(|e| VerifyBuildOutcome::BuildFailed {
            detail: format!("Failed to run `stellar contract build`: {e}"),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(VerifyBuildOutcome::BuildFailed {
            detail: format!(
                "`stellar contract build` exited with {}.\nstdout: {}\nstderr: {}",
                output.status,
                truncate(&stdout),
                truncate(&stderr)
            ),
        });
    }

    let Some(wasm_path) = find_wasm_artifact(source_dir) else {
        return Err(VerifyBuildOutcome::BuildFailed {
            detail: "Build succeeded but no .wasm artifact was found under target/".to_string(),
        });
    };

    tokio::fs::read(&wasm_path)
        .await
        .map_err(|e| VerifyBuildOutcome::BuildFailed {
            detail: format!("Failed to read built artifact at {}: {e}", wasm_path.display()),
        })
}

/// `soroban-registry contract verify-build --manifest <path> --source-dir <dir> --expected-hash <hash> [--allow-toolchain-mismatch] [--json]`
pub async fn run(
    manifest_path: &str,
    source_dir: &str,
    expected_hash: &str,
    allow_toolchain_mismatch: bool,
    json: bool,
) -> Result<()> {
    log::debug!(
        "contract verify-build | manifest={} source_dir={} json={}",
        manifest_path,
        source_dir,
        json
    );

    let expected = normalize_hash(expected_hash)
        .with_context(|| format!("--expected-hash '{expected_hash}' must be a 64-character hex SHA-256 hash"))?;
    let provenance = load_provenance(manifest_path)?;

    let outcome = attempt_rebuild(&provenance, source_dir, &expected, allow_toolchain_mismatch).await;

    if json {
        println!("{}", serde_json::to_string_pretty(&outcome)?);
    } else {
        print_human(&outcome);
    }

    if outcome.is_success() {
        Ok(())
    } else {
        anyhow::bail!("verify-build did not reproduce the expected artifact")
    }
}

async fn attempt_rebuild(
    provenance: &BuildProvenance,
    source_dir: &str,
    expected_hash: &str,
    allow_toolchain_mismatch: bool,
) -> VerifyBuildOutcome {
    let dir = Path::new(source_dir);
    if !dir.join("Cargo.toml").is_file() {
        return VerifyBuildOutcome::MissingSource {
            detail: format!("No Cargo.toml found under source dir: {source_dir}"),
        };
    }

    if !allow_toolchain_mismatch {
        if let Some(recorded) = &provenance.toolchain.rustc {
            if let Some(installed) = installed_rustc_version().await {
                if !toolchain_matches(recorded, &installed) {
                    return VerifyBuildOutcome::ToolchainMismatch {
                        recorded: recorded.clone(),
                        installed,
                    };
                }
            }
        }
    }

    let wasm_bytes = match run_build(dir).await {
        Ok(bytes) => bytes,
        Err(outcome) => return outcome,
    };

    let actual = hex::encode(Sha256::digest(&wasm_bytes));
    if actual == expected_hash {
        VerifyBuildOutcome::Reproduced { hash: actual }
    } else {
        VerifyBuildOutcome::HashMismatch {
            expected: expected_hash.to_string(),
            actual,
        }
    }
}

fn print_human(outcome: &VerifyBuildOutcome) {
    println!("\n{}", "Contract Build Verification".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    match outcome {
        VerifyBuildOutcome::MissingSource { detail } => {
            println!("{} {}", "✗ Missing source:".red().bold(), detail);
        }
        VerifyBuildOutcome::ToolchainMismatch { recorded, installed } => {
            println!("{}", "✗ Toolchain mismatch".red().bold());
            println!("  Recorded:  {}", recorded);
            println!("  Installed: {}", installed);
            println!(
                "  {} pass --allow-toolchain-mismatch to build anyway.",
                "Hint:".dimmed()
            );
        }
        VerifyBuildOutcome::BuildFailed { detail } => {
            println!("{}", "✗ Build failed".red().bold());
            println!("{}", detail.dimmed());
        }
        VerifyBuildOutcome::HashMismatch { expected, actual } => {
            println!("{}", "✗ Hash mismatch".red().bold());
            println!("  Expected: {}", expected.bright_black());
            println!("  Actual:   {}", actual.bright_black());
            println!(
                "  {} the rebuild succeeded but produced a different artifact; this is metadata, not proof of tampering.",
                "Note:".dimmed()
            );
        }
        VerifyBuildOutcome::Reproduced { hash } => {
            println!("{}", "✔ Reproduced".green().bold());
            println!("  Hash: {}", hash.bright_black());
        }
    }
    println!();
}
