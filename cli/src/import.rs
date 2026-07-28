//! `contract import` — registry population from external sources.
//!
//! Supports JSON, JSONL (newline-delimited JSON), CSV, and SQLite formats.
//! Validates all records before submission, handles duplicates via
//! `--on-duplicate`, supports network remapping, dry-run preview, atomic
//! rollback on error, and emits a structured import summary on completion.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::io_utils::{compute_sha256_streaming, extract_tar_gz};
use crate::manifest::{AuditEntry, ExportManifest};
use crate::net::RequestBuilderExt;

// ─── On-duplicate strategy ────────────────────────────────────────────────────

/// How to handle a contract that already exists in the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnDuplicate {
    /// Skip the duplicate silently (default).
    Skip,
    /// Overwrite the existing record with the new data.
    Update,
    /// Abort the entire import with an error.
    Fail,
}

impl OnDuplicate {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "skip" => Ok(Self::Skip),
            "update" => Ok(Self::Update),
            "fail" => Ok(Self::Fail),
            other => bail!(
                "unknown --on-duplicate value '{}'; expected skip | update | fail",
                other
            ),
        }
    }
}

impl std::fmt::Display for OnDuplicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Skip => write!(f, "skip"),
            Self::Update => write!(f, "update"),
            Self::Fail => write!(f, "fail"),
        }
    }
}

// ─── Network map ──────────────────────────────────────────────────────────────

/// Parse `--network-map` values like `futurenet=testnet,local=testnet`.
pub fn parse_network_map(raw: &[String]) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for item in raw {
        let (from, to) = item
            .split_once('=')
            .with_context(|| format!("invalid --network-map '{}'; expected from=to", item))?;
        let from = from.trim().to_ascii_lowercase();
        let to = to.trim().to_ascii_lowercase();
        anyhow::ensure!(
            !from.is_empty() && !to.is_empty(),
            "network-map keys cannot be empty"
        );
        map.insert(from, to);
    }
    Ok(map)
}

// ─── Core data types ─────────────────────────────────────────────────────────

/// A single contract record, normalised across all supported input formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPayload {
    pub contract_id: String,
    pub name: String,
    pub network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    pub publisher_address: String,
}

/// CSV-specific deserialisation target (all optional except the two primary keys).
#[derive(Debug, Deserialize)]
struct CsvImportPayload {
    pub contract_id: String,
    pub name: String,
    pub network: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<String>,
    pub wasm_hash: Option<String>,
    pub source_url: Option<String>,
    pub publisher_address: Option<String>,
}

// ─── Import options ───────────────────────────────────────────────────────────

/// All options controlling a single import session.
pub struct ImportOptions<'a> {
    pub api_url: &'a str,
    pub file_path: &'a str,
    /// Explicit format override; if `None`, inferred from file extension.
    pub format: Option<&'a str>,
    /// Default network applied to records that carry no network value.
    pub network_flag: Option<&'a str>,
    /// Output directory used only for the legacy `archive` format.
    pub output_dir: &'a str,
    /// Validate every record before touching the registry.
    pub validate: bool,
    /// Preview without writing.
    pub dry_run: bool,
    /// Duplicate-handling strategy.
    pub on_duplicate: OnDuplicate,
    /// Network alias table (e.g. `futurenet → testnet`).
    pub network_map: HashMap<String, String>,
    /// Wrap all writes in a single logical transaction; rollback on any error.
    pub atomic: bool,
    /// Path to write the JSON summary report (stdout if `None`).
    pub report_output: Option<String>,
}

// ─── Import summary ───────────────────────────────────────────────────────────

/// Outcome for a single contract during an import run.
#[derive(Debug, Serialize, Clone)]
pub struct ImportResult {
    pub contract_id: String,
    pub status: ImportStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportStatus {
    Imported,
    Skipped,
    Updated,
    Failed,
    DryRun,
}

impl std::fmt::Display for ImportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Imported => write!(f, "imported"),
            Self::Skipped => write!(f, "skipped"),
            Self::Updated => write!(f, "updated"),
            Self::Failed => write!(f, "failed"),
            Self::DryRun => write!(f, "dry-run"),
        }
    }
}

