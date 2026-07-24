#![allow(dead_code)]

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const MAX_BATCH_SIZE: usize = 50;
const BATCH_TIMEOUT_SECS: u64 = 30;

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BatchVerifyRequest {
    pub contracts: Vec<BatchContractEntry>,
    pub initiated_by: String,
}

#[derive(Debug, Serialize)]
pub struct BatchContractEntry {
    pub contract_id: String,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchVerifyResponse {
    pub batch_id: String,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped_duplicates: usize,
    pub results: Vec<ContractVerifyResult>,
    pub initiated_by: String,
    pub initiated_at: String,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractVerifyResult {
    pub contract_id: String,
    pub version: Option<String>,
    pub status: String, // "verified" | "failed" | "timeout" | "skipped"
    pub error: Option<String>,
    pub verified_at: Option<String>,
}

// ── Main batch verify command ─────────────────────────────────────────────────

/// Run a batch verification of multiple contracts.
///
/// `contracts_input` is a comma-separated list of contract IDs, optionally
/// with a version suffix separated by `@`:
///   e.g. "abc123,def456@1.0.0,ghi789"
///
/// `initiated_by` is the Stellar address or username initiating the batch.
pub async fn run_batch_verify(
    api_url: &str,
    contracts_input: &str,
    initiated_by: &str,
    json: bool,
) -> Result<()> {
    // Parse and deduplicate contract entries
    let entries = parse_and_deduplicate(contracts_input)?;

    let total_input = contracts_input
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .count();
    let deduped_count = entries.len();
    let skipped_duplicates = total_input.saturating_sub(deduped_count);

    if deduped_count == 0 {
        anyhow::bail!("No valid contract IDs provided.");
    }

    if deduped_count > MAX_BATCH_SIZE {
        anyhow::bail!(
            "Batch size {} exceeds the maximum of {}. Please split into smaller batches.",
            deduped_count,
            MAX_BATCH_SIZE
        );
    }

    println!("\n{}", "Batch Contract Verification".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("  {}: {}", "Contracts".bold(), deduped_count);
    if skipped_duplicates > 0 {
        println!(
            "  {}: {} (deduplicated)",
            "Duplicates removed".bold(),
            skipped_duplicates.to_string().yellow()
        );
    }
    println!("  {}: {}", "Initiated by".bold(), initiated_by.bright_black());
    println!("  {}: {}s total / 5s per contract", "Timeout".bold(), BATCH_TIMEOUT_SECS);
    println!();

    let request = BatchVerifyRequest {
        contracts: entries,
        initiated_by: initiated_by.to_string(),
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(BATCH_TIMEOUT_SECS))
        .build()?;

    println!("{}", "Submitting batch to registry...".bright_black());

    let response = client
        .post(format!("{}/api/contracts/batch-verify", api_url))
        .json(&request)
        .send()
        .await
        .context("Failed to reach registry API — is the server running?")?;

    if !response.status().is_success() {
        let status = response.status();
        let err = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("API error (HTTP {}): {}", status, err);
    }

    let result: BatchVerifyResponse = response
        .json()
        .await
        .context("Failed to parse batch verify response")?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    display_results(&result);

    Ok(())
}

// ── Display ───────────────────────────────────────────────────────────────────

fn display_results(result: &BatchVerifyResponse) {
    println!("\n{}", "Batch Results".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("  {}: {}", "Batch ID".bold(), result.batch_id.bright_black());
    println!("  {}: {}", "Initiated at".bold(), result.initiated_at.bright_black());
    if let Some(ms) = result.duration_ms {
        println!("  {}: {}ms", "Duration".bold(), ms);
    }

    println!();

    // Summary line
    let succeeded_str = format!("{} verified", result.succeeded).green();
    let failed_str = format!("{} failed", result.failed).red();
    let skipped_str = if result.skipped_duplicates > 0 {
        format!(", {} duplicates skipped", result.skipped_duplicates)
            .bright_black()
            .to_string()
    } else {
        String::new()
    };

    println!(
        "  {} — {}, {}{}",
        "Summary".bold(),
        succeeded_str,
        failed_str,
        skipped_str
    );

    // All-or-nothing result banner
    println!();
    if result.failed == 0 {
        println!(
            "  {} All {} contracts verified successfully!",
            "✓".green().bold(),
            result.succeeded
        );
    } else {
        println!(
            "  {} Batch rolled back — {} contract(s) failed verification.",
            "✗".red().bold(),
            result.failed
        );
        println!(
            "  {} No contracts were marked as verified (atomic transaction).",
            "↩".yellow()
        );
    }

    println!("\n{}", "Per-contract results:".bold());

    for r in &result.results {
        let status_icon = match r.status.as_str() {
            "verified" => "✓".green(),
            "failed" => "✗".red(),
            "timeout" => "⏱".yellow(),
            "skipped" => "⊘".bright_black(),
            _ => "?".bright_black(),
        };

        let version_str = r
            .version
            .as_deref()
            .map(|v| format!("@{}", v))
            .unwrap_or_default();

        println!(
            "\n  {} {}{}",
            status_icon,
            r.contract_id.bold(),
            version_str.bright_black()
        );

        match r.status.as_str() {
            "verified" => {
                if let Some(at) = &r.verified_at {
                    println!("    Verified at: {}", at.bright_black());
                }
            }
            "timeout" => {
                println!(
                    "    {}",
                    "Exceeded 5s per-contract timeout — skipped.".yellow()
                );
            }
            "failed" => {
                if let Some(err) = &r.error {
                    println!("    Error: {}", err.red());
                }
            }
            _ => {}
        }
    }

    println!("\n{}\n", "=".repeat(60).cyan());
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Parse "id1,id2@version,id3" into deduplicated BatchContractEntry list.
fn parse_and_deduplicate(input: &str) -> Result<Vec<BatchContractEntry>> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut entries: Vec<BatchContractEntry> = Vec::new();

    for raw in input.split(',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }

        // Split on optional @version suffix
        let (contract_id, version) = if let Some(idx) = raw.find('@') {
            let id = raw[..idx].trim().to_string();
            let ver = raw[idx + 1..].trim().to_string();
            (id, Some(ver))
        } else {
            (raw.to_string(), None)
        };

        if contract_id.is_empty() {
            anyhow::bail!("Empty contract ID in input: {:?}", raw);
        }

        // Deduplicate by contract_id (ignore version for dedup key)
        if seen.contains(&contract_id) {
            continue;
        }
        seen.insert(contract_id.clone());

        entries.push(BatchContractEntry {
            contract_id,
            version,
        });
    }

    Ok(entries)
}
