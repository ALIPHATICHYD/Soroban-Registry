//! `soroban-registry contract category` — list and inspect contract categories.
//!
//! Both subcommands read `GET /api/categories`, which already returns each
//! category's description, contract count, and trending/recent tallies, plus a
//! set of recommendations. Requests go through [`crate::cached_http`], so the
//! category list is cached locally with the shared HTTP TTL (default 5 min);
//! `--no-cache` bypasses it. This keeps frequent calls fast without a bespoke
//! cache layer.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::output_format::{self, OutputFormat};

/// A single category as returned by `GET /api/categories`.
///
/// Only the fields the CLI renders are declared; unknown fields are ignored so
/// the command keeps working if the API adds more.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub contract_count: i64,
    #[serde(default)]
    pub new_24h: i64,
    #[serde(default)]
    pub trending: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub name: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CategoriesResponse {
    #[serde(default)]
    categories: Vec<Category>,
    #[serde(default)]
    recommendations: Vec<Recommendation>,
}

/// Fetch categories from the registry, sorted server-side by contract count.
///
/// `network` scopes the counts to a single network. The response is cached by
/// the shared HTTP cache keyed on the full URL + query, so repeated `list` and
/// `stats` calls within the TTL are served locally.
async fn fetch(api_url: &str, network: Option<&str>) -> Result<CategoriesResponse> {
    let url = format!("{}/api/categories", api_url.trim_end_matches('/'));

    // `sort_by=count` gives the most useful default ordering (busiest first);
    // stable ordering also keeps cached pages consistent.
    let mut query: Vec<(&str, String)> = vec![("sort_by", "count".to_string())];
    if let Some(net) = network {
        query.push(("network", net.to_string()));
    }

    let (status, body) = crate::cached_http::cached_get(&url, &query)
        .await
        .context("Failed to reach the registry API for categories")?;

    if !status.is_success() {
        anyhow::bail!("Registry returned {status} when listing categories: {body}");
    }

    serde_json::from_str::<CategoriesResponse>(&body)
        .context("Failed to parse categories response from the registry")
}

/// Build a JSON array of category objects for machine-readable rendering/export.
fn categories_to_json(categories: &[Category]) -> serde_json::Value {
    serde_json::Value::Array(
        categories
            .iter()
            .map(|c| {
                json!({
                    "name": c.name,
                    "slug": c.slug,
                    "description": c.description.clone().unwrap_or_default(),
                    "contract_count": c.contract_count,
                    "new_24h": c.new_24h,
                    "trending": c.trending,
                    "is_default": c.is_default,
                })
            })
            .collect(),
    )
}

/// Render categories to the requested stdout format.
fn render(categories: &[Category], format: OutputFormat, stats: bool) -> Result<String> {
    let value = categories_to_json(categories);
    match format {
        OutputFormat::Json => output_format::render_json(&value),
        OutputFormat::Yaml => output_format::render_yaml(&value),
        OutputFormat::Csv => output_format::render_csv(&value),
        OutputFormat::Table => Ok(render_table(categories, stats)),
    }
}

/// Human-readable table. `stats` mode adds the recent/trending columns.
fn render_table(categories: &[Category], stats: bool) -> String {
    if categories.is_empty() {
        return "No categories found.".to_string();
    }

    let name_w = categories
        .iter()
        .map(|c| c.name.len())
        .max()
        .unwrap_or(8)
        .max(8);

    let mut out = String::new();
    if stats {
        out.push_str(&format!(
            "{:<width$}  {:>9}  {:>7}  {:>8}\n",
            "CATEGORY",
            "CONTRACTS",
            "NEW 24H",
            "TRENDING",
            width = name_w
        ));
        out.push_str(&format!("{}\n", "-".repeat(name_w + 30)));
        for c in categories {
            out.push_str(&format!(
                "{:<width$}  {:>9}  {:>7}  {:>8}\n",
                c.name,
                c.contract_count,
                c.new_24h,
                c.trending,
                width = name_w
            ));
        }
    } else {
        out.push_str(&format!(
            "{:<width$}  {:>9}  DESCRIPTION\n",
            "CATEGORY",
            "CONTRACTS",
            width = name_w
        ));
        out.push_str(&format!("{}\n", "-".repeat(name_w + 40)));
        for c in categories {
            let desc = c.description.clone().unwrap_or_else(|| "—".to_string());
            out.push_str(&format!(
                "{:<width$}  {:>9}  {}\n",
                c.name,
                c.contract_count,
                desc,
                width = name_w
            ));
        }
    }
    out
}

