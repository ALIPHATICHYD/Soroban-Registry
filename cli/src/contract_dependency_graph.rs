//! `soroban-registry contract dependencies|dependents|dependency-risk` (Issue #1147).
//!
//! Thin, faithful clients for the four dependency endpoints. Two conventions
//! are load-bearing:
//!
//! - **`--json` prints the API response unmodified.** Reshaping it here would
//!   mean two schemas to keep in step, and the documented one is the API's.
//!   Scripts get exactly what `docs/dependency-graph.md` describes.
//! - **`--fail-on` compares against `overall_risk.effective_severity`,** which
//!   the server computes. Recomputing a severity client-side would let the CLI
//!   and the registry disagree about whether a deploy should be blocked.
//!
//! The address is positional, matching the other 18 `contract` subcommands.

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;

/// Severity ordering for `--fail-on`. Deliberately the same four levels as the
/// server's `IssueSeverity`, parsed from the same lowercase spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            other => anyhow::bail!(
                "invalid severity '{other}'. Expected one of: low, medium, high, critical"
            ),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }
}

/// Query options shared by the traversal commands.
#[derive(Debug, Default, Clone)]
pub struct GraphOptions {
    pub network: Option<String>,
    pub depth: Option<u32>,
    pub transitive: bool,
    pub include_telemetry: bool,
    pub json: bool,
}

/// `contract dependencies <ADDRESS>`
pub async fn dependencies(api_url: &str, address: &str, opts: &GraphOptions) -> Result<()> {
    let value = fetch(api_url, address, "dependencies", opts).await?;
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }
    print_tree(&value, "Dependencies");
    Ok(())
}

/// `contract dependents <ADDRESS>`
pub async fn dependents(api_url: &str, address: &str, opts: &GraphOptions) -> Result<()> {
    let value = fetch(api_url, address, "dependents", opts).await?;
    if opts.json {
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }
    print_tree(&value, "Dependents");
    Ok(())
}

/// `contract dependency-risk <ADDRESS> [--fail-on <severity>]`
///
/// Returns `Ok(true)` when the report meets or exceeds `fail_on`. Exiting is
/// left to the caller so this stays testable without a process boundary.
pub async fn risk(
    api_url: &str,
    address: &str,
    opts: &GraphOptions,
    fail_on: Option<Severity>,
) -> Result<bool> {
    let value = fetch(api_url, address, "dependency-risk", opts).await?;

    if opts.json {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        print_risk(&value);
    }

    let effective = value
        .get("overall_risk")
        .and_then(|r| r.get("effective_severity"))
        .and_then(Value::as_str)
        .map(Severity::parse)
        .transpose()?;

    let breached = match (fail_on, effective) {
        (Some(threshold), Some(actual)) => actual >= threshold,
        // No findings at all can never breach a threshold. This is only
        // reachable because diagnostics are kept out of `findings`.
        (Some(_), None) => false,
        (None, _) => false,
    };

    if breached && !opts.json {
        let threshold = fail_on.expect("breached implies a threshold");
        eprintln!(
            "\n  {} Overall risk {} meets or exceeds --fail-on {}. Exiting with code 1.",
            "!".red().bold(),
            effective
                .expect("breached implies a severity")
                .label()
                .red()
                .bold(),
            threshold.label().yellow()
        );
    }

    Ok(breached)
}

async fn fetch(api_url: &str, address: &str, endpoint: &str, opts: &GraphOptions) -> Result<Value> {
    let mut url = format!(
        "{}/api/contracts/{}/{}",
        api_url.trim_end_matches('/'),
        address,
        endpoint
    );

    let mut params: Vec<String> = Vec::new();
    if let Some(network) = &opts.network {
        params.push(format!("network={network}"));
    }
    if let Some(depth) = opts.depth {
        params.push(format!("depth={depth}"));
    }
    params.push(format!("transitive={}", opts.transitive));
    if opts.include_telemetry {
        params.push("include_telemetry=true".to_string());
    }
    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    log::debug!("GET {url}");
    let response = crate::net::client()
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to reach the registry API. Is the registry running?")?;

    let status = response.status();
    let value: Value = response.json().await.unwrap_or(Value::Null);

    if status.is_success() {
        return Ok(value);
    }

    // 409 means the address exists on more than one network. Surfacing the
    // candidates is the whole point of that status, so they are printed rather
    // than collapsed into a generic failure.
    if status.as_u16() == 409 {
        let candidates = value
            .get("details")
            .and_then(|d| d.get("candidates"))
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| row.get("network").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        anyhow::bail!(
            "'{address}' is registered on more than one network ({candidates}). Re-run with --network <NETWORK>."
        );
    }

    let message = value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("no message");
    anyhow::bail!("request failed ({status}): {message}");
}

// ── Human-readable output ───────────────────────────────────────────────────

