/// RPC client for polling Stellar network ledgers
/// Handles HTTP requests to Stellar RPC endpoints and deserializes ledger/operation data
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, instrument, warn};

#[derive(Error, Debug)]
pub enum RpcError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),
    #[error("RPC returned error: {0}")]
    Remote(String),
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
    #[error("Network timeout")]
    Timeout,
}

/// Stellar RPC client
pub struct StellarRpcClient {
    endpoint: String,
    client: reqwest::Client,
    request_timeout: Duration,
}

/// Ledger information from RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ledger {
    pub sequence: u64,
    pub id: String,
    pub hash: String,
    pub prev_hash: String,
    pub timestamp: String,
}

/// Operation from ledger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub tx_id: String,
    pub type_code: u32,
    #[serde(default)]
    pub type_name: String,
    #[serde(default)]
    pub body: serde_json::Value,
}

/// Contract deployment operation details
#[derive(Debug, Clone)]
pub struct ContractDeployment {
    pub contract_id: String,
    pub deployer: String,
    pub op_id: String,
    pub tx_id: String,
    pub ledger_sequence: u64,
    pub deployed_at: String,
}

/// RPC response for ledgers
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RpcResponse<T> {
    Success(T),
    Error { error: serde_json::Value },
}

#[derive(Debug, Clone, Deserialize)]
struct LedgerResponse {
    sequence: u64,
    id: String,
    hash: String,
    prev_hash: Option<String>,
    closed_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct OperationsResponse {
    records: Vec<OperationRecord>,
}

/// Typed response structures for the latest-ledger endpoint.
/// Using concrete types (instead of serde_json::Value) avoids retaining the
/// entire JSON tree in memory while only a handful of fields are needed.
#[derive(Debug, Deserialize)]
struct LatestLedgerResponse {
    #[serde(rename = "_embedded")]
    embedded: LatestLedgerEmbedded,
}

#[derive(Debug, Deserialize)]
struct LatestLedgerEmbedded {
    records: Vec<LedgerSummary>,
}

#[derive(Debug, Deserialize)]
struct LedgerSummary {
    sequence: u64,
    id: Option<String>,
    hash: String,
    prev_hash: Option<String>,
    closed_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OperationRecord {
    id: String,
    transaction_hash: String,
    type_code: u32,
    type_name: String,
    #[serde(default)]
    body: serde_json::Value,
}

impl StellarRpcClient {
    /// Create new Stellar RPC client
    pub fn new(endpoint: String) -> Self {
        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        StellarRpcClient {
            endpoint,
            client,
            request_timeout: Duration::from_secs(30),
        }
    }

    /// Fetch ledger by sequence number
    #[instrument(skip(self), fields(rpc.endpoint = %self.endpoint, ledger.sequence = sequence))]
    pub async fn get_ledger(&self, sequence: u64) -> Result<Ledger, RpcError> {
        let url = format!("{}/ledgers/{}", self.endpoint, sequence);
        debug!("Fetching ledger from {}", url);

        let mut headers = reqwest::header::HeaderMap::new();
        crate::telemetry::inject_current_trace_context(&mut headers);

        let response = self
            .client
            .get(&url)
            .timeout(self.request_timeout)
            .headers(headers)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    RpcError::Timeout
                } else {
                    RpcError::RequestFailed(e.to_string())
                }
            })?;

        if !response.status().is_success() {
            return Err(RpcError::Remote(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let data: LedgerResponse = response.json().await.map_err(|e| {
            RpcError::InvalidResponse(format!("Failed to parse ledger response: {}", e))
        })?;

        Ok(Ledger {
            sequence: data.sequence,
            id: data.id,
            hash: data.hash,
            prev_hash: data.prev_hash.unwrap_or_default(),
            timestamp: data.closed_at,
        })
    }

    /// Fetch operations for a ledger
    #[instrument(skip(self), fields(rpc.endpoint = %self.endpoint, ledger.sequence = sequence))]
    pub async fn get_ledger_operations(&self, sequence: u64) -> Result<Vec<Operation>, RpcError> {
        let url = format!(
            "{}/ledgers/{}/operations?order=asc&limit=200",
            self.endpoint, sequence
        );
        debug!("Fetching operations for ledger {} from {}", sequence, url);

        let mut headers = reqwest::header::HeaderMap::new();
        crate::telemetry::inject_current_trace_context(&mut headers);

        let response = self
            .client
            .get(&url)
            .timeout(self.request_timeout)
            .headers(headers)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    RpcError::Timeout
                } else {
                    RpcError::RequestFailed(e.to_string())
                }
            })?;

        if !response.status().is_success() {
            return Err(RpcError::Remote(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let data: OperationsResponse = response.json().await.map_err(|e| {
            RpcError::InvalidResponse(format!("Failed to parse operations response: {}", e))
        })?;

        Ok(data
            .records
            .into_iter()
            .map(|op| Operation {
                id: op.id,
                tx_id: op.transaction_hash,
                type_code: op.type_code,
                type_name: op.type_name,
                body: op.body,
            })
            .collect())
    }

    /// Get the latest ledger
    #[instrument(skip(self), fields(rpc.endpoint = %self.endpoint))]
    pub async fn get_latest_ledger(&self) -> Result<Ledger, RpcError> {
        let url = format!("{}/ledgers?order=desc&limit=1", self.endpoint);
        debug!("Fetching latest ledger from {}", url);

        let mut headers = reqwest::header::HeaderMap::new();
        crate::telemetry::inject_current_trace_context(&mut headers);

        let response = self
            .client
            .get(&url)
            .timeout(self.request_timeout)
            .headers(headers)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    RpcError::Timeout
                } else {
                    RpcError::RequestFailed(e.to_string())
                }
            })?;

        if !response.status().is_success() {
            return Err(RpcError::Remote(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        // Deserialize directly into typed structs — avoids holding the full
        // serde_json::Value tree in memory while extracting a handful of fields.
        let data: LatestLedgerResponse = response.json().await.map_err(|e| {
            error!("Invalid JSON in latest-ledger response: {}", e);
            RpcError::InvalidResponse(format!("Failed to parse latest ledger response: {}", e))
        })?;

        let ledger = data.embedded.records.into_iter().next().ok_or_else(|| {
            error!("Empty records array in latest ledger response");
            RpcError::InvalidResponse("Empty records array".to_string())
        })?;

        Ok(Ledger {
            sequence: ledger.sequence,
            id: ledger.id.unwrap_or_else(|| ledger.hash.clone()),
            hash: ledger.hash.clone(),
            prev_hash: ledger.prev_hash.unwrap_or_default(),
            timestamp: ledger.closed_at.unwrap_or_default(),
        })
    }

    /// Check endpoint health
    pub async fn health_check(&self) -> Result<(), RpcError> {
        let url = format!("{}/health", self.endpoint);
        debug!("Checking RPC health at {}", url);

        let response = self
            .client
            .get(&url)
            .timeout(self.request_timeout)
            .send()
            .await
            .map_err(|e| {
                warn!("Health check failed: {}", e);
                if e.is_timeout() {
                    RpcError::Timeout
                } else {
                    RpcError::RequestFailed(e.to_string())
                }
            })?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(RpcError::Remote(format!(
                "Health check failed with status {}",
                response.status()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_client_creation() {
        let client = StellarRpcClient::new("https://rpc-futurenet.stellar.org".to_string());
        assert_eq!(client.endpoint, "https://rpc-futurenet.stellar.org");
    }
}
