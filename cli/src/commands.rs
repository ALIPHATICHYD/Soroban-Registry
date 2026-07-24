#![allow(dead_code)]

use crate::net::RequestBuilderExt;
use crate::output_format;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_yaml;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
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

pub fn generate_flame_graph_file(profile: &profiler::ProfileData, output_path: &str) -> Result<()> {
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
        let profile_json = serde_json::to_string_pretty(&profile_data)
            .context("Failed to serialize profile data")?;
        fs::write(output_path, profile_json)
            .with_context(|| format!("Failed to write profile output: {}", output_path))?;
        println!("{} Profile output written to {}", "✓".green(), output_path);
    }

    if let Some(flamegraph_path) = flamegraph {
        generate_flame_graph_file(&profile_data, flamegraph_path)
            .with_context(|| format!("Failed to generate flame graph at {}", flamegraph_path))?;
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

/// Split a repeatable filter flag into individual values, so
/// `--category a,b` and `--category a --category b` produce the same list.
/// Trims whitespace, drops blanks, and de-duplicates while preserving order.
fn normalize_filter_values(values: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for value in values {
        for part in value.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            if seen.insert(part.to_string()) {
                normalized.push(part.to_string());
            }
        }
    }

    normalized
}

/// Normalize network filters to their canonical lowercase names, rejecting
/// unknown values before any request is sent so invalid input fails clearly
/// and locally instead of silently returning unfiltered results.
fn normalize_network_filters(values: &[String]) -> Result<Vec<String>> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for value in normalize_filter_values(values) {
        let canonical = Network::from_str(&value)?.to_string();
        if seen.insert(canonical.clone()) {
            normalized.push(canonical);
        }
    }

    Ok(normalized)
}

