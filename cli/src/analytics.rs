use crate::output_format::{self, OutputFormat};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;

#[derive(Debug, Clone, Copy)]
pub enum AnalyticsQuery {
    TopContracts,
    Trending,
    ByCategory,
    ByNetwork,
}

impl AnalyticsQuery {
    pub fn parse(input: &str) -> Result<Self> {
        match input {
            "top-contracts" => Ok(Self::TopContracts),
            "trending" => Ok(Self::Trending),
            "by-category" => Ok(Self::ByCategory),
            "by-network" => Ok(Self::ByNetwork),
            _ => anyhow::bail!(
                "Invalid analytics query '{}'. Allowed values: top-contracts, trending, by-category, by-network",
                input
            ),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AnalyticsRow {
    pub key: String,
    pub value: f64,
    pub secondary: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsReport {
    pub query: String,
    pub period: String,
    pub generated_at: String,
    pub rows: Vec<AnalyticsRow>,
}

pub async fn run(
    api_url: &str,
    query: AnalyticsQuery,
    period: &str,
    format: &str,
    sort: Option<&str>,
    export: Option<&str>,
) -> Result<()> {
    let contracts = fetch_contracts(api_url).await?;
    let _period_start = resolve_period_start(period)?;

    let mut rows = match query {
        AnalyticsQuery::TopContracts => top_contracts(&contracts),
        AnalyticsQuery::Trending => trending_contracts(&contracts),
        AnalyticsQuery::ByCategory => aggregate_by_field(&contracts, "category"),
        AnalyticsQuery::ByNetwork => aggregate_by_field(&contracts, "network"),
    };

    apply_sort(&mut rows, sort);

    let report = AnalyticsReport {
        query: query_name(query).to_string(),
        period: period.to_string(),
        generated_at: Utc::now().to_rfc3339(),
        rows,
    };

    emit_report(&report, format, export)?;
    Ok(())
}

async fn fetch_contracts(api_url: &str) -> Result<Vec<Value>> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/api/contracts?limit=500&offset=0", api_url))
        .send()
        .await
        .context("Failed to fetch contracts for analytics")?;
    let body: Value = response
        .json()
        .await
        .context("Invalid contracts analytics response")?;
    let items = body["items"].as_array().cloned().unwrap_or_default();
    Ok(items)
}

fn top_contracts(contracts: &[Value]) -> Vec<AnalyticsRow> {
    let mut rows: Vec<AnalyticsRow> = contracts
        .iter()
        .map(|item| AnalyticsRow {
            key: item["name"].as_str().unwrap_or("Unnamed").to_string(),
            value: score_from_contract(item),
            secondary: item["contract_id"].as_str().map(|s| s.to_string()),
        })
        .collect();
    rows.sort_by(|a, b| b.value.total_cmp(&a.value));
    rows.truncate(20);
    rows
}

fn trending_contracts(contracts: &[Value]) -> Vec<AnalyticsRow> {
    let now = Utc::now();
    let mut rows: Vec<AnalyticsRow> = contracts
        .iter()
        .map(|item| {
            let created_at = parse_datetime(item["created_at"].as_str()).unwrap_or(now);
            let age_hours = (now - created_at).num_hours().max(1) as f64;
            let velocity = score_from_contract(item) / age_hours;
            AnalyticsRow {
                key: item["name"].as_str().unwrap_or("Unnamed").to_string(),
                value: velocity,
                secondary: item["network"].as_str().map(|s| s.to_string()),
            }
        })
        .collect();
    rows.sort_by(|a, b| b.value.total_cmp(&a.value));
    rows.truncate(20);
    rows
}

fn aggregate_by_field(contracts: &[Value], field: &str) -> Vec<AnalyticsRow> {
    let mut buckets: BTreeMap<String, f64> = BTreeMap::new();
    for item in contracts {
        let key = item[field].as_str().unwrap_or("unknown").to_string();
        *buckets.entry(key).or_insert(0.0) += 1.0;
    }
    let mut rows: Vec<AnalyticsRow> = buckets
        .into_iter()
        .map(|(key, value)| AnalyticsRow {
            key,
            value,
            secondary: None,
        })
        .collect();
    rows.sort_by(|a, b| b.value.total_cmp(&a.value));
    rows
}

fn score_from_contract(contract: &Value) -> f64 {
    contract["interaction_count"]
        .as_f64()
        .or_else(|| contract["usage_count"].as_f64())
        .or_else(|| contract["download_count"].as_f64())
        .or_else(|| contract["health_score"].as_f64())
        .unwrap_or(0.0)
}

fn resolve_period_start(period: &str) -> Result<DateTime<Utc>> {
    let now = Utc::now();
    if period == "7d" {
        return Ok(now - Duration::days(7));
    }
    if period == "30d" {
        return Ok(now - Duration::days(30));
    }
    if period == "90d" {
        return Ok(now - Duration::days(90));
    }
    if let Some((start, _end)) = period.split_once("..") {
        let parsed = DateTime::parse_from_rfc3339(start)
            .with_context(|| "Custom period must be RFC3339 range start..end")?
            .with_timezone(&Utc);
        return Ok(parsed);
    }
    anyhow::bail!(
        "Invalid period '{}'. Use 7d, 30d, 90d, or start..end",
        period
    )
}

fn parse_datetime(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

fn apply_sort(rows: &mut [AnalyticsRow], sort: Option<&str>) {
    match sort.unwrap_or("value_desc") {
        "key_asc" => rows.sort_by(|a, b| a.key.cmp(&b.key)),
        "key_desc" => rows.sort_by(|a, b| b.key.cmp(&a.key)),
        "value_asc" => rows.sort_by(|a, b| a.value.total_cmp(&b.value)),
        _ => rows.sort_by(|a, b| b.value.total_cmp(&a.value)),
    }
}

fn emit_report(report: &AnalyticsReport, format: &str, export: Option<&str>) -> Result<()> {
    let rendered = match format {
        "json" => serde_json::to_string_pretty(report)?,
        "csv" => to_csv(report),
        "yaml" | "yml" => {
            let json_val = serde_json::to_value(report)?;
            output_format::render_yaml(&json_val)?
        }
        "table" => to_table(report),
        _ => anyhow::bail!(
            "Invalid format '{}'. Allowed: table, json, csv, yaml",
            format
        ),
    };

    println!("{}", rendered);
    if let Some(path) = export {
        fs::write(path, rendered)
            .with_context(|| format!("Failed to write export file '{}'", path))?;
        println!("Exported analytics to {}", path);
    }
    Ok(())
}

fn to_csv(report: &AnalyticsReport) -> String {
    let mut out = String::from("key,value,secondary\n");
    for row in &report.rows {
        out.push_str(&format!(
            "{},{},{}\n",
            row.key.replace(',', " "),
            row.value,
            row.secondary.clone().unwrap_or_default().replace(',', " ")
        ));
    }
    out
}

fn to_table(report: &AnalyticsReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "Analytics: {} (period: {})",
        report.query, report.period
    ));
    lines.push("=".repeat(72));
    lines.push(format!("{:<40} {:>12} {:<16}", "Key", "Value", "Secondary"));
    lines.push("-".repeat(72));
    for row in &report.rows {
        lines.push(format!(
            "{:<40} {:>12.3} {:<16}",
            row.key,
            row.value,
            row.secondary.clone().unwrap_or_default()
        ));
    }
    lines.join("\n")
}

fn query_name(query: AnalyticsQuery) -> &'static str {
    match query {
        AnalyticsQuery::TopContracts => "top-contracts",
        AnalyticsQuery::Trending => "trending",
        AnalyticsQuery::ByCategory => "by-category",
        AnalyticsQuery::ByNetwork => "by-network",
    }
}
