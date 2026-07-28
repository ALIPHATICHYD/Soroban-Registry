use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    fn rank(&self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Critical => 4,
        }
    }
}

impl PartialOrd for Severity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Severity {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank().cmp(&other.rank())
    }
}

impl std::str::FromStr for Severity {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            _ => anyhow::bail!("Unsupported severity '{}'", value),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFinding {
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    pub file: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub contract_path: String,
    pub summary: AuditSummary,
    pub findings: Vec<AuditFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_findings: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

pub fn run(
    contract_path: &str,
    format: &str,
    output: Option<&str>,
    fail_on: Option<&str>,
) -> Result<()> {
    let sources = collect_sources(Path::new(contract_path))?;
    let mut findings = Vec::new();

    for source in sources {
        let content = fs::read_to_string(&source)
            .with_context(|| format!("Failed to read source file: {}", source.display()))?;
        scan_source(&source, &content, &mut findings);
    }

    let summary = AuditSummary {
        total_findings: findings.len(),
        critical: findings
            .iter()
            .filter(|finding| finding.severity == Severity::Critical)
            .count(),
        high: findings
            .iter()
            .filter(|finding| finding.severity == Severity::High)
            .count(),
        medium: findings
            .iter()
            .filter(|finding| finding.severity == Severity::Medium)
            .count(),
        low: findings
            .iter()
            .filter(|finding| finding.severity == Severity::Low)
            .count(),
    };

    let report = AuditReport {
        contract_path: contract_path.to_string(),
        summary,
        findings,
    };

    let rendered = match format.to_lowercase().as_str() {
        "json" => serde_json::to_string_pretty(&report)?,
        "markdown" | "md" => to_markdown(&report),
        _ => to_text(&report),
    };

    if let Some(output_path) = output {
        fs::write(output_path, rendered)
            .with_context(|| format!("Failed to write audit report: {}", output_path))?;
        println!("{} Audit report written to {}", "✓".green(), output_path);
    } else {
        println!("{}", rendered);
    }

    if let Some(threshold) = fail_on {
        let threshold = threshold.parse::<Severity>()?;
        if report
            .findings
            .iter()
            .any(|finding| finding.severity >= threshold)
        {
            anyhow::bail!(
                "Audit failed due to findings at or above {}",
                threshold.as_str()
            );
        }
    }

    Ok(())
}

pub fn collect_sources(path: &Path) -> Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_path = entry.path();
        if file_path.is_dir() {
            continue;
        }
        if matches!(
            file_path.extension().and_then(|ext| ext.to_str()),
            Some("rs") | Some("toml")
        ) {
            files.push(file_path);
        }
    }
    Ok(files)
}

pub fn scan_source(path: &Path, content: &str, findings: &mut Vec<AuditFinding>) {
    let file = path.display().to_string();

    if content.contains("unwrap(") || content.contains("expect(") {
        findings.push(AuditFinding {
            severity: Severity::Medium,
            title: "Unchecked panic path".to_string(),
            detail: "Found unwrap/expect usage. Replace with explicit error handling for contract safety.".to_string(),
            file: file.clone(),
            category: "sast".to_string(),
        });
    }

    if content.contains("panic!") {
        findings.push(AuditFinding {
            severity: Severity::High,
            title: "Explicit panic detected".to_string(),
            detail: "panic! found in source. Audit contract call paths before deployment."
                .to_string(),
            file: file.clone(),
            category: "sast".to_string(),
        });
    }

    if content.contains("unsafe ") {
        findings.push(AuditFinding {
            severity: Severity::High,
            title: "Unsafe block detected".to_string(),
            detail: "unsafe code paths require manual review and formal verification.".to_string(),
            file: file.clone(),
            category: "formal-verification".to_string(),
        });
    }

    if !content.contains("require_auth")
        && path.extension().and_then(|ext| ext.to_str()) == Some("rs")
    {
        findings.push(AuditFinding {
            severity: Severity::Medium,
            title: "Authorization check not detected".to_string(),
            detail: "No require_auth call found in this file. Confirm write methods enforce caller auth.".to_string(),
            file: file.clone(),
            category: "access-control".to_string(),
        });
    }

    if content.contains('*') && !content.contains("checked_") {
        findings.push(AuditFinding {
            severity: Severity::Low,
            title: "Arithmetic path should be reviewed".to_string(),
            detail:
                "Multiplication detected. Consider checked math or explicit overflow assumptions."
                    .to_string(),
            file: file.clone(),
            category: "sast".to_string(),
        });
    }

    if path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml")
        && content.contains('*')
    {
        findings.push(AuditFinding {
            severity: Severity::Medium,
            title: "Wildcard dependency version".to_string(),
            detail: "Dependency versions should be pinned before release audits.".to_string(),
            file,
            category: "dependency".to_string(),
        });
    }
}

fn to_text(report: &AuditReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "\nContract audit summary for {}\n{}\n",
        report.contract_path,
        "=".repeat(72)
    ));
    out.push_str(&format!(
        "Findings: {} total | critical {} | high {} | medium {} | low {}\n\n",
        report.summary.total_findings,
        report.summary.critical,
        report.summary.high,
        report.summary.medium,
        report.summary.low
    ));

    if report.findings.is_empty() {
        out.push_str("No heuristic findings detected.\n");
        return out;
    }

    for finding in &report.findings {
        out.push_str(&format!(
            "[{}] {} ({})\n{}\n{}\n\n",
            finding.severity.as_str(),
            finding.title,
            finding.category,
            finding.file,
            finding.detail
        ));
    }

    out
}

fn to_markdown(report: &AuditReport) -> String {
    let mut out = format!(
        "# Audit Report\n\n- Contract path: `{}`\n- Total findings: `{}`\n\n",
        report.contract_path, report.summary.total_findings
    );
    for finding in &report.findings {
        out.push_str(&format!(
            "## {} `{}`\n\n- Category: `{}`\n- File: `{}`\n- Detail: {}\n\n",
            finding.title,
            finding.severity.as_str(),
            finding.category,
            finding.file,
            finding.detail
        ));
    }
    out
}

pub fn run_to_report(contract_path: &str) -> Result<AuditReport> {
    let sources = collect_sources(Path::new(contract_path))?;
    let mut findings = Vec::new();

    for source in sources {
        let content = fs::read_to_string(&source)
            .with_context(|| format!("Failed to read source file: {}", source.display()))?;
        scan_source(&source, &content, &mut findings);
    }

    let summary = AuditSummary {
        total_findings: findings.len(),
        critical: findings
            .iter()
            .filter(|finding| finding.severity == Severity::Critical)
            .count(),
        high: findings
            .iter()
            .filter(|finding| finding.severity == Severity::High)
            .count(),
        medium: findings
            .iter()
            .filter(|finding| finding.severity == Severity::Medium)
            .count(),
        low: findings
            .iter()
            .filter(|finding| finding.severity == Severity::Low)
            .count(),
    };

    Ok(AuditReport {
        contract_path: contract_path.to_string(),
        summary,
        findings,
    })
}
