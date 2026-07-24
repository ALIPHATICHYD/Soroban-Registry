#![allow(dead_code)]

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::Duration;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

// ── Event types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    ContractPublished,
    ContractVerified,
    ContractFailedVerification,
    VersionCreated,
}

impl std::fmt::Display for WebhookEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            WebhookEvent::ContractPublished => "contract.published",
            WebhookEvent::ContractVerified => "contract.verified",
            WebhookEvent::ContractFailedVerification => "contract.failed_verification",
            WebhookEvent::VersionCreated => "version.created",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for WebhookEvent {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "contract.published" => Ok(WebhookEvent::ContractPublished),
            "contract.verified" => Ok(WebhookEvent::ContractVerified),
            "contract.failed_verification" => Ok(WebhookEvent::ContractFailedVerification),
            "version.created" => Ok(WebhookEvent::VersionCreated),
            _ => anyhow::bail!(
                "Unknown event type: {}. Valid: contract.published, contract.verified, contract.failed_verification, version.created",
                s
            ),
        }
    }
}

// ── Data models ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSubscription {
    pub id: String,
    pub url: String,
    pub events: Vec<String>,
    pub secret_key: String,
    pub created_at: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub id: String,
    pub webhook_id: String,
    pub event: String,
    pub payload: serde_json::Value,
    pub attempt: u32,
    pub status: DeliveryStatus,
    pub response_code: Option<u16>,
    pub delivered_at: Option<String>,
    pub error: Option<String>,
    pub next_retry_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed,
    DeadLetter,
}

impl std::fmt::Display for DeliveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeliveryStatus::Pending => write!(f, "pending"),
            DeliveryStatus::Delivered => write!(f, "delivered"),
            DeliveryStatus::Failed => write!(f, "failed"),
            DeliveryStatus::DeadLetter => write!(f, "dead_letter"),
        }
    }
}

// ── HMAC-SHA256 signature ─────────────────────────────────────────────────────

/// Returns the hex-encoded HMAC-SHA256 signature for a payload.
pub fn sign_payload(secret: &str, payload: &[u8]) -> Result<String> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .context("Failed to create HMAC — invalid secret key length")?;
    mac.update(payload);
    Ok(hex::encode(mac.finalize().into_bytes()))
}

/// Verify a received signature against the payload and secret.
pub fn verify_signature(secret: &str, payload: &[u8], received_sig: &str) -> Result<bool> {
    let expected = sign_payload(secret, payload)?;
    Ok(expected == received_sig)
}

// ── Delivery with retry logic ─────────────────────────────────────────────────

/// Attempt to POST the payload to the webhook URL with exponential backoff.
/// Max 5 attempts over a 24-hour window. Returns the final delivery record.
pub async fn deliver_with_retry(
    webhook: &WebhookSubscription,
    event: &str,
    payload: serde_json::Value,
) -> Result<WebhookDelivery> {
    const MAX_ATTEMPTS: u32 = 5;
    const TIMEOUT_SECS: u64 = 30;

    let delivery_id = Uuid::new_v4().to_string();
    let payload_bytes = serde_json::to_vec(&payload)?;
    let signature = sign_payload(&webhook.secret_key, &payload_bytes)?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(TIMEOUT_SECS))
        .build()?;

    let mut last_error: Option<String> = None;
    let mut last_code: Option<u16> = None;

    for attempt in 1..=MAX_ATTEMPTS {
        // Exponential backoff: 0s, 30s, 120s, 450s, 1200s
        if attempt > 1 {
            let delay_secs = 30u64 * 4u64.pow(attempt - 2);
            println!(
                "  {} Retry attempt {}/{} in {}s...",
                "↻".yellow(),
                attempt,
                MAX_ATTEMPTS,
                delay_secs
            );
            tokio::time::sleep(Duration::from_secs(delay_secs)).await;
        }

        let result = client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .header("X-Soroban-Event", event)
            .header("X-Soroban-Signature", format!("sha256={}", &signature))
            .header("X-Soroban-Delivery-Id", &delivery_id)
            .body(payload_bytes.clone())
            .send_with_retry()
            .await;

        match result {
            Ok(resp) => {
                let code = resp.status().as_u16();
                last_code = Some(code);

                if resp.status().is_success() {
                    return Ok(WebhookDelivery {
                        id: delivery_id,
                        webhook_id: webhook.id.clone(),
                        event: event.to_string(),
                        payload,
                        attempt,
                        status: DeliveryStatus::Delivered,
                        response_code: Some(code),
                        delivered_at: Some(chrono::Utc::now().to_rfc3339()),
                        error: None,
                        next_retry_at: None,
                    });
                } else {
                    last_error = Some(format!("HTTP {}", code));
                }
            }
            Err(e) => {
                last_error = Some(e.to_string());
            }
        }
    }

    // All attempts exhausted → dead-letter queue
    Ok(WebhookDelivery {
        id: delivery_id,
        webhook_id: webhook.id.clone(),
        event: event.to_string(),
        payload,
        attempt: MAX_ATTEMPTS,
        status: DeliveryStatus::DeadLetter,
        response_code: last_code,
        delivered_at: None,
        error: last_error,
        next_retry_at: None,
    })
}

