use std::collections::HashSet;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use shared::{Contract, Network, RegistryError};
use stellar_strkey::{Contract as ContractStrkey, Strkey};
use stellar_xdr::curr::{
    ContractCodeEntry, ContractDataDurability, ContractExecutable, ContractId, Hash, LedgerEntry,
    LedgerEntryData, LedgerKey, LedgerKeyContractCode, LedgerKeyContractData, Limits, ReadXdr,
    ScAddress, ScContractInstance, ScVal, WriteXdr,
};

use crate::cache::CacheLayer;
use crate::simulation;
use contract_abi::parser::parse_json_spec;
use sha2::{Digest, Sha256};

const DEFAULT_RPC_MAINNET: &str = "https://mainnet.sorobanrpc.com";
const DEFAULT_RPC_TESTNET: &str = "https://soroban-testnet.stellar.org";
const DEFAULT_RPC_FUTURENET: &str = "https://rpc-futurenet.stellar.org";
const DEFAULT_RPC_TIMEOUT_SECS: u64 = 15;
const DEFAULT_RPC_MAX_RETRIES: u32 = 3;
const DEFAULT_ACTIVITY_LOOKBACK_LEDGERS: u32 = 2_000;
const DEFAULT_ACTIVITY_LIMIT: u32 = 25;

/// Precise reason an on-chain verification check failed.
///
/// Each variant carries just enough context for the caller to understand
/// *why* verification failed and what action to take — without leaking
/// internal RPC details or stack traces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum OnChainFailureReason {
    /// The contract address does not exist on the queried network.
    ContractNotOnChain {
        contract_id: String,
        network: String,
        hint: String,
    },
    /// The stored wasm hash does not match the hash found on-chain.
    WasmHashMismatch {
        stored: String,
        on_chain: String,
        hint: String,
    },
    /// An ABI is stored but cannot be parsed against the contract spec.
    AbiMismatch { detail: String },
    /// No ABI is available; interface conformance cannot be confirmed.
    AbiMissing,
}

