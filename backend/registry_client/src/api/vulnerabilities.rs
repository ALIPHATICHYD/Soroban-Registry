//! Dependency scanning and vulnerability reports.

use shared::models::DependencyScanReport;

use crate::error::Result;
use crate::http::RequestSpec;
use crate::RegistryClient;

impl RegistryClient {
    /// `GET /api/contracts/{id}/dependency-scan` — the latest scan report.
    ///
    /// `id` is the registry UUID; this endpoint does not accept slugs.
    pub async fn dependency_scan_report(&self, id: &str) -> Result<DependencyScanReport> {
        self.transport
            .send_json(RequestSpec::get(format!(
                "/api/contracts/{id}/dependency-scan"
            )))
            .await
    }

    /// `POST /api/contracts/{id}/dependency-scan` — run a scan now.
    ///
    /// Scanning consumes upstream advisory-database quota, so it is only
    /// retried when `idempotency_key` is supplied.
    pub async fn trigger_dependency_scan(
        &self,
        id: &str,
        idempotency_key: Option<String>,
    ) -> Result<DependencyScanReport> {
        let spec = RequestSpec::post(format!("/api/contracts/{id}/dependency-scan"))
            .idempotency_key(idempotency_key)?;
        self.transport.send_json(spec).await
    }

    /// `GET /api/v1/contracts/{id}/vulnerability-assessment`.
    ///
    /// The assessment aggregates several scanners into a document with no
    /// single backend type, so it is returned as-is.
    pub async fn vulnerability_assessment(&self, id: &str) -> Result<serde_json::Value> {
        self.transport
            .send_json(RequestSpec::get(format!(
                "/api/v1/contracts/{id}/vulnerability-assessment"
            )))
            .await
    }
}
