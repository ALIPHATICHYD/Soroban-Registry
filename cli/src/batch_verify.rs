#![allow(dead_code)]

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use colored::Colorize;
use csv::Writer;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tokio::task::JoinSet;
use uuid::Uuid;

const MAX_BATCH_SIZE: usize = 50;
const BATCH_TIMEOUT_SECS: u64 = 30;
const CHUNK_CONCURRENCY: usize = 4;

// ── Public entry point ─────────────────────────────────────────────────────────

pub struct BatchVerifyArgs<'a> {
    pub api_url: &'a str,
    pub file: Option<&'a str>,
    pub contracts: Option<&'a str>,
    pub network: Option<&'a str>,
    pub category: Option<&'a str>,
    pub age: Option<u32>,
    pub initiated_by: &'a str,
    pub level: &'a str,
    pub export: Option<&'a str>,
    pub output: Option<&'a str>,
    pub schedule: Option<&'a str>,
    pub json: bool,
}

// ── Request types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
pub struct BatchContractEntry {
    pub contract_id: String,
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compiler_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
}

#[derive(Debug, Serialize)]
struct BatchVerifyRequest {
    contracts: Vec<BatchContractEntry>,
    initiated_by: String,
}

// ── Response types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct BackendBatchResponse {
    pub total: usize,
    pub verified: usize,
    pub failed: usize,
    pub cached: usize,
    pub results: Vec<serde_json::Value>,
}

impl Default for BackendBatchResponse {
    fn default() -> Self {
        Self {
            total: 0,
            verified: 0,
            failed: 0,
            cached: 0,
            results: Vec::new(),
        }
    }
}

// ── Report types ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BatchVerifyReport {
    pub batch_id: String,
    pub generated_at: String,
    pub level: String,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub cached: usize,
    pub skipped_duplicates: usize,
    pub duration_ms: Option<u64>,
    pub results: Vec<ContractVerifyResult>,
}

#[derive(Debug, Serialize)]
pub struct ContractVerifyResult {
    pub contract_id: String,
    pub version: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub verified_at: Option<String>,
    pub level: Option<String>,
}

// ── Manifest types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct VerifyManifest {
    pub contracts: Vec<VerifyManifestEntry>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyManifestEntry {
    pub contract_id: String,
    pub version: Option<String>,
    pub source_code: Option<String>,
    pub compiler_version: Option<String>,
}

// ── Schedule types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SchedulesConfig {
    pub schedules: Vec<ScheduleEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub name: String,
    pub cron: String,
    pub command: String,
    pub created_at: String,
}

// ── Main entry point ───────────────────────────────────────────────────────────

pub async fn run_batch_verify(args: BatchVerifyArgs<'_>) -> Result<()> {
    validate_level(args.level)?;

    let start = std::time::Instant::now();

    let (mut entries, skipped_dups) = if let Some(file) = args.file {
        let raw = load_ids_from_file(file)?;
        deduplicate_entries(raw)
    } else if let Some(contracts) = args.contracts {
        let raw = parse_and_deduplicate_csv(contracts)?;
        (raw, 0)
    } else if args.network.is_some() || args.category.is_some() || args.age.is_some() {
        let raw = fetch_ids_by_filter(args.api_url, args.network, args.category, args.age).await?;
        deduplicate_entries(raw)
    } else {
        anyhow::bail!(
            "Provide --contracts, --file, or filter flags (--network, --category, --age)"
        );
    };

    if entries.is_empty() {
        anyhow::bail!("No valid contract IDs provided.");
    }

    apply_level(&mut entries, args.level);

    println!("\n{}", "Batch Contract Verification".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("  {}: {}", "Contracts".bold(), entries.len());
    println!("  {}: {}", "Level".bold(), args.level.bright_yellow());
    if skipped_dups > 0 {
        println!(
            "  {}: {} (deduplicated)",
            "Duplicates removed".bold(),
            skipped_dups.to_string().yellow()
        );
    }
    println!(
        "  {}: {}",
        "Initiated by".bold(),
        args.initiated_by.bright_black()
    );
    println!();

    if let Some(cron) = args.schedule {
        let command_repr = build_command_repr(&args);
        save_schedule(cron, &command_repr)?;
        println!(
            "  {}: {}",
            "Schedule saved".bold().green(),
            format_crontab_entry(cron, &command_repr)
        );
        println!();
    }

    println!("{}", "Dispatching batch to registry...".bright_black());

    let response = dispatch_chunks(args.api_url, entries, args.initiated_by).await?;
    let duration_ms = start.elapsed().as_millis() as u64;

    let report = build_report(&response, args.level, skipped_dups, Some(duration_ms));

    if let Some(output_path) = args.output {
        save_text_report(&report, output_path)?;
        println!(
            "  {} Report saved to {}",
            "✓".green(),
            output_path.bright_black()
        );
    }

    if let Some(export_path) = args.export {
        export_report(&report, export_path)?;
        println!(
            "  {} Export saved to {}",
            "✓".green(),
            export_path.bright_black()
        );
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        display_results(&response);
        display_statistics(&report);
    }

    if report.failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

// ── Input loading ──────────────────────────────────────────────────────────────

fn load_ids_from_file(path: &str) -> Result<Vec<BatchContractEntry>> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read file: {}", path))?;
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "json" => parse_json_manifest(&content),
        "yaml" | "yml" => parse_yaml_manifest(&content),
        _ => parse_plain_text(&content),
    }
}

