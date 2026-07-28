use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::audit_command::{self, AuditFinding, AuditReport, Severity};

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractAuditResult {
    pub path: String,
    pub risk_level: String,
    pub total_findings: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchAuditSummary {
    pub total_contracts: usize,
    pub high_risk_count: usize,
    pub total_findings: usize,
    pub results: Vec<ContractAuditResult>,
}

pub fn run_batch_audit(
    file: &str,
    format: &str,
    output_dir: Option<&str>,
    fail_on: Option<&str>,
    high_risk_only: bool,
    profile: &str,
    export: Option<&str>,
    json_out: bool,
) -> Result<()> {
    let paths = parse_input_paths(file)?;

    if paths.is_empty() {
        anyhow::bail!("No contract paths provided");
    }

    anyhow::ensure!(
        matches!(format, "text" | "json" | "markdown"),
        "Invalid format: {}. Must be text, json, or markdown",
        format
    );

    if !json_out {
        println!("\n{}", "Batch Audit".bold().cyan());
        println!("{}", "=".repeat(80).cyan());
        println!("Contracts: {}", paths.len());
        println!("Profile: {}", profile);
        if high_risk_only {
            println!("Filter: high-risk only");
        }
        if output_dir.is_some() {
            println!("Output directory: {}", output_dir.unwrap().bright_black());
        }
        println!("{}", "-".repeat(80).cyan());
    }

    let mut results = Vec::new();
    let mut total_findings = 0;
    let mut high_risk_count = 0;

    for (idx, path) in paths.iter().enumerate() {
        if !json_out {
            print!("  [{}/{}] {} ... ", idx + 1, paths.len(), path.bold());
        }

        match run_audit_for_contract(path, profile) {
            Ok(report) => {
                let findings: Vec<_> = if high_risk_only {
                    report
                        .findings
                        .into_iter()
                        .filter(|f| f.severity >= Severity::High)
                        .collect()
                } else {
                    report.findings
                };

                let risk_level = determine_risk_level(&findings);
                let recommendations = generate_recommendations(&findings, profile);

                if risk_level == "high" || risk_level == "critical" {
                    high_risk_count += 1;
                }

                total_findings += findings.len();

                let result = ContractAuditResult {
                    path: path.clone(),
                    risk_level,
                    total_findings: findings.len(),
                    critical: findings
                        .iter()
                        .filter(|f| f.severity == Severity::Critical)
                        .count(),
                    high: findings
                        .iter()
                        .filter(|f| f.severity == Severity::High)
                        .count(),
                    medium: findings
                        .iter()
                        .filter(|f| f.severity == Severity::Medium)
                        .count(),
                    low: findings
                        .iter()
                        .filter(|f| f.severity == Severity::Low)
                        .count(),
                    recommendations,
                };

                results.push(result);

                if !json_out {
                    println!("{}", "✓".green());
                }
            }
            Err(e) => {
                results.push(ContractAuditResult {
                    path: path.clone(),
                    risk_level: "error".to_string(),
                    total_findings: 0,
                    critical: 0,
                    high: 0,
                    medium: 0,
                    low: 0,
                    recommendations: vec![format!("Error: {}", e)],
                });

                if !json_out {
                    println!("{} — {}", "✗".red(), e.to_string().red());
                }
            }
        }
    }

    let summary = BatchAuditSummary {
        total_contracts: paths.len(),
        high_risk_count,
        total_findings,
        results,
    };

    emit_summary(&summary, json_out)?;

    if let Some(export_path) = export {
        write_export(&summary, export_path, format)?;
    }

    if let Some(dir) = output_dir {
        write_per_contract_reports(&summary, dir, format)?;
    }

    if let Some(fail_severity) = fail_on {
        let threshold = fail_severity.parse::<Severity>()?;
        for result in &summary.results {
            let max_severity = if result.critical > 0 {
                Severity::Critical
            } else if result.high > 0 {
                Severity::High
            } else if result.medium > 0 {
                Severity::Medium
            } else if result.low > 0 {
                Severity::Low
            } else {
                continue;
            };

            if max_severity >= threshold {
                anyhow::bail!(
                    "Audit failed: contract {} has {} severity findings",
                    result.path,
                    max_severity.as_str()
                );
            }
        }
    }

    Ok(())
}

fn parse_input_paths(input: &str) -> Result<Vec<String>> {
    let path = Path::new(input);
    if path.is_file() {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read contract paths file: {}", input))?;
        let paths: Vec<String> = content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| line.to_string())
            .collect();
        Ok(paths)
    } else {
        Ok(input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }
}

fn run_audit_for_contract(contract_path: &str, profile: &str) -> Result<AuditReport> {
    let mut report = audit_command::run_to_report(contract_path)?;

    if profile == "basic" {
        report
            .findings
            .retain(|f| f.severity == Severity::Critical || f.severity == Severity::High);
    } else if profile == "comprehensive" {
        // Keep all findings (already included)
    }

    report.summary.total_findings = report.findings.len();
    report.summary.critical = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Critical)
        .count();
    report.summary.high = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::High)
        .count();
    report.summary.medium = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Medium)
        .count();
    report.summary.low = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Low)
        .count();

    Ok(report)
}

fn determine_risk_level(findings: &[AuditFinding]) -> String {
    if findings.is_empty() {
        "low".to_string()
    } else if findings.iter().any(|f| f.severity == Severity::Critical) {
        "critical".to_string()
    } else if findings.iter().any(|f| f.severity == Severity::High) {
        "high".to_string()
    } else if findings.iter().any(|f| f.severity == Severity::Medium) {
        "medium".to_string()
    } else {
        "low".to_string()
    }
}

