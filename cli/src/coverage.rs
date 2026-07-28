#![allow(dead_code)]

use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

pub async fn run(contract_path: &str, _tests: &str, threshold: f64, output: &str) -> Result<()> {
    println!("\n{}", "Running Code Coverage Analysis...".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    ensure_tarpaulin_installed()?;

    let path = Path::new(contract_path);
    if !path.exists() {
        anyhow::bail!("Contract path does not exist: {}", contract_path);
    }

    let out_dir = Path::new(contract_path).join(output);
    if !out_dir.exists() {
        fs::create_dir_all(&out_dir).context("Failed to create output directory")?;
    }

    // We output paths relative to out_dir
    let out_dir_abs = fs::canonicalize(&out_dir).unwrap_or(out_dir.clone());
    let out_dir_str = out_dir_abs.to_string_lossy().to_string();

    let mut cmd = Command::new("cargo");
    cmd.current_dir(path);
    cmd.args([
        "tarpaulin",
        "--out",
        "Html",
        "--out",
        "Json",
        "--output-dir",
        &out_dir_str,
        "--branch",
    ]);

    if threshold > 0.0 {
        cmd.arg("--fail-under");
        cmd.arg(threshold.to_string());
    }

    let covignore_path = path.join(".covignore");
    if covignore_path.exists() {
        if let Ok(content) = fs::read_to_string(&covignore_path) {
            let excludes: Vec<String> = content
                .lines()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && !s.starts_with('#'))
                .map(String::from)
                .collect();

            if !excludes.is_empty() {
                cmd.arg("--exclude-files");
                for ext in excludes {
                    cmd.arg(&ext);
                }
            }
        }
    }

    println!("{} Executing cargo tarpaulin...", "→".bright_black());

    let status = cmd.status().context("Failed to execute cargo tarpaulin")?;

    let report_json = out_dir_abs.join("tarpaulin-report.json");
    if report_json.exists() {
        let json_content = fs::read_to_string(&report_json)?;
        if let Ok(report) = serde_json::from_str::<Value>(&json_content) {
            parse_and_print_report(&report);
        }
    } else {
        println!("{}", "⚠ Warning: tarpaulin-report.json not found".yellow());
    }

    println!(
        "\n{} HTML report generated at: {}",
        "✓".green(),
        out_dir_abs.join("tarpaulin-report.html").display()
    );

    if !status.success() {
        println!(
            "\n{}",
            "Coverage checks failed or threshold not met.".red().bold()
        );
        std::process::exit(1);
    } else {
        println!(
            "\n{}",
            "Coverage checks passed successfully!".green().bold()
        );
    }

    Ok(())
}

fn ensure_tarpaulin_installed() -> Result<()> {
    let check = Command::new("cargo")
        .args(["tarpaulin", "--version"])
        .output();

    if check.is_err() || !check.unwrap().status.success() {
        println!(
            "{}",
            "cargo-tarpaulin not found. Installing... (this may take a while)"
                .yellow()
                .bold()
        );
        let install = Command::new("cargo")
            .args(["install", "cargo-tarpaulin"])
            .status()
            .context("Failed to run cargo install cargo-tarpaulin")?;

        if !install.success() {
            anyhow::bail!(
                "Installation failed. Please install manually: cargo install cargo-tarpaulin"
            );
        }
        println!("{}", "✓ cargo-tarpaulin installed successfully.".green());
    }

    Ok(())
}

fn parse_and_print_report(report: &Value) {
    // Tarpaulin JSON format varies, but usually it lists coverage at the root or under 'files'.
    // If we can't easily parse perfectly, we can at least show trend or summaries if tarpaulin provides them.
    // Tarpaulin provides total coverage if we look deeply, or we can just let tarpaulin's cli output guide the user
    // However, for "trend" track coverage over versions as per spec.

    // Simplistic read of coverage % from tarpaulin's json if we wanted to process lines.
    // Often tarpaulin prints the summary to stdout anyway. We'll leave stdout visible during run.

    // We can save a small trend file
    let mut covered_lines = 0;
    let mut coverable_lines = 0;

    if let Some(files) = report.get("files").and_then(|f| f.as_array()) {
        for file in files {
            if let Some(traces) = file.get("traces").and_then(|t| t.as_array()) {
                for trace in traces {
                    if let Some(line) = trace.get("line").and_then(|l| l.as_u64()) {
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
    }

    if coverable_lines > 0 {
        let percent = (covered_lines as f64 / coverable_lines as f64) * 100.0;
        println!("\n{}", "Coverage Summary".bold().magenta());
        println!("  Lines covered: {}/{}", covered_lines, coverable_lines);
        println!("  Total line coverage: {:.2}%", percent);

        // Save trend data
        let trend_file = "coverage-trend.json";
        track_trend(trend_file, percent).unwrap_or_else(|e| {
            println!("{} Failed to save trend data: {}", "⚠".yellow(), e);
        });
    }
}

fn track_trend(trend_file: &str, current_percent: f64) -> Result<()> {
    let mut trends: Vec<f64> = Vec::new();

    if Path::new(trend_file).exists() {
        let content = fs::read_to_string(trend_file)?;
        if let Ok(parsed) = serde_json::from_str::<Vec<f64>>(&content) {
            trends = parsed;
        }
    }

    if let Some(last) = trends.last() {
        let diff = current_percent - last;
        if diff > 0.0 {
            println!("  Coverage changed: {:+.2}% 📈", diff.to_string().green());
        } else if diff < 0.0 {
            println!("  Coverage changed: {:+.2}% 📉", diff.to_string().red());
        } else {
            println!("  Coverage unchanged ➖");
        }
    }

    trends.push(current_percent);
    fs::write(trend_file, serde_json::to_string_pretty(&trends)?)?;
    Ok(())
}
