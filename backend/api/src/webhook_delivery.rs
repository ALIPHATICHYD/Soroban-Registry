// Webhook delivery background task.
//
// Polls `notification_delivery_logs` for pending entries, makes HMAC-SHA256-signed
// HTTP POST requests to the configured endpoint, and writes back the result.
// Retries up to MAX_ATTEMPTS times with exponential backoff.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{error, info, warn};
use uuid::Uuid;

const MAX_ATTEMPTS: i32 = 5;
const POLL_INTERVAL: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

// Exponential backoff delays between attempts (seconds).
const BACKOFF_SECS: [u64; 5] = [0, 30, 120, 450, 1200];

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, sqlx::FromRow)]
struct PendingDelivery {
    id: Uuid,
    webhook_id: Uuid,
    notification_id: Option<Uuid>,
    event_type: String,
    attempt_number: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct WebhookEndpoint {
    url: String,
    secret: Option<String>,
    verify_ssl: bool,
    custom_headers: Option<serde_json::Value>,
    is_active: bool,
}

/// Spawn the delivery worker as a detached Tokio task.
pub fn spawn_webhook_delivery_task(db: PgPool) {
    tokio::spawn(async move {
        run_delivery_loop(db).await;
    });
}

async fn run_delivery_loop(db: PgPool) {
    info!("Webhook delivery worker started");
    let client = build_client();

    loop {
        match fetch_pending(&db).await {
            Ok(deliveries) if deliveries.is_empty() => {}
            Ok(deliveries) => {
                for delivery in deliveries {
                    process_delivery(&db, &client, delivery).await;
                }
            }
            Err(e) => error!(error = %e, "Failed to fetch pending webhook deliveries"),
        }
        sleep(POLL_INTERVAL).await;
    }
}

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent("Soroban-Registry-Webhook/1.0")
        .build()
        .expect("Failed to build HTTP client for webhook delivery")
}

async fn fetch_pending(db: &PgPool) -> Result<Vec<PendingDelivery>, sqlx::Error> {
    // Claim up to 10 deliveries atomically to avoid double-processing under concurrency.
    sqlx::query_as::<_, PendingDelivery>(
        r#"
        UPDATE notification_delivery_logs
        SET status = 'processing', updated_at = NOW()
        WHERE id IN (
            SELECT id FROM notification_delivery_logs
            WHERE status = 'pending'
              AND attempt_number < $1
            ORDER BY created_at ASC
            LIMIT 10
            FOR UPDATE SKIP LOCKED
        )
        RETURNING id, webhook_id, notification_id, event_type, attempt_number
        "#,
    )
    .bind(MAX_ATTEMPTS)
    .fetch_all(db)
    .await
}

async fn process_delivery(db: &PgPool, client: &reqwest::Client, delivery: PendingDelivery) {
    let endpoint = match fetch_endpoint(db, delivery.webhook_id).await {
        Ok(Some(e)) => e,
        Ok(None) => {
            mark_failed(db, delivery.id, "Webhook configuration not found", None).await;
            return;
        }
        Err(e) => {
            mark_failed(db, delivery.id, &e.to_string(), None).await;
            return;
        }
    };

    if !endpoint.is_active {
        mark_failed(db, delivery.id, "Webhook is disabled", None).await;
        return;
    }

    // Apply backoff before retrying (first attempt has 0-second backoff).
    let attempt = delivery.attempt_number as usize;
    let backoff = BACKOFF_SECS.get(attempt).copied().unwrap_or(1200);
    if backoff > 0 {
        sleep(Duration::from_secs(backoff)).await;
    }

    let payload = build_payload(&delivery);
    let signature = sign_payload(&payload, endpoint.secret.as_deref());
    let delivery_id = delivery.id;

    let start = Instant::now();
    let result = send_request(client, &endpoint, &payload, &signature, delivery_id).await;
    let elapsed_ms = start.elapsed().as_millis() as i64;

    match result {
        Ok(status_code) if status_code < 300 => {
            info!(
                delivery_id = %delivery_id,
                status_code = status_code,
                elapsed_ms = elapsed_ms,
                "Webhook delivery succeeded"
            );
            mark_delivered(db, delivery_id, status_code, elapsed_ms).await;
            update_webhook_stats(db, delivery.webhook_id, true).await;
        }
        Ok(status_code) => {
            let msg = format!("Endpoint returned HTTP {}", status_code);
            warn!(delivery_id = %delivery_id, status_code = status_code, "Webhook delivery failed");
            retry_or_fail(
                db,
                delivery_id,
                delivery.webhook_id,
                &msg,
                Some(status_code),
                elapsed_ms,
                attempt,
            )
            .await;
        }
        Err(e) => {
            warn!(delivery_id = %delivery_id, error = %e, "Webhook delivery error");
            retry_or_fail(
                db,
                delivery_id,
                delivery.webhook_id,
                &e.to_string(),
                None,
                elapsed_ms,
                attempt,
            )
            .await;
        }
    }
}

