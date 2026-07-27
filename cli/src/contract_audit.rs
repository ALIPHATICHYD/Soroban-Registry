//! contract_audit.rs — `soroban-registry contract audit` (#1060)
//!
//! Detect drift between a local lockfile (`soroban-registry.lock.json`) and the
//! registry's current state. Reports added, removed, and changed dependencies
//! clearly, with an optional `--fix` flag to auto-sync the lockfile.
//!
//! Usage:
//!   soroban-registry contract audit [--lockfile PATH] [--fix] [--init --contracts a,b,c] [--format text|json]

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

// ── Lockfile types ───────────────────────────────────────────────────────────

/// On-disk lockfile: `soroban-registry.lock.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// ISO-8601 timestamp of last generation/sync.
    pub generated_at: String,
    /// Registry API URL used when the lockfile was generated.
    pub registry_url: String,
    /// Pinned contract entries keyed by contract ID.
    pub contracts: BTreeMap<String, LockEntry>,
}

/// A single pinned contract in the lockfile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LockEntry {
    pub contract_id: String,
    pub name: String,
    pub version: String,
    pub network: String,
    /// SHA-256 hash of the contract WASM, if available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// ISO-8601 timestamp of the last registry update.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// Contract status (e.g. active, deprecated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

// ── Drift report types ───────────────────────────────────────────────────────

/// Full drift report comparing local lockfile against registry state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    /// Contracts present in the registry but missing from the lockfile.
    pub added: Vec<DriftEntry>,
    /// Contracts present in the lockfile but missing from (or removed in) the registry.
    pub removed: Vec<DriftEntry>,
    /// Contracts whose metadata has changed between local and remote.
    pub changed: Vec<ChangedEntry>,
    /// Total number of contracts that were audited.
    pub audited: usize,
    /// Whether any drift was detected.
    pub has_drift: bool,
}

/// A contract that was added or removed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftEntry {
    pub contract_id: String,
    pub name: String,
}

/// A contract whose fields have changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedEntry {
    pub contract_id: String,
    pub name: String,
    pub changes: Vec<FieldChange>,
}

/// A single field difference between local and remote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    pub field: String,
    pub local: String,
    pub remote: String,
}

// ── Entry point ──────────────────────────────────────────────────────────────

/// Main entry point for `soroban-registry contract audit`.
pub async fn run(
    api_url: &str,
    lockfile_path: &str,
    fix: bool,
    init: bool,
    contract_ids: &[String],
    format: &str,
) -> Result<()> {
    if init {
        return run_init(api_url, lockfile_path, contract_ids).await;
    }

    let lockfile_path = Path::new(lockfile_path);
    if !lockfile_path.exists() {
        anyhow::bail!(
            "Lockfile not found: {}\n\
             Hint: run `soroban-registry contract audit --init --contracts <ID1,ID2,...>` to generate one.",
            lockfile_path.display()
        );
    }

    let lockfile = read_lockfile(lockfile_path)?;
    let report = detect_drift(api_url, &lockfile).await?;

    render_report(&report, format)?;

    if fix && report.has_drift {
        let updated = apply_fix(api_url, &lockfile).await?;
        write_lockfile(lockfile_path, &updated)?;
        println!(
            "\n{} Lockfile updated at {}",
            "[OK]".green(),
            lockfile_path.display()
        );
    }

    if report.has_drift && !fix {
        // Exit with code 1 for CI: drift detected but not fixed.
        std::process::exit(1);
    }

    Ok(())
}

// ── Init subflow ─────────────────────────────────────────────────────────────

