#![allow(dead_code)]

use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{Duration, Instant};
use sysinfo::{System, SystemExt};

// ── Types ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Pass => write!(f, "{}", "PASS".green().bold()),
            CheckStatus::Warn => write!(f, "{}", "WARN".yellow().bold()),
            CheckStatus::Fail => write!(f, "{}", "FAIL".red().bold()),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    pub detail: Option<String>,
    pub recommendation: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PerfResult {
    pub name: String,
    pub min_ms: u64,
    pub avg_ms: u64,
    pub max_ms: u64,
    pub samples: u8,
    pub status: CheckStatus,
}

#[derive(Debug, Serialize, Clone)]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub cli_version: String,
    pub config_dir_bytes: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DiagnosticReport {
    pub timestamp: String,
    pub cli_version: String,
    pub api_url: String,
    pub system: SystemInfo,
    pub checks: Vec<CheckResult>,
    pub performance: Vec<PerfResult>,
    pub issues_found: usize,
    pub recommendations: Vec<String>,
}

pub struct DiagnosticArgs<'a> {
    pub api_url: &'a str,
    pub detailed: bool,
    pub export: Option<&'a str>,
    pub json: bool,
}

// ── Internal checks ────────────────────────────────────────────────────────────

async fn check_config_validity(api_url: &str) -> CheckResult {
    let start = Instant::now();
    let config_path = match dirs::home_dir() {
        Some(h) => h.join(".soroban-registry").join("config.toml"),
        None => {
            return CheckResult {
                name: "Config Validity".to_string(),
                status: CheckStatus::Warn,
                message: "Could not determine home directory".to_string(),
                detail: None,
                recommendation: Some("Ensure HOME is set in your environment".to_string()),
                duration_ms: Some(start.elapsed().as_millis() as u64),
            };
        }
    };

    let file_exists = config_path.exists();
    let url_ok = api_url.starts_with("http://") || api_url.starts_with("https://");

    let duration_ms = Some(start.elapsed().as_millis() as u64);

    match (file_exists, url_ok) {
        (true, true) => CheckResult {
            name: "Config Validity".to_string(),
            status: CheckStatus::Pass,
            message: format!("Config file found at {}", config_path.display()),
            detail: Some(format!("API URL: {}", api_url)),
            recommendation: None,
            duration_ms,
        },
        (false, true) => CheckResult {
            name: "Config Validity".to_string(),
            status: CheckStatus::Warn,
            message: "Config file not found; using defaults".to_string(),
            detail: Some(format!("Expected: {}", config_path.display())),
            recommendation: Some(
                "Run `soroban-registry config init` to create a config file".to_string(),
            ),
            duration_ms,
        },
        (_, false) => CheckResult {
            name: "Config Validity".to_string(),
            status: CheckStatus::Fail,
            message: format!("Invalid API URL: {}", api_url),
            detail: Some("URL must start with http:// or https://".to_string()),
            recommendation: Some(
                "Set a valid API URL with --api-url or in your config file".to_string(),
            ),
            duration_ms,
        },
    }
}

async fn check_api_connectivity(api_url: &str) -> CheckResult {
    let url = format!("{}/api/contracts?page_size=1", api_url);
    let start = Instant::now();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    match client.get(&url).send().await {
        Ok(resp) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let status_code = resp.status().as_u16();
            if elapsed <= 500 {
                CheckResult {
                    name: "API Connectivity".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("API reachable in {}ms (HTTP {})", elapsed, status_code),
                    detail: Some(format!("URL: {}", url)),
                    recommendation: None,
                    duration_ms: Some(elapsed),
                }
            } else if elapsed <= 2000 {
                CheckResult {
                    name: "API Connectivity".to_string(),
                    status: CheckStatus::Warn,
                    message: format!(
                        "API reachable but slow: {}ms (HTTP {})",
                        elapsed, status_code
                    ),
                    detail: Some(format!("URL: {}", url)),
                    recommendation: Some("Check network conditions or API server load".to_string()),
                    duration_ms: Some(elapsed),
                }
            } else {
                CheckResult {
                    name: "API Connectivity".to_string(),
                    status: CheckStatus::Warn,
                    message: format!("API very slow: {}ms (HTTP {})", elapsed, status_code),
                    detail: Some(format!("URL: {}", url)),
                    recommendation: Some(
                        "Consider using a closer API endpoint or checking server health"
                            .to_string(),
                    ),
                    duration_ms: Some(elapsed),
                }
            }
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let (message, recommendation) = if e.is_timeout() {
                (
                    "API request timed out (>5s)".to_string(),
                    "Check that the API server is running and reachable".to_string(),
                )
            } else if e.is_connect() {
                (
                    format!("Cannot connect to API: {}", api_url),
                    "Verify the API URL and that the server is running".to_string(),
                )
            } else {
                (
                    format!("API request failed: {}", e),
                    "Check network connectivity and API server status".to_string(),
                )
            };
            CheckResult {
                name: "API Connectivity".to_string(),
                status: CheckStatus::Fail,
                message,
                detail: Some(format!("URL: {}", url)),
                recommendation: Some(recommendation),
                duration_ms: Some(elapsed),
            }
        }
    }
}

