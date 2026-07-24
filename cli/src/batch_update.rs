#![allow(dead_code)]

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use tokio::task::JoinSet;
use uuid::Uuid;

const MAX_BATCH_SIZE: usize = 50;
const BATCH_TIMEOUT_SECS: u64 = 30;
const CHUNK_CONCURRENCY: usize = 4;

// ── Public entry point ─────────────────────────────────────────────────────────

pub struct BatchUpdateArgs<'a> {
    pub api_url: &'a str,
    pub file: Option<&'a str>,
    pub filter: Option<&'a str>,
    pub preview: bool,
    pub condition: Option<&'a str>,
    pub user_id: Option<&'a str>,
    pub rollback_on_error: bool,
    pub json: bool,
}

// ── Manifest types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct UpdateManifest {
    #[serde(default)]
    pub metadata: MetadataUpdate,
    #[serde(default)]
    pub contracts: Vec<ContractUpdateEntry>,
    pub change_summary: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct MetadataUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ContractUpdateEntry {
    pub contract_id: String,
    // Per-contract overrides — win over global metadata.
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
}

// ── Request / response types ───────────────────────────────────────────────────

#[derive(Debug, Serialize, Clone)]
struct BatchUpdateItem {
    contract_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    change_summary: Option<String>,
}

#[derive(Debug, Serialize)]
struct BatchUpdateRequest {
    items: Vec<BatchUpdateItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
    batch_id: String,
}