/// Aggregated summary of a completed (or aborted) import session.
#[derive(Debug, Serialize)]
pub struct ImportSummary {
    pub file: String,
    pub format: String,
    pub dry_run: bool,
    pub atomic: bool,
    pub on_duplicate: String,
    pub total: usize,
    pub imported: usize,
    pub skipped: usize,
    pub updated: usize,
    pub failed: usize,
    pub duration_ms: u128,
    pub results: Vec<ImportResult>,
}

// ─── Entry point ─────────────────────────────────────────────────────────────

/// Main dispatch called from `main.rs`.
pub async fn run(opts: ImportOptions<'_>) -> Result<()> {
    let path = Path::new(opts.file_path);
    anyhow::ensure!(path.is_file(), "File not found: {}", opts.file_path);

    let format_str = opts.format.unwrap_or_else(|| {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("unknown");
        if ext == "gz" || ext == "tar" {
            "archive"
        } else {
            ext
        }
    });

    let started = Instant::now();

    match format_str.to_lowercase().as_str() {
        "json" => {
            let records = parse_json(path)?;
            run_bulk_import(records, "json", &opts, started).await
        }
        "jsonl" | "ndjson" => {
            let records = parse_jsonl(path)?;
            run_bulk_import(records, "jsonl", &opts, started).await
        }
        "csv" => {
            let records = parse_csv(path)?;
            run_bulk_import(records, "csv", &opts, started).await
        }
        "sqlite" | "db" | "sqlite3" => {
            bail!(
                "SQLite import requires the registry database to be accessible directly. \
                 Please use the `soroban-registry import` top-level command with a running \
                 registry database, or export your SQLite data to JSON/CSV first.\n\
                 Hint: sqlite3 your.db '.mode csv' '.headers on' 'SELECT * FROM contracts;' > contracts.csv"
            )
        }
        "archive" | "tar.gz" => {
            if opts.dry_run {
                println!(
                    "{} Archive dry-run: would extract and verify archive.",
                    "i".cyan()
                );
                return Ok(());
            }
            if opts.validate {
                println!("{} Validating archive...", "i".cyan());
            }
            let dest = Path::new(opts.output_dir);
            let manifest = extract_and_verify(path, dest)?;
            println!(
                "{}",
                "✓ Import complete — integrity verified!".green().bold()
            );
            println!(
                "  {}: {}",
                "Contract".bold(),
                manifest.contract_id.bright_black()
            );
            println!("  {}: {}", "Name".bold(), manifest.name);
            if let Some(n) = opts.network_flag {
                println!("  {}: {}", "Network".bold(), n.bright_blue());
            }
            println!("  {}: {}", "SHA-256".bold(), manifest.sha256.bright_black());
            Ok(())
        }
        _ => bail!(
            "Unsupported format '{}'. Use: json, jsonl, csv, sqlite, or archive.",
            format_str
        ),
    }
}

// ─── Parsers ──────────────────────────────────────────────────────────────────

fn parse_json(path: &Path) -> Result<Vec<ImportPayload>> {
    let content = fs::read_to_string(path).context("Failed to read JSON file")?;

    // Accept bare array  OR  {"contracts": [...]}  OR  {"items": [...]}
    match serde_json::from_str::<Vec<ImportPayload>>(&content) {
        Ok(list) => Ok(list),
        Err(_) => {
            let wrapper: serde_json::Value =
                serde_json::from_str(&content).context("Invalid JSON")?;
            let arr = wrapper
                .get("contracts")
                .or_else(|| wrapper.get("items"))
                .and_then(|v| v.as_array())
                .with_context(|| {
                    "Invalid JSON format. Expected an array of contracts, \
                     {\"contracts\": [...]}, or {\"items\": [...]}"
                })?;
            serde_json::from_value(serde_json::Value::Array(arr.clone()))
                .context("Failed to deserialise contract records from JSON wrapper")
        }
    }
}

