//! contract_verify.rs — `soroban-registry contract verify <address>` (#522)
//!
//! Verifies a deployed contract's authenticity against the on-chain registry.
//! Displays verification status, security scan results, and audit/review info.
//!
//! Features:
//! - Single contract verification with caching (24-hour TTL)
//! - Batch verification for multiple contracts (--batch flag)
//! - Strict mode (--strict) to fail on any warning or error
//! - Detailed verification reports with audit information

use crate::cache;
use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time::{sleep, Duration};

// ── Response shapes ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct VerificationResult {
    pub address: String,
    pub network: String,
    pub name: Option<String>,
    pub is_verified: bool,
    pub verification_status: String, // "verified" | "unverified" | "pending" | "failed"
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub security_scan: Option<SecurityScan>,
    pub audit: Option<AuditInfo>,
    pub publisher: Option<String>,
    pub wasm_hash: Option<String>,
    pub verified_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SecurityScan {
    pub status: String, // "clean" | "warning" | "critical"
    pub vulnerabilities: u64,
    pub warnings: u64,
    pub last_scanned_at: Option<String>,
    pub findings: Vec<ScanFinding>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ScanFinding {
    pub severity: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuditInfo {
    pub auditor: Option<String>,
    pub report_url: Option<String>,
    pub audited_at: Option<String>,
    pub passed: bool,
}

// ── Main entry point ─────────────────────────────────────────────────────────

/// `soroban-registry contract verify <address> --network <network> [--json] [--strict] [--batch] [--no-cache]`
///
/// # Parameters
/// - `api_url`: Registry API base URL
/// - `address`: Contract address or comma-separated list for batch mode
/// - `network`: Stellar network (mainnet | testnet | futurenet)
/// - `json`: Output as JSON instead of human-readable format
/// - `strict`: Fail if any warnings or errors are found
/// - `batch`: Process multiple contracts (comma-separated in address)
/// - `no_cache`: Skip cache and always fetch fresh data
pub async fn run(
    api_url: &str,
    address: &str,
    network: &str,
    json: bool,
    strict: bool,
    batch: bool,
    no_cache: bool,
) -> Result<()> {
    log::debug!(
        "contract verify | address={} network={} batch={} strict={} no_cache={} api_url={}",
        address,
        network,
        batch,
        strict,
        no_cache,
        api_url
    );

    // Handle batch verification if enabled
    if batch {
        return run_batch(api_url, address, network, json, strict, no_cache).await;
    }

    // Single contract verification
    verify_single_contract(api_url, address, network, json, strict, no_cache).await
}

/// Verify a single contract with optional caching
async fn verify_single_contract(
    api_url: &str,
    address: &str,
    network: &str,
    json: bool,
    strict: bool,
    no_cache: bool,
) -> Result<()> {
    let client = crate::net::client();

    // ── 1. Try cache first (if not disabled)
    if !no_cache {
        if let Ok(Some(cached)) = cache::get(address, network) {
            if !json {
                println!("{} Loaded from cache", "◀".cyan());
            }

            let result = serde_json::from_value::<VerificationResult>(cached.result.clone())
                .unwrap_or_else(|_| VerificationResult::default());

            if json {
                print_json(&result, &cached.detail, &json!({}))?;
            } else {
                print_human(&result, &cached.detail);
            }

            // Check strict mode
            if strict && (!result.errors.is_empty() || !result.warnings.is_empty()) {
                anyhow::bail!("Verification has issues (strict mode enabled)");
            }

            return Ok(());
        }
    }

    // ── 2. Fetch contract from registry by on-chain address ──────────────────
    let mut contract = fetch_contract(api_url, &client, address, network, json).await?;
    if contract.is_null() {
        return Ok(());
    }

    // ── 3. Initiate verification (source verify when available; fallback batch verify)
    if !json {
        println!("{} Initiating verification...", "•".cyan().bold());
    }
    let initiation = initiate_verification(api_url, &client, &contract, address).await?;

    // ── 4. Wait for completion when backend reports pending/in-progress
    contract =
        wait_for_verification_completion(api_url, &client, address, network, &initiation).await?;

    // ── 5. Build result with status/errors/warnings
    let result = build_result(&contract, &initiation, address, network);

    // ── 6. Fetch verification detail (security scan + audit)
    let detail = fetch_detail(api_url, &client, &contract).await;

    // ── 7. Cache the result (if not disabled)
    if !no_cache {
        let _ = cache::set(
            address,
            network,
            serde_json::to_value(&result)?,
            detail.clone(),
        );
    }

    // ── 8. Output
    if json {
        print_json(&result, &detail, &initiation)?;
    } else {
        print_human(&result, &detail);
    }

    // ── 9. Check strict mode
    if strict && (!result.errors.is_empty() || !result.warnings.is_empty()) {
        anyhow::bail!(
            "Verification completed with {} error(s) and {} warning(s) (strict mode enabled)",
            result.errors.len(),
            result.warnings.len()
        );
    }

    Ok(())
}

/// Run batch verification for multiple contracts
async fn run_batch(
    api_url: &str,
    addresses_str: &str,
    network: &str,
    json: bool,
    strict: bool,
    no_cache: bool,
) -> Result<()> {
    // Parse addresses
    let addresses: Vec<&str> = addresses_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if addresses.is_empty() {
        anyhow::bail!("No contract addresses provided for batch verification");
    }

    if addresses.len() > 50 {
        anyhow::bail!(
            "Batch size {} exceeds maximum of 50 contracts",
            addresses.len()
        );
    }

    if !json {
        println!("{}", "Batch Contract Verification".bold().cyan());
        println!("{}", "═".repeat(60).cyan());
        println!("  {}: {}", "Contracts".bold(), addresses.len());
        println!("  {}: {}", "Network".bold(), network.bright_blue());
        println!(
            "  {}: {}",
            "Strict Mode".bold(),
            if strict { "enabled" } else { "disabled" }
        );
        println!();
    }

    let mut results = Vec::new();
    let mut batch_errors = Vec::new();
    let mut batch_warnings = Vec::new();

    for address in &addresses {
        if !json {
            print!("  Verifying {}... ", address.bright_black());
        }

        match verify_single_contract(api_url, address, network, true, false, no_cache).await {
            Ok(_) => {
                // Extract result from single verification
                if let Ok(Some(cached)) = cache::get(address, network) {
                    let result = serde_json::from_value::<VerificationResult>(cached.result)
                        .unwrap_or_else(|_| VerificationResult::default());

                    batch_errors.extend(result.errors.clone());
                    batch_warnings.extend(result.warnings.clone());

                    if !json {
                        let status = if result.is_verified {
                            "✓".green()
                        } else {
                            "✗".red()
                        };
                        println!("{}", status);
                    }
                    results.push((address.to_string(), result));
                }
            }
            Err(e) => {
                if !json {
                    println!("{}", "✗".red());
                }
                batch_errors.push(format!("Failed to verify {}: {}", address, e));
            }
        }
    }

    // Output batch results
    if json {
        let batch_result = json!({
            "batch": {
                "total": addresses.len(),
                "network": network,
                "strict_mode": strict,
            },
            "results": results.iter().map(|(_, r)| r).collect::<Vec<_>>(),
            "summary": {
                "verified": results.iter().filter(|(_, r)| r.is_verified).count(),
                "unverified": results.iter().filter(|(_, r)| !r.is_verified).count(),
                "total_errors": batch_errors.len(),
                "total_warnings": batch_warnings.len(),
            }
        });
        println!("{}", serde_json::to_string_pretty(&batch_result)?);
    } else {
        println!();
        println!("{} Batch Results Summary", "═".repeat(60).cyan());
        println!(
            "  Verified: {}",
            results
                .iter()
                .filter(|(_, r)| r.is_verified)
                .count()
                .to_string()
                .green()
        );
        println!(
            "  Unverified: {}",
            results
                .iter()
                .filter(|(_, r)| !r.is_verified)
                .count()
                .to_string()
                .red()
        );
        if !batch_errors.is_empty() {
            println!("  Errors: {}", batch_errors.len().to_string().red());
        }
        if !batch_warnings.is_empty() {
            println!("  Warnings: {}", batch_warnings.len().to_string().yellow());
        }
        println!();
    }

    // Check strict mode for batch
    if strict && (!batch_errors.is_empty() || !batch_warnings.is_empty()) {
        anyhow::bail!(
            "Batch verification completed with {} error(s) and {} warning(s) (strict mode enabled)",
            batch_errors.len(),
            batch_warnings.len()
        );
    }

    Ok(())
}

/// Default implementation for VerificationResult (used when parsing fails)
impl Default for VerificationResult {
    fn default() -> Self {
        Self {
            address: String::new(),
            network: String::new(),
            name: None,
            is_verified: false,
            verification_status: "unknown".to_string(),
            errors: Vec::new(),
            warnings: Vec::new(),
            security_scan: None,
            audit: None,
            publisher: None,
            wasm_hash: None,
            verified_at: None,
        }
    }
}

async fn fetch_contract(
    api_url: &str,
    client: &reqwest::Client,
    address: &str,
    network: &str,
    json_output: bool,
) -> Result<Value> {
    let search_url = format!(
        "{}/api/contracts?contract_id={}&network={}",
        api_url, address, network
    );
    log::debug!("GET {}", search_url);

    let response = client
        .get(&search_url)
        .send_with_retry()
        .await
        .context("Failed to connect to registry API. Is the registry running?")?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        if json_output {
            let out = json!({
                "address": address,
                "network": network,
                "is_verified": false,
                "verification_status": "not_found",
                "error": "Contract not found in registry"
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        } else {
            print_header();
            println!(
                "{} Contract {} not found in the {} registry.",
                "✗".red().bold(),
                address.bright_black(),
                network.bright_blue()
            );
            println!(
                "\n  {}: Use 'soroban-registry search' to discover registered contracts.",
                "Hint".bold()
            );
            print_footer();
        }
        return Ok(Value::Null);
    }

    if !status.is_success() {
        let err = response.text().await.unwrap_or_default();
        anyhow::bail!("Registry API error ({}): {}", status, err);
    }

    let raw: Value = response
        .json()
        .await
        .context("Failed to parse registry response")?;
    extract_contract(&raw, address)
}

async fn initiate_verification(
    api_url: &str,
    client: &reqwest::Client,
    contract: &Value,
    address: &str,
) -> Result<Value> {
    let contract_id = contract["contract_id"]
        .as_str()
        .unwrap_or(address)
        .to_string();

    // Preferred path: source verification endpoint when source is available.
    if let Some(source_code) = contract["source_code"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
    {
        let compiler_version = contract["compiler_version"]
            .as_str()
            .unwrap_or("latest")
            .to_string();
        let build_params = contract
            .get("build_params")
            .cloned()
            .unwrap_or_else(|| json!({}));

        let verify_url = format!("{}/api/contracts/verify", api_url);
        log::debug!("POST {}", verify_url);
        let response = client
            .post(&verify_url)
            .json(&json!({
                "contract_id": contract_id,
                "source_code": source_code,
                "build_params": build_params,
                "compiler_version": compiler_version
            }))
            .send_with_retry()
            .await
            .context("Failed to initiate source verification")?;

        if response.status().is_success() {
            return response
                .json::<Value>()
                .await
                .context("Failed to parse verification initiation response");
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        log::warn!(
            "Source verification initiation failed ({}). Falling back to on-chain batch verify: {}",
            status,
            body
        );
    }

    let batch_url = format!("{}/api/contracts/batch-verify", api_url);
    log::debug!("POST {}", batch_url);
    let response = client
        .post(&batch_url)
        .json(&json!({
            "contracts": [
                {
                    "contract_id": contract_id
                }
            ]
        }))
        .send_with_retry()
        .await
        .context("Failed to initiate verification")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Verification initiation failed ({}): {}", status, body);
    }

    response
        .json::<Value>()
        .await
        .context("Failed to parse verification initiation response")
}

async fn wait_for_verification_completion(
    api_url: &str,
    client: &reqwest::Client,
    address: &str,
    network: &str,
    initiation: &Value,
) -> Result<Value> {
    let mut latest = fetch_contract(api_url, client, address, network, false).await?;
    if !is_pending(initiation) {
        return Ok(latest);
    }

    for _ in 0..15 {
        sleep(Duration::from_secs(2)).await;
        latest = fetch_contract(api_url, client, address, network, false).await?;
        let status = latest["verification_status"]
            .as_str()
            .unwrap_or("unverified")
            .to_lowercase();
        if status != "pending" && status != "processing" && status != "in_progress" {
            break;
        }
    }

    Ok(latest)
}

fn is_pending(initiation: &Value) -> bool {
    let status = initiation["status"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();
    if status == "pending" || status == "processing" || status == "in_progress" {
        return true;
    }

    initiation["results"]
        .as_array()
        .and_then(|results| results.first())
        .and_then(|first| first["status"].as_str())
        .map(|s| {
            let norm = s.to_lowercase();
            norm == "pending" || norm == "processing" || norm == "in_progress"
        })
        .unwrap_or(false)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Pull the first contract item from either a paginated list or a direct object.
fn extract_contract<'a>(raw: &'a Value, address: &str) -> Result<Value> {
    // Paginated: { items: [...] }
    if let Some(items) = raw["items"].as_array() {
        return items
            .iter()
            .find(|c| {
                c["contract_id"].as_str().map_or(false, |id| id == address)
                    || c["network_configs"].as_object().map_or(false, |nc| {
                        nc.values()
                            .any(|v| v["contract_id"].as_str() == Some(address))
                    })
            })
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Contract address '{}' not found in registry response",
                    address
                )
            });
    }

    // Direct object
    if raw.is_object() && (raw["contract_id"].is_string() || raw["id"].is_string()) {
        return Ok(raw.clone());
    }

    anyhow::bail!("Unexpected registry response format")
}

fn build_result(
    contract: &Value,
    initiation: &Value,
    address: &str,
    network: &str,
) -> VerificationResult {
    let is_verified = contract["is_verified"].as_bool().unwrap_or(false);
    let verification_status = resolve_verification_status(contract, initiation, is_verified);
    let (errors, warnings) = collect_messages(initiation, contract);

    VerificationResult {
        address: address.to_string(),
        network: network.to_string(),
        name: contract["name"].as_str().map(str::to_string),
        is_verified,
        verification_status,
        errors,
        warnings,
        security_scan: None,
        audit: None,
        publisher: contract["publisher_address"]
            .as_str()
            .or(contract["publisher"].as_str())
            .map(str::to_string),
        wasm_hash: contract["wasm_hash"].as_str().map(str::to_string),
        verified_at: contract["verified_at"]
            .as_str()
            .or(contract["updated_at"].as_str())
            .map(str::to_string),
    }
}

fn resolve_verification_status(contract: &Value, initiation: &Value, is_verified: bool) -> String {
    if let Some(status) = contract["verification_status"].as_str() {
        return status.to_lowercase();
    }

    if let Some(status) = initiation["status"].as_str() {
        return status.to_lowercase();
    }

    let batch_verified = initiation["results"]
        .as_array()
        .and_then(|results| results.first())
        .and_then(|first| first["verified"].as_bool());

    match batch_verified {
        Some(true) => "verified".to_string(),
        Some(false) => "failed".to_string(),
        None if is_verified => "verified".to_string(),
        None => "unverified".to_string(),
    }
}

fn collect_messages(initiation: &Value, contract: &Value) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if let Some(message) = initiation["message"].as_str() {
        if initiation["verified"].as_bool() == Some(false) {
            errors.push(message.to_string());
        }
    }

    if let Some(results) = initiation["results"].as_array() {
        if let Some(first) = results.first() {
            if let Some(err) = first["error"].as_str() {
                errors.push(err.to_string());
            }

            if let Some(source_err) = first["source_verification"]["error"].as_str() {
                errors.push(source_err.to_string());
            }

            if let Some(source_message) = first["source_verification"]["message"].as_str() {
                if first["source_verification"]["verified"].as_bool() != Some(true) {
                    warnings.push(source_message.to_string());
                }
            }

            if let Some(on_chain_warnings) = first["on_chain"]["warnings"].as_array() {
                for warning in on_chain_warnings {
                    if let Some(text) = warning.as_str() {
                        warnings.push(text.to_string());
                    }
                }
            }
        }
    }

    if let Some(status) = contract["verification_status"].as_str() {
        let norm = status.to_lowercase();
        if norm == "failed" && errors.is_empty() {
            errors.push("Verification failed according to registry status".to_string());
        }
    }

    errors.sort();
    errors.dedup();
    warnings.sort();
    warnings.dedup();

    (errors, warnings)
}

/// Try to fetch verification detail endpoint. Non-fatal — returns None on error.
async fn fetch_detail(api_url: &str, client: &reqwest::Client, contract: &Value) -> Option<Value> {
    let id = contract["id"]
        .as_str()
        .or(contract["contract_id"].as_str())?;

    let url = format!("{}/api/contracts/{}/verification-status", api_url, id);
    log::debug!("GET {}", url);

    let res = client.get(&url).send_with_retry().await.ok()?;
    if res.status().is_success() {
        res.json::<Value>().await.ok()
    } else {
        None
    }
}

fn print_json(
    result: &VerificationResult,
    detail: &Option<Value>,
    initiation: &Value,
) -> Result<()> {
    let mut out = serde_json::to_value(result)?;

    if let Some(obj) = out.as_object_mut() {
        obj.insert("verification_initiation".to_string(), initiation.clone());
    }

    if let Some(d) = detail {
        // Merge detail fields into output
        if let (Some(obj), Some(dobj)) = (out.as_object_mut(), d.as_object()) {
            for (k, v) in dobj {
                obj.entry(k).or_insert(v.clone());
            }
        }
    }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn print_header() {
    println!();
    println!("{}", "Contract Verification".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
}

fn print_footer() {
    println!("{}", "═".repeat(60).cyan());
    println!();
}

fn print_human(result: &VerificationResult, detail: &Option<Value>) {
    print_header();

    // ── Basic info ───────────────────────────────────────────────────────────
    if let Some(name) = &result.name {
        println!("  {}  {}", "Contract:".bold(), name.bold());
    }
    println!(
        "  {}   {}",
        "Address:".bold(),
        result.address.bright_black()
    );
    println!("  {}   {}", "Network:".bold(), result.network.bright_blue());

    if let Some(publisher) = &result.publisher {
        println!("  {} {}", "Publisher:".bold(), publisher.bright_black());
    }

    if let Some(wasm_hash) = &result.wasm_hash {
        println!("  {} {}", "WASM Hash:".bold(), wasm_hash.bright_black());
    }

    println!();

    // ── Verification status ──────────────────────────────────────────────────
    let (icon, status_label) = match result.verification_status.as_str() {
        "verified" => ("✔".green().bold(), "VERIFIED".green().bold()),
        "pending" | "processing" | "in_progress" => {
            ("●".yellow().bold(), "PENDING".yellow().bold())
        }
        "failed" => ("✘".red().bold(), "FAILED".red().bold()),
        _ => ("✘".red().bold(), "UNVERIFIED".red().bold()),
    };

    println!("  {} Verification Status: {}", icon, status_label);

    if let Some(verified_at) = &result.verified_at {
        println!("     {} {}", "Last updated:".dimmed(), verified_at.dimmed());
    }

    if !result.errors.is_empty() {
        println!("\n  {}", "Errors".bold().red());
        for error in &result.errors {
            println!("  {} {}", "✗".red().bold(), error);
        }
    }

    if !result.warnings.is_empty() {
        println!("\n  {}", "Warnings".bold().yellow());
        for warning in &result.warnings {
            println!("  {} {}", "⚠".yellow().bold(), warning);
        }
    }

    println!();

    // ── Security scan ────────────────────────────────────────────────────────
    println!("  {}", "Security Scan".bold().underline());

    let scan_status = detail
        .as_ref()
        .and_then(|d| d["security_scan"]["status"].as_str())
        .unwrap_or("unknown");

    let scan_label = match scan_status {
        "clean" => "Clean — no vulnerabilities found".green().bold(),
        "warning" => "Warning — potential issues detected".yellow().bold(),
        "critical" => "Critical — vulnerabilities detected".red().bold(),
        _ => "Not scanned".dimmed().bold(),
    };

    println!("  Status: {}", scan_label);

    if let Some(d) = detail {
        let vulns = d["security_scan"]["vulnerabilities"].as_u64().unwrap_or(0);
        let warnings = d["security_scan"]["warnings"].as_u64().unwrap_or(0);

        if scan_status != "unknown" {
            println!(
                "  Vulnerabilities: {}  Warnings: {}",
                if vulns > 0 {
                    vulns.to_string().red().bold()
                } else {
                    vulns.to_string().green().bold()
                },
                if warnings > 0 {
                    warnings.to_string().yellow().bold()
                } else {
                    warnings.to_string().green().bold()
                }
            );
        }

        if let Some(findings) = d["security_scan"]["findings"].as_array() {
            if !findings.is_empty() {
                println!("\n  {}", "Findings:".bold());
                for f in findings.iter().take(5) {
                    let sev = f["severity"].as_str().unwrap_or("info");
                    let title = f["title"].as_str().unwrap_or("Unknown finding");
                    let sev_label = match sev {
                        "critical" => format!("[{}]", sev.to_uppercase()).red().bold(),
                        "high" => format!("[{}]", sev.to_uppercase()).red(),
                        "medium" => format!("[{}]", sev.to_uppercase()).yellow(),
                        "low" => format!("[{}]", sev.to_uppercase()).bright_black(),
                        _ => format!("[{}]", sev.to_uppercase()).normal(),
                    };
                    println!("    {} {}", sev_label, title);
                    if let Some(desc) = f["description"].as_str() {
                        println!("       {}", desc.dimmed());
                    }
                }
                if findings.len() > 5 {
                    println!("    ...and {} more findings", findings.len() - 5);
                }
            }
        }

        println!();

        // ── Audit info ───────────────────────────────────────────────────────
        println!("  {}", "Audit / Review".bold().underline());

        let audit_passed = d["audit"]["passed"].as_bool();
        match audit_passed {
            Some(true) => println!("  {} {}", "✔".green(), "Audit passed".green().bold()),
            Some(false) => println!("  {} {}", "✘".red(), "Audit failed".red().bold()),
            None => println!("  {}", "No audit record".dimmed()),
        }

        if let Some(auditor) = d["audit"]["auditor"].as_str() {
            println!("  {}: {}", "Auditor".bold(), auditor);
        }
        if let Some(url) = d["audit"]["report_url"].as_str() {
            println!("  {}: {}", "Report".bold(), url.bright_blue());
        }
        if let Some(at) = d["audit"]["audited_at"].as_str() {
            println!("  {}: {}", "Audited at".bold(), at.dimmed());
        }

        println!();
    }

    // ── Summary line ─────────────────────────────────────────────────────────
    print_footer();

    if result.is_verified && scan_status != "critical" {
        println!(
            "  {} Contract {} is verified and safe to interact with.\n",
            "✔".green().bold(),
            result.address.bold()
        );
    } else if !result.is_verified {
        println!(
            "  {} Contract {} is NOT verified — proceed with caution.\n",
            "⚠".yellow().bold(),
            result.address.bold()
        );
        println!(
            "  {}: Run 'soroban-registry search {}' to find more information.\n",
            "Tip".bold(),
            result.address
        );
    } else {
        println!(
            "  {} Contract {} has security issues — review findings above.\n",
            "✘".red().bold(),
            result.address.bold()
        );
    }
}