async fn check_auth_status() -> CheckResult {
    let start = Instant::now();
    let auth_path = match dirs::home_dir() {
        Some(h) => h.join(".soroban-registry").join("auth.json"),
        None => {
            return CheckResult {
                name: "Auth Status".to_string(),
                status: CheckStatus::Warn,
                message: "Could not determine home directory".to_string(),
                detail: None,
                recommendation: Some("Ensure HOME is set in your environment".to_string()),
                duration_ms: Some(start.elapsed().as_millis() as u64),
            };
        }
    };

    let duration_ms = Some(start.elapsed().as_millis() as u64);

    let contents = match fs::read_to_string(&auth_path) {
        Ok(c) => c,
        Err(_) => {
            return CheckResult {
                name: "Auth Status".to_string(),
                status: CheckStatus::Warn,
                message: "Not logged in (no auth session found)".to_string(),
                detail: Some(format!("Expected: {}", auth_path.display())),
                recommendation: Some(
                    "Run `soroban-registry auth login` to authenticate".to_string(),
                ),
                duration_ms,
            };
        }
    };

    let session: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(v) => v,
        Err(_) => {
            return CheckResult {
                name: "Auth Status".to_string(),
                status: CheckStatus::Fail,
                message: "Auth file is corrupted or invalid JSON".to_string(),
                detail: Some(format!("Path: {}", auth_path.display())),
                recommendation: Some(
                    "Delete the auth file and run `soroban-registry auth login` again".to_string(),
                ),
                duration_ms,
            };
        }
    };

    let expires_at = session
        .get("access_token_expires_at")
        .and_then(|v| v.as_str())
        .or_else(|| {
            session
                .get("access_token_expires_at")
                .and_then(|v| v.as_i64())
                .map(|_| "numeric")
        });

    if let Some(expires_str) = expires_at {
        if expires_str == "numeric" {
            return CheckResult {
                name: "Auth Status".to_string(),
                status: CheckStatus::Pass,
                message: "Authenticated (expiry is a timestamp)".to_string(),
                detail: None,
                recommendation: None,
                duration_ms,
            };
        }

        match chrono::DateTime::parse_from_rfc3339(expires_str) {
            Ok(expiry) => {
                let now = Utc::now();
                let expiry_utc = expiry.with_timezone(&chrono::Utc);
                let remaining = expiry_utc.signed_duration_since(now);

                if remaining.num_seconds() <= 0 {
                    CheckResult {
                        name: "Auth Status".to_string(),
                        status: CheckStatus::Fail,
                        message: "Auth token has expired".to_string(),
                        detail: Some(format!("Expired at: {}", expires_str)),
                        recommendation: Some(
                            "Run `soroban-registry auth login` to refresh your session".to_string(),
                        ),
                        duration_ms,
                    }
                } else if remaining.num_seconds() < 3600 {
                    CheckResult {
                        name: "Auth Status".to_string(),
                        status: CheckStatus::Warn,
                        message: format!(
                            "Auth token expires soon ({}m remaining)",
                            remaining.num_minutes()
                        ),
                        detail: Some(format!("Expires at: {}", expires_str)),
                        recommendation: Some(
                            "Run `soroban-registry auth login` to refresh your session".to_string(),
                        ),
                        duration_ms,
                    }
                } else {
                    CheckResult {
                        name: "Auth Status".to_string(),
                        status: CheckStatus::Pass,
                        message: format!(
                            "Authenticated (expires in {}h {}m)",
                            remaining.num_hours(),
                            remaining.num_minutes() % 60
                        ),
                        detail: Some(format!("Expires at: {}", expires_str)),
                        recommendation: None,
                        duration_ms,
                    }
                }
            }
            Err(_) => CheckResult {
                name: "Auth Status".to_string(),
                status: CheckStatus::Warn,
                message: "Could not parse token expiry".to_string(),
                detail: Some(format!("access_token_expires_at: {}", expires_str)),
                recommendation: Some("Re-authenticate to ensure a fresh session".to_string()),
                duration_ms,
            },
        }
    } else {
        CheckResult {
            name: "Auth Status".to_string(),
            status: CheckStatus::Warn,
            message: "Auth file exists but has no expiry field".to_string(),
            detail: None,
            recommendation: Some("Re-authenticate with `soroban-registry auth login`".to_string()),
            duration_ms,
        }
    }
}

