use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::export::{self, RegistryExportFormat, RegistryExportOptions};

#[derive(Debug, Serialize)]
pub struct ExportResult {
    pub contract_id: String,
    pub output_path: String,
    pub sha256: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchExportSummary {
    pub output_dir: String,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub manifest_path: String,
    pub results: Vec<ExportResult>,
}

pub async fn run_batch_export(
    api_url: &str,
    output_dir: &str,
    filter: Option<&str>,
    format: &str,
    organize: bool,
    compress: bool,
    json_out: bool,
) -> Result<()> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory: {}", output_dir))?;

    let export_format = RegistryExportFormat::parse(format)?;

    if !json_out {
        println!("\n{}", "Batch Export".bold().cyan());
        println!("{}", "=".repeat(80).cyan());
        println!("Output directory: {}", output_dir.bright_black());
        println!("Format: {}", format);
        if organize {
            println!(
                "Organization: {} (by network/category)",
                "enabled".bright_blue()
            );
        }
        if compress {
            println!("Compression: {}", "enabled".bright_blue());
        }
        println!("{}", "-".repeat(80).cyan());
    }

    let client = reqwest::Client::new();

    let contracts = fetch_all_contracts(&client, api_url, filter).await?;

    if contracts.is_empty() {
        println!("{}", "No contracts found matching filter".yellow());
        return Ok(());
    }

    let mut results = Vec::new();

    for (idx, contract) in contracts.iter().enumerate() {
        let contract_id = contract
            .get("id")
            .and_then(Value::as_str)
            .or_else(|| contract.get("contract_id").and_then(Value::as_str))
            .unwrap_or("unknown");

        let network = contract
            .get("network")
            .and_then(Value::as_str)
            .unwrap_or("unknown");

        if !json_out {
            print!(
                "  [{}/{}] Exporting {} ... ",
                idx + 1,
                contracts.len(),
                contract_id.bold()
            );
        }

        let output_path = if organize {
            let path = Path::new(output_dir).join(network).join(format!(
                "{}.{}",
                contract_id,
                get_extension(export_format)
            ));
            fs::create_dir_all(path.parent().unwrap())?;
            path.to_string_lossy().to_string()
        } else {
            Path::new(output_dir)
                .join(format!("{}.{}", contract_id, get_extension(export_format)))
                .to_string_lossy()
                .to_string()
        };

        match export::export_registry_data(RegistryExportOptions {
            api_url,
            id: Some(contract_id),
            output: Some(&output_path),
            contract_dir: ".",
            format: export_format,
            filters: vec![],
            page_size: 100,
            compress: false,
            include_related: false,
        })
        .await
        {
            Ok(summary) => {
                results.push(ExportResult {
                    contract_id: contract_id.to_string(),
                    output_path: output_path.clone(),
                    sha256: summary.sha256,
                    success: true,
                    error: None,
                });
                if !json_out {
                    println!("{}", "✓".green());
                }
            }
            Err(e) => {
                results.push(ExportResult {
                    contract_id: contract_id.to_string(),
                    output_path,
                    sha256: String::new(),
                    success: false,
                    error: Some(e.to_string()),
                });
                if !json_out {
                    println!("{} — {}", "✗".red(), e.to_string().red());
                }
            }
        }
    }

    let manifest_path = Path::new(output_dir).join("export-manifest.json");
    let manifest_data = serde_json::json!({
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "total_contracts": results.len(),
        "succeeded": results.iter().filter(|r| r.success).count(),
        "failed": results.iter().filter(|r| !r.success).count(),
        "format": format,
        "results": results.iter().map(|r| serde_json::json!({
            "contract_id": r.contract_id,
            "output_path": r.output_path,
            "sha256": r.sha256,
            "success": r.success,
            "error": r.error
        })).collect::<Vec<_>>()
    });

    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest_data)?,
    )
    .with_context(|| format!("Failed to write manifest to {}", manifest_path.display()))?;

    if compress {
        create_archive(output_dir, &results, json_out)?;
    }

    let summary = BatchExportSummary {
        output_dir: output_dir.to_string(),
        total: results.len(),
        succeeded: results.iter().filter(|r| r.success).count(),
        failed: results.iter().filter(|r| !r.success).count(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
        results,
    };

    emit_summary(&summary, json_out)?;

    Ok(())
}