fn print_tree(value: &Value, title: &str) {
    let root = value.get("root");
    println!("\n{}", title.bold().cyan());
    println!("{}", "=".repeat(72).cyan());

    if let Some(root) = root {
        println!(
            "{:<14}{}",
            "Contract:".bold(),
            root.get("name")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        );
        println!(
            "{:<14}{}",
            "Address:".bold(),
            root.get("contract_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .bright_black()
        );
    }

    println!(
        "{:<14}{}",
        "Total:".bold(),
        value
            .get("total_dependencies")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    println!(
        "{:<14}{}",
        "Max depth:".bold(),
        value.get("max_depth").and_then(Value::as_u64).unwrap_or(0)
    );

    if value
        .get("has_circular")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        println!("{:<14}{}", "Cycles:".bold(), "detected".yellow());
    }

    println!();
    if let Some(children) = root
        .and_then(|r| r.get("dependencies"))
        .and_then(Value::as_array)
    {
        if children.is_empty() {
            println!("  {}", "none".bright_black());
        }
        for child in children {
            print_node(child, 1);
        }
    }

    print_diagnostics(
        root.and_then(|r| r.get("visualization_hints"))
            .and_then(|h| h.get("diagnostics")),
    );
    println!();
}

fn print_node(node: &Value, depth: usize) {
    let indent = "  ".repeat(depth);
    let id = node
        .get("contract_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let name = node.get("name").and_then(Value::as_str);
    let status = node.get("status").and_then(Value::as_str).unwrap_or("");
    let circular = node
        .get("is_circular")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let label = match name {
        Some(name) => format!("{name} ({id})"),
        None => id.to_string(),
    };
    let marker = if circular {
        " [cycle]".yellow().to_string()
    } else {
        String::new()
    };

    println!("{indent}- {label} {}{marker}", status.bright_black());

    if let Some(children) = node.get("dependencies").and_then(Value::as_array) {
        for child in children {
            print_node(child, depth + 1);
        }
    }
}

fn print_risk(value: &Value) {
    println!("\n{}", "Dependency Risk".bold().cyan());
    println!("{}", "=".repeat(72).cyan());

    let overall = severity_of(value.get("overall_risk"));
    let direct = severity_of(value.get("direct_risk"));
    println!("{:<16}{}", "Overall:".bold(), colorize(overall));
    println!("{:<16}{}", "Direct only:".bold(), colorize(direct));
    println!(
        "{:<16}{}",
        "Dependencies:".bold(),
        value
            .get("total_dependencies")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    if value
        .get("truncated")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        println!(
            "{:<16}{}",
            "Note:".bold(),
            "graph was truncated; findings may be incomplete".yellow()
        );
    }

    print_findings("Direct findings", value.get("direct_findings"));
    print_findings("Inherited findings", value.get("inherited_findings"));
    print_diagnostics(value.get("diagnostics"));
    println!();
}

fn print_findings(title: &str, findings: Option<&Value>) {
    let Some(findings) = findings.and_then(Value::as_array) else {
        return;
    };
    println!("\n{}", title.bold());
    if findings.is_empty() {
        println!("  {}", "none".green());
        return;
    }
    for finding in findings {
        let severity = finding
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let rule = finding
            .get("rule_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let depth = finding
            .get("inherited_via_depth")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let detail = finding.get("detail").and_then(Value::as_str).unwrap_or("");
        println!(
            "  [{}] {} (depth {depth}) {}",
            colorize(Some(severity.to_string())),
            rule,
            detail.bright_black()
        );
        // The path is the reason this feature exists: it is not enough to say a
        // dependency is vulnerable, the operator has to know which chain reaches
        // it in order to fix it.
        if let Some(path) = finding.get("path").and_then(Value::as_array) {
            if path.len() > 1 {
                let hops: Vec<&str> = path.iter().filter_map(Value::as_str).collect();
                println!("      via {}", hops.join(" -> ").bright_black());
            }
        }
    }
}

fn print_diagnostics(diagnostics: Option<&Value>) {
    let Some(diagnostics) = diagnostics.and_then(Value::as_array) else {
        return;
    };
    if diagnostics.is_empty() {
        return;
    }
    println!("\n{}", "Diagnostics".bold());
    for diagnostic in diagnostics {
        let kind = diagnostic
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let detail = diagnostic
            .get("detail")
            .and_then(Value::as_str)
            .unwrap_or("");
        println!("  {} {}", kind.yellow(), detail.bright_black());
        // Two diagnostics of the same kind differ only by their path -- two
        // distinct cycles through the same graph, say -- so printing the detail
        // alone makes genuinely different findings look like duplicates.
        if let Some(path) = diagnostic.get("path").and_then(Value::as_array) {
            if path.len() > 1 {
                let hops: Vec<&str> = path.iter().filter_map(Value::as_str).collect();
                println!("      {}", hops.join(" -> ").bright_black());
            }
        }
    }
}

fn severity_of(risk: Option<&Value>) -> Option<String> {
    risk?
        .get("effective_severity")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn colorize(severity: Option<String>) -> String {
    match severity.as_deref() {
        Some("Critical") | Some("critical") => "CRITICAL".red().bold().to_string(),
        Some("High") | Some("high") => "HIGH".red().to_string(),
        Some("Medium") | Some("medium") => "MEDIUM".yellow().to_string(),
        Some("Low") | Some("low") => "LOW".to_string(),
        _ => "none".green().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_parses_case_insensitively() {
        assert_eq!(Severity::parse("HIGH").unwrap(), Severity::High);
        assert_eq!(Severity::parse(" critical ").unwrap(), Severity::Critical);
        assert!(Severity::parse("catastrophic").is_err());
    }

    #[test]
    fn severity_orders_low_to_critical() {
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
    }

    #[test]
    fn colorize_reports_no_severity_as_none() {
        // A report with zero findings has no effective severity. It must not be
        // rendered as "LOW", which would imply a finding exists.
        assert!(colorize(None).contains("none"));
    }
}