/// Generate an initial lockfile from a list of contract IDs.
async fn run_init(api_url: &str, lockfile_path: &str, contract_ids: &[String]) -> Result<()> {
    if contract_ids.is_empty() {
        anyhow::bail!("No contract IDs provided. Use --contracts <ID1,ID2,...> with --init.");
    }

    let path = Path::new(lockfile_path);
    if path.exists() {
        anyhow::bail!(
            "Lockfile already exists at {}. Remove it first or run without --init.",
            path.display()
        );
    }

    println!(
        "{} Generating lockfile for {} contract(s)…",
        "[...]".cyan(),
        contract_ids.len()
    );

    let mut contracts = BTreeMap::new();
    let client = crate::net::client();

    for id in contract_ids {
        let id = id.trim();
        if id.is_empty() {
            continue;
        }
        match fetch_contract_meta(&client, api_url, id).await {
            Ok(entry) => {
                println!("  {} {}", "[OK]".green(), id);
                contracts.insert(id.to_string(), entry);
            }
            Err(err) => {
                eprintln!("  {} {} — {}", "[FAIL]".red(), id, err);
            }
        }
    }

    if contracts.is_empty() {
        anyhow::bail!("No contracts could be fetched. Lockfile was not created.");
    }

    let lockfile = Lockfile {
        version: 1,
        generated_at: Utc::now().to_rfc3339(),
        registry_url: api_url.to_string(),
        contracts,
    };

    write_lockfile(path, &lockfile)?;
    println!(
        "\n{} Lockfile created at {} with {} contract(s)",
        "[OK]".green(),
        path.display(),
        lockfile.contracts.len()
    );

    Ok(())
}

// ── Drift detection ──────────────────────────────────────────────────────────

/// Compare every entry in the lockfile against the registry.
async fn detect_drift(api_url: &str, lockfile: &Lockfile) -> Result<DriftReport> {
    let client = crate::net::client();
    let mut changed = Vec::new();
    let mut removed = Vec::new();

    for (id, local_entry) in &lockfile.contracts {
        match fetch_contract_meta(&client, api_url, id).await {
            Ok(remote_entry) => {
                let diffs = diff_entries(local_entry, &remote_entry);
                if !diffs.is_empty() {
                    changed.push(ChangedEntry {
                        contract_id: id.clone(),
                        name: remote_entry.name.clone(),
                        changes: diffs,
                    });
                }
            }
            Err(_) => {
                // Contract no longer found in registry — treat as removed.
                removed.push(DriftEntry {
                    contract_id: id.clone(),
                    name: local_entry.name.clone(),
                });
            }
        }
    }

    let has_drift = !changed.is_empty() || !removed.is_empty();

    Ok(DriftReport {
        added: Vec::new(), // added is only meaningful if we have a "desired" list beyond the lockfile
        removed,
        changed,
        audited: lockfile.contracts.len(),
        has_drift,
    })
}

/// Compare two `LockEntry` values field-by-field, returning a list of changes.
pub fn diff_entries(local: &LockEntry, remote: &LockEntry) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    if local.name != remote.name {
        changes.push(FieldChange {
            field: "name".to_string(),
            local: local.name.clone(),
            remote: remote.name.clone(),
        });
    }

    if local.version != remote.version {
        changes.push(FieldChange {
            field: "version".to_string(),
            local: local.version.clone(),
            remote: remote.version.clone(),
        });
    }

    if local.network != remote.network {
        changes.push(FieldChange {
            field: "network".to_string(),
            local: local.network.clone(),
            remote: remote.network.clone(),
        });
    }

    if local.hash != remote.hash {
        changes.push(FieldChange {
            field: "hash".to_string(),
            local: local.hash.clone().unwrap_or_default(),
            remote: remote.hash.clone().unwrap_or_default(),
        });
    }

    if local.status != remote.status {
        changes.push(FieldChange {
            field: "status".to_string(),
            local: local.status.clone().unwrap_or_default(),
            remote: remote.status.clone().unwrap_or_default(),
        });
    }

    if local.updated_at != remote.updated_at {
        changes.push(FieldChange {
            field: "updated_at".to_string(),
            local: local.updated_at.clone().unwrap_or_default(),
            remote: remote.updated_at.clone().unwrap_or_default(),
        });
    }

    changes
}

// ── Auto-fix ─────────────────────────────────────────────────────────────────

/// Re-fetch all contracts and produce a fresh lockfile.
async fn apply_fix(api_url: &str, lockfile: &Lockfile) -> Result<Lockfile> {
    let client = crate::net::client();
    let mut contracts = BTreeMap::new();

    for id in lockfile.contracts.keys() {
        match fetch_contract_meta(&client, api_url, id).await {
            Ok(entry) => {
                contracts.insert(id.clone(), entry);
            }
            Err(err) => {
                log::warn!("Could not refresh {}: {}. Keeping local entry.", id, err);
                if let Some(local) = lockfile.contracts.get(id) {
                    contracts.insert(id.clone(), local.clone());
                }
            }
        }
    }

    Ok(Lockfile {
        version: lockfile.version,
        generated_at: Utc::now().to_rfc3339(),
        registry_url: api_url.to_string(),
        contracts,
    })
}

