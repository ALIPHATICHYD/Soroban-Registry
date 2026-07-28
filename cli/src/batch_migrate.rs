use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use reqwest::Url;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize)]
struct MigrationReport {
    source: String,
    destination: String,
    preview: bool,
    atomic: bool,
    filter: Option<String>,
    total: usize,
    migrated: usize,
    failed: usize,
    checksum: String,
    audit_log: Vec<MigrationAuditEntry>,
}

#[derive(Debug, Serialize)]
struct MigrationAuditEntry {
    contract_id: String,
    status: String,
    message: String,
}

pub async fn run_batch_migrate(
    source: &str,
    destination: &str,
    filter: Option<&str>,
    preview: bool,
    atomic: bool,
    report_path_arg: Option<&str>,
    json_out: bool,
) -> Result<()> {
    let mut contracts = load_source(source).await?;
    if let Some(filter) = filter {
        contracts = apply_filter(contracts, filter)?;
    }
    validate_contracts(&contracts)?;

    let checksum = checksum_contracts(&contracts)?;
    if preview {
        let report_data = MigrationReport {
            source: source.to_string(),
            destination: destination.to_string(),
            preview,
            atomic,
            filter: filter.map(ToOwned::to_owned),
            total: contracts.len(),
            migrated: 0,
            failed: 0,
            checksum,
            audit_log: contracts
                .iter()
                .map(|contract| MigrationAuditEntry {
                    contract_id: contract_id(contract).unwrap_or("<unknown>").to_string(),
                    status: "preview".to_string(),
                    message: "validated for migration".to_string(),
                })
                .collect(),
        };
        write_report(&report_data, report_path(report_path_arg, None), json_out)?;
        return Ok(());
    }

    let report_data = if is_local_destination(destination) {
        write_destination_file(destination, &contracts, &checksum)?;
        MigrationReport {
            source: source.to_string(),
            destination: destination.to_string(),
            preview,
            atomic,
            filter: filter.map(ToOwned::to_owned),
            total: contracts.len(),
            migrated: contracts.len(),
            failed: 0,
            checksum,
            audit_log: contracts
                .iter()
                .map(|contract| MigrationAuditEntry {
                    contract_id: contract_id(contract).unwrap_or("<unknown>").to_string(),
                    status: "migrated".to_string(),
                    message: "written to destination file".to_string(),
                })
                .collect(),
        }
    } else {
        migrate_to_registry(destination, contracts, source, filter, atomic, checksum).await?
    };

    write_report(
        &report_data,
        report_path(report_path_arg, Some("migration-report.json")),
        json_out,
    )
}

async fn load_source(source: &str) -> Result<Vec<Value>> {
    if Path::new(source).is_file() {
        let raw = fs::read_to_string(source)
            .with_context(|| format!("Failed to read source file '{}'", source))?;
        return parse_contracts_json(&raw);
    }

    let base = source.trim_end_matches('/');
    let url = if base.ends_with("/api/contracts/export") || base.ends_with("/api/contracts") {
        base.to_string()
    } else {
        format!("{}/api/contracts/export", base)
    };

    let raw = reqwest::get(url)
        .await
        .context("Failed to fetch source registry export")?
        .error_for_status()
        .context("Source registry returned an error")?
        .text()
        .await
        .context("Failed to read source registry response")?;
    parse_contracts_json(&raw)
}

fn parse_contracts_json(raw: &str) -> Result<Vec<Value>> {
    let value: Value = serde_json::from_str(raw).context("Invalid source JSON")?;
    let contracts = value
        .get("contracts")
        .or_else(|| value.get("items"))
        .or_else(|| value.get("data"))
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .context("Source must be an array or contain contracts/items/data")?;
    Ok(contracts.clone())
}

fn apply_filter(contracts: Vec<Value>, filter: &str) -> Result<Vec<Value>> {
    let filters = filter
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            item.split_once('=')
                .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
                .with_context(|| format!("Invalid filter '{}'; expected key=value", item))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(contracts
        .into_iter()
        .filter(|contract| {
            filters.iter().all(|(key, expected)| {
                contract
                    .get(key)
                    .and_then(Value::as_str)
                    .map(|actual| actual.eq_ignore_ascii_case(expected))
                    .unwrap_or(false)
            })
        })
        .collect())
}

