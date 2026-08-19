//! Signed ownership transfers and their append-only provenance chain.

use shared::models::{
    ConfirmOwnershipTransferRequest, CreateOwnershipTransferRequest, OwnershipTransfer,
    OwnershipTransferLog,
};

use crate::error::Result;
use crate::http::RequestSpec;
use crate::RegistryClient;

impl RegistryClient {
    /// `POST /api/contracts/{id}/ownership-transfer` — initiate a transfer.
    ///
    /// The request body carries the sender's signature over a nonce, so the
    /// registry already rejects replays; an `idempotency_key` additionally makes
    /// the *transport* retryable when a response is lost.
    pub async fn create_ownership_transfer(
        &self,
        contract_id: &str,
        request: &CreateOwnershipTransferRequest,
        idempotency_key: Option<String>,
    ) -> Result<OwnershipTransfer> {
        let spec = RequestSpec::post(format!("/api/contracts/{contract_id}/ownership-transfer"))
            .json_body(request)?
            .idempotency_key(idempotency_key)?;
        self.transport.send_json(spec).await
    }

    /// `GET /api/contracts/{id}/ownership-transfer` — transfers for a contract.
    pub async fn list_ownership_transfers(
        &self,
        contract_id: &str,
    ) -> Result<Vec<OwnershipTransfer>> {
        self.transport
            .send_json(RequestSpec::get(format!(
                "/api/contracts/{contract_id}/ownership-transfer"
            )))
            .await
    }

    /// `GET /api/ownership-transfers/{id}`.
    pub async fn get_ownership_transfer(&self, transfer_id: &str) -> Result<OwnershipTransfer> {
        self.transport
            .send_json(RequestSpec::get(format!(
                "/api/ownership-transfers/{transfer_id}"
            )))
            .await
    }

    /// `POST /api/ownership-transfers/{id}/confirm` — accept or reject.
    pub async fn confirm_ownership_transfer(
        &self,
        transfer_id: &str,
        request: &ConfirmOwnershipTransferRequest,
        idempotency_key: Option<String>,
    ) -> Result<OwnershipTransfer> {
        let spec = RequestSpec::post(format!("/api/ownership-transfers/{transfer_id}/confirm"))
            .json_body(request)?
            .idempotency_key(idempotency_key)?;
        self.transport.send_json(spec).await
    }

    /// `GET /api/ownership-transfers/{id}/logs` — the append-only provenance
    /// chain for a transfer, oldest entry first.
    pub async fn ownership_transfer_logs(
        &self,
        transfer_id: &str,
    ) -> Result<Vec<OwnershipTransferLog>> {
        self.transport
            .send_json(RequestSpec::get(format!(
                "/api/ownership-transfers/{transfer_id}/logs"
            )))
            .await
    }
}
