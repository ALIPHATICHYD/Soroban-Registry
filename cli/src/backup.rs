use crate::net::RequestBuilderExt;
use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct CreateBackupRequest {
    include_state: bool,
}

#[derive(Debug, Deserialize)]
struct ContractBackup {
    id: String,
    contract_id: String,
    backup_date: String,
    wasm_hash: String,
    storage_size_bytes: i64,
    verified: bool,
    primary_region: String,
    backup_regions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RestoreBackupRequest {
    backup_date: String,
}

#[derive(Debug, Deserialize)]
struct BackupRestoration {
    id: String,
    restore_duration_ms: i32,
    success: bool,
    restored_at: String,
}

fn validate_contract_id(contract_id: &str) -> Result<()> {
    if contract_id.trim().is_empty() {
        bail!("Contract ID cannot be empty. Provide a valid on-chain contract ID.");
    }
    Ok(())
}

fn validate_backup_date(date: &str) -> Result<()> {
    if date.trim().is_empty() {
        bail!("Backup date cannot be empty. Provide a date in YYYY-MM-DD format.");
    }
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| {
            anyhow::anyhow!(
                "Invalid backup date format: '{}'. Expected format: YYYY-MM-DD (e.g. 2026-05-31)",
                date
            )
        })
}

pub async fn create_backup(api_url: &str, contract_id: &str, include_state: bool) -> Result<()> {
    validate_contract_id(contract_id)?;
    let client = crate::net::client();
    let backup: ContractBackup = client
        .post(format!("{}/api/contracts/{}/backups", api_url, contract_id))
        .json(&CreateBackupRequest { include_state })
        .send_with_retry()
        .await?
        .json()
        .await?;

    println!("✅ Backup created successfully");
    println!("   Date: {}", backup.backup_date);
    println!("   Size: {} bytes", backup.storage_size_bytes);
    println!("   Regions: {}", backup.backup_regions.join(", "));
    Ok(())
}

pub async fn list_backups(api_url: &str, contract_id: &str) -> Result<()> {
    validate_contract_id(contract_id)?;
    let client = crate::net::client();
    let backups: Vec<ContractBackup> = client
        .get(format!("{}/api/contracts/{}/backups", api_url, contract_id))
        .send_with_retry()
        .await?
        .json()
        .await?;

    println!("📦 Contract Backups (last 30 days)");
    println!("═══════════════════════════════════════════════════════");
    for backup in backups {
        let status = if backup.verified { "✓" } else { "○" };
        println!(
            "{} {} - {} bytes - {}",
            status, backup.backup_date, backup.storage_size_bytes, backup.primary_region
        );
    }
    Ok(())
}

pub async fn restore_backup(api_url: &str, contract_id: &str, backup_date: &str) -> Result<()> {
    validate_contract_id(contract_id)?;
    validate_backup_date(backup_date)?;
    let client = crate::net::client();

    println!("Restoring backup from {}...", backup_date);

    let restoration: BackupRestoration = client
        .post(format!(
            "{}/api/contracts/{}/backups/restore",
            api_url, contract_id
        ))
        .json(&RestoreBackupRequest {
            backup_date: backup_date.to_string(),
        })
        .send_with_retry()
        .await?
        .json()
        .await?;

    if restoration.success {
        println!("Restoration completed successfully");
        println!("   Duration: {}ms", restoration.restore_duration_ms);
        println!("   Restored at: {}", restoration.restored_at);
    } else {
        bail!("Restoration failed for contract '{}' backup '{}'. The backup may be corrupted or incomplete.", contract_id, backup_date);
    }
    Ok(())
}

pub async fn verify_backup(api_url: &str, contract_id: &str, backup_date: &str) -> Result<()> {
    validate_contract_id(contract_id)?;
    validate_backup_date(backup_date)?;
    let client = crate::net::client();
    client
        .post(format!(
            "{}/api/contracts/{}/backups/{}/verify",
            api_url, contract_id, backup_date
        ))
        .send_with_retry()
        .await?;

    println!("Backup verified: {}", backup_date);
    Ok(())
}

pub async fn backup_stats(api_url: &str, contract_id: &str) -> Result<()> {
    validate_contract_id(contract_id)?;
    let client = crate::net::client();
    let stats: serde_json::Value = client
        .get(format!(
            "{}/api/contracts/{}/backups/stats",
            api_url, contract_id
        ))
        .send_with_retry()
        .await?
        .json()
        .await?;

    println!("📊 Backup Statistics");
    println!("═══════════════════════════════════════════════════════");
    println!("Total backups: {}", stats["total_backups"]);
    println!("Verified: {}", stats["verified_backups"]);
    println!("Total size: {} bytes", stats["total_size_bytes"]);
    if let Some(latest) = stats["latest_backup"].as_str() {
        println!("Latest backup: {}", latest);
    }
    Ok(())
}