// ── API helpers ───────────────────────────────────────────────────────────────

/// Create a new webhook subscription.
pub async fn create_webhook(
    api_url: &str,
    url: &str,
    events: Vec<String>,
    secret_key: Option<&str>,
) -> Result<()> {
    let client = crate::net::client();

    // Generate a secret key if not provided
    let secret = secret_key.map(|s| s.to_string()).unwrap_or_else(|| {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        hex::encode(bytes)
    });

    let body = serde_json::json!({
        "url": url,
        "events": events,
        "secret_key": secret,
    });

    let response = client
        .post(format!("{}/api/webhooks", api_url))
        .json(&body)
        .send_with_retry()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        anyhow::bail!("API error: {}", err);
    }

    let webhook: WebhookSubscription = response.json().await?;

    println!("\n{}", "Webhook Created".bold().green());
    println!("{}", "=".repeat(60).green());
    println!("  {}: {}", "ID".bold(), webhook.id.bright_black());
    println!("  {}: {}", "URL".bold(), webhook.url);
    println!(
        "  {}: {}",
        "Events".bold(),
        webhook.events.join(", ").bright_blue()
    );
    println!(
        "  {}: {}",
        "Secret Key".bold(),
        webhook.secret_key.bright_yellow()
    );
    println!(
        "\n  {} Store your secret key safely — it won't be shown again.",
        "⚠".yellow()
    );
    println!("{}\n", "=".repeat(60).green());

    Ok(())
}

/// List all webhook subscriptions.
pub async fn list_webhooks(api_url: &str) -> Result<()> {
    let client = crate::net::client();

    let response = client
        .get(format!("{}/api/webhooks", api_url))
        .send_with_retry()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        anyhow::bail!("API error: {}", err);
    }

    let webhooks: Vec<WebhookSubscription> = response.json().await?;

    println!("\n{}", "Webhook Subscriptions".bold().cyan());
    println!("{}", "=".repeat(60).cyan());

    if webhooks.is_empty() {
        println!("{}", "No webhooks registered.".yellow());
    } else {
        for wh in &webhooks {
            let status = if wh.active {
                "● active".green()
            } else {
                "○ inactive".yellow()
            };
            println!("\n  {} {} {}", status, wh.id.bright_black(), wh.url.bold());
            println!("    Events: {}", wh.events.join(", ").bright_blue());
            println!("    Created: {}", wh.created_at.bright_black());
        }
    }

    println!("\n{}\n", "=".repeat(60).cyan());

    Ok(())
}

