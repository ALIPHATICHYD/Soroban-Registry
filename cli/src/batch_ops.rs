use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::json;
use std::fs;

#[derive(Debug, Clone, Copy)]
pub enum BatchOperation {
    Tag,
    Categorize,
    Verify,
    Deprecate,
}

impl BatchOperation {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "tag" => Ok(Self::Tag),
            "categorize" => Ok(Self::Categorize),
            "verify" => Ok(Self::Verify),
            "deprecate" => Ok(Self::Deprecate),
            _ => anyhow::bail!(
                "Invalid batch operation '{}'. Allowed: tag, categorize, verify, deprecate",
                raw
            ),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct BatchResult {
    pub contract_id: String,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct BatchSummary {
    pub operation: String,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub rolled_back: bool,
    pub results: Vec<BatchResult>,
}

pub async fn run(
    api_url: &str,
    operation: BatchOperation,
    contracts: Vec<String>,
    file: Option<&str>,
    value: Option<&str>,
    rollback_on_error: bool,
    json_out: bool,
) -> Result<()> {
    let mut ids = contracts;
    if let Some(file_path) = file {
        let file_ids = read_ids_from_file(file_path)?;
        ids.extend(file_ids);
    }
    ids.sort();
    ids.dedup();

    if ids.is_empty() {
        anyhow::bail!("No contracts provided. Pass IDs or --file contracts.txt");
    }

    if matches!(operation, BatchOperation::Tag | BatchOperation::Categorize) && value.is_none() {
        anyhow::bail!("Operation requires --value (tag/category value)");
    }

    let client = reqwest::Client::new();
    let mut results = Vec::new();
    let mut applied_ids = Vec::new();
    let total = ids.len();

    for (idx, id) in ids.iter().enumerate() {
        println!("[{}/{}] {}", idx + 1, total, id);
        match apply_operation(&client, api_url, operation, id, value).await {
            Ok(msg) => {
                applied_ids.push(id.clone());
                results.push(BatchResult {
                    contract_id: id.clone(),
                    success: true,
                    message: msg,
                });
            }
            Err(err) => {
                results.push(BatchResult {
                    contract_id: id.clone(),
                    success: false,
                    message: err.to_string(),
                });
                if rollback_on_error {
                    rollback(&client, api_url, operation, &applied_ids).await?;
                    let summary = build_summary(operation, results, true);
                    emit_summary(&summary, json_out)?;
                    anyhow::bail!("Batch operation rolled back due to error.");
                }
            }
        }
    }

    let summary = build_summary(operation, results, false);
    emit_summary(&summary, json_out)?;
    Ok(())
}

async fn apply_operation(
    client: &reqwest::Client,
    api_url: &str,
    operation: BatchOperation,
    contract_id: &str,
    value: Option<&str>,
) -> Result<String> {
    match operation {
        BatchOperation::Verify => {
            let resp = client
                .post(format!("{}/api/contracts/verify", api_url))
                .json(&json!({ "contract_id": contract_id }))
                .send()
                .await
                .context("verify request failed")?;
            ensure_success(resp).await?;
            Ok("verified".to_string())
        }
        BatchOperation::Tag => {
            let resp = client
                .patch(format!("{}/api/contracts/{}", api_url, contract_id))
                .json(&json!({ "tags": [value.unwrap_or_default()] }))
                .send()
                .await
                .context("tag request failed")?;
            ensure_success(resp).await?;
            Ok(format!("tagged: {}", value.unwrap_or_default()))
        }
        BatchOperation::Categorize => {
            let resp = client
                .patch(format!("{}/api/contracts/{}", api_url, contract_id))
                .json(&json!({ "category": value.unwrap_or_default() }))
                .send()
                .await
                .context("categorize request failed")?;
            ensure_success(resp).await?;
            Ok(format!("categorized: {}", value.unwrap_or_default()))
        }
        BatchOperation::Deprecate => {
            let resp = client
                .patch(format!("{}/api/contracts/{}", api_url, contract_id))
                .json(&json!({ "deprecated": true }))
                .send()
                .await
                .context("deprecate request failed")?;
            ensure_success(resp).await?;
            Ok("deprecated".to_string())
        }
    }
}

async fn rollback(
    client: &reqwest::Client,
    api_url: &str,
    operation: BatchOperation,
    applied_ids: &[String],
) -> Result<()> {
    for id in applied_ids {
        match operation {
            BatchOperation::Deprecate => {
                let _ = client
                    .patch(format!("{}/api/contracts/{}", api_url, id))
                    .json(&json!({ "deprecated": false }))
                    .send()
                    .await;
            }
            BatchOperation::Tag => {
                let _ = client
                    .patch(format!("{}/api/contracts/{}", api_url, id))
                    .json(&json!({ "tags": [] }))
                    .send()
                    .await;
            }
            BatchOperation::Categorize => {
                let _ = client
                    .patch(format!("{}/api/contracts/{}", api_url, id))
                    .json(&json!({ "category": null }))
                    .send()
                    .await;
            }
            BatchOperation::Verify => {}
        }
    }
    Ok(())
}

fn read_ids_from_file(path: &str) -> Result<Vec<String>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read contracts file '{}'", path))?;
    let ids = raw
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    Ok(ids)
}

async fn ensure_success(resp: reqwest::Response) -> Result<()> {
    if resp.status().is_success() {
        return Ok(());
    }
    let code = resp.status();
    let body = resp.text().await.unwrap_or_default();
    anyhow::bail!("HTTP {}: {}", code, body);
}

fn build_summary(
    operation: BatchOperation,
    results: Vec<BatchResult>,
    rolled_back: bool,
) -> BatchSummary {
    let total = results.len();
    let succeeded = results.iter().filter(|r| r.success).count();
    let failed = total.saturating_sub(succeeded);
    BatchSummary {
        operation: match operation {
            BatchOperation::Tag => "tag",
            BatchOperation::Categorize => "categorize",
            BatchOperation::Verify => "verify",
            BatchOperation::Deprecate => "deprecate",
        }
        .to_string(),
        total,
        succeeded,
        failed,
        rolled_back,
        results,
    }
}

fn emit_summary(summary: &BatchSummary, json_out: bool) -> Result<()> {
    if json_out {
        println!("{}", serde_json::to_string_pretty(summary)?);
        return Ok(());
    }
    println!("\nBatch operation: {}", summary.operation);
    println!("Total: {}", summary.total);
    println!("Succeeded: {}", summary.succeeded);
    println!("Failed: {}", summary.failed);
    println!("Rolled back: {}", summary.rolled_back);
    println!("{}", "-".repeat(60));
    for result in &summary.results {
        println!(
            "{} {}: {}",
            if result.success { "OK" } else { "ERR" },
            result.contract_id,
            result.message
        );
    }
    Ok(())
}