/// Write categories to `categories.<ext>` in the requested export format.
/// Returns the path written. Only `csv` and `json` are valid exports.
fn export_file(categories: &[Category], export: &str) -> Result<String> {
    let format = match export.to_lowercase().as_str() {
        "csv" => OutputFormat::Csv,
        "json" => OutputFormat::Json,
        other => anyhow::bail!("Invalid --export format '{other}'. Use 'csv' or 'json'."),
    };

    let value = categories_to_json(categories);
    let contents = match format {
        OutputFormat::Csv => output_format::render_csv(&value)?,
        OutputFormat::Json => output_format::render_json(&value)?,
        _ => unreachable!("export format is csv or json"),
    };

    let path = format!("categories.{}", export.to_lowercase());
    std::fs::write(&path, contents).with_context(|| format!("Failed to write {path}"))?;
    Ok(path)
}

/// `contract category list` — all categories with descriptions and counts.
pub async fn list(
    api_url: &str,
    network: Option<&str>,
    format: OutputFormat,
    export: Option<&str>,
) -> Result<()> {
    let response = fetch(api_url, network).await?;

    if matches!(format, OutputFormat::Table) {
        println!("\n{}", "Contract Categories".bold().cyan());
        if let Some(net) = network {
            println!("{} {}", "Network:".bold(), net.bright_blue());
        }
        println!();
    }

    println!("{}", render(&response.categories, format, false)?);

    if let Some(export) = export {
        let path = export_file(&response.categories, export)?;
        println!("{} {}", "✓ Exported to".green(), path.bright_black());
    }

    Ok(())
}

/// `contract category stats` — detailed per-category statistics.
pub async fn stats(
    api_url: &str,
    network: Option<&str>,
    format: OutputFormat,
    export: Option<&str>,
) -> Result<()> {
    let response = fetch(api_url, network).await?;

    // Machine-readable formats emit the raw rows only (scriptable, stable).
    if !matches!(format, OutputFormat::Table) {
        println!("{}", render(&response.categories, format, true)?);
        if let Some(export) = export {
            let path = export_file(&response.categories, export)?;
            eprintln!("{} {}", "✓ Exported to".green(), path.bright_black());
        }
        return Ok(());
    }

    println!("\n{}", "Category Statistics".bold().cyan());
    if let Some(net) = network {
        println!("{} {}", "Network:".bold(), net.bright_blue());
    }

    let total: i64 = response.categories.iter().map(|c| c.contract_count).sum();
    println!(
        "{} {} categories, {} contracts total\n",
        "Summary:".bold(),
        response.categories.len(),
        total
    );

    println!("{}", render_table(&response.categories, true));

    if !response.recommendations.is_empty() {
        println!("\n{}", "Recommended:".bold());
        for rec in &response.recommendations {
            println!("  • {} ({})", rec.name.bright_blue(), rec.reason);
        }
    }

    if let Some(export) = export {
        let path = export_file(&response.categories, export)?;
        println!("\n{} {}", "✓ Exported to".green(), path.bright_black());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Category> {
        vec![
            Category {
                name: "DeFi".into(),
                slug: "defi".into(),
                description: Some("Decentralized finance".into()),
                is_default: true,
                contract_count: 12,
                new_24h: 2,
                trending: 5,
            },
            Category {
                name: "NFT".into(),
                slug: "nft".into(),
                description: None,
                is_default: false,
                contract_count: 3,
                new_24h: 0,
                trending: 1,
            },
        ]
    }

    #[test]
    fn json_export_is_valid_and_roundtrips() {
        let value = categories_to_json(&sample());
        let rendered = output_format::render_json(&value).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 2);
        assert_eq!(parsed[0]["name"], "DeFi");
        assert_eq!(parsed[0]["contract_count"], 12);
    }

    #[test]
    fn csv_export_has_header_and_rows() {
        let value = categories_to_json(&sample());
        let csv = output_format::render_csv(&value).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        // header + 2 data rows
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("name"));
        assert!(lines[0].contains("contract_count"));
        assert!(csv.contains("DeFi"));
        assert!(csv.contains("NFT"));
    }

    #[test]
    fn export_rejects_unknown_format() {
        let err = export_file(&sample(), "xml").unwrap_err();
        assert!(err.to_string().contains("Invalid --export format"));
    }

    #[test]
    fn table_list_shows_description_and_missing_placeholder() {
        let table = render_table(&sample(), false);
        assert!(table.contains("Decentralized finance"));
        assert!(table.contains("—")); // NFT has no description
        assert!(table.contains("CONTRACTS"));
    }

    #[test]
    fn stats_table_has_trending_columns() {
        let table = render_table(&sample(), true);
        assert!(table.contains("TRENDING"));
        assert!(table.contains("NEW 24H"));
    }

    #[test]
    fn empty_categories_render_cleanly() {
        assert_eq!(render_table(&[], false), "No categories found.");
        let json = render(&[], OutputFormat::Json, false).unwrap();
        assert_eq!(json.trim(), "[]");
    }
}