fn generate_recommendations(findings: &[AuditFinding], _profile: &str) -> Vec<String> {
    let mut recommendations = Vec::new();

    if findings.iter().any(|f| f.title.contains("panic")) {
        recommendations.push(
            "Replace panic! paths with explicit error handling using Result types".to_string(),
        );
    }

    if findings.iter().any(|f| f.title.contains("unsafe")) {
        recommendations.push("Review and formally verify all unsafe code blocks".to_string());
    }

    if findings.iter().any(|f| f.title.contains("Authorization")) {
        recommendations
            .push("Ensure all state-modifying methods enforce authorization checks".to_string());
    }

    if findings.iter().any(|f| f.title.contains("Arithmetic")) {
        recommendations
            .push("Use checked arithmetic or add explicit overflow assumptions".to_string());
    }

    if findings.iter().any(|f| f.title.contains("Wildcard")) {
        recommendations.push("Pin all dependency versions before production release".to_string());
    }

    recommendations
}

fn write_per_contract_reports(
    summary: &BatchAuditSummary,
    output_dir: &str,
    format: &str,
) -> Result<()> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir))?;

    for result in &summary.results {
        let file_name = Path::new(&result.path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("contract");

        let ext = match format {
            "json" => "json",
            "markdown" | "md" => "md",
            _ => "txt",
        };

        let report_path = Path::new(output_dir).join(format!("{}.{}", file_name, ext));
        let content = format_contract_report(result, format);

        fs::write(&report_path, content)
            .with_context(|| format!("Failed to write report to {}", report_path.display()))?;
    }

    Ok(())
}

fn format_contract_report(result: &ContractAuditResult, format: &str) -> String {
    match format {
        "json" => serde_json::to_string_pretty(result).unwrap_or_default(),
        "markdown" | "md" => {
            let mut out = format!("# Audit Report: {}\n\n", result.path);
            out.push_str(&format!("**Risk Level**: {}\n\n", result.risk_level));
            out.push_str(&format!(
                "- Critical: {}\n- High: {}\n- Medium: {}\n- Low: {}\n\n",
                result.critical, result.high, result.medium, result.low
            ));
            if !result.recommendations.is_empty() {
                out.push_str("## Recommendations\n\n");
                for rec in &result.recommendations {
                    out.push_str(&format!("- {}\n", rec));
                }
            }
            out
        }
        _ => {
            format!(
                "Contract: {}\nRisk Level: {}\nFindings: {}\nCritical: {}, High: {}, Medium: {}, Low: {}\n\nRecommendations:\n{}",
                result.path,
                result.risk_level,
                result.total_findings,
                result.critical,
                result.high,
                result.medium,
                result.low,
                result.recommendations.join("\n")
            )
        }
    }
}

fn emit_summary(summary: &BatchAuditSummary, json_out: bool) -> Result<()> {
    if json_out {
        println!("{}", serde_json::to_string_pretty(summary)?);
        return Ok(());
    }

    println!("\n{}", "Audit Results".bold().cyan());
    println!(
        "Total contracts: {} | High-risk: {} | Total findings: {}",
        summary.total_contracts, summary.high_risk_count, summary.total_findings
    );
    println!("{}", "-".repeat(80).cyan());

    for result in &summary.results {
        let risk_color = match result.risk_level.as_str() {
            "critical" => result.risk_level.red(),
            "high" => result.risk_level.yellow(),
            "medium" => result.risk_level.bright_black(),
            _ => result.risk_level.green(),
        };

        println!(
            "{} {} [{}] findings: {} (critical: {}, high: {}, medium: {}, low: {})",
            if result.risk_level == "error" {
                "✗"
            } else {
                "◆"
            },
            result.path.bold(),
            risk_color,
            result.total_findings,
            result.critical,
            result.high,
            result.medium,
            result.low
        );

        if !result.recommendations.is_empty() {
            for rec in &result.recommendations {
                println!("  → {}", rec.bright_black());
            }
        }
    }

    Ok(())
}

fn write_export(summary: &BatchAuditSummary, export_path: &str, format: &str) -> Result<()> {
    let output = match format.to_lowercase().as_str() {
        "json" => serde_json::to_string_pretty(summary)?,
        "markdown" | "md" => format_as_markdown(summary),
        _ => serde_json::to_string_pretty(summary)?,
    };

    fs::write(export_path, output)
        .with_context(|| format!("Failed to write audit export to {}", export_path))?;

    Ok(())
}

fn format_as_markdown(summary: &BatchAuditSummary) -> String {
    let mut out = String::new();
    out.push_str("# Batch Audit Report\n\n");
    out.push_str(&format!(
        "- **Total contracts**: {}\n",
        summary.total_contracts
    ));
    out.push_str(&format!(
        "- **High-risk contracts**: {}\n",
        summary.high_risk_count
    ));
    out.push_str(&format!(
        "- **Total findings**: {}\n\n",
        summary.total_findings
    ));

    for result in &summary.results {
        out.push_str(&format!("## `{}`\n\n", result.path));
        out.push_str(&format!("**Risk Level**: {}\n\n", result.risk_level));
        out.push_str(&format!(
            "- Critical: {}\n- High: {}\n- Medium: {}\n- Low: {}\n\n",
            result.critical, result.high, result.medium, result.low
        ));

        if !result.recommendations.is_empty() {
            out.push_str("### Recommendations\n\n");
            for rec in &result.recommendations {
                out.push_str(&format!("- {}\n", rec));
            }
            out.push_str("\n");
        }
    }

    out
}
