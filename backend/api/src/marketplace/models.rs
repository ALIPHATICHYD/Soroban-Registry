use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PricingPlan {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i64,
    pub currency: String,
    pub billing_period: String,
    pub call_quota: Option<i64>,
    pub features: serde_json::Value,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePricingPlan {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub price_cents: i64,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default = "default_billing_period")]
    pub billing_period: String,
    #[serde(default)]
    pub call_quota: Option<i64>,
    #[serde(default)]
    pub features: serde_json::Value,
}

fn default_currency() -> String {
    "USD".to_string()
}

fn default_billing_period() -> String {
    "monthly".to_string()
}

/// PATCH payload — only fields present are applied. Note that
/// `call_quota` is intentionally not patchable in v1: distinguishing
/// "omitted" from "explicit null" via plain serde requires extra
/// machinery, and owners can disable a plan + create a replacement
/// with the desired quota. This will revisit when billing lands.
#[derive(Debug, Deserialize)]
pub struct UpdatePricingPlan {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub price_cents: Option<i64>,
    #[serde(default)]
    pub features: Option<serde_json::Value>,
    #[serde(default)]
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct LicenseRecord {
    pub id: Uuid,
    pub jti: Uuid,
    pub contract_id: Uuid,
    pub plan_id: Uuid,
    pub owner_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub status: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct IssueLicenseRequest {
    pub plan_id: Uuid,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct IssuedLicense {
    pub license: LicenseRecord,
    pub token: String,
    pub public_key_b64: String,
}

#[derive(Debug, Deserialize)]
pub struct ValidateLicenseRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateLicenseResponse {
    pub valid: bool,
    pub reason: Option<String>,
    pub claims: Option<super::license::LicenseClaims>,
    pub status: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct RecordUsageRequest {
    #[serde(default = "one")]
    pub call_count: i32,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

fn one() -> i32 {
    1
}

#[derive(Debug, Serialize)]
pub struct UsageSummary {
    pub license_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_calls: i64,
    pub event_count: i64,
    pub call_quota: Option<i64>,
    pub quota_exceeded: bool,
}