impl std::fmt::Display for OnChainFailureReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContractNotOnChain {
                contract_id,
                network,
                ..
            } => {
                write!(f, "contract {contract_id} not found on {network}")
            }
            Self::WasmHashMismatch {
                stored, on_chain, ..
            } => {
                write!(f, "wasm hash mismatch: stored={stored} on_chain={on_chain}")
            }
            Self::AbiMismatch { detail } => write!(f, "abi mismatch: {detail}"),
            Self::AbiMissing => write!(f, "abi missing"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainVerificationResult {
    pub contract_id: String,
    pub network: String,
    pub rpc_endpoint: String,
    pub cached: bool,
    pub contract_exists_on_chain: bool,
    pub abi_available: bool,
    pub abi_valid: bool,
    pub abi_matches_deployed_contract: bool,
    pub recent_call_count: usize,
    pub latest_ledger: Option<u32>,
    pub oldest_ledger: Option<u32>,
    pub on_chain_wasm_hash: Option<String>,
    pub on_chain_code_hash: Option<String>,
    pub stored_wasm_hash: String,
    pub wasm_hash_matches: bool,
    pub warnings: Vec<String>,
    /// Structured failure reasons collected during verification.
    /// Empty when verification passes; contains at least one entry on failure.
    pub failure_reasons: Vec<OnChainFailureReason>,
}

impl OnChainVerificationResult {
    pub fn cache_key(contract: &Contract, abi_json: Option<&str>) -> String {
        let abi_hash = abi_json
            .map(|abi| {
                let mut hasher = Sha256::new();
                hasher.update(abi.as_bytes());
                hex::encode(hasher.finalize())
            })
            .unwrap_or_else(|| "noabi".to_string());

        format!(
            "onchain:{}:{}:{}:{}",
            contract.network, contract.contract_id, contract.wasm_hash, abi_hash
        )
    }
}

#[derive(Debug, Clone)]
struct NetworkConfig {
    rpc_endpoint: String,
    timeout: Duration,
    max_retries: u32,
}

impl NetworkConfig {
    fn from_env(network: &Network) -> Self {
        let rpc_endpoint = match network {
            Network::Mainnet => std::env::var("SOROBAN_RPC_MAINNET")
                .unwrap_or_else(|_| DEFAULT_RPC_MAINNET.to_string()),
            Network::Testnet => std::env::var("SOROBAN_RPC_TESTNET")
                .unwrap_or_else(|_| DEFAULT_RPC_TESTNET.to_string()),
            Network::Futurenet => std::env::var("SOROBAN_RPC_FUTURENET")
                .unwrap_or_else(|_| DEFAULT_RPC_FUTURENET.to_string()),
        };
        let timeout_secs = std::env::var("SOROBAN_RPC_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_RPC_TIMEOUT_SECS);
        let max_retries = std::env::var("SOROBAN_RPC_MAX_RETRIES")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_RPC_MAX_RETRIES);

        Self {
            rpc_endpoint,
            timeout: Duration::from_secs(timeout_secs),
            max_retries,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OnChainVerifier {
    client: Client,
}

impl OnChainVerifier {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_RPC_TIMEOUT_SECS))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    pub async fn verify_contract(
        &self,
        cache: &CacheLayer,
        contract: &Contract,
        abi_json: Option<&str>,
    ) -> Result<OnChainVerificationResult, RegistryError> {
        let cache_key = OnChainVerificationResult::cache_key(contract, abi_json);
        if let Some(cached) = cache.get_verification(&cache_key).await {
            let mut parsed: OnChainVerificationResult =
                serde_json::from_str(&cached).map_err(|e| {
                    RegistryError::internal(format!(
                        "Failed to decode cached verification result: {}",
                        e
                    ))
                })?;
            parsed.cached = true;
            return Ok(parsed);
        }

        let config = NetworkConfig::from_env(&contract.network);
        let latest_ledger = self.get_latest_ledger(&config).await.ok();

        let mut warnings = Vec::new();
        let mut failure_reasons: Vec<OnChainFailureReason> = Vec::new();

        let on_chain = match self
            .fetch_contract_instance(&config, &contract.contract_id)
            .await?
        {
            Some(value) => value,
            None => {
                failure_reasons.push(OnChainFailureReason::ContractNotOnChain {
                    contract_id: contract.contract_id.clone(),
                    network: contract.network.to_string(),
                    hint: "Verify the contract address is correct and that it has been deployed \
                           to the specified network."
                        .to_string(),
                });
                let result = OnChainVerificationResult {
                    contract_id: contract.contract_id.clone(),
                    network: contract.network.to_string(),
                    rpc_endpoint: config.rpc_endpoint.clone(),
                    cached: false,
                    contract_exists_on_chain: false,
                    abi_available: abi_json.is_some(),
                    abi_valid: false,
                    recent_call_count: 0,
                    latest_ledger,
                    oldest_ledger: latest_ledger
                        .map(|ledger| ledger.saturating_sub(DEFAULT_ACTIVITY_LOOKBACK_LEDGERS)),
                    on_chain_wasm_hash: None,
                    on_chain_code_hash: None,
                    stored_wasm_hash: contract.wasm_hash.clone(),
                    wasm_hash_matches: false,
                    abi_matches_deployed_contract: false,
                    warnings,
                    failure_reasons,
                };
                cache
                    .put_verification(
                        &cache_key,
                        serde_json::to_string(&result).map_err(|e| {
                            RegistryError::internal(format!(
                                "Failed to encode verification cache entry: {}",
                                e
                            ))
                        })?,
                    )
                    .await;
                return Ok(result);
            }
        };

        let on_chain_wasm_hash = on_chain.1;
        let (on_chain_code_bytes, on_chain_code_hash) = match self
            .fetch_contract_code_entry(&config, &on_chain_wasm_hash)
            .await
        {
            Ok(Some((bytes, hash))) => (Some(bytes), Some(hash)),
            Ok(None) => (None, None),
            Err(err) => {
                warnings.push(err.to_string());
                (None, None)
            }
        };

        let mut abi_available = false;
        let mut abi_valid = false;
        let mut abi_matches_deployed_contract = false;

        if let Some(abi) = abi_json {
            abi_available = true;
            match parse_json_spec(abi, &contract.contract_id) {
                Ok(parsed_abi) => {
                    let stored_function_names: HashSet<_> = parsed_abi
                        .public_functions()
                        .map(|f| f.name.to_ascii_lowercase())
                        .collect();

                    let extracted_function_names = on_chain_code_bytes
                        .as_deref()
                        .map(extract_function_names_from_wasm)
                        .unwrap_or_default();

                    if stored_function_names.is_empty() {
                        warnings.push("Stored ABI contains no public functions".to_string());
                    }

                    if extracted_function_names.is_empty() {
                        warnings.push(
                            "Unable to infer contract functions from on-chain WASM for ABI comparison".to_string(),
                        );
                    }

                    abi_matches_deployed_contract = !stored_function_names.is_empty()
                        && !extracted_function_names.is_empty()
                        && stored_function_names
                            .intersection(&extracted_function_names)
                            .next()
                            .is_some();

                    if !abi_matches_deployed_contract {
                        warnings.push(
                            "Stored ABI did not match inferred on-chain contract functions"
                                .to_string(),
                        );
                    }

                    abi_valid = abi_matches_deployed_contract;
                }
                Err(err) => {
                    warnings.push(format!(
                        "Stored ABI could not be parsed successfully: {}",
                        err
                    ));
                }
            }
        }

        if abi_json.is_some() && on_chain_code_bytes.is_none() {
            warnings.push("On-chain contract code was unavailable for ABI validation".to_string());
        }

        let recent_call_count = match latest_ledger {
            Some(ledger) => match self
                .fetch_recent_activity_count(&config, &contract.contract_id, ledger)
                .await
            {
                Ok(count) => count,
                Err(err) => {
                    warnings.push(err.to_string());
                    0
                }
            },
            None => {
                warnings.push(
                    "latest ledger unavailable; recent call history could not be checked"
                        .to_string(),
                );
                0
            }
        };

        let wasm_hash_matches = contract.wasm_hash.eq_ignore_ascii_case(&on_chain_wasm_hash)
            || on_chain_code_hash
                .as_deref()
                .map(|hash| contract.wasm_hash.eq_ignore_ascii_case(hash))
                .unwrap_or(false);

        if !wasm_hash_matches {
            failure_reasons.push(OnChainFailureReason::WasmHashMismatch {
                stored: contract.wasm_hash.clone(),
                on_chain: on_chain_wasm_hash.clone(),
                hint: "Ensure the registry entry references the correct deployed wasm hash. \
                       Re-publish the contract if the wasm was updated on-chain."
                    .to_string(),
            });
        }

        let result = OnChainVerificationResult {
            contract_id: contract.contract_id.clone(),
            network: contract.network.to_string(),
            rpc_endpoint: config.rpc_endpoint.clone(),
            cached: false,
            contract_exists_on_chain: true,
            abi_available,
            abi_valid,
            recent_call_count,
            latest_ledger,
            oldest_ledger: latest_ledger
                .map(|ledger| ledger.saturating_sub(DEFAULT_ACTIVITY_LOOKBACK_LEDGERS)),
            on_chain_wasm_hash: Some(on_chain_wasm_hash),
            on_chain_code_hash,
            stored_wasm_hash: contract.wasm_hash.clone(),
            wasm_hash_matches,
            abi_matches_deployed_contract,
            warnings,
            failure_reasons,
        };

        cache
            .put_verification(
                &cache_key,
                serde_json::to_string(&result).map_err(|e| {
                    RegistryError::internal(format!(
                        "Failed to encode verification cache entry: {}",
                        e
                    ))
                })?,
            )
            .await;

        Ok(result)
    }

    async fn fetch_contract_instance(
        &self,
        config: &NetworkConfig,
        contract_id: &str,
    ) -> Result<Option<(ScContractInstance, String)>, RegistryError> {
        let key = build_contract_instance_ledger_key(contract_id)?;
        let response = self
            .rpc_call::<GetLedgerEntriesResult>(
                config,
                "getLedgerEntries",
                serde_json::json!({
                    "keys": [key],
                    "xdrFormat": "base64"
                }),
            )
            .await?;

        let Some(entry) = response.entries.into_iter().next() else {
            return Ok(None);
        };

        let ledger_entry =
            LedgerEntry::from_xdr_base64(&entry.xdr, Limits::none()).map_err(|e| {
                RegistryError::stellar_rpc(format!("Failed to decode contract ledger entry: {}", e))
            })?;

        let LedgerEntryData::ContractData(contract_data) = ledger_entry.data else {
            return Err(RegistryError::stellar_rpc(
                "Unexpected ledger entry type for contract instance".to_string(),
            ));
        };

        let ScVal::ContractInstance(instance) = contract_data.val else {
            return Err(RegistryError::stellar_rpc(
                "Contract instance ledger entry did not contain a contract instance value"
                    .to_string(),
            ));
        };

        let ContractExecutable::Wasm(hash) = instance.executable.clone() else {
            return Err(RegistryError::stellar_rpc(
                "Contract executable is not a WASM contract".to_string(),
            ));
        };

        Ok(Some((instance, hex::encode(hash.0))))
    }

    async fn fetch_contract_code_entry(
        &self,
        config: &NetworkConfig,
        wasm_hash: &str,
    ) -> Result<Option<(Vec<u8>, String)>, RegistryError> {
        let key = build_contract_code_ledger_key(wasm_hash)?;
        let response = self
            .rpc_call::<GetLedgerEntriesResult>(
                config,
                "getLedgerEntries",
                serde_json::json!({
                    "keys": [key],
                    "xdrFormat": "base64"
                }),
            )
            .await?;

        let Some(entry) = response.entries.into_iter().next() else {
            return Ok(None);
        };

        let ledger_entry =
            LedgerEntry::from_xdr_base64(&entry.xdr, Limits::none()).map_err(|e| {
                RegistryError::stellar_rpc(format!(
                    "Failed to decode contract code ledger entry: {}",
                    e
                ))
            })?;
        let LedgerEntryData::ContractCode(ContractCodeEntry { code, .. }) = ledger_entry.data
        else {
            return Err(RegistryError::stellar_rpc(
                "Unexpected ledger entry type for contract code".to_string(),
            ));
        };

        let bytes = code.as_slice().to_vec();
        let hash = verifier::hash_wasm(bytes.as_slice());
        Ok(Some((bytes, hash)))
    }

    async fn fetch_recent_activity_count(
        &self,
        config: &NetworkConfig,
        contract_id: &str,
        latest_ledger: u32,
    ) -> Result<usize, RegistryError> {
        let start_ledger = latest_ledger.saturating_sub(DEFAULT_ACTIVITY_LOOKBACK_LEDGERS);

        match self
            .rpc_call::<GetEventsResult>(
                config,
                "getEvents",
                serde_json::json!({
                    "startLedger": start_ledger,
                    "filters": [{
                        "type": "contract",
                        "contractIds": [contract_id]
                    }],
                    "pagination": {
                        "limit": DEFAULT_ACTIVITY_LIMIT
                    }
                }),
            )
            .await
        {
            Ok(result) => Ok(result.events.len()),
            Err(events_err) => {
                let fallback = self
                    .rpc_call::<GetTransactionsResult>(
                        config,
                        "getTransactions",
                        serde_json::json!({
                            "startLedger": start_ledger,
                            "pagination": {
                                "limit": DEFAULT_ACTIVITY_LIMIT
                            },
                            "xdrFormat": "json"
                        }),
                    )
                    .await?;
                let count = fallback
                    .transactions
                    .into_iter()
                    .filter(|tx| {
                        serde_json::to_string(&tx.events)
                            .unwrap_or_default()
                            .contains(contract_id)
                    })
                    .count();
                if count == 0 {
                    return Err(RegistryError::stellar_rpc(format!(
                        "event lookup failed ({}); transaction fallback found no recent calls",
                        events_err
                    )));
                }
                Ok(count)
            }
        }
    }

    async fn get_latest_ledger(&self, config: &NetworkConfig) -> Result<u32, RegistryError> {
        let response = self
            .rpc_call::<GetLatestLedgerResult>(config, "getLatestLedger", serde_json::json!({}))
            .await?;
        Ok(response.sequence)
    }

    async fn rpc_call<T: for<'de> Deserialize<'de>>(
        &self,
        config: &NetworkConfig,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, RegistryError> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": method,
            "method": method,
            "params": params
        });

        let mut delay_ms = 200_u64;
        for attempt in 0..config.max_retries {
            let mut headers = reqwest::header::HeaderMap::new();
            crate::request_tracing::inject_current_trace_context(&mut headers);
            let response = self
                .client
                .post(&config.rpc_endpoint)
                .timeout(config.timeout)
                .headers(headers)
                .json(&payload)
                .send()
                .await;

            match response {
                Ok(response) => {
                    let status = response.status();
                    let value: RpcEnvelope<T> = response.json().await.map_err(|e| {
                        RegistryError::stellar_rpc(format!(
                            "Failed to parse {} response: {}",
                            method, e
                        ))
                    })?;

                    if let Some(result) = value.result {
                        return Ok(result);
                    }

                    let rpc_error = value
                        .error
                        .map(|err| err.message)
                        .unwrap_or_else(|| format!("HTTP {} returned an empty error body", status));
                    if attempt + 1 >= config.max_retries {
                        return Err(RegistryError::stellar_rpc(format!(
                            "{} failed after {} attempts: {}",
                            method,
                            attempt + 1,
                            rpc_error
                        )));
                    }
                    continue;
                }
                Err(err) => {
                    if attempt + 1 >= config.max_retries {
                        return Err(RegistryError::stellar_rpc(format!(
                            "{} network request failed after {} attempts: {}",
                            method,
                            attempt + 1,
                            err
                        )));
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            delay_ms = (delay_ms * 2).min(2_000);
        }

        Err(RegistryError::stellar_rpc(format!(
            "{} failed without returning a result",
            method
        )))
    }
}

fn build_contract_instance_ledger_key(contract_id: &str) -> Result<String, RegistryError> {
    let contract = parse_contract_strkey(contract_id)?;
    let key = LedgerKey::ContractData(LedgerKeyContractData {
        contract: ScAddress::Contract(ContractId(Hash(contract.0))),
        key: ScVal::LedgerKeyContractInstance,
        durability: ContractDataDurability::Persistent,
    });
    key.to_xdr_base64(Limits::none()).map_err(|e| {
        RegistryError::internal(format!("Failed to encode contract ledger key: {}", e))
    })
}

fn build_contract_code_ledger_key(wasm_hash: &str) -> Result<String, RegistryError> {
    let normalized = verifier::normalize_hash(wasm_hash)
        .ok_or_else(|| RegistryError::invalid_input("Invalid on-chain wasm hash".to_string()))?;
    let bytes = hex::decode(normalized)
        .map_err(|e| RegistryError::invalid_input(format!("Invalid wasm hash hex: {}", e)))?;
    let mut hash = [0_u8; 32];
    hash.copy_from_slice(&bytes);
    let key = LedgerKey::ContractCode(LedgerKeyContractCode { hash: Hash(hash) });
    key.to_xdr_base64(Limits::none())
        .map_err(|e| RegistryError::internal(format!("Failed to encode contract code key: {}", e)))
}

fn extract_function_names_from_wasm(wasm_bytes: &[u8]) -> HashSet<String> {
    simulation::extract_abi(wasm_bytes)
        .functions
        .into_iter()
        .map(|func| func.name.to_ascii_lowercase())
        .collect()
}

fn parse_contract_strkey(contract_id: &str) -> Result<ContractStrkey, RegistryError> {
    match Strkey::from_string(contract_id)
        .map_err(|e| RegistryError::invalid_input(format!("Invalid contract address: {}", e)))?
    {
        Strkey::Contract(contract) => Ok(contract),
        _ => Err(RegistryError::invalid_input(
            "contract_id must be a Stellar contract address".to_string(),
        )),
    }
}

#[derive(Debug, Deserialize)]
struct RpcEnvelope<T> {
    result: Option<T>,
    error: Option<RpcErrorBody>,
}

#[derive(Debug, Deserialize)]
struct RpcErrorBody {
    message: String,
}

#[derive(Debug, Deserialize)]
struct GetLatestLedgerResult {
    sequence: u32,
}

#[derive(Debug, Deserialize)]
struct GetLedgerEntriesResult {
    entries: Vec<LedgerEntryResponse>,
}

#[derive(Debug, Deserialize)]
struct LedgerEntryResponse {
    xdr: String,
}

#[derive(Debug, Deserialize)]
struct GetEventsResult {
    #[serde(default)]
    events: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GetTransactionsResult {
    #[serde(default)]
    transactions: Vec<TransactionResponse>,
}

#[derive(Debug, Deserialize)]
struct TransactionResponse {
    #[serde(default)]
    events: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_contract(network: Network, wasm_hash: &str) -> Contract {
        Contract {
            id: uuid::Uuid::nil(),
            contract_id: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4".to_string(),
            wasm_hash: wasm_hash.to_string(),
            name: "demo".to_string(),
            slug: "demo".to_string(),
            description: None,
            publisher_id: uuid::Uuid::nil(),
            network,
            is_verified: false,
            verification_status: shared::VerificationStatus::Unverified,
            category: None,
            tags: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            verified_at: None,
            deployed_at: None,
            verified_by: None,
            verification_notes: None,
            health_score: 0,
            is_maintenance: false,
            last_accessed_at: None,
            logical_id: None,
            network_configs: None,
            relevance_score: None,
            organization_id: None,
            visibility: shared::VisibilityType::Public,
            current_version: None,
            usage_count: 0,
        }
    }

    #[test]
    fn code_key_requires_valid_hash() {
        let result = build_contract_code_ledger_key("not-a-hash");
        assert!(result.is_err());
    }

    #[test]
    fn cache_key_is_network_specific() {
        let contract = dummy_contract(Network::Testnet, "abc123");
        assert_eq!(
            OnChainVerificationResult::cache_key(&contract, None),
            "onchain:testnet:CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4:abc123"
        );
    }

    #[test]
    fn cache_key_changes_when_abi_changes() {
        let contract = Contract {
            id: uuid::Uuid::nil(),
            contract_id: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4".to_string(),
            wasm_hash: "abc123".to_string(),
            name: "demo".to_string(),
            slug: "demo".to_string(),
            description: None,
            publisher_id: uuid::Uuid::nil(),
            network: Network::Testnet,
            is_verified: false,
            verification_status: shared::VerificationStatus::Unverified,
            category: None,
            tags: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            verified_at: None,
            deployed_at: None,
            verified_by: None,
            verification_notes: None,
            health_score: 0,
            is_maintenance: false,
            last_accessed_at: None,
            logical_id: None,
            network_configs: None,
            relevance_score: None,
            organization_id: None,
            visibility: shared::VisibilityType::Public,
            current_version: None,
            usage_count: 0,
        };

        let key_no_abi = OnChainVerificationResult::cache_key(&contract, None);
        let key_with_abi = OnChainVerificationResult::cache_key(&contract, Some("[]"));
        assert_ne!(key_no_abi, key_with_abi);
    }

    #[test]
    fn wasm_function_extraction_finds_matching_symbols() {
        let bytes = b"random bytes transfer balance";
        let functions = extract_function_names_from_wasm(bytes);
        assert!(functions.contains("transfer"));
        assert!(functions.contains("balance"));
    }
}
