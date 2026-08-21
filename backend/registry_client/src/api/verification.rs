//! Contract verification: submit a build, read status, read history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::Result;
use crate::http::RequestSpec;
use crate::RegistryClient;

/// Body for `POST /api/contracts/{id}/verify`.
///
/// Mirrors the API's `ContractVerifyRequest`, which lives in the API crate
/// rather than `shared`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractVerifyRequest {
    pub source_code: String,
    pub build_params: serde_json::Value,
    pub compiler_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// Network passphrase the contract was deployed against. Optional for the
    /// three well-known networks; required for custom ones, and a mismatch with
    /// a previously recorded passphrase is rejected with 422.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_passphrase: Option<String>,
}

impl ContractVerifyRequest {
    pub fn new(
        source_code: impl Into<String>,
        compiler_version: impl Into<String>,
        build_params: serde_json::Value,
    ) -> Self {
        Self {
            source_code: source_code.into(),
            compiler_version: compiler_version.into(),
            build_params,
            notes: None,
            network_passphrase: None,
        }
    }

    pub fn with_notes(mut self, notes: Option<String>) -> Self {
        self.notes = notes;
        self
    }

    pub fn with_network_passphrase(mut self, passphrase: Option<String>) -> Self {
        self.network_passphrase = passphrase;
        self
    }
}

/// Response to a verification submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSubmitResponse {
    pub verification_id: Uuid,
    pub contract_id: String,
    pub status: String,
    pub message: String,
    pub submitted_at: DateTime<Utc>,
    /// Set when the registry already held a passphrase and the request omitted
    /// one; start sending it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passphrase_warning: Option<String>,
}

/// Current verification state of a contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationStatusResponse {
    pub contract_id: String,
    pub verification_status: String,
    pub is_verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub verification_method: Option<String>,
    pub auditor: Option<String>,
    pub report_url: Option<String>,
    pub verification_notes: Option<String>,
    /// Whether the registry served this from its status cache.
    #[serde(default)]
    pub cached: bool,
    #[serde(default)]
    pub network_passphrase: Option<String>,
}

/// One status transition in a contract's verification history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationHistoryEntry {
    pub id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub changed_by: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Chronological audit trail of verification status changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationHistoryResponse {
    pub contract_id: String,
    pub total: usize,
    pub history: Vec<VerificationHistoryEntry>,
}

impl RegistryClient {
    /// `POST /api/contracts/{id}/verify` — submit a build for verification.
    ///
    /// A submission has side effects (it queues work), so it is only retried
    /// when `idempotency_key` is supplied.
    pub async fn submit_contract_verification(
        &self,
        id: &str,
        request: &ContractVerifyRequest,
        idempotency_key: Option<String>,
    ) -> Result<VerificationSubmitResponse> {
        let spec = RequestSpec::post(format!("/api/contracts/{id}/verify"))
            .json_body(request)?
            .idempotency_key(idempotency_key)?;
        self.transport.send_json(spec).await
    }

    /// `GET /api/contracts/{id}/verification-status`.
    pub async fn contract_verification_status(
        &self,
        id: &str,
    ) -> Result<VerificationStatusResponse> {
        self.transport
            .send_json(RequestSpec::get(format!(
                "/api/contracts/{id}/verification-status"
            )))
            .await
    }

    /// `GET /api/contracts/{id}/verification-history`.
    pub async fn contract_verification_history(
        &self,
        id: &str,
    ) -> Result<VerificationHistoryResponse> {
        self.transport
            .send_json(RequestSpec::get(format!(
                "/api/contracts/{id}/verification-history"
            )))
            .await
    }
}