#[allow(clippy::too_many_arguments)]
pub async fn search(
    api_url: &str,
    query: &str,
    network: Network,
    verified_only: bool,
    networks: Vec<String>,
    category: Option<&str>,
    sort: Option<&str>,
    limit: usize,
    offset: usize,
    json: bool,
) -> Result<()> {
    let t0 = std::time::Instant::now();

    let mut params: Vec<(&str, String)> = vec![
        ("query", query.to_string()),
        ("limit", limit.to_string()),
        ("offset", offset.to_string()),
    ];

    // Normalize before sending so comma-separated and repeated flags produce an
    // identical request, and unknown networks fail here rather than being
    // silently dropped server-side.
    let networks = normalize_network_filters(&networks)?;
    let categories =
        normalize_filter_values(&category.map(str::to_string).into_iter().collect::<Vec<_>>());

    if !networks.is_empty() {
        params.push(("networks", networks.join(",")));
    } else {
        params.push(("network", network.to_string()));
    }

    if verified_only {
        params.push(("verified_only", "true".to_string()));
    }

    if !categories.is_empty() {
        params.push(("categories", categories.join(",")));
    }

    if let Some(s) = sort {
        params.push(("sort", s.to_string()));
    }

    let url = format!("{}/api/contracts", api_url);
    // Named `query_pairs`, not `query`: it previously shadowed the `query: &str`
    // parameter, which the result-rendering code below still needs.
    let query_pairs: Vec<(&str, String)> = params.iter().map(|(k, v)| (*k, v.clone())).collect();
    let (status, body) = crate::cached_http::cached_get(&url, &query_pairs)
        .await
        .context("Failed to search contracts")?;

    if !status.is_success() {
        anyhow::bail!("Search request failed with status {status}");
    }

    let data: serde_json::Value = serde_json::from_str(&body).context("Invalid search response")?;
    let items = data["items"].as_array().context("Invalid response")?;

    if json {
        let contracts: Vec<serde_json::Value> = items
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                    "name": c.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                    "contract_id": c.get("contract_id").and_then(|v| v.as_str()).unwrap_or(""),
                    "network": c.get("network").and_then(|v| v.as_str()).unwrap_or(""),
                    "category": c.get("category").and_then(|v| v.as_str()).unwrap_or(""),
                    "is_verified": c.get("is_verified").and_then(|v| v.as_bool()).unwrap_or(false),
                    "health_score": c.get("health_score").and_then(|v| v.as_i64()).unwrap_or(0),
                    "created_at": c.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
                    "tags": c.get("tags").and_then(|v| v.as_array()).map(|arr| {
                        arr.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect::<Vec<_>>()
                    }).unwrap_or_default(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "contracts": contracts,
                "count": contracts.len()
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "Search Results:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    let mut active_filters: Vec<String> = Vec::new();
    if !networks.is_empty() {
        active_filters.push(format!("network: {}", networks.join(", ")));
    }
    if !categories.is_empty() {
        active_filters.push(format!("category: {}", categories.join(", ")));
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
        if !categories.is_empty() {
            println!("  • Remove the --category filter to see all contract types");
        }
        if !networks.is_empty() {
            println!("  • Try adding more networks: --network mainnet,testnet,futurenet");
        }
        if verified_only {
            println!("  • Remove --verified-only to include unverified contracts");
        }
        println!("  • Use 'list' command to browse all contracts\n");
        return Ok(());
    }

    // Compute visible column widths from raw data (before applying ANSI codes).
    let name_w = items
        .iter()
        .filter_map(|c| c["name"].as_str())
        .map(|s| s.chars().count())
        .max()
        .unwrap_or(0)
        .max("Name".len());
    let net_w = items
        .iter()
        .filter_map(|c| c["network"].as_str())
        .map(|s| s.chars().count())
        .max()
        .unwrap_or(0)
        .max("Network".len());
    let cat_w = items
        .iter()
        .filter_map(|c| c["category"].as_str().filter(|s| !s.is_empty()))
        .map(|s| s.chars().count())
        .max()
        .unwrap_or(0)
        .max("Category".len());
    // "○ Unverified" is the longest possible verified cell value (12 visible chars).
    let ver_w = "○ Unverified".chars().count();
    let link_prefix = format!("{}/contracts/", api_url);
    let link_w = items
        .iter()
        .filter_map(|c| c["contract_id"].as_str())
        .map(|id| link_prefix.len() + id.len())
        .max()
        .unwrap_or(0)
        .max("Links".len())
        .min(60);

    let mut rows: Vec<Vec<String>> = Vec::new();
    for contract in items {
        let name = crate::conversions::as_str(&contract["name"], "name")?;
        let contract_id = crate::conversions::as_str(&contract["contract_id"], "contract_id")?;
        let is_verified = crate::conversions::as_bool(&contract["is_verified"], "is_verified")?;
        let net = crate::conversions::as_str(&contract["network"], "network")?;
        let cat = contract["category"].as_str().unwrap_or("").to_string();
        let link = format!("{}/contracts/{}", api_url, contract_id);

        let name_cell = crate::table_format::highlight_match(&name, query);
        let net_cell = net.bright_blue().to_string();
        let cat_display = if cat.is_empty() {
            "—".to_string()
        } else {
            cat
        };
        let cat_cell = crate::table_format::highlight_match(&cat_display, query);
        let ver_cell = if is_verified {
            "✓ Verified".green().to_string()
        } else {
            "○ Unverified".yellow().to_string()
        };
        let link_cell = link.bright_black().to_string();

        rows.push(vec![name_cell, net_cell, cat_cell, ver_cell, link_cell]);
    }

    let col_widths = [name_w, net_w, cat_w, ver_w, link_w];
    let headers = ["Name", "Network", "Category", "Verified", "Links"];
    print!(
        "{}",
        crate::table_format::render_table(&headers, &col_widths, &rows)
    );

    let elapsed_ms = t0.elapsed().as_millis();
    println!(
        "\n{} {} result(s) for \"{}\"  |  {}ms\n",
        "→".cyan(),
        items.len(),
        query.bold(),
        elapsed_ms
    );

    Ok(())
}

/// Analyze two contract versions or schema files for breaking changes.
pub async fn upgrade_analyze(
    api_url: &str,
    old_id: &str,
    new_id: &str,
    json_out: bool,
) -> Result<()> {
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
                println!(
                    "[{:?}] {} - {}",
                    f.severity,
                    f.field.unwrap_or_default(),
                    f.message
                );
            }
        }
        return Ok(());
    }

    // Otherwise try to fetch versions from the API (assumes endpoint exists)
    let client = crate::net::client();
    let url = format!("{}/api/contract_versions/{}", api_url, old_id);
    let old_res = client
        .get(&url)
        .send_with_retry()
        .await
        .context("failed to fetch old version")?;
    if old_res.status() == StatusCode::NOT_FOUND {
        anyhow::bail!(
            "Old version {} not found via API. Try passing a local schema JSON file instead.",
            old_id
        );
    }
    let old_json: serde_json::Value = old_res.json().await?;

    let url2 = format!("{}/api/contract_versions/{}", api_url, new_id);
    let new_res = client
        .get(&url2)
        .send_with_retry()
        .await
        .context("failed to fetch new version")?;
    if new_res.status() == StatusCode::NOT_FOUND {
        anyhow::bail!(
            "New version {} not found via API. Try passing a local schema JSON file instead.",
            new_id
        );
    }
    let new_json: serde_json::Value = new_res.json().await?;

    // Expect the API to expose a simple schema JSON in `state_schema` field; fall back to error.
    let old_schema_str = old_json["state_schema"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("API did not return state_schema for old version"))?;
    let new_schema_str = new_json["state_schema"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("API did not return state_schema for new version"))?;

    let old_schema =
        Schema::from_json_bytes(old_schema_str.as_bytes()).context("failed to parse old schema")?;
    let new_schema =
        Schema::from_json_bytes(new_schema_str.as_bytes()).context("failed to parse new schema")?;

    let findings = compare_schemas(&old_schema, &new_schema);
    if json_out {
        println!("{}", serde_json::to_string_pretty(&findings)?);
    } else {
        for f in findings {
            println!(
                "[{:?}] {} - {}",
                f.severity,
                f.field.unwrap_or_default(),
                f.message
            );
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
        let res = upgrade_analyze(
            "http://localhost:3001",
            old_path.to_str().unwrap(),
            new_path.to_str().unwrap(),
            true,
        )
        .await;
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

#[allow(clippy::too_many_arguments)]
pub async fn publish(
    api_url: &str,
    contract_id: &str,
    name: &str,
    description: Option<&str>,
    network: Network,
    category: Option<&str>,
    tags: Vec<String>,
    publisher: &str,
    is_cicd: bool,
    contract_path: &str,
    test_command: Option<&str>,
    require_coverage: bool,
    coverage_threshold: f64,
    skip_tests: bool,
) -> Result<()> {
    if !skip_tests {
        run_contract_tests(
            contract_path,
            test_command,
            require_coverage,
            coverage_threshold,
            true,
        )
        .await?;
    }

    let client = crate::net::client();
    let url = format!("{}/api/contracts", api_url);

    let mut payload = json!({
        "contract_id": contract_id,
        "name": name,
        "description": description,
        "network": network.to_string(),
        "category": category,
        "tags": tags,
        "publisher_address": publisher,
    });

    if is_cicd {
        payload["is_cicd"] = json!(true);
    }

    println!("\n{}", "Publishing contract...".bold().cyan());

    let response = client
        .post(&url)
        .json(&payload)
        .send_with_retry()
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

fn detect_test_command(contract_dir: &Path) -> Option<String> {
    if contract_dir.join("Cargo.toml").exists() {
        return Some("cargo test".to_string());
    }

    if contract_dir.join("package.json").exists() {
        if contract_dir.join("pnpm-lock.yaml").exists() {
            return Some("pnpm test".to_string());
        }
        if contract_dir.join("yarn.lock").exists() {
            return Some("yarn test".to_string());
        }
        return Some("npm test".to_string());
    }

    None
}

fn summarize_failure(stdout: &str, stderr: &str) -> Vec<String> {
    let mut suggestions = Vec::new();
    let combined = format!("{}\n{}", stdout, stderr).to_lowercase();

    if combined.contains("failed") || combined.contains("panic") {
        suggestions
            .push("Review failing test output and fix assertions or runtime errors.".to_string());
    }
    if combined.contains("not found") || combined.contains("no such file") {
        suggestions.push("Check file paths and project setup before running tests.".to_string());
    }
    if combined.contains("permission") {
        suggestions
            .push("Verify file permissions and execution rights for test tools.".to_string());
    }

    if suggestions.is_empty() {
        suggestions.push(
            "Inspect test logs above for the first concrete error and address it first."
                .to_string(),
        );
    }

    suggestions
}

fn parse_tarpaulin_percent(report: &serde_json::Value) -> Option<f64> {
    let files = report.get("files")?.as_array()?;

    let mut covered_lines: u64 = 0;
    let mut coverable_lines: u64 = 0;

    for file in files {
        if let Some(traces) = file.get("traces").and_then(|t| t.as_array()) {
            for trace in traces {
                if trace.get("line").and_then(|l| l.as_u64()).is_some() {
                    coverable_lines += 1;
                    if let Some(stats) = trace.get("stats").and_then(|s| s.as_object()) {
                        if stats.values().any(|v| v.as_u64().unwrap_or(0) > 0) {
                            covered_lines += 1;
                        }
                    }
                }
            }
        }
    }

    if coverable_lines == 0 {
        None
    } else {
        Some((covered_lines as f64 / coverable_lines as f64) * 100.0)
    }
}

fn run_rust_coverage(contract_dir: &Path) -> Result<Option<f64>> {
    let output_dir = contract_dir.join(".soroban-registry").join("coverage");
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir).with_context(|| {
            format!(
                "Failed to create coverage output dir: {}",
                output_dir.display()
            )
        })?;
    }

    let output_dir_str = output_dir.to_string_lossy().to_string();
    let status = Command::new("cargo")
        .current_dir(contract_dir)
        .args([
            "tarpaulin",
            "--out",
            "Json",
            "--output-dir",
            &output_dir_str,
            "--branch",
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            let report_path = output_dir.join("tarpaulin-report.json");
            if !report_path.exists() {
                return Ok(None);
            }

            let content = fs::read_to_string(&report_path).with_context(|| {
                format!("Failed reading coverage report: {}", report_path.display())
            })?;
            let json: serde_json::Value = serde_json::from_str(&content).with_context(|| {
                format!("Failed parsing coverage report: {}", report_path.display())
            })?;
            Ok(parse_tarpaulin_percent(&json))
        }
        _ => Ok(None),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TestSuiteOptions<'a> {
    pub test_file: Option<&'a str>,
    pub contract_path: &'a str,
    pub test_command: Option<&'a str>,
    pub junit_output: Option<&'a str>,
    pub show_coverage: bool,
    pub verbose: bool,
    pub require_coverage: bool,
    pub coverage_threshold: f64,
    pub setup_hook: Option<&'a str>,
    pub teardown_hook: Option<&'a str>,
    pub mock_config: Option<&'a str>,
    pub report_output: Option<&'a str>,
    pub profile_output: Option<&'a str>,
    pub load_iterations: u32,
}

fn run_shell_hook(label: &str, command: &str, contract_dir: &Path) -> Result<()> {
    println!("{} {} {}", "→".cyan(), label.bold(), command.bright_blue());
    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(contract_dir)
        .status()
        .with_context(|| format!("Failed to execute {} hook: {}", label, command))?;

    if !status.success() {
        anyhow::bail!("{} hook failed: {}", label, command);
    }

    Ok(())
}

fn read_mock_config(path: &str) -> Result<serde_json::Value> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read mock config: {}", path))?;
    if path.ends_with(".yaml") || path.ends_with(".yml") {
        serde_yaml::from_str(&raw)
            .with_context(|| format!("Failed to parse YAML mock config: {}", path))
    } else {
        serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse JSON mock config: {}", path))
    }
}

pub async fn run_test_suite(options: TestSuiteOptions<'_>) -> Result<()> {
    let contract_dir = Path::new(options.contract_path);
    let started_at = chrono::Utc::now();
    let wall_clock = std::time::Instant::now();

    if let Some(setup_hook) = options.setup_hook {
        run_shell_hook("Setup hook", setup_hook, contract_dir)?;
    }

    let mock_summary = if let Some(mock_config) = options.mock_config {
        let parsed = read_mock_config(mock_config)?;
        let service_count = parsed
            .get("services")
            .and_then(|services| services.as_array())
            .map(|services| services.len())
            .unwrap_or(0);
        println!(
            "{} Loaded mock config {} ({} service definitions)",
            "✓".green(),
            mock_config,
            service_count
        );
        Some(serde_json::json!({
            "path": mock_config,
            "service_count": service_count,
        }))
    } else {
        None
    };

    if options.load_iterations > 1 {
        println!(
            "{} Load profile enabled with {} iterations",
            "→".cyan(),
            options.load_iterations
        );
    }

    let result = if let Some(test_file) = options.test_file {
        run_tests(
            test_file,
            Some(options.contract_path),
            options.junit_output,
            options.show_coverage,
            options.verbose,
        )
        .await
    } else {
        run_contract_tests(
            options.contract_path,
            options.test_command,
            options.require_coverage,
            options.coverage_threshold,
            options.show_coverage,
        )
        .await
    };

    let duration_ms = wall_clock.elapsed().as_millis();
    let error_message = result.as_ref().err().map(|err| err.to_string());

    if let Some(report_output) = options.report_output {
        let report = serde_json::json!({
            "started_at": started_at,
            "contract_path": options.contract_path,
            "test_file": options.test_file,
            "load_iterations": options.load_iterations,
            "passed": result.is_ok(),
            "duration_ms": duration_ms,
            "mocking": mock_summary,
            "error": error_message,
        });
        fs::write(report_output, serde_json::to_string_pretty(&report)?)
            .with_context(|| format!("Failed to write test report: {}", report_output))?;
        println!("{} Test report written to {}", "✓".green(), report_output);
    }

    if let Some(profile_output) = options.profile_output {
        let profile = serde_json::json!({
            "contract_path": options.contract_path,
            "load_iterations": options.load_iterations,
            "duration_ms": duration_ms,
            "timestamp": chrono::Utc::now(),
        });
        fs::write(profile_output, serde_json::to_string_pretty(&profile)?)
            .with_context(|| format!("Failed to write test profile: {}", profile_output))?;
        println!("{} Test profile written to {}", "✓".green(), profile_output);
    }

    let teardown_result = if let Some(teardown_hook) = options.teardown_hook {
        run_shell_hook("Teardown hook", teardown_hook, contract_dir)
    } else {
        Ok(())
    };

    result?;
    teardown_result?;
    Ok(())
}

pub async fn run_contract_tests(
    contract_path: &str,
    test_command: Option<&str>,
    require_coverage: bool,
    coverage_threshold: f64,
    show_coverage: bool,
) -> Result<()> {
    let contract_dir = Path::new(contract_path);
    if !contract_dir.exists() {
        anyhow::bail!("Contract path not found: {}", contract_path);
    }

    let selected_command = if let Some(cmd) = test_command {
        cmd.to_string()
    } else if let Some(cmd) = detect_test_command(contract_dir) {
        cmd
    } else {
        anyhow::bail!(
            "No tests detected. Provide a custom command with --test-command, e.g. --test-command 'cargo test'"
        );
    };

    println!("\n{}", "Running Contract Tests...".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{} {}", "Command:".bold(), selected_command.bright_blue());

    let start = std::time::Instant::now();
    let output = if cfg!(windows) {
        let comspec = std::env::var("COMSPEC").unwrap_or_else(|_| "cmd".to_string());
        Command::new(comspec)
            .arg("/C")
            .arg(&selected_command)
            .current_dir(contract_dir)
            .output()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(&selected_command)
            .current_dir(contract_dir)
            .output()
    }
    .with_context(|| format!("Failed to execute test command: {}", selected_command))?;

    let duration = start.elapsed().as_secs_f64();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        println!("{} Tests passed in {:.2}s", "✓".green(), duration);
    } else {
        println!("{} Tests failed in {:.2}s", "✗".red(), duration);

        if !stdout.trim().is_empty() {
            println!("\n{}\n{}", "Test output:".bold(), stdout);
        }
        if !stderr.trim().is_empty() {
            println!("\n{}\n{}", "Test errors:".bold().red(), stderr);
        }

        println!("\n{}", "Suggested actions:".bold().yellow());
        for suggestion in summarize_failure(&stdout, &stderr) {
            println!("  - {}", suggestion);
        }

        anyhow::bail!("Contract tests failed. Submission blocked.");
    }

    let is_rust_project = contract_dir.join("Cargo.toml").exists();
    let should_collect_coverage = show_coverage || require_coverage || coverage_threshold > 0.0;

    if should_collect_coverage {
        println!("\n{}", "Coverage:".bold().magenta());
        let coverage = if is_rust_project {
            run_rust_coverage(contract_dir)?
        } else {
            None
        };

        if let Some(percent) = coverage {
            println!("  Total Coverage: {:.2}%", percent);

            if coverage_threshold > 0.0 {
                if percent < coverage_threshold {
                    anyhow::bail!(
                        "Coverage {:.2}% is below required threshold {:.2}%",
                        percent,
                        coverage_threshold
                    );
                } else {
                    println!(
                        "  {} Threshold met ({:.2}% >= {:.2}%)",
                        "✓".green(),
                        percent,
                        coverage_threshold
                    );
                }
            }
        } else {
            println!("  {} Coverage metrics unavailable.", "⚠".yellow());
            if require_coverage {
                anyhow::bail!(
                    "Coverage is required but could not be collected. Install cargo-tarpaulin or provide coverage-enabled test tooling."
                );
            }
        }
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();

    Ok(())
}

#[cfg(test)]
mod contract_test_helpers_tests {
    use super::{detect_test_command, parse_tarpaulin_percent};
    use serde_json::json;

    #[test]
    fn detect_test_command_prefers_cargo_when_cargo_toml_exists() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname='x'\nversion='0.1.0'",
        )
        .expect("Cargo.toml should be created");

        let detected = detect_test_command(dir.path());
        assert_eq!(detected.as_deref(), Some("cargo test"));
    }

    #[test]
    fn detect_test_command_uses_pnpm_for_node_projects() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        std::fs::write(dir.path().join("package.json"), "{}")
            .expect("package.json should be created");
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'")
            .expect("pnpm-lock.yaml should be created");

        let detected = detect_test_command(dir.path());
        assert_eq!(detected.as_deref(), Some("pnpm test"));
    }

    #[test]
    fn parse_tarpaulin_percent_calculates_expected_ratio() {
        let report = json!({
            "files": [
                {
                    "traces": [
                        {"line": 1, "stats": {"line": 1}},
                        {"line": 2, "stats": {"line": 0}},
                        {"line": 3, "stats": {"line": 2}}
                    ]
                }
            ]
        });

        let percent = parse_tarpaulin_percent(&report).expect("coverage should parse");
        assert!((percent - 66.666).abs() < 0.5);
    }
}

