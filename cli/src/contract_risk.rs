//! contract_risk.rs — `soroban-registry contract risk <address>` (#837)
//!
//! Assesses security and operational risks for a registered Soroban contract.
//! Analyses code age, update frequency, audit status, known vulnerabilities,
//! developer reputation, high-privilege functions, and dependency health.
//! Outputs an actionable risk report with remediation steps.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

// ── Risk level ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    fn from_score(score: u32) -> Self {
        match score {
            0..=24 => Self::Low,
            25..=49 => Self::Medium,
            50..=74 => Self::High,
            _ => Self::Critical,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }
}

impl std::str::FromStr for RiskLevel {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            _ => anyhow::bail!(
                "invalid risk level '{}' — expected: low | medium | high | critical",
                s
            ),
        }
    }
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// ── Report types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskReport {
    pub address: String,
    pub network: String,
    /// Overall risk score 0–100; higher means riskier.
    pub risk_score: u32,
    pub risk_level: String,
    pub findings: Vec<RiskFinding>,
    pub dependency_vulnerability_score: u32,
    pub recommendations: Vec<String>,
    pub assessed_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskFinding {
    pub category: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub remediation: Option<String>,
}

// ── Main entry point ──────────────────────────────────────────────────────────

/// `soroban-registry contract risk <address> [--network <n>] [--threshold <level>] [--json]`
pub async fn run(
    api_url: &str,
    address: &str,
    network: &str,
    threshold: Option<&str>,
    json: bool,
) -> Result<()> {
    log::debug!(
        "contract risk | address={} network={} threshold={:?} json={}",
        address,
        network,
        threshold,
        json
    );

    // Parse threshold early so bad input fails fast.
    let threshold_level: Option<RiskLevel> = threshold
        .map(|t| t.parse())
        .transpose()
        .context("Invalid --threshold value")?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")?;

    // ── 1. Fetch contract ─────────────────────────────────────────────────────
    let contract = fetch_contract(&client, api_url, address, network).await?;

    // ── 2. Fetch verification detail (audit + security scan + ABI) ────────────
    let detail = fetch_detail(&client, api_url, &contract).await;

    // ── 3. Fetch dependencies ─────────────────────────────────────────────────
    let deps = fetch_dependencies(&client, api_url, address).await;

    // ── 4. Assess risks ───────────────────────────────────────────────────────
    let report = build_report(address, network, &contract, &detail, &deps);

    // ── 5. Output ─────────────────────────────────────────────────────────────
    if json {
        print_json(&report)?;
    } else {
        print_human(&report);
    }

    // ── 6. Threshold check (non-zero exit if risk is at/above threshold) ──────
    if let Some(threshold_level) = threshold_level {
        let actual: RiskLevel = report
            .risk_level
            .to_lowercase()
            .parse()
            .unwrap_or(RiskLevel::Low);
        if actual >= threshold_level {
            if !json {
                eprintln!(
                    "\n  {} Risk level {} meets or exceeds the --threshold {}. Exiting with code 1.",
                    "!".red().bold(),
                    actual.label().red().bold(),
                    threshold_level.label().yellow()
                );
            }
            std::process::exit(1);
        }
    }

    Ok(())
}

// ── API helpers ───────────────────────────────────────────────────────────────

