//! Deprecation lifecycle: announce, inspect, and withdraw a deprecation.

use shared::models::{DeprecateContractRequest, DeprecationInfo};

use crate::error::Result;
use crate::http::RequestSpec;
use crate::RegistryClient;

impl RegistryClient {
    /// `GET /api/contracts/{id}/deprecation-info`.
    pub async fn deprecation_info(&self, id: &str) -> Result<DeprecationInfo> {
        self.transport
            .send_json(RequestSpec::get(format!(
                "/api/contracts/{id}/deprecation-info"
            )))
            .await
    }

    /// `POST /api/contracts/{id}/deprecate`.
    ///
    /// Deprecation notifies subscribers, so it is only retried when
    /// `idempotency_key` is supplied.
    pub async fn deprecate_contract(
        &self,
        id: &str,
        request: &DeprecateContractRequest,
        idempotency_key: Option<String>,
    ) -> Result<DeprecationInfo> {
        let spec = RequestSpec::post(format!("/api/contracts/{id}/deprecate"))
            .json_body(request)?
            .idempotency_key(idempotency_key)?;
        self.transport.send_json(spec).await
    }

    /// `DELETE /api/contracts/{id}/deprecate` — withdraw a deprecation.
    pub async fn undeprecate_contract(
        &self,
        id: &str,
        idempotency_key: Option<String>,
    ) -> Result<DeprecationInfo> {
        let spec = RequestSpec::delete(format!("/api/contracts/{id}/deprecate"))
            .idempotency_key(idempotency_key)?;
        self.transport.send_json(spec).await
    }
}
