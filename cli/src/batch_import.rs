use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::import;

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub file: String,
    pub success: bool,
    pub imported_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchImportSummary {
    pub input_dir: String,
    pub total_files: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub dry_run: bool,
    pub results: Vec<ImportResult>,
}

pub async fn run_batch_import(
    api_url: &str,
    input_dir: &str,
    format: Option<&str>,
    on_duplicate: &str,
    dry_run: bool,
    atomic: bool,
    output_dir: &str,
    json_out: bool,
) -> Result<()> {
    let input_path = Path::new(input_dir);
    anyhow::ensure!(
        input_path.is_dir(),
        "Input directory not found: {}",
        input_dir
    );

    validate_on_duplicate_strategy(on_duplicate)?;

    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir))?;

    if !json_out {
        println!("\n{}", "Batch Import".bold().cyan());
        println!("{}", "=".repeat(80).cyan());
        println!("Input directory: {}", input_dir.bright_black());
        println!("Output directory: {}", output_dir.bright_black());
        println!("On duplicate: {}", on_duplicate);
        if dry_run {
            println!("Mode: {} (preview only)", "dry-run".yellow());
        }
        if atomic {
            println!("Mode: {} (stop on first error)", "atomic".yellow());
        }
        println!("{}", "-".repeat(80).cyan());
    }

    let files = collect_import_files(input_dir)?;

    if files.is_empty() {
        println!(
            "{}",
            "No import files found (supported: .json, .csv, .tar.gz)".yellow()
        );
        return Ok(());
    }

    let mut results = Vec::new();

    for (idx, file_path) in files.iter().enumerate() {
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        if !json_out {
            print!("  [{}/{}] {} ... ", idx + 1, files.len(), file_name.bold());
        }

        let opts = import::ImportOptions {
            api_url,
            file_path,
            format,
            network_flag: None,
            output_dir,
            validate: true,
            dry_run,
            on_duplicate: import::OnDuplicate::parse(on_duplicate)?,
            network_map: std::collections::HashMap::new(),
            atomic,
            report_output: None,
        };
        match import::run(opts).await {
            Ok(_) => {
                results.push(ImportResult {
                    file: file_name.to_string(),
                    success: true,
                    imported_count: 1,
                    error: None,
                });
                if !json_out {
                    println!("{}", "✓".green());
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                let is_duplicate = error_msg.contains("409") || error_msg.contains("duplicate");

                if is_duplicate && on_duplicate == "skip" {
                    results.push(ImportResult {
                        file: file_name.to_string(),
                        success: true,
                        imported_count: 0,
                        error: Some("skipped (duplicate)".to_string()),
                    });
                    if !json_out {
                        println!("{}", "⊘ (skipped)".yellow());
                    }
                } else {
                    results.push(ImportResult {
                        file: file_name.to_string(),
                        success: false,
                        imported_count: 0,
                        error: Some(error_msg.clone()),
                    });
                    if !json_out {
                        println!("{} — {}", "✗".red(), error_msg.red());
                    }

                    if atomic {
                        let summary = BatchImportSummary {
                            input_dir: input_dir.to_string(),
                            total_files: files.len(),
                            succeeded: results.iter().filter(|r| r.success).count(),
                            failed: results.iter().filter(|r| !r.success).count(),
                            dry_run,
                            results,
                        };
                        emit_summary(&summary, json_out)?;
                        anyhow::bail!("Import stopped due to --atomic flag");
                    }
                }
            }
        }
    }

    let summary = BatchImportSummary {
        input_dir: input_dir.to_string(),
        total_files: files.len(),
        succeeded: results.iter().filter(|r| r.success).count(),
        failed: results.iter().filter(|r| !r.success).count(),
        dry_run,
        results,
    };

    emit_summary(&summary, json_out)?;

    Ok(())
}

fn collect_import_files(input_dir: &str) -> Result<Vec<String>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(input_dir)
        .with_context(|| format!("Failed to read directory: {}", input_dir))?
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

            if matches!(ext, "json" | "csv")
                || path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with(".tar.gz"))
                    .unwrap_or(false)
            {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }

    files.sort();
    Ok(files)
}

fn validate_on_duplicate_strategy(strategy: &str) -> Result<()> {
    anyhow::ensure!(
        matches!(strategy, "skip" | "fail"),
        "Invalid on-duplicate strategy: {}. Must be skip or fail",
        strategy
    );
    Ok(())
}

fn emit_summary(summary: &BatchImportSummary, json_out: bool) -> Result<()> {
    if json_out {
        println!("{}", serde_json::to_string_pretty(summary)?);
        return Ok(());
    }

    println!("\n{}", "Import Summary".bold().cyan());
    let mode = if summary.dry_run {
        "(dry-run)".yellow().to_string()
    } else {
        "".to_string()
    };
    println!(
        "Total: {} | Succeeded: {} | Failed: {} {}",
        summary.total_files,
        summary.succeeded.to_string().green(),
        summary.failed.to_string().red(),
        mode
    );
    println!("{}", "-".repeat(80).cyan());

    for result in &summary.results {
        if result.success {
            let status = if let Some(err) = &result.error {
                format!("⊘ {}", err.bright_black())
            } else {
                "✓".green().to_string()
            };
            println!("{} {}", status, result.file.bold());
        } else {
            println!(
                "{} {} — {}",
                "✗".red(),
                result.file.bold(),
                result.error.as_ref().unwrap().red()
            );
        }
    }

    Ok(())
}
