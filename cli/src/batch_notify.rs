use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use colored::Colorize;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NotificationRecipient {
    contract_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    recipient: Option<String>,
}

#[derive(Debug, Serialize)]
struct BatchNotifyReport {
    message_type: String,
    channels: Vec<String>,
    preview: bool,
    scheduled_at: Option<String>,
    total: usize,
    accepted: usize,
    failed: usize,
    recipients: Vec<NotificationRecipient>,
    response: Option<Value>,
}

pub async fn run_batch_notify(
    api_url: &str,
    message: &str,
    recipients: &str,
    message_type: &str,
    template: Option<&str>,
    preview: bool,
    schedule: Option<&str>,
    channels: Vec<String>,
    json_out: bool,
) -> Result<()> {
    validate_message_type(message_type)?;
    let channels = normalize_channels(channels)?;
    let scheduled_at = parse_schedule(schedule)?;
    let resolved = resolve_recipients(api_url, recipients).await?;
    anyhow::ensure!(!resolved.is_empty(), "No notification recipients matched");

    let rendered_message = apply_template(template, message)
        .with_context(|| "Failed to apply notification template")?;

    if preview {
        let report = BatchNotifyReport {
            message_type: message_type.to_string(),
            channels,
            preview,
            scheduled_at,
            total: resolved.len(),
            accepted: 0,
            failed: 0,
            recipients: resolved,
            response: None,
        };
        emit_report(&report, json_out)?;
        return Ok(());
    }

    let payload = json!({
        "message": rendered_message,
        "message_type": message_type,
        "channels": channels,
        "scheduled_at": scheduled_at,
        "recipients": resolved,
    });

    let client = reqwest::Client::new();
    let response = client
        .post(format!(
            "{}/api/notifications/batch",
            api_url.trim_end_matches('/')
        ))
        .json(&payload)
        .send()
        .await
        .context("Failed to submit batch notification")?;

    let status = response.status();
    let body: Value = response.json().await.unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        anyhow::bail!("Batch notification failed with HTTP {}: {}", status, body);
    }

    let accepted = body.get("accepted").and_then(Value::as_u64).unwrap_or(0) as usize;
    let failed = body.get("failed").and_then(Value::as_u64).unwrap_or(0) as usize;
    let recipients = body
        .get("recipients")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| serde_json::from_value(item.clone()).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let report = BatchNotifyReport {
        message_type: message_type.to_string(),
        channels,
        preview,
        scheduled_at,
        total: accepted + failed,
        accepted,
        failed,
        recipients,
        response: Some(body),
    };
    emit_report(&report, json_out)?;
    Ok(())
}

fn validate_message_type(message_type: &str) -> Result<()> {
    anyhow::ensure!(
        matches!(
            message_type,
            "info" | "warning" | "critical" | "action-required"
        ),
        "Invalid message type '{}'. Use info, warning, critical, or action-required",
        message_type
    );
    Ok(())
}

fn normalize_channels(channels: Vec<String>) -> Result<Vec<String>> {
    let raw = if channels.is_empty() {
        vec!["in-app".to_string()]
    } else {
        channels
    };

    let mut normalized = BTreeSet::new();
    for channel in raw {
        let channel = channel.trim().to_ascii_lowercase().replace('_', "-");
        let mapped = match channel.as_str() {
            "email" => "email",
            "in-app" | "inapp" => "in-app",
            "webhook" => "webhook",
            other => anyhow::bail!("Invalid channel '{}'. Use email, in-app, or webhook", other),
        };
        normalized.insert(mapped.to_string());
    }
    Ok(normalized.into_iter().collect())
}

fn parse_schedule(schedule: Option<&str>) -> Result<Option<String>> {
    let Some(raw) = schedule else {
        return Ok(None);
    };
    let parsed = DateTime::parse_from_rfc3339(raw)
        .with_context(|| "--schedule must be an RFC3339 datetime, e.g. 2026-06-01T09:00:00Z")?;
    anyhow::ensure!(
        parsed.with_timezone(&Utc) > Utc::now(),
        "--schedule must be in the future"
    );
    Ok(Some(parsed.to_rfc3339()))
}

fn apply_template(template: Option<&str>, message: &str) -> Result<String> {
    let Some(template) = template else {
        return Ok(message.to_string());
    };
    let raw = if Path::new(template).is_file() {
        fs::read_to_string(template)
            .with_context(|| format!("Failed to read template '{}'", template))?
    } else {
        template.to_string()
    };
    Ok(raw.replace("{{message}}", message))
}