#[derive(Debug, Deserialize)]
struct BatchUpdateItemResult {
    contract_id: String,
    ok: bool,
    rollback_version_id: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BackendBatchUpdateResponse {
    batch_id: String,
    total: usize,
    succeeded: usize,
    failed: usize,
    results: Vec<BatchUpdateItemResult>,
}

// ── Report types ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BatchUpdateResult {
    pub contract_id: String,
    pub status: String, // "updated" | "skipped" | "failed" | "preview" | "rolled_back"
    pub rollback_version_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchUpdateReport {
    pub batch_id: String,
    pub preview: bool,
    pub total: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub rolled_back: usize,
    pub results: Vec<BatchUpdateResult>,
}

// ── Main entry point ───────────────────────────────────────────────────────────

pub async fn run_batch_update(args: BatchUpdateArgs<'_>) -> Result<()> {
    if args.file.is_none() && args.filter.is_none() {
        anyhow::bail!("Provide --file and/or --filter to identify contracts to update");
    }

    let manifest = if let Some(path) = args.file {
        load_manifest(path)?
    } else {
        UpdateManifest::default()
    };

    // Resolve IDs from manifest entries.
    let mut contract_ids: Vec<String> = manifest
        .contracts
        .iter()
        .map(|e| e.contract_id.clone())
        .collect();

    // Augment from filter if provided.
    if let Some(filter) = args.filter {
        let filter_ids = fetch_ids_by_filter(args.api_url, filter).await?;
        contract_ids.extend(filter_ids);
    }

    // Deduplicate while preserving order.
    let contract_ids = deduplicate_ids(contract_ids);

    if contract_ids.is_empty() {
        anyhow::bail!("No contract IDs resolved — check --file / --filter");
    }

    // Build per-contract payloads by merging global metadata with per-contract overrides.
    let overrides: std::collections::HashMap<String, &ContractUpdateEntry> = manifest
        .contracts
        .iter()
        .map(|e| (e.contract_id.clone(), e))
        .collect();

    let mut items: Vec<BatchUpdateItem> = contract_ids
        .iter()
        .map(|id| {
            let ovr = overrides.get(id.as_str());
            BatchUpdateItem {
                contract_id: id.clone(),
                name: ovr
                    .and_then(|o| o.name.clone())
                    .or_else(|| manifest.metadata.name.clone()),
                description: ovr
                    .and_then(|o| o.description.clone())
                    .or_else(|| manifest.metadata.description.clone()),
                category: ovr
                    .and_then(|o| o.category.clone())
                    .or_else(|| manifest.metadata.category.clone()),
                tags: ovr
                    .and_then(|o| o.tags.clone())
                    .or_else(|| manifest.metadata.tags.clone()),
                change_summary: manifest.change_summary.clone(),
            }
        })
        .collect();

    // Apply --if condition client-side: mark non-matching IDs to skip.
    let mut skipped_ids: HashSet<String> = HashSet::new();
    if let Some(condition) = args.condition {
        let (field, value) = parse_condition(condition)?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        for id in &contract_ids {
            let url = format!("{}/api/contracts/{}", args.api_url, id);
            let resp = client.get(&url).send_with_retry().await;
            if let Ok(r) = resp {
                if let Ok(body) = r.json::<serde_json::Value>().await {
                    let actual = body.get(&field).and_then(|v| v.as_str()).unwrap_or("");
                    if actual != value {
                        skipped_ids.insert(id.clone());
                    }
                }
            }
        }
        items.retain(|i| !skipped_ids.contains(&i.contract_id));
    }

    let batch_id = Uuid::new_v4().to_string();

    // Preview mode: show diff table without making any writes.
    if args.preview {
        let report = build_preview_report(&batch_id, &items, &skipped_ids);
        if args.json {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            print_preview_table(&report);
        }
        return Ok(());
    }

    println!("\n{}", "Batch Metadata Update".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("  {}: {}", "Contracts".bold(), items.len());
    if !skipped_ids.is_empty() {
        println!(
            "  {}: {} (condition not met)",
            "Skipped".bold(),
            skipped_ids.len().to_string().yellow()
        );
    }
    println!("  {}: {}", "Batch ID".bold(), batch_id.bright_black());
    println!();

    // Chunk and dispatch.
    let request = BatchUpdateRequest {
        items,
        user_id: args.user_id.map(|s| s.to_string()),
        batch_id: batch_id.clone(),
    };

    let backend_resp = dispatch_chunks(args.api_url, request).await?;

    // Handle --rollback-on-error: rollback all successfully applied contracts on failure.
    let mut rolled_back = 0usize;
    if args.rollback_on_error && backend_resp.failed > 0 {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(BATCH_TIMEOUT_SECS))
            .build()?;
        for result in &backend_resp.results {
            if result.ok {
                if let Some(ref version_id) = result.rollback_version_id {
                    let url = format!(
                        "{}/api/contracts/{}/metadata/rollback/{}",
                        args.api_url, result.contract_id, version_id
                    );
                    let _ = client.post(&url).send_with_retry().await;
                    rolled_back += 1;
                }
            }
        }
        if rolled_back > 0 {
            println!(
                "  {} Rolled back {} contract(s) due to partial failure",
                "⚠".yellow(),
                rolled_back
            );
        }
    }

    let mut results: Vec<BatchUpdateResult> = Vec::new();
    for r in &backend_resp.results {
        results.push(BatchUpdateResult {
            contract_id: r.contract_id.clone(),
            status: if rolled_back > 0 && r.ok {
                "rolled_back".to_string()
            } else if r.ok {
                "updated".to_string()
            } else {
                "failed".to_string()
            },
            rollback_version_id: r.rollback_version_id.clone(),
            error: r.error.clone(),
        });
    }
    for id in &skipped_ids {
        results.push(BatchUpdateResult {
            contract_id: id.clone(),
            status: "skipped".to_string(),
            rollback_version_id: None,
            error: None,
        });
    }

    let report = BatchUpdateReport {
        batch_id: backend_resp.batch_id,
        preview: false,
        total: results.len(),
        updated: backend_resp.succeeded,
        skipped: skipped_ids.len(),
        failed: backend_resp.failed,
        rolled_back,
        results,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report_table(&report);
    }

    if report.failed > 0 && !args.rollback_on_error {
        std::process::exit(1);
    }

    Ok(())
}

// ── Manifest loading ───────────────────────────────────────────────────────────

fn load_manifest(path: &str) -> Result<UpdateManifest> {
    let content =
        fs::read_to_string(path).with_context(|| format!("Failed to read manifest: {}", path))?;
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "json" => serde_json::from_str(&content).context("Failed to parse JSON manifest"),
        "yaml" | "yml" => serde_yaml::from_str(&content).context("Failed to parse YAML manifest"),
        _ => anyhow::bail!(
            "Unsupported manifest format '{}' — use .yaml, .yml, or .json",
            ext
        ),
    }
}

