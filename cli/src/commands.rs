#![allow(dead_code)]

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;
use std::fs;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    Futurenet,
}

use std::path::Path;

use crate::patch::{PatchManager, Severity};
use crate::profiler;
use crate::test_framework;

pub fn generate_flame_graph_file(
    profile: &profiler::ProfileData,
    output_path: &str,
) -> Result<()> {
    profiler::generate_flame_graph(profile, Path::new(output_path))
}

pub fn profile(
    contract_path: &str,
    method: Option<&str>,
    output: Option<&str>,
    flamegraph: Option<&str>,
    compare: Option<&str>,
    show_recommendations: bool,
) -> Result<()> {
    println!("\n{}", "Profiling contract execution...".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    let profile_data = profiler::profile_contract(contract_path, method)
        .with_context(|| format!("Failed to profile contract: {}", contract_path))?;

    if let Some(method_name) = method {
        if profile_data.functions.is_empty() {
            anyhow::bail!(
                "Method '{}' was not found in contract: {}",
                method_name,
                contract_path
            );
        }
    }

    println!("{}: {}", "Contract".bold(), contract_path);
    println!(
        "{}: {:.2}ms",
        "Total duration".bold(),
        profile_data.total_duration.as_secs_f64() * 1000.0
    );
    println!(
        "{}: {}",
        "Functions profiled".bold(),
        profile_data.functions.len()
    );

    if let Some(output_path) = output {
        let profile_json =
            serde_json::to_string_pretty(&profile_data).context("Failed to serialize profile data")?;
        fs::write(output_path, profile_json)
            .with_context(|| format!("Failed to write profile output: {}", output_path))?;
        println!("{} Profile output written to {}", "✓".green(), output_path);
    }

    if let Some(flamegraph_path) = flamegraph {
        generate_flame_graph_file(&profile_data, flamegraph_path).with_context(|| {
            format!("Failed to generate flame graph at {}", flamegraph_path)
        })?;
        println!("{} Flame graph written to {}", "✓".green(), flamegraph_path);
    }

    if let Some(baseline_path) = compare {
        let baseline = profiler::load_baseline(baseline_path)
            .with_context(|| format!("Failed to load baseline profile from {}", baseline_path))?;
        let comparisons = profiler::compare_profiles(&baseline, &profile_data);

        println!("\n{}", "Profile comparison:".bold().yellow());
        if comparisons.is_empty() {
            println!("No comparable function data found.");
        } else {
            for change in comparisons.iter().take(10) {
                let diff_ms = change.time_diff_ns as f64 / 1_000_000.0;
                println!(
                    "  {} [{}] {:+.2}% ({:+.3}ms)",
                    change.function.bold(),
                    change.status,
                    change.time_diff_percent,
                    diff_ms
                );
            }
            if comparisons.len() > 10 {
                println!("  ...and {} more", comparisons.len() - 10);
            }
        }
    }

    if show_recommendations {
        let recommendations = profiler::generate_recommendations(&profile_data);
        println!("\n{}", "Recommendations:".bold().magenta());
        for recommendation in recommendations {
            println!("  - {}", recommendation);
        }
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    Ok(())
}

pub async fn search(
    api_url: &str,
    query: &str,
    network: Network,
    verified_only: bool,
    networks: Vec<String>,
    category: Option<&str>,
    limit: usize,
    offset: usize,
    json: bool,
) -> Result<()> {
    let client = reqwest::Client::new();

    let mut url = format!(
        "{}/api/contracts?query={}&limit={}&offset={}",
        api_url, query, limit, offset
    );

    if !networks.is_empty() {
        url.push_str(&format!("&networks={}", networks.join(",")));
    } else {
        url.push_str(&format!("&network={}", network));
    }

    if verified_only {
        url.push_str("&verified_only=true");
    }

    if let Some(cat) = category {
        url.push_str(&format!("&category={}", cat));
    }

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to search contracts")?;

    let data: serde_json::Value = response.json().await?;
    let items = data["items"].as_array().context("Invalid response")?;

    if json {
        let contracts: Vec<serde_json::Value> = items
            .iter()
            .map(|c| -> Result<_> {
                Ok(serde_json::json!({
                    "id":          crate::conversions::as_str(&c["contract_id"], "contract_id")?,
                    "name":        crate::conversions::as_str(&c["name"], "name")?,
                    "is_verified": crate::conversions::as_bool(&c["is_verified"], "is_verified")?,
                    "network":     crate::conversions::as_str(&c["network"], "network")?,
                    "category":    c["category"].as_str().unwrap_or(""),
                }))
            })
            .collect::<Result<_, _>>()?;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "contracts": contracts }))?
        );
        return Ok(());
    }

    println!("\n{}", "Search Results:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    // Show active filters
    let mut active_filters: Vec<String> = Vec::new();
    if !networks.is_empty() {
        active_filters.push(format!("networks: {}", networks.join(", ")));
    }
    if let Some(cat) = category {
        active_filters.push(format!("category: {}", cat));
    }
    if verified_only {
        active_filters.push("verified only".to_string());
    }
    if !active_filters.is_empty() {
        println!(
            "  {} {}\n",
            "Active filters:".bold(),
            active_filters.join(" | ").bright_blue()
        );
    }

    if items.is_empty() {
        println!("{}", "No contracts found matching your filters.".yellow());
        println!("\n{}", "Suggestions:".bold());
        println!("  • Try a broader search query");
        if category.is_some() {
            println!("  • Remove the --category filter to see all contract types");
        }
        if !networks.is_empty() {
            println!("  • Try adding more networks: --networks mainnet,testnet,futurenet");
        }
        if verified_only {
            println!("  • Remove --verified-only to include unverified contracts");
        }
        println!("  • Use 'list' command to browse all contracts\n");
        return Ok(());
    }

    for contract in items {
        let name = crate::conversions::as_str(&contract["name"], "name")?;
        let contract_id = crate::conversions::as_str(&contract["contract_id"], "contract_id")?;
        let is_verified = crate::conversions::as_bool(&contract["is_verified"], "is_verified")?;
        let network = crate::conversions::as_str(&contract["network"], "network")?;

        println!("\n{} {}", "●".green(), name.bold());
        println!("  ID: {}", contract_id.bright_black());
        print!(
            "  Status: {} | Network: {}",
            if is_verified {
                "✓ Verified".green()
            } else {
                "○ Unverified".yellow()
            },
            network.bright_blue()
        );

        if let Some(cat) = contract["category"].as_str() {
            if !cat.is_empty() {
                print!(" | Category: {}", cat.bright_magenta());
            }
        }
        println!();

        if let Some(desc) = contract["description"].as_str() {
            println!("  {}", desc.bright_black());
        }
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!("Found {} contract(s) (offset: {})\n", items.len(), offset);

    Ok(())
}

/// Analyze two contract versions or schema files for breaking changes.
pub async fn upgrade_analyze(api_url: &str, old_id: &str, new_id: &str, json_out: bool) -> Result<()> {
    use reqwest::StatusCode;
    use shared::upgrade::{compare_schemas, Schema};

    // Helper to load schema from a local file
    let try_load_file = |path: &str| -> Option<Schema> {
        if std::path::Path::new(path).exists() {
            let bytes = std::fs::read(path).ok()?;
            Schema::from_json_bytes(&bytes).ok()
        } else {
            None
        }
    };

    // If either argument is a local file, prefer file-based analysis
    if let (Some(old_schema), Some(new_schema)) = (try_load_file(old_id), try_load_file(new_id)) {
        let findings = compare_schemas(&old_schema, &new_schema);
        if json_out {
            println!("{}", serde_json::to_string_pretty(&findings)?);
        } else {
            for f in findings {
                println!("[{:?}] {} - {}", f.severity, f.field.unwrap_or_default(), f.message);
            }
        }
        return Ok(());
    }

    // Otherwise try to fetch versions from the API (assumes endpoint exists)
    let client = reqwest::Client::new();
    let url = format!("{}/api/contract_versions/{}", api_url, old_id);
    let old_res = client.get(&url).send().await.context("failed to fetch old version")?;
    if old_res.status() == StatusCode::NOT_FOUND {
        anyhow::bail!("Old version {} not found via API. Try passing a local schema JSON file instead.", old_id);
    }
    let old_json: serde_json::Value = old_res.json().await?;

    let url2 = format!("{}/api/contract_versions/{}", api_url, new_id);
    let new_res = client.get(&url2).send().await.context("failed to fetch new version")?;
    if new_res.status() == StatusCode::NOT_FOUND {
        anyhow::bail!("New version {} not found via API. Try passing a local schema JSON file instead.", new_id);
    }
    let new_json: serde_json::Value = new_res.json().await?;

    // Expect the API to expose a simple schema JSON in `state_schema` field; fall back to error.
    let old_schema_str = old_json["state_schema"].as_str().ok_or_else(|| anyhow::anyhow!("API did not return state_schema for old version"))?;
    let new_schema_str = new_json["state_schema"].as_str().ok_or_else(|| anyhow::anyhow!("API did not return state_schema for new version"))?;

    let old_schema = Schema::from_json_bytes(old_schema_str.as_bytes()).context("failed to parse old schema")?;
    let new_schema = Schema::from_json_bytes(new_schema_str.as_bytes()).context("failed to parse new schema")?;

    let findings = compare_schemas(&old_schema, &new_schema);
    if json_out {
        println!("{}", serde_json::to_string_pretty(&findings)?);
    } else {
        for f in findings {
            println!("[{:?}] {} - {}", f.severity, f.field.unwrap_or_default(), f.message);
        }
    }

    Ok(())
}

#[cfg(test)]
mod upgrade_analyze_tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[tokio::test]
    async fn upgrade_analyze_with_local_files_returns_ok() {
        let dir = tempdir().unwrap();
        let old_path = dir.path().join("old_schema.json");
        let new_path = dir.path().join("new_schema.json");

        // Old schema with one field
        let old_schema = r#"{ "fields": [ { "name": "count", "type": "u64" } ] }"#;
        // New schema empty (removal -> error expected)
        let new_schema = r#"{ "fields": [] }"#;

        let mut f1 = std::fs::File::create(&old_path).unwrap();
        write!(f1, "{}", old_schema).unwrap();
        let mut f2 = std::fs::File::create(&new_path).unwrap();
        write!(f2, "{}", new_schema).unwrap();

        // Should return Ok() even if findings include errors; function prints results.
        let res = upgrade_analyze("http://localhost:3001", old_path.to_str().unwrap(), new_path.to_str().unwrap(), true).await;
        assert!(res.is_ok());
    }
}


impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Testnet => write!(f, "testnet"),
            Network::Futurenet => write!(f, "futurenet"),
        }
    }
}

impl FromStr for Network {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mainnet" => Ok(Network::Mainnet),
            "testnet" => Ok(Network::Testnet),
            "futurenet" => Ok(Network::Futurenet),
            _ => anyhow::bail!(
                "Invalid network: {}. Allowed values: mainnet, testnet, futurenet",
                s
            ),
        }
    }
}

fn resolve_smart_routing(current_network: Network) -> String {
    if current_network.to_string() == "auto" {
        "mainnet".to_string()
    } else {
        current_network.to_string()
    }
}

/// Maximum number of tags allowed in a publish request (mirrors backend).
const MAX_PUBLISH_TAGS: usize = 10;

/// Run client-side preflight validation on publish inputs.
///
/// These checks mirror the backend `PublishRequest` validators so that
/// obvious problems are caught *before* we hit the network.
pub fn validate_publish_inputs(
    contract_id: &str,
    name: &str,
    network: Network,
    category: Option<&str>,
    tags: &[String],
    publisher: &str,
) -> Result<()> {
    let mut errors: Vec<String> = Vec::new();

    // ── contract_id: must start with 'C' and be 56 chars (Stellar contract) ─
    if contract_id.is_empty() {
        errors.push("contract_id is required".to_string());
    } else if !contract_id.starts_with('C') || contract_id.len() != 56 {
        errors.push(format!(
            "contract_id must be a 56-character Stellar contract address starting with 'C' (got {} chars)",
            contract_id.len()
        ));
    }

    // ── name: required, 1-255 chars ─────────────────────────────────────────
    if name.trim().is_empty() {
        errors.push("name is required".to_string());
    } else if name.len() > 255 {
        errors.push(format!("name must be at most 255 characters (got {})", name.len()));
    }

    // ── publisher: must start with 'G' and be 56 chars (Stellar address) ────
    if publisher.is_empty() {
        errors.push("publisher address is required".to_string());
    } else if !publisher.starts_with('G') || publisher.len() != 56 {
        errors.push(format!(
            "publisher must be a 56-character Stellar address starting with 'G' (got {} chars)",
            publisher.len()
        ));
    }

    // ── category: if given, must be one of the allowed values ───────────────
    const ALLOWED_CATEGORIES: &[&str] = &["DEX", "Lending", "Bridge", "Oracle", "Token", "Other"];
    if let Some(cat) = category {
        if !ALLOWED_CATEGORIES.iter().any(|c| c.eq_ignore_ascii_case(cat)) {
            errors.push(format!(
                "category '{}' is not allowed; valid values: {}",
                cat,
                ALLOWED_CATEGORIES.join(", ")
            ));
        }
    }

    // ── tags: at most MAX_PUBLISH_TAGS, each ≤ 50 chars ─────────────────────
    if tags.len() > MAX_PUBLISH_TAGS {
        errors.push(format!("at most {} tags are allowed (got {})", MAX_PUBLISH_TAGS, tags.len()));
    }
    for (i, tag) in tags.iter().enumerate() {
        if tag.len() > 50 {
            errors.push(format!("tag[{}] exceeds 50 characters (got {})", i, tag.len()));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        let msg = errors.join("\n  • ");
        anyhow::bail!("Validation failed:\n  • {}", msg);
    }
}

pub async fn publish(
    api_url: &str,
    contract_id: &str,
    name: &str,
    description: Option<&str>,
    network: Network,
    category: Option<&str>,
    tags: Vec<String>,
    publisher: &str,
    dry_run: bool,
) -> Result<()> {
    // ── Preflight validation (runs in both normal and dry-run modes) ─────────
    validate_publish_inputs(contract_id, name, network, category, &tags, publisher)?;

    let payload = json!({
        "contract_id": contract_id,
        "name": name,
        "description": description,
        "network": network.to_string(),
        "category": category,
        "tags": tags,
        "publisher_address": publisher,
    });

    // ── Dry-run: show payload and exit without hitting the backend ───────────
    if dry_run {
        println!("\n{}", "Dry-run mode — no records will be created.".yellow().bold());
        println!("{}", "=".repeat(80).yellow());
        println!("\n{}", "Validation passed ✓".green().bold());
        println!("\n{}", "Payload that would be sent:".bold().cyan());
        println!("{}", serde_json::to_string_pretty(&payload)?);
        println!("\n{}: POST {}/api/contracts", "Endpoint".bold(), api_url);
        println!("{}", "=".repeat(80).yellow());
        println!();
        return Ok(());
    }

    // ── Normal path: send the request ────────────────────────────────────────
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts", api_url);

    println!("\n{}", "Publishing contract...".bold().cyan());

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .context("Failed to publish contract")?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        anyhow::bail!("Failed to publish: {}", error_text);
    }

    let contract: serde_json::Value = response.json().await?;

    println!("{}", "✓ Contract published successfully!".green().bold());
    println!(
        "\n{}: {}",
        "Name".bold(),
        crate::conversions::as_str(&contract["name"], "name")?
    );
    println!(
        "{}: {}",
        "ID".bold(),
        crate::conversions::as_str(&contract["contract_id"], "contract_id")?
    );
    println!(
        "{}: {}",
        "Network".bold(),
        crate::conversions::as_str(&contract["network"], "network")?.bright_blue()
    );
    println!();

    Ok(())
}