fn validate_contracts(contracts: &[Value]) -> Result<()> {
    for (idx, contract) in contracts.iter().enumerate() {
        let id = contract_id(contract)
            .with_context(|| format!("contract[{}] missing contract_id", idx))?;
        anyhow::ensure!(
            id.len() == 56 && id.starts_with('C') && id.chars().all(|c| c.is_ascii_alphanumeric()),
            "contract[{}] has invalid Stellar contract_id '{}'",
            idx,
            id
        );
        anyhow::ensure!(
            contract.get("name").and_then(Value::as_str).is_some(),
            "contract[{}] missing name",
            idx
        );
        anyhow::ensure!(
            contract.get("network").and_then(Value::as_str).is_some(),
            "contract[{}] missing network",
            idx
        );
    }
    Ok(())
}

fn contract_id(contract: &Value) -> Option<&str> {
    contract.get("contract_id").and_then(Value::as_str)
}

fn checksum_contracts(contracts: &[Value]) -> Result<String> {
    let bytes = serde_json::to_vec(contracts)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_local_destination(destination: &str) -> bool {
    Url::parse(destination).is_err()
        && (destination.ends_with(".json")
            || destination.ends_with(".jsonl")
            || destination.contains('/'))
}

fn write_destination_file(destination: &str, contracts: &[Value], checksum: &str) -> Result<()> {
    let envelope = json!({
        "schema_version": 1,
        "migrated_at": Utc::now().to_rfc3339(),
        "checksum": checksum,
        "contracts": contracts,
    });
    fs::write(destination, serde_json::to_vec_pretty(&envelope)?)
        .with_context(|| format!("Failed to write destination '{}'", destination))
}

async fn migrate_to_registry(
    destination: &str,
    contracts: Vec<Value>,
    source: &str,
    filter: Option<&str>,
    atomic: bool,
    checksum: String,
) -> Result<MigrationReport> {
    let endpoint = import_endpoint(destination)?;
    let payload = json!({
        "contracts": contracts,
        "fail_safe": atomic,
        "async_mode": false,
        "skip_existing": false,
    });

    let response = reqwest::Client::new()
        .post(endpoint)
        .json(&payload)
        .send()
        .await
        .context("Failed to submit registry migration")?;
    let status = response.status();
    let body: Value = response.json().await.unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        anyhow::bail!("Destination registry returned HTTP {}: {}", status, body);
    }

    let results = body
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let migrated = results
        .iter()
        .filter(|item| {
            item.get("success")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let failed = results.len().saturating_sub(migrated);

    Ok(MigrationReport {
        source: source.to_string(),
        destination: destination.to_string(),
        preview: false,
        atomic,
        filter: filter.map(ToOwned::to_owned),
        total: contracts.len(),
        migrated,
        failed,
        checksum,
        audit_log: results
            .iter()
            .map(|item| MigrationAuditEntry {
                contract_id: item
                    .get("contract_id")
                    .and_then(Value::as_str)
                    .unwrap_or("<unknown>")
                    .to_string(),
                status: if item
                    .get("success")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "migrated".to_string()
                } else {
                    "failed".to_string()
                },
                message: item
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("ok")
                    .to_string(),
            })
            .collect(),
    })
}

fn import_endpoint(destination: &str) -> Result<Url> {
    let base = destination.trim_end_matches('/');
    let url = if base.ends_with("/api/contracts/import") {
        base.to_string()
    } else {
        format!("{}/api/contracts/import", base)
    };
    Url::parse(&url).context("Invalid destination registry URL")
}

fn report_path<'a>(explicit: Option<&'a str>, default: Option<&'a str>) -> Option<&'a str> {
    explicit.or(default)
}

fn write_report(report: &MigrationReport, path: Option<&str>, json_out: bool) -> Result<()> {
    let encoded = serde_json::to_string_pretty(report)?;
    if let Some(path) = path {
        fs::write(path, &encoded).with_context(|| format!("Failed to write report '{}'", path))?;
    }

    if json_out {
        println!("{}", encoded);
        return Ok(());
    }

    println!("\n{}", "Batch Migration".bold().cyan());
    println!("Source: {}", report.source);
    println!("Destination: {}", report.destination);
    println!("Total: {}", report.total);
    println!("Migrated: {}", report.migrated.to_string().green());
    println!("Failed: {}", report.failed.to_string().red());
    println!("Checksum: {}", report.checksum.bright_black());
    if report.preview {
        println!("{}", "Preview only; no data was migrated.".yellow());
    }
    Ok(())
}