fn parse_plain_text(content: &str) -> Result<Vec<BatchContractEntry>> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (contract_id, version) = if let Some(idx) = line.find('@') {
            (
                line[..idx].trim().to_string(),
                Some(line[idx + 1..].trim().to_string()),
            )
        } else {
            (line.to_string(), None)
        };
        if contract_id.is_empty() {
            continue;
        }
        entries.push(BatchContractEntry {
            contract_id,
            version,
            source_code: None,
            compiler_version: None,
            level: None,
        });
    }
    Ok(entries)
}

fn parse_json_manifest(content: &str) -> Result<Vec<BatchContractEntry>> {
    let manifest: VerifyManifest =
        serde_json::from_str(content).context("Failed to parse JSON manifest")?;
    Ok(manifest_to_entries(manifest))
}

fn parse_yaml_manifest(content: &str) -> Result<Vec<BatchContractEntry>> {
    let manifest: VerifyManifest =
        serde_yaml::from_str(content).context("Failed to parse YAML manifest")?;
    Ok(manifest_to_entries(manifest))
}

fn manifest_to_entries(manifest: VerifyManifest) -> Vec<BatchContractEntry> {
    manifest
        .contracts
        .into_iter()
        .map(|e| BatchContractEntry {
            contract_id: e.contract_id,
            version: e.version,
            source_code: e.source_code,
            compiler_version: e.compiler_version,
            level: None,
        })
        .collect()
}

fn parse_and_deduplicate_csv(input: &str) -> Result<Vec<BatchContractEntry>> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut entries: Vec<BatchContractEntry> = Vec::new();
    for raw in input.split(',') {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let (contract_id, version) = if let Some(idx) = raw.find('@') {
            (
                raw[..idx].trim().to_string(),
                Some(raw[idx + 1..].trim().to_string()),
            )
        } else {
            (raw.to_string(), None)
        };
        if contract_id.is_empty() {
            anyhow::bail!("Empty contract ID in input: {:?}", raw);
        }
        if seen.contains(&contract_id) {
            continue;
        }
        seen.insert(contract_id.clone());
        entries.push(BatchContractEntry {
            contract_id,
            version,
            source_code: None,
            compiler_version: None,
            level: None,
        });
    }
    Ok(entries)
}

// ── Filter-based discovery ─────────────────────────────────────────────────────