async fn resolve_recipients(api_url: &str, source: &str) -> Result<Vec<NotificationRecipient>> {
    if Path::new(source).is_file() {
        return read_recipients_file(source);
    }
    fetch_recipients_by_filter(api_url, source).await
}

fn read_recipients_file(path: &str) -> Result<Vec<NotificationRecipient>> {
    let raw = fs::read_to_string(path).with_context(|| format!("Failed to read {}", path))?;
    if path.ends_with(".json") {
        let value: Value = serde_json::from_str(&raw).context("Invalid recipients JSON")?;
        return parse_recipient_json(&value);
    }

    let mut recipients = Vec::new();
    for line in raw.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (contract_id, recipient) = line
            .split_once(',')
            .map(|(id, target)| (id.trim(), Some(target.trim().to_string())))
            .unwrap_or((line, None));
        recipients.push(NotificationRecipient {
            contract_id: contract_id.to_string(),
            recipient,
        });
    }
    Ok(dedup_recipients(recipients))
}

fn parse_recipient_json(value: &Value) -> Result<Vec<NotificationRecipient>> {
    let items = value
        .as_array()
        .or_else(|| value.get("recipients").and_then(Value::as_array))
        .context("Recipients JSON must be an array or {\"recipients\": [...]}")?;

    let mut recipients = Vec::new();
    for item in items {
        if let Some(contract_id) = item.as_str() {
            recipients.push(NotificationRecipient {
                contract_id: contract_id.to_string(),
                recipient: None,
            });
            continue;
        }
        recipients.push(NotificationRecipient {
            contract_id: item
                .get("contract_id")
                .and_then(Value::as_str)
                .context("Recipient objects require contract_id")?
                .to_string(),
            recipient: item
                .get("recipient")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
        });
    }
    Ok(dedup_recipients(recipients))
}

async fn fetch_recipients_by_filter(
    api_url: &str,
    filter: &str,
) -> Result<Vec<NotificationRecipient>> {
    let mut url = Url::parse(&format!("{}/api/contracts", api_url.trim_end_matches('/')))
        .context("Invalid registry API URL")?;
    {
        let mut pairs = url.query_pairs_mut();
        for clause in filter.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (key, value) = clause
                .split_once('=')
                .with_context(|| format!("Invalid filter '{}'; expected key=value", clause))?;
            pairs.append_pair(key.trim(), value.trim());
        }
    }

    let value: Value = reqwest::get(url)
        .await
        .context("Failed to fetch filtered contracts")?
        .error_for_status()
        .context("Contract filter request failed")?
        .json()
        .await
        .context("Invalid contract filter response")?;

    let contracts = value
        .get("data")
        .or_else(|| value.get("contracts"))
        .or_else(|| value.get("items"))
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .context("Contract filter response did not contain an array")?;

    let recipients = contracts
        .iter()
        .filter_map(|contract| {
            let contract_id = contract.get("contract_id").and_then(Value::as_str)?;
            let recipient = contract
                .get("publisher_stellar_address")
                .or_else(|| contract.get("publisher"))
                .or_else(|| contract.get("owner"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            Some(NotificationRecipient {
                contract_id: contract_id.to_string(),
                recipient,
            })
        })
        .collect();
    Ok(dedup_recipients(recipients))
}

fn dedup_recipients(recipients: Vec<NotificationRecipient>) -> Vec<NotificationRecipient> {
    let mut seen = BTreeSet::new();
    recipients
        .into_iter()
        .filter(|recipient| seen.insert(recipient.contract_id.clone()))
        .collect()
}

fn emit_report(report: &BatchNotifyReport, json_out: bool) -> Result<()> {
    if json_out {
        println!("{}", serde_json::to_string_pretty(report)?);
        return Ok(());
    }

    println!("\n{}", "Batch Notify".bold().cyan());
    println!("Type: {}", report.message_type);
    println!("Channels: {}", report.channels.join(", "));
    if let Some(scheduled_at) = &report.scheduled_at {
        println!("Scheduled: {}", scheduled_at);
    }
    println!("Total: {}", report.total);
    if report.preview {
        println!("{}", "Preview only; no notifications were sent.".yellow());
    } else {
        println!("Accepted: {}", report.accepted.to_string().green());
        println!("Failed: {}", report.failed.to_string().red());
    }
    Ok(())
}
