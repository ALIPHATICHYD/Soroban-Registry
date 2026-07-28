use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    disaster_recovery_models::{
        CreateNotificationTemplateRequest, CreateUserNotificationPreferenceRequest,
        NotificationTemplate, SendNotificationRequest, UserNotificationPreference,
    },
    error::{ApiError, ApiResult},
    state::AppState,
};

pub async fn create_notification_template(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateNotificationTemplateRequest>,
) -> ApiResult<Json<NotificationTemplate>> {
    let template = sqlx::query_as::<_, NotificationTemplate>(
        r#"
        INSERT INTO notification_templates 
        (name, subject, message_template, channel)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(&req.name)
    .bind(&req.subject)
    .bind(&req.message_template)
    .bind(&req.channel)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create notification template: {}", e)))?;

    Ok(Json(template))
}

pub async fn get_notification_template(
    State(state): State<AppState>,
    Path(template_name): Path<String>,
) -> ApiResult<Json<NotificationTemplate>> {
    let template = sqlx::query_as::<_, NotificationTemplate>(
        "SELECT * FROM notification_templates WHERE name = $1",
    )
    .bind(&template_name)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| {
        ApiError::not_found("notification_template", "Notification template not found")
    })?;

    Ok(Json(template))
}

pub async fn create_user_notification_preference(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateUserNotificationPreferenceRequest>,
) -> ApiResult<Json<UserNotificationPreference>> {
    let preference = sqlx::query_as::<_, UserNotificationPreference>(
        r#"
        INSERT INTO user_notification_preferences 
        (user_id, contract_id, notification_types, channels, enabled)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(req.user_id)
    .bind(req.contract_id)
    .bind(&req.notification_types)
    .bind(&req.channels)
    .bind(req.enabled)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create notification preference: {}", e)))?;

    Ok(Json(preference))
}

pub async fn get_user_notification_preferences(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Json<Vec<UserNotificationPreference>>> {
    let preferences = sqlx::query_as::<_, UserNotificationPreference>(
        "SELECT * FROM user_notification_preferences WHERE user_id = $1 AND enabled = true",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(preferences))
}

pub async fn send_notification(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<SendNotificationRequest>,
) -> ApiResult<StatusCode> {
    // First, get the notification template
    let template = sqlx::query_as::<_, NotificationTemplate>(
        "SELECT * FROM notification_templates WHERE name = $1",
    )
    .bind(&req.notification_type)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    .ok_or_else(|| {
        ApiError::not_found("notification_template", "Notification template not found")
    })?;

    // Process template with variables
    let mut message = template.message_template.clone();
    for (key, value) in &req.template_variables {
        let placeholder = format!("{{{{{}}}}}", key); // {{variable}}
        message = message.replace(&placeholder, value);
    }

    // For now, just log the notification - in a real implementation this would send via email/SMS/etc.
    println!(
        "Notification sent to {:?}: {} - {}",
        req.recipients, template.subject, message
    );

    // In a real system, we'd store the notification in a queue table for processing
    // and track delivery status

    // Log the notification for audit purposes
    sqlx::query(
        r#"
        INSERT INTO notification_logs 
        (contract_id, notification_type, recipients, message, sent_at, status)
        VALUES ($1, $2, $3, $4, $5, 'sent')
        "#,
    )
    .bind(req.contract_id)
    .bind(&req.notification_type)
    .bind(&req.recipients)
    .bind(&message)
    .bind(Utc::now())
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to log notification: {}", e)))?;

    Ok(StatusCode::OK)
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchNotificationRecipient {
    pub contract_id: String,
    pub recipient: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchNotificationRequest {
    pub message: String,
    pub message_type: String,
    pub channels: Vec<String>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub recipients: Vec<BatchNotificationRecipient>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchNotificationResult {
    pub contract_id: String,
    pub recipient: Option<String>,
    pub delivery_ids: Vec<Uuid>,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchNotificationResponse {
    pub batch_id: Uuid,
    pub accepted: usize,
    pub failed: usize,
    pub scheduled_at: DateTime<Utc>,
    pub recipients: Vec<BatchNotificationResult>,
}

pub async fn send_batch_notifications(
    State(state): State<AppState>,
    Json(req): Json<BatchNotificationRequest>,
) -> ApiResult<Json<BatchNotificationResponse>> {
    validate_batch_notification_request(&req)?;

    let scheduled_at = req.scheduled_at.unwrap_or_else(Utc::now);
    let batch_id = Uuid::new_v4();
    let channels = req
        .channels
        .iter()
        .map(|channel| map_batch_channel(channel))
        .collect::<ApiResult<Vec<String>>>()?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| ApiError::internal(format!("begin batch notification: {}", e)))?;

    sqlx::query(
        "INSERT INTO batch_notification_jobs
         (id, message_type, message, channels, scheduled_at, status, total_recipients)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(batch_id)
    .bind(&req.message_type)
    .bind(&req.message)
    .bind(&channels)
    .bind(scheduled_at)
    .bind(if scheduled_at > Utc::now() {
        "scheduled"
    } else {
        "sent"
    })
    .bind(req.recipients.len() as i32)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(format!("create notification batch: {}", e)))?;

    let mut results = Vec::with_capacity(req.recipients.len());
    for recipient in &req.recipients {
        let contract = sqlx::query_as::<_, (Uuid, String, Option<String>, String)>(
            "SELECT c.id, c.contract_id, p.email, p.stellar_address
             FROM contracts c
             JOIN publishers p ON p.id = c.publisher_id
             WHERE c.contract_id = $1 OR c.id::text = $1
             ORDER BY c.updated_at DESC
             LIMIT 1",
        )
        .bind(&recipient.contract_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| ApiError::internal(format!("lookup notification recipient: {}", e)))?;

        let Some((contract_uuid, contract_address, email, stellar_address)) = contract else {
            results.push(BatchNotificationResult {
                contract_id: recipient.contract_id.clone(),
                recipient: recipient.recipient.clone(),
                delivery_ids: Vec::new(),
                status: "failed".to_string(),
                message: "contract not found".to_string(),
            });
            continue;
        };

        let delivery_target = recipient
            .recipient
            .clone()
            .or(email)
            .unwrap_or(stellar_address);
        let recipients = vec![delivery_target.clone()];
        let log_status = if scheduled_at > Utc::now() {
            "pending"
        } else {
            "sent"
        };

        let mut delivery_ids = Vec::with_capacity(channels.len());
        for channel in &channels {
            let delivery_id = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO batch_notification_deliveries
                 (job_id, contract_id, contract_address, recipient, channel, delivery_status, sent_at)
                 VALUES ($1, $2, $3, $4, $5, $6, CASE WHEN $6 = 'sent' THEN NOW() ELSE NULL END)
                 RETURNING id",
            )
            .bind(batch_id)
            .bind(contract_uuid)
            .bind(&contract_address)
            .bind(&delivery_target)
            .bind(channel)
            .bind(log_status)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| ApiError::internal(format!("track notification delivery: {}", e)))?;
            delivery_ids.push(delivery_id);
        }

        sqlx::query(
            "INSERT INTO notification_logs
             (contract_id, notification_type, recipients, message, sent_at, status)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(contract_uuid)
        .bind(&req.message_type)
        .bind(&recipients)
        .bind(&req.message)
        .bind(scheduled_at)
        .bind(if log_status == "pending" {
            "scheduled"
        } else {
            "sent"
        })
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::internal(format!("log batch notification: {}", e)))?;

        results.push(BatchNotificationResult {
            contract_id: recipient.contract_id.clone(),
            recipient: Some(delivery_target),
            delivery_ids,
            status: log_status.to_string(),
            message: format!("notification {}", log_status),
        });
    }

    let accepted = results
        .iter()
        .filter(|item| item.status == "sent" || item.status == "pending")
        .count();
    let failed = results.len().saturating_sub(accepted);
    sqlx::query(
        "UPDATE batch_notification_jobs
         SET total_recipients = $1,
             delivered_count = $2,
             failed_count = $3,
             status = CASE
                 WHEN $4 > NOW() THEN 'scheduled'
                 WHEN $3 = 0 THEN 'sent'
                 WHEN $2 > 0 THEN 'partial'
                 ELSE 'failed'
             END,
             updated_at = NOW()
         WHERE id = $5",
    )
    .bind(results.len() as i32)
    .bind(accepted as i32)
    .bind(failed as i32)
    .bind(scheduled_at)
    .bind(batch_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(format!("update notification batch: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::internal(format!("commit batch notification: {}", e)))?;

    Ok(Json(BatchNotificationResponse {
        batch_id,
        accepted,
        failed,
        scheduled_at,
        recipients: results,
    }))
}

pub async fn get_batch_notification_status(
    State(state): State<AppState>,
    Path(batch_id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let job = sqlx::query_as::<_, (Uuid, String, String, DateTime<Utc>, String, i32, i32, i32)>(
        "SELECT id, message_type, message, scheduled_at, status,
                total_recipients, delivered_count, failed_count
         FROM batch_notification_jobs
         WHERE id = $1",
    )
    .bind(batch_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("fetch batch notification status: {}", e)))?
    .ok_or_else(|| ApiError::not_found("BatchNotFound", "batch notification not found"))?;

    let deliveries = sqlx::query_as::<_, (Uuid, String, String, String, Option<DateTime<Utc>>)>(
        "SELECT id, contract_address, recipient, delivery_status, read_at
         FROM batch_notification_deliveries
         WHERE job_id = $1
         ORDER BY created_at ASC",
    )
    .bind(batch_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("fetch notification deliveries: {}", e)))?;

    Ok(Json(serde_json::json!({
        "batch_id": job.0,
        "message_type": job.1,
        "message": job.2,
        "scheduled_at": job.3,
        "status": job.4,
        "total_recipients": job.5,
        "delivered_count": job.6,
        "failed_count": job.7,
        "read_count": deliveries.iter().filter(|d| d.4.is_some()).count(),
        "deliveries": deliveries.into_iter().map(|d| serde_json::json!({
            "delivery_id": d.0,
            "contract_id": d.1,
            "recipient": d.2,
            "delivery_status": d.3,
            "read_at": d.4,
        })).collect::<Vec<_>>()
    })))
}

pub async fn mark_batch_notification_read(
    State(state): State<AppState>,
    Path(delivery_id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let result = sqlx::query(
        "UPDATE batch_notification_deliveries
         SET read_at = COALESCE(read_at, NOW())
         WHERE id = $1",
    )
    .bind(delivery_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("mark notification read: {}", e)))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found(
            "DeliveryNotFound",
            "notification delivery not found",
        ));
    }
    Ok(StatusCode::OK)
}

fn validate_batch_notification_request(req: &BatchNotificationRequest) -> ApiResult<()> {
    if req.message.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidNotification",
            "message cannot be empty",
        ));
    }
    if req.recipients.is_empty() {
        return Err(ApiError::bad_request(
            "InvalidNotification",
            "recipients cannot be empty",
        ));
    }
    if req.channels.is_empty() {
        return Err(ApiError::bad_request(
            "InvalidNotification",
            "channels cannot be empty",
        ));
    }
    if !matches!(
        req.message_type.as_str(),
        "info" | "warning" | "critical" | "action-required"
    ) {
        return Err(ApiError::bad_request(
            "InvalidNotificationType",
            "message_type must be info, warning, critical, or action-required",
        ));
    }
    if let Some(scheduled_at) = req.scheduled_at {
        if scheduled_at < Utc::now() {
            return Err(ApiError::bad_request(
                "InvalidSchedule",
                "scheduled_at must not be in the past",
            ));
        }
    }
    Ok(())
}

fn map_batch_channel(channel: &str) -> ApiResult<String> {
    match channel
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-")
        .as_str()
    {
        "email" => Ok("email".to_string()),
        "webhook" => Ok("webhook".to_string()),
        "in-app" | "inapp" => Ok("in-app".to_string()),
        _ => Err(ApiError::bad_request(
            "InvalidNotificationChannel",
            "channels must contain only email, in-app, or webhook",
        )),
    }
}

pub async fn get_user_notifications(
    State(_state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> ApiResult<Json<serde_json::Value>> {
    // In a real system, this would return user-specific notifications
    // For now, return a placeholder response
    Ok(Json(serde_json::json!({
        "user_id": user_id,
        "notifications": [],
        "unread_count": 0,
        "last_checked": Utc::now()
    })))
}