// ── Filter-based ID discovery ──────────────────────────────────────────────────

async fn fetch_ids_by_filter(api_url: &str, filter: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // filter format: "field=value[,field=value,...]"
    let query_params = filter
        .split(',')
        .filter_map(|part| {
            let mut kv = part.splitn(2, '=');
            let k = kv.next()?.trim();
            let v = kv.next()?.trim();
            if k.is_empty() || v.is_empty() {
                None
            } else {
                Some(format!("{}={}", k, v))
            }
        })
        .collect::<Vec<_>>()
        .join("&");

    let mut ids = Vec::new();
    let mut page = 1i64;

    loop {
        let url = format!(
            "{}/api/contracts?limit=100&page={}&{}",
            api_url, page, query_params
        );
        let resp = client
            .get(&url)
            .send_with_retry()
            .await
            .context("Failed to fetch contracts from API")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let err = resp.text().await.unwrap_or_else(|_| "unknown".to_string());
            anyhow::bail!("API error fetching contracts (HTTP {}): {}", status, err);
        }

        let body: serde_json::Value = resp
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
            if let Some(id) = item
                .get("id")
                .or_else(|| item.get("contract_id"))
                .and_then(|v| v.as_str())
            {
                if !id.is_empty() {
                    ids.push(id.to_string());
                }
            }
        }

        if page >= total_pages {
            break;
        }
        page += 1;
    }

    Ok(ids)
}

// ── Deduplication ──────────────────────────────────────────────────────────────

fn deduplicate_ids(ids: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    ids.into_iter()
        .filter(|id| seen.insert(id.clone()))
        .collect()
}

// ── Condition parsing ──────────────────────────────────────────────────────────

fn parse_condition(condition: &str) -> Result<(String, String)> {
    let mut parts = condition.splitn(2, '=');
    let field = parts
        .next()
        .filter(|s| !s.is_empty())
        .context("Condition must be in the form field=value")?
        .to_string();
    let value = parts
        .next()
        .context("Condition must be in the form field=value")?
        .to_string();
    Ok((field, value))
}

// ── Chunked dispatch ───────────────────────────────────────────────────────────

async fn dispatch_chunks(
    api_url: &str,
    req: BatchUpdateRequest,
) -> Result<BackendBatchUpdateResponse> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(BATCH_TIMEOUT_SECS))
        .build()?;

    let chunks: Vec<Vec<BatchUpdateItem>> = req
        .items
        .chunks(MAX_BATCH_SIZE)
        .map(|c| c.to_vec())
        .collect();

    let total_chunks = chunks.len();
    if total_chunks > 1 {
        println!(
            "  {} {} chunks of up to {} contracts",
            "Dispatching".bold(),
            total_chunks,
            MAX_BATCH_SIZE
        );
    }

    let mut set: JoinSet<Result<BackendBatchUpdateResponse>> = JoinSet::new();
    let mut merged = BackendBatchUpdateResponse {
        batch_id: req.batch_id.clone(),
        total: 0,
        succeeded: 0,
        failed: 0,
        results: Vec::new(),
    };
    let user_id = req.user_id.clone();
    let batch_id = req.batch_id.clone();
    let mut chunks_iter = chunks.into_iter();

    for chunk in chunks_iter.by_ref().take(CHUNK_CONCURRENCY) {
        let c = client.clone();
        let url = api_url.to_string();
        let uid = user_id.clone();
        let bid = batch_id.clone();
        set.spawn(async move { send_chunk(&c, &url, chunk, uid, bid).await });
    }

    while let Some(result) = set.join_next().await {
        let resp = result.context("Chunk task panicked")??;
        merged.total += resp.total;
        merged.succeeded += resp.succeeded;
        merged.failed += resp.failed;
        merged.results.extend(resp.results);

        if let Some(chunk) = chunks_iter.next() {
            let c = client.clone();
            let url = api_url.to_string();
            let uid = user_id.clone();
            let bid = batch_id.clone();
            set.spawn(async move { send_chunk(&c, &url, chunk, uid, bid).await });
        }
    }

    Ok(merged)
}

