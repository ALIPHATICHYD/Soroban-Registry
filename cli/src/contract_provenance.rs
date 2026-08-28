//! contract_provenance.rs — `soroban-registry contract provenance` (#1140)
//!
//! Displays build-provenance metadata (source repo/commit, toolchain
//! versions, pinned build environment, reproducibility status) recorded for
//! a contract, read from a local manifest file. The registry does not yet
//! expose provenance through a lookup-by-contract-ID endpoint, so this
//! mirrors the local-artifact precedent set by `contract interfaces --wasm`
//! and `contract compatibility --from/--to`.

use anyhow::{Context, Result};
use colored::Colorize;
use shared::provenance::{validate_provenance, BuildProvenance, ReproducibilityStatus};

/// `soroban-registry contract provenance --manifest <path> [--json]`
pub async fn run_local(manifest_path: &str, json: bool) -> Result<()> {
    log::debug!(
        "contract provenance (local) | manifest={} json={}",
        manifest_path,
        json
    );

    let provenance = load_provenance(manifest_path)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&provenance)?);
        return Ok(());
    }

    print_human(manifest_path, &provenance);
    Ok(())
}

/// Read and validate a provenance manifest file. Shared with
/// `contract_verify_build`.
pub fn load_provenance(manifest_path: &str) -> Result<BuildProvenance> {
    let path = std::path::Path::new(manifest_path);
    if !path.exists() {
        anyhow::bail!("Provenance manifest not found: {}", manifest_path);
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read provenance manifest at {}", manifest_path))?;
    let provenance: BuildProvenance = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse provenance manifest at {}", manifest_path))?;

    let errors = validate_provenance(&provenance);
    if !errors.is_empty() {
        anyhow::bail!(
            "Provenance manifest at {} is invalid:\n{}",
            manifest_path,
            errors
                .iter()
                .map(|e| format!("  - {e}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    Ok(provenance)
}

fn print_human(manifest_path: &str, provenance: &BuildProvenance) {
    println!("\n{}", "Contract Build Provenance".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{:<10}{}", "Manifest:".bold(), manifest_path);

    println!("\n{}", "Source".bold().underline());
    print_field("Repository", provenance.source.repository.as_deref());
    print_field("Commit", provenance.source.commit.as_deref());

    println!("\n{}", "Toolchain".bold().underline());
    print_field("rustc", provenance.toolchain.rustc.as_deref());
    print_field("soroban-sdk", provenance.toolchain.soroban_sdk.as_deref());
    print_field("stellar-cli", provenance.toolchain.stellar_cli.as_deref());
    print_field("target", provenance.toolchain.target.as_deref());

    println!("\n{}", "Dependencies".bold().underline());
    print_field(
        "Lockfile SHA-256",
        provenance.dependencies.lockfile_sha256.as_deref(),
    );

    println!("\n{}", "Build Environment".bold().underline());
    print_field("Image", provenance.build_environment.image.as_deref());

    println!("\n{}", "Reproducibility".bold().underline());
    let status_label = match provenance.reproducibility.status {
        ReproducibilityStatus::NotChecked => "not checked".dimmed().bold(),
        ReproducibilityStatus::Reproduced => "reproduced".green().bold(),
        ReproducibilityStatus::Mismatched => "mismatched".red().bold(),
        ReproducibilityStatus::BuildFailed => "build failed".red().bold(),
    };
    println!("  Status: {}", status_label);
    if matches!(
        provenance.reproducibility.status,
        ReproducibilityStatus::NotChecked
    ) {
        println!(
            "  {} Run `soroban-registry contract verify-build` to attempt an independent rebuild.",
            "Hint:".dimmed()
        );
    }
    println!();
}

fn print_field(label: &str, value: Option<&str>) {
    match value {
        Some(v) => println!("  {:<18}{}", format!("{label}:"), v),
        None => println!("  {:<18}{}", format!("{label}:"), "(not recorded)".dimmed()),
    }
}