// ── Registry fetch ───────────────────────────────────────────────────────────

/// Fetch contract metadata from the registry and build a `LockEntry`.
async fn fetch_contract_meta(
    client: &reqwest::Client,
    api_url: &str,
    contract_id: &str,
) -> Result<LockEntry> {
    let url = format!(
        "{}/api/contracts/{}",
        api_url.trim_end_matches('/'),
        contract_id
    );
    log::debug!("GET {}", url);

    let resp = client
        .get(&url)
        .send_with_retry()
        .await
        .with_context(|| format!("Failed to fetch contract {}", contract_id))?;

    let status = resp.status();
    if status.as_u16() == 404 {
        anyhow::bail!("contract {} not found in registry", contract_id);
    }
    if !status.is_success() {
        anyhow::bail!("registry returned {} for contract {}", status, contract_id);
    }

    let body: Value = resp
        .json()
        .await
        .with_context(|| format!("Failed to parse response for contract {}", contract_id))?;

    Ok(entry_from_api_response(contract_id, &body))
}

/// Map registry API JSON to a `LockEntry`.
pub fn entry_from_api_response(contract_id: &str, value: &Value) -> LockEntry {
    let str_field = |key: &str| -> String {
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let opt_field =
        |key: &str| -> Option<String> { value.get(key).and_then(Value::as_str).map(String::from) };

    let api_contract_id = str_field("contract_id");
    LockEntry {
        contract_id: if api_contract_id.is_empty() {
            contract_id.to_string()
        } else {
            api_contract_id
        },
        name: str_field("name"),
        version: str_field("version"),
        network: str_field("network"),
        hash: opt_field("hash").or_else(|| opt_field("wasm_hash")),
        updated_at: opt_field("updated_at"),
        status: opt_field("status"),
    }
}

// ── Lockfile I/O ─────────────────────────────────────────────────────────────

/// Read and deserialize a lockfile from disk.
pub fn read_lockfile(path: &Path) -> Result<Lockfile> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read lockfile: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse lockfile: {}", path.display()))
}

/// Serialize and write a lockfile to disk.
pub fn write_lockfile(path: &Path, lockfile: &Lockfile) -> Result<()> {
    let json = serde_json::to_string_pretty(lockfile).context("Failed to serialize lockfile")?;
    fs::write(path, json)
        .with_context(|| format!("Failed to write lockfile: {}", path.display()))?;
    Ok(())
}

// ── Report rendering ─────────────────────────────────────────────────────────

/// Render the drift report to stdout.
fn render_report(report: &DriftReport, format: &str) -> Result<()> {
    match format.to_lowercase().as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(report)?;
            println!("{}", json);
        }
        _ => {
            render_text_report(report);
        }
    }
    Ok(())
}