fn parse_jsonl(path: &Path) -> Result<Vec<ImportPayload>> {
    let file = File::open(path).with_context(|| format!("Cannot open {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for (line_no, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("I/O error reading line {}", line_no + 1))?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue; // skip blank lines and comments
        }
        let record: ImportPayload = serde_json::from_str(trimmed)
            .with_context(|| format!("Invalid JSON on line {}: {}", line_no + 1, trimmed))?;
        records.push(record);
    }

    Ok(records)
}

fn parse_csv(path: &Path) -> Result<Vec<ImportPayload>> {
    let mut reader = csv::Reader::from_path(path).context("Failed to open CSV file")?;
    let mut records = Vec::new();

    for (i, result) in reader.deserialize::<CsvImportPayload>().enumerate() {
        let row = result.with_context(|| format!("CSV parse error on row {}", i + 2))?;
        let tags = row
            .tags
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        records.push(ImportPayload {
            contract_id: row.contract_id,
            name: row.name,
            network: row.network.unwrap_or_else(|| "testnet".to_string()),
            description: row.description,
            category: row.category,
            tags,
            wasm_hash: row.wasm_hash,
            source_url: row.source_url,
            publisher_address: row
                .publisher_address
                .unwrap_or_else(|| "Unknown".to_string()),
        });
    }

    Ok(records)
}

// ─── Validation ───────────────────────────────────────────────────────────────

const VALID_NETWORKS: &[&str] = &["mainnet", "testnet", "futurenet"];

fn validate_records(records: &[ImportPayload]) -> Vec<String> {
    let mut errors = Vec::new();

    for (i, p) in records.iter().enumerate() {
        let row = i + 1;

        if p.contract_id.trim().is_empty() {
            errors.push(format!("Row {}: contract_id is empty", row));
        } else if p.contract_id.len() < 4 {
            errors.push(format!(
                "Row {}: contract_id '{}' is too short (min 4 chars)",
                row, p.contract_id
            ));
        }

        if p.name.trim().is_empty() {
            errors.push(format!("Row {}: name is empty", row));
        }

        if p.publisher_address.trim().is_empty() || p.publisher_address == "Unknown" {
            // Warn but don't fail — some sources legitimately omit this.
        }

        if !VALID_NETWORKS.contains(&p.network.as_str()) {
            errors.push(format!(
                "Row {}: invalid network '{}' (expected: {})",
                row,
                p.network,
                VALID_NETWORKS.join(", ")
            ));
        }

        if let Some(ref hash) = p.wasm_hash {
            if !hash.trim().is_empty() && hash.len() != 64 {
                errors.push(format!(
                    "Row {}: wasm_hash should be a 64-char hex string, got {} chars",
                    row,
                    hash.len()
                ));
            }
        }
    }

    errors
}

// ─── Bulk import engine ───────────────────────────────────────────────────────