async fn fetch_all_contracts(
    client: &reqwest::Client,
    api_url: &str,
    filter: Option<&str>,
) -> Result<Vec<Value>> {
    let mut contracts = Vec::new();
    let mut seen = HashSet::new();
    let mut offset = 0;
    let page_size = 50;

    loop {
        let mut url =
            reqwest::Url::parse(&format!("{}/api/contracts", api_url.trim_end_matches('/')))?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("limit", &page_size.to_string());
            query.append_pair("offset", &offset.to_string());
            if let Some(f) = filter {
                for (key, value) in parse_filter(f)? {
                    query.append_pair(&key, &value);
                }
            }
        }

        let response = client.get(url).send().await?;
        let page: Value = response.error_for_status()?.json().await?;

        let items = page_items(&page);
        let has_more = page_has_more(&page).ok_or_else(|| {
            anyhow::anyhow!(
                "batch export response missing pagination continuation flag (expected has_more, hasMore, or pagination.has_more)"
            )
        })?;

        if items.is_empty() && has_more {
            anyhow::bail!(
                "batch export pagination returned no contracts while indicating more records at offset {}",
                offset
            );
        }

        extend_unique_contracts(&mut contracts, &mut seen, items);

        if !has_more {
            break;
        }

        offset = page_next_offset(&page).unwrap_or(offset + page_size);
    }

    Ok(contracts)
}

fn page_items(page: &Value) -> Vec<Value> {
    page.get("items")
        .and_then(Value::as_array)
        .or_else(|| page.get("contracts").and_then(Value::as_array))
        .cloned()
        .unwrap_or_default()
}

fn page_has_more(page: &Value) -> Option<bool> {
    page.get("has_more")
        .and_then(Value::as_bool)
        .or_else(|| page.get("hasMore").and_then(Value::as_bool))
        .or_else(|| {
            page.get("pagination")
                .and_then(|p| p.get("has_more"))
                .and_then(Value::as_bool)
        })
        .or_else(|| {
            page.get("pagination")
                .and_then(|p| p.get("hasMore"))
                .and_then(Value::as_bool)
        })
}

fn page_next_offset(page: &Value) -> Option<usize> {
    page.get("next_offset")
        .and_then(Value::as_u64)
        .or_else(|| page.get("nextOffset").and_then(Value::as_u64))
        .or_else(|| {
            page.get("pagination")
                .and_then(|p| p.get("next_offset"))
                .and_then(Value::as_u64)
        })
        .or_else(|| {
            page.get("pagination")
                .and_then(|p| p.get("nextOffset"))
                .and_then(Value::as_u64)
        })
        .map(|n| n as usize)
}

fn contract_key(contract: &Value) -> Option<String> {
    contract
        .get("id")
        .and_then(Value::as_str)
        .or_else(|| contract.get("contract_id").and_then(Value::as_str))
        .map(str::to_string)
}

fn extend_unique_contracts(
    contracts: &mut Vec<Value>,
    seen: &mut HashSet<String>,
    items: Vec<Value>,
) {
    for item in items {
        if let Some(key) = contract_key(&item) {
            if !seen.insert(key) {
                continue;
            }
        }
        contracts.push(item);
    }
}

fn parse_filter(filter: &str) -> Result<Vec<(String, String)>> {
    let mut result = Vec::new();
    for part in filter.split('&') {
        if let Some((key, value)) = part.split_once('=') {
            result.push((key.trim().to_string(), value.trim().to_string()));
        }
    }
    Ok(result)
}