async fn send_chunk(
    client: &reqwest::Client,
    api_url: &str,
    items: Vec<BatchUpdateItem>,
    user_id: Option<String>,
    batch_id: String,
) -> Result<BackendBatchUpdateResponse> {
    let req = BatchUpdateRequest {
        items,
        user_id,
        batch_id,
    };

    let resp = client
        .post(format!("{}/api/contracts/metadata/batch", api_url))
        .json(&req)
        .send_with_retry()
        .await
        .context("Failed to reach registry API — is the server running?")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let err = resp.text().await.unwrap_or_else(|_| "unknown".to_string());
        anyhow::bail!("API error (HTTP {}): {}", status, err);
    }

    resp.json::<BackendBatchUpdateResponse>()
        .await
        .context("Failed to parse batch update response")
}

// ── Preview ────────────────────────────────────────────────────────────────────

fn build_preview_report(
    batch_id: &str,
    items: &[BatchUpdateItem],
    skipped_ids: &HashSet<String>,
) -> BatchUpdateReport {
    let mut results: Vec<BatchUpdateResult> = items
        .iter()
        .map(|i| BatchUpdateResult {
            contract_id: i.contract_id.clone(),
            status: "preview".to_string(),
            rollback_version_id: None,
            error: None,
        })
        .collect();
    for id in skipped_ids {
        results.push(BatchUpdateResult {
            contract_id: id.clone(),
            status: "skipped".to_string(),
            rollback_version_id: None,
            error: None,
        });
    }
    BatchUpdateReport {
        batch_id: batch_id.to_string(),
        preview: true,
        total: results.len(),
        updated: 0,
        skipped: skipped_ids.len(),
        failed: 0,
        rolled_back: 0,
        results,
    }
}

// ── Display helpers ────────────────────────────────────────────────────────────

fn print_preview_table(report: &BatchUpdateReport) {
    println!("\n{}", "Batch Metadata Update — Preview".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("  {}: {}", "Total contracts".bold(), report.total);
    println!(
        "  {}: {}",
        "Would update".bold(),
        (report.total - report.skipped).to_string().green()
    );
    if report.skipped > 0 {
        println!(
            "  {}: {}",
            "Would skip".bold(),
            report.skipped.to_string().yellow()
        );
    }
    println!();
    println!("{:<40} {}", "Contract ID".bold(), "Action".bold());
    println!("{}", "-".repeat(50));
    for r in &report.results {
        let action = match r.status.as_str() {
            "preview" => "would update".green(),
            "skipped" => "skip (condition)".yellow(),
            _ => r.status.as_str().normal(),
        };
        println!(
            "{:<40} {}",
            &r.contract_id[..r.contract_id.len().min(40)],
            action
        );
    }
    println!();
    println!(
        "{}",
        "(no writes were made — remove --preview to apply)".bright_black()
    );
}

fn print_report_table(report: &BatchUpdateReport) {
    println!("{}", "Results".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!(
        "  {}: {}",
        "Updated".bold(),
        report.updated.to_string().green()
    );
    if report.skipped > 0 {
        println!(
            "  {}: {}",
            "Skipped".bold(),
            report.skipped.to_string().yellow()
        );
    }
    if report.failed > 0 {
        println!("  {}: {}", "Failed".bold(), report.failed.to_string().red());
    }
    if report.rolled_back > 0 {
        println!(
            "  {}: {}",
            "Rolled back".bold(),
            report.rolled_back.to_string().yellow()
        );
    }
    println!(
        "  {}: {}",
        "Batch ID".bold(),
        report.batch_id.bright_black()
    );
    println!();

    let has_errors = report.results.iter().any(|r| r.error.is_some());
    if has_errors {
        println!("{}", "Failures:".bold().red());
        for r in report.results.iter().filter(|r| r.error.is_some()) {
            println!(
                "  {} — {}",
                r.contract_id.bright_red(),
                r.error.as_deref().unwrap_or("unknown error").red()
            );
        }
    }
}