async fn fetch_endpoint(
    db: &PgPool,
    webhook_id: Uuid,
) -> Result<Option<WebhookEndpoint>, sqlx::Error> {
    sqlx::query_as::<_, WebhookEndpoint>(
        "SELECT url, secret, verify_ssl, custom_headers, is_active FROM webhook_configurations WHERE id = $1",
    )
    .bind(webhook_id)
    .fetch_optional(db)
    .await
}

fn build_payload(delivery: &PendingDelivery) -> String {
    serde_json::json!({
        "delivery_id": delivery.id,
        "event_type": delivery.event_type,
        "notification_id": delivery.notification_id,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })
    .to_string()
}

fn sign_payload(payload: &str, secret: Option<&str>) -> String {
    if let Some(secret) = secret.filter(|s| !s.is_empty()) {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
        mac.update(payload.as_bytes());
        let result = mac.finalize().into_bytes();
        format!("sha256={}", hex::encode(result))
    } else {
        String::new()
    }
}

async fn send_request(
    client: &reqwest::Client,
    endpoint: &WebhookEndpoint,
    payload: &str,
    signature: &str,
    delivery_id: Uuid,
) -> Result<u16, reqwest::Error> {
    let mut req = client
        .post(&endpoint.url)
        .header("Content-Type", "application/json")
        .header("X-Soroban-Delivery-Id", delivery_id.to_string())
        .body(payload.to_owned());

    if !signature.is_empty() {
        req = req.header("X-Soroban-Signature", signature);
    }

    // Apply user-supplied custom headers.
    if let Some(serde_json::Value::Object(headers)) = &endpoint.custom_headers {
        for (k, v) in headers {
            if let Some(v_str) = v.as_str() {
                req = req.header(k.as_str(), v_str);
            }
        }
    }

    let response = req.send().await?;
    Ok(response.status().as_u16())
}

async fn retry_or_fail(
    db: &PgPool,
    delivery_id: Uuid,
    webhook_id: Uuid,
    error_msg: &str,
    response_code: Option<u16>,
    elapsed_ms: i64,
    current_attempt: usize,
) {
    let next_attempt = current_attempt as i32 + 1;
    if next_attempt >= MAX_ATTEMPTS {
        mark_failed(db, delivery_id, error_msg, response_code).await;
        update_webhook_stats(db, webhook_id, false).await;
    } else {
        // Put back as pending for the next poll cycle (backoff is applied at pickup time).
        let _ = sqlx::query(
            r#"
            UPDATE notification_delivery_logs
            SET status = 'pending', attempt_number = $1, error_message = $2,
                delivery_duration_ms = $3, response_code = $4, updated_at = NOW()
            WHERE id = $5
            "#,
        )
        .bind(next_attempt)
        .bind(error_msg)
        .bind(elapsed_ms)
        .bind(response_code.map(|c| c as i32))
        .bind(delivery_id)
        .execute(db)
        .await;
    }
}

async fn mark_delivered(db: &PgPool, delivery_id: Uuid, status_code: u16, elapsed_ms: i64) {
    let _ = sqlx::query(
        r#"
        UPDATE notification_delivery_logs
        SET status = 'delivered', response_code = $1, delivery_duration_ms = $2,
            error_message = NULL, updated_at = NOW()
        WHERE id = $3
        "#,
    )
    .bind(status_code as i32)
    .bind(elapsed_ms)
    .bind(delivery_id)
    .execute(db)
    .await;
}

async fn mark_failed(db: &PgPool, delivery_id: Uuid, error_msg: &str, response_code: Option<u16>) {
    let _ = sqlx::query(
        r#"
        UPDATE notification_delivery_logs
        SET status = 'failed', error_message = $1, response_code = $2, updated_at = NOW()
        WHERE id = $3
        "#,
    )
    .bind(error_msg)
    .bind(response_code.map(|c| c as i32))
    .bind(delivery_id)
    .execute(db)
    .await;
}

async fn update_webhook_stats(db: &PgPool, webhook_id: Uuid, success: bool) {
    let query = if success {
        r#"
        UPDATE webhook_configurations
        SET total_deliveries = total_deliveries + 1,
            last_delivery_at = NOW(),
            last_success_at = NOW(),
            consecutive_failures = 0,
            updated_at = NOW()
        WHERE id = $1
        "#
    } else {
        r#"
        UPDATE webhook_configurations
        SET total_deliveries = total_deliveries + 1,
            failed_deliveries = failed_deliveries + 1,
            last_delivery_at = NOW(),
            last_failure_at = NOW(),
            consecutive_failures = consecutive_failures + 1,
            updated_at = NOW()
        WHERE id = $1
        "#
    };
    let _ = sqlx::query(query).bind(webhook_id).execute(db).await;
}