fn gather_system_info(api_url: &str) -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    let os_name = sys.name().unwrap_or_else(|| "Unknown".to_string());
    let os_version = sys
        .long_os_version()
        .unwrap_or_else(|| "Unknown".to_string());
    let cli_version = env!("CARGO_PKG_VERSION").to_string();

    let config_dir_bytes = dirs::home_dir().and_then(|h| {
        let dir = h.join(".soroban-registry");
        dir_size(&dir).ok()
    });

    let _ = api_url;
    SystemInfo {
        os_name,
        os_version,
        cli_version,
        config_dir_bytes,
    }
}

fn dir_size(path: &std::path::Path) -> Result<u64> {
    let mut total = 0u64;
    if !path.exists() {
        return Ok(0);
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_file() {
            total += meta.len();
        } else if meta.is_dir() {
            total += dir_size(&entry.path())?;
        }
    }
    Ok(total)
}

async fn perf_api_response_time(api_url: &str) -> PerfResult {
    let url = format!("{}/api/contracts?page_size=1", api_url);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());

    let mut samples: Vec<u64> = Vec::with_capacity(5);

    for _ in 0..5 {
        let start = Instant::now();
        let _ = client.get(&url).send().await;
        samples.push(start.elapsed().as_millis() as u64);
    }

    let min_ms = *samples.iter().min().unwrap_or(&0);
    let max_ms = *samples.iter().max().unwrap_or(&0);
    let avg_ms = samples.iter().sum::<u64>() / samples.len() as u64;

    let status = if avg_ms > 1000 {
        CheckStatus::Warn
    } else {
        CheckStatus::Pass
    };

    PerfResult {
        name: "API Response Time".to_string(),
        min_ms,
        avg_ms,
        max_ms,
        samples: samples.len() as u8,
        status,
    }
}

// ── Report builder ─────────────────────────────────────────────────────────────

async fn build_report(args: &DiagnosticArgs<'_>) -> DiagnosticReport {
    let config_check = check_config_validity(args.api_url).await;
    let connectivity_check = check_api_connectivity(args.api_url).await;
    let auth_check = check_auth_status().await;

    let perf = if args.detailed {
        vec![perf_api_response_time(args.api_url).await]
    } else {
        vec![]
    };

    let checks = vec![config_check, connectivity_check, auth_check];

    let issues_found = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Fail || c.status == CheckStatus::Warn)
        .count();

    let recommendations: Vec<String> = checks
        .iter()
        .filter_map(|c| c.recommendation.clone())
        .collect();

    let system = gather_system_info(args.api_url);

    DiagnosticReport {
        timestamp: Utc::now().to_rfc3339(),
        cli_version: env!("CARGO_PKG_VERSION").to_string(),
        api_url: args.api_url.to_string(),
        system,
        checks,
        performance: perf,
        issues_found,
        recommendations,
    }
}

// ── Output formatting ──────────────────────────────────────────────────────────