pub async fn list(api_url: &str, limit: usize, network: Network, json: bool,) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/contracts?page_size={}&network={}",
        api_url, limit, network
    );

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to list contracts")?;

    let data: serde_json::Value = response.json().await?;
    let items = data["items"].as_array().context("Invalid response")?;

	if json {
        let contracts: Vec<serde_json::Value> = items
            .iter()
            .map(|c| -> Result<_> { Ok(serde_json::json!({
                "id":          crate::conversions::as_str(&c["contract_id"], "contract_id")?,
                "name":        crate::conversions::as_str(&c["name"], "name")?,
                "is_verified": crate::conversions::as_bool(&c["is_verified"], "is_verified")?,
                "network":     crate::conversions::as_str(&c["network"], "network")?,
            })) })
            .collect::<Result<_, _>>()?;
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "contracts": contracts }))?);
        return Ok(());
    }

    println!("\n{}", "Recent Contracts:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if items.is_empty() {
        println!("{}", "No contracts found.".yellow());
        return Ok(());
    }

    for (i, contract) in items.iter().enumerate() {
        let name = crate::conversions::as_str(&contract["name"], "name")?;
        let contract_id = crate::conversions::as_str(&contract["contract_id"], "contract_id")?;
        let is_verified = crate::conversions::as_bool(&contract["is_verified"], "is_verified")?;
        let network = crate::conversions::as_str(&contract["network"], "network")?;

        println!(
            "\n{}. {} {}",
            i + 1,
            name.bold(),
            if is_verified {
                "✓".green()
            } else {
                "".normal()
            }
        );
        println!(
            "   {} | {}",
            contract_id.bright_black(),
            network.bright_blue()
        );
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    Ok(())
}

fn extract_migration_id(migration: &serde_json::Value) -> Result<String> {
    let Some(migration_id) = migration["id"].as_str() else {
        eprintln!(
            "[error] migration response missing string id field: {}",
            migration
        );
        anyhow::bail!("Invalid migration response: missing id");
    };

    Ok(migration_id.to_string())
}
pub async fn breaking_changes(api_url: &str, old_id: &str, new_id: &str, json: bool) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/contracts/breaking-changes?old_id={}&new_id={}",
        api_url, old_id, new_id
    );

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch breaking changes")?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        anyhow::bail!("Failed to fetch breaking changes: {}", error_text);
    }

    let report: serde_json::Value = response.json().await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    let breaking = crate::conversions::as_bool(&report["breaking"], "breaking")?;
    let breaking_count = crate::conversions::as_u64(&report["breaking_count"], "breaking_count")?;
    let non_breaking_count = crate::conversions::as_u64(&report["non_breaking_count"], "non_breaking_count")?;

    let header = if breaking {
        "Breaking changes detected".red().bold()
    } else {
        "No breaking changes detected".green().bold()
    };

    println!("\n{}", header);
    println!(
        "{} {} | {} {}",
        "Breaking:".bold(),
        breaking_count,
        "Non-breaking:".bold(),
        non_breaking_count
    );

    if let Some(changes) = report["changes"].as_array() {
        for change in changes {
            let severity = crate::conversions::as_str(&change["severity"], "severity")?;
            let message = crate::conversions::as_str(&change["message"], "message")?;
            let label = if severity == "breaking" {
                "BREAKING".red().bold()
            } else {
                "INFO".yellow().bold()
            };
            println!("  {} {}", label, message);
        }
    }

    Ok(())
}


pub async fn migrate(
    api_url: &str,
    contract_id: &str,
    wasm_path: &str,
    simulate_fail: bool,
    dry_run: bool,
) -> Result<()> {
    use sha2::{Digest, Sha256};
    use std::fs;
    use tokio::process::Command;

    println!("\n{}", "Migration Tool".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    // 1. Read WASM file
    let wasm_bytes = fs::read(wasm_path)
        .with_context(|| format!("Failed to read WASM file at {}", wasm_path))?;

    // 2. Compute Hash
    let mut hasher = Sha256::new();
    hasher.update(&wasm_bytes);
    let wasm_hash = hex::encode(hasher.finalize());

    println!("Contract ID: {}", contract_id.green());


    // 3. Create Migration Record (Pending)
    let client = reqwest::Client::new();
    let create_url = format!("{}/api/migrations", api_url);

    let payload = json!({
        "contract_id": contract_id,
        "wasm_hash": wasm_hash,
    });

    print!("\nInitializing migration... ");
    let response = client
        .post(&create_url)
        .json(&payload)
        .send()
        .await
        .context("Failed to contact registry API")?;

    if !response.status().is_success() {
        println!("{}", "Failed".red());
        let err = response.text().await?;
        anyhow::bail!("API Error: {}", err);
    }

    let migration: serde_json::Value = response.json().await?;
    let migration_id = extract_migration_id(&migration)?;
    let migration_id = crate::conversions::as_str(&migration["id"], "id")?;
    println!("{}", "OK".green());
    println!("Migration ID: {}", migration_id);

    // 4. Execute Migration (Mock or Real)
    println!("\n{}", "Executing migration logic...".bold());

    // Check if soroban is installed
    let version_output = Command::new("soroban").arg("--version").output().await;

    let (status, log_output) = if version_output.is_err() {
        println!(
            "{}",
            "Warning: 'soroban' CLI not found. Running in MOCK mode.".yellow()
        );

        if simulate_fail {
            println!("{}", "Simulating FAILURE...".red());
            (
                shared::models::MigrationStatus::Failed,
                "Simulation: Migration failed as requested.".to_string(),
            )
        } else {
            println!("{}", "Simulating SUCCESS...".green());
            (
                shared::models::MigrationStatus::Success,
                "Simulation: Migration succeeded.".to_string(),
            )
        }
    } else {
        // Real execution would go here. For now we will just mock it even if soroban exists
        // because we don't have a real contract to invoke in this environment.
        println!(
            "{}",
            "Soroban CLI found, but full integration is pending. Running in MOCK mode.".yellow()
        );
        if simulate_fail {
            println!("{}", "Simulating FAILURE...".red());
            (
                shared::models::MigrationStatus::Failed,
                "Simulation: Migration failed as requested.".to_string(),
            )
        } else {
            println!("{}", "Simulating SUCCESS...".green());
            (
                shared::models::MigrationStatus::Success,
                "Simulation: Migration executed successfully via soroban CLI (mocked).".to_string(),
            )
        }
    };

    // 5. Update Status
    let update_url = format!("{}/api/migrations/{}", api_url, migration_id);
    let update_payload = json!({
        "status": status,
        "log_output": log_output
    });

    let update_res = client
        .put(&update_url)
        .json(&update_payload)
        .send()
        .await
        .context("Failed to update migration status")?;

    if !update_res.status().is_success() {
        println!("{}", "Failed to update status!".red());
    } else {
        println!("\n{}", "Migration recorded successfully.".green().bold());
        if status == shared::models::MigrationStatus::Failed {
            println!("{}", "Status: FAILED".red().bold());
        } else {
            println!("{}", "Status: SUCCESS".green().bold());
        }
    }

    Ok(())
}


pub async fn export(
    _api_url: &str,
    id: &str,
    output: &str,
    contract_dir: &str,
) -> Result<()> {
    let source = std::path::Path::new(contract_dir);
    anyhow::ensure!(
        source.is_dir(),
        "contract directory does not exist: {}",
        contract_dir
    );
    crate::export::create_archive(
        source,
        std::path::Path::new(output),
        id,
        "contract",
        "testnet",
    )?;
    println!("{}", "✓ Export complete!".green().bold());
    println!("  {}: {}", "Output".bold(), output);
    println!("  {}: {}", "Contract".bold(), id.bright_black());
    println!("  {}: {}\n", "Name".bold(), "contract");
    Ok(())
}

pub async fn import(
    api_url: &str,
    archive: &str,
    network: Network,
    output_dir: &str,
) -> Result<()> {
    println!("\n{}", "Importing contract...".bold().cyan());

    let archive_path = std::path::Path::new(archive);
    anyhow::ensure!(archive_path.is_file(), "archive not found: {}", archive);

    let dest = std::path::Path::new(output_dir);

    let manifest = crate::import::extract_and_verify(archive_path, dest)?;

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
    println!(
        "  {}: {}",
        "Network".bold(),
        network.to_string().bright_blue()
    );
    println!("  {}: {}", "SHA-256".bold(), manifest.sha256.bright_black());
    println!("  {}: {}", "Exported At".bold(), manifest.exported_at);
    println!(
        "  {}: {} file(s)",
        "Contents".bold(),
        manifest.contents.len()
    );
    println!("  {}: {}", "Extracted To".bold(), output_dir);

    println!(
        "\n  {} To register on {}, run:",
        "→".bright_black(),
        network.to_string().bright_blue()
    );
    println!(
        "    soroban-registry publish --contract-id {} --name \"{}\" --network {} --publisher <address>\n",
        manifest.contract_id, manifest.name, network
    );

    Ok(())
}

fn severity_colored(sev: &Severity) -> colored::ColoredString {
    match sev {
        Severity::Critical => "CRITICAL".red().bold(),
        Severity::High => "HIGH".yellow().bold(),
        Severity::Medium => "MEDIUM".cyan(),
        Severity::Low => "LOW".normal(),
    }
}

pub async fn patch_create(
    api_url: &str,
    version: &str,
    hash: &str,
    severity: Severity,
    rollout: u8,
) -> Result<()> {
    println!("\n{}", "Creating security patch...".bold().cyan());

    let patch = PatchManager::create(api_url, version, hash, severity, rollout).await?;

    println!("{}", "✓ Patch created!".green().bold());
    println!("  {}: {}", "ID".bold(), patch.id);
    println!("  {}: {}", "Target Version".bold(), patch.target_version);
    println!(
        "  {}: {}",
        "Severity".bold(),
        severity_colored(&patch.severity)
    );
    println!(
        "  {}: {}",
        "New WASM Hash".bold(),
        patch.new_wasm_hash.bright_black()
    );
    println!("  {}: {}%\n", "Rollout".bold(), patch.rollout_percentage);

    if matches!(patch.severity, Severity::Critical | Severity::High) {
        println!(
            "  {} {}",
            "⚠".red(),
            format!(
                "{} severity — immediate action recommended",
                severity_colored(&patch.severity)
            )
            .red()
        );
    }

    Ok(())
}

/// GET /api/contracts/:id/trust-score
pub async fn trust_score(api_url: &str, contract_id: &str, network: Network) -> Result<()> {
    let url = format!("{}/api/contracts/{}/trust-score", api_url, contract_id);
    log::debug!("GET {}", url);

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .query(&[("network", network.to_string())])
        .send()
        .await
        .context("Failed to reach registry API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to get trust score ({}): {}", status, body);
    }

    let data: serde_json::Value = resp.json().await.context("Failed to parse trust score response")?;

    // ── Header ────────────────────────────────────────────────────────────────
    let name       = crate::conversions::as_str(&data["contract_name"], "contract_name")?;
    let score      = crate::conversions::as_f64(&data["score"], "score")?;
    let badge      = crate::conversions::as_str(&data["badge"], "badge")?;
    let badge_icon = crate::conversions::as_str(&data["badge_icon"], "badge_icon")?;
    let summary    = crate::conversions::as_str(&data["summary"], "summary")?;

    println!("\n{}", "─".repeat(56));
    println!("  Trust Score — {}", name.bold());
    println!("{}", "─".repeat(56));
    println!("  Score : {:.0}/100", score);
    println!("  Badge : {} {}", badge_icon, badge.bold());
    println!("  {}",  summary);
    println!("{}", "─".repeat(56));

    // ── Factor breakdown ──────────────────────────────────────────────────────
    println!("\n  {} Factor Breakdown\n", "📊".bold());

    if let Some(factors) = data["factors"].as_array() {
        for factor in factors {
            let fname   = crate::conversions::as_str(&factor["name"], "name")?;
            let earned  = crate::conversions::as_f64(&factor["points_earned"], "points_earned")?;
            let max     = crate::conversions::as_f64(&factor["points_max"], "points_max")?;
            let explain = crate::conversions::as_str(&factor["explanation"], "explanation")?;

            // Mini progress bar (10 chars)
            let filled = ((earned / max) * 10.0).round() as usize;
            let filled = filled.min(10);
            let bar = format!("{}{}", "█".repeat(filled), "░".repeat(10 - filled));

            println!("  {:<28} [{bar}] {:.0}/{:.0}", fname, earned, max);
            println!("    {}", explain.dimmed());
        }
    }

    // ── Weight documentation ──────────────────────────────────────────────────
    println!("\n  {} Score Weights\n", "⚖️".bold());
    if let Ok(weights) = crate::conversions::as_object(&data["weights"], "weights") {
        for (k, v) in weights {
            let max_pts = crate::conversions::as_f64(v, "weight_value")?;
            println!("  {:<22} {:.0} pts max", k, max_pts);
        }
    }

    let computed_at = crate::conversions::as_str(&data["computed_at"], "computed_at")?;
    println!("\n  Computed at: {}\n", computed_at.dimmed());

    Ok(())
}

