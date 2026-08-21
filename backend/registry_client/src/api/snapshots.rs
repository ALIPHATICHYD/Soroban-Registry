//! Signed, offline-verifiable contract snapshots.

use serde::{Deserialize, Serialize};
use shared::snapshot::ContractSnapshot;

use crate::error::Result;
use crate::http::RequestSpec;
use crate::RegistryClient;

/// The registry's snapshot-signing public key.
///
/// Pin `key_fingerprint` in a verifier so a snapshot signed by a different key
/// is rejected rather than trusted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySigningKey {
    pub algorithm: String,
    /// Base64-encoded public key. Public material — safe to print.
    pub public_key: String,
    pub key_fingerprint: String,
}

impl RegistryClient {
    /// `GET /api/contracts/{id}/snapshot` — a signed snapshot of a contract.
    ///
    /// The payload verifies offline against [`RegistryClient::registry_signing_key`]
    /// using `shared::snapshot::verify_snapshot`.
    pub async fn contract_snapshot(&self, id: &str) -> Result<ContractSnapshot> {
        self.transport
            .send_json(RequestSpec::get(format!("/api/contracts/{id}/snapshot")))
            .await
    }

    /// `GET /api/registry/signing-key`.
    pub async fn registry_signing_key(&self) -> Result<RegistrySigningKey> {
        self.transport
            .send_json(RequestSpec::get("/api/registry/signing-key"))
            .await
    }
}