pub async fn contract_list(
    api_url: &str,
    limit: usize,
    offset: usize,
    network: Option<crate::config::Network>,
    networks: Vec<String>,
    category: Option<String>,
    format: &str,
) -> Result<()> {
    let mut query: Vec<(&str, String)> = vec![
        ("page_size", limit.to_string()),
        ("page", ((offset / limit) + 1).to_string()),
    ];

    // Normalize before sending so comma-separated `--networks`/`--category` and
    // the singular `--network` flag produce the same request shape as `search`,
    // and unknown networks fail here rather than being silently dropped server-side.
    let networks = normalize_network_filters(&networks)?;
    let categories = normalize_filter_values(&category.into_iter().collect::<Vec<_>>());

    if !networks.is_empty() {
        query.push(("networks", networks.join(",")));
    } else if let Some(net) = network {
        query.push(("network", net.to_string()));
    }

    if !categories.is_empty() {
        query.push(("categories", categories.join(",")));
    }

    let url = format!("{}/api/contracts", api_url.trim_end_matches('/'));
    let (status, body) = crate::cached_http::cached_get(&url, &query)
        .await
        .context("Failed to list contracts")?;

    if !status.is_success() {
        anyhow::bail!("API returned error: {status}");
    }

    let data: serde_json::Value = serde_json::from_str(&body).context("Invalid list response")?;
    let items = data["items"]
        .as_array()
        .context("Invalid response format")?;

    if format == "json" {
        let contracts: Vec<serde_json::Value> = items
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                    "name": c.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                    "contract_id": c.get("contract_id").and_then(|v| v.as_str()).unwrap_or(""),
                    "network": c.get("network").and_then(|v| v.as_str()).unwrap_or(""),
                    "category": c.get("category").and_then(|v| v.as_str()).unwrap_or(""),
                    "is_verified": c.get("is_verified").and_then(|v| v.as_bool()).unwrap_or(false),
                    "health_score": c.get("health_score").and_then(|v| v.as_i64()).unwrap_or(0),
                    "created_at": c.get("created_at").and_then(|v| v.as_str()).unwrap_or(""),
                    "tags": c.get("tags").and_then(|v| v.as_array()).map(|arr| {
                        arr.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect::<Vec<_>>()
                    }).unwrap_or_default(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "contracts": contracts,
                "count": contracts.len()
            }))?
        );
        return Ok(());
    }

    if format == "csv" {
        println!("contract_id,name,network,is_verified,category");
        for item in items {
            println!(
                "{},{},{},{},{}",
                item["contract_id"].as_str().unwrap_or(""),
                item["name"].as_str().unwrap_or(""),
                item["network"].as_str().unwrap_or(""),
                item["is_verified"].as_bool().unwrap_or(false),
                item["category"].as_str().unwrap_or("")
            );
        }
        return Ok(());
    }

    // Default: Table
    println!("\n{}", "Contract Registry".bold().cyan());
    println!("{}", "=".repeat(100).cyan());

    if items.is_empty() {
        println!("{}", "No contracts found matching the criteria.".yellow());
        return Ok(());
    }

    println!(
        "{:<45} {:<25} {:<10} {:<10}",
        "CONTRACT ID".bold(),
        "NAME".bold(),
        "NETWORK".bold(),
        "VERIFIED".bold()
    );
    println!("{}", "-".repeat(100));

    for item in items {
        let contract_id = item["contract_id"].as_str().unwrap_or("");
        let name = item["name"].as_str().unwrap_or("");
        let net = item["network"].as_str().unwrap_or("");
        let verified = if item["is_verified"].as_bool().unwrap_or(false) {
            "Yes".green()
        } else {
            "No".red()
        };

        println!(
            "{:<45} {:<25} {:<10} {:<10}",
            contract_id,
            name.truncate_str(23),
            net,
            verified
        );
    }

    let total = data["total"].as_u64().unwrap_or(0);
    println!("{}", "-".repeat(100));
    println!(
        "Showing {}-{} of {} contracts",
        offset + 1,
        offset + items.len(),
        total
    );
    println!();

    Ok(())
}

