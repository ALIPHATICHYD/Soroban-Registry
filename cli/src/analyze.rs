//! analyze.rs — `soroban-registry analyze` (#530)
//!
//! Advanced contract analysis: complexity, security patterns, dependency graph,
//! performance estimates, and actionable optimisation suggestions.

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

// ── Report types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub contract_id: String,
    pub network: String,
    pub complexity: ComplexityMetrics,
    pub security: SecurityAnalysis,
    pub dependencies: DependencyAnalysis,
    pub performance: PerformanceEstimate,
    pub suggestions: Vec<Suggestion>,
    pub generated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Number of public entry points in the ABI
    pub entry_points: u64,
    /// Estimated cyclomatic complexity score (1–100)
    pub cyclomatic_score: u64,
    /// Number of distinct data types in the ABI
    pub data_type_count: u64,
    /// Qualitative rating: Low / Medium / High / Very High
    pub rating: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityAnalysis {
    pub patterns_detected: Vec<SecurityPattern>,
    pub anti_patterns_detected: Vec<SecurityPattern>,
    pub risk_level: String, // Low | Medium | High | Critical
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityPattern {
    pub name: String,
    pub description: String,
    pub severity: String, // info | low | medium | high | critical
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyAnalysis {
    pub direct_dependencies: Vec<DependencyInfo>,
    pub total_count: usize,
    pub has_unverified: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub name: String,
    pub version_constraint: String,
    pub is_verified: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceEstimate {
    /// Estimated ledger operations per typical invocation
    pub estimated_ops: u64,
    /// Estimated instruction budget consumption (% of Soroban limit)
    pub instruction_budget_pct: f64,
    /// Estimated read/write ledger entries
    pub ledger_entries: u64,
    /// Qualitative rating: Optimal / Good / Fair / Needs Improvement
    pub rating: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Suggestion {
    pub category: String, // security | performance | maintainability | best_practice
    pub priority: String, // low | medium | high
    pub title: String,
    pub detail: String,
}

// ── Main entry point ─────────────────────────────────────────────────────────

/// `soroban-registry analyze <contract-id> --network <n> [--report-format <fmt>]`
pub async fn run(
    api_url: &str,
    contract_id: &str,
    network: &str,
    report_format: &str,
    output: Option<&str>,
) -> Result<()> {
    log::debug!(
        "analyze | contract_id={} network={} format={} output={:?}",
        contract_id,
        network,
        report_format,
        output
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")?;

    if report_format != "json" {
        print_header();
        println!("  {}  {}", "Contract:".bold(), contract_id.bright_black());
        println!("  {}  {}", "Network:".bold(), network.bright_blue());
        println!("  {}   {}", "Format:".bold(), report_format);
        println!();
    }

    // ── 1. Fetch contract from registry ──────────────────────────────────────
    let contract = fetch_contract(&client, api_url, contract_id, network).await?;

    // ── 2. Fetch ABI / verification detail ───────────────────────────────────
    let detail = fetch_detail(&client, api_url, &contract).await;

    // ── 3. Fetch dependency list ──────────────────────────────────────────────
    let deps_raw = fetch_dependencies(&client, api_url, contract_id).await;

    // ── 4. Run analysis ───────────────────────────────────────────────────────
    if report_format != "json" {
        print!("  {} Analysing...", "⟳".cyan());
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }

    let report = build_report(contract_id, network, &contract, &detail, deps_raw);

    if report_format != "json" {
        println!("\r  {} Analysis complete.    ", "✔".green());
        println!();
    }

    // ── 5. Output ─────────────────────────────────────────────────────────────
    let rendered = render(&report, report_format)?;

    if let Some(path) = output {
        std::fs::write(path, &rendered)
            .with_context(|| format!("Failed to write report to {}", path))?;
        if report_format != "json" {
            println!("  {} Report written to {}", "✔".green(), path.bold());
        } else {
            println!("{}", rendered);
        }
    } else {
        println!("{}", rendered);
    }

    Ok(())
}

// ── API helpers ───────────────────────────────────────────────────────────────

async fn fetch_contract(
    client: &reqwest::Client,
    api_url: &str,
    contract_id: &str,
    network: &str,
) -> Result<Value> {
    let url = format!(
        "{}/api/contracts?contract_id={}&network={}",
        api_url, contract_id, network
    );
    log::debug!("GET {}", url);

    let res = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to connect to registry API")?;

    let status = res.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!(
            "Contract '{}' not found in the {} registry.",
            contract_id,
            network
        );
    }
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        anyhow::bail!("Registry API error ({}): {}", status, body);
    }

    let raw: Value = res
        .json()
        .await
        .context("Failed to parse registry response")?;

    // Handle paginated list or direct object
    if let Some(items) = raw["items"].as_array() {
        return items
            .iter()
            .find(|c| {
                c["contract_id"].as_str() == Some(contract_id)
                    || c["id"].as_str() == Some(contract_id)
            })
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!("Contract '{}' not found in registry response", contract_id)
            });
    }

    if raw.is_object() && (raw["contract_id"].is_string() || raw["id"].is_string()) {
        return Ok(raw);
    }

    anyhow::bail!("Unexpected registry response format")
}

async fn fetch_detail(client: &reqwest::Client, api_url: &str, contract: &Value) -> Option<Value> {
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

async fn fetch_dependencies(
    client: &reqwest::Client,
    api_url: &str,
    contract_id: &str,
) -> Vec<Value> {
    let url = format!("{}/api/contracts/{}/dependencies", api_url, contract_id);
    log::debug!("GET {}", url);
    let res = match client.get(&url).send_with_retry().await {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    if !res.status().is_success() {
        return vec![];
    }
    res.json::<Value>()
        .await
        .ok()
        .and_then(|v| v["items"].as_array().or_else(|| v.as_array()).cloned())
        .unwrap_or_default()
}

// ── Analysis logic ────────────────────────────────────────────────────────────

fn build_report(
    contract_id: &str,
    network: &str,
    contract: &Value,
    detail: &Option<Value>,
    deps_raw: Vec<Value>,
) -> AnalysisReport {
    let complexity = analyse_complexity(contract, detail);
    let security = analyse_security(contract, detail);
    let dependencies = analyse_dependencies(deps_raw);
    let performance = estimate_performance(contract, detail, &complexity);
    let suggestions = generate_suggestions(
        contract,
        &complexity,
        &security,
        &dependencies,
        &performance,
    );

    AnalysisReport {
        contract_id: contract_id.to_string(),
        network: network.to_string(),
        complexity,
        security,
        dependencies,
        performance,
        suggestions,
        generated_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn analyse_complexity(contract: &Value, detail: &Option<Value>) -> ComplexityMetrics {
    // Use ABI method count from detail, or fall back to heuristics from registry data
    let entry_points = detail
        .as_ref()
        .and_then(|d| d["abi"]["functions"].as_array())
        .map(|f| f.len() as u64)
        .or_else(|| contract["entry_points"].as_u64())
        .unwrap_or_else(|| {
            // Estimate from contract size / metadata
            let tags = contract["tags"].as_array().map_or(0, |t| t.len());
            (3 + tags as u64).min(20)
        });

    let data_type_count = detail
        .as_ref()
        .and_then(|d| d["abi"]["types"].as_array())
        .map(|t| t.len() as u64)
        .unwrap_or_else(|| (entry_points / 2).max(1));

    // Cyclomatic score: heuristic based on entry points and type complexity
    let cyclomatic_score = ((entry_points * 3) + data_type_count).min(100);

    let rating = match cyclomatic_score {
        0..=20 => "Low",
        21..=45 => "Medium",
        46..=70 => "High",
        _ => "Very High",
    }
    .to_string();

    ComplexityMetrics {
        entry_points,
        cyclomatic_score,
        data_type_count,
        rating,
    }
}

fn analyse_security(contract: &Value, detail: &Option<Value>) -> SecurityAnalysis {
    let mut patterns: Vec<SecurityPattern> = Vec::new();
    let mut anti_patterns: Vec<SecurityPattern> = Vec::new();

    let is_verified = contract["is_verified"].as_bool().unwrap_or(false);
    let health_score = contract["health_score"].as_i64().unwrap_or(0);

    // Positive security patterns
    if is_verified {
        patterns.push(SecurityPattern {
            name: "Source Verified".to_string(),
            description: "Contract source code matches the deployed bytecode hash.".to_string(),
            severity: "info".to_string(),
        });
    }

    if health_score >= 80 {
        patterns.push(SecurityPattern {
            name: "High Health Score".to_string(),
            description: format!("Registry health score is {} / 100.", health_score),
            severity: "info".to_string(),
        });
    }

    // Check audit info from detail
    if let Some(d) = detail {
        if d["audit"]["passed"].as_bool() == Some(true) {
            patterns.push(SecurityPattern {
                name: "Audit Passed".to_string(),
                description: format!(
                    "Audited by {}.",
                    d["audit"]["auditor"].as_str().unwrap_or("unknown auditor")
                ),
                severity: "info".to_string(),
            });
        }

        // Check security scan findings
        if let Some(findings) = d["security_scan"]["findings"].as_array() {
            for f in findings {
                let sev = f["severity"].as_str().unwrap_or("info").to_string();
                let title = f["title"].as_str().unwrap_or("Unknown finding").to_string();
                let desc = f["description"].as_str().unwrap_or("").to_string();
                let is_anti = matches!(sev.as_str(), "high" | "critical" | "medium");
                let pat = SecurityPattern {
                    name: title,
                    description: desc,
                    severity: sev,
                };
                if is_anti {
                    anti_patterns.push(pat);
                } else {
                    patterns.push(pat);
                }
            }
        }
    }

    // Anti-pattern: unverified contract
    if !is_verified {
        anti_patterns.push(SecurityPattern {
            name: "Unverified Source".to_string(),
            description: "Source code has not been verified against deployed bytecode.".to_string(),
            severity: "medium".to_string(),
        });
    }

    // Anti-pattern: maintenance mode
    if contract["is_maintenance"].as_bool().unwrap_or(false) {
        anti_patterns.push(SecurityPattern {
            name: "Maintenance Mode".to_string(),
            description:
                "Contract is currently in maintenance mode — interactions may be restricted."
                    .to_string(),
            severity: "high".to_string(),
        });
    }

    let risk_level = if anti_patterns.iter().any(|p| p.severity == "critical") {
        "Critical"
    } else if anti_patterns.iter().any(|p| p.severity == "high") {
        "High"
    } else if anti_patterns.iter().any(|p| p.severity == "medium") {
        "Medium"
    } else {
        "Low"
    }
    .to_string();

    SecurityAnalysis {
        patterns_detected: patterns,
        anti_patterns_detected: anti_patterns,
        risk_level,
    }
}

fn analyse_dependencies(deps_raw: Vec<Value>) -> DependencyAnalysis {
    let direct_dependencies: Vec<DependencyInfo> = deps_raw
        .iter()
        .map(|d| DependencyInfo {
            name: d["dependency_name"]
                .as_str()
                .or(d["name"].as_str())
                .unwrap_or("unknown")
                .to_string(),
            version_constraint: d["version_constraint"].as_str().unwrap_or("*").to_string(),
            is_verified: d["is_verified"].as_bool().unwrap_or(false),
        })
        .collect();

    let has_unverified = direct_dependencies.iter().any(|d| !d.is_verified);
    let total_count = direct_dependencies.len();

    DependencyAnalysis {
        direct_dependencies,
        total_count,
        has_unverified,
    }
}

fn estimate_performance(
    contract: &Value,
    detail: &Option<Value>,
    complexity: &ComplexityMetrics,
) -> PerformanceEstimate {
    // Derive estimates from complexity and available wasm metadata
    let wasm_hash_present = contract["wasm_hash"].as_str().is_some();

    // Base ops: 1 per entry point, plus overhead from data types
    let estimated_ops = complexity.entry_points + (complexity.data_type_count / 2).max(1);

    // Budget estimate: cyclomatic score maps roughly to instruction usage
    let instruction_budget_pct = (complexity.cyclomatic_score as f64 * 0.45).min(95.0);

    // Ledger entries: at minimum 1 read, 1 write per operation
    let ledger_entries = (estimated_ops * 2).max(2);

    let rating = if !wasm_hash_present {
        "Unknown"
    } else if instruction_budget_pct < 20.0 {
        "Optimal"
    } else if instruction_budget_pct < 45.0 {
        "Good"
    } else if instruction_budget_pct < 70.0 {
        "Fair"
    } else {
        "Needs Improvement"
    }
    .to_string();

    // Suppress unused variable warning on detail (used implicitly via complexity)
    let _ = detail;

    PerformanceEstimate {
        estimated_ops,
        instruction_budget_pct,
        ledger_entries,
        rating,
    }
}

fn generate_suggestions(
    contract: &Value,
    complexity: &ComplexityMetrics,
    security: &SecurityAnalysis,
    deps: &DependencyAnalysis,
    perf: &PerformanceEstimate,
) -> Vec<Suggestion> {
    let mut suggestions: Vec<Suggestion> = Vec::new();

    // Security suggestions
    if security.risk_level == "High" || security.risk_level == "Critical" {
        suggestions.push(Suggestion {
            category: "security".to_string(),
            priority: "high".to_string(),
            title: "Address security findings".to_string(),
            detail: format!(
                "{} anti-pattern(s) detected. Review and remediate high/critical findings before production use.",
                security.anti_patterns_detected.len()
            ),
        });
    }

    if !contract["is_verified"].as_bool().unwrap_or(false) {
        suggestions.push(Suggestion {
            category: "security".to_string(),
            priority: "high".to_string(),
            title: "Verify contract source".to_string(),
            detail: "Submit source code via `soroban-registry publish` to enable source verification and increase trust.".to_string(),
        });
    }

    if contract["health_score"].as_i64().unwrap_or(0) < 60 {
        suggestions.push(Suggestion {
            category: "best_practice".to_string(),
            priority: "medium".to_string(),
            title: "Improve registry health score".to_string(),
            detail:
                "Add a description, tags, and category to improve discoverability and health score."
                    .to_string(),
        });
    }

    // Complexity suggestions
    if complexity.cyclomatic_score > 60 {
        suggestions.push(Suggestion {
            category: "maintainability".to_string(),
            priority: "medium".to_string(),
            title: "Reduce contract complexity".to_string(),
            detail: format!(
                "Cyclomatic score of {} is elevated. Consider splitting complex entry points into focused sub-contracts.",
                complexity.cyclomatic_score
            ),
        });
    }

    if complexity.entry_points > 15 {
        suggestions.push(Suggestion {
            category: "maintainability".to_string(),
            priority: "low".to_string(),
            title: "Consider interface decomposition".to_string(),
            detail: format!(
                "{} entry points is large for a single contract. Splitting by domain improves auditability.",
                complexity.entry_points
            ),
        });
    }

    // Dependency suggestions
    if deps.has_unverified {
        suggestions.push(Suggestion {
            category: "security".to_string(),
            priority: "medium".to_string(),
            title: "Unverified dependencies detected".to_string(),
            detail: "One or more dependencies have not been source-verified. Verify or pin versions to reduce supply-chain risk.".to_string(),
        });
    }

    if deps.total_count > 10 {
        suggestions.push(Suggestion {
            category: "maintainability".to_string(),
            priority: "low".to_string(),
            title: "High dependency count".to_string(),
            detail: format!(
                "{} dependencies increase the attack surface. Audit each dependency and remove unused ones.",
                deps.total_count
            ),
        });
    }

    // Performance suggestions
    if perf.instruction_budget_pct > 60.0 {
        suggestions.push(Suggestion {
            category: "performance".to_string(),
            priority: "medium".to_string(),
            title: "High instruction budget usage".to_string(),
            detail: format!(
                "Estimated ~{:.0}% of Soroban instruction budget consumed. Profile hot paths and consider caching computed values.",
                perf.instruction_budget_pct
            ),
        });
    }

    if perf.ledger_entries > 20 {
        suggestions.push(Suggestion {
            category: "performance".to_string(),
            priority: "low".to_string(),
            title: "Reduce ledger entry footprint".to_string(),
            detail: "High ledger entry count increases transaction fees. Consider merging related state into composite keys.".to_string(),
        });
    }

    suggestions
}

// ── Rendering ─────────────────────────────────────────────────────────────────

fn render(report: &AnalysisReport, format: &str) -> Result<String> {
    match format.to_lowercase().as_str() {
        "json" => Ok(serde_json::to_string_pretty(report)?),
        "yaml" => serde_yaml::to_string(report).context("Failed to serialise report as YAML"),
        "text" | "txt" => Ok(render_text(report)),
        other => anyhow::bail!(
            "Unsupported report format '{}'. Valid options: text, json, yaml",
            other
        ),
    }
}

fn render_text(report: &AnalysisReport) -> String {
    let mut out = String::new();

    // ── Header ────────────────────────────────────────────────────────────────
    out.push_str(&format!(
        "\n{}\n{}\n\n",
        "Contract Analysis Report".bold().cyan(),
        "═".repeat(60).cyan()
    ));
    out.push_str(&format!(
        "  {}  {}\n",
        "Contract:".bold(),
        report.contract_id.bright_black()
    ));
    out.push_str(&format!(
        "  {}  {}\n",
        "Network:".bold(),
        report.network.bright_blue()
    ));
    out.push_str(&format!(
        "  {}  {}\n\n",
        "Generated:".bold(),
        report.generated_at.dimmed()
    ));

    // ── Complexity ────────────────────────────────────────────────────────────
    out.push_str(&format!("  {}\n", "CODE COMPLEXITY".bold().underline()));
    out.push_str(&format!(
        "  Entry points:      {}\n",
        report.complexity.entry_points
    ));
    out.push_str(&format!(
        "  Cyclomatic score:  {} / 100\n",
        report.complexity.cyclomatic_score
    ));
    out.push_str(&format!(
        "  Data types:        {}\n",
        report.complexity.data_type_count
    ));
    let complexity_label = match report.complexity.rating.as_str() {
        "Low" => report.complexity.rating.green().bold(),
        "Medium" => report.complexity.rating.yellow().bold(),
        _ => report.complexity.rating.red().bold(),
    };
    out.push_str(&format!("  Rating:            {}\n\n", complexity_label));

    // ── Security ──────────────────────────────────────────────────────────────
    out.push_str(&format!("  {}\n", "SECURITY ANALYSIS".bold().underline()));
    let risk_label = match report.security.risk_level.as_str() {
        "Low" => report.security.risk_level.green().bold(),
        "Medium" => report.security.risk_level.yellow().bold(),
        "High" => report.security.risk_level.red().bold(),
        _ => report.security.risk_level.red().bold(),
    };
    out.push_str(&format!("  Risk level:        {}\n", risk_label));

    if !report.security.patterns_detected.is_empty() {
        out.push_str(&format!("  {} Good patterns:\n", "✔".green()));
        for p in &report.security.patterns_detected {
            out.push_str(&format!(
                "    • {} — {}\n",
                p.name.bold(),
                p.description.dimmed()
            ));
        }
    }

    if !report.security.anti_patterns_detected.is_empty() {
        out.push_str(&format!("  {} Issues detected:\n", "✘".red()));
        for p in &report.security.anti_patterns_detected {
            let sev_label = match p.severity.as_str() {
                "critical" => format!("[{}]", p.severity.to_uppercase()).red().bold(),
                "high" => format!("[{}]", p.severity.to_uppercase()).red(),
                "medium" => format!("[{}]", p.severity.to_uppercase()).yellow(),
                _ => format!("[{}]", p.severity.to_uppercase()).normal(),
            };
            out.push_str(&format!(
                "    {} {} — {}\n",
                sev_label,
                p.name.bold(),
                p.description.dimmed()
            ));
        }
    }
    out.push('\n');

    // ── Dependencies ──────────────────────────────────────────────────────────
    out.push_str(&format!("  {}\n", "DEPENDENCY ANALYSIS".bold().underline()));
    out.push_str(&format!(
        "  Total dependencies: {}\n",
        report.dependencies.total_count
    ));
    if report.dependencies.total_count == 0 {
        out.push_str("  No registered dependencies.\n");
    } else {
        for dep in &report.dependencies.direct_dependencies {
            let verified_label = if dep.is_verified {
                "verified".green()
            } else {
                "unverified".yellow()
            };
            out.push_str(&format!(
                "    • {} ({})  [{}]\n",
                dep.name.bold(),
                dep.version_constraint.dimmed(),
                verified_label
            ));
        }
        if report.dependencies.has_unverified {
            out.push_str(&format!(
                "  {} Some dependencies are unverified.\n",
                "⚠".yellow()
            ));
        }
    }
    out.push('\n');

    // ── Performance ───────────────────────────────────────────────────────────
    out.push_str(&format!(
        "  {}\n",
        "PERFORMANCE ESTIMATES".bold().underline()
    ));
    out.push_str(&format!(
        "  Estimated ops:       {}\n",
        report.performance.estimated_ops
    ));
    out.push_str(&format!(
        "  Instruction budget:  ~{:.1}%\n",
        report.performance.instruction_budget_pct
    ));
    out.push_str(&format!(
        "  Ledger entries:      {}\n",
        report.performance.ledger_entries
    ));
    let perf_label = match report.performance.rating.as_str() {
        "Optimal" => report.performance.rating.green().bold(),
        "Good" => report.performance.rating.green(),
        "Fair" => report.performance.rating.yellow().bold(),
        _ => report.performance.rating.red().bold(),
    };
    out.push_str(&format!("  Rating:              {}\n\n", perf_label));

    // ── Suggestions ───────────────────────────────────────────────────────────
    if report.suggestions.is_empty() {
        out.push_str(&format!(
            "  {} No actionable suggestions — contract looks good!\n\n",
            "✔".green().bold()
        ));
    } else {
        out.push_str(&format!(
            "  {} ({} items)\n",
            "OPTIMISATION SUGGESTIONS".bold().underline(),
            report.suggestions.len()
        ));
        for (i, s) in report.suggestions.iter().enumerate() {
            let priority_label = match s.priority.as_str() {
                "high" => format!("[{}]", s.priority.to_uppercase()).red().bold(),
                "medium" => format!("[{}]", s.priority.to_uppercase()).yellow().bold(),
                _ => format!("[{}]", s.priority.to_uppercase()).dimmed(),
            };
            out.push_str(&format!(
                "\n  {}. {} {} — {}\n",
                i + 1,
                priority_label,
                s.title.bold(),
                s.category.dimmed()
            ));
            out.push_str(&format!("     {}\n", s.detail));
        }
        out.push('\n');
    }

    out.push_str(&format!("{}\n", "═".repeat(60).cyan()));
    out
}

fn print_header() {
    println!();
    println!("{}", "Advanced Contract Analysis".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
}