fn print_report(report: &DiagnosticReport, detailed: bool) {
    println!();
    println!("{}", "═══ Soroban Registry Diagnostics ═══".bold());
    println!("  {} {}", "Timestamp:".dimmed(), report.timestamp);
    println!("  {} {}", "CLI Version:".dimmed(), report.cli_version);
    println!("  {} {}", "API URL:".dimmed(), report.api_url);
    println!(
        "  {} {} ({})",
        "OS:".dimmed(),
        report.system.os_name,
        report.system.os_version
    );
    if let Some(bytes) = report.system.config_dir_bytes {
        println!("  {} {} bytes", "Config dir size:".dimmed(), bytes);
    }

    println!();
    println!("{}", "── Health Checks ──".bold());
    println!(
        "  {:<28} {:<8} {}",
        "Check".bold(),
        "Status".bold(),
        "Message".bold()
    );
    println!("  {}", "─".repeat(72));

    for check in &report.checks {
        let status_str = match &check.status {
            CheckStatus::Pass => "PASS".green().bold().to_string(),
            CheckStatus::Warn => "WARN".yellow().bold().to_string(),
            CheckStatus::Fail => "FAIL".red().bold().to_string(),
        };
        let duration = check
            .duration_ms
            .map(|d| format!(" ({}ms)", d))
            .unwrap_or_default();
        println!(
            "  {:<28} {:<8} {}{}",
            check.name, status_str, check.message, duration
        );
        if detailed {
            if let Some(detail) = &check.detail {
                println!("  {:<28}          {}", "", detail.dimmed());
            }
        }
    }

    if !report.performance.is_empty() {
        println!();
        println!("{}", "── Performance ──".bold());
        println!(
            "  {:<28} {:>8} {:>8} {:>8}  {}",
            "Benchmark".bold(),
            "Min".bold(),
            "Avg".bold(),
            "Max".bold(),
            "Status".bold()
        );
        println!("  {}", "─".repeat(72));
        for perf in &report.performance {
            let status_str = match &perf.status {
                CheckStatus::Pass => "PASS".green().bold().to_string(),
                CheckStatus::Warn => "WARN".yellow().bold().to_string(),
                CheckStatus::Fail => "FAIL".red().bold().to_string(),
            };
            println!(
                "  {:<28} {:>7}ms {:>7}ms {:>7}ms  {}",
                perf.name, perf.min_ms, perf.avg_ms, perf.max_ms, status_str
            );
        }
    }

    println!();
    if report.issues_found == 0 {
        println!(
            "  {} All checks passed — your environment looks healthy!",
            "✓".green().bold()
        );
    } else {
        println!(
            "  {} {} issue(s) found",
            "!".yellow().bold(),
            report.issues_found
        );
        println!();
        println!("{}", "── Recommendations ──".bold());
        for (i, rec) in report.recommendations.iter().enumerate() {
            println!("  {}. {}", i + 1, rec);
        }
    }
    println!();
}

// ── Public entry points ────────────────────────────────────────────────────────

pub async fn run_diagnostic(args: DiagnosticArgs<'_>) -> Result<()> {
    let report = build_report(&args).await;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report, args.detailed);
    }

    if let Some(path) = args.export {
        let json = serde_json::to_string_pretty(&report)?;
        fs::write(path, &json)?;
        println!("{} Diagnostic report exported to {}", "✓".green(), path);
    }

    Ok(())
}

pub async fn generate_report(args: DiagnosticArgs<'_>) -> Result<()> {
    run_diagnostic(args).await
}

pub async fn export_diagnostic(output: &str, detailed: bool, api_url: &str) -> Result<()> {
    let args = DiagnosticArgs {
        api_url,
        detailed,
        export: None,
        json: false,
    };
    let report = build_report(&args).await;
    let json = serde_json::to_string_pretty(&report)?;
    fs::write(output, &json)?;
    println!(
        "{} Diagnostic data exported to {}",
        "✓".green().bold(),
        output
    );
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_status_serializes_correctly() {
        let pass = serde_json::to_string(&CheckStatus::Pass).unwrap();
        let warn = serde_json::to_string(&CheckStatus::Warn).unwrap();
        let fail = serde_json::to_string(&CheckStatus::Fail).unwrap();
        assert_eq!(pass, "\"pass\"");
        assert_eq!(warn, "\"warn\"");
        assert_eq!(fail, "\"fail\"");

        let roundtrip: CheckStatus = serde_json::from_str("\"pass\"").unwrap();
        assert_eq!(roundtrip, CheckStatus::Pass);
    }

    #[test]
    fn diagnostic_report_has_all_fields() {
        let report = DiagnosticReport {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            cli_version: "0.1.0".to_string(),
            api_url: "http://localhost:3001".to_string(),
            system: SystemInfo {
                os_name: "Linux".to_string(),
                os_version: "5.15".to_string(),
                cli_version: "0.1.0".to_string(),
                config_dir_bytes: Some(1024),
            },
            checks: vec![CheckResult {
                name: "Test Check".to_string(),
                status: CheckStatus::Pass,
                message: "All good".to_string(),
                detail: None,
                recommendation: None,
                duration_ms: Some(10),
            }],
            performance: vec![],
            issues_found: 0,
            recommendations: vec![],
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("timestamp"));
        assert!(json.contains("cli_version"));
        assert!(json.contains("api_url"));
        assert!(json.contains("system"));
        assert!(json.contains("checks"));
        assert!(json.contains("performance"));
        assert!(json.contains("issues_found"));
        assert!(json.contains("recommendations"));
    }

    #[test]
    fn check_config_validity_fails_on_bad_url() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(check_config_validity("ftp://invalid"));
        assert_eq!(result.status, CheckStatus::Fail);
        assert!(result.message.contains("Invalid API URL"));
    }

    #[tokio::test]
    async fn check_api_connectivity_fails_on_unreachable_url() {
        let result = check_api_connectivity("http://127.0.0.1:19999").await;
        assert_eq!(result.status, CheckStatus::Fail);
    }
}