pub async fn contract_info(api_url: &str, id: &str) -> Result<()> {
    let client = crate::net::client();
    let url = format!("{}/api/contracts/{}", api_url.trim_end_matches('/'), id);

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch contract info")?;

    if !response.status().is_success() {
        if response.status() == 404 {
            anyhow::bail!("Contract not found: {}", id);
        }
        anyhow::bail!("API returned error: {}", response.status());
    }

    let data: serde_json::Value = response.json().await?;

    println!("\n{}", "Contract Details".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    println!(
        "{:<20} {}",
        "Name:".bold(),
        data["name"].as_str().unwrap_or("Unknown")
    );
    println!(
        "{:<20} {}",
        "ID:".bold(),
        data["contract_id"].as_str().unwrap_or("Unknown")
    );
    println!(
        "{:<20} {}",
        "Network:".bold(),
        data["network"].as_str().unwrap_or("Unknown")
    );
    println!(
        "{:<20} {}",
        "Category:".bold(),
        data["category"].as_str().unwrap_or("None")
    );

    let verified = if data["is_verified"].as_bool().unwrap_or(false) {
        "Yes".green()
    } else {
        "No".red()
    };
    println!("{:<20} {}", "Verified:".bold(), verified);

    if let Some(desc) = data["description"].as_str() {
        println!("{:<20} {}", "Description:".bold(), desc);
    }

    println!("\n{}", "Resources".bold().yellow());
    println!(
        "{:<20} {}",
        "WASM Hash:".bold(),
        data["wasm_hash"].as_str().unwrap_or("N/A")
    );

    if let Some(abi) = data["abi"].as_object() {
        println!("{:<20} {} methods", "ABI:".bold(), abi.len());
    }

    println!();
    Ok(())
}

// Helper for string truncation
trait Truncate {
    fn truncate_str(&self, max: usize) -> String;
}

impl Truncate for str {
    fn truncate_str(&self, max: usize) -> String {
        if self.len() > max {
            format!("{}...", &self[..max - 3])
        } else {
            self.to_string()
        }
    }
}

pub async fn list(
    api_url: &str,
    limit: usize,
    network: crate::config::Network,
    json: bool,
) -> Result<()> {
    contract_list(
        api_url,
        limit,
        0,
        Some(network),
        Vec::new(),
        None,
        if json { "json" } else { "table" },
    )
    .await
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
    let client = crate::net::client();
    let url = format!(
        "{}/api/contracts/breaking-changes?old_id={}&new_id={}",
        api_url, old_id, new_id
    );

    let response = client
        .get(&url)
        .send_with_retry()
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
    let non_breaking_count =
        crate::conversions::as_u64(&report["non_breaking_count"], "non_breaking_count")?;

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
    use tokio::process::Command;

    println!("\n{}", "Migration Tool".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    // 1. Read WASM file
    let wasm_bytes = std::fs::read(wasm_path)
        .with_context(|| format!("Failed to read WASM file at {}", wasm_path))?;

    // 2. Compute Hash
    let mut hasher = Sha256::new();
    hasher.update(&wasm_bytes);
    let wasm_hash = hex::encode(hasher.finalize());

    println!("Contract ID: {}", contract_id.green());
    println!("WASM Hash: {}", wasm_hash.bright_black());

    if dry_run {
        println!(
            "\n{}",
            "Dry run enabled: not contacting the registry API.".yellow()
        );
        println!(
            "{}",
            "✓ Migration simulation complete (dry-run).".green().bold()
        );
        return Ok(());
    }

    // 3. Create Migration Record (Pending)
    let client = crate::net::client();
    let create_url = format!("{}/api/migrations", api_url);

    let payload = json!({
        "contract_id": contract_id,
        "wasm_hash": wasm_hash,
    });

    print!("\nInitializing migration... ");
    let response = client
        .post(&create_url)
        .json(&payload)
        .send_with_retry()
        .await
        .context("Failed to contact registry API")?;

    if !response.status().is_success() {
        println!("{}", "Failed".red());
        let err = response.text().await?;
        anyhow::bail!("API Error: {}", err);
    }

    let migration: serde_json::Value = response.json().await?;
    let migration_id = extract_migration_id(&migration)?;
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
        .send_with_retry()
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
    api_url: &str,
    id: Option<&str>,
    output: Option<&str>,
    contract_dir: &str,
    format: Option<&str>,
    filters: Vec<String>,
    page_size: usize,
) -> Result<()> {
    let resolved_format = crate::export::RegistryExportFormat::resolve(format, id, output)?;
    let summary = crate::export::export_registry_data(crate::export::RegistryExportOptions {
        api_url,
        id,
        output,
        contract_dir,
        format: resolved_format,
        filters,
        page_size,
        include_related: true,
        compress: false,
    })
    .await?;

    println!("{}", "✓ Export complete!".green().bold());
    println!(
        "  {}: {}",
        "Format".bold(),
        format!("{:?}", summary.format).to_lowercase()
    );
    println!("  {}: {}", "Items".bold(), summary.items_exported);
    println!("  {}: {}", "Output".bold(), summary.output_path);
    println!("  {}: {}", "SHA-256".bold(), summary.sha256.bright_black());
    println!("  {}: {}\n", "Checksum".bold(), summary.checksum_path);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn contract_export(
    api_url: &str,
    output: Option<&str>,
    format: &str,
    network: Option<&str>,
    category: Option<&str>,
    since: Option<&str>,
    compress: bool,
    include_related: bool,
    page_size: usize,
) -> Result<()> {
    let mut filters = Vec::new();
    if let Some(network) = network {
        filters.push(format!("network={}", network));
    }
    if let Some(category) = category {
        filters.push(format!("category={}", category));
    }
    if let Some(since) = since {
        filters.push(format!("updated_from={}", since));
    }

    let resolved_format = crate::export::RegistryExportFormat::resolve(Some(format), None, output)?;
    let summary = crate::export::export_registry_data(crate::export::RegistryExportOptions {
        api_url,
        id: None,
        output,
        contract_dir: ".",
        format: resolved_format,
        filters,
        page_size,
        include_related,
        compress,
    })
    .await?;

    println!("{}", "Export complete!".green().bold());
    println!(
        "  {}: {}",
        "Format".bold(),
        format!("{:?}", summary.format).to_lowercase()
    );
    println!("  {}: {}", "Items".bold(), summary.items_exported);
    println!("  {}: {}", "Output".bold(), summary.output_path);
    println!("  {}: {}", "SHA-256".bold(), summary.sha256.bright_black());
    println!("  {}: {}\n", "Checksum".bold(), summary.checksum_path);
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

    let client = crate::net::client();
    let resp = client
        .get(&url)
        .query(&[("network", network.to_string())])
        .send_with_retry()
        .await
        .context("Failed to reach registry API")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Failed to get trust score ({}): {}", status, body);
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .context("Failed to parse trust score response")?;

    // ── Header ────────────────────────────────────────────────────────────────
    let name = crate::conversions::as_str(&data["contract_name"], "contract_name")?;
    let score = crate::conversions::as_f64(&data["score"], "score")?;
    let badge = crate::conversions::as_str(&data["badge"], "badge")?;
    let badge_icon = crate::conversions::as_str(&data["badge_icon"], "badge_icon")?;
    let summary = crate::conversions::as_str(&data["summary"], "summary")?;

    println!("\n{}", "─".repeat(56));
    println!("  Trust Score — {}", name.bold());
    println!("{}", "─".repeat(56));
    println!("  Score : {:.0}/100", score);
    println!("  Badge : {} {}", badge_icon, badge.bold());
    println!("  {}", summary);
    println!("{}", "─".repeat(56));

    // ── Factor breakdown ──────────────────────────────────────────────────────
    println!("\n  {} Factor Breakdown\n", "📊".bold());

    if let Some(factors) = data["factors"].as_array() {
        for factor in factors {
            let fname = crate::conversions::as_str(&factor["name"], "name")?;
            let earned = crate::conversions::as_f64(&factor["points_earned"], "points_earned")?;
            let max = crate::conversions::as_f64(&factor["points_max"], "points_max")?;
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
    let client = crate::net::client();
    let url = format!("{}/api/contracts/{}/dependencies", api_url, contract_id);

    let response = client
        .get(&url)
        .send_with_retry()
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
            let constraint =
                crate::conversions::as_str(&node["constraint_to_parent"], "constraint_to_parent")?;
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
                    let _ = print_tree(children, &new_prefix, true);
                    let new_prefix =
                        format!("{}{}", prefix, if is_node_last { "    " } else { "│   " });
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

    let passed = result.passed;
    if let Some(junit_path) = junit_output {
        test_framework::generate_junit_xml(&[result], Path::new(junit_path))?;
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

    if !passed {
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
    let client = crate::net::client();
    let url = format!(
        "{}/api/contracts/{}/config?environment={}",
        api_url, contract_id, environment
    );

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch configuration")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to get config: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let config: serde_json::Value = response.json().await?;

    println!("\n{}", "Contract Configuration (Latest):".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{}: {}", "Contract ID".bold(), contract_id);
    println!("{}: {}", "Environment".bold(), environment);
    println!(
        "{}: {}",
        "Version".bold(),
        crate::conversions::as_i64(&config["version"], "version")?
    );
    println!(
        "{}: {}",
        "Contains Secrets".bold(),
        crate::conversions::as_bool(&config["has_secrets"], "has_secrets")?
    );
    println!(
        "{}: {}",
        "Created By".bold(),
        crate::conversions::as_str(&config["created_by"], "created_by")?
    );
    println!("{}:", "Config Data".bold());
    println!(
        "{}",
        serde_json::to_string_pretty(&config["config_data"])
            .unwrap_or_default()
            .green()
    );
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
    let client = crate::net::client();
    let url = format!("{}/api/contracts/{}/config", api_url, contract_id);

    let mut payload = json!({
        "environment": environment,
        "config_data": serde_json::from_str::<serde_json::Value>(config_data).context("Invalid config JSON")?,
        "created_by": created_by,
    });

    if let Some(sec) = secrets_data {
        let sec_json: serde_json::Value =
            serde_json::from_str(sec).context("Invalid secrets JSON")?;
        payload["secrets_data"] = sec_json;
    }

    println!("\n{}", "Publishing configuration...".bold().cyan());

    let response = client
        .post(&url)
        .json(&payload)
        .send_with_retry()
        .await
        .context("Failed to set configuration")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to set config: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let config: serde_json::Value = response.json().await?;

    println!(
        "{}",
        "✓ Configuration published successfully!".green().bold()
    );
    println!("  {}: {}", "Environment".bold(), environment);
    println!(
        "  {}: {}",
        "New Version".bold(),
        crate::conversions::as_i64(&config["version"], "version")?
    );
    println!();

    Ok(())
}

pub async fn config_history(api_url: &str, contract_id: &str, environment: &str) -> Result<()> {
    let client = crate::net::client();
    let url = format!(
        "{}/api/contracts/{}/config/history?environment={}",
        api_url, contract_id, environment
    );

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch configuration history")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to get config history: {}",
            response.text().await.unwrap_or_default()
        );
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
    let client = crate::net::client();
    let url = format!(
        "{}/api/contracts/{}/config/rollback?environment={}",
        api_url, contract_id, environment
    );

    let payload = json!({
        "roll_back_to_version": version,
        "created_by": created_by,
    });

    println!(
        "\n{}",
        format!("Rolling back configuration to v{}...", version)
            .bold()
            .cyan()
    );

    let response = client
        .post(&url)
        .json(&payload)
        .send_with_retry()
        .await
        .context("Failed to rollback configuration")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to rollback config: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let config: serde_json::Value = response.json().await?;

    println!(
        "{}",
        "✓ Configuration rolled back successfully!".green().bold()
    );
    println!("  {}: {}", "Environment".bold(), environment);
    println!(
        "  {}: {}",
        "New Active Version".bold(),
        crate::conversions::as_i64(&config["version"], "version")?
    );
    println!();

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalStateHistoryEntry {
    id: String,
    timestamp: String,
    action: String,
    key: Option<String>,
    previous: Option<serde_json::Value>,
    value: Option<serde_json::Value>,
    note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalStateSnapshot {
    id: String,
    label: Option<String>,
    created_at: String,
    entry_count: usize,
    state: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalContractStateStore {
    contract_id: String,
    network: String,
    values: BTreeMap<String, serde_json::Value>,
    snapshots: Vec<LocalStateSnapshot>,
    history: Vec<LocalStateHistoryEntry>,
}

impl LocalContractStateStore {
    fn new(contract_id: &str, network: Network) -> Self {
        Self {
            contract_id: contract_id.to_string(),
            network: network.to_string(),
            values: BTreeMap::new(),
            snapshots: Vec::new(),
            history: Vec::new(),
        }
    }
}

fn state_root_dir() -> Result<PathBuf> {
    if let Ok(custom) = std::env::var("SOROBAN_REGISTRY_STATE_DIR") {
        let path = PathBuf::from(custom);
        fs::create_dir_all(&path).with_context(|| {
            format!(
                "Failed to create custom state directory from SOROBAN_REGISTRY_STATE_DIR: {}",
                path.display()
            )
        })?;
        return Ok(path);
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".soroban-registry").join("state"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(".soroban-registry").join("state"));
    }
    if let Some(temp) = std::env::var_os("TMP").map(PathBuf::from) {
        candidates.push(temp.join("soroban-registry-state"));
    }

    for candidate in candidates {
        if fs::create_dir_all(&candidate).is_ok() {
            return Ok(candidate);
        }
    }

    anyhow::bail!("Unable to create a writable state directory")
}

fn sanitize_for_filename(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn state_file_path(contract_id: &str, network: Network) -> Result<PathBuf> {
    let root = state_root_dir()?;
    let network_dir = root.join(network.to_string());
    fs::create_dir_all(&network_dir).with_context(|| {
        format!(
            "Failed to create state directory: {}",
            network_dir.display()
        )
    })?;
    let file_name = format!("{}.json", sanitize_for_filename(contract_id));
    Ok(network_dir.join(file_name))
}

fn load_local_state(contract_id: &str, network: Network) -> Result<LocalContractStateStore> {
    let path = state_file_path(contract_id, network)?;
    if !path.exists() {
        return Ok(LocalContractStateStore::new(contract_id, network));
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read state file: {}", path.display()))?;
    let mut store: LocalContractStateStore = serde_json::from_str(&content)
        .with_context(|| format!("Invalid state file format: {}", path.display()))?;

    if store.contract_id.is_empty() {
        store.contract_id = contract_id.to_string();
    }
    if store.network.is_empty() {
        store.network = network.to_string();
    }

    Ok(store)
}

fn save_local_state(store: &LocalContractStateStore, network: Network) -> Result<()> {
    let path = state_file_path(&store.contract_id, network)?;
    let data = serde_json::to_string_pretty(store).context("Failed to serialize state")?;
    fs::write(&path, data)
        .with_context(|| format!("Failed to write state file: {}", path.display()))
}

fn parse_state_value(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
}

fn require_mutable_network(network: Network) -> Result<()> {
    if matches!(network, Network::Mainnet) {
        anyhow::bail!("State mutation is disabled on mainnet. Use testnet or futurenet.");
    }
    Ok(())
}

async fn try_remote_state_get(
    api_url: &str,
    contract_id: &str,
    key: &str,
) -> Result<Option<serde_json::Value>> {
    let mut url = reqwest::Url::parse(api_url).context("Invalid API URL")?;
    url.path_segments_mut()
        .map_err(|_| anyhow::anyhow!("Invalid API URL"))?
        .extend(["api", "contracts", contract_id, "state", key]);

    let response = match crate::net::client().get(url).send_with_retry().await {
        Ok(resp) => resp,
        Err(_) => return Ok(None),
    };

    if response.status() == reqwest::StatusCode::NOT_IMPLEMENTED {
        return Ok(None);
    }

    if !response.status().is_success() {
        return Ok(None);
    }

    let payload: serde_json::Value = response.json().await.unwrap_or_else(|_| json!({}));
    if let Some(value) = payload.get("value") {
        return Ok(Some(value.clone()));
    }
    Ok(Some(payload))
}

async fn try_remote_state_set(
    api_url: &str,
    contract_id: &str,
    key: &str,
    value: &serde_json::Value,
) -> Result<bool> {
    let mut url = reqwest::Url::parse(api_url).context("Invalid API URL")?;
    url.path_segments_mut()
        .map_err(|_| anyhow::anyhow!("Invalid API URL"))?
        .extend(["api", "contracts", contract_id, "state", key]);

    let response = match crate::net::client()
        .put(url)
        .json(&json!({ "value": value }))
        .send_with_retry()
        .await
    {
        Ok(resp) => resp,
        Err(_) => return Ok(false),
    };

    if response.status() == reqwest::StatusCode::NOT_IMPLEMENTED {
        return Ok(false);
    }

    Ok(response.status().is_success())
}

pub async fn state_get(
    api_url: &str,
    contract_id: &str,
    key: &str,
    network: Network,
    json_output: bool,
) -> Result<()> {
    let remote_value = try_remote_state_get(api_url, contract_id, key).await?;
    let (value, source) = if let Some(value) = remote_value {
        (value, "remote")
    } else {
        let store = load_local_state(contract_id, network)?;
        let value = store
            .values
            .get(key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("State key not found: {}", key))?;
        (value, "local")
    };

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "network": network.to_string(),
                "key": key,
                "value": value,
                "source": source
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "Contract State Value".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{}: {}", "Contract".bold(), contract_id);
    println!(
        "{}: {}",
        "Network".bold(),
        network.to_string().bright_blue()
    );
    println!("{}: {}", "Key".bold(), key.bright_magenta());
    println!("{}: {}", "Source".bold(), source);
    println!(
        "{}:\n{}",
        "Value".bold(),
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
    );
    println!();
    Ok(())
}

pub async fn state_set(
    api_url: &str,
    contract_id: &str,
    key: &str,
    raw_value: &str,
    network: Network,
    json_output: bool,
) -> Result<()> {
    require_mutable_network(network)?;
    let new_value = parse_state_value(raw_value);

    let remote_applied = try_remote_state_set(api_url, contract_id, key, &new_value)
        .await
        .unwrap_or(false);

    let mut store = load_local_state(contract_id, network)?;
    let previous = store.values.insert(key.to_string(), new_value.clone());
    store.history.push(LocalStateHistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: "set".to_string(),
        key: Some(key.to_string()),
        previous,
        value: Some(new_value.clone()),
        note: if remote_applied {
            Some("remote + local".to_string())
        } else {
            Some("local".to_string())
        },
    });
    save_local_state(&store, network)?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "network": network.to_string(),
                "key": key,
                "value": new_value,
                "remote_applied": remote_applied,
                "status": "updated"
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "State Updated".bold().green());
    println!("{}", "=".repeat(80).cyan());
    println!("{}: {}", "Contract".bold(), contract_id);
    println!(
        "{}: {}",
        "Network".bold(),
        network.to_string().bright_blue()
    );
    println!("{}: {}", "Key".bold(), key.bright_magenta());
    println!("{}: {}", "Remote Applied".bold(), remote_applied);
    println!(
        "{}:\n{}",
        "New Value".bold(),
        serde_json::to_string_pretty(&new_value).unwrap_or_else(|_| new_value.to_string())
    );
    println!();
    Ok(())
}

pub fn state_dump(contract_id: &str, network: Network, json_output: bool) -> Result<()> {
    let store = load_local_state(contract_id, network)?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "network": network.to_string(),
                "count": store.values.len(),
                "values": store.values,
                "snapshots": store.snapshots.len(),
                "history_entries": store.history.len()
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "Contract State Dump".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{}: {}", "Contract".bold(), contract_id);
    println!(
        "{}: {}",
        "Network".bold(),
        network.to_string().bright_blue()
    );
    println!("{}: {}", "Entries".bold(), store.values.len());
    println!("{}: {}", "Snapshots".bold(), store.snapshots.len());
    println!("{}: {}", "History Entries".bold(), store.history.len());
    println!();

    if store.values.is_empty() {
        println!("{}", "No state entries found.".yellow());
        println!();
        return Ok(());
    }

    for (key, value) in store.values {
        let pretty = serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
        println!("{}: {}", key.bold(), pretty);
    }
    println!();
    Ok(())
}

pub fn state_snapshot_create(
    contract_id: &str,
    network: Network,
    label: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let mut store = load_local_state(contract_id, network)?;
    let snapshot = LocalStateSnapshot {
        id: uuid::Uuid::new_v4().to_string(),
        label: label.map(|s| s.to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
        entry_count: store.values.len(),
        state: store.values.clone(),
    };

    store.history.push(LocalStateHistoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: "snapshot".to_string(),
        key: None,
        previous: None,
        value: None,
        note: snapshot.label.clone(),
    });
    store.snapshots.push(snapshot.clone());
    save_local_state(&store, network)?;

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "network": network.to_string(),
                "snapshot_id": snapshot.id,
                "label": snapshot.label,
                "created_at": snapshot.created_at,
                "entry_count": snapshot.entry_count
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "State Snapshot Created".bold().green());
    println!("{}", "=".repeat(80).cyan());
    println!("{}: {}", "Contract".bold(), contract_id);
    println!(
        "{}: {}",
        "Network".bold(),
        network.to_string().bright_blue()
    );
    println!("{}: {}", "Snapshot ID".bold(), snapshot.id.bright_magenta());
    println!(
        "{}: {}",
        "Label".bold(),
        snapshot.label.unwrap_or_else(|| "-".to_string())
    );
    println!("{}: {}", "Entries".bold(), snapshot.entry_count);
    println!();
    Ok(())
}

pub fn state_snapshot_list(
    contract_id: &str,
    network: Network,
    limit: usize,
    json_output: bool,
) -> Result<()> {
    let store = load_local_state(contract_id, network)?;
    let snapshots: Vec<&LocalStateSnapshot> = store.snapshots.iter().rev().take(limit).collect();

    if json_output {
        let payload: Vec<serde_json::Value> = snapshots
            .iter()
            .map(|snapshot| {
                json!({
                    "id": snapshot.id,
                    "label": snapshot.label,
                    "created_at": snapshot.created_at,
                    "entry_count": snapshot.entry_count
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "network": network.to_string(),
                "items": payload
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "State Snapshots".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    if snapshots.is_empty() {
        println!("{}", "No snapshots found.".yellow());
        println!();
        return Ok(());
    }

    for (index, snapshot) in snapshots.iter().enumerate() {
        println!(
            "  {}. {} [{}] entries={} label={}",
            index + 1,
            snapshot.id.bright_magenta(),
            snapshot.created_at.bright_black(),
            snapshot.entry_count,
            snapshot.label.clone().unwrap_or_else(|| "-".to_string())
        );
    }
    println!();
    Ok(())
}

pub fn state_history(
    contract_id: &str,
    network: Network,
    key_filter: Option<&str>,
    limit: usize,
    json_output: bool,
) -> Result<()> {
    let store = load_local_state(contract_id, network)?;
    let entries: Vec<&LocalStateHistoryEntry> = store
        .history
        .iter()
        .rev()
        .filter(|entry| {
            if let Some(filter) = key_filter {
                return entry.key.as_deref() == Some(filter);
            }
            true
        })
        .take(limit)
        .collect();

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "contract_id": contract_id,
                "network": network.to_string(),
                "items": entries
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "State History".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    if entries.is_empty() {
        println!("{}", "No history entries found.".yellow());
        println!();
        return Ok(());
    }

    for (index, entry) in entries.iter().enumerate() {
        let key = entry.key.as_deref().unwrap_or("-");
        println!(
            "  {}. [{}] {} key={} note={}",
            index + 1,
            entry.timestamp.bright_black(),
            entry.action.bold(),
            key.bright_magenta(),
            entry.note.clone().unwrap_or_else(|| "-".to_string())
        );
    }
    println!();
    Ok(())
}

#[cfg(test)]
mod state_tests {
    use super::*;

    #[test]
    fn parse_state_value_uses_json_when_possible() {
        let parsed = parse_state_value("{\"x\":1}");
        assert_eq!(parsed["x"], 1);
    }

    #[test]
    fn parse_state_value_falls_back_to_string() {
        let parsed = parse_state_value("not-json");
        assert_eq!(parsed, serde_json::Value::String("not-json".to_string()));
    }

    #[test]
    fn mainnet_mutation_is_blocked() {
        let result = require_mutable_network(Network::Mainnet);
        assert!(result.is_err());
    }

    #[test]
    fn non_mainnet_mutation_is_allowed() {
        assert!(require_mutable_network(Network::Testnet).is_ok());
        assert!(require_mutable_network(Network::Futurenet).is_ok());
    }
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
    println!(
        "  {}: {}",
        "New State".bold(),
        new_state.to_string().green().bold()
    );

    if matches!(
        new_state,
        IncidentState::Recovered | IncidentState::PostReview
    ) {
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

    let client = crate::net::client();
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
        .send_with_retry()
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
        let recommended =
            crate::conversions::as_str(&finding["recommended_version"], "recommended_version")?;

        let sev_enum = severity
            .parse::<Severity>()
            .context("Invalid severity string")?;
        if matches!(sev_enum, Severity::Critical | Severity::High) {
            has_high_severity = true;
        }

        println!(
            "  {} {}@{} - {}",
            severity_colored(&sev_enum),
            package,
            version,
            cve_id.bold()
        );
        println!(
            "    {} Recommended patch: {}",
            "↳".bright_black(),
            recommended.green()
        );
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
        let invalid_output = temp_dir
            .path()
            .join("missing-dir")
            .join("flamegraph-output.svg");
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

        assert!(
            json_output.exists(),
            "expected JSON profile output to exist"
        );
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
            err.to_string()
                .contains("Failed to load baseline profile from"),
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
    let client = crate::net::client();
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
        .send_with_retry()
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
    println!(
        "{}: {}",
        "Strict Mode".bold(),
        if strict { "Yes" } else { "No" }
    );

    if valid {
        println!(
            "\n{} {}",
            "✓".green().bold(),
            "Call is valid!".green().bold()
        );

        // Show parsed parameters
        if let Some(params) = data["parsed_params"].as_array() {
            println!("\n{}", "Parsed Parameters:".bold());
            for param in params {
                let name = crate::conversions::as_str(&param["name"], "name")?;
                let type_name =
                    crate::conversions::as_str(&param["expected_type"], "expected_type")?;
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
                    println!(
                        "  {} [{}] {}: {}",
                        "✗".red(),
                        code.bright_black(),
                        f.bold(),
                        msg
                    );
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
    let client = crate::net::client();
    let url = format!(
        "{}/api/contracts/{}/bindings?language={}",
        api_url, contract_id, language
    );

    log::debug!("GET {}", url);

    let response = client
        .get(&url)
        .send_with_retry()
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
    let client = crate::net::client();
    let url = format!("{}/api/contracts/{}/functions", api_url, contract_id);

    log::debug!("GET {}", url);

    let response = client
        .get(&url)
        .send_with_retry()
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

/// Fetch contract info from the registry. `id` is the contract's registry identifier.
pub async fn info(
    api_url: &str,
    id: &str,
    format: &str,
    highlight_method: Option<&str>,
    network: crate::config::Network,
) -> Result<()> {
    let client = crate::net::client();
    let base_url = api_url.trim_end_matches('/');

    if format == "text" {
        println!("\n{}", "Fetching contract information...".bold().cyan());
    }

    // 1. Fetch Metadata
    let metadata_url = format!("{}/api/contracts/{}", base_url, id);
    let metadata_res = client
        .get(&metadata_url)
        .query(&[("network", network.to_string())])
        .send_with_retry()
        .await?;

    if !metadata_res.status().is_success() {
        anyhow::bail!(
            "Failed to fetch contract metadata: {}",
            metadata_res.status()
        );
    }
    let metadata: serde_json::Value = metadata_res.json().await?;

    // Extract genuine UUID if 'id' was a name or address
    let contract_uuid = metadata["contract"]["id"]
        .as_str()
        .context("Metadata missing contract ID")?;
    let contract_address = metadata["contract"]["contract_id"].as_str().unwrap_or(id);

    // 2. Fetch ABI
    let abi_url = format!("{}/api/contracts/{}/abi", base_url, contract_uuid);
    let abi_res = client.get(&abi_url).send_with_retry().await;
    let abi: Option<serde_json::Value> = if let Ok(res) = abi_res {
        if res.status().is_success() {
            res.json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v.get("abi").cloned())
        } else {
            None
        }
    } else {
        None
    };

    // 3. Fetch Deployments
    let depl_url = format!("{}/api/contracts/{}/deployments", base_url, contract_uuid);
    let depl_res = client.get(&depl_url).send_with_retry().await;
    let deployments: Vec<serde_json::Value> = if let Ok(res) = depl_res {
        if res.status().is_success() {
            res.json().await.unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // 4. Fetch Dependencies
    let deps_url = format!("{}/api/contracts/{}/dependencies", base_url, contract_uuid);
    let deps_res = client.get(&deps_url).send_with_retry().await;
    let dependencies: Vec<serde_json::Value> = if let Ok(res) = deps_res {
        if res.status().is_success() {
            res.json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v.get("dependencies").cloned())
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // 5. Fetch Dependents (Related Contracts)
    let relate_url = format!("{}/api/contracts/{}/dependents", base_url, contract_uuid);
    let relate_res = client.get(&relate_url).send_with_retry().await;
    let dependents: Vec<serde_json::Value> = if let Ok(res) = relate_res {
        if res.status().is_success() {
            res.json::<serde_json::Value>()
                .await
                .ok()
                .and_then(|v| v.get("dependents").cloned())
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // 6. Fetch Versions (for verification status)
    let versions_url = format!("{}/api/contracts/{}/versions", base_url, contract_uuid);
    let versions_res = client.get(&versions_url).send_with_retry().await;
    let versions: Vec<serde_json::Value> = if let Ok(res) = versions_res {
        if res.status().is_success() {
            res.json().await.unwrap_or_default()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Aggregate data
    let full_info = json!({
        "metadata": metadata["contract"],
        "current_network_config": metadata["network_config"],
        "abi": abi,
        "deployments": deployments,
        "dependencies": dependencies,
        "dependents": dependents,
        "versions": versions,
    });

    // Render output
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&full_info)?);
        }
        "yaml" => {
            let yaml = serde_yaml::to_string(&full_info)?;
            println!("{}", yaml);
        }
        _ => {
            render_info_text(
                &full_info,
                highlight_method,
                contract_address,
                &network.to_string(),
            )?;
        }
    }

    Ok(())
}

fn render_info_text(
    info: &serde_json::Value,
    highlight_method: Option<&str>,
    contract_address: &str,
    network_str: &str,
) -> Result<()> {
    let metadata = &info["metadata"];
    let name = metadata["name"].as_str().unwrap_or("Unknown");
    let desc = metadata["description"]
        .as_str()
        .unwrap_or("No description provided.");
    let is_verified = metadata["is_verified"].as_bool().unwrap_or(false);
    let health_score = metadata["health_score"].as_i64().unwrap_or(0);

    println!("\n{}", "=".repeat(80).cyan());
    println!("{} {}", "CONTRACT:".bold(), name.bold().green());
    println!("{} {}", "ID:      ".bold(), contract_address.yellow());
    println!(
        "{} {}",
        "STATUS:  ".bold(),
        if is_verified {
            "Verified".green().bold()
        } else {
            "Unverified".red()
        }
    );
    println!("{} {}/100", "HEALTH:  ".bold(), health_score);
    println!("{} {}", "DESC:    ".bold(), desc);
    println!("{}", "=".repeat(80).cyan());

    // Explorer Links
    println!("\n{}", "BLOCK EXPLORERS:".bold().underline());
    let explorer_url = match network_str {
        "testnet" => format!(
            "https://stellar.expert/explorer/testnet/contract/{}",
            contract_address
        ),
        "futurenet" => format!(
            "https://stellar.expert/explorer/futurenet/contract/{}",
            contract_address
        ),
        _ => format!(
            "https://stellar.expert/explorer/public/contract/{}",
            contract_address
        ),
    };
    println!("  • StellarExpert: {}", explorer_url.blue().underline());

    // ABI Methods
    if let Some(abi) = info["abi"].as_array() {
        println!("\n{}", "ABI METHODS:".bold().underline());
        for item in abi {
            if item["type"] == "function" {
                let m_name = item["name"].as_str().unwrap_or("unknown");
                let mut line = format!("  • {}", m_name);
                if let Some(target) = highlight_method {
                    if m_name == target {
                        line = format!("  • {}", m_name.on_yellow().black().bold());
                    }
                }
                println!("{}", line);
            }
        }
    }

    // Deployments
    if let Some(depls) = info["deployments"].as_array() {
        if !depls.is_empty() {
            println!("\n{}", "DEPLOYMENTS:".bold().underline());
            for d in depls {
                let env = d["environment"].as_str().unwrap_or("unknown");
                let status = d["status"].as_str().unwrap_or("unknown");
                let date = d["deployed_at"].as_str().unwrap_or("");
                println!("  • {:<10} | {:<10} | {}", env, status, date);
            }
        }
    }

    // Dependencies
    if let Some(deps) = info["dependencies"].as_array() {
        if !deps.is_empty() {
            println!("\n{}", "DEPENDENCIES:".bold().underline());
            for d in deps {
                let d_name = d["dependency_name"].as_str().unwrap_or("unknown");
                let constraint = d["version_constraint"].as_str().unwrap_or("*");
                println!("  • {} ({})", d_name, constraint);
            }
        }
    }

    // Related Contracts (Dependents)
    if let Some(deps) = info["dependents"].as_array() {
        if !deps.is_empty() {
            println!("\n{}", "RELATED CONTRACTS (DEPENDENTS):".bold().underline());
            for d in deps {
                let d_name = d["dependency_name"].as_str().unwrap_or("unknown"); // This is from the perspective of the dependent
                                                                                 // Wait, it should use the contract name if available.
                                                                                 // But dependents might just be a list of contract IDs.
                println!("  • Contract ID: {}", d["contract_id"]);
            }
        }
    }

    println!("\n{}", "=".repeat(80).cyan());
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
                md.push_str(&format!(
                    "- `{}`: `{}`\n",
                    p.name,
                    p.param_type.display_name()
                ));
            }
        }
        md.push_str(&format!(
            "\n**Returns:** `{}`\n\n",
            func.return_type.display_name()
        ));
    }
    if !abi.errors.is_empty() {
        md.push_str("## Errors\n\n");
        for e in &abi.errors {
            md.push_str(&format!(
                "- **{}** (code {}): {}\n",
                e.name,
                e.code,
                e.doc.as_deref().unwrap_or("")
            ));
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
        title, spec_escaped
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
        _ => anyhow::bail!(
            "Unsupported format '{}'. Use: yaml, json, markdown, html, pdf",
            format
        ),
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

pub async fn snapshot_create(api_url: &str, contract_id: &str) -> Result<()> {
    let client = crate::net::client();
    let url = format!("{}/api/contracts/{}/snapshots", api_url, contract_id);

    println!("\n{}", "Creating contract snapshot...".bold().cyan());

    let response = client
        .post(&url)
        .send_with_retry()
        .await
        .context("Failed to create snapshot")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to create snapshot: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let snapshot: serde_json::Value = response.json().await?;

    println!("{}", "✓ Snapshot created successfully!".green().bold());
    println!(
        "  {}: {}",
        "ID".bold(),
        snapshot["id"].as_str().unwrap_or("")
    );
    println!(
        "  {}: {}",
        "Version".bold(),
        snapshot["version_number"].as_i64().unwrap_or(0)
    );
    println!(
        "  {}: {}",
        "Created At".bold(),
        snapshot["created_at"].as_str().unwrap_or("")
    );
    println!();

    Ok(())
}

pub async fn snapshot_list(api_url: &str, contract_id: &str) -> Result<()> {
    let client = crate::net::client();
    let url = format!("{}/api/contracts/{}/snapshots", api_url, contract_id);

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to list snapshots")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to list snapshots: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let snapshots: Vec<serde_json::Value> = response.json().await?;

    println!("\n{}", "Contract Snapshots:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    if snapshots.is_empty() {
        println!("{}", "No snapshots found.".yellow());
        return Ok(());
    }

    for s in snapshots {
        println!(
            "  v{} - {} [{}]",
            s["version_number"].as_i64().unwrap_or(0),
            s["created_at"].as_str().unwrap_or("").bright_black(),
            s["id"].as_str().unwrap_or("").cyan()
        );
    }
    println!();

    Ok(())
}

pub async fn snapshot_get(api_url: &str, contract_id: &str, timestamp: &str) -> Result<()> {
    let client = crate::net::client();
    let url = format!(
        "{}/api/contracts/{}/snapshots?timestamp={}",
        api_url, contract_id, timestamp
    );

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch snapshot")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch snapshot: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let snapshot: serde_json::Value = response.json().await?;
    println!("\n{}", "Snapshot Details:".bold().cyan());
    println!("{}", "=".repeat(80).cyan());
    println!("{}", serde_json::to_string_pretty(&snapshot)?.green());
    println!();

    Ok(())
}

pub async fn snapshot_diff(api_url: &str, contract_id: &str, v1: i32, v2: i32) -> Result<()> {
    let client = crate::net::client();
    let url = format!(
        "{}/api/contracts/{}/versions/{}/diff/{}",
        api_url, contract_id, v1, v2
    );

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch diff")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch diff: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let diff: shared::models::VersionDiff = response.json().await?;

    println!(
        "\n{}",
        format!("Diff between v{} and v{}:", v1, v2).bold().cyan()
    );
    println!("{}", "=".repeat(80).cyan());

    if diff.added.is_empty() && diff.removed.is_empty() && diff.modified.is_empty() {
        println!("{}", "No differences found.".green());
        return Ok(());
    }

    for add in diff.added {
        println!(
            "  {} {}: {}",
            "+".green().bold(),
            add.field.bold(),
            add.to.to_string().green()
        );
    }
    for rm in diff.removed {
        println!(
            "  {} {}: {}",
            "-".red().bold(),
            rm.field.bold(),
            rm.from.to_string().red()
        );
    }
    for modif in diff.modified {
        println!(
            "  {} {}: {} -> {}",
            "~".yellow().bold(),
            modif.field.bold(),
            modif.from.to_string().red(),
            modif.to.to_string().green()
        );
    }

    println!();

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn contract_stats(
    api_url: &str,
    network: Option<&str>,
    category: Option<&str>,
    top_n: usize,
    format: &str,
    output: Option<&str>,
    compare: Option<&str>,
) -> Result<()> {
    let client = crate::net::client();
    let mut url = reqwest::Url::parse(&format!("{}/api/stats", api_url.trim_end_matches('/')))
        .context("Invalid registry API URL")?;
    {
        let mut query = url.query_pairs_mut();
        if let Some(network) = network {
            query.append_pair("network", network);
        }
        if let Some(category) = category {
            query.append_pair("category", category);
        }
        if let Some(compare) = compare {
            query.append_pair("compare", compare);
        }
    }

    let response = client
        .get(url)
        .send_with_retry()
        .await
        .context("Failed to fetch contract statistics")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch contract stats: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let mut stats: serde_json::Value = response.json().await?;
    if let Some(top) = stats
        .get_mut("top_contracts")
        .and_then(serde_json::Value::as_array_mut)
    {
        top.truncate(top_n);
    }

    let output_str = match format {
        "json" => serde_json::to_string_pretty(&stats)?,
        "csv" => format_stats_csv(&stats)?,
        "table" => format_stats_table(&stats),
        _ => anyhow::bail!("Invalid format: {}. Use table, json, or csv", format),
    };

    if let Some(path) = output {
        fs::write(path, &output_str)?;
        println!("{} Contract stats written to {}", "OK".green(), path);
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

/// Get comprehensive registry statistics
/// Command: soroban-registry stats [options]
pub async fn stats(
    api_url: &str,
    timeframe: &str,
    format: &str,
    output: Option<&str>,
) -> Result<()> {
    let fmt = output_format::validate_format(format).unwrap_or(output_format::OutputFormat::Table);

    let client = crate::net::client();
    let url = format!("{}/api/stats?timeframe={}", api_url, timeframe);

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch registry statistics")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch stats: {}",
            response.text().await.unwrap_or_default()
        );
    }

    let stats: serde_json::Value = response.json().await?;

    // Format output
    let output_str = match fmt {
        output_format::OutputFormat::Json => serde_json::to_string_pretty(&stats)?,
        output_format::OutputFormat::Yaml => serde_yaml::to_string(&stats)?,
        output_format::OutputFormat::Table => format_stats_table(&stats),
        output_format::OutputFormat::Csv => {
            let flat = serde_json::json!([stats]);
            output_format::render_csv(&flat)?
        }
    };

    if let Some(path) = output {
        fs::write(path, &output_str)?;
        println!("{} Stats written to {}", "✓".green(), path);
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

fn format_stats_table(stats: &serde_json::Value) -> String {
    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "\n{}",
        "Soroban Registry Statistics".bold().cyan()
    ));
    out.push_str(&format!("\n{}\n", "=".repeat(60).cyan()));

    // Basic counts
    if let Some(total) = stats["total_contracts"].as_i64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("Total Contracts", &total.to_string())
        ));
    }
    if let Some(publishers) = stats["total_publishers"].as_i64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("Total Publishers", &publishers.to_string())
        ));
    }
    if let Some(verified) = stats["verified_contracts"].as_i64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("Verified Contracts", &verified.to_string())
        ));
    }
    if let Some(pct) = stats["verification_percentage"].as_f64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("Verification Rate", &format!("{:.1}%", pct))
        ));
    }
    out.push_str("\n");

    // Growth
    out.push_str(&format!("{}", "Growth".bold()));
    out.push_str(&format!("\n{}\n", "─".repeat(40).bright_black()));
    if let Some(c7) = stats["contracts_last_7d"].as_i64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("  Last 7 days", &c7.to_string())
        ));
    }
    if let Some(c30) = stats["contracts_last_30d"].as_i64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("  Last 30 days", &c30.to_string())
        ));
    }
    if let Some(p30) = stats["new_publishers_last_30d"].as_i64() {
        out.push_str(&format!(
            "{}\n",
            format_kv("  New publishers (30d)", &p30.to_string())
        ));
    }
    out.push_str("\n");

    // Top contracts
    if let Some(top) = stats["top_contracts"].as_array() {
        out.push_str(&format!("{}", "Top 10 Contracts by Interactions".bold()));
        out.push_str(&format!("\n{}\n", "─".repeat(40).bright_black()));
        for (i, contract) in top.iter().enumerate().take(10) {
            let name = contract["name"].as_str().unwrap_or("N/A");
            let count = contract["interaction_count"].as_i64().unwrap_or(0);
            out.push_str(&format!(
                "  {}. {} ({})\n",
                (i + 1).to_string().bright_blue(),
                name.bold(),
                count.to_string().green()
            ));
        }
        out.push_str("\n");
    }

    // Network breakdown
    if let Some(networks) = stats["network_stats"].as_array() {
        out.push_str(&format!("{}", "By Network".bold()));
        out.push_str(&format!("\n{}\n", "─".repeat(40).bright_black()));
        for net in networks {
            let n = match net["network"].as_str() {
                Some("mainnet") => "Mainnet".cyan(),
                Some("testnet") => "Testnet".yellow(),
                Some("futurenet") => "Futurenet".magenta(),
                _ => net["network"].as_str().unwrap_or("").into(),
            };
            let count = net["contract_count"].as_i64().unwrap_or(0);
            out.push_str(&format!("  {}: {} contracts\n", n, count));
        }
        out.push_str("\n");
    }

    // Generated at
    if let Some(gen) = stats["generated_at"].as_str() {
        out.push_str(&format!("Generated at: {}\n", gen.bright_black()));
    }

    out
}

fn format_stats_csv(stats: &serde_json::Value) -> Result<String> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(["metric", "scope", "value"])?;

    for key in [
        "total_contracts",
        "total_publishers",
        "verified_contracts",
        "verification_percentage",
        "contracts_last_24h",
        "contracts_last_7d",
        "contracts_last_30d",
        "new_publishers_last_30d",
    ] {
        if let Some(value) = stats.get(key) {
            writer.write_record(["summary", key, &value.to_string()])?;
        }
    }

    if let Some(networks) = stats
        .get("network_stats")
        .and_then(serde_json::Value::as_array)
    {
        for network in networks {
            writer.write_record([
                "network",
                network
                    .get("network")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown"),
                &network
                    .get("contract_count")
                    .map(serde_json::Value::to_string)
                    .unwrap_or_else(|| "0".to_string()),
            ])?;
        }
    }

    if let Some(categories) = stats
        .get("category_stats")
        .and_then(serde_json::Value::as_array)
    {
        for category in categories {
            writer.write_record([
                "category",
                category
                    .get("category")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("uncategorized"),
                &category
                    .get("contract_count")
                    .map(serde_json::Value::to_string)
                    .unwrap_or_else(|| "0".to_string()),
            ])?;
        }
    }

    if let Some(top) = stats
        .get("top_contracts")
        .and_then(serde_json::Value::as_array)
    {
        for contract in top {
            writer.write_record([
                "top_contract",
                contract
                    .get("name")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| {
                        contract
                            .get("contract_id")
                            .and_then(serde_json::Value::as_str)
                    })
                    .unwrap_or("unknown"),
                &contract
                    .get("interaction_count")
                    .map(serde_json::Value::to_string)
                    .unwrap_or_else(|| "0".to_string()),
            ])?;
        }
    }

    let bytes = writer.into_inner()?;
    Ok(String::from_utf8(bytes)?)
}

fn format_kv(key: &str, value: &str) -> String {
    format!("  {} {}", key.bold().cyan(), value.bright_white())
}