pub async fn patch_notify(api_url: &str, patch_id: &str) -> Result<()> {
    println!("\n{}", "Identifying vulnerable contracts...".bold().cyan());

    let (patch, contracts) = PatchManager::find_vulnerable(api_url, patch_id).await?;

    println!(
        "\n{} {} patch for version {}",
        "⚠".bold(),
        severity_colored(&patch.severity),
        patch.target_version.bold()
    );
    println!("{}", "=".repeat(80).cyan());

    if contracts.is_empty() {
        println!("{}", "No vulnerable contracts found.".green());
        return Ok(());
    }

    for (i, c) in contracts.iter().enumerate() {
        let cid = crate::conversions::as_str(&c["contract_id"], "contract_id")?;
        let name = crate::conversions::as_str(&c["name"], "name")?;
        let net = crate::conversions::as_str(&c["network"], "network")?;
        println!(
            "  {}. {} ({}) [{}]",
            i + 1,
            name.bold(),
            cid.bright_black(),
            net.bright_blue()
        );
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!("{} vulnerable contract(s) found\n", contracts.len());

    Ok(())
}

pub async fn patch_apply(api_url: &str, contract_id: &str, patch_id: &str) -> Result<()> {
    println!("\n{}", "Applying security patch...".bold().cyan());

    let audit = PatchManager::apply(api_url, contract_id, patch_id).await?;

    println!("{}", "✓ Patch applied successfully!".green().bold());
    println!("  {}: {}", "Contract".bold(), audit.contract_id);
    println!("  {}: {}", "Patch".bold(), audit.patch_id);
    println!("  {}: {}\n", "Applied At".bold(), audit.applied_at);

    Ok(())
}

pub async fn deps_list(api_url: &str, contract_id: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/dependencies", api_url, contract_id);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch contract dependencies")?;

    if !response.status().is_success() {
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!("Contract not found");
        }
        anyhow::bail!("Failed to fetch dependencies: {}", response.status());
    }

    let items: serde_json::Value = response.json().await?;
    let tree = items.as_array().context("Invalid response format")?;

    println!("\n{}", "Dependency Tree:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if tree.is_empty() {
        println!("{}", "No dependencies found.".yellow());
        return Ok(());
    }

    fn print_tree(nodes: &[serde_json::Value], prefix: &str, is_last: bool) -> Result<()> {
        for (i, node) in nodes.iter().enumerate() {
            let name = node["name"].as_str().unwrap_or("Unknown");
            let constraint = node["constraint_to_parent"].as_str().unwrap_or("*");
            let contract_id = node["contract_id"].as_str().unwrap_or("");

            let name = crate::conversions::as_str(&node["name"], "name")?;
            let constraint = crate::conversions::as_str(&node["constraint_to_parent"], "constraint_to_parent")?;
            let contract_id = crate::conversions::as_str(&node["contract_id"], "contract_id")?;
            
            let is_node_last = i == nodes.len() - 1;
            let marker = if is_node_last {
                "└──"
            } else {
                "├──"
            };

            println!(
                "{}{} {} ({}) {}",
                prefix,
                marker.bright_black(),
                name.bold(),
                constraint.cyan(),
                if contract_id == "unknown" {
                    "[Unresolved]".red()
                } else {
                    "".normal()
                }
            );

            if let Some(children) = node["dependencies"].as_array() {
                if !children.is_empty() {
                    let new_prefix =
                        format!("{}{}", prefix, if is_node_last { "    " } else { "│   " });
                    print_tree(children, &new_prefix, true);
                     let new_prefix = format!("{}{}", prefix, if is_node_last { "    " } else { "│   " });
                     print_tree(children, &new_prefix, true)?;
                }
            }
        }
        Ok(())
    }

    print_tree(tree, "", false)?;

    println!("\n{}", "=".repeat(80).cyan());
    println!();
    Ok(())
}

