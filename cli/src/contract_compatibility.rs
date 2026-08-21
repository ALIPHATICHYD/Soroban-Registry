//! contract_compatibility.rs — `soroban-registry contract compatibility` (#1143)
//!
//! Structurally compares the `contractspecv0` specs of two local compiled
//! WASM artifacts and classifies the differences as compatible,
//! potentially breaking, breaking, or unknown, using the shared engine in
//! `shared::contract_compatibility` (built on the #1139 spec parser so both
//! features stay consistent).
//!
//! This runs entirely offline against local artifacts, matching the
//! `contract verify --wasm` / `contract interfaces --wasm` precedent — the
//! registry does not yet expose a `--from`/`--to` version lookup endpoint.

use anyhow::{Context, Result};
use colored::Colorize;
use shared::contract_compatibility::{
    compare, ChangeCategory, CompatibilityLevel, CompatibilityReport, NetworkContext, SpecSource,
};

/// `--fail-on` threshold for `--strict` exit-code behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailOn {
    Breaking,
    Potential,
    Unknown,
}

impl FailOn {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "breaking" => Ok(FailOn::Breaking),
            "potential" | "potentially_breaking" => Ok(FailOn::Potential),
            "unknown" => Ok(FailOn::Unknown),
            other => anyhow::bail!(
                "Invalid --fail-on value '{other}': expected one of breaking, potential, unknown"
            ),
        }
    }

    fn threshold(self) -> CompatibilityLevel {
        match self {
            FailOn::Breaking => CompatibilityLevel::Breaking,
            FailOn::Potential => CompatibilityLevel::PotentiallyBreaking,
            FailOn::Unknown => CompatibilityLevel::Unknown,
        }
    }
}

fn load_spec(wasm_path: &str) -> Result<SpecSource> {
    let path = std::path::Path::new(wasm_path);
    if !path.exists() {
        anyhow::bail!("WASM file not found: {}", wasm_path);
    }
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read WASM file at {}", wasm_path))?;
    if bytes.is_empty() {
        anyhow::bail!("WASM file is empty: {}", wasm_path);
    }

    let Some(spec_bytes) = shared::wasm::extract_contract_spec_bytes(&bytes) else {
        return Ok(SpecSource::Missing);
    };

    match shared::contract_spec::parse_contract_spec(&spec_bytes) {
        Ok(entries) => Ok(SpecSource::Entries(entries)),
        Err(e) => Ok(SpecSource::Malformed(e.to_string())),
    }
}

/// `soroban-registry contract compatibility --from <wasm> --to <wasm> [--strict] [--json] [--fail-on <level>] [--from-network-passphrase <p>] [--to-network-passphrase <p>]`
#[allow(clippy::too_many_arguments)]
pub async fn run(
    from_wasm: &str,
    to_wasm: &str,
    from_network_passphrase: Option<String>,
    to_network_passphrase: Option<String>,
    strict: bool,
    json: bool,
    fail_on: FailOn,
) -> Result<()> {
    log::debug!(
        "contract compatibility | from={} to={} strict={} json={} fail_on={:?}",
        from_wasm,
        to_wasm,
        strict,
        json,
        fail_on
    );

    let from_spec = load_spec(from_wasm)?;
    let to_spec = load_spec(to_wasm)?;
    let from_net = NetworkContext {
        passphrase: from_network_passphrase,
    };
    let to_net = NetworkContext {
        passphrase: to_network_passphrase,
    };

    let report = compare(&from_spec, &to_spec, &from_net, &to_net);

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_human(from_wasm, to_wasm, &report);
    }

    if strict && report.has_at_least(fail_on.threshold()) {
        anyhow::bail!(
            "Compatibility check found changes at or above '{:?}' (strict mode enabled)",
            fail_on.threshold()
        );
    }

    Ok(())
}

fn level_label(level: CompatibilityLevel) -> colored::ColoredString {
    match level {
        CompatibilityLevel::Compatible => "compatible".green().bold(),
        CompatibilityLevel::PotentiallyBreaking => "potentially_breaking".yellow().bold(),
        CompatibilityLevel::Breaking => "breaking".red().bold(),
        CompatibilityLevel::Unknown => "unknown".bright_black().bold(),
    }
}

fn category_label(category: ChangeCategory) -> &'static str {
    match category {
        ChangeCategory::Function => "function",
        ChangeCategory::Type => "type",
        ChangeCategory::Event => "event",
        ChangeCategory::Error => "error",
        ChangeCategory::Network => "network",
    }
}

fn print_human(from_wasm: &str, to_wasm: &str, report: &CompatibilityReport) {
    println!("\n{}", "Contract Compatibility".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{:<10}{}", "From:".bold(), from_wasm);
    println!("{:<10}{}", "To:".bold(), to_wasm);
    println!("{:<10}{}", "Algorithm:".bold(), report.algorithm);
    println!("{:<10}{}", "Overall:".bold(), level_label(report.overall));

    if report.changes.is_empty() {
        println!("\n{}", "No differences detected.".green());
        return;
    }

    println!("\n{}", "Changes:".bold().underline());
    for change in &report.changes {
        println!(
            "  {} [{}] {} — {}",
            "•".cyan(),
            category_label(change.category),
            level_label(change.level),
            change.description
        );
    }
    println!();
}