async fn fetch_ids_by_filter(
    api_url: &str,
    network: Option<&str>,
    category: Option<&str>,
    age_days: Option<u32>,
) -> Result<Vec<BatchContractEntry>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut all_entries: Vec<BatchContractEntry> = Vec::new();
    let mut page = 1i64;

    loop {
        let mut url = format!("{}/api/contracts?limit=100&page={}", api_url, page);
        if let Some(n) = network {
            url.push_str(&format!("&network={}", n));
        }
        if let Some(c) = category {
            url.push_str(&format!("&category={}", c));
        }
        if let Some(days) = age_days {
            let created_from = (Utc::now() - Duration::days(days as i64)).to_rfc3339();
            url.push_str(&format!("&created_from={}", created_from));
        }

        let response = client
            .get(&url)
            .send_with_retry()
            .await
            .context("Failed to fetch contracts from API")?;

        if !response.status().is_success() {
            let status = response.status();
            let err = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("API error fetching contracts (HTTP {}): {}", status, err);
        }

        let body: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse contracts list response")?;

        let items = body
            .get("items")
            .or_else(|| body.get("contracts"))
            .or_else(|| body.get("data"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if items.is_empty() {
            break;
        }

        let total_pages = body.get("pages").and_then(|v| v.as_i64()).unwrap_or(1);

        for item in &items {
            let contract_id = item
                .get("contract_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if !contract_id.is_empty() {
                all_entries.push(BatchContractEntry {
                    contract_id,
                    version: None,
                    source_code: None,
                    compiler_version: None,
                    level: None,
                });
            }
        }

        if page >= total_pages {
            break;
        }
        page += 1;
    }

    if all_entries.is_empty() {
        anyhow::bail!("No contracts found matching the given filters.");
    }

    Ok(all_entries)
}

// ── Parallel chunked dispatch ──────────────────────────────────────────────────

async fn dispatch_chunks(
    api_url: &str,
    entries: Vec<BatchContractEntry>,
    initiated_by: &str,
) -> Result<BackendBatchResponse> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(BATCH_TIMEOUT_SECS))
        .build()?;

    let chunks: Vec<Vec<BatchContractEntry>> =
        entries.chunks(MAX_BATCH_SIZE).map(|c| c.to_vec()).collect();

    let total_chunks = chunks.len();
    if total_chunks > 1 {
        println!(
            "  {} {} chunks of up to {} contracts",
            "Dispatching".bold(),
            total_chunks,
            MAX_BATCH_SIZE
        );
    }

    let mut set: JoinSet<Result<BackendBatchResponse>> = JoinSet::new();
    let mut merged = BackendBatchResponse::default();
    let mut chunks_iter = chunks.into_iter();

    // Pre-fill up to CHUNK_CONCURRENCY tasks
    for chunk in chunks_iter.by_ref().take(CHUNK_CONCURRENCY) {
        let c = client.clone();
        let url = api_url.to_string();
        let initiator = initiated_by.to_string();
        set.spawn(async move { send_chunk(&c, &url, chunk, &initiator).await });
    }

    // Drain completed tasks and refill
    while let Some(result) = set.join_next().await {
        let resp = result.context("Chunk task panicked")??;
        merged.total += resp.total;
        merged.verified += resp.verified;
        merged.failed += resp.failed;
        merged.cached += resp.cached;
        merged.results.extend(resp.results);

        if let Some(chunk) = chunks_iter.next() {
            let c = client.clone();
            let url = api_url.to_string();
            let initiator = initiated_by.to_string();
            set.spawn(async move { send_chunk(&c, &url, chunk, &initiator).await });
        }
    }

    Ok(merged)
}