/// Delete a webhook by ID.
pub async fn delete_webhook(api_url: &str, webhook_id: &str) -> Result<()> {
    let client = crate::net::client();

    let response = client
        .delete(format!("{}/api/webhooks/{}", api_url, webhook_id))
        .send_with_retry()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        anyhow::bail!("API error: {}", err);
    }

    println!(
        "{} Webhook {} deleted.",
        "✓".green(),
        webhook_id.bright_black()
    );

    Ok(())
}

/// Send a test event to a webhook.
pub async fn test_webhook(api_url: &str, webhook_id: &str) -> Result<()> {
    let client = crate::net::client();

    let response = client
        .post(format!("{}/api/webhooks/{}/test", api_url, webhook_id))
        .send_with_retry()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        anyhow::bail!("API error: {}", err);
    }

    println!(
        "{} Test event sent to webhook {}.",
        "✓".green(),
        webhook_id.bright_black()
    );
    println!("  Check your endpoint for the incoming request.\n");

    Ok(())
}

/// View delivery logs for a webhook, including dead-letter entries.
pub async fn webhook_logs(api_url: &str, webhook_id: &str, limit: usize) -> Result<()> {
    let client = crate::net::client();

    let response = client
        .get(format!(
            "{}/api/webhooks/{}/deliveries?limit={}",
            api_url, webhook_id, limit
        ))
        .send_with_retry()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        anyhow::bail!("API error: {}", err);
    }

    let deliveries: Vec<WebhookDelivery> = response.json().await?;

    println!("\n{}", "Webhook Delivery Logs".bold().cyan());
    println!("{}", "=".repeat(60).cyan());
    println!("  Webhook: {}", webhook_id.bright_black());

    if deliveries.is_empty() {
        println!("  {}", "No deliveries found.".yellow());
    } else {
        for d in &deliveries {
            let status_str = match d.status {
                DeliveryStatus::Delivered => "✓ delivered".green(),
                DeliveryStatus::Pending => "⏳ pending".yellow(),
                DeliveryStatus::Failed => "✗ failed".red(),
                DeliveryStatus::DeadLetter => "☠ dead-letter".bright_red(),
            };

            println!(
                "\n  {} {} — {}",
                status_str,
                d.id.bright_black(),
                d.event.bold()
            );
            println!("    Attempt: {}/5", d.attempt);

            if let Some(code) = d.response_code {
                println!("    Response: HTTP {}", code);
            }
            if let Some(err) = &d.error {
                println!("    Error: {}", err.red());
            }
            if let Some(delivered_at) = &d.delivered_at {
                println!("    Delivered at: {}", delivered_at.bright_black());
            }
            if d.status == DeliveryStatus::DeadLetter {
                println!(
                    "    {} Max retries exhausted. Use 'webhook retry' to manually re-queue.",
                    "⚠".yellow()
                );
            }
        }
    }

    println!("\n{}\n", "=".repeat(60).cyan());

    Ok(())
}

/// Manually retry a dead-letter delivery.
pub async fn retry_delivery(api_url: &str, delivery_id: &str) -> Result<()> {
    let client = crate::net::client();

    let response = client
        .post(format!(
            "{}/api/webhook-deliveries/{}/retry",
            api_url, delivery_id
        ))
        .send_with_retry()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        anyhow::bail!("API error: {}", err);
    }

    println!(
        "{} Delivery {} re-queued for retry.",
        "✓".green(),
        delivery_id.bright_black()
    );

    Ok(())
}

/// Verify a signature locally without hitting the API.
pub fn verify_signature_cmd(secret: &str, payload: &str, signature: &str) -> Result<()> {
    let sig_hex = signature.strip_prefix("sha256=").unwrap_or(signature);
    let is_valid = verify_signature(secret, payload.as_bytes(), sig_hex)?;

    if is_valid {
        println!("{} Signature is valid.", "✓".green());
    } else {
        println!("{} Signature is INVALID.", "✗".red());
        anyhow::bail!("Signature verification failed");
    }

    Ok(())
}