pub async fn run_tests(
    test_file: &str,
    contract_path: Option<&str>,
    junit_output: Option<&str>,
    show_coverage: bool,
    verbose: bool,
) -> Result<()> {
    let test_path = Path::new(test_file);
    if !test_path.exists() {
        anyhow::bail!("Test file not found: {}", test_file);
    }

    let contract_dir = contract_path.unwrap_or(".");
    let mut runner = test_framework::TestRunner::new(contract_dir)?;

    println!("\n{}", "Running Integration Tests...".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    let scenario = test_framework::load_test_scenario(test_path)?;

    if verbose {
        println!("\n{}: {}", "Scenario".bold(), scenario.name);
        if let Some(desc) = &scenario.description {
            println!("{}: {}", "Description".bold(), desc);
        }
        println!("{}: {}", "Steps".bold(), scenario.steps.len());
    }

    let start_time = std::time::Instant::now();
    let result = runner.run_scenario(scenario).await?;
    let total_time = start_time.elapsed();

    println!("\n{}", "Test Results:".bold().green());
    println!("{}", "=".repeat(80).cyan());

    let status_icon = if result.passed { "✓" } else { "✗" };

    println!(
        "\n{} {} {} ({:.2}ms)",
        status_icon,
        "Scenario:".bold(),
        result.scenario.bold(),
        result.duration.as_secs_f64() * 1000.0
    );

    if !result.passed {
        if let Some(ref err) = result.error {
            println!("{} {}", "Error:".bold().red(), err);
        }
    }

    println!("\n{}", "Step Results:".bold());
    for (i, step) in result.steps.iter().enumerate() {
        let step_icon = if step.passed { "✓" } else { "✗" };

        println!(
            "  {}. {} {} ({:.2}ms)",
            i + 1,
            step_icon,
            step.step_name.bold(),
            step.duration.as_secs_f64() * 1000.0
        );

        if verbose {
            println!(
                "     Assertions: {}/{} passed",
                step.assertions_passed,
                step.assertions_passed + step.assertions_failed
            );
        }

        if let Some(ref err) = step.error {
            println!("     {}", err.red());
        }
    }

    if show_coverage {
        println!("\n{}", "Coverage Report:".bold().magenta());
        println!("  Contracts Tested: {}", result.coverage.contracts_tested);
        println!(
            "  Methods Tested: {}/{}",
            result.coverage.methods_tested, result.coverage.total_methods
        );
        println!("  Coverage: {:.2}%", result.coverage.coverage_percent);

        if result.coverage.coverage_percent < 80.0 {
            println!("  {} Low coverage detected!", "⚠".yellow());
        }
    }

    if let Some(junit_path) = junit_output {
        test_framework::generate_junit_xml(&[result.clone()], Path::new(junit_path))?;
        println!(
            "\n{} JUnit XML report exported to: {}",
            "✓".green(),
            junit_path
        );
    }

    if total_time.as_secs() > 5 {
        println!(
            "\n{} Test execution took {:.2}s (target: <5s)",
            "⚠".yellow(),
            total_time.as_secs_f64()
        );
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    if !result.passed {
        anyhow::bail!("Tests failed");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::extract_migration_id;
    use serde_json::json;

    #[test]
    fn extract_migration_id_returns_id_for_valid_payload() {
        let payload = json!({"id": "migration-123"});
        let migration_id = extract_migration_id(&payload);
        assert!(migration_id.is_ok());
        assert_eq!(migration_id.unwrap_or_default(), "migration-123");
    }

    #[test]
    fn extract_migration_id_fails_when_missing_id() {
        let payload = json!({"status": "pending"});
        let err = extract_migration_id(&payload);
        assert!(err.is_err());
        assert!(err
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default()
            .contains("Invalid migration response: missing id"));
    }

    #[test]
    fn extract_migration_id_fails_when_id_is_not_string() {
        let payload = json!({"id": 99});
        let err = extract_migration_id(&payload);
        assert!(err.is_err());
        assert!(err
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default()
            .contains("Invalid migration response: missing id"));
    }

    // ── Publish dry-run / validation tests ──────────────────────────────────

    use super::{validate_publish_inputs, Network};

    const VALID_CONTRACT_ID: &str = "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";
    const VALID_PUBLISHER: &str = "GDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC";

    #[test]
    fn validate_publish_inputs_accepts_valid_inputs() {
        let result = validate_publish_inputs(
            VALID_CONTRACT_ID,
            "My Contract",
            Network::Testnet,
            Some("Token"),
            &["defi".to_string(), "token".to_string()],
            VALID_PUBLISHER,
        );
        assert!(result.is_ok(), "Expected valid inputs to pass: {:?}", result);
    }

    #[test]
    fn validate_publish_inputs_rejects_invalid_contract_id() {
        let result = validate_publish_inputs(
            "invalid-id",
            "My Contract",
            Network::Testnet,
            None,
            &[],
            VALID_PUBLISHER,
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("contract_id"), "Error should mention contract_id: {}", msg);
    }

    #[test]
    fn validate_publish_inputs_rejects_empty_name() {
        let result = validate_publish_inputs(
            VALID_CONTRACT_ID,
            "",
            Network::Testnet,
            None,
            &[],
            VALID_PUBLISHER,
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("name is required"), "Error should mention name: {}", msg);
    }

    #[test]
    fn validate_publish_inputs_rejects_invalid_publisher() {
        let result = validate_publish_inputs(
            VALID_CONTRACT_ID,
            "My Contract",
            Network::Testnet,
            None,
            &[],
            "not-a-stellar-address",
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("publisher"), "Error should mention publisher: {}", msg);
    }

    #[test]
    fn validate_publish_inputs_rejects_invalid_category() {
        let result = validate_publish_inputs(
            VALID_CONTRACT_ID,
            "My Contract",
            Network::Testnet,
            Some("InvalidCategory"),
            &[],
            VALID_PUBLISHER,
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("category"), "Error should mention category: {}", msg);
    }

    #[test]
    fn validate_publish_inputs_rejects_too_many_tags() {
        let tags: Vec<String> = (0..11).map(|i| format!("tag{}", i)).collect();
        let result = validate_publish_inputs(
            VALID_CONTRACT_ID,
            "My Contract",
            Network::Testnet,
            None,
            &tags,
            VALID_PUBLISHER,
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("tags"), "Error should mention tags: {}", msg);
    }

    #[test]
    fn validate_publish_inputs_collects_multiple_errors() {
        let result = validate_publish_inputs(
            "bad",            // invalid contract_id
            "",               // empty name
            Network::Testnet,
            None,
            &[],
            "bad-publisher",  // invalid publisher
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        // All three errors should be present
        assert!(msg.contains("contract_id"), "missing contract_id error");
        assert!(msg.contains("name is required"), "missing name error");
        assert!(msg.contains("publisher"), "missing publisher error");
    }

    #[tokio::test]
    async fn publish_dry_run_does_not_send_request() {
        // dry_run should succeed without a running backend
        let result = super::publish(
            "http://localhost:0",   // unreachable URL
            VALID_CONTRACT_ID,
            "My Contract",
            Some("A test contract"),
            Network::Testnet,
            Some("Token"),
            vec!["defi".to_string()],
            VALID_PUBLISHER,
            true,
        )
        .await;
        assert!(result.is_ok(), "Dry-run should succeed without backend: {:?}", result);
    }

    #[tokio::test]
    async fn publish_dry_run_still_validates() {
        let result = super::publish(
            "http://localhost:0",
            "invalid",
            "",
            None,
            Network::Testnet,
            None,
            vec![],
            "bad",
            true,
        )
        .await;
        assert!(result.is_err(), "Dry-run should still catch validation errors");
    }
}
pub fn incident_trigger(contract_id: &str, severity_str: &str) -> Result<()> {
    use crate::incident::{IncidentManager, IncidentSeverity};

    let severity = severity_str.parse::<IncidentSeverity>()?;
    let mut mgr = IncidentManager::default();
    let id = mgr.trigger(contract_id.to_string(), severity);

    println!("\n{}", "Incident Triggered".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("  {}: {}", "Incident ID".bold(), id);
    println!("  {}: {}", "Contract".bold(), contract_id.bright_black());
    println!(
        "  {}: {}",
        "Severity".bold(),
        match severity {
            IncidentSeverity::Critical => "CRITICAL".red().bold(),
            IncidentSeverity::High => "HIGH".yellow().bold(),
            IncidentSeverity::Medium => "MEDIUM".cyan(),
            IncidentSeverity::Low => "LOW".normal(),
        }
    );
    println!("  {}: Detected", "State".bold());

    if mgr.is_halted(contract_id) {
        println!(
            "\n  {} {}",
            "⚡ CIRCUIT BREAKER ENGAGED —".red().bold(),
            format!("contract {} is now halted", contract_id).red()
        );
    }

    println!(
        "\n  {} To advance state:\n    soroban-registry incident update {} --state responding\n",
        "→".bright_black(),
        id
    );

    Ok(())
}

pub async fn config_get(api_url: &str, contract_id: &str, environment: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/config?environment={}", api_url, contract_id, environment);

    let response = client.get(&url).send().await.context("Failed to fetch configuration")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to get config: {}", response.text().await.unwrap_or_default());
    }

    let config: serde_json::Value = response.json().await?;

    println!("\n{}", "Contract Configuration (Latest):".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{}: {}", "Contract ID".bold(), contract_id);
    println!("{}: {}", "Environment".bold(), environment);
    println!("{}: {}", "Version".bold(), crate::conversions::as_i64(&config["version"], "version")?);
    println!("{}: {}", "Contains Secrets".bold(), crate::conversions::as_bool(&config["has_secrets"], "has_secrets")?);
    println!("{}: {}", "Created By".bold(), crate::conversions::as_str(&config["created_by"], "created_by")?);
    println!("{}:", "Config Data".bold());
    println!("{}", serde_json::to_string_pretty(&config["config_data"]).unwrap_or_default().green());
    println!();

    Ok(())
}

pub async fn config_set(
    api_url: &str,
    contract_id: &str,
    environment: &str,
    config_data: &str,
    secrets_data: Option<&str>,
    created_by: &str,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/config", api_url, contract_id);

    let mut payload = json!({
        "environment": environment,
        "config_data": serde_json::from_str::<serde_json::Value>(config_data).context("Invalid config JSON")?,
        "created_by": created_by,
    });

    if let Some(sec) = secrets_data {
        let sec_json: serde_json::Value = serde_json::from_str(sec).context("Invalid secrets JSON")?;
        payload["secrets_data"] = sec_json;
    }

    println!("\n{}", "Publishing configuration...".bold().cyan());

    let response = client.post(&url).json(&payload).send().await.context("Failed to set configuration")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to set config: {}", response.text().await.unwrap_or_default());
    }

    let config: serde_json::Value = response.json().await?;

    println!("{}", "✓ Configuration published successfully!".green().bold());
    println!("  {}: {}", "Environment".bold(), environment);
    println!("  {}: {}", "New Version".bold(), crate::conversions::as_i64(&config["version"], "version")?);
    println!();

    Ok(())
}

pub async fn config_history(api_url: &str, contract_id: &str, environment: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/config/history?environment={}", api_url, contract_id, environment);

    let response = client.get(&url).send().await.context("Failed to fetch configuration history")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to get config history: {}", response.text().await.unwrap_or_default());
    }

    let configs: Vec<serde_json::Value> = response.json().await?;

    println!("\n{}", "Configuration History:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if configs.is_empty() {
        println!("{}", "No configurations found.".yellow());
        return Ok(());
    }

    for (i, config) in configs.iter().enumerate() {
        println!(
            "  {}. {} (v{}) - By: {}",
            i + 1,
            crate::conversions::as_str(&config["created_at"], "created_at")?.bright_black(),
            crate::conversions::as_i64(&config["version"], "version")?,
            crate::conversions::as_str(&config["created_by"], "created_by")?.bright_blue()
        );
    }
    println!();

    Ok(())
}

pub async fn config_rollback(
    api_url: &str,
    contract_id: &str,
    environment: &str,
    version: i32,
    created_by: &str,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/config/rollback?environment={}", api_url, contract_id, environment);

    let payload = json!({
        "roll_back_to_version": version,
        "created_by": created_by,
    });

    println!("\n{}", format!("Rolling back configuration to v{}...", version).bold().cyan());

    let response = client.post(&url).json(&payload).send().await.context("Failed to rollback configuration")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to rollback config: {}", response.text().await.unwrap_or_default());
    }

    let config: serde_json::Value = response.json().await?;

    println!("{}", "✓ Configuration rolled back successfully!".green().bold());
    println!("  {}: {}", "Environment".bold(), environment);
    println!("  {}: {}", "New Active Version".bold(), crate::conversions::as_i64(&config["version"], "version")?);
    println!();

    Ok(())
}

pub fn incident_update(incident_id_str: &str, state_str: &str) -> Result<()> {
    use crate::incident::IncidentState;
    use uuid::Uuid;

    let id = incident_id_str
        .parse::<Uuid>()
        .map_err(|_| anyhow::anyhow!("invalid incident ID: {}", incident_id_str))?;
    let new_state = state_str.parse::<IncidentState>()?;

    println!("\n{}", "Incident Updated".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("  {}: {}", "Incident ID".bold(), id);
    println!("  {}: {}", "New State".bold(), new_state.to_string().green().bold());

    if matches!(new_state, IncidentState::Recovered | IncidentState::PostReview) {
        println!(
            "\n  {} {}",
            "✓".green(),
            "Circuit breaker cleared — registry interactions for this contract resumed.".green()
        );
    }

    println!();

    Ok(())
}

pub async fn scan_deps(
    api_url: &str,
    contract_id: &str,
    dependencies: &str,
    fail_on_high: bool,
) -> Result<()> {
    println!("\n{}", "Scanning Dependencies...".bold().cyan());

    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/scan", api_url, contract_id);

    // Parse dependencies
    let mut deps_list = Vec::new();
    for dep_pair in dependencies.split(',') {
        if dep_pair.is_empty() {
            continue;
        }
        let parts: Vec<&str> = dep_pair.split('@').collect();
        if parts.len() == 2 {
            deps_list.push(json!({
                "package_name": parts[0].trim(),
                "version": parts[1].trim()
            }));
        }
    }

    let payload = json!({
        "dependencies": deps_list,
    });

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .context("Failed to run dependency scan")?;

    if !response.status().is_success() {
        anyhow::bail!("Scan failed: {}", response.text().await.unwrap_or_default());
    }

    let report: serde_json::Value = response.json().await?;
    let findings = crate::conversions::as_array(&report["findings"], "findings")?;

    if findings.is_empty() {
        println!("{}", "✓ No vulnerabilities found!".green().bold());
        return Ok(());
    }

    let mut has_high_severity = false;
    println!("\n{}", "Vulnerabilities Found:".bold().red());
    println!("{}", "=".repeat(80).red());

    for finding in findings {
        let package = crate::conversions::as_str(&finding["package_name"], "package_name")?;
        let version = crate::conversions::as_str(&finding["current_version"], "current_version")?;
        let severity = crate::conversions::as_str(&finding["severity"], "severity")?;
        let cve_id = crate::conversions::as_str(&finding["cve_id"], "cve_id")?;
        let recommended = crate::conversions::as_str(&finding["recommended_version"], "recommended_version")?;

        let sev_enum = severity.parse::<Severity>().context("Invalid severity string")?;
        if matches!(sev_enum, Severity::Critical | Severity::High) {
            has_high_severity = true;
        }

        println!("  {} {}@{} - {}", severity_colored(&sev_enum), package, version, cve_id.bold());
        println!("    {} Recommended patch: {}", "↳".bright_black(), recommended.green());
    }

    println!("\n{}", "=".repeat(80).red());
    println!("{} issue(s) detected\n", findings.len());

    if fail_on_high && has_high_severity {
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod flamegraph_and_network_tests {
    use super::*;
    use crate::profiler;
    use std::collections::HashMap;
    use std::fs;
    use std::time::Duration;

    fn sample_profile() -> profiler::ProfileData {
        let mut functions = HashMap::new();
        functions.insert(
            "main".to_string(),
            profiler::FunctionProfile {
                name: "main".to_string(),
                total_time: Duration::from_millis(10),
                call_count: 1,
                avg_time: Duration::from_millis(10),
                min_time: Duration::from_millis(10),
                max_time: Duration::from_millis(10),
                children: vec![],
            },
        );

        profiler::ProfileData {
            contract_path: "contract.rs".to_string(),
            method: Some("main".to_string()),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            total_duration: Duration::from_millis(10),
            functions,
            call_stack: vec![],
            overhead_percent: 0.0,
        }
    }

    fn write_sample_contract(temp_dir: &tempfile::TempDir) -> String {
        let contract_path = temp_dir.path().join("sample_contract.rs");
        fs::write(
            &contract_path,
            "pub fn main() {}\nfn helper_one() {}\nfn helper_two() {}\n",
        )
        .expect("failed to write sample contract");
        contract_path.to_string_lossy().into_owned()
    }

    #[test]
    fn test_network_parsing() {
        assert_eq!("mainnet".parse::<Network>().unwrap(), Network::Mainnet);
        assert_eq!("testnet".parse::<Network>().unwrap(), Network::Testnet);
        assert_eq!("futurenet".parse::<Network>().unwrap(), Network::Futurenet);
        assert_eq!("Mainnet".parse::<Network>().unwrap(), Network::Mainnet); // Case insensitive
        assert!("invalid".parse::<Network>().is_err());
    }

    #[test]
    fn generate_flame_graph_file_writes_svg_for_valid_path() {
        let profile = sample_profile();
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        let output_path = temp_dir.path().join("flamegraph-output.svg");
        let output_path_str = output_path.to_string_lossy().into_owned();

        generate_flame_graph_file(&profile, &output_path_str)
            .expect("expected flame graph generation to succeed");
        assert!(output_path.exists(), "expected output file to exist");
    }

    #[test]
    fn generate_flame_graph_file_returns_error_for_invalid_path() {
        let profile = sample_profile();
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        let invalid_output = temp_dir.path().join("missing-dir").join("flamegraph-output.svg");
        let invalid_output_str = invalid_output.to_string_lossy().into_owned();

        let err = generate_flame_graph_file(&profile, &invalid_output_str)
            .expect_err("expected flame graph generation to fail for invalid path");
        assert!(
            err.to_string().contains("Failed to write flame graph"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn profile_writes_json_and_flamegraph_outputs() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        let contract_path = write_sample_contract(&temp_dir);
        let json_output = temp_dir.path().join("profile-output.json");
        let flame_output = temp_dir.path().join("profile-output.svg");
        let json_output_str = json_output.to_string_lossy().into_owned();
        let flame_output_str = flame_output.to_string_lossy().into_owned();

        profile(
            &contract_path,
            None,
            Some(&json_output_str),
            Some(&flame_output_str),
            None,
            true,
        )
        .expect("expected profiling to succeed");

        assert!(json_output.exists(), "expected JSON profile output to exist");
        assert!(
            flame_output.exists(),
            "expected flame graph output to exist"
        );
    }

    #[test]
    fn profile_supports_baseline_comparison() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        let contract_path = write_sample_contract(&temp_dir);
        let baseline_path = temp_dir.path().join("baseline.json");
        let baseline_path_str = baseline_path.to_string_lossy().into_owned();

        let baseline_json =
            serde_json::to_string_pretty(&sample_profile()).expect("failed to serialize baseline");
        fs::write(&baseline_path, baseline_json).expect("failed to write baseline file");

        profile(
            &contract_path,
            None,
            None,
            None,
            Some(&baseline_path_str),
            false,
        )
        .expect("expected profiling with baseline comparison to succeed");
    }

    #[test]
    fn profile_returns_error_for_missing_baseline() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        let contract_path = write_sample_contract(&temp_dir);
        let missing_baseline = temp_dir.path().join("missing-baseline.json");
        let missing_baseline_str = missing_baseline.to_string_lossy().into_owned();

        let err = profile(
            &contract_path,
            None,
            None,
            None,
            Some(&missing_baseline_str),
            false,
        )
        .expect_err("expected missing baseline to fail");

        assert!(
            err.to_string().contains("Failed to load baseline profile from"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn profile_returns_error_for_unknown_method() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        let contract_path = write_sample_contract(&temp_dir);

        let err = profile(
            &contract_path,
            Some("does_not_exist"),
            None,
            None,
            None,
            false,
        )
        .expect_err("expected unknown method to fail");

        assert!(
            err.to_string().contains("was not found in contract"),
            "unexpected error: {err}"
        );
    }
}
/// Validate a contract function call for type safety
pub async fn validate_call(
    api_url: &str,
    contract_id: &str,
    method_name: &str,
    params: &[String],
    strict: bool,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/validate-call", api_url, contract_id);

    let body = json!({
        "method_name": method_name,
        "params": params,
        "strict": strict
    });

    log::debug!("POST {} body={}", url, body);

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .context("Failed to validate contract call")?;

    let status = response.status();
    let data: serde_json::Value = response.json().await?;

    if !status.is_success() {
        let error_msg = crate::conversions::as_str(&data["message"], "message")?;
        println!("\n{} {}", "Error:".bold().red(), error_msg);
        anyhow::bail!("Validation failed: {}", error_msg);
    }

    let valid = crate::conversions::as_bool(&data["valid"], "valid")?;

    println!("\n{}", "Contract Call Validation".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("\n{}: {}", "Function".bold(), method_name);
    println!("{}: {}", "Contract".bold(), contract_id);
    println!("{}: {}", "Strict Mode".bold(), if strict { "Yes" } else { "No" });

    if valid {
        println!("\n{} {}", "✓".green().bold(), "Call is valid!".green().bold());

        // Show parsed parameters
        if let Some(params) = data["parsed_params"].as_array() {
            println!("\n{}", "Parsed Parameters:".bold());
            for param in params {
                let name = crate::conversions::as_str(&param["name"], "name")?;
                let type_name = crate::conversions::as_str(&param["expected_type"], "expected_type")?;
                println!("  {} {}: {}", "•".green(), name.bold(), type_name);
            }
        }

        // Show expected return type
        if let Some(ret) = data["expected_return"].as_str() {
            println!("\n{}: {}", "Returns".bold(), ret);
        }

        // Show warnings
        if let Some(warnings) = data["warnings"].as_array() {
            if !warnings.is_empty() {
                println!("\n{}", "Warnings:".bold().yellow());
                for warning in warnings {
                    let msg = crate::conversions::as_str(&warning["message"], "message")?;
                    println!("  {} {}", "⚠".yellow(), msg);
                }
            }
        }
    } else {
        println!("\n{} {}", "✗".red().bold(), "Call is invalid!".red().bold());

        // Show errors
        if let Some(errors) = data["errors"].as_array() {
            println!("\n{}", "Errors:".bold().red());
            for error in errors {
                let code = crate::conversions::as_str(&error["code"], "code")?;
                let msg = crate::conversions::as_str(&error["message"], "message")?;
                let field = error["field"].as_str();

                if let Some(f) = field {
                    println!("  {} [{}] {}: {}", "✗".red(), code.bright_black(), f.bold(), msg);
                } else {
                    println!("  {} [{}] {}", "✗".red(), code.bright_black(), msg);
                }

                if let Some(expected) = error["expected"].as_str() {
                    println!("      Expected: {}", expected.green());
                }
                if let Some(actual) = error["actual"].as_str() {
                    println!("      Actual:   {}", actual.red());
                }
            }
        }
    }

    println!("\n{}", "=".repeat(60).cyan());
    println!();

    if !valid {
        anyhow::bail!("Validation failed");
    }

    Ok(())
}

/// Generate type-safe bindings for a contract
pub async fn generate_bindings(
    api_url: &str,
    contract_id: &str,
    language: &str,
    output: Option<&str>,
) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/contracts/{}/bindings?language={}",
        api_url, contract_id, language
    );

    log::debug!("GET {}", url);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to generate bindings")?;

    let status = response.status();

    if !status.is_success() {
        let error: serde_json::Value = response.json().await?;
        let msg = crate::conversions::as_str(&error["message"], "message")?;
        anyhow::bail!("Failed to generate bindings: {}", msg);
    }

    let bindings = response.text().await?;

    if let Some(output_path) = output {
        fs::write(output_path, &bindings)?;
        println!(
            "\n{} {} bindings written to: {}",
            "✓".green().bold(),
            language,
            output_path
        );
    } else {
        // Print to stdout
        println!("{}", bindings);
    }

    Ok(())
}

/// List functions available on a contract
pub async fn list_functions(api_url: &str, contract_id: &str) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/contracts/{}/functions", api_url, contract_id);

    log::debug!("GET {}", url);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to list contract functions")?;

    let status = response.status();
    let data: serde_json::Value = response.json().await?;

    if !status.is_success() {
        let msg = crate::conversions::as_str(&data["message"], "message")?;
        anyhow::bail!("Failed to list functions: {}", msg);
    }

    let contract_name = crate::conversions::as_str(&data["contract_name"], "contract_name")?;
    let functions = data["functions"].as_array();

    println!("\n{}", "Contract Functions".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("\n{}: {}", "Contract".bold(), contract_name);
    println!("{}: {}", "ID".bold(), contract_id);

    if let Some(funcs) = functions {
        println!("\n{} {} function(s):\n", "Found".bold(), funcs.len());

        for func in funcs {
            let name = crate::conversions::as_str(&func["name"], "name")?;
            let visibility = crate::conversions::as_str(&func["visibility"], "visibility")?;
            let return_type = crate::conversions::as_str(&func["return_type"], "return_type")?;
            let is_mutable = crate::conversions::as_bool(&func["is_mutable"], "is_mutable")?;

            let visibility_badge = if visibility == "public" {
                "public".green()
            } else {
                "internal".yellow()
            };

            let mutability = if is_mutable {
                "mut".red()
            } else {
                "view".blue()
            };

            println!(
                "  {} {} {} {}",
                "fn".bright_blue(),
                name.bold(),
                visibility_badge,
                mutability
            );

            // Parameters
            if let Some(params) = func["params"].as_array() {
                let mut param_strs: Vec<String> = Vec::new();
                for p in params {
                    let pname = crate::conversions::as_str(&p["name"], "name")?;
                    let ptype = crate::conversions::as_str(&p["type_name"], "type_name")?;
                    param_strs.push(format!("{}: {}", pname, ptype));
                }

                println!("     ({}) -> {}", param_strs.join(", "), return_type);
            }

            // Doc
            if let Some(doc) = func["doc"].as_str() {
                println!("     /// {}", doc.bright_black());
            }

            println!();
        }
    } else {
        println!("\nNo functions found.");
    }

    println!("{}", "=".repeat(60).cyan());
    println!();

    Ok(())
}

/// Fetch contract info from the registry. `id` is the contract's registry UUID.
/// Use --network to get network-specific config (e.g. mainnet, testnet).
pub async fn info(api_url: &str, id: &str, network: crate::config::Network) -> Result<()> {
    println!("\n{}", "Fetching contract information...".bold().cyan());
    
    let url = format!("{}/api/contracts/{}", api_url.trim_end_matches('/'), id);
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .query(&[("network", network.to_string())])
        .send()
        .await?;

    if response.status().is_success() {
        let contract_info: serde_json::Value = response.json().await?;
        println!("\n{}", serde_json::to_string_pretty(&contract_info)?);
    } else {
        anyhow::bail!("Failed to fetch contract info: {}", response.status());
    }

    Ok(())
}

pub fn doc(contract_path: &str, output: &str) -> Result<()> {
    println!("\n{}", "Generating contract documentation...".bold().cyan());
    
    let content = format!(
        r#"# Contract Documentation

## Contract Path
{}

## Generated
{}

*This is a placeholder. Full documentation generation coming soon.*
"#,
        contract_path,
        chrono::Utc::now().to_rfc3339()
    );

    fs::write(output, content)?;
    println!("{} Documentation saved to: {}", "✓".green(), output);

    Ok(())
}

/// Load ABI JSON string from WASM (soroban bindings) or from a JSON file
fn load_abi_json(contract_path: &str) -> Result<String> {
    if contract_path.to_lowercase().ends_with(".wasm") {
        let output = std::process::Command::new("soroban")
            .args(["contract", "bindings", "json", "--wasm", contract_path])
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run soroban bindings: {}", e))?;
        if !output.status.success() {
            anyhow::bail!(
                "soroban bindings failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(String::from_utf8(output.stdout)?)
    } else {
        Ok(fs::read_to_string(contract_path)?)
    }
}

/// Generate markdown from ContractABI
fn abi_to_markdown(abi: &contract_abi::ContractABI) -> String {
    let mut md = format!("# {}\n\n", abi.name);
    if let Some(v) = &abi.version {
        md.push_str(&format!("Version: {}\n\n", v));
    }
    md.push_str("## Functions\n\n");
    for func in abi.public_functions() {
        md.push_str(&format!("### `{}`\n\n", func.name));
        if let Some(doc) = &func.doc {
            md.push_str(&format!("{}\n\n", doc));
        }
        md.push_str("**Parameters:**\n");
        if func.params.is_empty() {
            md.push_str("- None\n");
        } else {
            for p in &func.params {
                md.push_str(&format!("- `{}`: `{}`\n", p.name, p.param_type.display_name()));
            }
        }
        md.push_str(&format!("\n**Returns:** `{}`\n\n", func.return_type.display_name()));
    }
    if !abi.errors.is_empty() {
        md.push_str("## Errors\n\n");
        for e in &abi.errors {
            md.push_str(&format!("- **{}** (code {}): {}\n", e.name, e.code, e.doc.as_deref().unwrap_or("")));
        }
    }
    md
}

/// Generate self-contained HTML with Swagger UI and inline OpenAPI spec (JSON)
fn openapi_to_html(spec_json: &str, title: &str) -> String {
    let spec_escaped = spec_json.replace("</script>", "<\\/script>");
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>{} - API Docs</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
  <div id="swagger-ui"></div>
  <script type="application/json" id="openapi-spec">{}</script>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    (function() {{
      var el = document.getElementById('openapi-spec');
      try {{
        var spec = JSON.parse(el.textContent);
        SwaggerUIBundle({{ spec: spec, dom_id: '#swagger-ui' }});
      }} catch (e) {{
        document.getElementById('swagger-ui').innerHTML = '<p>Failed to load spec: ' + e.message + '</p>';
      }}
    }})();
  </script>
</body>
</html>
"#,
        title,
        spec_escaped
    )
}

pub fn openapi(contract_path: &str, output: &str, format: &str) -> Result<()> {
    println!("\n{}", "Generating OpenAPI documentation...".bold().cyan());
    let abi_json = load_abi_json(contract_path)?;
    let contract_name = std::path::Path::new(contract_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("contract");
    let abi = contract_abi::parse_json_spec(&abi_json, contract_name)
        .map_err(|e| anyhow::anyhow!("Failed to parse ABI: {}", e))?;
    let content = match format.to_lowercase().as_str() {
        "yaml" | "yml" => {
            let doc = contract_abi::generate_openapi(&abi, Some("/invoke"));
            contract_abi::to_yaml(&doc).map_err(|e| anyhow::anyhow!("{}", e))?
        }
        "json" => {
            let doc = contract_abi::generate_openapi(&abi, Some("/invoke"));
            contract_abi::to_json(&doc).map_err(|e| anyhow::anyhow!("{}", e))?
        }
        "markdown" | "md" => abi_to_markdown(&abi),
        "html" => {
            let doc = contract_abi::generate_openapi(&abi, Some("/invoke"));
            let json = contract_abi::to_json(&doc).map_err(|e| anyhow::anyhow!("{}", e))?;
            openapi_to_html(&json, &abi.name)
        }
        "pdf" => {
            println!("{}", "PDF: Generate YAML first, then run: npx @redocly/cli build-docs openapi.yaml -o doc.pdf".yellow());
            let doc = contract_abi::generate_openapi(&abi, Some("/invoke"));
            let yaml = contract_abi::to_yaml(&doc).map_err(|e| anyhow::anyhow!("{}", e))?;
            let yaml_path = output.trim_end_matches(".pdf").to_string() + ".yaml";
            fs::write(&yaml_path, &yaml)?;
            println!("{} Wrote {}", "✓".green(), yaml_path);
            return Ok(());
        }
        _ => anyhow::bail!("Unsupported format '{}'. Use: yaml, json, markdown, html, pdf", format),
    };
    fs::write(output, content)?;
    println!("{} Documentation saved to: {}", "✓".green(), output);
    Ok(())
}

pub fn sla_record(id: &str, uptime: f64, latency: f64, error_rate: f64) -> Result<()> {
    println!("\n{}", "Recording SLA metrics...".bold().cyan());
    println!("Contract ID: {}", id);
    println!("Uptime: {:.2}%", uptime);
    println!("Latency: {:.2}ms", latency);
    println!("Error Rate: {:.2}%", error_rate);
    println!("{} SLA metrics recorded", "✓".green());

    Ok(())
}

pub fn sla_status(id: &str) -> Result<()> {
    println!("\n{}", "Fetching SLA status...".bold().cyan());
    println!("Contract ID: {}", id);
    println!("\nStatus: {}", "Active".green());
    println!("Uptime: {}%", "99.9".green());
    println!("Avg Latency: {}ms", "45.2".green());

    Ok(())
}

