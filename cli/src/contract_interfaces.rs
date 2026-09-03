//! contract_interfaces.rs — `soroban-registry contract interfaces --wasm <path>` (#1139)
//!
//! Derives the deterministic `soroban-interface-v1` fingerprint for a local
//! compiled contract WASM artifact and displays its functions, types,
//! events and errors alongside their per-entry and contract-level
//! fingerprints.
//!
//! This runs entirely offline against a local artifact — it does not
//! require a registry lookup, since the registry does not yet expose
//! interface fingerprints through its API.

use anyhow::{Context, Result};
use colored::Colorize;
use shared::interface_fingerprint::{fingerprint_spec, EntryKind, InterfaceFingerprint};

/// `soroban-registry contract interfaces --wasm <path> [--json]`
pub async fn run_local(wasm_path: &str, json: bool) -> Result<()> {
    log::debug!(
        "contract interfaces (local) | wasm={} json={}",
        wasm_path,
        json
    );

    let path = std::path::Path::new(wasm_path);
    if !path.exists() {
        anyhow::bail!(
            "WASM file not found: {}\n  → Pass the path to a compiled contract, e.g. \
             target/wasm32-unknown-unknown/release/<contract>.wasm",
            wasm_path
        );
    }
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read WASM file at {}", wasm_path))?;
    if bytes.is_empty() {
        anyhow::bail!("WASM file is empty: {}", wasm_path);
    }

    let spec_bytes = shared::wasm::extract_contract_spec_bytes(&bytes).ok_or_else(|| {
        anyhow::anyhow!(
            "No {} section found: this WASM does not embed a Soroban contract spec, \
             so no interface fingerprint can be derived.",
            shared::wasm::CONTRACT_SPEC_SECTION
        )
    })?;

    let entries = shared::contract_spec::parse_contract_spec(&spec_bytes).map_err(|e| {
        anyhow::anyhow!(
            "Malformed {} section, cannot derive an interface fingerprint: {e}",
            shared::wasm::CONTRACT_SPEC_SECTION
        )
    })?;

    let fp = fingerprint_spec(&entries);

    if json {
        println!("{}", serde_json::to_string_pretty(&fp)?);
        return Ok(());
    }

    print_human(wasm_path, &fp);
    Ok(())
}

fn print_human(wasm_path: &str, fp: &InterfaceFingerprint) {
    println!("\n{}", "Contract Interface".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{:<14}{}", "File:".bold(), wasm_path);
    println!("{:<14}{}", "Algorithm:".bold(), fp.algorithm);
    println!(
        "{:<14}{}",
        "Interface ID:".bold(),
        fp.interface_id.bright_black()
    );

    print_group("Functions", EntryKind::Function, &fp.functions);
    print_group("Types", EntryKind::Type, &fp.types);
    print_group("Events", EntryKind::Event, &fp.events);
    print_group("Errors", EntryKind::Error, &fp.errors);

    println!();
}

fn print_group(
    label: &str,
    _kind: EntryKind,
    entries: &[shared::interface_fingerprint::EntryFingerprint],
) {
    println!("\n{} ({})", label.bold().underline(), entries.len());
    if entries.is_empty() {
        println!("  {}", "(none)".dimmed());
        return;
    }
    for entry in entries {
        println!("  {} {}", "•".cyan(), entry.name.bold());
        println!("      {}", entry.signature.bright_black());
        println!(
            "      {} {}",
            "fingerprint:".dimmed(),
            entry.fingerprint.dimmed()
        );
    }
}
