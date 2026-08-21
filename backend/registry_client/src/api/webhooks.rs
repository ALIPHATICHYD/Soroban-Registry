//! Webhook configuration, delivery logs, and redelivery.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::models::{CreateWebhookRequest, PaginatedResponse, WebhookConfiguration};
use uuid::Uuid;

use crate::config::Secret;
use crate::error::Result;
use crate::http::RequestSpec;
use crate::RegistryClient;

/// One delivery attempt for a webhook.
///
/// Mirrors the API's `WebhookDeliveryLog`, which lives in the API crate rather
/// than `shared`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub notification_id: Option<Uuid>,
    pub event_type: String,
    pub status: String,
    pub response_code: Option<i32>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub attempt_number: i32,
    pub delivery_duration_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
}

/// A freshly created webhook, with its signing secret held apart.
///
/// The registry returns the signing secret exactly once, at creation. It is
/// lifted out of the configuration into a [`Secret`] so that neither logging
/// the webhook nor logging this struct can print it — reach it deliberately
/// with [`Secret::expose`].
#[derive(Debug, Clone)]
pub struct CreatedWebhook {
    /// The stored configuration, with the secret field cleared.
    pub webhook: WebhookConfiguration,
    /// The signing secret, if the registry issued one.
    pub secret: Option<Secret>,
}

impl RegistryClient {
    /// `GET /api/webhooks` — the caller's webhooks.
    pub async fn list_webhooks(
        &self,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<PaginatedResponse<WebhookConfiguration>> {
        let spec = RequestSpec::get("/api/webhooks")
            .maybe_query("page", page)
            .maybe_query("limit", limit);
        self.transport.send_json(spec).await
    }

    /// `POST /api/webhooks` — register a webhook.
    ///
    /// The response's signing secret is moved into [`CreatedWebhook::secret`]
    /// so it cannot be printed by accident.
    pub async fn create_webhook(
        &self,
        request: &CreateWebhookRequest,
        idempotency_key: Option<String>,
    ) -> Result<CreatedWebhook> {
        let spec = RequestSpec::post("/api/webhooks")
            .json_body(request)?
            .idempotency_key(idempotency_key)?;
        let mut webhook: WebhookConfiguration = self.transport.send_json(spec).await?;
        let secret = webhook.secret.take().map(Secret::new);
        Ok(CreatedWebhook { webhook, secret })
    }

    /// `DELETE /api/webhooks/{id}`.
    ///
    /// Deleting is naturally idempotent in effect, but a repeat returns 404, so
    /// it is only retried when `idempotency_key` is supplied.
    pub async fn delete_webhook(
        &self,
        webhook_id: &str,
        idempotency_key: Option<String>,
    ) -> Result<()> {
        let spec = RequestSpec::delete(format!("/api/webhooks/{webhook_id}"))
            .idempotency_key(idempotency_key)?;
        self.transport.send_discard(spec).await
    }

    /// `GET /api/webhooks/{id}/deliveries` — delivery attempts, newest first.
    pub async fn webhook_deliveries(
        &self,
        webhook_id: &str,
        page: Option<i64>,
        limit: Option<i64>,
    ) -> Result<PaginatedResponse<WebhookDelivery>> {
        let spec = RequestSpec::get(format!("/api/webhooks/{webhook_id}/deliveries"))
            .maybe_query("page", page)
            .maybe_query("limit", limit);
        self.transport.send_json(spec).await
    }

    /// `POST /api/webhooks/{id}/test` — send a test event.
    pub async fn test_webhook(
        &self,
        webhook_id: &str,
        idempotency_key: Option<String>,
    ) -> Result<()> {
        let spec = RequestSpec::post(format!("/api/webhooks/{webhook_id}/test"))
            .idempotency_key(idempotency_key)?;
        self.transport.send_discard(spec).await
    }

    /// `POST /api/webhook-deliveries/{id}/retry` — redeliver one event.
    pub async fn retry_webhook_delivery(
        &self,
        delivery_id: &str,
        idempotency_key: Option<String>,
    ) -> Result<()> {
        let spec = RequestSpec::post(format!("/api/webhook-deliveries/{delivery_id}/retry"))
            .idempotency_key(idempotency_key)?;
        self.transport.send_discard(spec).await
    }
}