/// Human-readable text report.
fn render_text_report(report: &DriftReport) {
    println!(
        "\n{}\n{}",
        "Contract Drift Audit Report".bold(),
        "═".repeat(40)
    );
    println!("  Contracts audited: {}", report.audited.to_string().bold());

    if !report.has_drift {
        println!(
            "\n  {} No drift detected. Local lockfile is in sync with the registry.",
            "[OK]".green()
        );
        return;
    }

    if !report.removed.is_empty() {
        println!(
            "\n  {} {} contract(s)",
            "Removed".red().bold(),
            report.removed.len()
        );
        for entry in &report.removed {
            println!(
                "    {} {} ({})",
                "−".red(),
                entry.contract_id.cyan(),
                entry.name.dimmed()
            );
        }
    }

    if !report.added.is_empty() {
        println!(
            "\n  {} {} contract(s)",
            "Added".green().bold(),
            report.added.len()
        );
        for entry in &report.added {
            println!(
                "    {} {} ({})",
                "+".green(),
                entry.contract_id.cyan(),
                entry.name.dimmed()
            );
        }
    }

    if !report.changed.is_empty() {
        println!(
            "\n  {} {} contract(s)",
            "Changed".yellow().bold(),
            report.changed.len()
        );
        for entry in &report.changed {
            println!(
                "    {} {} ({})",
                "~".yellow(),
                entry.contract_id.cyan(),
                entry.name.dimmed()
            );
            for change in &entry.changes {
                println!(
                    "      {}: {} → {}",
                    change.field.bold(),
                    change.local.red(),
                    change.remote.green()
                );
            }
        }
    }

    let total = report.added.len() + report.removed.len() + report.changed.len();
    println!(
        "\n  {} drift(s) detected. Run with {} to auto-sync.",
        total.to_string().yellow().bold(),
        "--fix".bold()
    );
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(name: &str, version: &str, network: &str) -> LockEntry {
        LockEntry {
            contract_id: format!("C{}", name.to_uppercase()),
            name: name.to_string(),
            version: version.to_string(),
            network: network.to_string(),
            hash: Some("abc123".to_string()),
            updated_at: Some("2025-01-01T00:00:00Z".to_string()),
            status: Some("active".to_string()),
        }
    }

    #[test]
    fn diff_identical_entries_yields_no_changes() {
        let a = make_entry("token", "1.0.0", "mainnet");
        let b = a.clone();
        assert!(diff_entries(&a, &b).is_empty());
    }

    #[test]
    fn diff_detects_version_change() {
        let local = make_entry("token", "1.0.0", "mainnet");
        let mut remote = local.clone();
        remote.version = "1.1.0".to_string();

        let diffs = diff_entries(&local, &remote);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].field, "version");
        assert_eq!(diffs[0].local, "1.0.0");
        assert_eq!(diffs[0].remote, "1.1.0");
    }

    #[test]
    fn diff_detects_name_change() {
        let local = make_entry("token", "1.0.0", "mainnet");
        let mut remote = local.clone();
        remote.name = "token-v2".to_string();

        let diffs = diff_entries(&local, &remote);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].field, "name");
    }

    #[test]
    fn diff_detects_network_change() {
        let local = make_entry("token", "1.0.0", "mainnet");
        let mut remote = local.clone();
        remote.network = "testnet".to_string();

        let diffs = diff_entries(&local, &remote);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].field, "network");
    }

    #[test]
    fn diff_detects_hash_change() {
        let local = make_entry("token", "1.0.0", "mainnet");
        let mut remote = local.clone();
        remote.hash = Some("def456".to_string());

        let diffs = diff_entries(&local, &remote);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].field, "hash");
    }

    #[test]
    fn diff_detects_status_change() {
        let local = make_entry("token", "1.0.0", "mainnet");
        let mut remote = local.clone();
        remote.status = Some("deprecated".to_string());

        let diffs = diff_entries(&local, &remote);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].field, "status");
        assert_eq!(diffs[0].local, "active");
        assert_eq!(diffs[0].remote, "deprecated");
    }

    #[test]
    fn diff_detects_multiple_changes() {
        let local = make_entry("token", "1.0.0", "mainnet");
        let mut remote = local.clone();
        remote.version = "2.0.0".to_string();
        remote.name = "token-v2".to_string();
        remote.status = Some("deprecated".to_string());

        let diffs = diff_entries(&local, &remote);
        assert_eq!(diffs.len(), 3);
        let fields: Vec<&str> = diffs.iter().map(|d| d.field.as_str()).collect();
        assert!(fields.contains(&"name"));
        assert!(fields.contains(&"version"));
        assert!(fields.contains(&"status"));
    }

    #[test]
    fn diff_handles_none_hash_to_some() {
        let mut local = make_entry("token", "1.0.0", "mainnet");
        local.hash = None;
        let remote = make_entry("token", "1.0.0", "mainnet");

        let diffs = diff_entries(&local, &remote);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].field, "hash");
        assert_eq!(diffs[0].local, "");
        assert_eq!(diffs[0].remote, "abc123");
    }

    #[test]
    fn lockfile_roundtrip_serialization() {
        let mut contracts = BTreeMap::new();
        contracts.insert(
            "CABC123".to_string(),
            make_entry("token", "1.0.0", "testnet"),
        );

        let lockfile = Lockfile {
            version: 1,
            generated_at: "2025-06-01T00:00:00Z".to_string(),
            registry_url: "http://localhost:3000".to_string(),
            contracts,
        };

        let json = serde_json::to_string_pretty(&lockfile).unwrap();
        let parsed: Lockfile = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.contracts.len(), 1);
        assert_eq!(parsed.contracts["CABC123"].version, "1.0.0");
    }

    #[test]
    fn lockfile_read_write_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.lock.json");

        let mut contracts = BTreeMap::new();
        contracts.insert(
            "CTEST1".to_string(),
            make_entry("defi-pool", "2.1.0", "mainnet"),
        );
        contracts.insert(
            "CTEST2".to_string(),
            make_entry("oracle", "1.0.0", "testnet"),
        );

        let lockfile = Lockfile {
            version: 1,
            generated_at: "2025-07-01T12:00:00Z".to_string(),
            registry_url: "https://registry.example.com".to_string(),
            contracts,
        };

        write_lockfile(&path, &lockfile).unwrap();
        let loaded = read_lockfile(&path).unwrap();

        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.contracts.len(), 2);
        assert_eq!(loaded.contracts["CTEST1"].name, "defi-pool");
        assert_eq!(loaded.contracts["CTEST2"].network, "testnet");
    }

    #[test]
    fn entry_from_api_response_extracts_fields() {
        let json = serde_json::json!({
            "contract_id": "CABC123",
            "name": "My Token",
            "version": "1.2.3",
            "network": "mainnet",
            "hash": "deadbeef",
            "updated_at": "2025-06-15T10:00:00Z",
            "status": "active"
        });

        let entry = entry_from_api_response("CABC123", &json);
        assert_eq!(entry.contract_id, "CABC123");
        assert_eq!(entry.name, "My Token");
        assert_eq!(entry.version, "1.2.3");
        assert_eq!(entry.network, "mainnet");
        assert_eq!(entry.hash.as_deref(), Some("deadbeef"));
        assert_eq!(entry.status.as_deref(), Some("active"));
    }

    #[test]
    fn entry_from_api_response_uses_wasm_hash_fallback() {
        let json = serde_json::json!({
            "contract_id": "CABC123",
            "name": "Token",
            "version": "1.0.0",
            "network": "testnet",
            "wasm_hash": "fallback_hash"
        });

        let entry = entry_from_api_response("CABC123", &json);
        assert_eq!(entry.hash.as_deref(), Some("fallback_hash"));
    }

    #[test]
    fn entry_from_api_response_falls_back_to_arg_contract_id() {
        let json = serde_json::json!({
            "name": "Token",
            "version": "1.0.0",
            "network": "testnet"
        });

        let entry = entry_from_api_response("MY_ID", &json);
        assert_eq!(entry.contract_id, "MY_ID");
    }

    #[test]
    fn drift_report_no_drift() {
        let report = DriftReport {
            added: vec![],
            removed: vec![],
            changed: vec![],
            audited: 3,
            has_drift: false,
        };
        assert!(!report.has_drift);
        assert_eq!(report.audited, 3);
    }

    #[test]
    fn drift_report_with_changes() {
        let report = DriftReport {
            added: vec![],
            removed: vec![DriftEntry {
                contract_id: "CREMOVED".to_string(),
                name: "old-contract".to_string(),
            }],
            changed: vec![ChangedEntry {
                contract_id: "CCHANGED".to_string(),
                name: "my-contract".to_string(),
                changes: vec![FieldChange {
                    field: "version".to_string(),
                    local: "1.0.0".to_string(),
                    remote: "2.0.0".to_string(),
                }],
            }],
            audited: 5,
            has_drift: true,
        };
        assert!(report.has_drift);
        assert_eq!(report.removed.len(), 1);
        assert_eq!(report.changed.len(), 1);
        assert_eq!(report.changed[0].changes[0].field, "version");
    }

    #[test]
    fn drift_report_json_serialization() {
        let report = DriftReport {
            added: vec![DriftEntry {
                contract_id: "CNEW".to_string(),
                name: "new-one".to_string(),
            }],
            removed: vec![],
            changed: vec![],
            audited: 1,
            has_drift: true,
        };

        let json = serde_json::to_string(&report).unwrap();
        let parsed: DriftReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.added.len(), 1);
        assert_eq!(parsed.added[0].contract_id, "CNEW");
    }
}