async fn send_chunk(
    client: &reqwest::Client,
    api_url: &str,
    chunk: Vec<BatchContractEntry>,
    initiated_by: &str,
) -> Result<BackendBatchResponse> {
    let request = BatchVerifyRequest {
        contracts: chunk,
        initiated_by: initiated_by.to_string(),
    };

    let response = client
        .post(format!("{}/api/contracts/batch-verify", api_url))
        .json(&request)
        .send_with_retry()
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

    response
        .json::<BackendBatchResponse>()
        .await
        .context("Failed to parse batch verify response")
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn deduplicate_entries(entries: Vec<BatchContractEntry>) -> (Vec<BatchContractEntry>, usize) {
    let mut seen = HashSet::new();
    let total = entries.len();
    let deduped: Vec<BatchContractEntry> = entries
        .into_iter()
        .filter(|e| seen.insert(e.contract_id.clone()))
        .collect();
    let skipped = total - deduped.len();
    (deduped, skipped)
}

fn apply_level(entries: &mut [BatchContractEntry], level: &str) {
    for e in entries.iter_mut() {
        e.level = Some(level.to_string());
    }
}

fn validate_level(level: &str) -> Result<()> {
    match level {
        "basic" | "standard" | "strict" => Ok(()),
        _ => anyhow::bail!(
            "Invalid verification level '{}'. Must be one of: basic, standard, strict",
            level
        ),
    }
}

// ── Report building ────────────────────────────────────────────────────────────

fn build_report(
    response: &BackendBatchResponse,
    level: &str,
    skipped_dups: usize,
    duration_ms: Option<u64>,
) -> BatchVerifyReport {
    let results: Vec<ContractVerifyResult> = response
        .results
        .iter()
        .map(|v| {
            let contract_id = v
                .get("contract_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let verified = v.get("verified").and_then(|v| v.as_bool()).unwrap_or(false);
            let status = if verified { "verified" } else { "failed" }.to_string();
            let error = v
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            ContractVerifyResult {
                contract_id,
                version: None,
                status,
                error,
                verified_at: None,
                level: Some(level.to_string()),
            }
        })
        .collect();

    BatchVerifyReport {
        batch_id: Uuid::new_v4().to_string(),
        generated_at: Utc::now().to_rfc3339(),
        level: level.to_string(),
        total: response.total,
        succeeded: response.verified,
        failed: response.failed,
        cached: response.cached,
        skipped_duplicates: skipped_dups,
        duration_ms,
        results,
    }
}

// ── Display ────────────────────────────────────────────────────────────────────

fn display_results(response: &BackendBatchResponse) {
    println!("\n{}", "Per-contract results:".bold());

    for r in &response.results {
        let contract_id = r
            .get("contract_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let verified = r.get("verified").and_then(|v| v.as_bool()).unwrap_or(false);
        let error = r.get("error").and_then(|v| v.as_str());

        let status_icon = if verified { "✓".green() } else { "✗".red() };

        println!("\n  {} {}", status_icon, contract_id.bold());

        if !verified {
            if let Some(err) = error {
                println!("    Error: {}", err.red());
            }
        }
    }
    println!();
}

fn display_statistics(report: &BatchVerifyReport) {
    println!("{}", "Verification Statistics".bold().cyan());
    println!("{}", "=======================".cyan());

    let pct_verified = if report.total > 0 {
        format!(
            " ({:.1}%)",
            report.succeeded as f64 / report.total as f64 * 100.0
        )
    } else {
        String::new()
    };
    let pct_failed = if report.total > 0 {
        format!(
            " ({:.1}%)",
            report.failed as f64 / report.total as f64 * 100.0
        )
    } else {
        String::new()
    };

    println!("  {}: {}", "Total".bold(), report.total);
    println!(
        "  {}: {}{}",
        "Verified".bold(),
        report.succeeded.to_string().green(),
        pct_verified.green()
    );
    println!(
        "  {}: {}{}",
        "Failed".bold(),
        report.failed.to_string().red(),
        pct_failed.red()
    );
    println!("  {}: {}", "Cached".bold(), report.cached);
    println!("  {}: {}", "Level".bold(), report.level.bright_yellow());
    if let Some(ms) = report.duration_ms {
        println!("  {}: {}ms", "Duration".bold(), ms);
    }
    println!();

    if report.failed > 0 {
        println!("{}", "Failed contracts:".bold().red());
        for r in &report.results {
            if r.status == "failed" {
                println!("  {} {}", "✗".red(), r.contract_id.bold());
                if let Some(err) = &r.error {
                    println!("    {}", err.red());
                }
            }
        }
        println!();
    }
}

// ── Export ─────────────────────────────────────────────────────────────────────

fn export_report(report: &BatchVerifyReport, path: &str) -> Result<()> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "csv" => export_csv(report, path),
        _ => export_json(report, path),
    }
}

fn export_json(report: &BatchVerifyReport, path: &str) -> Result<()> {
    let content = serde_json::to_string_pretty(report)?;
    fs::write(path, content).with_context(|| format!("Failed to write export file: {}", path))?;
    Ok(())
}

fn export_csv(report: &BatchVerifyReport, path: &str) -> Result<()> {
    let mut wtr =
        Writer::from_path(path).with_context(|| format!("Failed to create CSV file: {}", path))?;
    wtr.write_record([
        "batch_id",
        "contract_id",
        "version",
        "status",
        "error",
        "verified_at",
        "level",
    ])?;
    for r in &report.results {
        wtr.write_record([
            &report.batch_id,
            &r.contract_id,
            r.version.as_deref().unwrap_or(""),
            &r.status,
            r.error.as_deref().unwrap_or(""),
            r.verified_at.as_deref().unwrap_or(""),
            r.level.as_deref().unwrap_or(&report.level),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

fn save_text_report(report: &BatchVerifyReport, path: &str) -> Result<()> {
    let mut lines = Vec::new();
    lines.push("Batch Verification Report".to_string());
    lines.push("=========================".to_string());
    lines.push(format!("Batch ID:    {}", report.batch_id));
    lines.push(format!("Generated:   {}", report.generated_at));
    lines.push(format!("Level:       {}", report.level));
    lines.push(format!("Total:       {}", report.total));
    lines.push(format!("Verified:    {}", report.succeeded));
    lines.push(format!("Failed:      {}", report.failed));
    lines.push(format!("Duplicates:  {}", report.skipped_duplicates));
    if let Some(ms) = report.duration_ms {
        lines.push(format!("Duration:    {}ms", ms));
    }
    lines.push(String::new());
    lines.push("Results:".to_string());
    for r in &report.results {
        let ver = r
            .version
            .as_deref()
            .map(|v| format!("@{}", v))
            .unwrap_or_default();
        lines.push(format!(
            "  [{}] {}{}",
            r.status.to_uppercase(),
            r.contract_id,
            ver
        ));
        if let Some(err) = &r.error {
            lines.push(format!("      Error: {}", err));
        }
    }
    fs::write(path, lines.join("\n"))
        .with_context(|| format!("Failed to write report file: {}", path))?;
    Ok(())
}

// ── Schedule ───────────────────────────────────────────────────────────────────

fn schedules_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".soroban-registry").join("schedules.json"))
}

fn save_schedule(cron: &str, command: &str) -> Result<()> {
    let Some(path) = schedules_file_path() else {
        anyhow::bail!("Could not resolve home directory for schedules file");
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let mut config: SchedulesConfig = if path.exists() {
        let raw = fs::read_to_string(&path).context("Failed to read schedules.json")?;
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        SchedulesConfig::default()
    };

    config.schedules.push(ScheduleEntry {
        name: format!("batch-verify-{}", Uuid::new_v4()),
        cron: cron.to_string(),
        command: command.to_string(),
        created_at: Utc::now().to_rfc3339(),
    });

    let content = serde_json::to_string_pretty(&config)?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write schedules file: {}", path.display()))?;
    Ok(())
}

fn format_crontab_entry(cron: &str, command: &str) -> String {
    format!("{} {}", cron, command)
}

fn build_command_repr(args: &BatchVerifyArgs<'_>) -> String {
    let mut parts = vec!["soroban-registry batch-verify".to_string()];
    if let Some(file) = args.file {
        parts.push(format!("--file {}", file));
    }
    if let Some(contracts) = args.contracts {
        parts.push(format!("--contracts {}", contracts));
    }
    if let Some(network) = args.network {
        parts.push(format!("--network {}", network));
    }
    if let Some(category) = args.category {
        parts.push(format!("--category {}", category));
    }
    if let Some(age) = args.age {
        parts.push(format!("--age {}", age));
    }
    parts.push(format!("--initiated-by {}", args.initiated_by));
    parts.push(format!("--level {}", args.level));
    parts.join(" ")
}