fn get_extension(format: RegistryExportFormat) -> &'static str {
    match format {
        RegistryExportFormat::Json => "json",
        RegistryExportFormat::Csv => "csv",
        RegistryExportFormat::Markdown => "md",
        RegistryExportFormat::Archive => "tar.gz",
        RegistryExportFormat::JsonLines => "jsonl",
        RegistryExportFormat::Sqlite => "sqlite",
    }
}

fn create_archive(output_dir: &str, _results: &[ExportResult], json_out: bool) -> Result<()> {
    let archive_path = format!("{}-export.tar.gz", output_dir.trim_end_matches('/'));
    if !json_out {
        println!(
            "Compressing {} into {}...",
            output_dir.bright_black(),
            archive_path.bright_black()
        );
    }

    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::fs::File;
    use tar::Builder;

    let tar_gz = File::create(&archive_path)?;
    let encoder = GzEncoder::new(tar_gz, Compression::default());
    let mut builder = Builder::new(encoder);

    walk_dir_and_add(&mut builder, output_dir, output_dir)?;

    let encoder = builder.into_inner()?;
    encoder.finish()?;

    if !json_out {
        println!(
            "{} Archive created: {}",
            "✓".green(),
            archive_path.bright_black()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_final_page_stops_only_on_backend_signal() {
        let page = serde_json::json!({
            "items": [{"id": "c1"}, {"id": "c2"}],
            "has_more": false
        });

        assert_eq!(page_items(&page).len(), 2);
        assert_eq!(page_has_more(&page), Some(false));
    }

    #[test]
    fn nested_pagination_signal_is_supported() {
        let page = serde_json::json!({
            "contracts": [{"contract_id": "c1"}],
            "pagination": {
                "has_more": true,
                "next_offset": 1
            }
        });

        assert_eq!(page_has_more(&page), Some(true));
        assert_eq!(page_next_offset(&page), Some(1));
    }

    #[test]
    fn duplicate_contracts_are_added_once() {
        let mut contracts = Vec::new();
        let mut seen = HashSet::new();
        extend_unique_contracts(
            &mut contracts,
            &mut seen,
            vec![
                serde_json::json!({"id": "c1"}),
                serde_json::json!({"id": "c1"}),
                serde_json::json!({"contract_id": "c2"}),
            ],
        );

        assert_eq!(contracts.len(), 2);
    }
}

fn walk_dir_and_add<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    base_dir: &str,
    current_dir: &str,
) -> Result<()> {
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let relative = path.strip_prefix(base_dir)?;
            builder.append_path_with_name(&path, relative)?;
        } else if path.is_dir() {
            let dir_str = path.to_string_lossy();
            walk_dir_and_add(builder, base_dir, &dir_str)?;
        }
    }

    Ok(())
}

fn emit_summary(summary: &BatchExportSummary, json_out: bool) -> Result<()> {
    if json_out {
        println!("{}", serde_json::to_string_pretty(summary)?);
        return Ok(());
    }

    println!("\n{}", "Export Summary".bold().cyan());
    println!(
        "Total: {} | Succeeded: {} | Failed: {}",
        summary.total,
        summary.succeeded.to_string().green(),
        summary.failed.to_string().red()
    );
    println!("Manifest: {}", summary.manifest_path.bright_black());
    println!("{}", "-".repeat(80).cyan());

    let mut by_status: HashMap<bool, Vec<_>> = HashMap::new();
    for result in &summary.results {
        by_status
            .entry(result.success)
            .or_insert_with(Vec::new)
            .push(result);
    }

    if let Some(success_results) = by_status.get(&true) {
        for result in success_results.iter().take(3) {
            println!(
                "{} {} → {}",
                "✓".green(),
                result.contract_id.bold(),
                result.output_path.bright_black()
            );
        }
        if success_results.len() > 3 {
            println!(
                "  {} more exported",
                (success_results.len() - 3).to_string().bright_black()
            );
        }
    }

    if let Some(failed_results) = by_status.get(&false) {
        println!();
        for result in failed_results {
            println!(
                "{} {} — {}",
                "✗".red(),
                result.contract_id.bold(),
                result.error.as_ref().unwrap().red()
            );
        }
    }

    Ok(())
}