async fn fetch_contract(
    client: &reqwest::Client,
    api_url: &str,
    address: &str,
    network: &str,
) -> Result<Value> {
    let url = format!(
        "{}/api/contracts?contract_id={}&network={}",
        api_url, address, network
    );
    log::debug!("GET {}", url);

    let res = client
        .get(&url)
        .send()
        .await
        .context("Failed to connect to registry API. Is the registry running?")?;

    let status = res.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!(
            "Contract '{}' not found in the {} registry.",
            address,
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

    // Handle paginated list or direct object.
    if let Some(items) = raw["items"].as_array() {
        return items
            .iter()
            .find(|c| {
                c["contract_id"].as_str() == Some(address)
                    || c["id"].as_str() == Some(address)
                    || c["network_configs"].as_object().map_or(false, |nc| {
                        nc.values()
                            .any(|v| v["contract_id"].as_str() == Some(address))
                    })
            })
            .cloned()
            .ok_or_else(|| {
                anyhow::anyhow!("Contract '{}' not found in registry response", address)
            });
    }
    if raw.is_object() && (raw["contract_id"].is_string() || raw["id"].is_string()) {
        return Ok(raw);
    }

    anyhow::bail!("Unexpected registry response format")
}

/// Non-fatal — returns None when the endpoint is absent or returns an error.
async fn fetch_detail(client: &reqwest::Client, api_url: &str, contract: &Value) -> Option<Value> {
    let id = contract["id"]
        .as_str()
        .or(contract["contract_id"].as_str())?;
    let url = format!("{}/api/contracts/{}/verification-status", api_url, id);
    log::debug!("GET {}", url);
    let res = client.get(&url).send().await.ok()?;
    if res.status().is_success() {
        res.json::<Value>().await.ok()
    } else {
        None
    }
}

/// Non-fatal — returns empty vec when the endpoint is absent or returns an error.
async fn fetch_dependencies(client: &reqwest::Client, api_url: &str, address: &str) -> Vec<Value> {
    let url = format!("{}/api/contracts/{}/dependencies", api_url, address);
    log::debug!("GET {}", url);
    let res = match client.get(&url).send().await {
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

// ── Risk assessment ───────────────────────────────────────────────────────────

fn build_report(
    address: &str,
    network: &str,
    contract: &Value,
    detail: &Option<Value>,
    deps: &[Value],
) -> RiskReport {
    let mut findings: Vec<RiskFinding> = Vec::new();
    let mut score: u32 = 0;

    // ── Source verification ───────────────────────────────────────────────────
    let is_verified = contract["is_verified"].as_bool().unwrap_or(false);
    if !is_verified {
        findings.push(RiskFinding {
            category: "verification".to_string(),
            severity: "medium".to_string(),
            title: "Unverified source code".to_string(),
            description:
                "Contract source code has not been verified against the deployed bytecode."
                    .to_string(),
            remediation: Some(
                "Publish and verify the source code in the Soroban Registry.".to_string(),
            ),
        });
        score = score.saturating_add(20);
    }

    // ── Maintenance mode ──────────────────────────────────────────────────────
    if contract["is_maintenance"].as_bool().unwrap_or(false) {
        findings.push(RiskFinding {
            category: "operational".to_string(),
            severity: "high".to_string(),
            title: "Contract is in maintenance mode".to_string(),
            description: "Interactions with this contract may be restricted or unavailable."
                .to_string(),
            remediation: Some(
                "Check the registry for a status update or contact the publisher.".to_string(),
            ),
        });
        score = score.saturating_add(25);
    }

    // ── Health score ──────────────────────────────────────────────────────────
    let health_score = contract["health_score"].as_i64().unwrap_or(0);
    if health_score < 20 {
        findings.push(RiskFinding {
            category: "reputation".to_string(),
            severity: "high".to_string(),
            title: format!("Very low registry health score ({})", health_score),
            description: "Health score below 20 indicates significant issues with this contract's standing in the registry.".to_string(),
            remediation: Some("Review the contract's history and investigate why the health score is low.".to_string()),
        });
        score = score.saturating_add(25);
    } else if health_score < 50 {
        findings.push(RiskFinding {
            category: "reputation".to_string(),
            severity: "medium".to_string(),
            title: format!("Low registry health score ({})", health_score),
            description: "Health score below 50 suggests this contract may have unresolved issues."
                .to_string(),
            remediation: Some(
                "Investigate the registry health score factors for this contract.".to_string(),
            ),
        });
        score = score.saturating_add(15);
    }

    // ── Code age and update frequency ─────────────────────────────────────────
    if let Some(updated_at) = contract["updated_at"].as_str() {
        if let Ok(updated) = chrono::DateTime::parse_from_rfc3339(updated_at) {
            let age_days = (chrono::Utc::now() - updated.with_timezone(&chrono::Utc))
                .num_days()
                .max(0) as u64;
            if age_days > 730 {
                findings.push(RiskFinding {
                    category: "maintenance".to_string(),
                    severity: "medium".to_string(),
                    title: format!("Contract not updated in over {} days", age_days),
                    description: "A long period without updates may indicate an abandoned contract or unpatched vulnerabilities.".to_string(),
                    remediation: Some("Confirm whether the contract is still actively maintained by its publisher.".to_string()),
                });
                score = score.saturating_add(15);
            } else if age_days > 365 {
                findings.push(RiskFinding {
                    category: "maintenance".to_string(),
                    severity: "low".to_string(),
                    title: format!("Contract not updated in over {} days", age_days),
                    description:
                        "No update in over a year — verify the contract is still supported."
                            .to_string(),
                    remediation: Some(
                        "Check the publisher's changelog or release history.".to_string(),
                    ),
                });
                score = score.saturating_add(8);
            }
        }
    }

    // ── Audit status and known vulnerabilities (from detail) ──────────────────
    if let Some(d) = detail {
        // Audit
        match d["audit"]["passed"].as_bool() {
            Some(false) => {
                findings.push(RiskFinding {
                    category: "audit".to_string(),
                    severity: "high".to_string(),
                    title: "Audit failed".to_string(),
                    description: format!(
                        "A security audit by {} did not pass.",
                        d["audit"]["auditor"].as_str().unwrap_or("an unknown auditor")
                    ),
                    remediation: Some("Review the audit report and remediate all flagged issues before integrating.".to_string()),
                });
                score = score.saturating_add(30);
            }
            None => {
                findings.push(RiskFinding {
                    category: "audit".to_string(),
                    severity: "medium".to_string(),
                    title: "No audit record found".to_string(),
                    description: "This contract has not undergone a recorded third-party security audit.".to_string(),
                    remediation: Some("Request or commission a security audit before using this contract in production.".to_string()),
                });
                score = score.saturating_add(15);
            }
            Some(true) => {} // Positive — no score added.
        }

        // Known vulnerabilities from security scan
        if let Some(findings_raw) = d["security_scan"]["findings"].as_array() {
            let mut crit_added = false;
            let mut high_added = false;

            for f in findings_raw {
                let sev = f["severity"].as_str().unwrap_or("info");
                let title = f["title"].as_str().unwrap_or("Unknown finding");
                let desc = f["description"].as_str().unwrap_or("").to_string();

                let remediation = match sev {
                    "critical" => Some(
                        "Address this critical vulnerability immediately before any further use."
                            .to_string(),
                    ),
                    "high" => Some(
                        "Treat this as a high-priority fix before production deployment."
                            .to_string(),
                    ),
                    "medium" => Some(
                        "Investigate and remediate this issue in the next release cycle."
                            .to_string(),
                    ),
                    _ => None,
                };

                findings.push(RiskFinding {
                    category: "vulnerability".to_string(),
                    severity: sev.to_string(),
                    title: format!("[Security Scan] {}", title),
                    description: desc,
                    remediation,
                });

                match sev {
                    "critical" if !crit_added => {
                        score = score.saturating_add(40);
                        crit_added = true;
                    }
                    "high" if !high_added => {
                        score = score.saturating_add(20);
                        high_added = true;
                    }
                    "medium" => score = score.saturating_add(10),
                    _ => {}
                }
            }
        }

        // High-privilege functions from ABI
        let privilege_keywords = [
            "admin",
            "upgrade",
            "migrate",
            "set_admin",
            "transfer_ownership",
            "pause",
            "unpause",
            "emergency",
            "selfdestruct",
            "destroy",
            "initialize",
            "set_owner",
            "grant_role",
            "revoke_role",
        ];

        if let Some(funcs) = d["abi"]["functions"].as_array() {
            let mut priv_funcs: Vec<String> = Vec::new();
            for func in funcs {
                if let Some(name) = func["name"].as_str() {
                    let name_lower = name.to_lowercase();
                    if privilege_keywords.iter().any(|kw| name_lower.contains(kw)) {
                        priv_funcs.push(name.to_string());
                    }
                }
            }
            if !priv_funcs.is_empty() {
                findings.push(RiskFinding {
                    category: "privilege".to_string(),
                    severity: "medium".to_string(),
                    title: format!(
                        "High-privilege functions detected ({})",
                        priv_funcs.join(", ")
                    ),
                    description: "This contract exposes functions that can alter ownership, pause operations, or upgrade logic.".to_string(),
                    remediation: Some("Verify that access control on these functions is correct and review who holds the admin keys.".to_string()),
                });
                score = score.saturating_add(10);
            }
        }
    } else {
        // No detail available — flag as unable to scan
        findings.push(RiskFinding {
            category: "audit".to_string(),
            severity: "medium".to_string(),
            title: "Audit and scan data unavailable".to_string(),
            description:
                "Could not retrieve verification details, security scan, or ABI from the registry."
                    .to_string(),
            remediation: Some(
                "Ensure the registry API is reachable and retry. Manually audit before use."
                    .to_string(),
            ),
        });
        score = score.saturating_add(15);
    }

    // ── Dependency vulnerability score ────────────────────────────────────────
    let unverified_deps: Vec<&Value> = deps
        .iter()
        .filter(|d| !d["is_verified"].as_bool().unwrap_or(false))
        .collect();
    let dep_score: u32 = (unverified_deps.len() as u32 * 5).min(15);

    if !unverified_deps.is_empty() {
        let names: Vec<&str> = unverified_deps
            .iter()
            .filter_map(|d| d["dependency_name"].as_str().or(d["name"].as_str()))
            .collect();
        let names_str = if names.is_empty() {
            format!("{} unverified", unverified_deps.len())
        } else {
            names.join(", ")
        };
        findings.push(RiskFinding {
            category: "dependency".to_string(),
            severity: if dep_score >= 10 { "medium" } else { "low" }.to_string(),
            title: format!("Unverified dependencies: {}", names_str),
            description: format!(
                "{} of {} direct dependenc{} lack source verification in the registry.",
                unverified_deps.len(),
                deps.len(),
                if deps.len() == 1 { "y" } else { "ies" }
            ),
            remediation: Some(
                "Ensure all dependencies are verified in the registry before production use."
                    .to_string(),
            ),
        });
        score = score.saturating_add(dep_score);
    }

    score = score.min(100);
    let level = RiskLevel::from_score(score);
    let recommendations = build_recommendations(&findings, &level);

    RiskReport {
        address: address.to_string(),
        network: network.to_string(),
        risk_score: score,
        risk_level: level.label().to_lowercase(),
        findings,
        dependency_vulnerability_score: dep_score,
        recommendations,
        assessed_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn build_recommendations(findings: &[RiskFinding], level: &RiskLevel) -> Vec<String> {
    let mut recs: Vec<String> = Vec::new();

    let has_category = |cat: &str| findings.iter().any(|f| f.category == cat);
    let has_sev = |sev: &str| findings.iter().any(|f| f.severity == sev);

    if has_sev("critical") {
        recs.push(
            "STOP: Do not integrate this contract. Resolve all critical vulnerabilities first."
                .to_string(),
        );
    }

    if has_category("audit")
        && findings
            .iter()
            .any(|f| f.category == "audit" && f.title.contains("failed"))
    {
        recs.push("Obtain and review the full audit report. Do not deploy until all issues are remediated.".to_string());
    }

    if has_category("audit")
        && findings
            .iter()
            .any(|f| f.category == "audit" && f.title.contains("No audit"))
    {
        recs.push(
            "Commission a third-party security audit before using this contract in production."
                .to_string(),
        );
    }

    if has_category("verification") {
        recs.push(
            "Verify the contract's source code in the registry to increase trust.".to_string(),
        );
    }

    if has_category("privilege") {
        recs.push("Review all high-privilege functions. Confirm access controls are correctly implemented and admin keys are secured.".to_string());
    }

    if has_category("dependency") {
        recs.push(
            "Audit unverified dependencies independently before relying on this contract."
                .to_string(),
        );
    }

    if has_category("maintenance") {
        recs.push(
            "Contact the publisher to confirm the contract is still actively maintained."
                .to_string(),
        );
    }

    if has_category("operational") {
        recs.push(
            "Monitor the contract's maintenance status before initiating any transactions."
                .to_string(),
        );
    }

    if recs.is_empty() {
        match level {
            RiskLevel::Low => recs.push("No significant risks detected. Continue to monitor the contract's registry status over time.".to_string()),
            RiskLevel::Medium => recs.push("Some moderate risks present. Review findings and remediate before production use.".to_string()),
            _ => recs.push("Review all findings above and remediate before integrating this contract.".to_string()),
        }
    }

    recs
}

// ── Output ────────────────────────────────────────────────────────────────────

fn print_json(report: &RiskReport) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(report)?);
    Ok(())
}

fn print_header() {
    println!();
    println!("{}", "Contract Risk Assessment".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
}

fn print_footer() {
    println!("{}", "═".repeat(60).cyan());
    println!();
}

fn severity_label(sev: &str) -> colored::ColoredString {
    match sev {
        "critical" => format!("[{}]", sev.to_uppercase()).red().bold(),
        "high" => format!("[{}]", sev.to_uppercase()).red(),
        "medium" => format!("[{}]", sev.to_uppercase()).yellow(),
        "low" => format!("[{}]", sev.to_uppercase()).bright_black(),
        _ => format!("[{}]", sev.to_uppercase()).normal(),
    }
}

fn print_human(report: &RiskReport) {
    print_header();

    // ── Contract info ─────────────────────────────────────────────────────────
    println!(
        "  {}   {}",
        "Address:".bold(),
        report.address.bright_black()
    );
    println!("  {}   {}", "Network:".bold(), report.network.bright_blue());
    println!("  {}", "Assessed:".bold());
    println!("     {}", report.assessed_at.dimmed());
    println!();

    // ── Overall risk ──────────────────────────────────────────────────────────
    let level: RiskLevel = report.risk_level.parse().unwrap_or(RiskLevel::Low);
    let (level_colored, score_colored) = match level {
        RiskLevel::Critical => (
            level.label().red().bold(),
            report.risk_score.to_string().red().bold(),
        ),
        RiskLevel::High => (level.label().red(), report.risk_score.to_string().red()),
        RiskLevel::Medium => (
            level.label().yellow(),
            report.risk_score.to_string().yellow(),
        ),
        RiskLevel::Low => (level.label().green(), report.risk_score.to_string().green()),
    };

    println!(
        "  {} Overall Risk:  {} (score: {} / 100)",
        match level {
            RiskLevel::Critical | RiskLevel::High => "✘".red().bold(),
            RiskLevel::Medium => "⚠".yellow().bold(),
            RiskLevel::Low => "✔".green().bold(),
        },
        level_colored,
        score_colored
    );
    println!(
        "  {} Dependency Vulnerability Score: {} / 15",
        "·".dimmed(),
        report.dependency_vulnerability_score
    );
    println!();

    // ── Findings ──────────────────────────────────────────────────────────────
    if report.findings.is_empty() {
        println!(
            "  {} {}",
            "✔".green(),
            "No risk findings detected.".green().bold()
        );
    } else {
        println!("  {}", "Findings".bold().underline());
        println!();
        for f in &report.findings {
            println!(
                "  {} {} [{}]",
                severity_label(&f.severity),
                f.title.bold(),
                f.category.dimmed()
            );
            println!("     {}", f.description.dimmed());
            if let Some(rem) = &f.remediation {
                println!("     {} {}", "Fix:".bold(), rem);
            }
            println!();
        }
    }

    // ── Recommendations ───────────────────────────────────────────────────────
    println!("  {}", "Recommendations".bold().underline());
    println!();
    for (i, rec) in report.recommendations.iter().enumerate() {
        println!("  {}. {}", i + 1, rec);
    }
    println!();

    print_footer();
}