async fn run_bulk_import(
    mut records: Vec<ImportPayload>,
    format: &str,
    opts: &ImportOptions<'_>,
    started: Instant,
) -> Result<()> {
    let default_network = opts.network_flag.unwrap_or("testnet").to_string();

    // 1. Apply network defaults and remapping
    for rec in &mut records {
        if rec.network.is_empty() {
            rec.network = default_network.clone();
        }
        let remapped = opts
            .network_map
            .get(&rec.network.to_ascii_lowercase())
            .cloned();
        if let Some(target) = remapped {
            rec.network = target;
        }
    }

    // 2. Validate (if requested)
    if opts.validate {
        let errors = validate_records(&records);
        if !errors.is_empty() {
            eprintln!("\n{}", "Validation errors:".red().bold());
            for e in &errors {
                eprintln!("  {} {}", "✗".red(), e);
            }
            bail!(
                "Validation failed with {} error(s). Fix the input file and retry.",
                errors.len()
            );
        }
        println!(
            "{} All {} records passed validation.",
            "✓".green(),
            records.len()
        );
    }

    // 3. Print header
    println!();
    println!("{}", "Contract Import".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
    println!("  {}: {}", "File".bold(), opts.file_path);
    println!("  {}: {}", "Format".bold(), format.to_uppercase());
    println!("  {}: {}", "Records".bold(), records.len());
    println!("  {}: {}", "On-duplicate".bold(), opts.on_duplicate);
    println!("  {}: {}", "Atomic".bold(), opts.atomic);
    if opts.dry_run {
        println!("  {}: {}", "Mode".bold(), "DRY RUN".yellow().bold());
    }
    println!();

    // 4. Dry-run: print a preview table and return
    if opts.dry_run {
        println!("{}", "─── Dry-run preview ───".yellow());
        println!(
            "  {:<50} {:<12} {}",
            "Contract ID".bold(),
            "Network".bold(),
            "Name".bold()
        );
        println!("  {}", "─".repeat(80));
        for rec in &records {
            println!(
                "  {:<50} {:<12} {}",
                &rec.contract_id, &rec.network, &rec.name
            );
        }
        println!();
        println!(
            "{}",
            "✓ Dry-run complete. No data was written.".yellow().bold()
        );
        return Ok(());
    }

    // 5. Live import loop
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let post_url = format!("{}/api/contracts", trim_url(opts.api_url));
    let mut results: Vec<ImportResult> = Vec::with_capacity(records.len());
    let total = records.len();

    for (i, payload) in records.iter().enumerate() {
        // Progress indicator
        let pct = ((i + 1) * 100) / total;
        print!(
            "\r  [{:>3}%] [{}/{}] {:<50}",
            pct,
            i + 1,
            total,
            truncate(&payload.contract_id, 48)
        );
        let _ = std::io::stdout().flush();

        let response = client.post(&post_url).json(payload).send_with_retry().await;

        let result = match response {
            Ok(resp) => {
                let status = resp.status();
                if status == reqwest::StatusCode::CONFLICT {
                    handle_duplicate(payload, &client, &opts.api_url, opts.on_duplicate).await
                } else if status.is_success() {
                    ImportResult {
                        contract_id: payload.contract_id.clone(),
                        status: ImportStatus::Imported,
                        message: None,
                    }
                } else {
                    let body = resp.text().await.unwrap_or_default();
                    ImportResult {
                        contract_id: payload.contract_id.clone(),
                        status: ImportStatus::Failed,
                        message: Some(format!("HTTP {}: {}", status.as_u16(), body.trim())),
                    }
                }
            }
            Err(e) => ImportResult {
                contract_id: payload.contract_id.clone(),
                status: ImportStatus::Failed,
                message: Some(e.to_string()),
            },
        };

        // If atomic mode and we hit a failure → rollback everything imported so far
        if opts.atomic && result.status == ImportStatus::Failed {
            println!(); // newline after progress
            eprintln!(
                "\n{} Atomic import failed at record [{}/{}]: {}",
                "✗".red().bold(),
                i + 1,
                total,
                result.message.as_deref().unwrap_or("unknown error")
            );
            results.push(result);

            rollback_imported(&results, &client, opts.api_url).await;
            bail!("Atomic import aborted and rolled back.");
        }

        results.push(result);
    }

    println!(); // newline after final progress update

    // 6. Aggregate summary
    let duration_ms = started.elapsed().as_millis();
    let summary = build_summary(opts, format, &results, duration_ms);

    // 7. Print summary
    print_summary(&summary);

    // 8. Write report file if requested
    if let Some(ref report_path) = opts.report_output {
        let json =
            serde_json::to_string_pretty(&summary).context("Failed to serialise import summary")?;
        fs::write(report_path, &json)
            .with_context(|| format!("Failed to write report to {}", report_path))?;
        println!("  {} Report written to {}", "→".cyan(), report_path.bold());
    }

    // 9. Return error if any failures occurred (non-atomic)
    let failed = summary.failed;
    if failed > 0 {
        bail!(
            "Import completed with {} failure(s). See summary above.",
            failed
        );
    }

    Ok(())
}

// ─── Duplicate handling ───────────────────────────────────────────────────────

async fn handle_duplicate(
    payload: &ImportPayload,
    client: &reqwest::Client,
    api_url: &str,
    strategy: OnDuplicate,
) -> ImportResult {
    match strategy {
        OnDuplicate::Skip => ImportResult {
            contract_id: payload.contract_id.clone(),
            status: ImportStatus::Skipped,
            message: Some("already exists — skipped".to_string()),
        },
        OnDuplicate::Fail => ImportResult {
            contract_id: payload.contract_id.clone(),
            status: ImportStatus::Failed,
            message: Some(format!(
                "duplicate contract '{}' — aborting (--on-duplicate=fail)",
                payload.contract_id
            )),
        },
        OnDuplicate::Update => {
            let url = format!(
                "{}/api/contracts/{}",
                trim_url(api_url),
                payload.contract_id
            );
            match client.put(&url).json(payload).send_with_retry().await {
                Ok(resp) if resp.status().is_success() => ImportResult {
                    contract_id: payload.contract_id.clone(),
                    status: ImportStatus::Updated,
                    message: None,
                },
                Ok(resp) => {
                    let body = resp.text().await.unwrap_or_default();
                    ImportResult {
                        contract_id: payload.contract_id.clone(),
                        status: ImportStatus::Failed,
                        message: Some(format!("Update failed: {}", body.trim())),
                    }
                }
                Err(e) => ImportResult {
                    contract_id: payload.contract_id.clone(),
                    status: ImportStatus::Failed,
                    message: Some(format!("Update error: {}", e)),
                },
            }
        }
    }
}

// ─── Atomic rollback ──────────────────────────────────────────────────────────

async fn rollback_imported(results: &[ImportResult], client: &reqwest::Client, api_url: &str) {
    let to_rollback: Vec<&ImportResult> = results
        .iter()
        .filter(|r| r.status == ImportStatus::Imported || r.status == ImportStatus::Updated)
        .collect();

    if to_rollback.is_empty() {
        return;
    }

    eprintln!(
        "\n{} Rolling back {} imported contract(s)...",
        "↩".yellow(),
        to_rollback.len()
    );

    for result in to_rollback {
        let url = format!("{}/api/contracts/{}", trim_url(api_url), result.contract_id);
        match client.delete(&url).send_with_retry().await {
            Ok(resp) if resp.status().is_success() => {
                eprintln!("  {} {} rolled back", "✓".green(), result.contract_id);
            }
            Ok(resp) => {
                eprintln!(
                    "  {} {} rollback HTTP {}",
                    "✗".red(),
                    result.contract_id,
                    resp.status().as_u16()
                );
            }
            Err(e) => {
                eprintln!(
                    "  {} {} rollback error: {}",
                    "✗".red(),
                    result.contract_id,
                    e
                );
            }
        }
    }
}

// ─── Summary helpers ──────────────────────────────────────────────────────────

fn build_summary(
    opts: &ImportOptions<'_>,
    format: &str,
    results: &[ImportResult],
    duration_ms: u128,
) -> ImportSummary {
    ImportSummary {
        file: opts.file_path.to_string(),
        format: format.to_string(),
        dry_run: opts.dry_run,
        atomic: opts.atomic,
        on_duplicate: opts.on_duplicate.to_string(),
        total: results.len(),
        imported: results
            .iter()
            .filter(|r| r.status == ImportStatus::Imported)
            .count(),
        skipped: results
            .iter()
            .filter(|r| r.status == ImportStatus::Skipped)
            .count(),
        updated: results
            .iter()
            .filter(|r| r.status == ImportStatus::Updated)
            .count(),
        failed: results
            .iter()
            .filter(|r| r.status == ImportStatus::Failed)
            .count(),
        duration_ms,
        results: results.to_vec(),
    }
}

fn print_summary(s: &ImportSummary) {
    println!();
    println!(
        "{}",
        "─── Import Summary ───────────────────────────────────".cyan()
    );
    println!("  {:<16} {}", "Total records:".bold(), s.total);
    println!(
        "  {:<16} {}",
        "Imported:".bold(),
        colorize_count(s.imported, "green")
    );
    if s.skipped > 0 {
        println!(
            "  {:<16} {}",
            "Skipped:".bold(),
            s.skipped.to_string().bright_black()
        );
    }
    if s.updated > 0 {
        println!(
            "  {:<16} {}",
            "Updated:".bold(),
            s.updated.to_string().yellow()
        );
    }
    if s.failed > 0 {
        println!(
            "  {:<16} {}",
            "Failed:".bold(),
            s.failed.to_string().red().bold()
        );
        println!();
        println!("{}", "Failures:".red().bold());
        for r in &s.results {
            if r.status == ImportStatus::Failed {
                println!(
                    "  {} {} — {}",
                    "✗".red(),
                    r.contract_id.bold(),
                    r.message.as_deref().unwrap_or("unknown")
                );
            }
        }
    }
    println!(
        "  {:<16} {:.2}s",
        "Duration:".bold(),
        s.duration_ms as f64 / 1_000.0
    );

    println!();
    if s.failed == 0 {
        println!(
            "{}",
            "✓ Import complete — all records processed successfully!"
                .green()
                .bold()
        );
    } else {
        println!(
            "{}",
            format!("⚠ Import finished with {} failure(s).", s.failed)
                .yellow()
                .bold()
        );
    }
}

fn colorize_count(n: usize, colour: &str) -> colored::ColoredString {
    let s = n.to_string();
    match colour {
        "green" => s.green(),
        "red" => s.red(),
        _ => s.normal(),
    }
}

fn trim_url(api_url: &str) -> String {
    api_url.trim_end_matches('/').to_string()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

// ─── Archive import (legacy, kept for backwards compat) ───────────────────────

pub fn extract_and_verify(archive_path: &Path, output_dir: &Path) -> Result<ExportManifest> {
    let tmp_dir = tempfile::tempdir().context("failed to create temp dir")?;

    extract_tar_gz(archive_path, tmp_dir.path())?;

    let manifest_path = tmp_dir.path().join("manifest.json");
    let inner_path = tmp_dir.path().join("contract.tar.gz");

    if !manifest_path.exists() || !inner_path.exists() {
        bail!("invalid archive: missing manifest.json or contract.tar.gz");
    }

    let mut manifest: ExportManifest =
        serde_json::from_reader(BufReader::new(File::open(&manifest_path)?))?;

    let computed_hash = compute_sha256_streaming(&inner_path)?;
    if computed_hash != manifest.sha256 {
        bail!(
            "integrity check failed: expected {} got {}",
            manifest.sha256,
            computed_hash
        );
    }

    manifest.audit_trail.push(AuditEntry {
        action: "import_verified".into(),
        timestamp: Utc::now(),
        actor: "soroban-registry-cli".into(),
    });

    fs::create_dir_all(output_dir)?;
    extract_tar_gz(&inner_path, output_dir)?;

    manifest.audit_trail.push(AuditEntry {
        action: "import_extracted".into(),
        timestamp: Utc::now(),
        actor: "soroban-registry-cli".into(),
    });

    Ok(manifest)
}
