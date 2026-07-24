pub mod compatibility;
pub mod contract_metadata;
pub mod reviews;
pub mod validators;

use crate::validation::extractors::ValidatedJson;
use crate::validation::handler_requests::BatchContractIdsRequest;
use axum::{
    body::Body,
    extract::{
        rejection::{JsonRejection, QueryRejection},
        Path, Query, State,
    },
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use flate2::{write::GzEncoder, Compression};
use once_cell::sync::Lazy;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use shared::models::SourceFormat;
use shared::{
    pagination::Cursor, AdvancedSearchRequest, AnalyticsEventType, AuditActionType,
    ChangePublisherRequest, Contract, ContractAuditLog, ContractDeploymentHistory,
    ContractExportAcceptedResponse, ContractExportFormat, ContractExportJobStatus,
    ContractExportMetadata, ContractExportRequest, ContractExportStatusResponse,
    ContractGetResponse, ContractInteractionResponse, ContractMetadataExportEnvelope,
    ContractMetadataExportRecord, ContractSearchParams, ContractSource, ContractVersion,
    CreateContractVersionRequest, CreateInteractionBatchRequest, CreateInteractionRequest,
    DeploymentHistoryQueryParams, FavoriteSearch, FieldOperator, GraphResponse,
    InteractionTimeSeriesPoint, InteractionTimeSeriesResponse, InteractionsListResponse,
    InteractionsQueryParams, Network, NetworkConfig, NetworkEndpoints, NetworkHealth,
    NetworkHealthResponse, NetworkInfo, NetworkListResponse, NetworkStatus,
    PaginatedAuditsResponse, PaginatedResponse, PublishRequest, Publisher, QueryCondition,
    QueryNode, QueryOperator, SaveFavoriteSearchRequest, SearchSuggestion,
    SearchSuggestionsResponse, SemVer, TrendingParams, UpdateContractMetadataRequest,
    UpdateContractStatusRequest, VerifyRequest,
};

// ────────────────────────────────────────────────────────────────────────────
// NOTE: All types are now imported from the shared crate.
// Duplicate definitions have been removed to maintain a single source of truth.
// ────────────────────────────────────────────────────────────────────────────
use sqlx::{error::DatabaseError, Postgres, QueryBuilder};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;
use std::path::{Path as StdPath, PathBuf};
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Query params for GET /contracts/:id (Issue #43)
#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct GetContractQuery {
    pub network: Option<Network>,
    pub from_search: Option<bool>,
    pub search_query: Option<String>,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct BatchContractsQuery {
    /// Comma-separated list of fields to include in each contract result.
    /// Example: fields=name,address,network
    pub fields: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ContractStatsResponse {
    pub contract_id: Uuid,
    pub usage_count: i64,
    pub last_accessed_at: Option<chrono::DateTime<chrono::Utc>>,
}

use crate::{
    analytics,
    breaking_changes::{diff_abi, has_breaking_changes, resolve_abi},
    contract_events::ContractEventEnvelope,
    dependency,
    error::{ApiError, ApiResult},
    onchain_verification::OnChainVerifier,
    state::{AppState, ContractEventVisibility},
};

use contract_abi::{generate_openapi, parse_json_spec, to_json, to_yaml};

pub(crate) fn db_internal_error(operation: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = operation, error = ?err, "database operation failed");
    if let sqlx::Error::Database(db_err) = &err {
        // Bind the Cow returned by code() before borrowing it, so the temporary
        // outlives the `match code` below (fixes E0716: temporary dropped while borrowed).
        let code = db_err.code();
        let code = code.as_deref();
        let constraint = db_err.constraint();
        let message = database_constraint_message(operation, constraint, db_err.message());

        return match code {
            Some("23505") => ApiError::conflict("DatabaseConstraintViolation", message),
            Some("23502") | Some("23503") | Some("23514") | Some("22001") | Some("P0001") => {
                ApiError::bad_request("DatabaseConstraintViolation", message)
            }
            _ => ApiError::internal(message),
        };
    }

    ApiError::internal("An unexpected database error occurred")
}

fn database_constraint_message(
    operation: &str,
    constraint: Option<&str>,
    db_message: &str,
) -> String {
    match constraint.unwrap_or("") {
        "chk_publishers_stellar_address_format" => {
            "stellar_address must be a valid Stellar address".to_string()
        }
        "chk_publishers_email_format" => "email must be a valid email address".to_string(),
        "chk_publishers_github_url_format" => {
            "github_url must be a valid http:// or https:// URL".to_string()
        }
        "chk_publishers_website_url_format" => {
            "website must be a valid http:// or https:// URL".to_string()
        }
        "chk_contracts_contract_id_format" => {
            "contract_id must be a valid Stellar contract ID".to_string()
        }
        "chk_contracts_wasm_hash_format" => {
            "wasm_hash must be a 64-character hexadecimal hash".to_string()
        }
        "chk_contracts_slug_format" => {
            "slug must use lowercase letters, numbers, and hyphens".to_string()
        }
        "chk_contracts_health_score_range" => "health_score must be between 0 and 100".to_string(),
        "chk_contracts_deployment_count_non_negative" => {
            "deployment_count must be zero or greater".to_string()
        }
        "chk_contracts_network_configs_object" => {
            "network_configs must be a JSON object".to_string()
        }
        "chk_contracts_current_version_length" => "current_version is too long".to_string(),
        "chk_contributors_stellar_address_format" => {
            "contributor stellar_address must be a valid Stellar address".to_string()
        }
        "chk_contributors_name_length" => "contributor name is too long".to_string(),
        "chk_tags_prefix_not_blank" => "tag prefix cannot be blank".to_string(),
        "chk_tags_name_not_blank" => "tag name cannot be blank".to_string(),
        "chk_tags_usage_count_non_negative" => {
            "tag usage_count must be zero or greater".to_string()
        }
        "chk_tag_aliases_alias_not_blank" => "tag alias cannot be blank".to_string(),
        "chk_tag_usage_log_usage_count_non_negative" => {
            "tag usage log count must be zero or greater".to_string()
        }
        "chk_organizations_name_not_blank" => "organization name cannot be blank".to_string(),
        "chk_organizations_slug_format" => {
            "organization slug must use lowercase letters, numbers, and hyphens".to_string()
        }
        "chk_organizations_quota_contracts_positive" => {
            "quota_contracts must be greater than zero".to_string()
        }
        "chk_organizations_rate_limit_requests_positive" => {
            "rate_limit_requests must be greater than zero".to_string()
        }
        "chk_contract_versions_signature_algorithm" => {
            "signature_algorithm must be ed25519 when provided".to_string()
        }
        "chk_contract_versions_commit_hash_format" => {
            "commit_hash must be a 40-character hexadecimal hash".to_string()
        }
        "chk_contract_versions_source_url_format" => {
            "source_url must be a valid http:// or https:// URL".to_string()
        }
        "chk_reviews_review_text_length" => "review_text is too long".to_string(),
        "chk_reviews_version_length" => "review version is too long".to_string(),
        "chk_verifications_error_message_length" => "error_message is too long".to_string(),
        "chk_verifications_compiler_version_length" => "compiler_version is too long".to_string(),
        "chk_organization_invitations_email_format" => {
            "organization invitation email must be valid".to_string()
        }
        "chk_organization_invitations_time_order" => {
            "expires_at must be later than created_at".to_string()
        }
        "chk_organization_invitations_accepted_at_order" => {
            "accepted_at must be null or later than created_at".to_string()
        }
        "chk_user_preferences_theme" => "theme must be one of: dark, light, system".to_string(),
        "chk_user_preferences_language_not_blank" => {
            "language must be between 2 and 10 characters".to_string()
        }
        "chk_user_preferences_favorites_json_array" => {
            "favorites must be stored as a JSON array".to_string()
        }
        "chk_user_preferences_extensible_settings_json_object" => {
            "extensible_settings must be stored as a JSON object".to_string()
        }
        "chk_user_preferences_webhook_url_format" => {
            "webhook_url must be a valid http:// or https:// URL".to_string()
        }
        "chk_notification_queue_status" => "notification queue status is invalid".to_string(),
        "chk_notification_queue_priority_range" => {
            "notification queue priority must be between 1 and 10".to_string()
        }
        "chk_notification_queue_retry_count_non_negative" => {
            "retry_count must be zero or greater".to_string()
        }
        "chk_notification_queue_max_retries_non_negative" => {
            "max_retries must be zero or greater".to_string()
        }
        "chk_notification_delivery_logs_status" => {
            "notification delivery status is invalid".to_string()
        }
        "chk_contract_version_integrity" => "contract version data is inconsistent".to_string(),
        "chk_verification_integrity" => "verification data is inconsistent".to_string(),
        _ => {
            if db_message.trim().is_empty() {
                format!("{} failed due to a database constraint", operation)
            } else {
                db_message.to_string()
            }
        }
    }
}

fn maturity_filter_value(maturity: &shared::MaturityLevel) -> &'static str {
    match maturity {
        shared::MaturityLevel::Experimental => "experimental",
        shared::MaturityLevel::Beta => "beta",
        shared::MaturityLevel::Stable => "stable",
        shared::MaturityLevel::Production => "production",
    }
}

pub(crate) async fn fetch_contract_identity(
    state: &AppState,
    id: &str,
) -> ApiResult<(Uuid, String)> {
    if let Ok(uuid) = Uuid::parse_str(id) {
        let (contract_id,): (String,) =
            sqlx::query_as("SELECT contract_id FROM contracts WHERE id = $1")
                .bind(uuid)
                .fetch_one(&state.db)
                .await
                .map_err(|err| match err {
                    sqlx::Error::RowNotFound => {
                        ApiError::not_found("ContractNotFound", "Contract not found")
                    }
                    _ => db_internal_error("fetch contract identity by uuid", err),
                })?;
        return Ok((uuid, contract_id));
    }

    let (uuid, contract_id): (Uuid, String) =
        sqlx::query_as("SELECT id, contract_id FROM contracts WHERE contract_id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => {
                    ApiError::not_found("ContractNotFound", "Contract not found")
                }
                _ => db_internal_error("fetch contract identity by address", err),
            })?;
    Ok((uuid, contract_id))
}

#[derive(Debug, sqlx::FromRow)]
struct MultisigProposalValidation {
    contract_id: String,
    status: String,
    expires_at: chrono::DateTime<chrono::Utc>,
    required_approvals: i32,
    approved_signatures: i64,
}

async fn require_multisig_approval_for_sensitive_update(
    state: &AppState,
    headers: &HeaderMap,
    contract: &Contract,
    action_label: &str,
) -> ApiResult<()> {
    let proposal_id = headers
        .get("x-multisig-proposal-id")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            ApiError::forbidden_with_error(
                "MultisigRequired",
                format!(
                    "{} requires multisig approval; include x-multisig-proposal-id",
                    action_label
                ),
            )
        })
        .and_then(|raw| {
            Uuid::parse_str(raw)
                .map_err(|_| ApiError::bad_request("InvalidProposalId", "invalid proposal id"))
        })?;

    let proposal = sqlx::query_as::<_, MultisigProposalValidation>(
        "SELECT
            p.contract_id,
            p.status::text AS status,
            p.expires_at,
            p.required_approvals,
            COALESCE((
                SELECT COUNT(*)
                FROM proposal_signatures s
                WHERE s.proposal_id = p.id AND s.decision = 'approved'
            ), 0)::BIGINT AS approved_signatures
         FROM deploy_proposals p
         WHERE p.id = $1",
    )
    .bind(proposal_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("validate multisig proposal", err))?
    .ok_or_else(|| ApiError::not_found("ProposalNotFound", "multisig proposal not found"))?;

    if proposal.contract_id != contract.contract_id {
        return Err(ApiError::conflict(
            "ProposalContractMismatch",
            "multisig proposal does not match this contract",
        ));
    }

    if proposal.expires_at <= chrono::Utc::now() {
        return Err(ApiError::conflict(
            "ProposalExpired",
            "multisig proposal has expired",
        ));
    }

    if proposal.status != "approved" && proposal.status != "executed" {
        return Err(ApiError::conflict(
            "ProposalNotApproved",
            "multisig proposal must be approved before performing this update",
        ));
    }

    if proposal.approved_signatures < i64::from(proposal.required_approvals) {
        return Err(ApiError::forbidden_with_error(
            "ThresholdNotMet",
            "required multisig signature threshold has not been met",
        ));
    }

    Ok(())
}

#[allow(dead_code)]
fn map_json_rejection(err: JsonRejection) -> ApiError {
    ApiError::bad_request_with(
        "InvalidRequest",
        format!("Invalid JSON payload: {}", err.body_text()),
    )
}

fn map_query_rejection(err: QueryRejection) -> ApiError {
    ApiError::bad_request_with(
        "InvalidQuery",
        format!("Invalid query parameters: {}", err.body_text()),
    )
}

#[allow(dead_code)]
fn sort_timestamp_column(sort_by: &shared::SortBy) -> Option<&'static str> {
    match sort_by {
        shared::SortBy::CreatedAt => Some("c.created_at"),
        shared::SortBy::UpdatedAt => Some("c.updated_at"),
        shared::SortBy::VerifiedAt => Some("c.verified_at"),
        shared::SortBy::LastAccessedAt => Some("c.last_accessed_at"),
        _ => None,
    }
}

fn contract_timestamp_for_sort(
    contract: &Contract,
    sort_by: &shared::SortBy,
) -> Option<chrono::DateTime<chrono::Utc>> {
    match sort_by {
        shared::SortBy::CreatedAt => Some(contract.created_at),
        shared::SortBy::UpdatedAt => Some(contract.updated_at),
        shared::SortBy::VerifiedAt => contract.verified_at,
        shared::SortBy::LastAccessedAt => contract.last_accessed_at,
        _ => None,
    }
}

pub(crate) async fn track_contract_access(state: &AppState, contract_id: Uuid) {
    let cache_key = contract_id.to_string();
    if !state.cache.should_refresh_contract_access(&cache_key).await {
        return;
    }

    let db = state.db.clone();
    tokio::spawn(async move {
        let _ = sqlx::query("UPDATE contracts SET last_accessed_at = NOW() WHERE id = $1")
            .bind(contract_id)
            .execute(&db)
            .await;
    });
}

const NETWORKS_CACHE_NAMESPACE: &str = "system";
const NETWORKS_CACHE_KEY: &str = "network_catalog";
const NETWORKS_V1_CACHE_KEY: &str = "network_catalog_v1";
const NETWORKS_REFRESH_INTERVAL_SECS: u64 = 60;
const NETWORKS_V1_CACHE_TTL_SECS: i64 = 15 * 60;
const NETWORKS_HEALTH_CACHE_NAMESPACE: &str = "health";
const NETWORKS_HEALTH_CACHE_KEY: &str = "all_networks";
const NETWORKS_HEALTH_TTL: u64 = 30;
const NETWORK_DEGRADED_FAILURE_THRESHOLD: i32 = 1;
const NETWORK_OFFLINE_FAILURE_THRESHOLD: i32 = 5;
const NETWORK_STALE_AFTER_MINUTES: i64 = 10;
const SLOW_SEARCH_QUERY_THRESHOLD_MS: u128 = 200;
const ASYNC_EXPORT_ROW_THRESHOLD: i64 = 1_000;
const EXPORT_ARTIFACT_DIR: &str = "soroban-registry-exports";

#[derive(Debug, Clone)]
struct ContractExportJob {
    job_id: Uuid,
    status: ContractExportJobStatus,
    format: ContractExportFormat,
    filters: ContractSearchParams,
    total_count: i64,
    requested_at: chrono::DateTime<chrono::Utc>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    artifact_path: Option<PathBuf>,
    error: Option<String>,
}

static EXPORT_JOBS: Lazy<RwLock<HashMap<Uuid, ContractExportJob>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

fn observe_search_query(
    kind: &str,
    started_at: std::time::Instant,
    query: Option<&str>,
    limit: i64,
) {
    let elapsed = started_at.elapsed();
    crate::metrics::SEARCH_QUERY_DURATION
        .with_label_values(&[kind])
        .observe(elapsed.as_secs_f64());

    if elapsed.as_millis() > SLOW_SEARCH_QUERY_THRESHOLD_MS {
        crate::metrics::SEARCH_SLOW_QUERIES
            .with_label_values(&[kind])
            .inc();
        tracing::warn!(
            query_type = kind,
            duration_ms = elapsed.as_millis(),
            query = query.unwrap_or(""),
            limit = limit,
            "slow search query detected"
        );
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct IndexerStateSnapshot {
    last_indexed_ledger_height: i64,
    indexed_at: chrono::DateTime<chrono::Utc>,
    consecutive_failures: i32,
    error_message: Option<String>,
}

#[derive(Debug, Clone)]
struct StaticNetworkDefinition {
    id: &'static str,
    name: &'static str,
    network_type: Network,
    chain_id: &'static str,
    description: &'static str,
    logo_url: &'static str,
    avg_block_time_seconds: f64,
    rpc_url: String,
    explorer_url: String,
    friendbot_url: Option<String>,
}

fn default_rpc_url(network: &Network) -> &'static str {
    match network {
        Network::Mainnet => "https://rpc-mainnet.stellar.org",
        Network::Testnet => "https://rpc-testnet.stellar.org",
        Network::Futurenet => "https://rpc-futurenet.stellar.org",
    }
}

fn default_explorer_url(network: &Network) -> &'static str {
    match network {
        Network::Mainnet => "https://stellar.expert/explorer/public",
        Network::Testnet => "https://stellar.expert/explorer/testnet",
        Network::Futurenet => "https://stellar.expert/explorer/futurenet",
    }
}

fn default_friendbot_url(network: &Network) -> Option<&'static str> {
    match network {
        Network::Mainnet => None,
        Network::Testnet => Some("https://friendbot.stellar.org"),
        Network::Futurenet => Some("https://friendbot-futurenet.stellar.org"),
    }
}

fn configured_networks() -> Vec<StaticNetworkDefinition> {
    let entries = [
        (
            "mainnet",
            "Stellar Mainnet",
            Network::Mainnet,
            "Public Global Stellar Network ; September 2015",
            "Production Stellar public network for live Soroban contracts and assets.",
            "https://assets.stellar.org/logos/stellar-logo.svg",
            5.0,
            "STELLAR_RPC_MAINNET",
            "STELLAR_EXPLORER_MAINNET",
            "STELLAR_FRIENDBOT_MAINNET",
        ),
        (
            "testnet",
            "Stellar Testnet",
            Network::Testnet,
            "Test SDF Network ; September 2015",
            "Public resettable Stellar test network for integration testing.",
            "https://assets.stellar.org/logos/stellar-logo.svg",
            5.0,
            "STELLAR_RPC_TESTNET",
            "STELLAR_EXPLORER_TESTNET",
            "STELLAR_FRIENDBOT_TESTNET",
        ),
        (
            "futurenet",
            "Stellar Futurenet",
            Network::Futurenet,
            "Test SDF Future Network ; October 2022",
            "Preview network for upcoming Stellar and Soroban protocol changes.",
            "https://assets.stellar.org/logos/stellar-logo.svg",
            5.0,
            "STELLAR_RPC_FUTURENET",
            "STELLAR_EXPLORER_FUTURENET",
            "STELLAR_FRIENDBOT_FUTURENET",
        ),
    ];

    entries
        .into_iter()
        .map(
            |(
                id,
                name,
                network_type,
                chain_id,
                description,
                logo_url,
                avg_block_time_seconds,
                rpc_env,
                explorer_env,
                friendbot_env,
            )| {
                StaticNetworkDefinition {
                    id,
                    name,
                    chain_id,
                    description,
                    logo_url,
                    avg_block_time_seconds,
                    rpc_url: std::env::var(rpc_env)
                        .unwrap_or_else(|_| default_rpc_url(&network_type).to_string()),
                    explorer_url: std::env::var(explorer_env)
                        .unwrap_or_else(|_| default_explorer_url(&network_type).to_string()),
                    friendbot_url: std::env::var(friendbot_env)
                        .ok()
                        .or_else(|| default_friendbot_url(&network_type).map(str::to_string)),
                    network_type,
                }
            },
        )
        .collect()
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct NetworkListV1Query {
    #[serde(default, rename = "real-time", alias = "real_time")]
    pub real_time: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum NetworkStatusIndicator {
    Healthy,
    Degraded,
    Maintenance,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct NetworkStatsV1 {
    pub contract_count: i64,
    pub transaction_volume_24h: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct NetworkHealthMetricsV1 {
    pub avg_block_time_seconds: f64,
    pub validator_count: i64,
    pub last_indexed_ledger_height: Option<i64>,
    pub last_indexed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub consecutive_failures: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct NetworkInfoV1 {
    pub name: String,
    pub chain_id: String,
    pub network: Network,
    pub rpc_endpoint: String,
    pub status: NetworkStatusIndicator,
    pub description: String,
    pub logo_url: String,
    pub explorer_url: String,
    pub friendbot_url: Option<String>,
    pub stats: NetworkStatsV1,
    pub health: NetworkHealthMetricsV1,
    pub last_checked_at: chrono::DateTime<chrono::Utc>,
    pub status_message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct NetworkListResponseV1 {
    pub networks: Vec<NetworkInfoV1>,
    pub cached_at: chrono::DateTime<chrono::Utc>,
    pub cache_ttl_seconds: i64,
    pub real_time: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct NetworkStatsRow {
    network: Network,
    contract_count: i64,
    transaction_volume_24h: i64,
}

fn network_maintenance_override(network_id: &str) -> bool {
    let env_name = format!("STELLAR_NETWORK_{}_MAINTENANCE", network_id.to_uppercase());
    std::env::var(env_name)
        .map(|value| matches!(value.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

fn status_indicator(network_id: &str, status: &NetworkStatus) -> NetworkStatusIndicator {
    if network_maintenance_override(network_id) {
        return NetworkStatusIndicator::Maintenance;
    }

    match status {
        NetworkStatus::Online => NetworkStatusIndicator::Healthy,
        NetworkStatus::Degraded => NetworkStatusIndicator::Degraded,
        NetworkStatus::Offline => NetworkStatusIndicator::Maintenance,
    }
}

fn derive_network_status(
    rpc_healthy: bool,
    snapshot: Option<&IndexerStateSnapshot>,
    now: chrono::DateTime<chrono::Utc>,
) -> (NetworkStatus, Option<String>) {
    if !rpc_healthy {
        return (
            NetworkStatus::Offline,
            Some("RPC health check failed".to_string()),
        );
    }

    if let Some(snapshot) = snapshot {
        let stale =
            now - snapshot.indexed_at > chrono::Duration::minutes(NETWORK_STALE_AFTER_MINUTES);

        if snapshot.consecutive_failures >= NETWORK_OFFLINE_FAILURE_THRESHOLD {
            return (
                NetworkStatus::Offline,
                snapshot.error_message.clone().or_else(|| {
                    Some(format!(
                        "{} consecutive indexer failures",
                        snapshot.consecutive_failures
                    ))
                }),
            );
        }

        if stale || snapshot.consecutive_failures >= NETWORK_DEGRADED_FAILURE_THRESHOLD {
            return (
                NetworkStatus::Degraded,
                if stale {
                    Some("Indexer status is stale".to_string())
                } else {
                    snapshot.error_message.clone().or_else(|| {
                        Some(format!(
                            "{} consecutive indexer failures",
                            snapshot.consecutive_failures
                        ))
                    })
                },
            );
        }
    }

    (NetworkStatus::Online, None)
}

async fn probe_network_health(client: &reqwest::Client, health_url: &str) -> bool {
    let mut request = client.get(health_url);
    let mut headers = reqwest::header::HeaderMap::new();
    crate::request_tracing::inject_current_trace_context(&mut headers);
    request = request.headers(headers);

    match request.send().await {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

async fn fetch_network_catalog(db: &sqlx::PgPool) -> Result<NetworkListResponse, sqlx::Error> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let now = chrono::Utc::now();

    let mut networks = Vec::new();
    for definition in configured_networks() {
        let health_url = format!("{}/health", definition.rpc_url.trim_end_matches('/'));
        let snapshot: Option<IndexerStateSnapshot> = sqlx::query_as(
            "SELECT last_indexed_ledger_height, indexed_at, consecutive_failures, error_message
             FROM indexer_state
             WHERE network = $1",
        )
        .bind(&definition.network_type)
        .fetch_optional(db)
        .await?;

        let rpc_healthy = probe_network_health(&client, &health_url).await;
        let (status, status_message) = derive_network_status(rpc_healthy, snapshot.as_ref(), now);

        networks.push(NetworkInfo {
            id: definition.id.to_string(),
            name: definition.name.to_string(),
            network_type: definition.network_type,
            status,
            endpoints: NetworkEndpoints {
                rpc_url: definition.rpc_url,
                health_url,
                explorer_url: definition.explorer_url,
                friendbot_url: definition.friendbot_url,
            },
            last_checked_at: now,
            last_indexed_ledger_height: snapshot.as_ref().map(|s| s.last_indexed_ledger_height),
            last_indexed_at: snapshot.as_ref().map(|s| s.indexed_at),
            consecutive_failures: snapshot
                .as_ref()
                .map(|s| s.consecutive_failures)
                .unwrap_or(0),
            status_message,
        });
    }

    Ok(NetworkListResponse {
        networks,
        cached_at: now,
    })
}

async fn refresh_network_catalog_cache(state: &AppState) -> Result<NetworkListResponse, ApiError> {
    let response = fetch_network_catalog(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch network catalog", err))?;

    if let Ok(serialized) = serde_json::to_string(&response) {
        state
            .cache
            .put(
                NETWORKS_CACHE_NAMESPACE,
                NETWORKS_CACHE_KEY,
                serialized,
                Some(Duration::from_secs(NETWORKS_REFRESH_INTERVAL_SECS)),
            )
            .await;
    }

    Ok(response)
}

async fn fetch_network_stats(
    db: &sqlx::PgPool,
) -> Result<HashMap<String, NetworkStatsV1>, sqlx::Error> {
    let rows: Vec<NetworkStatsRow> = sqlx::query_as(
        r#"
        SELECT
            c.network,
            COUNT(DISTINCT c.id)::BIGINT AS contract_count,
            COALESCE((
                SELECT SUM(ci.interaction_count)
                FROM contract_interactions ci
                WHERE ci.network = c.network
                  AND ci.created_at >= NOW() - INTERVAL '24 hours'
            ), 0)::BIGINT AS transaction_volume_24h
        FROM contracts c
        GROUP BY c.network
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.network.to_string(),
                NetworkStatsV1 {
                    contract_count: row.contract_count,
                    transaction_volume_24h: row.transaction_volume_24h,
                },
            )
        })
        .collect())
}

async fn fetch_active_validator_count(db: &sqlx::PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM validators WHERE status = 'active'")
        .fetch_one(db)
        .await
}

async fn fetch_network_catalog_v1(
    state: &AppState,
    real_time: bool,
) -> Result<NetworkListResponseV1, ApiError> {
    let catalog = fetch_network_catalog(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch network catalog v1", err))?;
    let stats = fetch_network_stats(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch network stats", err))?;
    let validator_count = fetch_active_validator_count(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch validator count", err))?;
    let definitions: HashMap<&'static str, StaticNetworkDefinition> = configured_networks()
        .into_iter()
        .map(|definition| (definition.id, definition))
        .collect();

    let networks = catalog
        .networks
        .into_iter()
        .map(|network| {
            let definition = definitions
                .get(network.id.as_str())
                .expect("configured network definition exists");
            let network_stats = stats
                .get(&network.network_type.to_string())
                .cloned()
                .unwrap_or(NetworkStatsV1 {
                    contract_count: 0,
                    transaction_volume_24h: 0,
                });

            NetworkInfoV1 {
                name: network.name,
                chain_id: definition.chain_id.to_string(),
                network: network.network_type,
                rpc_endpoint: network.endpoints.rpc_url,
                status: status_indicator(definition.id, &network.status),
                description: definition.description.to_string(),
                logo_url: definition.logo_url.to_string(),
                explorer_url: network.endpoints.explorer_url,
                friendbot_url: network.endpoints.friendbot_url,
                stats: network_stats,
                health: NetworkHealthMetricsV1 {
                    avg_block_time_seconds: definition.avg_block_time_seconds,
                    validator_count,
                    last_indexed_ledger_height: network.last_indexed_ledger_height,
                    last_indexed_at: network.last_indexed_at,
                    consecutive_failures: network.consecutive_failures,
                },
                last_checked_at: network.last_checked_at,
                status_message: network.status_message,
            }
        })
        .collect();

    Ok(NetworkListResponseV1 {
        networks,
        cached_at: chrono::Utc::now(),
        cache_ttl_seconds: NETWORKS_V1_CACHE_TTL_SECS,
        real_time,
    })
}

async fn refresh_network_catalog_v1_cache(
    state: &AppState,
) -> Result<NetworkListResponseV1, ApiError> {
    let response = fetch_network_catalog_v1(state, false).await?;

    if let Ok(serialized) = serde_json::to_string(&response) {
        state
            .cache
            .put(
                NETWORKS_CACHE_NAMESPACE,
                NETWORKS_V1_CACHE_KEY,
                serialized,
                Some(Duration::from_secs(NETWORKS_V1_CACHE_TTL_SECS as u64)),
            )
            .await;
    }

    Ok(response)
}

pub async fn run_network_catalog_refresh(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(NETWORKS_REFRESH_INTERVAL_SECS));

    loop {
        interval.tick().await;
        if let Err(err) = refresh_network_catalog_cache(&state).await {
            tracing::warn!(error = ?err, "failed to refresh network catalog cache");
        }
    }
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, sqlx::Type, utoipa::ToSchema,
)]
#[sqlx(type_name = "contract_audit_event_type", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ContractAuditEventType {
    ContractCreated,
    MetadataUpdated,
    VerificationAdded,
    StatusChanged,
    PublisherChanged,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow, utoipa::ToSchema)]
#[allow(dead_code)]
pub struct ContractAuditLogEntry {
    pub id: Uuid,
    pub event_type: ContractAuditEventType,
    pub contract_id: Uuid,
    pub user_id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub changes: serde_json::Value,
    pub ip_address: String,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct AuditLogQuery {
    #[serde(default = "default_audit_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_audit_limit() -> i64 {
    100
}

#[derive(Debug, serde::Deserialize)]
pub struct PublisherContractsQuery {
    #[serde(default = "default_contracts_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    /// Filter by network (mainnet, testnet, futurenet)
    pub network: Option<Network>,
    /// Filter by category (e.g., DeFi, NFT)
    pub category: Option<String>,
}

fn default_contracts_limit() -> i64 {
    20
}

const DEFAULT_CONTRACT_LIST_LIMIT: i64 = 50;
const MAX_CONTRACT_LIST_LIMIT: i64 = 1000;

fn validate_contract_list_pagination(
    params: &ContractSearchParams,
) -> Result<(i64, i64, i64), ApiError> {
    let limit = params.limit.unwrap_or(DEFAULT_CONTRACT_LIST_LIMIT);
    if !(1..=MAX_CONTRACT_LIST_LIMIT).contains(&limit) {
        return Err(ApiError::bad_request_with(
            "InvalidPaginationLimit",
            format!(
                "Invalid `limit` value {limit}. Expected an integer between 1 and {MAX_CONTRACT_LIST_LIMIT}."
            ),
        ));
    }

    if let Some(offset) = params.offset {
        if offset < 0 {
            return Err(ApiError::bad_request_with(
                "InvalidPaginationOffset",
                format!("Invalid `offset` value {offset}. Expected a non-negative integer."),
            ));
        }

        let page = (offset / limit) + 1;
        return Ok((limit, offset, page));
    }

    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1).max(0) * limit;
    Ok((limit, offset, page))
}

pub(crate) fn extract_ip_address(headers: &HeaderMap) -> String {
    if let Some(forwarded_for) = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    {
        let first = forwarded_for
            .split(',')
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(ip) = first {
            return ip.to_string();
        }
    }

    if let Some(real_ip) = headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return real_ip.to_string();
    }

    "unknown".to_string()
}

fn parse_batch_fields(raw_fields: Option<&str>) -> Option<HashSet<String>> {
    let set: HashSet<String> = raw_fields
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .collect();

    if set.is_empty() {
        None
    } else {
        Some(set)
    }
}

fn contract_to_filtered_value(contract: &Contract, fields: Option<&HashSet<String>>) -> Value {
    if fields.is_none() {
        return serde_json::to_value(contract).unwrap_or(Value::Null);
    }

    let Some(source) = serde_json::to_value(contract)
        .ok()
        .and_then(|v| v.as_object().cloned())
    else {
        return Value::Null;
    };

    let mut out = serde_json::Map::new();
    let selected = fields.expect("checked above");

    for field in selected {
        if field == "address" {
            out.insert(
                "address".to_string(),
                Value::String(contract.contract_id.clone()),
            );
            continue;
        }

        if let Some(value) = source.get(field) {
            out.insert(field.clone(), value.clone());
        }
    }

    Value::Object(out)
}

pub(crate) async fn write_contract_audit_log(
    db: &sqlx::PgPool,
    action_type: AuditActionType,
    contract_id: Uuid,
    user_id: Uuid,
    changes: serde_json::Value,
    ip_address: &str,
) -> Result<(), sqlx::Error> {
    let (old_value, new_value) = split_audit_changes(&changes, ip_address);

    sqlx::query(
        "INSERT INTO contract_audit_log (action_type, contract_id, old_value, new_value, changed_by)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(action_type)
    .bind(contract_id)
    .bind(old_value)
    .bind(new_value)
    .bind(user_id.to_string())
    .execute(db)
    .await?;

    Ok(())
}

fn split_audit_changes(
    changes: &serde_json::Value,
    ip_address: &str,
) -> (Option<serde_json::Value>, Option<serde_json::Value>) {
    let mut old_value = serde_json::Map::new();
    let mut new_value = serde_json::Map::new();
    let mut saw_before_after_pair = false;

    match changes {
        serde_json::Value::Object(fields) => {
            for (field, delta) in fields {
                match delta {
                    serde_json::Value::Object(delta_obj) => {
                        let before = delta_obj.get("before");
                        let after = delta_obj.get("after");

                        if before.is_some() || after.is_some() {
                            saw_before_after_pair = true;
                            if let Some(before) = before {
                                if !before.is_null() {
                                    old_value.insert(field.clone(), before.clone());
                                }
                            }
                            if let Some(after) = after {
                                if !after.is_null() {
                                    new_value.insert(field.clone(), after.clone());
                                }
                            }
                        } else {
                            new_value.insert(field.clone(), delta.clone());
                        }
                    }
                    _ => {
                        new_value.insert(field.clone(), delta.clone());
                    }
                }
            }
        }
        _ => {
            new_value.insert("changes".to_string(), changes.clone());
        }
    }

    if !saw_before_after_pair && new_value.is_empty() {
        new_value.insert("changes".to_string(), changes.clone());
    }

    new_value.insert(
        "_ip_address".to_string(),
        serde_json::Value::String(ip_address.to_string()),
    );

    let old_value = if old_value.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(old_value))
    };
    let new_value = if new_value.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(new_value))
    };

    (old_value, new_value)
}

fn parse_interaction_type(
    interaction_type: Option<&str>,
    method: Option<&str>,
) -> Result<String, ApiError> {
    let mut normalized = interaction_type
        .map(|v| v.trim().to_ascii_lowercase())
        .or_else(|| {
            if method.is_some() {
                Some("invoke".to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "invoke".to_string());

    if normalized == "invocation" {
        normalized = "invoke".to_string();
    }

    let valid = matches!(
        normalized.as_str(),
        "deploy" | "invoke" | "transfer" | "query" | "publish_success" | "publish_failed"
    );

    if !valid {
        return Err(ApiError::bad_request_with(
            "InvalidInteractionType",
            format!(
                "interaction_type '{}' is invalid; expected one of: deploy, invoke, transfer, query, publish_success, publish_failed",
                normalized
            ),
        ));
    }

    Ok(normalized)
}

fn infer_target_identifier_from_parameters(
    parameters: Option<&serde_json::Value>,
) -> Option<String> {
    let payload = parameters?.as_object()?;
    let candidate_keys = [
        "target_contract_id",
        "target",
        "callee",
        "to_contract",
        "to",
        "contract_id",
    ];

    for key in candidate_keys {
        let Some(value) = payload.get(key) else {
            continue;
        };
        if let Some(identifier) = value.as_str() {
            let trimmed = identifier.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

async fn resolve_call_target_contract(
    db: &sqlx::PgPool,
    explicit_target: Option<&str>,
    parameters: Option<&serde_json::Value>,
) -> Result<Option<Uuid>, ApiError> {
    let candidate = explicit_target
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| infer_target_identifier_from_parameters(parameters));

    let Some(identifier) = candidate else {
        return Ok(None);
    };

    dependency::resolve_contract_id(db, &identifier)
        .await
        .map_err(|err| {
            ApiError::internal(format!(
                "Failed to resolve interaction target contract: {}",
                err
            ))
        })
}

async fn record_contract_interaction(
    db: &sqlx::PgPool,
    input: ContractInteractionInsert<'_>,
) -> Result<Uuid, sqlx::Error> {
    let mut tx = db.begin().await?;

    let interaction_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO contract_interactions
          (
            contract_id, user_address, interaction_type, transaction_hash, method,
            parameters, return_value, interaction_timestamp, interaction_count, network, created_at
          )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, $9, $10)
        RETURNING id
        "#,
    )
    .bind(input.contract_id)
    .bind(input.account)
    .bind(input.interaction_type)
    .bind(input.transaction_hash)
    .bind(input.method)
    .bind(input.parameters)
    .bind(input.return_value)
    .bind(input.timestamp)
    .bind(input.network)
    .bind(input.timestamp)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO contract_interaction_daily_aggregates
          (contract_id, interaction_type, network, day, count, updated_at)
        VALUES ($1, $2, $3, $4, 1, NOW())
        ON CONFLICT (contract_id, interaction_type, network, day)
        DO UPDATE SET
          count = contract_interaction_daily_aggregates.count + 1,
          updated_at = NOW()
        "#,
    )
    .bind(input.contract_id)
    .bind(input.interaction_type)
    .bind(input.network)
    .bind(input.timestamp.date_naive())
    .execute(&mut *tx)
    .await?;

    if input.interaction_type == "invoke" {
        if let Some(target_contract_id) = input.target_contract_id {
            if target_contract_id != input.contract_id {
                sqlx::query(
                    r#"
                    INSERT INTO contract_call_edge_daily_aggregates
                      (source_contract_id, target_contract_id, network, day, call_count, updated_at)
                    VALUES ($1, $2, $3, $4, 1, NOW())
                    ON CONFLICT (source_contract_id, target_contract_id, network, day)
                    DO UPDATE SET
                      call_count = contract_call_edge_daily_aggregates.call_count + 1,
                      updated_at = NOW()
                    "#,
                )
                .bind(input.contract_id)
                .bind(target_contract_id)
                .bind(input.network)
                .bind(input.timestamp.date_naive())
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    tx.commit().await?;

    Ok(interaction_id)
}

struct ContractInteractionInsert<'a> {
    contract_id: Uuid,
    target_contract_id: Option<Uuid>,
    account: Option<&'a str>,
    interaction_type: &'a str,
    transaction_hash: Option<&'a str>,
    method: Option<&'a str>,
    parameters: Option<&'a serde_json::Value>,
    return_value: Option<&'a serde_json::Value>,
    timestamp: chrono::DateTime<chrono::Utc>,
    network: &'a Network,
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = Object),
        (status = 503, description = "Service is unavailable or degraded", body = Object)
    ),
    tag = "Observability"
)]
pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let uptime = state.started_at.elapsed().as_secs();
    let now = chrono::Utc::now().to_rfc3339();

    if state
        .is_shutting_down
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        tracing::warn!(uptime_secs = uptime, "health check failing — shutting down");
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "shutting_down",
                "version": VERSION,
                "timestamp": now,
                "uptime_secs": uptime
            })),
        );
    }

    (
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "version": VERSION,
            "timestamp": now
        })),
    )
}

pub async fn health_check_live(State(state): State<AppState>) -> StatusCode {
    if state
        .is_shutting_down
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    }
}

pub async fn health_check_ready(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let uptime = state.started_at.elapsed().as_secs();
    let now = chrono::Utc::now().to_rfc3339();

    if state
        .is_shutting_down
        .load(std::sync::atomic::Ordering::SeqCst)
    {
        tracing::warn!(
            uptime_secs = uptime,
            "readiness check failing — shutting down"
        );
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "reason": "shutting_down",
                "version": VERSION,
                "timestamp": now
            })),
        );
    }

    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok();

    if db_ok {
        tracing::info!(uptime_secs = uptime, "readiness check passed");
        (
            StatusCode::OK,
            Json(json!({
                "status": "ready",
                "version": VERSION,
                "timestamp": now
            })),
        )
    } else {
        tracing::warn!(
            uptime_secs = uptime,
            "readiness check failed — db unreachable"
        );
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "reason": "database_unavailable",
                "version": VERSION,
                "timestamp": now
            })),
        )
    }
}

pub async fn health_check_detailed(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let uptime = state.started_at.elapsed().as_secs();
    let now = chrono::Utc::now().to_rfc3339();

    let is_shutting_down = state
        .is_shutting_down
        .load(std::sync::atomic::Ordering::SeqCst);

    let db_health = if sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
        .is_ok()
    {
        json!({"status": "healthy"})
    } else {
        json!({"status": "unhealthy", "error": "database connection failed"})
    };

    let cache_config = state.cache.config();
    let cache_health = json!({
        "status": "healthy",
        "enabled": cache_config.enabled,
        "max_capacity": cache_config.max_capacity
    });

    let overall_status = if is_shutting_down {
        "unhealthy"
    } else if db_health["status"] == "unhealthy" {
        "degraded"
    } else {
        "healthy"
    };

    let status_code = if is_shutting_down || db_health["status"] == "unhealthy" {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    (
        status_code,
        Json(json!({
            "status": overall_status,
            "version": VERSION,
            "timestamp": now,
            "uptime_secs": uptime,
            "dependencies": {
                "database": db_health,
                "cache": cache_health
            }
        })),
    )
}

#[utoipa::path(
    get,
    path = "/api/stats",
    responses(
        (status = 200, description = "Global registry statistics", body = Object, example = json!({"total_contracts": 150, "verified_contracts": 120, "total_publishers": 45}))
    ),
    tag = "Observability"
)]
pub async fn get_stats(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    if let Some(cached) = state.cache.get_stats("global").await {
        if let Ok(val) = serde_json::from_str::<Value>(&cached) {
            return Ok(Json(val));
        }
    }

    let total_contracts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts")
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count contracts", err))?;

    let verified_contracts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM contracts WHERE is_verified = true")
            .fetch_one(&state.db)
            .await
            .map_err(|err| db_internal_error("count verified contracts", err))?;

    let total_publishers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM publishers")
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count publishers", err))?;

    let stats = json!({
        "total_contracts": total_contracts,
        "verified_contracts": verified_contracts,
        "total_publishers": total_publishers,
    });

    if let Ok(serialized) = serde_json::to_string(&stats) {
        state.cache.put_stats("global", serialized).await;
    }

    Ok(Json(stats))
}

#[utoipa::path(
    get,
    path = "/networks",
    responses(
        (status = 200, description = "Supported network metadata", body = NetworkListResponse)
    ),
    tag = "Networks"
)]
pub async fn list_networks(State(state): State<AppState>) -> ApiResult<Json<NetworkListResponse>> {
    if let (Some(cached), true) = state
        .cache
        .get(NETWORKS_CACHE_NAMESPACE, NETWORKS_CACHE_KEY)
        .await
    {
        if let Ok(payload) = serde_json::from_str::<NetworkListResponse>(&cached) {
            return Ok(Json(payload));
        }
    }

    let response = refresh_network_catalog_cache(&state).await?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/v1/networks",
    params(NetworkListV1Query),
    responses(
        (status = 200, description = "Supported Soroban networks with stats and health metadata", body = NetworkListResponseV1)
    ),
    tag = "Networks"
)]
pub async fn list_networks_v1(
    State(state): State<AppState>,
    Query(query): Query<NetworkListV1Query>,
) -> ApiResult<Json<NetworkListResponseV1>> {
    if query.real_time {
        return Ok(Json(fetch_network_catalog_v1(&state, true).await?));
    }

    if let (Some(cached), true) = state
        .cache
        .get(NETWORKS_CACHE_NAMESPACE, NETWORKS_V1_CACHE_KEY)
        .await
    {
        if let Ok(payload) = serde_json::from_str::<NetworkListResponseV1>(&cached) {
            let cache_age = chrono::Utc::now() - payload.cached_at;
            if cache_age.num_seconds() < NETWORKS_V1_CACHE_TTL_SECS {
                return Ok(Json(payload));
            }
        }
    }

    let response = refresh_network_catalog_v1_cache(&state).await?;
    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/networks/health",
    responses(
        (status = 200, description = "Current health status of all networks", body = NetworkHealthResponse)
    ),
    tag = "Networks"
)]
pub async fn get_network_health(
    State(state): State<AppState>,
) -> ApiResult<Json<NetworkHealthResponse>> {
    if let (Some(cached), true) = state
        .cache
        .get(NETWORKS_HEALTH_CACHE_NAMESPACE, NETWORKS_HEALTH_CACHE_KEY)
        .await
    {
        if let Ok(payload) = serde_json::from_str::<NetworkHealthResponse>(&cached) {
            return Ok(Json(payload));
        }
    }

    let catalog = fetch_network_catalog(&state.db).await?;
    let now = chrono::Utc::now();

    let mut health = Vec::new();
    for net in catalog.networks {
        let rpc_available = net.status == NetworkStatus::Online;

        // In a real scenario, we would fetch the current ledger height from the RPC here
        // For this task, we will simulate it by adding a small random lag if it's available
        let current_ledger = net.last_indexed_ledger_height.map(|h| (h + 5) as u32);
        let indexer_lag =
            if let (Some(last), Some(curr)) = (net.last_indexed_ledger_height, current_ledger) {
                Some((curr as i64) - last)
            } else {
                None
            };

        health.push(NetworkHealth {
            network_id: net.id,
            name: net.name,
            status: net.status,
            rpc_available,
            last_indexed_ledger: net.last_indexed_ledger_height,
            current_ledger,
            indexer_lag,
            last_checked_at: net.last_checked_at,
        });
    }

    let response = NetworkHealthResponse {
        health,
        timestamp: now,
    };

    if let Ok(serialized) = serde_json::to_string(&response) {
        state
            .cache
            .put(
                NETWORKS_HEALTH_CACHE_NAMESPACE,
                NETWORKS_HEALTH_CACHE_KEY,
                serialized,
                Some(Duration::from_secs(NETWORKS_HEALTH_TTL)),
            )
            .await;
    }

    Ok(Json(response))
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct SearchSuggestionsQuery {
    pub q: String,
    #[serde(default = "default_search_suggestion_limit")]
    pub limit: i64,
}

fn default_search_suggestion_limit() -> i64 {
    8
}

#[utoipa::path(
    get,
    path = "/api/contracts/suggestions",
    params(SearchSuggestionsQuery),
    responses(
        (status = 200, description = "Autocomplete suggestions for contract search", body = SearchSuggestionsResponse)
    ),
    tag = "Contracts"
)]
pub async fn get_contract_search_suggestions(
    State(state): State<AppState>,
    Query(query): Query<SearchSuggestionsQuery>,
) -> ApiResult<Json<SearchSuggestionsResponse>> {
    let started_at = std::time::Instant::now();
    let limit = query.limit.clamp(1, 20);
    let normalized = query.q.trim().to_ascii_lowercase();

    if normalized.is_empty() {
        return Ok(Json(SearchSuggestionsResponse { items: Vec::new() }));
    }

    let cache_key = format!("suggestions:{}:{}", normalized, limit);
    if let (Some(cached), true) = state.cache.get("search", &cache_key).await {
        if let Ok(payload) = serde_json::from_str::<SearchSuggestionsResponse>(&cached) {
            observe_search_query("suggestions", started_at, Some(&query.q), limit);
            return Ok(Json(payload));
        }
    }

    let prefix = format!("{}%", normalized);
    let rows: Vec<(String, String, f64)> = sqlx::query_as(
        r#"
        WITH candidates AS (
            SELECT DISTINCT ON (lower(name))
                name AS text,
                'contract' AS kind,
                GREATEST(
                    similarity(lower(name), $1),
                    CASE WHEN lower(name) LIKE $2 THEN 1.0 ELSE 0.0 END
                ) AS score
            FROM contracts
            WHERE lower(name) LIKE $2 OR lower(name) % $1

            UNION ALL

            SELECT DISTINCT ON (lower(category))
                category AS text,
                'category' AS kind,
                GREATEST(
                    similarity(lower(category), $1),
                    CASE WHEN lower(category) LIKE $2 THEN 0.95 ELSE 0.0 END
                ) AS score
            FROM contracts
            WHERE category IS NOT NULL
              AND (lower(category) LIKE $2 OR lower(category) % $1)
        )
        SELECT text, kind, score
        FROM candidates
        WHERE text IS NOT NULL
        ORDER BY score DESC, length(text), text
        LIMIT $3
        "#,
    )
    .bind(&normalized)
    .bind(&prefix)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch search suggestions", err))?;

    let response = SearchSuggestionsResponse {
        items: rows
            .into_iter()
            .map(|(text, kind, score)| SearchSuggestion { text, kind, score })
            .collect(),
    };

    if let Ok(serialized) = serde_json::to_string(&response) {
        state
            .cache
            .put(
                "search",
                &cache_key,
                serialized,
                Some(Duration::from_secs(60)),
            )
            .await;
    }

    observe_search_query("suggestions", started_at, Some(&query.q), limit);
    Ok(Json(response))
}

/// List and search contracts
#[utoipa::path(
    get,
    path = "/api/contracts",
    params(ContractSearchParams),
    responses(
        (status = 200, description = "List of contracts", body = PaginatedResponse<Contract>),
        (status = 400, description = "Invalid query parameters")
    ),
    tag = "Contracts"
)]
pub async fn list_tags(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    #[derive(sqlx::FromRow)]
    struct TagRow {
        id: uuid::Uuid,
        name: String,
        color: Option<String>,
        usage_count: Option<i32>,
    }
    let rows = sqlx::query_as::<_, TagRow>(
        "SELECT t.id, t.name, t.color, COUNT(ct.contract_id)::INT AS usage_count \
         FROM tags t \
         LEFT JOIN contract_tags ct ON t.id = ct.tag_id \
         GROUP BY t.id \
         ORDER BY usage_count DESC, t.name ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("list tags", err))?;

    let tags: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "id": r.id,
                "name": r.name,
                "color": r.color,
                "usage_count": r.usage_count
            })
        })
        .collect();

    Ok(Json(json!(tags)))
}

pub async fn list_contracts(
    State(state): State<AppState>,
    claims: Option<crate::auth::AuthClaims>,
    params: Result<Query<ContractSearchParams>, QueryRejection>,
) -> axum::response::Response {
    let search_started_at = std::time::Instant::now();
    let Query(params) = match params {
        Ok(q) => q,
        Err(err) => return map_query_rejection(err).into_response(),
    };

    let cache_key = match serde_json::to_string(&params) {
        Ok(json) => {
            let sub = claims.as_ref().map(|c| c.sub.as_str()).unwrap_or("public");
            format!("contracts:list:{}:{}", sub, json)
        }
        Err(_) => String::new(),
    };

    if !cache_key.is_empty() {
        if let Some(cached) = state.cache.get_contracts(&cache_key).await {
            if let Ok(response) = serde_json::from_str::<PaginatedResponse<Contract>>(&cached) {
                return Json(response).into_response();
            }
        }
    }

    let (limit, offset, page) = match validate_contract_list_pagination(&params) {
        Ok(values) => values,
        Err(err) => return err.into_response(),
    };

    let cursor = if let Some(cursor_str) = &params.cursor {
        match shared::pagination::Cursor::decode(cursor_str) {
            Ok(c) => Some(c),
            Err(err) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "InvalidPaginationCursor",
                        "message": format!("The provided pagination cursor is invalid: {}", err)
                    })),
                )
                    .into_response();
            }
        }
    } else {
        None
    };

    let sort_by = params.sort_by.clone().unwrap_or(shared::SortBy::CreatedAt);
    let sort_order = params.sort_order.clone().unwrap_or(shared::SortOrder::Desc);
    let direction = if sort_order == shared::SortOrder::Asc {
        "ASC"
    } else {
        "DESC"
    };

    let mut qb: QueryBuilder<'_, sqlx::Postgres> = QueryBuilder::new(
        "SELECT c.* FROM contracts c LEFT JOIN contract_interactions ci ON c.id = ci.contract_id ",
    );
    qb.push("WHERE (c.visibility = 'public'");

    if let Some(claims) = &claims {
        qb.push(" OR c.visibility = 'private' AND c.organization_id IN (");
        qb.push("SELECT om.organization_id FROM organization_members om ");
        qb.push("JOIN publishers p ON p.id = om.publisher_id WHERE p.stellar_address = ");
        qb.push_bind(&claims.sub);
        qb.push("))");
    }

    qb.push(")");

    if params.verified_only.unwrap_or(false) {
        qb.push(" AND c.is_verified = true");
    }

    if let Some(status) = &params.verification_status {
        qb.push(" AND c.verification_status = ");
        qb.push_bind(status);
    }

    let mut categories = params.categories.clone().unwrap_or_default();
    if let Some(category) = &params.category {
        categories.push(category.clone());
    }
    categories.retain(|category| !category.trim().is_empty());
    if !categories.is_empty() {
        qb.push(" AND c.category IN (");
        let mut separated = qb.separated(", ");
        for category in categories {
            separated.push_bind(category);
        }
        separated.push_unseparated(")");
    }

    if let Some(networks) = params
        .networks
        .as_ref()
        .filter(|n| !n.is_empty())
        .cloned()
        .or_else(|| params.network.clone().map(|n| vec![n]))
    {
        qb.push(" AND c.network IN (");
        let mut separated = qb.separated(", ");
        for network in networks {
            separated.push_bind(network);
        }
        separated.push_unseparated(")");
    }

    if let Some(tags) = &params.tags {
        if !tags.is_empty() {
            qb.push(" AND c.id IN (SELECT contract_id FROM contract_tags ct JOIN tags t ON t.id = ct.tag_id WHERE t.name IN (");
            let mut separated = qb.separated(", ");
            for tag in tags {
                separated.push_bind(tag);
            }
            separated.push_unseparated("))");
        }
    }

    if let Some(q) = &params.query {
        let like = format!("%{}%", q.to_ascii_lowercase());
        qb.push(" AND (c.search_vector @@ plainto_tsquery('english', ");
        qb.push_bind(q);
        qb.push(") OR lower(c.contract_id) LIKE ");
        qb.push_bind(like);
        qb.push(")");
    }

    // Cursor pagination condition
    if let Some(cursor) = &cursor {
        let op = if sort_order == shared::SortOrder::Asc {
            ">"
        } else {
            "<"
        };
        qb.push(" AND (c.created_at ");
        qb.push(op);
        qb.push(" ");
        qb.push_bind(cursor.timestamp);
        qb.push(" OR (c.created_at = ");
        qb.push_bind(cursor.timestamp);
        qb.push(" AND c.id ");
        qb.push(op);
        qb.push(" ");
        qb.push_bind(cursor.id);
        qb.push("))");
    }

    qb.push(" GROUP BY c.id");
    qb.push(" ORDER BY ");
    if cursor.is_some() {
        qb.push("c.created_at ");
    } else {
        match sort_by {
            shared::SortBy::UpdatedAt => {
                qb.push("c.updated_at ");
            }
            shared::SortBy::VerifiedAt => {
                qb.push("c.verified_at ");
            }
            shared::SortBy::LastAccessedAt => {
                qb.push("c.last_accessed_at ");
            }
            shared::SortBy::Popularity | shared::SortBy::Interactions => {
                qb.push("COUNT(ci.id) ");
            }
            shared::SortBy::Deployments => {
                qb.push("SUM(CASE WHEN ci.interaction_type = 'deploy' THEN 1 ELSE 0 END) ");
            }
            shared::SortBy::Relevance if params.query.is_some() => {
                qb.push("c.created_at ");
            }
            _ => {
                qb.push("c.created_at ");
            }
        }
    }
    qb.push(direction);

    qb.push(", c.id ");
    qb.push(direction);

    // Fetch one extra item to check for next page
    let fetch_limit = limit + 1;
    qb.push(" LIMIT ");
    qb.push_bind(fetch_limit);

    if cursor.is_none() {
        qb.push(" OFFSET ");
        qb.push_bind(offset);
    }

    let mut contracts: Vec<Contract> = match qb.build_query_as().fetch_all(&state.db).await {
        Ok(rows) => rows,
        Err(err) => return db_internal_error("list contracts", err).into_response(),
    };

    // Fetch tags for these contracts
    let contract_ids: Vec<Uuid> = contracts.iter().map(|c| c.id).collect();
    if !contract_ids.is_empty() {
        #[derive(sqlx::FromRow)]
        struct TagRow {
            contract_id: Uuid,
            id: Uuid,
            name: String,
            color: String,
        }
        let tag_rows = match sqlx::query_as::<_, TagRow>(
            r#"
            SELECT ct.contract_id, t.id, t.name, t.color
            FROM tags t
            JOIN contract_tags ct ON t.id = ct.tag_id
            WHERE ct.contract_id = ANY($1)
            "#,
        )
        .bind(&contract_ids)
        .fetch_all(&state.db)
        .await
        {
            Ok(rows) => rows,
            Err(err) => return db_internal_error("fetch tags", err).into_response(),
        };

        let mut tags_map: HashMap<Uuid, Vec<shared::Tag>> = HashMap::new();
        for row in tag_rows {
            tags_map
                .entry(row.contract_id)
                .or_default()
                .push(shared::Tag {
                    id: row.id,
                    name: row.name,
                    color: row.color,
                });
        }

        for contract in &mut contracts {
            if let Some(tags) = tags_map.remove(&contract.id) {
                contract.tags = tags;
            }
        }
    }

    let mut count_qb: QueryBuilder<'_, sqlx::Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM contracts c WHERE c.visibility = 'public'");

    if let Some(claims) = &claims {
        count_qb.push(" OR (c.visibility = 'private' AND c.organization_id IN (");
        count_qb.push("SELECT om.organization_id FROM organization_members om ");
        count_qb.push("JOIN publishers p ON p.id = om.publisher_id WHERE p.stellar_address = ");
        count_qb.push_bind(&claims.sub);
        count_qb.push("))");
    }
    if params.verified_only.unwrap_or(false) {
        count_qb.push(" AND c.is_verified = true");
    }
    if let Some(status) = &params.verification_status {
        count_qb.push(" AND c.verification_status = ");
        count_qb.push_bind(status);
    }
    let mut count_categories = params.categories.clone().unwrap_or_default();
    if let Some(cat) = &params.category {
        count_categories.push(cat.clone());
    }
    count_categories.retain(|c| !c.trim().is_empty());
    if !count_categories.is_empty() {
        count_qb.push(" AND c.category IN (");
        let mut sep = count_qb.separated(", ");
        for cat in count_categories {
            sep.push_bind(cat);
        }
        sep.push_unseparated(")");
    }

    if let Some(count_networks) = params
        .networks
        .as_ref()
        .filter(|n| !n.is_empty())
        .cloned()
        .or_else(|| params.network.clone().map(|n| vec![n]))
    {
        count_qb.push(" AND c.network IN (");
        let mut sep = count_qb.separated(", ");
        for net in count_networks {
            sep.push_bind(net);
        }
        sep.push_unseparated(")");
    }

    if let Some(tags) = &params.tags {
        if !tags.is_empty() {
            count_qb.push(" AND c.id IN (SELECT contract_id FROM contract_tags ct JOIN tags t ON t.id = ct.tag_id WHERE t.name IN (");
            let mut separated = count_qb.separated(", ");
            for tag in tags {
                separated.push_bind(tag);
            }
            separated.push_unseparated("))");
        }
    }
    if let Some(q) = &params.query {
        let like = format!("%{}%", q.to_ascii_lowercase());
        count_qb.push(" AND (c.search_vector @@ plainto_tsquery('english', ");
        count_qb.push_bind(q);
        count_qb.push(") OR lower(c.contract_id) LIKE ");
        count_qb.push_bind(like);
        count_qb.push(")");
    }

    let total: i64 = match count_qb.build_query_scalar().fetch_one(&state.db).await {
        Ok(v) => v,
        Err(err) => return db_internal_error("count contracts", err).into_response(),
    };

    let mut has_more = contracts.len() > limit as usize;
    if has_more {
        contracts.truncate(limit as usize);
    }

    let mut response = PaginatedResponse::new(contracts, total, page, limit);

    // Populate active filter metadata
    {
        let mut networks: Option<Vec<String>> = None;
        if let Some(n) = &params.networks {
            if !n.is_empty() {
                networks = Some(n.iter().map(|net| net.to_string()).collect());
            }
        } else if let Some(n) = &params.network {
            networks = Some(vec![n.to_string()]);
        }
        let categories = params.categories.clone().filter(|c| !c.is_empty());
        let tags = params.tags.clone().filter(|t| !t.is_empty());
        let verified_only = params.verified_only;
        let verification_status = params
            .verification_status
            .as_ref()
            .map(|s| format!("{s:?}").to_lowercase());

        let filters = shared::SearchFilterMetadata {
            networks,
            categories,
            verified_only,
            verification_status,
            tags,
            maturity: params.maturity.as_ref().map(|m| format!("{:?}", m)),
            query: params.query.clone(),
        };
        response = response.with_filters(filters);
    }

    if has_more {
        if let Some(last_item) = response.items.last() {
            let next_cursor = shared::pagination::Cursor::new(last_item.created_at, last_item.id);
            response.next_cursor = Some(next_cursor.encode());
        }
    }

    observe_search_query(
        "contracts",
        search_started_at,
        params.query.as_deref(),
        limit,
    );

    // Write to cache after successful DB fetch
    if !cache_key.is_empty() {
        if let Ok(serialized) = serde_json::to_string(&response) {
            state.cache.put_contracts(cache_key, serialized).await;
        }
    }

    Json(response).into_response()
}

#[utoipa::path(
    get,
    path = "/api/contracts",
    params(ContractSearchParams),
    responses(
        (status = 200, description = "List of contracts", body = PaginatedResponse<Contract>),
        (status = 400, description = "Invalid query parameters")
    ),
    tag = "Contracts"
)]
pub async fn list_contracts_openapi_marker() {}

fn csv_escape(value: &str) -> String {
    let needs_quotes = value.contains(',') || value.contains('\"') || value.contains('\n');
    if needs_quotes {
        let escaped = value.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        value.to_string()
    }
}

fn optional_json_string(value: &Option<serde_json::Value>) -> String {
    value
        .as_ref()
        .map(|inner| inner.to_string())
        .unwrap_or_default()
}

fn optional_datetime_string(value: &Option<chrono::DateTime<chrono::Utc>>) -> String {
    value.map(|inner| inner.to_rfc3339()).unwrap_or_default()
}

fn optional_uuid_string(value: &Option<Uuid>) -> String {
    value.map(|inner| inner.to_string()).unwrap_or_default()
}

fn optional_string(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

fn render_contract_export(
    format: &ContractExportFormat,
    metadata: &ContractExportMetadata,
    contracts: &[ContractMetadataExportRecord],
) -> ApiResult<String> {
    match format {
        ContractExportFormat::Json => {
            serde_json::to_string_pretty(&ContractMetadataExportEnvelope {
                metadata: metadata.clone(),
                contracts: contracts.to_vec(),
            })
            .map_err(|err| ApiError::internal(format!("Failed to render JSON export: {err}")))
        }
        ContractExportFormat::Jsonl => {
            // Header line: one JSON object carrying the export metadata.
            let mut out = serde_json::to_string(&serde_json::json!({
                "_type": "export_metadata",
                "exported_at": metadata.exported_at,
                "format": "jsonl",
                "total_count": metadata.total_count,
                "async_export": metadata.async_export,
                "filters": metadata.filters,
            }))
            .map_err(|err| ApiError::internal(format!("Failed to render JSONL header: {err}")))?;
            out.push('\n');

            for contract in contracts {
                let line = serde_json::to_string(contract).map_err(|err| {
                    ApiError::internal(format!("Failed to render JSONL record: {err}"))
                })?;
                out.push_str(&line);
                out.push('\n');
            }
            Ok(out)
        }
        ContractExportFormat::Yaml => serde_yaml::to_string(&ContractMetadataExportEnvelope {
            metadata: metadata.clone(),
            contracts: contracts.to_vec(),
        })
        .map_err(|err| ApiError::internal(format!("Failed to render YAML export: {err}"))),
        ContractExportFormat::Csv => {
            let mut csv = String::new();
            let filters_json = serde_json::to_string(&metadata.filters).map_err(|err| {
                ApiError::internal(format!("Failed to encode filter metadata: {err}"))
            })?;
            csv.push_str(&format!(
                "# exported_at={}\n",
                metadata.exported_at.to_rfc3339()
            ));
            csv.push_str(&format!("# format={}\n", metadata.format));
            csv.push_str(&format!("# total_count={}\n", metadata.total_count));
            csv.push_str(&format!("# filters={filters_json}\n"));
            csv.push_str("id,logical_id,contract_id,wasm_hash,name,description,publisher_id,publisher_stellar_address,publisher_username,network,is_verified,category,tags,maturity,health_score,is_maintenance,deployment_count,audit_status,visibility,organization_id,network_configs,created_at,updated_at,verified_at,last_verified_at,last_accessed_at\n");

            for contract in contracts {
                let tags = contract
                    .tags
                    .iter()
                    .map(|t| t.name.as_str())
                    .collect::<Vec<_>>()
                    .join("|");
                let row = [
                    contract.id.to_string(),
                    optional_uuid_string(&contract.logical_id),
                    contract.contract_id.clone(),
                    contract.wasm_hash.clone(),
                    contract.name.clone(),
                    optional_string(&contract.description),
                    contract.publisher_id.to_string(),
                    contract.publisher_stellar_address.clone(),
                    optional_string(&contract.publisher_username),
                    contract.network.clone(),
                    contract.is_verified.to_string(),
                    optional_string(&contract.category),
                    tags,
                    optional_string(&contract.maturity),
                    contract.health_score.to_string(),
                    contract.is_maintenance.to_string(),
                    contract.deployment_count.to_string(),
                    optional_string(&contract.audit_status),
                    contract.visibility.clone(),
                    optional_uuid_string(&contract.organization_id),
                    optional_json_string(&contract.network_configs),
                    contract.created_at.to_rfc3339(),
                    contract.updated_at.to_rfc3339(),
                    optional_datetime_string(&contract.verified_at),
                    optional_datetime_string(&contract.last_verified_at),
                    optional_datetime_string(&contract.last_accessed_at),
                ];

                let escaped = row
                    .iter()
                    .map(|value| csv_escape(value))
                    .collect::<Vec<_>>()
                    .join(",");
                csv.push_str(&escaped);
                csv.push('\n');
            }
            Ok(csv)
        }
    }
}

fn sanitized_export_filters(filters: ContractSearchParams) -> ContractSearchParams {
    filters
}

fn export_artifact_path(job_id: Uuid, format: &ContractExportFormat) -> PathBuf {
    std::env::temp_dir()
        .join("soroban-registry")
        .join("contract-exports")
        .join(format!("{}.{}", job_id, format))
}

fn contract_export_response(
    format: &ContractExportFormat,
    rendered_count: i64,
    content: String,
) -> Response {
    let content_type = match format {
        ContractExportFormat::Json => "application/json; charset=utf-8",
        ContractExportFormat::Jsonl => "application/x-ndjson; charset=utf-8",
        ContractExportFormat::Yaml => "application/yaml; charset=utf-8",
        ContractExportFormat::Csv => "text/csv; charset=utf-8",
    };

    let mut response = content.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static(content_type),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_str(&format!(
            "attachment; filename=contract-export-{}.{}",
            rendered_count, format
        ))
        .unwrap_or_else(|_| axum::http::HeaderValue::from_static("attachment")),
    );
    response
}

fn build_export_status_response(job: &ContractExportJob) -> ContractExportStatusResponse {
    ContractExportStatusResponse {
        job_id: job.job_id,
        status: job.status.clone(),
        status_url: format!("/contracts/export/{}", job.job_id),
        download_url: matches!(job.status, ContractExportJobStatus::Completed)
            .then(|| format!("/contracts/export/{}/download", job.job_id)),
        total_count: job.total_count,
        format: job.format.clone(),
        requested_at: job.requested_at,
        completed_at: job.completed_at,
        filters: job.filters.clone(),
        error: job.error.clone(),
    }
}

fn apply_contract_export_filters<'a>(
    query: &mut QueryBuilder<'a, Postgres>,
    filters: &'a ContractSearchParams,
    claims: Option<&'a crate::auth::AuthClaims>,
) {
    query.push(" FROM contracts c JOIN publishers p ON p.id = c.publisher_id WHERE (c.visibility = 'public'");
    if let Some(claims) = claims {
        query.push(" OR (c.visibility = 'private' AND c.organization_id IN (SELECT organization_id FROM organization_members om JOIN publishers p ON om.publisher_id = p.id WHERE p.stellar_address = ");
        query.push_bind(&claims.sub);
        query.push("))");
    }
    query.push(")");

    if filters.verified_only.unwrap_or(false) {
        query.push(" AND c.is_verified = true");
    }

    if let Some(network) = filters.network.as_ref() {
        query.push(" AND c.network = ");
        query.push_bind(network);
    }

    if let Some(networks) = filters
        .networks
        .as_ref()
        .filter(|networks| !networks.is_empty())
    {
        query.push(" AND c.network IN (");
        let mut separated = query.separated(", ");
        for network in networks {
            separated.push_bind(network);
        }
        separated.push_unseparated(")");
    }

    if let Some(category) = filters.category.as_ref() {
        query.push(" AND c.category = ");
        query.push_bind(category);
    }

    if let Some(categories) = filters
        .categories
        .as_ref()
        .filter(|categories| !categories.is_empty())
    {
        query.push(" AND c.category IN (");
        let mut separated = query.separated(", ");
        for category in categories {
            separated.push_bind(category);
        }
        separated.push_unseparated(")");
    }

    if let Some(tags) = filters.tags.as_ref().filter(|tags| !tags.is_empty()) {
        query.push(" AND c.id IN (SELECT contract_id FROM contract_tags ct JOIN tags t ON t.id = ct.tag_id WHERE t.name IN (");
        let mut separated = query.separated(", ");
        for tag in tags {
            separated.push_bind(tag);
        }
        separated.push_unseparated("))");
    }

    if let Some(maturity) = filters.maturity.as_ref() {
        query.push(" AND c.maturity::text = ");
        query.push_bind(maturity_filter_value(maturity));
    }

    if let Some(query_text) = filters
        .query
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        let search_pattern = format!("%{}%", query_text.to_ascii_lowercase());
        query.push(" AND (");
        query.push("c.search_vector @@ plainto_tsquery('english', ");
        query.push_bind(query_text.to_string());
        query.push(") OR lower(c.name) LIKE ");
        query.push_bind(search_pattern.clone());
        query.push(" OR lower(coalesce(c.description, '')) LIKE ");
        query.push_bind(search_pattern.clone());
        query.push(" OR lower(c.contract_id) LIKE ");
        query.push_bind(search_pattern);
        query.push(")");
    }

    if let Some(created_from) = filters.created_from {
        query.push(" AND c.created_at >= ");
        query.push_bind(created_from);
    }

    if let Some(created_to) = filters.created_to {
        query.push(" AND c.created_at <= ");
        query.push_bind(created_to);
    }

    if let Some(updated_from) = filters.updated_from {
        query.push(" AND c.updated_at >= ");
        query.push_bind(updated_from);
    }

    if let Some(updated_to) = filters.updated_to {
        query.push(" AND c.updated_at <= ");
        query.push_bind(updated_to);
    }

    if let Some(verified_from) = filters.verified_from {
        query.push(" AND c.verified_at >= ");
        query.push_bind(verified_from);
    }

    if let Some(verified_to) = filters.verified_to {
        query.push(" AND c.verified_at <= ");
        query.push_bind(verified_to);
    }

    if let Some(last_accessed_from) = filters.last_accessed_from {
        query.push(" AND c.last_accessed_at >= ");
        query.push_bind(last_accessed_from);
    }

    if let Some(last_accessed_to) = filters.last_accessed_to {
        query.push(" AND c.last_accessed_at <= ");
        query.push_bind(last_accessed_to);
    }
}

async fn count_contract_export_rows(
    state: &AppState,
    filters: &ContractSearchParams,
    claims: Option<&crate::auth::AuthClaims>,
) -> ApiResult<i64> {
    let mut query = QueryBuilder::<Postgres>::new("SELECT COUNT(*)");
    apply_contract_export_filters(&mut query, filters, claims);
    query
        .build_query_scalar()
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count contracts for export", err))
}

async fn fetch_contract_export_rows(
    state: &AppState,
    filters: &ContractSearchParams,
    claims: Option<&crate::auth::AuthClaims>,
) -> ApiResult<Vec<ContractMetadataExportRecord>> {
    let sort_by = filters.sort_by.clone().unwrap_or(shared::SortBy::CreatedAt);
    let sort_order = filters
        .sort_order
        .clone()
        .unwrap_or(shared::SortOrder::Desc);
    let direction = if sort_order == shared::SortOrder::Asc {
        "ASC"
    } else {
        "DESC"
    };

    let mut query = QueryBuilder::<Postgres>::new(
        "SELECT c.id, c.logical_id, c.contract_id, c.wasm_hash, c.name, c.description, c.publisher_id, \
         p.stellar_address AS publisher_stellar_address, p.username AS publisher_username, \
         c.network::text AS network, c.is_verified, c.category, c.maturity::text AS maturity, \
         c.health_score, c.is_maintenance, c.deployment_count, c.audit_status::text AS audit_status, \
         c.visibility::text AS visibility, c.organization_id, c.network_configs, c.created_at, \
         c.updated_at, c.verified_at, c.last_verified_at, c.last_accessed_at",
    );

    apply_contract_export_filters(&mut query, filters, claims);

    query.push(" ORDER BY ");
    match sort_by {
        shared::SortBy::UpdatedAt => {
            query.push("c.updated_at ");
        }
        shared::SortBy::VerifiedAt => {
            query.push("c.verified_at ");
        }
        shared::SortBy::LastAccessedAt => {
            query.push("c.last_accessed_at ");
        }
        shared::SortBy::Popularity | shared::SortBy::Interactions => {
            query.push("c.last_accessed_at ");
        }
        shared::SortBy::Deployments => {
            query.push("c.deployment_count ");
        }
        shared::SortBy::Relevance => {
            if let Some(query_text) = filters
                .query
                .as_ref()
                .filter(|value| !value.trim().is_empty())
            {
                query.push("ts_rank_cd(c.search_vector, plainto_tsquery('english', ");
                query.push_bind(query_text.to_string());
                query.push(")) ");
            } else {
                query.push("c.created_at ");
            }
        }
        shared::SortBy::CreatedAt => {
            query.push("c.created_at ");
        }
    };
    query.push(direction);
    query.push(" NULLS LAST, c.id ");
    query.push(direction);

    let mut records = query
        .build_query_as::<ContractMetadataExportRecord>()
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch contracts for export", err))?;

    // Fetch tags for these records
    let record_ids: Vec<Uuid> = records.iter().map(|r| r.id).collect();
    if !record_ids.is_empty() {
        #[derive(sqlx::FromRow)]
        struct TagRow {
            contract_id: Uuid,
            id: Uuid,
            name: String,
            color: String,
        }
        let tag_rows = match sqlx::query_as::<_, TagRow>(
            "SELECT ct.contract_id, t.id, t.name, t.color FROM tags t JOIN contract_tags ct ON t.id = ct.tag_id WHERE ct.contract_id = ANY($1)",
        )
        .bind(&record_ids)
        .fetch_all(&state.db)
        .await {
            Ok(rows) => rows,
            Err(err) => return Err(db_internal_error("fetch tags for export", err)),
        };

        let mut tags_map: HashMap<Uuid, Vec<shared::Tag>> = HashMap::new();
        for row in tag_rows {
            tags_map
                .entry(row.contract_id)
                .or_default()
                .push(shared::Tag {
                    id: row.id,
                    name: row.name,
                    color: row.color,
                });
        }

        for record in &mut records {
            if let Some(tags) = tags_map.remove(&record.id) {
                record.tags = tags;
            }
        }
    }

    Ok(records)
}

async fn generate_contract_export_payload(
    state: &AppState,
    filters: &ContractSearchParams,
    format: &ContractExportFormat,
    claims: Option<&crate::auth::AuthClaims>,
    async_export: bool,
) -> ApiResult<(String, i64)> {
    let contracts = fetch_contract_export_rows(state, filters, claims).await?;
    let metadata = ContractExportMetadata {
        exported_at: chrono::Utc::now(),
        format: format.clone(),
        total_count: contracts.len() as i64,
        async_export,
        filters: filters.clone(),
    };
    let rendered = render_contract_export(format, &metadata, &contracts)?;
    Ok((rendered, contracts.len() as i64))
}

async fn persist_contract_export_artifact(path: &StdPath, content: String) -> ApiResult<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|err| {
            ApiError::internal(format!("Failed to prepare export directory: {err}"))
        })?;
    }

    tokio::fs::write(path, content)
        .await
        .map_err(|err| ApiError::internal(format!("Failed to write export artifact: {err}")))?;
    Ok(())
}

#[utoipa::path(
    post,
    path = "/contracts/export",
    request_body = ContractExportRequest,
    responses(
        (status = 200, description = "Contract metadata export stream"),
        (status = 202, description = "Large export accepted", body = ContractExportAcceptedResponse),
        (status = 400, description = "Invalid export request")
    ),
    tag = "Contracts"
)]
pub async fn export_contract_metadata(
    State(state): State<AppState>,
    claims: Option<crate::auth::AuthClaims>,
    ValidatedJson(req): ValidatedJson<ContractExportRequest>,
) -> ApiResult<Response> {
    let filters = sanitized_export_filters(req.filters.clone());
    let total_count = count_contract_export_rows(&state, &filters, claims.as_ref()).await?;
    let should_run_async =
        req.async_mode.unwrap_or(false) || total_count > ASYNC_EXPORT_ROW_THRESHOLD;

    if should_run_async {
        let job_id = Uuid::new_v4();
        let job = ContractExportJob {
            job_id,
            status: ContractExportJobStatus::Pending,
            format: req.format.clone(),
            filters: filters.clone(),
            total_count,
            requested_at: chrono::Utc::now(),
            completed_at: None,
            artifact_path: None,
            error: None,
        };

        EXPORT_JOBS.write().await.insert(job_id, job.clone());

        let state_clone = state.clone();
        let filters_clone = filters.clone();
        let format_clone = req.format.clone();
        let claims_clone = claims.clone();

        tokio::spawn(async move {
            {
                let mut jobs = EXPORT_JOBS.write().await;
                if let Some(job) = jobs.get_mut(&job_id) {
                    job.status = ContractExportJobStatus::Processing;
                    job.error = None;
                }
            }

            let outcome = async {
                let (content, _) = generate_contract_export_payload(
                    &state_clone,
                    &filters_clone,
                    &format_clone,
                    claims_clone.as_ref(),
                    true,
                )
                .await?;
                let artifact_path = export_artifact_path(job_id, &format_clone);
                persist_contract_export_artifact(&artifact_path, content).await?;
                Ok::<PathBuf, ApiError>(artifact_path)
            }
            .await;

            let mut jobs = EXPORT_JOBS.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                match outcome {
                    Ok(artifact_path) => {
                        job.status = ContractExportJobStatus::Completed;
                        job.completed_at = Some(chrono::Utc::now());
                        job.artifact_path = Some(artifact_path);
                    }
                    Err(err) => {
                        tracing::error!(job_id = %job_id, error = %err, "contract export job failed");
                        job.status = ContractExportJobStatus::Failed;
                        job.completed_at = Some(chrono::Utc::now());
                        job.error = Some(err.to_string());
                    }
                }
            }
        });

        return Ok((
            StatusCode::ACCEPTED,
            Json(ContractExportAcceptedResponse {
                job_id,
                status: ContractExportJobStatus::Pending,
                status_url: format!("/contracts/export/{job_id}"),
                download_url: None,
                total_count,
                format: req.format,
                requested_at: job.requested_at,
                filters,
            }),
        )
            .into_response());
    }

    let (content, rendered_count) =
        generate_contract_export_payload(&state, &filters, &req.format, claims.as_ref(), false)
            .await?;
    Ok(contract_export_response(
        &req.format,
        rendered_count,
        content,
    ))
}

#[utoipa::path(
    get,
    path = "/contracts/export/{job_id}",
    params(
        ("job_id" = Uuid, Path, description = "Export job ID")
    ),
    responses(
        (status = 200, description = "Export job status", body = ContractExportStatusResponse),
        (status = 404, description = "Export job not found")
    ),
    tag = "Contracts"
)]
pub async fn get_contract_export_status(
    Path(job_id): Path<Uuid>,
) -> ApiResult<Json<ContractExportStatusResponse>> {
    let jobs = EXPORT_JOBS.read().await;
    let job = jobs.get(&job_id).ok_or_else(|| {
        ApiError::not_found("ExportNotFound", "No export job found for the supplied ID")
    })?;
    Ok(Json(build_export_status_response(job)))
}

pub async fn download_contract_export(Path(job_id): Path<Uuid>) -> ApiResult<Response> {
    let job = {
        let jobs = EXPORT_JOBS.read().await;
        jobs.get(&job_id).cloned()
    }
    .ok_or_else(|| {
        ApiError::not_found("ExportNotFound", "No export job found for the supplied ID")
    })?;

    match job.status {
        ContractExportJobStatus::Pending | ContractExportJobStatus::Processing => Err(
            ApiError::conflict("ExportNotReady", "The export is still being generated"),
        ),
        ContractExportJobStatus::Failed => {
            Err(ApiError::internal(job.error.unwrap_or_else(|| {
                "The export job failed before producing an artifact".to_string()
            })))
        }
        ContractExportJobStatus::Completed => {
            let artifact_path = job.artifact_path.ok_or_else(|| {
                ApiError::internal("The export job completed without an artifact path")
            })?;
            let content = tokio::fs::read_to_string(&artifact_path)
                .await
                .map_err(|err| {
                    ApiError::internal(format!("Failed to read export artifact: {err}"))
                })?;
            Ok(contract_export_response(
                &job.format,
                job.total_count,
                content,
            ))
        }
    }
}

/// Get a specific contract by ID. Optional ?network= returns network-specific config (Issue #43).
#[utoipa::path(
    get,
    path = "/api/contracts/{id}",
    params(
        ("id" = String, Path, description = "Contract UUID"),
        GetContractQuery
    ),
    responses(
        (status = 200, description = "Contract details", body = ContractGetResponse),
        (status = 404, description = "Contract not found"),
        (status = 400, description = "Invalid contract ID format")
    ),
    tag = "Contracts"
)]
pub async fn get_contract(
    State(state): State<AppState>,
    claims: Option<crate::auth::AuthClaims>,
    Path(id): Path<String>,
    Query(query): Query<GetContractQuery>,
) -> ApiResult<Json<ContractGetResponse>> {
    // Only cache public contract fetches (no auth context, no network override)
    // to avoid leaking private data through the cache.
    let cache_key = if claims.is_none() && query.network.is_none() {
        Some(id.clone())
    } else {
        None
    };

    if let Some(ref key) = cache_key {
        if let Some(cached) = state.cache.get_contract_meta(key).await {
            if let Ok(contract) = serde_json::from_str::<Contract>(&cached) {
                track_contract_access(&state, contract.id).await;
                return Ok(Json(ContractGetResponse {
                    contract,
                    current_network: None,
                    network_config: None,
                }));
            }
        }
    }

    let mut contract: Contract = if let Ok(contract_uuid) = Uuid::parse_str(&id) {
        sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
            .bind(contract_uuid)
            .fetch_one(&state.db)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => ApiError::not_found(
                    "ContractNotFound",
                    format!("No contract found with ID: {}", id),
                ),
                _ => db_internal_error("get contract by id", err),
            })?
    } else {
        // Fetch by slug
        let network = query.network.clone().unwrap_or(Network::Mainnet);
        sqlx::query_as("SELECT * FROM contracts WHERE slug = $1 AND network = $2")
            .bind(&id)
            .bind(&network)
            .fetch_one(&state.db)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => ApiError::not_found(
                    "ContractNotFound",
                    format!(
                        "No contract found with slug: {} on network: {}",
                        id, network
                    ),
                ),
                _ => db_internal_error("get contract by slug", err),
            })?
    };

    // Fetch tags
    #[derive(sqlx::FromRow)]
    struct TagRow {
        id: Uuid,
        name: String,
        color: String,
    }
    let tag_rows = match sqlx::query_as::<_, TagRow>(
        "SELECT t.id, t.name, t.color FROM tags t JOIN contract_tags ct ON t.id = ct.tag_id WHERE ct.contract_id = $1",
    )
    .bind(contract.id)
    .fetch_all(&state.db)
    .await {
        Ok(rows) => rows,
        Err(err) => return Err(db_internal_error("fetch tags", err)),
    };

    contract.tags = tag_rows
        .into_iter()
        .map(|r| shared::Tag {
            id: r.id,
            name: r.name,
            color: r.color,
        })
        .collect();

    // Cache public contracts after tags are populated
    if contract.visibility == shared::VisibilityType::Public {
        if let Some(ref key) = cache_key {
            if let Ok(serialized) = serde_json::to_string(&contract) {
                state.cache.put_contract_meta(key, serialized.clone()).await;
                // Also cache by the canonical contract_id string so both UUID and slug lookups hit
                if key != &contract.contract_id {
                    state
                        .cache
                        .put_contract_meta(&contract.contract_id, serialized)
                        .await;
                }
            }
        }
    }

    // Visibility check
    if contract.visibility == shared::VisibilityType::Private {
        let is_member = if let Some(ref claims) = claims {
            if let Some(org_id) = contract.organization_id {
                crate::org_handlers::check_org_role(
                    &state.db,
                    org_id,
                    &claims.sub,
                    shared::OrganizationRole::Viewer,
                )
                .await
                .is_ok()
            } else {
                false
            }
        } else {
            false
        };

        if !is_member {
            return Err(ApiError::forbidden_with_error(
                "AccessDenied",
                "This contract is private and you do not have access to it",
            ));
        }
    }

    let current_network = query.network.clone();
    let network_config = if let Some(ref net) = current_network {
        let configs: Option<std::collections::HashMap<String, NetworkConfig>> = contract
            .network_configs
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        let net_key = net.to_string();
        let config = configs.and_then(|m| m.get(&net_key).cloned());
        if let Some(ref cfg) = config {
            contract.contract_id = cfg.contract_id.clone();
            contract.is_verified = cfg.is_verified;
            contract.network = net.clone();
        }
        config
    } else {
        None
    };

    // Record search click if applicable
    if query.from_search.unwrap_or(false) {
        let _ = analytics::record_event(
            &state.db,
            shared::AnalyticsEventType::SearchClick,
            Some(contract.id),
            Some(contract.publisher_id),
            None,
            query.network.as_ref(),
            Some(serde_json::json!({
                "search_query": query.search_query,
                "timestamp": chrono::Utc::now()
            })),
        )
        .await;
    }
    track_contract_access(&state, contract.id).await;

    // Increment usage counter asynchronously (fire-and-forget)
    // Failures are logged but never block the main request
    let contract_id = contract.id;
    let db_clone = state.db.clone();
    tokio::spawn(async move {
        if let Err(err) =
            crate::usage_counter::increment_usage_counter_with_timeout(contract_id, &db_clone).await
        {
            tracing::warn!(
                contract_id = %contract_id,
                error = ?err,
                "Failed to increment usage counter for contract"
            );
        }
    });

    Ok(Json(ContractGetResponse {
        contract,
        current_network,
        network_config,
    }))
}

/// Fetch multiple contracts in a single request, preserving request order.
#[utoipa::path(
    post,
    path = "/api/contracts/batch",
    params(BatchContractsQuery),
    request_body = Vec<String>,
    responses(
        (status = 200, description = "Batch contract results in request order", body = [Object]),
        (status = 400, description = "Invalid request")
    ),
    tag = "Contracts"
)]
pub async fn get_contracts_batch(
    State(state): State<AppState>,
    Query(query): Query<BatchContractsQuery>,
    ValidatedJson(contract_ids): ValidatedJson<BatchContractIdsRequest>,
) -> ApiResult<Json<Vec<Option<Value>>>> {
    if contract_ids.len() > 100 {
        return Err(ApiError::bad_request(
            "BatchTooLarge",
            format!(
                "Maximum of 100 contract IDs allowed, received {}",
                contract_ids.len()
            ),
        ));
    }

    let fields = parse_batch_fields(query.fields.as_deref());

    let parsed_uuids: Vec<Uuid> = contract_ids
        .iter()
        .filter_map(|id| Uuid::parse_str(id.trim()).ok())
        .collect();

    let normalized_contract_ids: Vec<String> = contract_ids
        .iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    let contracts: Vec<Contract> = sqlx::query_as(
        "SELECT * FROM contracts
         WHERE id = ANY($1)
            OR contract_id = ANY($2)",
    )
    .bind(&parsed_uuids)
    .bind(&normalized_contract_ids)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("get batch contracts", err))?;

    let mut by_uuid: HashMap<Uuid, Contract> = HashMap::new();
    let mut by_contract_id: HashMap<String, Contract> = HashMap::new();

    for contract in contracts {
        by_contract_id.insert(contract.contract_id.clone(), contract.clone());
        by_uuid.insert(contract.id, contract);
    }

    let mut ordered_results: Vec<Option<Value>> = Vec::with_capacity(contract_ids.len());
    for requested in contract_ids {
        let trimmed = requested.trim();

        let contract = Uuid::parse_str(trimmed)
            .ok()
            .and_then(|id| by_uuid.get(&id))
            .or_else(|| by_contract_id.get(trimmed));

        ordered_results.push(contract.map(|c| contract_to_filtered_value(c, fields.as_ref())));
    }

    Ok(Json(ordered_results))
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/versions",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    responses(
        (status = 200, description = "List of contract versions", body = [ContractVersion]),
        (status = 404, description = "Contract not found"),
        (status = 400, description = "Invalid contract ID format")
    ),
    tag = "Versions"
)]
pub async fn get_contract_versions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<ContractVersion>>> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;

    let versions: Vec<ContractVersion> = sqlx::query_as(
        "SELECT * FROM contract_versions WHERE contract_id = $1 ORDER BY created_at DESC",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("get contract versions", err))?;

    Ok(Json(versions))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/contracts/:id/versions/:version  (Issue #486)
// ─────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/versions/{version}",
    params(
        ("id" = String, Path, description = "Contract UUID or on-chain contract_id"),
        ("version" = String, Path, description = "Semantic version string (e.g. 1.2.3)")
    ),
    responses(
        (status = 200, description = "Version details", body = ContractVersion),
        (status = 400, description = "Invalid contract ID"),
        (status = 404, description = "Version not found")
    ),
    tag = "Versions"
)]
pub async fn get_specific_contract_version(
    State(state): State<AppState>,
    Path((id, version)): Path<(String, String)>,
) -> ApiResult<Json<ContractVersion>> {
    let (contract_uuid, _) = fetch_contract_identity(&state, &id).await?;

    let version_row: Option<ContractVersion> =
        sqlx::query_as("SELECT * FROM contract_versions WHERE contract_id = $1 AND version = $2")
            .bind(contract_uuid)
            .bind(&version)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("get specific contract version", err))?;

    version_row.map(Json).ok_or_else(|| {
        ApiError::not_found(
            "VersionNotFound",
            format!("Version '{}' not found for contract {}", version, id),
        )
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/contracts/:id/versions/compare?from=v1&to=v2  (Issue #486)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct VersionCompareQuery {
    /// The base version to compare from
    pub from: String,
    /// The target version to compare to
    pub to: String,
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/versions/compare",
    params(
        ("id" = String, Path, description = "Contract UUID or on-chain contract_id"),
        VersionCompareQuery
    ),
    responses(
        (status = 200, description = "Version comparison result", body = shared::VersionCompareResponse),
        (status = 400, description = "Invalid contract ID or missing query params"),
        (status = 404, description = "One or both versions not found")
    ),
    tag = "Versions"
)]
pub async fn compare_contract_versions(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<VersionCompareQuery>,
) -> ApiResult<Json<shared::VersionCompareResponse>> {
    let (contract_uuid, _) = fetch_contract_identity(&state, &id).await?;

    let from_row: Option<ContractVersion> =
        sqlx::query_as("SELECT * FROM contract_versions WHERE contract_id = $1 AND version = $2")
            .bind(contract_uuid)
            .bind(&params.from)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch from-version for compare", err))?;

    let from_row = from_row.ok_or_else(|| {
        ApiError::not_found(
            "VersionNotFound",
            format!("Version '{}' not found for contract {}", params.from, id),
        )
    })?;

    let to_row: Option<ContractVersion> =
        sqlx::query_as("SELECT * FROM contract_versions WHERE contract_id = $1 AND version = $2")
            .bind(contract_uuid)
            .bind(&params.to)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch to-version for compare", err))?;

    let to_row = to_row.ok_or_else(|| {
        ApiError::not_found(
            "VersionNotFound",
            format!("Version '{}' not found for contract {}", params.to, id),
        )
    })?;

    let wasm_changed = from_row.wasm_hash != to_row.wasm_hash;

    let mut differences: Vec<shared::VersionFieldDiff> = Vec::new();

    if from_row.wasm_hash != to_row.wasm_hash {
        differences.push(shared::VersionFieldDiff {
            field: "wasm_hash".to_string(),
            from_value: Some(json!(from_row.wasm_hash)),
            to_value: Some(json!(to_row.wasm_hash)),
        });
    }
    if from_row.source_url != to_row.source_url {
        differences.push(shared::VersionFieldDiff {
            field: "source_url".to_string(),
            from_value: from_row.source_url.as_ref().map(|v| json!(v)),
            to_value: to_row.source_url.as_ref().map(|v| json!(v)),
        });
    }
    if from_row.commit_hash != to_row.commit_hash {
        differences.push(shared::VersionFieldDiff {
            field: "commit_hash".to_string(),
            from_value: from_row.commit_hash.as_ref().map(|v| json!(v)),
            to_value: to_row.commit_hash.as_ref().map(|v| json!(v)),
        });
    }
    if from_row.release_notes != to_row.release_notes {
        differences.push(shared::VersionFieldDiff {
            field: "release_notes".to_string(),
            from_value: from_row.release_notes.as_ref().map(|v| json!(v)),
            to_value: to_row.release_notes.as_ref().map(|v| json!(v)),
        });
    }
    if from_row.change_notes != to_row.change_notes {
        differences.push(shared::VersionFieldDiff {
            field: "change_notes".to_string(),
            from_value: from_row.change_notes.as_ref().map(|v| json!(v)),
            to_value: to_row.change_notes.as_ref().map(|v| json!(v)),
        });
    }

    Ok(Json(shared::VersionCompareResponse {
        contract_id: contract_uuid,
        from_version: from_row,
        to_version: to_row,
        differences,
        wasm_changed,
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/admin/contracts/:id/versions/:version/revert  (Issue #486)
// Admin-only: revert a contract to a previous version by creating a new version
// that carries the same wasm_hash as the target, marked as a revert.
// ─────────────────────────────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/admin/contracts/{id}/versions/{version}/revert",
    params(
        ("id" = String, Path, description = "Contract UUID or on-chain contract_id"),
        ("version" = String, Path, description = "Version to revert to")
    ),
    request_body = shared::RevertVersionRequest,
    responses(
        (status = 201, description = "Revert version created", body = ContractVersion),
        (status = 400, description = "Invalid input"),
        (status = 404, description = "Contract or version not found")
    ),
    tag = "Versions"
)]
pub async fn revert_contract_version(
    State(state): State<AppState>,
    Path((id, target_version)): Path<(String, String)>,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<shared::RevertVersionRequest>,
) -> ApiResult<Json<ContractVersion>> {
    let (contract_uuid, contract_id) = fetch_contract_identity(&state, &id).await?;

    // Fetch the version we are reverting to.
    let target: Option<ContractVersion> =
        sqlx::query_as("SELECT * FROM contract_versions WHERE contract_id = $1 AND version = $2")
            .bind(contract_uuid)
            .bind(&target_version)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch target version for revert", err))?;

    let target = target.ok_or_else(|| {
        ApiError::not_found(
            "VersionNotFound",
            format!(
                "Version '{}' not found for contract {}",
                target_version, contract_id
            ),
        )
    })?;

    // Determine the current latest version so we can compute the next patch bump.
    let existing_versions: Vec<String> =
        sqlx::query_scalar("SELECT version FROM contract_versions WHERE contract_id = $1")
            .bind(contract_uuid)
            .fetch_all(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch versions for revert bump", err))?;

    let new_version = {
        let mut parsed: Vec<SemVer> = existing_versions
            .iter()
            .filter_map(|v| SemVer::parse(v))
            .collect();
        parsed.sort();
        match parsed.last() {
            Some(latest) => {
                // Bump the patch component of the latest version.
                format!("{}.{}.{}", latest.major, latest.minor, latest.patch + 1)
            }
            None => "0.0.1".to_string(),
        }
    };

    let change_notes = req
        .change_notes
        .unwrap_or_else(|| format!("Reverted to version {}", target_version));

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| db_internal_error("begin revert transaction", err))?;

    let version_row: ContractVersion = sqlx::query_as(
        "INSERT INTO contract_versions \
            (contract_id, version, wasm_hash, source_url, commit_hash, release_notes, change_notes, is_revert, reverted_from) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE, $8) \
         RETURNING *",
    )
    .bind(contract_uuid)
    .bind(&new_version)
    .bind(&target.wasm_hash)
    .bind(&target.source_url)
    .bind(&target.commit_hash)
    .bind(Option::<String>::None) // release_notes left empty for reverts
    .bind(&change_notes)
    .bind(&target_version)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| db_internal_error("insert revert version", err))?;

    sqlx::query("UPDATE contracts SET current_version = $2 WHERE id = $1")
        .bind(contract_uuid)
        .bind(&new_version)
        .execute(&mut *tx)
        .await
        .map_err(|err| db_internal_error("update current_version on revert", err))?;

    tx.commit()
        .await
        .map_err(|err| db_internal_error("commit revert transaction", err))?;

    write_contract_audit_log(
        &state.db,
        AuditActionType::Rollback,
        contract_uuid,
        req.admin_id,
        json!({
            "reverted_to_version": target_version,
            "new_version": new_version,
            "change_notes": change_notes
        }),
        &extract_ip_address(&headers),
    )
    .await
    .map_err(|err| db_internal_error("write revert audit log", err))?;

    state.cache.invalidate_abi(&contract_id).await;
    state.cache.invalidate_abi(&contract_uuid.to_string()).await;
    state.cache.invalidate_contracts().await;
    state.cache.invalidate_contract_meta(&contract_id).await;
    state
        .cache
        .invalidate_contract_meta(&contract_uuid.to_string())
        .await;

    Ok(Json(version_row))
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct UploadContractSourceRequest {
    pub source_base64: String,
    pub source_format: String,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ContractSourceResponse {
    pub id: Uuid,
    pub contract_version_id: Uuid,
    pub source_format: String,
    pub storage_backend: String,
    pub storage_key: String,
    pub source_hash: String,
    pub source_size: i64,
    pub source_base64: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ContractChangelogEntry {
    pub version: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub commit_hash: Option<String>,
    pub source_url: Option<String>,
    pub release_notes: Option<String>,
    pub breaking: bool,
    pub breaking_changes: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ContractChangelogResponse {
    pub contract_id: Uuid,
    pub entries: Vec<ContractChangelogEntry>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::IntoParams)]
pub struct ContractSourceQuery {
    #[serde(default)]
    pub source_format: Option<String>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ContractSourceDiffResponse {
    pub base_version: String,
    pub target_version: String,
    pub source_format: String,
    pub diff: String,
}

#[utoipa::path(
    post,
    path = "/api/contracts/{id}/versions/{version}/source",
    params(
        ("id" = String, Path, description = "Contract UUID"),
        ("version" = String, Path, description = "Contract version")
    ),
    request_body = UploadContractSourceRequest,
    responses(
        (status = 201, description = "Source uploaded", body = ContractSourceResponse),
        (status = 400, description = "Invalid input"),
        (status = 404, description = "Contract version not found")
    ),
    tag = "Source"
)]
pub async fn upload_contract_source(
    State(state): State<AppState>,
    Path((id, version)): Path<(String, String)>,
    ValidatedJson(req): ValidatedJson<UploadContractSourceRequest>,
) -> ApiResult<Json<ContractSourceResponse>> {
    let (contract_uuid, contract_id) = fetch_contract_identity(&state, &id).await?;

    let version_row: Option<ContractVersion> =
        sqlx::query_as("SELECT * FROM contract_versions WHERE contract_id = $1 AND version = $2")
            .bind(contract_uuid)
            .bind(&version)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch contract version", err))?;

    let version_row = version_row.ok_or_else(|| {
        ApiError::not_found(
            "ContractVersionNotFound",
            format!(
                "Version '{}' not found for contract {}",
                version, contract_id
            ),
        )
    })?;

    let source_bytes = BASE64
        .decode(&req.source_base64)
        .map_err(|_| ApiError::bad_request("InvalidBase64", "source_base64 must be base64"))?;

    let source_format = match req.source_format.to_lowercase().as_str() {
        "rust" => SourceFormat::Rust,
        "wasm" => SourceFormat::Wasm,
        other => {
            return Err(ApiError::bad_request(
                "InvalidSourceFormat",
                format!(
                    "Unsupported source format '{}', expected 'rust' or 'wasm'",
                    other
                ),
            ))
        }
    };

    let (backend, storage_key, source_hash) = state
        .source_storage
        .store_source(&contract_id, &version, source_format.clone(), &source_bytes)
        .await
        .map_err(|e| ApiError::internal(format!("source storage error: {}", e)))?;

    let source_size = source_bytes.len() as i64;

    let source_row: ContractSource = sqlx::query_as(
        "INSERT INTO contract_sources (contract_version_id, source_format, storage_backend, storage_key, source_hash, source_size) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
    )
    .bind(version_row.id)
    .bind(source_format.to_string())
    .bind(&backend)
    .bind(&storage_key)
    .bind(&source_hash)
    .bind(source_size)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("insert contract source", err))?;

    sqlx::query(
        "INSERT INTO source_access_logs (contract_source_id, action, actor, request_ip, user_agent, details) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(source_row.id)
    .bind("upload")
    .bind(None::<String>)
    .bind(None::<String>)
    .bind(None::<String>)
    .bind(None::<serde_json::Value>)
    .execute(&state.db)
    .await
    .map_err(|err| db_internal_error("insert source access log", err))?;

    Ok(Json(ContractSourceResponse {
        id: source_row.id,
        contract_version_id: source_row.contract_version_id,
        source_format: source_row.source_format.to_string(),
        storage_backend: source_row.storage_backend,
        storage_key: source_row.storage_key,
        source_hash: source_row.source_hash,
        source_size: source_row.source_size,
        source_base64: Some(req.source_base64),
        created_at: source_row.created_at,
    }))
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/versions/{version}/source",
    params(
        ("id" = String, Path, description = "Contract UUID"),
        ("version" = String, Path, description = "Contract version")
    ),
    responses(
        (status = 200, description = "Retrieve source", body = ContractSourceResponse),
        (status = 404, description = "Source not found"),
        (status = 500, description = "Integrity verification failed")
    ),
    tag = "Source"
)]
pub async fn get_contract_source(
    State(state): State<AppState>,
    Path((id, version)): Path<(String, String)>,
    Query(query): Query<ContractSourceQuery>,
) -> ApiResult<Json<ContractSourceResponse>> {
    let (contract_uuid, _contract_id) = fetch_contract_identity(&state, &id).await?;

    let version_row: ContractVersion =
        sqlx::query_as("SELECT * FROM contract_versions WHERE contract_id = $1 AND version = $2")
            .bind(contract_uuid)
            .bind(&version)
            .fetch_one(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch contract version", err))?;

    let format = query
        .source_format
        .as_deref()
        .unwrap_or("rust")
        .to_lowercase();

    let source_format = match format.as_str() {
        "rust" => SourceFormat::Rust,
        "wasm" => SourceFormat::Wasm,
        other => {
            return Err(ApiError::bad_request(
                "InvalidSourceFormat",
                format!(
                    "Unsupported source format '{}', expected 'rust' or 'wasm'",
                    other
                ),
            ))
        }
    };

    let source_row: ContractSource = sqlx::query_as(
        "SELECT * FROM contract_sources WHERE contract_version_id = $1 AND source_format = $2",
    )
    .bind(version_row.id)
    .bind(source_format.to_string())
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch contract source", err))?;

    let source_bytes = state
        .source_storage
        .retrieve_source(&source_row.storage_backend, &source_row.storage_key)
        .await
        .map_err(|e| ApiError::internal(format!("source storage error: {}", e)))?;

    let check_hash = shared::source_storage::compute_sha256(&source_bytes);
    if check_hash != source_row.source_hash {
        return Err(ApiError::internal("Contract source integrity check failed"));
    }

    sqlx::query(
        "INSERT INTO source_access_logs (contract_source_id, action, actor, request_ip, user_agent, details) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(source_row.id)
    .bind("retrieve")
    .bind(None::<String>)
    .bind(None::<String>)
    .bind(None::<String>)
    .bind(Some(json!({"ip": "unknown"})))
    .execute(&state.db)
    .await
    .map_err(|err| db_internal_error("insert source access log", err))?;

    Ok(Json(ContractSourceResponse {
        id: source_row.id,
        contract_version_id: source_row.contract_version_id,
        source_format: source_row.source_format.to_string(),
        storage_backend: source_row.storage_backend,
        storage_key: source_row.storage_key,
        source_hash: source_row.source_hash,
        source_size: source_row.source_size,
        source_base64: Some(BASE64.encode(source_bytes)),
        created_at: source_row.created_at,
    }))
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/versions/{version}/source/diff",
    params(
        ("id" = String, Path, description = "Contract UUID"),
        ("version" = String, Path, description = "Contract version"),
        ("compare_version" = String, Query, description = "Contract version to compare against")
    ),
    responses(
        (status = 200, description = "Source diff", body = ContractSourceDiffResponse),
        (status = 404, description = "Version/source not found"),
        (status = 400, description = "Invalid input")
    ),
    tag = "Source"
)]
pub async fn get_contract_source_diff(
    State(state): State<AppState>,
    Path((id, version)): Path<(String, String)>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<Json<ContractSourceDiffResponse>> {
    let (contract_uuid, _contract_id) = fetch_contract_identity(&state, &id).await?;

    let compare_version = params
        .get("compare_version")
        .ok_or_else(|| {
            ApiError::bad_request("MissingCompareVersion", "compare_version is required")
        })?
        .to_string();

    async fn load_source(
        state: &AppState,
        contract_uuid: Uuid,
        version: &str,
        source_format: &str,
    ) -> Result<(String, Uuid), ApiError> {
        let version_row: ContractVersion = sqlx::query_as(
            "SELECT * FROM contract_versions WHERE contract_id = $1 AND version = $2",
        )
        .bind(contract_uuid)
        .bind(version)
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch contract version", err))?;

        let sf = match source_format {
            "rust" => SourceFormat::Rust,
            "wasm" => SourceFormat::Wasm,
            _ => {
                return Err(ApiError::bad_request(
                    "InvalidSourceFormat",
                    "source_format must be 'rust' or 'wasm'",
                ));
            }
        };

        let source_row: ContractSource = sqlx::query_as(
            "SELECT * FROM contract_sources WHERE contract_version_id = $1 AND source_format = $2",
        )
        .bind(version_row.id)
        .bind(sf.to_string())
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch contract source", err))?;

        let bytes = state
            .source_storage
            .retrieve_source(&source_row.storage_backend, &source_row.storage_key)
            .await
            .map_err(|e| ApiError::internal(format!("source storage error: {}", e)))?;

        let check_hash = shared::source_storage::compute_sha256(&bytes);
        if check_hash != source_row.source_hash {
            return Err(ApiError::internal("Contract source integrity check failed"));
        }

        let data_str = String::from_utf8(bytes)
            .map_err(|_| ApiError::internal("Source content is not UTF-8 serializable"))?;

        Ok((data_str, source_row.id))
    }

    let source_format = params
        .get("source_format")
        .map(|s| s.as_str())
        .unwrap_or("rust");

    let (base_source, base_source_id) =
        load_source(&state, contract_uuid, &version, source_format).await?;
    let (compare_source, _compare_source_id) =
        load_source(&state, contract_uuid, &compare_version, source_format).await?;

    let diff = difference::Changeset::new(&compare_source, &base_source, "\n");
    let diff_text = diff
        .diffs
        .iter()
        .map(|chunk| match chunk {
            difference::Difference::Same(txt) => format!(" {}", txt),
            difference::Difference::Add(txt) => format!("+{}", txt),
            difference::Difference::Rem(txt) => format!("-{}", txt),
        })
        .collect::<Vec<_>>()
        .join("\n");

    sqlx::query(
        "INSERT INTO source_access_logs (contract_source_id, action, actor, request_ip, user_agent, details) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(base_source_id)
    .bind("diff")
    .bind(None::<String>)
    .bind(None::<String>)
    .bind(None::<String>)
    .bind(Some(json!({"compare_version": compare_version, "target_version": version})))
    .execute(&state.db)
    .await
    .map_err(|err| db_internal_error("insert source access log", err))?;

    Ok(Json(ContractSourceDiffResponse {
        base_version: compare_version,
        target_version: version,
        source_format: source_format.to_string(),
        diff: diff_text,
    }))
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/changelog",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    responses(
        (status = 200, description = "Contract changelog with breaking-change markers", body = ContractChangelogResponse),
        (status = 400, description = "Invalid contract ID format"),
        (status = 404, description = "Contract not found")
    ),
    tag = "Versions"
)]
/// GET /api/contracts/:id/changelog (and /contracts/:id/changelog) — release history with breaking-change markers.
pub async fn get_contract_changelog(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ContractChangelogResponse>> {
    let (contract_uuid, contract_id) = fetch_contract_identity(&state, &id).await?;

    // Ascending order makes it easy to compute diffs against the previous version.
    let versions: Vec<ContractVersion> = sqlx::query_as(
        "SELECT * FROM contract_versions WHERE contract_id = $1 ORDER BY created_at ASC",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("get contract versions for changelog", err))?;

    let mut entries: Vec<ContractChangelogEntry> = Vec::with_capacity(versions.len());

    let mut prev_version: Option<String> = None;
    for v in &versions {
        let mut breaking = false;
        let mut breaking_changes: Vec<String> = Vec::new();

        if let Some(prev) = prev_version.as_deref() {
            let old_selector = format!("{}@{}", contract_id, prev);
            let new_selector = format!("{}@{}", contract_id, v.version);

            // Note: For internal resolution in handlers, we generally don't bypass unless requested.
            // But these specific calls are for diffing, so we use default false.
            let old_abi = resolve_abi(&state, &old_selector, false).await?;
            let new_abi = resolve_abi(&state, &new_selector, false).await?;

            let old_spec =
                contract_abi::parser::parse_json_spec(&old_abi, &old_selector).map_err(|e| {
                    ApiError::bad_request("InvalidABI", format!("Failed to parse old ABI: {}", e))
                })?;
            let new_spec =
                contract_abi::parser::parse_json_spec(&new_abi, &new_selector).map_err(|e| {
                    ApiError::bad_request("InvalidABI", format!("Failed to parse new ABI: {}", e))
                })?;

            let changes = diff_abi(&old_spec, &new_spec);
            breaking = has_breaking_changes(&changes);
            breaking_changes = changes
                .into_iter()
                .filter(|c| c.severity == crate::breaking_changes::ChangeSeverity::Breaking)
                .map(|c| c.message)
                .collect();
        }

        entries.push(ContractChangelogEntry {
            version: v.version.clone(),
            created_at: v.created_at,
            commit_hash: v.commit_hash.clone(),
            source_url: v.source_url.clone(),
            release_notes: v.release_notes.clone(),
            breaking,
            breaking_changes,
        });

        prev_version = Some(v.version.clone());
    }

    // Most APIs return newest-first for timelines.
    entries.reverse();

    Ok(Json(ContractChangelogResponse {
        contract_id: contract_uuid,
        entries,
    }))
}

#[utoipa::path(
    post,
    path = "/api/contracts/{id}/versions",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    request_body = CreateContractVersionRequest,
    responses(
        (status = 201, description = "Version created successfully", body = ContractVersion),
        (status = 400, description = "Invalid input or version conflict"),
        (status = 404, description = "Contract not found")
    ),
    tag = "Versions"
)]
pub async fn create_contract_version(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<CreateContractVersionRequest>,
) -> ApiResult<Json<ContractVersion>> {
    let (contract_uuid, contract_id) = fetch_contract_identity(&state, &id).await?;
    if !req.contract_id.trim().is_empty() && req.contract_id != contract_id {
        return Err(ApiError::bad_request(
            "ContractMismatch",
            "Contract ID in payload does not match path",
        ));
    }

    let new_version = SemVer::parse(&req.version).ok_or_else(|| {
        ApiError::bad_request(
            "InvalidVersion",
            "Version must be valid semver (e.g. 1.2.3)",
        )
    })?;

    // Optional Ed25519 signature verification for this contract version.
    // When a signature is provided, we require a matching publisher_key and
    // verify the detached signature over "{contract_id}:{version}:{wasm_hash}".
    let (version_signature, version_publisher_key, version_algorithm) =
        match (&req.signature, &req.publisher_key) {
            (Some(sig), Some(pk)) if !sig.trim().is_empty() && !pk.trim().is_empty() => {
                // Decode public key (base64, 32 bytes)
                let pk_bytes = BASE64.decode(pk.trim()).map_err(|_| {
                    ApiError::bad_request(
                        "InvalidPublisherKey",
                        "publisher_key must be valid base64-encoded Ed25519 public key",
                    )
                })?;
                let pk_array: [u8; 32] = pk_bytes.as_slice().try_into().map_err(|_| {
                    ApiError::bad_request(
                        "InvalidPublisherKey",
                        "publisher_key must decode to 32 bytes",
                    )
                })?;
                let verifying_key = VerifyingKey::from_bytes(&pk_array).map_err(|_| {
                    ApiError::bad_request(
                        "InvalidPublisherKey",
                        "publisher_key is not a valid Ed25519 public key",
                    )
                })?;

                // Decode signature (base64, 64 bytes)
                let sig_bytes = BASE64.decode(sig.trim()).map_err(|_| {
                    ApiError::bad_request(
                        "InvalidSignature",
                        "signature must be valid base64-encoded Ed25519 signature",
                    )
                })?;
                let sig_array: [u8; 64] = sig_bytes.as_slice().try_into().map_err(|_| {
                    ApiError::bad_request("InvalidSignature", "signature must decode to 64 bytes")
                })?;
                let signature = Signature::from_bytes(&sig_array);

                // Construct signing message and verify
                let message = crate::signing_handlers::create_signing_message(
                    &req.wasm_hash,
                    &contract_id,
                    &req.version,
                );

                let crypto_valid = verifying_key.verify(&message, &signature).is_ok();
                if !crypto_valid {
                    return Err(ApiError::unprocessable(
                        "InvalidSignature",
                        "Ed25519 signature verification failed for this contract version",
                    ));
                }

                let algo = req
                    .signature_algorithm
                    .clone()
                    .unwrap_or_else(|| "ed25519".to_string());

                tracing::info!(
                    contract_id = %contract_id,
                    version = %req.version,
                    wasm_hash = %req.wasm_hash,
                    "contract version signature verified"
                );

                (
                    Some(sig.trim().to_string()),
                    Some(pk.trim().to_string()),
                    Some(algo),
                )
            }
            (None, None) => {
                // No signature metadata provided – proceed without cryptographic binding.
                (None, None, None)
            }
            (Some(s), None) if s.trim().is_empty() => (None, None, None),
            (None, Some(pk)) if pk.trim().is_empty() => (None, None, None),
            _ => {
                return Err(ApiError::bad_request(
                    "InvalidSignatureMetadata",
                    "signature and publisher_key must both be provided (or both omitted)",
                ));
            }
        };

    let existing_versions: Vec<String> =
        sqlx::query_scalar("SELECT version FROM contract_versions WHERE contract_id = $1")
            .bind(contract_uuid)
            .fetch_all(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch contract versions", err))?;

    // prev_snapshot is populated when there is a prior version; used for delta storage.
    let mut prev_snapshot: Option<crate::patch_handlers::VersionSnapshot> = None;

    if !existing_versions.is_empty() {
        let mut parsed: Vec<SemVer> = Vec::with_capacity(existing_versions.len());
        for version in &existing_versions {
            let parsed_version = SemVer::parse(version).ok_or_else(|| {
                ApiError::unprocessable(
                    "InvalidExistingVersion",
                    format!("Existing version '{}' is not valid semver", version),
                )
            })?;
            parsed.push(parsed_version);
        }
        parsed.sort();
        let latest_version = parsed.last().cloned();

        if let Some(old_version) = latest_version {
            let old_selector = format!("{}@{}", contract_id, old_version);
            let old_abi = resolve_abi(&state, &old_selector, false).await?;
            let old_spec =
                contract_abi::parser::parse_json_spec(&old_abi, &contract_id).map_err(|e| {
                    ApiError::bad_request("InvalidABI", format!("Failed to parse old ABI: {}", e))
                })?;

            let new_spec = contract_abi::parser::parse_json_spec(
                &req.abi.to_string(),
                &contract_id,
            )
            .map_err(|e| {
                ApiError::bad_request("InvalidABI", format!("Failed to parse new ABI: {}", e))
            })?;

            let changes = diff_abi(&old_spec, &new_spec);
            if has_breaking_changes(&changes) && new_version.major == old_version.major {
                return Err(ApiError::unprocessable(
                    "BreakingChangeWithoutMajorBump",
                    format!(
                        "Breaking changes detected; bump major version from {} to {}",
                        old_version, new_version
                    ),
                ));
            }

            // Fetch the previous version row for delta computation.
            let old_ver_str = old_version.to_string();
            #[derive(sqlx::FromRow)]
            struct PrevRow {
                wasm_hash: String,
                source_url: Option<String>,
                commit_hash: Option<String>,
                release_notes: Option<String>,
                state_schema: Option<serde_json::Value>,
            }
            let prev_row: Option<PrevRow> = sqlx::query_as(
                "SELECT wasm_hash, source_url, commit_hash, release_notes, state_schema \
                 FROM contract_versions \
                 WHERE contract_id = $1 AND version = $2",
            )
            .bind(contract_uuid)
            .bind(&old_ver_str)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch previous version for patch", err))?;

            if let Some(pr) = prev_row {
                let old_abi_json: serde_json::Value =
                    serde_json::from_str(&old_abi).unwrap_or(serde_json::Value::Null);
                prev_snapshot = Some(crate::patch_handlers::VersionSnapshot {
                    version: old_ver_str,
                    wasm_hash: pr.wasm_hash,
                    source_url: pr.source_url,
                    commit_hash: pr.commit_hash,
                    release_notes: pr.release_notes,
                    state_schema: pr.state_schema,
                    abi: old_abi_json,
                });
            }
        }
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| db_internal_error("begin transaction", err))?;

    let version_row: ContractVersion = sqlx::query_as(
        "INSERT INTO contract_versions \
            (contract_id, version, wasm_hash, source_url, commit_hash, release_notes, change_notes, signature, publisher_key, signature_algorithm) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         RETURNING *",
    )
    .bind(contract_uuid)
    .bind(&req.version)
    .bind(&req.wasm_hash)
    .bind(&req.source_url)
    .bind(&req.commit_hash)
    .bind(&req.release_notes)
    .bind(&req.change_notes)
    .bind(&version_signature)
    .bind(&version_publisher_key)
    .bind(&version_algorithm)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| match err {
        sqlx::Error::Database(db_err)
            if db_err.constraint() == Some("contract_versions_contract_id_version_key") =>
        {
            ApiError::unprocessable(
                "VersionAlreadyExists",
                format!("Version '{}' already exists for this contract", req.version),
            )
        }
        _ => db_internal_error("insert contract version", err),
    })?;

    // Create verification task for the new version (Issue # validator network)
    sqlx::query(
        "INSERT INTO verification_tasks (contract_id, wasm_hash, status) VALUES ($1, $2, 'pending') ON CONFLICT DO NOTHING"
    )
    .bind(contract_uuid)
    .bind(&req.wasm_hash)
    .execute(&mut *tx)
    .await
    .map_err(|err| db_internal_error("create verification task", err))?;

    sqlx::query(
        "INSERT INTO contract_abis (contract_id, version, abi) VALUES ($1, $2, $3) \
         ON CONFLICT (contract_id, version) DO UPDATE SET abi = EXCLUDED.abi",
    )
    .bind(contract_uuid)
    .bind(&req.version)
    .bind(&req.abi)
    .execute(&mut *tx)
    .await
    .map_err(|err| db_internal_error("insert contract abi", err))?;

    sqlx::query(
        "UPDATE contracts SET deployment_count = deployment_count + 1, current_version = $2 WHERE id = $1",
    )
    .bind(contract_uuid)
    .bind(&req.version)
    .execute(&mut *tx)
    .await
    .map_err(|err| db_internal_error("update current_version", err))?;

    tx.commit()
        .await
        .map_err(|err| db_internal_error("commit contract version", err))?;

    state.cache.invalidate_abi(&contract_id).await;
    state.cache.invalidate_abi(&contract_uuid.to_string()).await;
    state
        .cache
        .invalidate_abi(&format!("{}@{}", contract_id, req.version))
        .await;
    state.cache.invalidate_contract_meta(&contract_id).await;
    state
        .cache
        .invalidate_contract_meta(&contract_uuid.to_string())
        .await;

    // Store differential patch for the new version (Issue #501).
    let new_snapshot = crate::patch_handlers::VersionSnapshot {
        version: req.version.clone(),
        wasm_hash: req.wasm_hash.clone(),
        source_url: req.source_url.clone(),
        commit_hash: req.commit_hash.clone(),
        release_notes: req.release_notes.clone(),
        state_schema: None,
        abi: req.abi.clone(),
    };
    if let Err(e) = crate::patch_handlers::store_patch(
        &state.db,
        contract_uuid,
        prev_snapshot.as_ref(),
        &new_snapshot,
    )
    .await
    {
        tracing::error!(
            "Failed to store differential patch for version {}: {}",
            req.version,
            e
        );
    }

    // Post-commit dependency analysis
    let detected_deps = dependency::detect_dependencies_from_abi(&req.abi);
    if !detected_deps.is_empty() {
        if let Err(e) =
            dependency::save_dependencies(&state.db, contract_uuid, &detected_deps).await
        {
            tracing::error!(
                "Failed to save dependencies for version {}: {}",
                req.version,
                e
            );
        }
        // Invalidate global graph cache
        state
            .cache
            .invalidate("system", "global:dependency_graph")
            .await;
    }

    let _ = analytics::record_event(
        &state.db,
        AnalyticsEventType::VersionCreated,
        Some(version_row.contract_id),
        None, // Version creation usually doesn't need publisher_id explicitly if we have contract_id
        None,
        None,
        Some(json!({ "version": version_row.version })),
    )
    .await;

    if let Ok(contract) = sqlx::query_as::<_, Contract>("SELECT * FROM contracts WHERE id = $1")
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
    {
        state
            .contract_events
            .publish(ContractEventEnvelope::version_created(
                &contract,
                &version_row,
            ));
    }
    state.cache.invalidate_contracts().await;

    // Increment usage counter asynchronously (fire-and-forget)
    // Failures are logged but never block the main request
    let db_clone = state.db.clone();
    tokio::spawn(async move {
        if let Err(err) =
            crate::usage_counter::increment_usage_counter_with_timeout(contract_uuid, &db_clone)
                .await
        {
            tracing::warn!(
                contract_id = %contract_uuid,
                error = ?err,
                "Failed to increment usage counter for contract"
            );
        }
    });

    Ok(Json(version_row))
}

async fn ensure_contract_exists(
    state: &AppState,
    contract_uuid: Uuid,
    contract_id_raw: &str,
    operation: &str,
) -> ApiResult<()> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM contracts WHERE id = $1)")
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error(operation, err))?;

    if exists {
        Ok(())
    } else {
        Err(ApiError::not_found(
            "ContractNotFound",
            format!("No contract found with ID: {}", contract_id_raw),
        ))
    }
}

async fn fetch_contract_network(
    state: &AppState,
    contract_uuid: Uuid,
    contract_id_raw: &str,
    operation: &str,
) -> ApiResult<Network> {
    let network: Option<Network> =
        sqlx::query_scalar("SELECT network FROM contracts WHERE id = $1")
            .bind(contract_uuid)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error(operation, err))?;

    network.ok_or_else(|| {
        ApiError::not_found(
            "ContractNotFound",
            format!("No contract found with ID: {}", contract_id_raw),
        )
    })
}

async fn generate_unique_slug(
    db: &sqlx::PgPool,
    name: &str,
    network: &Network,
    requested_slug: Option<String>,
) -> ApiResult<String> {
    let base_slug = requested_slug
        .map(|s| shared::slugify(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| shared::slugify(name));

    if base_slug.is_empty() {
        return Err(ApiError::bad_request(
            "InvalidSlug",
            "Contract name must contain alphanumeric characters to generate a slug",
        ));
    }

    let mut slug = base_slug.clone();
    let mut counter = 1;

    loop {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM contracts WHERE slug = $1 AND network = $2)",
        )
        .bind(&slug)
        .bind(network)
        .fetch_one(db)
        .await
        .map_err(|err| db_internal_error("check slug existence", err))?;

        if !exists {
            return Ok(slug);
        }

        slug = format!("{}-{}", base_slug, counter);
        counter += 1;

        if counter > 100 {
            return Err(ApiError::internal(
                "Failed to generate a unique slug after 100 attempts",
            ));
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/contracts",
    request_body = PublishRequest,
    responses(
        (status = 201, description = "Contract published successfully", body = Contract),
        (status = 400, description = "Invalid input or contract ID"),
        (status = 409, description = "Contract already registered")
    ),
    tag = "Contracts"
)]
pub async fn publish_contract(
    State(state): State<AppState>,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<PublishRequest>,
) -> ApiResult<Json<Contract>> {
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| db_internal_error("begin publish tx", err))?;

    let publisher: Publisher = sqlx::query_as(
        "INSERT INTO publishers (stellar_address) VALUES ($1)
         ON CONFLICT (stellar_address) DO UPDATE SET stellar_address = EXCLUDED.stellar_address
         RETURNING *",
    )
    .bind(&req.publisher_address)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| db_internal_error("upsert publisher", err))?;

    let wasm_hash = req.wasm_hash.clone();
    let network_key = req.network.to_string();
    let mut config_map = serde_json::Map::new();
    config_map.insert(
        network_key,
        serde_json::json!({
            "contract_id": req.contract_id,
            "is_verified": false,
            "min_version": null,
            "max_version": null
        }),
    );
    let network_configs = serde_json::Value::Object(config_map);

    let slug = generate_unique_slug(&state.db, &req.name, &req.network, req.slug.clone()).await?;

    let contract: Contract = sqlx::query_as(
        "INSERT INTO contracts (contract_id, wasm_hash, name, slug, description, publisher_id, network, category, tags, logical_id, network_configs)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         RETURNING *"
    )
    .bind(&req.contract_id)
    .bind(&wasm_hash)
    .bind(&req.name)
    .bind(&slug)
    .bind(&req.description)
    .bind(publisher.id)
    .bind(&req.network)
    .bind(&req.category)
    .bind(&req.tags)
    .bind(Option::<Uuid>::None as Option<Uuid>)
    .bind(&network_configs)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if let sqlx::Error::Database(ref e) = err {
            if e.constraint() == Some("contracts_contract_id_network_key") {
                return ApiError::conflict(
                    "ContractAlreadyRegistered",
                    format!(
                        "Contract {} is already registered for network {}",
                        req.contract_id,
                        req.network
                    ),
                );
            }
        }
        db_internal_error("create contract", err)
    })?;

    // Create verification task for the validator network (Issue # validator network)
    let _ = sqlx::query(
        "INSERT INTO verification_tasks (contract_id, wasm_hash, status) VALUES ($1, $2, 'pending') ON CONFLICT DO NOTHING"
    )
    .bind(contract.id)
    .bind(&wasm_hash)
    .execute(&state.db)
    .await;

    // Set logical_id = id so this row is its own logical contract (Issue #43)
    let _ = sqlx::query("UPDATE contracts SET logical_id = id WHERE id = $1")
        .bind(contract.id)
        .execute(&state.db)
        .await;

    let contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(contract.id)
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch contract after insert", err))?;

    // Backfill contract metadata cache for immediate reads
    if let Ok(contract_json) = serde_json::to_string(&contract) {
        let _ = state
            .cache
            .put_contract_meta(&contract.contract_id, contract_json.clone())
            .await;
        let _ = state
            .cache
            .put_contract_meta(&contract.id.to_string(), contract_json)
            .await;
    }

    // Save dependencies if provided
    if !req.dependencies.is_empty() {
        if let Err(e) =
            dependency::save_dependencies(&state.db, contract.id, &req.dependencies).await
        {
            tracing::error!(
                "Failed to save initial dependencies for contract {}: {}",
                contract.contract_id,
                e
            );
        }
        // Invalidate global graph cache
        state
            .cache
            .invalidate("system", "global:dependency_graph")
            .await;
    }

    let creation_changes = json!({
        "contract_id": { "before": Value::Null, "after": contract.contract_id },
        "name": { "before": Value::Null, "after": contract.name },
        "slug": { "before": Value::Null, "after": contract.slug },
        "description": { "before": Value::Null, "after": contract.description },
        "publisher_id": { "before": Value::Null, "after": contract.publisher_id },
        "network": { "before": Value::Null, "after": contract.network.to_string() },
        "is_verified": { "before": Value::Null, "after": contract.is_verified },
        "category": { "before": Value::Null, "after": contract.category },
        "tags": { "before": Value::Null, "after": contract.tags }
    });

    write_contract_audit_log(
        &state.db,
        AuditActionType::ContractPublished,
        contract.id,
        publisher.id,
        creation_changes,
        &extract_ip_address(&headers),
    )
    .await
    .map_err(|err| db_internal_error("write contract_created audit log", err))?;

    record_contract_interaction(
        &state.db,
        ContractInteractionInsert {
            contract_id: contract.id,
            target_contract_id: None,
            account: Some(&publisher.stellar_address),
            interaction_type: "publish_success",
            transaction_hash: None,
            method: None,
            parameters: None,
            return_value: None,
            timestamp: chrono::Utc::now(),
            network: &contract.network,
        },
    )
    .await
    .map_err(|err| db_internal_error("record publish_success interaction", err))?;

    let _ = analytics::record_event(
        &state.db,
        AnalyticsEventType::ContractPublished,
        Some(contract.id),
        Some(publisher.id),
        None,
        Some(&contract.network),
        Some(json!({ "name": contract.name })),
    )
    .await;

    state
        .contract_events
        .publish(ContractEventEnvelope::deployed(
            &contract,
            Some(publisher.stellar_address.clone()),
        ));

    if req.is_cicd {
        crate::events::emit_cicd_pipeline(
            &state,
            contract.contract_id.clone(),
            "published".to_string(),
            4, // Step 4 of 5
        );
    }

    state.cache.invalidate_contracts().await;

    // Increment usage counter asynchronously (fire-and-forget)
    // Failures are logged but never block the main request
    let contract_id = contract.id;
    let db_clone = state.db.clone();
    tokio::spawn(async move {
        if let Err(err) =
            crate::usage_counter::increment_usage_counter_with_timeout(contract_id, &db_clone).await
        {
            tracing::warn!(
                contract_id = %contract_id,
                error = ?err,
                "Failed to increment usage counter for contract"
            );
        }
    });
    Ok(Json(contract))
}

#[utoipa::path(
    post,
    path = "/api/publishers",
    request_body = Publisher,
    responses(
        (status = 201, description = "Publisher created successfully", body = Publisher),
        (status = 400, description = "Invalid input")
    ),
    tag = "Publishers"
)]
pub async fn create_publisher(
    State(state): State<AppState>,
    ValidatedJson(publisher): ValidatedJson<Publisher>,
) -> ApiResult<Json<Publisher>> {
    let created: Publisher = sqlx::query_as(
        "INSERT INTO publishers (stellar_address, username, email, github_url, website)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING *",
    )
    .bind(&publisher.stellar_address)
    .bind(&publisher.username)
    .bind(&publisher.email)
    .bind(&publisher.github_url)
    .bind(&publisher.website)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("create publisher", err))?;

    let _ = analytics::record_event(
        &state.db,
        AnalyticsEventType::PublisherCreated,
        None,
        Some(created.id),
        None,
        None,
        Some(json!({ "stellar_address": created.stellar_address })),
    )
    .await;

    Ok(Json(created))
}

#[utoipa::path(
    get,
    path = "/api/publishers/{id}",
    params(
        ("id" = String, Path, description = "Publisher UUID")
    ),
    responses(
        (status = 200, description = "Publisher details", body = Publisher),
        (status = 404, description = "Publisher not found")
    ),
    tag = "Publishers"
)]
pub async fn get_publisher(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Publisher>> {
    let publisher_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidPublisherId",
            format!("Invalid publisher ID format: {}", id),
        )
    })?;

    let publisher: Publisher = sqlx::query_as("SELECT * FROM publishers WHERE id = $1")
        .bind(publisher_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "PublisherNotFound",
                format!("No publisher found with ID: {}", id),
            ),
            _ => db_internal_error("get publisher by id", err),
        })?;

    Ok(Json(publisher))
}

#[utoipa::path(
    get,
    path = "/api/publishers/{id}/contracts",
    params(
        ("id" = String, Path, description = "Publisher UUID")
    ),
    responses(
        (status = 200, description = "List of contracts by publisher", body = [Contract]),
        (status = 404, description = "Publisher not found")
    ),
    tag = "Publishers"
)]
pub async fn get_publisher_contracts(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    axum::extract::OriginalUri(uri): axum::extract::OriginalUri,
    Path(id): Path<String>,
    Query(query): Query<PublisherContractsQuery>,
) -> ApiResult<crate::pagination::PagedJson<Contract>> {
    let publisher_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidPublisherId",
            format!("Invalid publisher ID format: {}", id),
        )
    })?;

    let limit = query.limit.clamp(1, 100);
    let offset = query.offset.max(0);

    let mut count_qb: QueryBuilder<'_, sqlx::Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM contracts WHERE publisher_id = ");
    count_qb.push_bind(publisher_uuid);

    if let Some(network) = &query.network {
        count_qb.push(" AND network = ");
        count_qb.push_bind(network);
    }
    if let Some(category) = &query.category {
        let cat = category.trim();
        if !cat.is_empty() {
            count_qb.push(" AND category = ");
            count_qb.push_bind(cat);
        }
    }

    let total: i64 = count_qb
        .build_query_scalar()
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("get publisher contracts count", err))?;

    let mut qb: QueryBuilder<'_, sqlx::Postgres> =
        QueryBuilder::new("SELECT * FROM contracts WHERE publisher_id = ");
    qb.push_bind(publisher_uuid);

    if let Some(network) = &query.network {
        qb.push(" AND network = ");
        qb.push_bind(network);
    }
    if let Some(category) = &query.category {
        let cat = category.trim();
        if !cat.is_empty() {
            qb.push(" AND category = ");
            qb.push_bind(cat);
        }
    }

    qb.push(" ORDER BY created_at DESC LIMIT ");
    qb.push_bind(limit);
    qb.push(" OFFSET ");
    qb.push_bind(offset);

    let contracts: Vec<Contract> = qb
        .build_query_as()
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_internal_error("get publisher contracts", err))?;

    let page = (offset / limit) + 1;
    let body = PaginatedResponse::new(contracts, total, page, limit);

    Ok(crate::pagination::PagedJson::new(body, &headers, &uri))
}

/// Query for contract ABI and OpenAPI (optional version)
#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct ContractAbiQuery {
    pub version: Option<String>,
    pub bypass_cache: Option<bool>,
    pub format: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct ContractAbiRow {
    contract_uuid: Uuid,
    contract_id: String,
    version: Option<String>,
    abi: Value,
    compiler: Option<String>,
    generated_at: chrono::DateTime<chrono::Utc>,
}

/// Fetch ABI JSON string for contract (by id or id@version)
async fn resolve_contract_abi(
    state: &AppState,
    id: &str,
    version: Option<&str>,
    bypass_cache: bool,
) -> ApiResult<String> {
    let selector = if let Some(v) = version {
        format!("{}@{}", id, v)
    } else {
        id.to_string()
    };
    resolve_abi(state, &selector, bypass_cache).await
}

// Contract ABI and OpenAPI endpoints
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/abi",
    params(
        ("id" = String, Path, description = "Contract identifier (address or name)"),
        ContractAbiQuery
    ),
    responses(
        (status = 200, description = "Contract ABI", body = Object),
        (status = 404, description = "Contract or version not found")
    ),
    tag = "Artifacts"
)]
pub async fn get_contract_abi(
    Path(id): Path<String>,
    Query(query): Query<ContractAbiQuery>,
    State(state): State<AppState>,
) -> ApiResult<Response> {
    build_contract_abi_response(HeaderMap::new(), Path(id), Query(query), State(state)).await
}

pub async fn get_contract_abi_v1(
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<ContractAbiQuery>,
    State(state): State<AppState>,
) -> ApiResult<Response> {
    build_contract_abi_response(headers, Path(id), Query(query), State(state)).await
}

async fn build_contract_abi_response(
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<ContractAbiQuery>,
    State(state): State<AppState>,
) -> ApiResult<Response> {
    let format = query.format.as_deref().unwrap_or("v1");
    if !matches!(format, "v1" | "v2") {
        return Err(ApiError::bad_request(
            "InvalidAbiFormat",
            "format must be v1 or v2",
        ));
    }

    let cache_key = format!(
        "abi-response:{}:{}:{}",
        id,
        query.version.as_deref().unwrap_or("latest"),
        format
    );
    let body = if let Some(cached) = state
        .cache
        .get_abi(&cache_key, query.bypass_cache.unwrap_or(false))
        .await
    {
        cached
    } else {
        let row = fetch_contract_abi_row(&state, &id, query.version.as_deref()).await?;
        let envelope = contract_abi_envelope(&row, format)?;
        let encoded = serde_json::to_string(&envelope)
            .map_err(|e| ApiError::internal(format!("serialize ABI response: {}", e)))?;
        state.cache.put_abi(&cache_key, encoded.clone()).await;
        encoded
    };

    let hash = abi_hash(&body);
    let accepts_gzip = headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(',').any(|part| part.trim().starts_with("gzip")))
        .unwrap_or(false);

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .header("X-ABI-Hash", hash);

    if accepts_gzip {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(body.as_bytes())
            .map_err(|_| ApiError::internal("Failed to compress ABI response"))?;
        let compressed = encoder
            .finish()
            .map_err(|_| ApiError::internal("Failed to finalize ABI compression"))?;
        builder = builder.header(header::CONTENT_ENCODING, "gzip");
        return builder
            .body(Body::from(compressed))
            .map_err(|_| ApiError::internal("Failed to build ABI response"));
    }

    builder
        .body(Body::from(body))
        .map_err(|_| ApiError::internal("Failed to build ABI response"))
}

async fn fetch_contract_abi_row(
    state: &AppState,
    id: &str,
    version: Option<&str>,
) -> ApiResult<ContractAbiRow> {
    let contract_uuid = fetch_contract_identity(state, id).await?.0;

    if let Some(version) = version {
        return sqlx::query_as::<_, ContractAbiRow>(
            "SELECT c.id AS contract_uuid,
                    c.contract_id,
                    ca.version,
                    ca.abi,
                    v.compiler_version AS compiler,
                    ca.created_at AS generated_at
             FROM contracts c
             JOIN contract_abis ca ON ca.contract_id = c.id
             LEFT JOIN verifications v ON v.contract_id = c.id
             WHERE c.id = $1 AND ca.version = $2
             ORDER BY v.verified_at DESC NULLS LAST
             LIMIT 1",
        )
        .bind(contract_uuid)
        .bind(version)
        .fetch_optional(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch contract ABI version", err))?
        .ok_or_else(|| ApiError::not_found("AbiNotFound", "ABI version not found"));
    }

    if let Some(row) = sqlx::query_as::<_, ContractAbiRow>(
        "SELECT c.id AS contract_uuid,
                c.contract_id,
                ca.version,
                ca.abi,
                v.compiler_version AS compiler,
                ca.created_at AS generated_at
         FROM contracts c
         JOIN contract_abis ca ON ca.contract_id = c.id
         LEFT JOIN verifications v ON v.contract_id = c.id
         WHERE c.id = $1
         ORDER BY ca.created_at DESC, v.verified_at DESC NULLS LAST
         LIMIT 1",
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch latest contract ABI", err))?
    {
        return Ok(row);
    }

    sqlx::query_as::<_, ContractAbiRow>(
        "SELECT c.id AS contract_uuid,
                c.contract_id,
                NULL::VARCHAR AS version,
                c.abi,
                v.compiler_version AS compiler,
                c.updated_at AS generated_at
         FROM contracts c
         LEFT JOIN verifications v ON v.contract_id = c.id
         WHERE c.id = $1 AND c.abi IS NOT NULL
         ORDER BY v.verified_at DESC NULLS LAST
         LIMIT 1",
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch fallback contract ABI", err))?
    .ok_or_else(|| ApiError::not_found("AbiNotFound", "No ABI found for contract"))
}

fn contract_abi_envelope(row: &ContractAbiRow, format: &str) -> ApiResult<Value> {
    let metadata = json!({
        "contract_uuid": row.contract_uuid,
        "contract_id": row.contract_id,
        "version": row.version.as_deref().unwrap_or("latest"),
        "compiler": row.compiler.as_deref().unwrap_or("unknown"),
        "generated_at": row.generated_at,
        "format": format,
    });

    match format {
        "v1" => Ok(json!({
            "metadata": metadata,
            "abi": row.abi,
        })),
        "v2" => {
            let mut envelope = BTreeMap::new();
            envelope.insert("metadata", metadata);
            envelope.insert("spec", row.abi.clone());
            Ok(json!(envelope))
        }
        _ => Err(ApiError::bad_request(
            "InvalidAbiFormat",
            "format must be v1 or v2",
        )),
    }
}

fn abi_hash(body: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/openapi.yaml",
    params(
        ("id" = String, Path, description = "Contract identifier"),
        ContractAbiQuery
    ),
    responses(
        (status = 200, description = "OpenAPI YAML specification", body = String),
        (status = 404, description = "Contract or version not found")
    ),
    tag = "Artifacts"
)]
pub async fn get_contract_openapi_yaml(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ContractAbiQuery>,
) -> ApiResult<Response> {
    let bypass = query.bypass_cache.unwrap_or(false);
    let abi_json = resolve_contract_abi(&state, &id, query.version.as_deref(), bypass).await?;
    let abi = parse_json_spec(&abi_json, &id)
        .map_err(|e| ApiError::bad_request("InvalidABI", format!("Failed to parse ABI: {}", e)))?;
    let doc = generate_openapi(&abi, Some("/invoke"));
    let yaml = to_yaml(&doc).map_err(|e| ApiError::internal(format!("OpenAPI YAML: {}", e)))?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/x-yaml")
        .body(axum::body::Body::from(yaml))
        .map_err(|_| ApiError::internal("Failed to build response"))
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/openapi.json",
    params(
        ("id" = String, Path, description = "Contract identifier"),
        ContractAbiQuery
    ),
    responses(
        (status = 200, description = "OpenAPI JSON specification", body = Object),
        (status = 404, description = "Contract or version not found")
    ),
    tag = "Artifacts"
)]
pub async fn get_contract_openapi_json(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ContractAbiQuery>,
) -> ApiResult<Response> {
    let bypass = query.bypass_cache.unwrap_or(false);
    let abi_json = resolve_contract_abi(&state, &id, query.version.as_deref(), bypass).await?;
    let abi = parse_json_spec(&abi_json, &id)
        .map_err(|e| ApiError::bad_request("InvalidABI", format!("Failed to parse ABI: {}", e)))?;
    let doc = generate_openapi(&abi, Some("/invoke"));
    let json = to_json(&doc).map_err(|e| ApiError::internal(format!("OpenAPI JSON: {}", e)))?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(json))
        .map_err(|_| ApiError::internal("Failed to build response"))
}

// Stubs for upstream added endpoints
fn planned_not_implemented_response() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({
            "error": "not_implemented",
            "message": "This endpoint is planned but not yet functional"
        })),
    )
}

pub async fn get_contract_state() -> impl IntoResponse {
    planned_not_implemented_response()
}

pub async fn update_contract_state() -> impl IntoResponse {
    planned_not_implemented_response()
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/trust-score",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    responses(
        (status = 501, description = "Not yet implemented – this endpoint is planned")
    ),
    tag = "Security"
)]
pub async fn get_trust_score() -> impl IntoResponse {
    planned_not_implemented_response()
}

#[allow(dead_code)]
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/dependencies",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    responses(
        (status = 200, description = "List of direct dependencies", body = Object),
        (status = 404, description = "Contract not found")
    ),
    tag = "Graphs"
)]
pub async fn get_contract_dependencies(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let contract_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request("InvalidContractId", format!("Invalid ID: {}", id)))?;

    let deps: Vec<shared::ContractDependency> =
        sqlx::query_as("SELECT * FROM contract_dependencies WHERE contract_id = $1")
            .bind(contract_uuid)
            .fetch_all(&state.db)
            .await
            .map_err(|e| db_internal_error("get_contract_dependencies", e))?;

    Ok(Json(json!({ "dependencies": deps })))
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/dependents",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    responses(
        (status = 200, description = "List of direct dependents", body = Object),
        (status = 404, description = "Contract not found")
    ),
    tag = "Graphs"
)]
pub async fn get_contract_dependents(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let contract_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request("InvalidContractId", format!("Invalid ID: {}", id)))?;

    let dependents: Vec<shared::ContractDependency> =
        sqlx::query_as("SELECT * FROM contract_dependencies WHERE dependency_contract_id = $1")
            .bind(contract_uuid)
            .fetch_all(&state.db)
            .await
            .map_err(|e| db_internal_error("get_contract_dependents", e))?;

    Ok(Json(json!({ "dependents": dependents })))
}

#[utoipa::path(
    get,
    path = "/api/contracts/graph",
    responses(
        (status = 200, description = "Full contract dependency graph", body = GraphResponse)
    ),
    tag = "Graphs"
)]
pub async fn get_contract_graph(
    State(state): State<AppState>,
    Query(query): Query<GetContractQuery>,
) -> ApiResult<Json<shared::GraphResponse>> {
    // Try cache first
    let cache_key = format!(
        "global:dependency_graph:{}",
        query
            .network
            .as_ref()
            .map(|network| network.to_string())
            .unwrap_or_else(|| "all".to_string())
    );
    if let (Some(cached), true) = state.cache.get("system", &cache_key).await {
        if let Ok(graph) = serde_json::from_str(&cached) {
            return Ok(Json(graph));
        }
    }

    let graph = dependency::build_dependency_graph(&state.db, query.network)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to build graph: {}", e)))?;

    // Invalidate/Refresh cache
    if let Ok(serialized) = serde_json::to_string(&graph) {
        state
            .cache
            .put(
                "system",
                &cache_key,
                serialized,
                Some(Duration::from_secs(300)),
            )
            .await;
    }

    Ok(Json(graph))
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct LocalGraphParams {
    pub depth: Option<u32>,
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/graph",
    responses(
        (status = 200, description = "Localized interaction graph", body = GraphResponse)
    ),
    params(
        ("id" = String, Path, description = "Contract identifier (UUID)"),
        ("depth" = Option<u32>, Query, description = "Graph traversal depth (default 1, max 3)")
    ),
    tag = "Graphs"
)]
pub async fn get_contract_local_graph(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<LocalGraphParams>,
) -> ApiResult<Json<shared::GraphResponse>> {
    let contract_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request("InvalidContractId", format!("Invalid ID: {}", id)))?;

    let depth = params.depth.unwrap_or(1).clamp(1, 3);

    let graph = dependency::build_local_graph(&state.db, contract_uuid, depth)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to build local graph: {}", e)))?;

    Ok(Json(graph))
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]

pub struct ImpactQuery {
    pub change: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/impact",
    params(
        ("id" = String, Path, description = "Contract UUID"),
        ImpactQuery
    ),
    responses(
        (status = 200, description = "Impact analysis for proposed changes", body = Object)
    ),
    tag = "Graphs"
)]
pub async fn get_impact_analysis(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<ImpactQuery>,
) -> ApiResult<Json<shared::ImpactAnalysisResponse>> {
    let contract_uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request("InvalidContractId", format!("Invalid ID: {}", id)))?;

    let affected_ids = dependency::get_transitive_dependents(&state.db, contract_uuid)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to get impact: {}", e)))?;

    // Check for cycles involving this contract
    let has_cycles = affected_ids.contains(&contract_uuid);

    // Fetch details for affected contracts
    let affected_contracts: Vec<shared::Contract> = if !affected_ids.is_empty() {
        sqlx::query_as("SELECT * FROM contracts WHERE id = ANY($1)")
            .bind(&affected_ids)
            .fetch_all(&state.db)
            .await
            .map_err(|e| db_internal_error("get_impact_contracts", e))?
    } else {
        Vec::new()
    };

    Ok(Json(shared::ImpactAnalysisResponse {
        contract_id: contract_uuid,
        change_type: query.change,
        affected_count: affected_ids.len(),
        affected_contracts,
        has_cycles,
    }))
}

#[utoipa::path(
    get,
    path = "/api/contracts/trending",
    responses(
        (status = 200, description = "List of trending contracts", body = Object)
    ),
    tag = "Contracts"
)]
pub async fn get_trending_contracts(
    State(state): State<AppState>,
    Query(params): Query<TrendingParams>,
) -> ApiResult<Json<Value>> {
    let limit = params.limit.unwrap_or(10).clamp(1, 50);
    let timeframe = params.timeframe.unwrap_or_else(|| "7d".to_string());
    let trailing_days = match timeframe.as_str() {
        "7d" => 7,
        "30d" => 30,
        "90d" => 90,
        _ => {
            return Err(ApiError::bad_request(
                "InvalidTimeframe",
                "timeframe must be one of: 7d, 30d, 90d",
            ));
        }
    };

    let rows: Vec<(Uuid, String, String, Network, i64, i64)> = sqlx::query_as(
        r#"
        WITH scored AS (
            SELECT
                c.id,
                c.contract_id,
                c.name,
                c.network,
                COALESCE(
                    SUM(a.count) FILTER (WHERE a.day >= CURRENT_DATE - INTERVAL '6 days'),
                    0
                )::bigint AS interactions_this_week,
                COALESCE(
                    SUM(a.count) FILTER (
                        WHERE a.day >= CURRENT_DATE - INTERVAL '13 days'
                          AND a.day < CURRENT_DATE - INTERVAL '6 days'
                    ),
                    0
                )::bigint AS interactions_last_week
            FROM contracts c
            LEFT JOIN contract_interaction_daily_aggregates a
              ON a.contract_id = c.id
             AND a.day >= CURRENT_DATE - make_interval(days => $1)
            GROUP BY c.id, c.contract_id, c.name, c.network
        )
        SELECT
            id,
            contract_id,
            name,
            network,
            interactions_this_week,
            interactions_last_week
        FROM scored
        WHERE interactions_this_week > interactions_last_week * 1.5
        ORDER BY
            CASE
                WHEN interactions_last_week = 0 THEN interactions_this_week::numeric
                ELSE interactions_this_week::numeric / interactions_last_week::numeric
            END DESC,
            interactions_this_week DESC
        LIMIT $2
        "#,
    )
    .bind(trailing_days)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch trending contracts", err))?;

    let trending: Vec<Value> = rows
        .into_iter()
        .map(
            |(id, contract_id, name, network, interactions_this_week, interactions_last_week)| {
                let ratio = if interactions_last_week == 0 {
                    if interactions_this_week > 0 {
                        serde_json::Value::String("inf".to_string())
                    } else {
                        serde_json::Value::from(0.0)
                    }
                } else {
                    serde_json::Value::from(
                        interactions_this_week as f64 / interactions_last_week as f64,
                    )
                };

                json!({
                    "id": id,
                    "contract_id": contract_id,
                    "name": name,
                    "network": network,
                    "interactions_this_week": interactions_this_week,
                    "interactions_last_week": interactions_last_week,
                    "ratio": ratio,
                    "is_trending": true
                })
            },
        )
        .collect();

    // Return the trending array directly so the frontend can use it as-is.
    // The frontend analytics page does `setTrending(trendJson)` and passes
    // the result directly to TrendingContractsTable which expects an array.
    Ok(Json(json!(trending)))
}

#[utoipa::path(
    post,
    path = "/api/contracts/verify",
    request_body = VerifyRequest,
    responses(
        (status = 200, description = "Verification successful", body = Object),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Contract not found")
    ),
    tag = "Verification"
)]
pub async fn verify_contract(
    State(state): State<AppState>,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<VerifyRequest>,
) -> ApiResult<Json<Value>> {
    let verification_started = std::time::Instant::now();
    let contract: Contract = sqlx::query_as(
        "SELECT * FROM contracts WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(&req.contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => ApiError::not_found(
            "ContractNotFound",
            format!("No contract found with contract_id: {}", req.contract_id),
        ),
        _ => db_internal_error("fetch contract for verification", err),
    })?;

    let previous_status: Option<String> = sqlx::query_scalar(
        "SELECT status::text FROM verifications WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(contract.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch previous verification status", err))?;

    let verification_id: Uuid = sqlx::query_scalar(
        "INSERT INTO verifications (contract_id, status, source_code, build_params, compiler_version, verified_at, error_message)
         VALUES ($1, 'pending', $2, $3, $4, NULL, NULL)
         RETURNING id",
    )
    .bind(contract.id)
    .bind(&req.source_code)
    .bind(&req.build_params)
    .bind(&req.compiler_version)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("insert verification record", err))?;

    let verification_result = verifier::verify_contract(
        &req.source_code,
        &contract.wasm_hash,
        Some(&req.compiler_version),
        Some(&req.build_params),
    )
    .await;
    let onchain_verifier = OnChainVerifier::new();
    let abi_json = resolve_abi(&state, &contract.contract_id, false).await.ok();
    let onchain_result = onchain_verifier
        .verify_contract(&state.cache, &contract, abi_json.as_deref())
        .await;

    let ip_address = extract_ip_address(&headers);
    let before_status = previous_status.unwrap_or_else(|| "pending".to_string());

    match (verification_result, onchain_result) {
        (Ok(result), Ok(onchain))
            if result.verified
                && onchain.contract_exists_on_chain
                && onchain.wasm_hash_matches
                && onchain.abi_valid =>
        {
            sqlx::query(
                "UPDATE verifications
                 SET status = 'verified', verified_at = NOW(), error_message = NULL
                 WHERE id = $1",
            )
            .bind(verification_id)
            .execute(&state.db)
            .await
            .map_err(|err| db_internal_error("mark verification as verified", err))?;

            // Update contract metadata (Issue #401)
            sqlx::query(
                "UPDATE contracts SET is_verified = true, verified_at = NOW(), updated_at = NOW() WHERE id = $1",
            )
            .bind(contract.id)
            .execute(&state.db)
            .await
            .map_err(|err| db_internal_error("mark contract verified", err))?;

            let verification_changes = json!({
                "verification_id": { "before": Value::Null, "after": verification_id },
                "status": { "before": Value::Null, "after": "verified" },
                "compiler_version": { "before": Value::Null, "after": req.compiler_version },
                "verified_at": { "before": Value::Null, "after": chrono::Utc::now() },
                "compiled_wasm_hash": { "before": Value::Null, "after": result.compiled_wasm_hash },
                "deployed_wasm_hash": { "before": Value::Null, "after": result.deployed_wasm_hash }
            });

            write_contract_audit_log(
                &state.db,
                AuditActionType::VerificationChanged,
                contract.id,
                contract.publisher_id,
                verification_changes,
                &ip_address,
            )
            .await
            .map_err(|err| db_internal_error("write verification_added audit log", err))?;

            if before_status != "verified" {
                let status_changes = json!({
                    "status": { "before": before_status, "after": "verified" },
                    "is_verified": { "before": contract.is_verified, "after": true }
                });
                write_contract_audit_log(
                    &state.db,
                    AuditActionType::VerificationChanged,
                    contract.id,
                    contract.publisher_id,
                    status_changes,
                    &ip_address,
                )
                .await
                .map_err(|err| db_internal_error("write status_changed audit log", err))?;
            }

            record_contract_interaction(
                &state.db,
                ContractInteractionInsert {
                    contract_id: contract.id,
                    target_contract_id: None,
                    account: None,
                    interaction_type: "publish_success",
                    transaction_hash: None,
                    method: Some("verify"),
                    parameters: None,
                    return_value: None,
                    timestamp: chrono::Utc::now(),
                    network: &contract.network,
                },
            )
            .await
            .map_err(|err| db_internal_error("record verification interaction", err))?;

            let _ = analytics::record_event(
                &state.db,
                AnalyticsEventType::ContractVerified,
                Some(contract.id),
                Some(contract.publisher_id),
                None,
                Some(&contract.network),
                Some(json!({ "verification_id": verification_id })),
            )
            .await;
            let _ = analytics::record_event(
                &state.db,
                AnalyticsEventType::ContractVerified,
                Some(contract.id),
                Some(contract.publisher_id),
                None,
                Some(&contract.network),
                Some(json!({ "verification_id": verification_id })),
            )
            .await;

            crate::metrics::observe_verification_latency(
                "success",
                verification_started.elapsed().as_secs_f64(),
            );

            Ok(Json(json!({
                "verified": true,
                "status": "verified",
                "verification_id": verification_id,
                "contract_id": contract.id,
                "compiled_wasm_hash": result.compiled_wasm_hash,
                "deployed_wasm_hash": result.deployed_wasm_hash,
                "on_chain": onchain
            })))
        }
        (Ok(result), Ok(onchain)) => {
            let mut reasons = Vec::new();
            if !result.verified {
                reasons.push(
                    result.message.unwrap_or_else(|| {
                        "Verification failed due to bytecode mismatch".to_string()
                    }),
                );
            }
            if !onchain.contract_exists_on_chain {
                reasons.push("Contract does not exist on-chain".to_string());
            }
            if onchain.contract_exists_on_chain && !onchain.wasm_hash_matches {
                reasons.push("On-chain deployment does not match the stored WASM hash".to_string());
            }
            if onchain.contract_exists_on_chain && !onchain.abi_valid {
                reasons
                    .push("Stored ABI does not validate against the deployed contract".to_string());
            }
            let failure_message = reasons.join("; ");

            sqlx::query(
                "UPDATE verifications
                 SET status = 'failed', verified_at = NULL, error_message = $2
                 WHERE id = $1",
            )
            .bind(verification_id)
            .bind(&failure_message)
            .execute(&state.db)
            .await
            .map_err(|err| db_internal_error("mark verification as failed", err))?;

            let verification_changes = json!({
                "verification_id": { "before": Value::Null, "after": verification_id },
                "status": { "before": Value::Null, "after": "failed" },
                "compiler_version": { "before": Value::Null, "after": req.compiler_version },
                "error_message": { "before": Value::Null, "after": failure_message },
                "compiled_wasm_hash": { "before": Value::Null, "after": result.compiled_wasm_hash },
                "deployed_wasm_hash": { "before": Value::Null, "after": result.deployed_wasm_hash }
            });
            write_contract_audit_log(
                &state.db,
                AuditActionType::VerificationChanged,
                contract.id,
                contract.publisher_id,
                verification_changes,
                &ip_address,
            )
            .await
            .map_err(|err| db_internal_error("write failed verification audit log", err))?;

            if before_status != "failed" {
                let status_changes = json!({
                    "status": { "before": before_status, "after": "failed" },
                    "is_verified": { "before": contract.is_verified, "after": contract.is_verified }
                });
                write_contract_audit_log(
                    &state.db,
                    AuditActionType::VerificationChanged,
                    contract.id,
                    contract.publisher_id,
                    status_changes,
                    &ip_address,
                )
                .await
                .map_err(|err| db_internal_error("write failed status audit log", err))?;
            }

            crate::metrics::observe_verification_latency(
                "failure",
                verification_started.elapsed().as_secs_f64(),
            );

            Err(ApiError::unprocessable(
                "VerificationFailed",
                failure_message,
            ))
        }
        (Err(err), _) | (_, Err(err)) => {
            let failure_message = err.to_string();

            sqlx::query(
                "UPDATE verifications
                 SET status = 'failed', verified_at = NULL, error_message = $2
                 WHERE id = $1",
            )
            .bind(verification_id)
            .bind(&failure_message)
            .execute(&state.db)
            .await
            .map_err(|db_err| db_internal_error("persist verifier error", db_err))?;

            let verification_changes = json!({
                "verification_id": { "before": Value::Null, "after": verification_id },
                "status": { "before": Value::Null, "after": "failed" },
                "compiler_version": { "before": Value::Null, "after": req.compiler_version },
                "error_message": { "before": Value::Null, "after": failure_message }
            });
            write_contract_audit_log(
                &state.db,
                AuditActionType::VerificationChanged,
                contract.id,
                contract.publisher_id,
                verification_changes,
                &ip_address,
            )
            .await
            .map_err(|db_err| db_internal_error("write verifier error audit log", db_err))?;

            if before_status != "failed" {
                let status_changes = json!({
                    "status": { "before": before_status, "after": "failed" },
                    "is_verified": { "before": contract.is_verified, "after": contract.is_verified }
                });
                write_contract_audit_log(
                    &state.db,
                    AuditActionType::VerificationChanged,
                    contract.id,
                    contract.publisher_id,
                    status_changes,
                    &ip_address,
                )
                .await
                .map_err(|db_err| {
                    db_internal_error("write verifier error status audit log", db_err)
                })?;
            }

            crate::metrics::observe_verification_latency(
                "failure",
                verification_started.elapsed().as_secs_f64(),
            );

            Err(ApiError::unprocessable(
                "VerificationFailed",
                failure_message,
            ))
        }
    }
}

#[utoipa::path(
    patch,
    path = "/api/contracts/{id}/metadata",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    request_body = UpdateContractMetadataRequest,
    responses(
        (status = 200, description = "Metadata updated successfully", body = Contract),
        (status = 404, description = "Contract not found"),
        (status = 400, description = "Invalid input")
    ),
    tag = "Contracts"
)]
pub async fn update_contract_metadata(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<UpdateContractMetadataRequest>,
) -> ApiResult<Json<Contract>> {
    if req.name.is_none()
        && req.description.is_none()
        && req.category.is_none()
        && req.tags.is_none()
    {
        return Err(ApiError::bad_request(
            "InvalidRequest",
            "At least one metadata field must be provided",
        ));
    }

    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;

    let before: Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContractNotFound",
                format!("No contract found with ID: {}", id),
            ),
            _ => db_internal_error("fetch contract for metadata update", err),
        })?;

    require_multisig_approval_for_sensitive_update(
        &state,
        &headers,
        &before,
        "contract metadata update",
    )
    .await?;

    // Fetch before tags for audit log
    let before_tag_names: Vec<String> = sqlx::query_scalar::<_, String>(
        "SELECT t.name FROM tags t JOIN contract_tags ct ON t.id = ct.tag_id WHERE ct.contract_id = $1",
    )
    .bind(before.id)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch before tags", err))?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| db_internal_error("begin update metadata tx", err))?;

    let mut after: Contract = sqlx::query_as(
        "UPDATE contracts
            SET name = COALESCE($2, name),
                description = COALESCE($3, description),
                category = COALESCE($4, category),
                updated_at = NOW()
          WHERE id = $1
          RETURNING *",
    )
    .bind(contract_uuid)
    .bind(req.name.as_deref())
    .bind(req.description.as_deref())
    .bind(req.category.as_deref())
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| db_internal_error("update contract metadata", err))?;

    let mut after_tag_names = before_tag_names.clone();
    if let Some(tag_names) = &req.tags {
        after_tag_names = tag_names.clone();
        sqlx::query("DELETE FROM contract_tags WHERE contract_id = $1")
            .bind(contract_uuid)
            .execute(&mut *tx)
            .await
            .map_err(|err| db_internal_error("delete old contract tags", err))?;

        let mut new_tags = Vec::new();
        for name in tag_names {
            let tag: shared::Tag = sqlx::query_as(
                "INSERT INTO tags (name) VALUES ($1)
                 ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
                 RETURNING id, name, color",
            )
            .bind(name)
            .fetch_one(&mut *tx)
            .await
            .map_err(|err| db_internal_error("upsert tag", err))?;

            sqlx::query(
                "INSERT INTO contract_tags (contract_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
            )
            .bind(contract_uuid)
            .bind(tag.id)
            .execute(&mut *tx)
            .await
            .map_err(|err| db_internal_error("link contract tag", err))?;

            new_tags.push(tag);
        }
        after.tags = new_tags;
    } else {
        // Fetch existing tags for after response
        #[derive(sqlx::FromRow)]
        struct TagRow {
            id: Uuid,
            name: String,
            color: String,
        }
        let after_tag_rows = sqlx::query_as::<_, TagRow>(
            "SELECT t.id, t.name, t.color FROM tags t JOIN contract_tags ct ON t.id = ct.tag_id WHERE ct.contract_id = $1",
        )
        .bind(after.id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|err| db_internal_error("fetch after tags", err))?;
        after.tags = after_tag_rows
            .into_iter()
            .map(|r| shared::Tag {
                id: r.id,
                name: r.name,
                color: r.color,
            })
            .collect();
    }

    tx.commit()
        .await
        .map_err(|err| db_internal_error("commit update metadata tx", err))?;

    let mut changes = serde_json::Map::new();
    if before.name != after.name {
        changes.insert(
            "name".to_string(),
            json!({ "before": before.name, "after": after.name }),
        );
    }
    if before.description != after.description {
        changes.insert(
            "description".to_string(),
            json!({ "before": before.description, "after": after.description }),
        );
    }
    if before.category != after.category {
        changes.insert(
            "category".to_string(),
            json!({ "before": before.category, "after": after.category }),
        );
    }
    if before.tags != after.tags {
        changes.insert(
            "tags".to_string(),
            json!({ "before": before.tags, "after": after.tags }),
        );
    }

    if !changes.is_empty() {
        let changes_value = Value::Object(changes.clone());
        write_contract_audit_log(
            &state.db,
            AuditActionType::MetadataUpdated,
            after.id,
            req.user_id.unwrap_or(before.publisher_id),
            changes_value.clone(),
            &extract_ip_address(&headers),
        )
        .await
        .map_err(|err| db_internal_error("write metadata_updated audit log", err))?;

        let _ = analytics::record_event(
            &state.db,
            AnalyticsEventType::ContractUpdated,
            Some(after.id),
            Some(after.publisher_id),
            None,
            Some(&after.network),
            Some(json!({ "changes": changes })),
        )
        .await;

        state
            .contract_events
            .publish(ContractEventEnvelope::metadata_updated(
                &after,
                changes_value,
                ContractEventVisibility::Public,
            ));
    }

    state.cache.invalidate_contracts().await;
    state
        .cache
        .invalidate_contract_meta(&after.contract_id)
        .await;
    state
        .cache
        .invalidate_contract_meta(&after.id.to_string())
        .await;

    // Increment usage counter asynchronously (fire-and-forget)
    // Failures are logged but never block the main request
    let contract_id = after.id;
    let db_clone = state.db.clone();
    tokio::spawn(async move {
        if let Err(err) =
            crate::usage_counter::increment_usage_counter_with_timeout(contract_id, &db_clone).await
        {
            tracing::warn!(
                contract_id = %contract_id,
                error = ?err,
                "Failed to increment usage counter for contract"
            );
        }
    });
    Ok(Json(after))
}

#[utoipa::path(
    patch,
    path = "/api/contracts/{id}/publisher",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    request_body = ChangePublisherRequest,
    responses(
        (status = 200, description = "Publisher changed successfully", body = Contract),
        (status = 404, description = "Contract not found")
    ),
    tag = "Contracts"
)]
pub async fn change_contract_publisher(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<ChangePublisherRequest>,
) -> ApiResult<Json<Contract>> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;

    let before: Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContractNotFound",
                format!("No contract found with ID: {}", id),
            ),
            _ => db_internal_error("fetch contract for publisher change", err),
        })?;

    require_multisig_approval_for_sensitive_update(
        &state,
        &headers,
        &before,
        "contract publisher change",
    )
    .await?;

    let old_publisher_address: String =
        sqlx::query_scalar("SELECT stellar_address FROM publishers WHERE id = $1")
            .bind(before.publisher_id)
            .fetch_one(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch current publisher address", err))?;

    let new_publisher: Publisher = sqlx::query_as(
        "INSERT INTO publishers (stellar_address)
         VALUES ($1)
         ON CONFLICT (stellar_address) DO UPDATE SET stellar_address = EXCLUDED.stellar_address
         RETURNING *",
    )
    .bind(&req.publisher_address)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("upsert new publisher", err))?;

    let after: Contract = sqlx::query_as(
        "UPDATE contracts
            SET publisher_id = $2,
                updated_at = NOW()
          WHERE id = $1
          RETURNING *",
    )
    .bind(contract_uuid)
    .bind(new_publisher.id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("update contract publisher", err))?;

    if before.publisher_id != after.publisher_id {
        let changes = json!({
            "publisher_id": { "before": before.publisher_id, "after": after.publisher_id },
            "publisher_address": { "before": old_publisher_address, "after": new_publisher.stellar_address }
        });
        write_contract_audit_log(
            &state.db,
            AuditActionType::PublisherChanged,
            after.id,
            req.user_id.unwrap_or(before.publisher_id),
            changes,
            &extract_ip_address(&headers),
        )
        .await
        .map_err(|err| db_internal_error("write publisher_changed audit log", err))?;
    }

    state.cache.invalidate_contracts().await;
    Ok(Json(after))
}

#[utoipa::path(
    patch,
    path = "/api/contracts/{id}/status",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    request_body = UpdateContractStatusRequest,
    responses(
        (status = 200, description = "Status updated successfully", body = Object),
        (status = 404, description = "Contract not found"),
        (status = 400, description = "Invalid status")
    ),
    tag = "Contracts"
)]
pub async fn update_contract_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<UpdateContractStatusRequest>,
) -> ApiResult<Json<Value>> {
    let normalized_status = req.status.to_ascii_lowercase();
    if normalized_status != "pending"
        && normalized_status != "verified"
        && normalized_status != "failed"
    {
        return Err(ApiError::bad_request(
            "InvalidStatus",
            "status must be one of: pending, verified, failed",
        ));
    }

    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;

    let contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContractNotFound",
                format!("No contract found with ID: {}", id),
            ),
            _ => db_internal_error("fetch contract for status update", err),
        })?;

    require_multisig_approval_for_sensitive_update(
        &state,
        &headers,
        &contract,
        "contract status update",
    )
    .await?;

    let previous_status: Option<String> = sqlx::query_scalar(
        "SELECT status::text FROM verifications WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch previous status for status update", err))?;

    let verified_at: Option<chrono::DateTime<chrono::Utc>> = if normalized_status == "verified" {
        Some(chrono::Utc::now())
    } else {
        None
    };
    let is_verified_after = normalized_status == "verified";

    let verification_id: Uuid = sqlx::query_scalar(
        "INSERT INTO verifications (contract_id, status, source_code, build_params, compiler_version, verified_at, error_message)
         VALUES ($1, $2::verification_status, NULL, NULL, NULL, $3, $4)
         RETURNING id",
    )
    .bind(contract_uuid)
    .bind(&normalized_status)
    .bind(verified_at)
    .bind(req.error_message.as_deref())
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("insert status verification row", err))?;

    let verified_at = if is_verified_after {
        Some(chrono::Utc::now())
    } else {
        contract.verified_at
    };

    sqlx::query("UPDATE contracts SET is_verified = $2, verified_at = COALESCE($3, verified_at), verification_status = $4::verification_status, verified_by = $5, verification_notes = $6, updated_at = NOW() WHERE id = $1")
        .bind(contract_uuid)
        .bind(is_verified_after)
        .bind(verified_at)
        .bind(&normalized_status)
        .bind(req.user_id)
        .bind(req.error_message.as_deref())
        .execute(&state.db)
        .await
        .map_err(|err| db_internal_error("update contract verification flag from status", err))?;

    let before_status = previous_status.unwrap_or_else(|| "pending".to_string());
    if before_status != normalized_status || contract.is_verified != is_verified_after {
        let changes = json!({
            "status": { "before": before_status, "after": normalized_status },
            "is_verified": { "before": contract.is_verified, "after": is_verified_after },
            "verification_id": { "before": Value::Null, "after": verification_id },
            "verified_by": { "before": contract.publisher_id, "after": req.user_id },
            "verification_notes": { "before": contract.verified_at.map(|d| d.to_string()), "after": req.error_message }
        });
        write_contract_audit_log(
            &state.db,
            AuditActionType::VerificationChanged,
            contract_uuid,
            req.user_id.unwrap_or(contract.publisher_id),
            changes,
            &extract_ip_address(&headers),
        )
        .await
        .map_err(|err| db_internal_error("write status_changed audit log", err))?;
    }

    if normalized_status == "verified" || normalized_status == "failed" {
        let interaction_type = if normalized_status == "verified" {
            "publish_success"
        } else {
            "publish_failed"
        };

        record_contract_interaction(
            &state.db,
            ContractInteractionInsert {
                contract_id: contract_uuid,
                target_contract_id: None,
                account: None,
                interaction_type,
                transaction_hash: None,
                method: Some("status_update"),
                parameters: None,
                return_value: None,
                timestamp: chrono::Utc::now(),
                network: &contract.network,
            },
        )
        .await
        .map_err(|err| db_internal_error("record status interaction", err))?;
    }

    let contract_after: Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch contract after status update", err))?;

    state
        .contract_events
        .publish(ContractEventEnvelope::status_updated(
            &contract_after,
            normalized_status.clone(),
            is_verified_after,
            None,
            ContractEventVisibility::Public,
        ));

    if req.error_message.is_some() {
        state
            .contract_events
            .publish(ContractEventEnvelope::status_updated(
                &contract_after,
                normalized_status.clone(),
                is_verified_after,
                Some(json!({
                    "error_message": req.error_message,
                    "publisher_id": contract.publisher_id,
                })),
                ContractEventVisibility::Private,
            ));
    }

    state.cache.invalidate_contracts().await;

    // Increment usage counter asynchronously (fire-and-forget)
    // Failures are logged but never block the main request
    let db_clone = state.db.clone();
    tokio::spawn(async move {
        if let Err(err) =
            crate::usage_counter::increment_usage_counter_with_timeout(contract_uuid, &db_clone)
                .await
        {
            tracing::warn!(
                contract_id = %contract_uuid,
                error = ?err,
                "Failed to increment usage counter for contract"
            );
        }
    });
    Ok(Json(json!({
        "contract_id": contract_uuid,
        "verification_id": verification_id,
        "status": normalized_status,
        "is_verified": is_verified_after
    })))
}

pub async fn bulk_update_contract_status(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<shared::BulkStatusUpdateRequest>,
) -> ApiResult<Json<Value>> {
    let mut results = Vec::new();

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| db_internal_error("begin tx for bulk status update", e))?;

    for item in req.items.into_iter() {
        let id = item.id;
        let normalized_status = item.status.to_ascii_lowercase();
        if normalized_status != "pending"
            && normalized_status != "verified"
            && normalized_status != "failed"
        {
            results.push(json!({ "id": id, "ok": false, "error": "invalid_status" }));
            continue;
        }

        let contract: Result<Contract, _> = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await;

        let contract = match contract {
            Ok(c) => c,
            Err(_) => {
                results.push(json!({ "id": id, "ok": false, "error": "not_found" }));
                continue;
            }
        };

        let verified_at: Option<chrono::DateTime<chrono::Utc>> = if normalized_status == "verified"
        {
            Some(chrono::Utc::now())
        } else {
            None
        };
        let is_verified_after = normalized_status == "verified";

        let verification_id: Result<Uuid, sqlx::Error> = sqlx::query_scalar(
            "INSERT INTO verifications (contract_id, status, source_code, build_params, compiler_version, verified_at, error_message) VALUES ($1, $2::verification_status, NULL, NULL, NULL, $3, $4) RETURNING id",
        )
        .bind(id)
        .bind(&normalized_status)
        .bind(verified_at)
        .bind(item.error_message.as_deref())
        .fetch_one(&mut *tx)
        .await;

        if let Err(e) = verification_id {
            results.push(
                json!({ "id": id, "ok": false, "error": format!("db_insert_verification: {}", e) }),
            );
            continue;
        }

        let verification_id = verification_id.unwrap();

        if let Err(e) = sqlx::query("UPDATE contracts SET is_verified = $2, verified_at = COALESCE($3, verified_at), verification_status = $4::verification_status, verified_by = $5, verification_notes = $6, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .bind(is_verified_after)
            .bind(verified_at)
            .bind(&normalized_status)
            .bind(item.user_id)
            .bind(item.error_message.as_deref())
            .execute(&mut *tx)
            .await
        {
            results.push(json!({ "id": id, "ok": false, "error": format!("db_update_contract: {}", e) }));
            continue;
        }

        // audit log
        let before_status = "pending".to_string();
        let changes = json!({
            "status": { "before": before_status, "after": normalized_status },
            "is_verified": { "before": contract.is_verified, "after": is_verified_after },
            "verification_id": { "before": Value::Null, "after": verification_id }
        });

        let _ = write_contract_audit_log(
            &state.db,
            AuditActionType::VerificationChanged,
            id,
            item.user_id.unwrap_or(contract.publisher_id),
            changes,
            "bulk",
        )
        .await;

        results.push(json!({ "id": id, "ok": true, "verification_id": verification_id }));
    }

    tx.commit()
        .await
        .map_err(|e| db_internal_error("commit bulk status update", e))?;

    state.cache.invalidate_contracts().await;
    Ok(Json(json!({ "results": results })))
}

/// Batch metadata update — apply name/description/category/tag changes to multiple contracts (#849).
/// Each item is processed in its own transaction so a per-item failure never rolls back prior items.
/// A version snapshot is written to `contract_metadata_versions` before each update, enabling
/// per-contract rollback via `POST /api/contracts/:id/metadata/rollback/:version_id`.
pub async fn batch_update_contract_metadata(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<shared::BatchMetadataUpdateRequest>,
) -> ApiResult<Json<shared::BatchMetadataUpdateResponse>> {
    let batch_id = req.batch_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let user_id = req.user_id;

    let mut results: Vec<shared::BatchMetadataUpdateItemResult> = Vec::new();
    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for item in req.items {
        // Require at least one metadata field per item.
        if item.name.is_none()
            && item.description.is_none()
            && item.category.is_none()
            && item.tags.is_none()
        {
            results.push(shared::BatchMetadataUpdateItemResult {
                contract_id: item.contract_id,
                ok: false,
                rollback_version_id: None,
                error: Some("at_least_one_field_required".to_string()),
            });
            failed += 1;
            continue;
        }

        let contract_uuid = match Uuid::parse_str(&item.contract_id) {
            Ok(id) => id,
            Err(_) => {
                results.push(shared::BatchMetadataUpdateItemResult {
                    contract_id: item.contract_id,
                    ok: false,
                    rollback_version_id: None,
                    error: Some("invalid_contract_id".to_string()),
                });
                failed += 1;
                continue;
            }
        };

        // Fetch current contract for before-state.
        let before: Contract = match sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
            .bind(contract_uuid)
            .fetch_one(&state.db)
            .await
        {
            Ok(c) => c,
            Err(_) => {
                results.push(shared::BatchMetadataUpdateItemResult {
                    contract_id: item.contract_id,
                    ok: false,
                    rollback_version_id: None,
                    error: Some("not_found".to_string()),
                });
                failed += 1;
                continue;
            }
        };

        // Fetch current tag names for the snapshot.
        let before_tag_names: Vec<String> = match sqlx::query_scalar::<_, String>(
            "SELECT t.name FROM tags t JOIN contract_tags ct ON t.id = ct.tag_id WHERE ct.contract_id = $1",
        )
        .bind(contract_uuid)
        .fetch_all(&state.db)
        .await
        {
            Ok(names) => names,
            Err(e) => {
                results.push(shared::BatchMetadataUpdateItemResult {
                    contract_id: item.contract_id,
                    ok: false,
                    rollback_version_id: None,
                    error: Some(format!("fetch_tags_failed: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        // Per-item transaction.
        let mut tx = match state.db.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                results.push(shared::BatchMetadataUpdateItemResult {
                    contract_id: item.contract_id,
                    ok: false,
                    rollback_version_id: None,
                    error: Some(format!("begin_tx_failed: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        // Snapshot current metadata so callers can rollback to this version.
        let snapshot_summary = item
            .change_summary
            .as_deref()
            .map(|s| format!("Before batch update: {}", s));
        let rollback_version_id: Uuid = match sqlx::query_scalar(
            "INSERT INTO contract_metadata_versions
                 (contract_id, user_id, name, description, category, tags, change_summary)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id",
        )
        .bind(contract_uuid)
        .bind(user_id)
        .bind(&before.name)
        .bind(&before.description)
        .bind(&before.category)
        .bind(&before_tag_names)
        .bind(snapshot_summary)
        .fetch_one(&mut *tx)
        .await
        {
            Ok(id) => id,
            Err(e) => {
                let _ = tx.rollback().await;
                results.push(shared::BatchMetadataUpdateItemResult {
                    contract_id: item.contract_id,
                    ok: false,
                    rollback_version_id: None,
                    error: Some(format!("snapshot_failed: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        // Apply COALESCE UPDATE.
        let mut after: Contract = match sqlx::query_as(
            "UPDATE contracts
             SET name        = COALESCE($2, name),
                 description = COALESCE($3, description),
                 category    = COALESCE($4, category),
                 updated_at  = NOW()
             WHERE id = $1
             RETURNING *",
        )
        .bind(contract_uuid)
        .bind(item.name.as_deref())
        .bind(item.description.as_deref())
        .bind(item.category.as_deref())
        .fetch_one(&mut *tx)
        .await
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.rollback().await;
                results.push(shared::BatchMetadataUpdateItemResult {
                    contract_id: item.contract_id,
                    ok: false,
                    rollback_version_id: Some(rollback_version_id),
                    error: Some(format!("update_failed: {}", e)),
                });
                failed += 1;
                continue;
            }
        };

        // Sync tags when the caller supplied a tag list.
        let mut after_tag_names = before_tag_names.clone();
        if let Some(tag_names) = &item.tags {
            after_tag_names = tag_names.clone();

            if let Err(e) = sqlx::query("DELETE FROM contract_tags WHERE contract_id = $1")
                .bind(contract_uuid)
                .execute(&mut *tx)
                .await
            {
                let _ = tx.rollback().await;
                results.push(shared::BatchMetadataUpdateItemResult {
                    contract_id: item.contract_id,
                    ok: false,
                    rollback_version_id: Some(rollback_version_id),
                    error: Some(format!("delete_tags_failed: {}", e)),
                });
                failed += 1;
                continue;
            }

            let mut new_tags: Vec<shared::Tag> = Vec::new();
            let mut tag_error: Option<String> = None;
            for tag_name in tag_names {
                let tag: Result<shared::Tag, _> = sqlx::query_as(
                    "INSERT INTO tags (name) VALUES ($1)
                     ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
                     RETURNING id, name, color",
                )
                .bind(tag_name)
                .fetch_one(&mut *tx)
                .await;

                match tag {
                    Ok(t) => {
                        let link = sqlx::query(
                            "INSERT INTO contract_tags (contract_id, tag_id)
                             VALUES ($1, $2) ON CONFLICT DO NOTHING",
                        )
                        .bind(contract_uuid)
                        .bind(t.id)
                        .execute(&mut *tx)
                        .await;
                        if let Err(e) = link {
                            tag_error = Some(format!("link_tag_failed: {}", e));
                            break;
                        }
                        new_tags.push(t);
                    }
                    Err(e) => {
                        tag_error = Some(format!("upsert_tag_failed: {}", e));
                        break;
                    }
                }
            }

            if let Some(err) = tag_error {
                let _ = tx.rollback().await;
                results.push(shared::BatchMetadataUpdateItemResult {
                    contract_id: item.contract_id,
                    ok: false,
                    rollback_version_id: Some(rollback_version_id),
                    error: Some(err),
                });
                failed += 1;
                continue;
            }
            after.tags = new_tags;
        }

        // Commit this item.
        if let Err(e) = tx.commit().await {
            results.push(shared::BatchMetadataUpdateItemResult {
                contract_id: item.contract_id,
                ok: false,
                rollback_version_id: Some(rollback_version_id),
                error: Some(format!("commit_failed: {}", e)),
            });
            failed += 1;
            continue;
        }

        // Audit log (best-effort, after commit).
        let changes = json!({
            "name":        { "before": before.name,        "after": after.name },
            "description": { "before": before.description, "after": after.description },
            "category":    { "before": before.category,    "after": after.category },
            "tags":        { "before": before_tag_names,   "after": after_tag_names },
            "batch_id":    &batch_id,
        });
        let _ = write_contract_audit_log(
            &state.db,
            AuditActionType::MetadataUpdated,
            contract_uuid,
            user_id.unwrap_or(before.publisher_id),
            changes,
            "batch",
        )
        .await;

        results.push(shared::BatchMetadataUpdateItemResult {
            contract_id: item.contract_id,
            ok: true,
            rollback_version_id: Some(rollback_version_id),
            error: None,
        });
        succeeded += 1;
    }

    state.cache.invalidate_contracts().await;

    Ok(Json(shared::BatchMetadataUpdateResponse {
        batch_id,
        total: results.len(),
        succeeded,
        failed,
        results,
    }))
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/audit-log",
    params(
        ("id" = String, Path, description = "Contract UUID"),
        AuditLogQuery
    ),
    responses(
        (status = 200, description = "Paginated audit logs for the contract", body = [ContractAuditLogEntry]),
        (status = 404, description = "Contract not found")
    ),
    tag = "Administration"
)]
pub async fn get_contract_audit_log(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<AuditLogQuery>,
) -> ApiResult<Json<Vec<ContractAuditLog>>> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;
    let limit = params.limit.clamp(1, 500);
    let offset = params.offset.max(0);

    let _contract: Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContractNotFound",
                format!("No contract found with ID: {}", id),
            ),
            _ => db_internal_error("check contract before audit log query", err),
        })?;
    ensure_contract_exists(
        &state,
        contract_uuid,
        &id,
        "check contract before audit log query",
    )
    .await?;

    let logs: Vec<ContractAuditLog> = sqlx::query_as(
        r#"
        SELECT id, contract_id, action_type, old_value, new_value, changed_by, "timestamp",
               previous_hash, hash, signature
          FROM contract_audit_log
         WHERE contract_id = $1
         ORDER BY "timestamp" DESC
         LIMIT $2 OFFSET $3
        "#,
    )
    .bind(contract_uuid)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch contract audit logs", err))?;

    Ok(Json(logs))
}

#[utoipa::path(
    get,
    path = "/api/admin/audit-logs",
    params(AuditLogQuery),
    responses(
        (status = 200, description = "Global audit logs (Admin only)", body = [ContractAuditLogEntry])
    ),
    tag = "Administration",
    security(("bearerAuth" = []))
)]
pub async fn get_all_audit_logs(
    State(state): State<AppState>,
    Query(params): Query<AuditLogQuery>,
) -> ApiResult<Json<Vec<ContractAuditLog>>> {
    let limit = params.limit.clamp(1, 500);
    let offset = params.offset.max(0);

    let logs: Vec<ContractAuditLog> = sqlx::query_as(
        r#"
        SELECT id, contract_id, action_type, old_value, new_value, changed_by, "timestamp",
               previous_hash, hash, signature
          FROM contract_audit_log
         ORDER BY "timestamp" DESC
         LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch all audit logs", err))?;

    Ok(Json(logs))
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct AuditQueryParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub sort_by: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/contracts/{id}/audits",
    params(
        ("id" = String, Path, description = "Contract UUID or Stellar ID"),
        AuditQueryParams
    ),
    responses(
        (status = 200, description = "Paginated audit results", body = PaginatedAuditsResponse),
        (status = 404, description = "Contract not found"),
        (status = 400, description = "Invalid parameters")
    ),
    tag = "Contracts"
)]
pub async fn get_contract_audits(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<AuditQueryParams>,
) -> ApiResult<Json<shared::PaginatedAuditsResponse>> {
    let contract_id = resolve_contract_id(&state, &id).await?;

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let since_dt = params.since.as_deref().and_then(parse_datetime);
    let until_dt = params.until.as_deref().and_then(parse_datetime);

    let rows: Vec<(Uuid, i32, i32, i32, i32, i32, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT id, total_issues, critical_issues, high_issues, medium_issues, low_issues, completed_at
         FROM security_scans
         WHERE contract_id = $1 AND completed_at IS NOT NULL
         AND (($2::timestamptz IS NULL) OR (completed_at >= $2))
         AND (($3::timestamptz IS NULL) OR (completed_at <= $3))
         ORDER BY completed_at DESC LIMIT $4 OFFSET $5"
    )
    .bind(contract_id)
    .bind(since_dt)
    .bind(until_dt)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_internal_error("fetch audits", e))?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM security_scans 
         WHERE contract_id = $1 AND completed_at IS NOT NULL
         AND (($2::timestamptz IS NULL) OR (completed_at >= $2))
         AND (($3::timestamptz IS NULL) OR (completed_at <= $3))",
    )
    .bind(contract_id)
    .bind(since_dt)
    .bind(until_dt)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let audits = rows
        .into_iter()
        .map(|(scan_id, total, crit, high, med, low, completed_at)| {
            let status = if crit > 0 {
                shared::AuditScanStatus::Failed
            } else if high > 0 || med > 0 {
                shared::AuditScanStatus::Issues
            } else {
                shared::AuditScanStatus::Passed
            };

            let mut findings = vec![];
            [
                (crit, "critical"),
                (high, "high"),
                (med, "medium"),
                (low, "low"),
            ]
            .iter()
            .filter(|(count, _)| *count > 0)
            .for_each(|(count, sev)| {
                findings.push(shared::ContractAuditFinding {
                    severity: sev.to_string(),
                    count: *count,
                });
            });

            shared::ContractAuditResponse {
                id: scan_id,
                contract_id,
                audit_type: shared::AuditType::Informal,
                status,
                auditor: Some("Automated Security Scanner".to_string()),
                audit_date: completed_at.unwrap_or_else(Utc::now),
                findings_summary: findings,
                total_issues: total,
                report_url: Some(format!(
                    "/api/v1/contracts/{}/audits/{}",
                    contract_id, scan_id
                )),
            }
        })
        .collect();

    Ok(Json(shared::PaginatedAuditsResponse {
        audits,
        total,
        page,
        per_page,
    }))
}

fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

async fn resolve_contract_id(state: &AppState, id: &str) -> ApiResult<Uuid> {
    if let Ok(uuid) = Uuid::parse_str(id) {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM contracts WHERE id = $1)")
                .bind(uuid)
                .fetch_one(&state.db)
                .await
                .map_err(|e| db_internal_error("check contract", e))?;
        return if exists {
            Ok(uuid)
        } else {
            Err(ApiError::not_found("contract", "Contract not found"))
        };
    }

    sqlx::query_scalar::<_, Uuid>("SELECT id FROM contracts WHERE contract_id = $1 LIMIT 1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| db_internal_error("resolve contract", e))?
        .ok_or_else(|| ApiError::not_found("contract", "Contract not found"))
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/deployments",
    params(
        ("id" = String, Path, description = "Contract UUID or Stellar ID"),
        DeploymentHistoryQueryParams
    ),
    responses(
        (status = 200, description = "List of contract on-chain deployments", body = PaginatedResponse<ContractDeploymentHistory>),
        (status = 404, description = "Contract not found")
    ),
    tag = "Deployments"
)]
pub async fn get_contract_deployments(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<DeploymentHistoryQueryParams>,
) -> ApiResult<Json<PaginatedResponse<ContractDeploymentHistory>>> {
    let contract_uuid = Uuid::parse_str(&id).ok();

    // Resolve target UUIDs (across all networks if logical_id exists)
    let target_uuids = if let Some(uuid) = contract_uuid {
        let logical_id: Option<Uuid> =
            sqlx::query_scalar("SELECT logical_id FROM contracts WHERE id = $1")
                .bind(uuid)
                .fetch_optional(&state.db)
                .await
                .map_err(|err| db_internal_error("get logical_id", err))?;

        ensure_contract_exists(&state, uuid, &id, "get contract for list deployments").await?;
        if let Some(lid) = logical_id {
            sqlx::query_scalar("SELECT id FROM contracts WHERE logical_id = $1")
                .bind(lid)
                .fetch_all(&state.db)
                .await
                .map_err(|err| db_internal_error("get contracts by logical_id", err))?
        } else {
            vec![uuid]
        }
    } else {
        // Try resolving by Stellar ID
        let uuid = dependency::resolve_contract_id(&state.db, &id)
            .await
            .map_err(|err| {
                ApiError::not_found(
                    "CONTRACT_NOT_FOUND",
                    format!("Contract {} not found: {}", id, err),
                )
            })?
            .ok_or_else(|| {
                ApiError::not_found("CONTRACT_NOT_FOUND", format!("Contract {} not found", id))
            })?;

        let logical_id: Option<Uuid> =
            sqlx::query_scalar("SELECT logical_id FROM contracts WHERE id = $1")
                .bind(uuid)
                .fetch_optional(&state.db)
                .await
                .map_err(|err| db_internal_error("get logical_id", err))?;

        if let Some(lid) = logical_id {
            sqlx::query_scalar("SELECT id FROM contracts WHERE logical_id = $1")
                .bind(lid)
                .fetch_all(&state.db)
                .await
                .map_err(|err| db_internal_error("get contracts by logical_id", err))?
        } else {
            vec![uuid]
        }
    };

    if target_uuids.is_empty() {
        return Err(ApiError::not_found(
            "CONTRACT_NOT_FOUND",
            format!("Contract {} not found", id),
        ));
    }

    // Pagination info
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    // Cache key
    let cache_key = format!(
        "deployments:{}:{}:{}:{:?}:{:?}",
        id, page, limit, params.from_date, params.to_date
    );

    if let (Some(cached), true) = state.cache.get("contract", &cache_key).await {
        if let Ok(response) =
            serde_json::from_str::<PaginatedResponse<ContractDeploymentHistory>>(&cached)
        {
            return Ok(Json(response));
        }
    }

    // Query builder for on-chain deployments
    let mut query_builder: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        "SELECT ci.created_at as deployed_at, ci.network, ci.user_address as deployer_address, ci.transaction_hash
         FROM contract_interactions ci
         WHERE ci.contract_id = ANY("
    );
    query_builder.push_bind(&target_uuids);
    query_builder.push(") AND ci.interaction_type = cast('deploy' as text)");

    query_builder.push(" ORDER BY ci.created_at DESC");
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let deployments: Vec<ContractDeploymentHistory> = query_builder
        .build_query_as()
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch deployment history", err))?;

    // Total count for pagination
    let mut count_builder: QueryBuilder<sqlx::Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM contract_interactions WHERE contract_id = ANY(");
    count_builder.push_bind(&target_uuids);
    count_builder.push(") AND interaction_type = cast('deploy' as text)");

    if let Some(from) = params.from_date {
        count_builder.push(" AND created_at >= ");
        count_builder.push_bind(from);
    }
    if let Some(to) = params.to_date {
        count_builder.push(" AND created_at <= ");
        count_builder.push_bind(to);
    }

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch deployment count", err))?;

    let response = PaginatedResponse::new(deployments, total, page, limit);

    // Cache the result
    if let Ok(serialized) = serde_json::to_string(&response) {
        state
            .cache
            .put(
                "contract",
                &cache_key,
                serialized,
                Some(std::time::Duration::from_secs(3600)),
            )
            .await;
    }

    Ok(Json(response))
}

/// Stub for dashboard analytics (Issue #415)
pub async fn get_dashboard_analytics(
    State(_state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "category_distribution": [],
        "network_usage": [],
        "deployment_trends": [],
        "recent_additions": []
    })))
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/deployments/status",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    responses(
        (status = 200, description = "Current deployment status", body = Object)
    ),
    tag = "Deployments"
)]
pub async fn get_deployment_status() -> impl IntoResponse {
    planned_not_implemented_response()
}

#[utoipa::path(
    post,
    path = "/api/deployments/green",
    responses(
        (status = 202, description = "Green deployment triggered", body = Object)
    ),
    tag = "Deployments"
)]
pub async fn deploy_green() -> impl IntoResponse {
    planned_not_implemented_response()
}

#[allow(dead_code)]
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/performance",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    responses(
        (status = 200, description = "Performance metrics and anomalies", body = Object)
    ),
    tag = "Analytics"
)]
pub async fn get_contract_performance() -> impl IntoResponse {
    Json(json!({"performance": {}}))
}

// ─── Contract interaction history (Issue #46) ─────────────────────────────────

/// GET /api/contracts/:id/interactions — list with optional filters (account, method, date range).
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/interactions",
    params(
        ("id" = String, Path, description = "Contract UUID"),
        InteractionsQueryParams
    ),
    responses(
        (status = 200, description = "List of contract interactions", body = InteractionsListResponse),
        (status = 404, description = "Contract not found")
    ),
    tag = "Analytics"
)]
pub async fn get_contract_interactions(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<InteractionsQueryParams>,
) -> ApiResult<Json<Value>> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;

    ensure_contract_exists(&state, contract_uuid, &id, "get contract for interactions").await?;

    if let Some(days) = params.days {
        let days = days.clamp(1, 365);

        let series_rows: Vec<(chrono::NaiveDate, String, i64)> = sqlx::query_as(
            r#"
            SELECT day, interaction_type, SUM(count)::bigint AS count
            FROM contract_interaction_daily_aggregates
            WHERE contract_id = $1
              AND day >= CURRENT_DATE - ($2::int - 1)
              AND ($3::text IS NULL OR interaction_type = $3)
              AND ($4::network_type IS NULL OR network = $4)
            GROUP BY day, interaction_type
            ORDER BY day ASC, interaction_type ASC
            "#,
        )
        .bind(contract_uuid)
        .bind(days as i32)
        .bind(params.interaction_type.as_deref())
        .bind(params.network.as_ref())
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch interaction time series", err))?;

        let (interactions_this_week, interactions_last_week): (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COALESCE(
                    SUM(count) FILTER (WHERE day >= CURRENT_DATE - INTERVAL '6 days'),
                    0
                )::bigint AS interactions_this_week,
                COALESCE(
                    SUM(count) FILTER (
                        WHERE day >= CURRENT_DATE - INTERVAL '13 days'
                          AND day < CURRENT_DATE - INTERVAL '6 days'
                    ),
                    0
                )::bigint AS interactions_last_week
            FROM contract_interaction_daily_aggregates
            WHERE contract_id = $1
              AND ($2::text IS NULL OR interaction_type = $2)
              AND ($3::network_type IS NULL OR network = $3)
            "#,
        )
        .bind(contract_uuid)
        .bind(params.interaction_type.as_deref())
        .bind(params.network.as_ref())
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch weekly interaction trend", err))?;

        let response = InteractionTimeSeriesResponse {
            contract_id: contract_uuid,
            days,
            interactions_this_week,
            interactions_last_week,
            is_trending: (interactions_this_week as f64) > (interactions_last_week as f64 * 1.5),
            series: series_rows
                .into_iter()
                .map(
                    |(date, interaction_type, count)| InteractionTimeSeriesPoint {
                        date,
                        interaction_type,
                        count,
                    },
                )
                .collect(),
        };

        return Ok(Json(json!(response)));
    }

    let limit = params.limit.clamp(1, 100);

    // Cursor logic
    let cursor = params.cursor.as_ref().and_then(|c| Cursor::decode(c).ok());

    let offset = if cursor.is_some() {
        0
    } else {
        params.offset.max(0)
    };

    let from_ts = params
        .from_timestamp
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));
    let to_ts = params
        .to_timestamp
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let rows: Vec<shared::ContractInteraction> = sqlx::query_as(
        r#"
        SELECT id, contract_id, user_address, interaction_type, transaction_hash,
               method, parameters, return_value, interaction_timestamp, interaction_count, network, created_at
        FROM contract_interactions
        WHERE contract_id = $1
          AND ($2::text IS NULL OR user_address = $2)
          AND ($3::text IS NULL OR method = $3)
          AND ($4::timestamptz IS NULL OR created_at >= $4)
          AND ($5::timestamptz IS NULL OR created_at <= $5)
          AND ($6::text IS NULL OR interaction_type = $6)
          AND ($7::network_type IS NULL OR network = $7)
          -- Cursor logic: tie-break with id
          AND ($8::timestamptz IS NULL OR (created_at < $8 OR (created_at = $8 AND id < $9)))
        ORDER BY created_at DESC, id DESC
        LIMIT $10 OFFSET $11
        "#,
    )
    .bind(contract_uuid)
    .bind(params.account.as_deref())
    .bind(params.method.as_deref())
    .bind(from_ts)
    .bind(to_ts)
    .bind(params.interaction_type.as_deref())
    .bind(params.network.as_ref())
    .bind(cursor.as_ref().map(|c| c.timestamp))
    .bind(cursor.as_ref().map(|c| c.id))
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("list contract interactions", err))?;

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM contract_interactions
        WHERE contract_id = $1
          AND ($2::text IS NULL OR user_address = $2)
          AND ($3::text IS NULL OR method = $3)
          AND ($4::timestamptz IS NULL OR created_at >= $4)
          AND ($5::timestamptz IS NULL OR created_at <= $5)
          AND ($6::text IS NULL OR interaction_type = $6)
          AND ($7::network_type IS NULL OR network = $7)
        "#,
    )
    .bind(contract_uuid)
    .bind(params.account.as_deref())
    .bind(params.method.as_deref())
    .bind(from_ts)
    .bind(to_ts)
    .bind(params.interaction_type.as_deref())
    .bind(params.network.as_ref())
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("count contract interactions", err))?;

    let items: Vec<ContractInteractionResponse> = rows
        .into_iter()
        .map(|r| ContractInteractionResponse {
            id: r.id,
            account: r.user_address,
            method: r.method,
            parameters: r.parameters,
            return_value: r.return_value,
            transaction_hash: r.transaction_hash,
            created_at: r.created_at,
        })
        .collect();

    let next_cursor = if items.len() >= limit as usize {
        items
            .last()
            .map(|last| Cursor::new(last.created_at, last.id).encode())
    } else {
        None
    };
    let prev_cursor = if params.cursor.is_some() || offset > 0 {
        items
            .first()
            .map(|first| Cursor::new(first.created_at, first.id).encode())
    } else {
        None
    };

    Ok(Json(json!(InteractionsListResponse {
        items,
        total,
        limit,
        offset,
        next_cursor,
        prev_cursor,
    })))
}

/// POST /api/contracts/:id/interactions — ingest one interaction.
#[utoipa::path(
    post,
    path = "/api/contracts/{id}/interactions",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    request_body = CreateInteractionRequest,
    responses(
        (status = 201, description = "Interaction logged", body = Object),
        (status = 404, description = "Contract not found")
    ),
    tag = "Analytics"
)]
pub async fn post_contract_interaction(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<CreateInteractionRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;

    let contract_network =
        fetch_contract_network(&state, contract_uuid, &id, "get contract for interaction").await?;

    let interaction_type =
        parse_interaction_type(req.interaction_type.as_deref(), req.method.as_deref())?;
    let created_at = req.timestamp.unwrap_or_else(chrono::Utc::now);
    let network = req.network.unwrap_or(contract_network);
    let target_contract_id = resolve_call_target_contract(
        &state.db,
        req.target_contract_id.as_deref(),
        req.parameters.as_ref(),
    )
    .await?;
    let interaction_id = record_contract_interaction(
        &state.db,
        ContractInteractionInsert {
            contract_id: contract_uuid,
            target_contract_id,
            account: req.account.as_deref(),
            interaction_type: &interaction_type,
            transaction_hash: req.transaction_hash.as_deref(),
            method: req.method.as_deref(),
            parameters: req.parameters.as_ref(),
            return_value: req.return_value.as_ref(),
            timestamp: created_at,
            network: &network,
        },
    )
    .await
    .map_err(|err| db_internal_error("insert contract interaction", err))?;

    tracing::info!(
        contract_id = %id,
        interaction_id = %interaction_id,
        "contract interaction logged"
    );

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": interaction_id })),
    ))
}

/// POST /api/contracts/:id/interactions/batch — ingest multiple interactions.
#[utoipa::path(
    post,
    path = "/api/contracts/{id}/interactions/batch",
    params(
        ("id" = String, Path, description = "Contract UUID")
    ),
    request_body = CreateInteractionBatchRequest,
    responses(
        (status = 201, description = "Batch of interactions logged", body = Object),
        (status = 404, description = "Contract not found")
    ),
    tag = "Analytics"
)]
pub async fn post_contract_interactions_batch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<CreateInteractionBatchRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;

    let contract_network = fetch_contract_network(
        &state,
        contract_uuid,
        &id,
        "get contract for interactions batch",
    )
    .await?;

    let mut ids = Vec::with_capacity(req.interactions.len());
    for i in &req.interactions {
        let interaction_type =
            parse_interaction_type(i.interaction_type.as_deref(), i.method.as_deref())?;
        let created_at = i.timestamp.unwrap_or_else(chrono::Utc::now);
        let network = i
            .network
            .clone()
            .unwrap_or_else(|| contract_network.clone());
        let target_contract_id = resolve_call_target_contract(
            &state.db,
            i.target_contract_id.as_deref(),
            i.parameters.as_ref(),
        )
        .await?;
        let interaction_id = record_contract_interaction(
            &state.db,
            ContractInteractionInsert {
                contract_id: contract_uuid,
                target_contract_id,
                account: i.account.as_deref(),
                interaction_type: &interaction_type,
                transaction_hash: i.transaction_hash.as_deref(),
                method: i.method.as_deref(),
                parameters: i.parameters.as_ref(),
                return_value: i.return_value.as_ref(),
                timestamp: created_at,
                network: &network,
            },
        )
        .await
        .map_err(|err| db_internal_error("insert contract interaction batch", err))?;
        ids.push(interaction_id);
    }

    tracing::info!(
        contract_id = %id,
        count = ids.len(),
        "contract interactions batch logged"
    );

    Ok((StatusCode::CREATED, Json(serde_json::json!({ "ids": ids }))))
}

pub async fn route_not_found() -> impl IntoResponse {
    ApiError::not_found("ROUTE_NOT_FOUND", "Route not found")
}

/// Get usage statistics for a specific contract by UUID or slug.
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/stats",
    params(
        ("id" = String, Path, description = "Contract UUID or slug")
    ),
    responses(
        (status = 200, description = "Contract usage statistics", body = ContractStatsResponse),
        (status = 404, description = "Contract not found"),
        (status = 400, description = "Invalid contract ID format")
    ),
    tag = "Analytics"
)]
pub async fn get_contract_stats(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ContractStatsResponse>> {
    let contract: Contract = if let Ok(contract_uuid) = Uuid::parse_str(&id) {
        sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
            .bind(contract_uuid)
            .fetch_one(&state.db)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => ApiError::not_found(
                    "ContractNotFound",
                    format!("No contract found with ID: {}", id),
                ),
                _ => db_internal_error("get contract stats by id", err),
            })?
    } else {
        // Fetch by slug (default to mainnet)
        sqlx::query_as("SELECT * FROM contracts WHERE slug = $1 AND network = $2")
            .bind(&id)
            .bind(Network::Mainnet)
            .fetch_one(&state.db)
            .await
            .map_err(|err| match err {
                sqlx::Error::RowNotFound => ApiError::not_found(
                    "ContractNotFound",
                    format!("No contract found with slug: {}", id),
                ),
                _ => db_internal_error("get contract stats by slug", err),
            })?
    };

    Ok(Json(ContractStatsResponse {
        contract_id: contract.id,
        usage_count: contract.usage_count,
        last_accessed_at: contract.last_accessed_at,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use prometheus::Registry;
    use sqlx::postgres::PgPoolOptions;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_health_check_shutdown_returns_503() {
        unsafe {
            std::env::set_var("JWT_SECRET", "abcdefghijklmnopqrstuvwxyz012345");
        }

        let is_shutting_down = Arc::new(AtomicBool::new(true));

        // Connect lazy so it doesn't fail immediately without a DB
        let db = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost:5432/soroban_registry")
            .unwrap();
        let registry = Registry::new();
        let (job_engine, _rx) = soroban_batch::engine::JobEngine::new();
        let rate_limit_state = Arc::new(crate::rate_limit::RateLimitState::from_env());
        let (event_broadcaster, _event_rx) = tokio::sync::broadcast::channel(1);
        let state = AppState::new(
            db,
            registry,
            Arc::new(job_engine),
            is_shutting_down,
            rate_limit_state,
            None,
            event_broadcaster,
        )
        .await
        .unwrap();

        let (status, json) = health_check(State(state)).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        let value = json.0;
        assert_eq!(value["status"], "shutting_down");
    }

    #[test]
    fn split_audit_changes_extracts_before_after() {
        let changes = json!({
            "name": { "before": "old-name", "after": "new-name" },
            "description": { "before": "old-desc", "after": "new-desc" },
            "is_verified": { "before": false, "after": true }
        });

        let (old_value, new_value) = split_audit_changes(&changes, "127.0.0.1");

        let old = old_value.expect("old_value should be populated");
        let new = new_value.expect("new_value should be populated");
        assert_eq!(old["name"], "old-name");
        assert_eq!(old["description"], "old-desc");
        assert_eq!(old["is_verified"], false);
        assert_eq!(new["name"], "new-name");
        assert_eq!(new["description"], "new-desc");
        assert_eq!(new["is_verified"], true);
        assert_eq!(new["_ip_address"], "127.0.0.1");
    }

    #[test]
    fn split_audit_changes_preserves_non_diff_payload() {
        let changes = json!({
            "status": "verified",
            "verification_id": "abc123"
        });

        let (old_value, new_value) = split_audit_changes(&changes, "unknown");

        assert!(old_value.is_none());
        let new = new_value.expect("new_value should be populated");
        assert_eq!(new["status"], "verified");
        assert_eq!(new["verification_id"], "abc123");
        assert_eq!(new["_ip_address"], "unknown");
    }

    #[test]
    fn timestamp_sort_helpers_cover_all_timestamp_fields() {
        let now = chrono::Utc::now();
        let contract = Contract {
            id: Uuid::nil(),
            contract_id: "C123".to_string(),
            wasm_hash: "hash".to_string(),
            name: "Demo".to_string(),
            slug: "demo".to_string(),
            description: None,
            publisher_id: Uuid::nil(),
            network: Network::Testnet,
            is_verified: true,
            verification_status: shared::VerificationStatus::Verified,
            category: None,
            tags: Vec::new(),
            created_at: now,
            updated_at: now + chrono::TimeDelta::seconds(10),
            verified_at: Some(now + chrono::TimeDelta::seconds(20)),
            deployed_at: None,
            verified_by: None,
            verification_notes: None,
            last_accessed_at: Some(now + chrono::TimeDelta::seconds(30)),
            health_score: 0,
            is_maintenance: false,
            logical_id: None,
            network_configs: None,
            organization_id: None,
            relevance_score: None,
            visibility: shared::VisibilityType::Public,
            current_version: None,
            usage_count: 0,
        };

        assert_eq!(
            sort_timestamp_column(&shared::SortBy::CreatedAt),
            Some("c.created_at")
        );
        assert_eq!(
            sort_timestamp_column(&shared::SortBy::UpdatedAt),
            Some("c.updated_at")
        );
        assert_eq!(
            sort_timestamp_column(&shared::SortBy::VerifiedAt),
            Some("c.verified_at")
        );
        assert_eq!(
            sort_timestamp_column(&shared::SortBy::LastAccessedAt),
            Some("c.last_accessed_at")
        );
        assert_eq!(
            contract_timestamp_for_sort(&contract, &shared::SortBy::VerifiedAt),
            contract.verified_at
        );
        assert_eq!(
            contract_timestamp_for_sort(&contract, &shared::SortBy::LastAccessedAt),
            contract.last_accessed_at
        );
    }

    #[test]
    fn derive_network_status_marks_rpc_failures_offline_and_stale_states_degraded() {
        let now = chrono::Utc::now();
        let healthy_snapshot = IndexerStateSnapshot {
            last_indexed_ledger_height: 42,
            indexed_at: now,
            consecutive_failures: 0,
            error_message: None,
        };
        let stale_snapshot = IndexerStateSnapshot {
            last_indexed_ledger_height: 42,
            indexed_at: now - chrono::Duration::minutes(NETWORK_STALE_AFTER_MINUTES + 1),
            consecutive_failures: 0,
            error_message: None,
        };
        let failing_snapshot = IndexerStateSnapshot {
            last_indexed_ledger_height: 42,
            indexed_at: now,
            consecutive_failures: NETWORK_OFFLINE_FAILURE_THRESHOLD,
            error_message: Some("RPC unavailable".to_string()),
        };

        assert_eq!(
            derive_network_status(true, Some(&healthy_snapshot), now),
            (NetworkStatus::Online, None)
        );
        assert_eq!(
            derive_network_status(true, Some(&stale_snapshot), now).0,
            NetworkStatus::Degraded
        );
        assert_eq!(
            derive_network_status(false, Some(&healthy_snapshot), now).0,
            NetworkStatus::Offline
        );
        assert_eq!(
            derive_network_status(true, Some(&failing_snapshot), now).0,
            NetworkStatus::Offline
        );
    }

    fn sample_export_record() -> ContractMetadataExportRecord {
        ContractMetadataExportRecord {
            id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
            logical_id: Some(Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()),
            contract_id: "CDUMMYEXPORT123".to_string(),
            wasm_hash: "deadbeef".to_string(),
            name: "Export Demo".to_string(),
            description: Some("contract metadata export".to_string()),
            publisher_id: Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap(),
            publisher_stellar_address: "GEXPORTADDRESS123".to_string(),
            publisher_username: Some("exporter".to_string()),
            network: "testnet".to_string(),
            is_verified: true,
            category: Some("DeFi".to_string()),
            tags: vec![
                shared::Tag {
                    id: Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap(),
                    name: "yield".to_string(),
                    color: "#888888".to_string(),
                },
                shared::Tag {
                    id: Uuid::parse_str("55555555-5555-5555-5555-555555555555").unwrap(),
                    name: "automation".to_string(),
                    color: "#888888".to_string(),
                },
            ],
            maturity: Some("stable".to_string()),
            health_score: 92,
            is_maintenance: false,
            deployment_count: 7,
            audit_status: Some("PASSED".to_string()),
            visibility: "public".to_string(),
            organization_id: None,
            network_configs: Some(json!({"testnet": {"contract_id": "CDUMMYEXPORT123"}})),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            verified_at: Some(Utc::now()),
            last_verified_at: Some(Utc::now()),
            last_accessed_at: Some(Utc::now()),
        }
    }

    #[test]
    fn contract_export_renderer_wraps_json_and_yaml_with_metadata() {
        let filters = ContractSearchParams {
            verified_only: Some(true),
            ..Default::default()
        };
        let metadata = ContractExportMetadata {
            exported_at: Utc::now(),
            format: ContractExportFormat::Json,
            total_count: 1,
            async_export: false,
            filters: filters.clone(),
        };
        let records = vec![sample_export_record()];

        let json_output = render_contract_export(&ContractExportFormat::Json, &metadata, &records)
            .expect("json export should render");
        let json_value: serde_json::Value =
            serde_json::from_str(&json_output).expect("json export should parse");
        assert_eq!(json_value["metadata"]["total_count"], 1);
        assert_eq!(json_value["metadata"]["filters"]["verified_only"], true);
        assert_eq!(json_value["contracts"][0]["contract_id"], "CDUMMYEXPORT123");

        let yaml_output = render_contract_export(&ContractExportFormat::Yaml, &metadata, &records)
            .expect("yaml export should render");
        assert!(yaml_output.contains("metadata:"));
        assert!(yaml_output.contains("contracts:"));
        assert!(yaml_output.contains("CDUMMYEXPORT123"));
    }

    #[test]
    fn contract_export_renderer_prefixes_csv_with_metadata_comments() {
        let metadata = ContractExportMetadata {
            exported_at: Utc::now(),
            format: ContractExportFormat::Csv,
            total_count: 1,
            async_export: true,
            filters: ContractSearchParams {
                category: Some("DeFi".to_string()),
                ..Default::default()
            },
        };
        let csv_output = render_contract_export(
            &ContractExportFormat::Csv,
            &metadata,
            &[sample_export_record()],
        )
        .expect("csv export should render");

        assert!(csv_output.starts_with("# exported_at="));
        assert!(csv_output.contains("# filters="));
        assert!(csv_output.contains("id,logical_id,contract_id"));
        assert!(csv_output.contains("CDUMMYEXPORT123"));
    }

    #[test]
    fn export_filters_drop_pagination_state() {
        let sanitized = sanitized_export_filters(ContractSearchParams {
            page: Some(3),
            limit: Some(25),
            offset: Some(50),
            cursor: Some("abc".to_string()),
            category: Some("DeFi".to_string()),
            ..Default::default()
        });

        assert_eq!(sanitized.page, None);
        assert_eq!(sanitized.limit, None);
        assert_eq!(sanitized.offset, None);
        assert_eq!(sanitized.cursor, None);
        assert_eq!(sanitized.category.as_deref(), Some("DeFi"));
    }

    fn minimal_export_metadata(format: ContractExportFormat) -> ContractExportMetadata {
        ContractExportMetadata {
            exported_at: Utc::now(),
            format,
            total_count: 0,
            async_export: false,
            filters: ContractSearchParams::default(),
        }
    }

    #[test]
    fn jsonl_export_empty_produces_only_header_line() {
        let metadata = minimal_export_metadata(ContractExportFormat::Jsonl);
        let output = render_contract_export(&ContractExportFormat::Jsonl, &metadata, &[])
            .expect("jsonl export should render with no records");

        let lines: Vec<&str> = output.lines().collect();
        // Only the metadata header line should be present.
        assert_eq!(lines.len(), 1, "empty export should have exactly one line");
        let header: serde_json::Value =
            serde_json::from_str(lines[0]).expect("header should be valid JSON");
        assert_eq!(header["_type"], "export_metadata");
        assert_eq!(header["format"], "jsonl");
        assert_eq!(header["total_count"], 0);
    }

    #[test]
    fn jsonl_export_with_records_produces_one_line_per_record_plus_header() {
        let metadata = ContractExportMetadata {
            exported_at: Utc::now(),
            format: ContractExportFormat::Jsonl,
            total_count: 2,
            async_export: false,
            filters: ContractSearchParams::default(),
        };
        let record = ContractMetadataExportRecord {
            id: Uuid::nil(),
            logical_id: None,
            contract_id: "CJSONLTEST".to_string(),
            wasm_hash: "aabbccdd".to_string(),
            name: "JSONL Contract".to_string(),
            description: None,
            publisher_id: Uuid::nil(),
            publisher_stellar_address: "GJSONLADDR".to_string(),
            publisher_username: None,
            network: "testnet".to_string(),
            is_verified: false,
            category: None,
            tags: vec![],
            maturity: None,
            health_score: 0,
            is_maintenance: false,
            deployment_count: 0,
            audit_status: None,
            visibility: "public".to_string(),
            organization_id: None,
            network_configs: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            verified_at: None,
            last_verified_at: None,
            last_accessed_at: None,
        };
        let records = vec![record.clone(), record];
        let output = render_contract_export(&ContractExportFormat::Jsonl, &metadata, &records)
            .expect("jsonl export should render");

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(
            lines.len(),
            3,
            "should have 1 header + 2 record lines, got {}: {:?}",
            lines.len(),
            lines
        );
        // Every line is valid JSON.
        for line in &lines {
            serde_json::from_str::<serde_json::Value>(line)
                .expect("each line should be valid JSON");
        }
        // First line is the metadata header.
        let header: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(header["_type"], "export_metadata");
        // Record lines contain the contract_id.
        assert!(lines[1].contains("CJSONLTEST"));
    }

    #[test]
    fn jsonl_format_display_is_jsonl() {
        assert_eq!(ContractExportFormat::Jsonl.to_string(), "jsonl");
    }

    #[test]
    fn jsonl_format_content_type_is_json() {
        assert_eq!(
            ContractExportFormat::Jsonl.content_type(),
            "application/json"
        );
    }

    #[test]
    fn csv_export_empty_has_header_row_only() {
        let metadata = minimal_export_metadata(ContractExportFormat::Csv);
        let output = render_contract_export(&ContractExportFormat::Csv, &metadata, &[])
            .expect("csv export should render with no records");

        // Must have metadata comments and the column header row; no data rows.
        assert!(output.contains("# exported_at="));
        assert!(output.contains("id,logical_id,contract_id"));
        let data_lines: Vec<&str> = output
            .lines()
            .filter(|l| !l.starts_with('#'))
            .skip(1) // skip the column header
            .collect();
        assert!(
            data_lines.is_empty(),
            "empty csv export should have no data rows, found: {data_lines:?}"
        );
    }

    #[test]
    fn parse_datetime_rfc3339_format() {
        let dt_str = "2024-01-15T10:30:00Z";
        let parsed = parse_datetime(dt_str);
        assert!(parsed.is_some());
        let dt = parsed.unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn parse_datetime_invalid_format_returns_none() {
        let dt_str = "2024-01-15 invalid";
        let parsed = parse_datetime(dt_str);
        assert!(parsed.is_none());
    }

    #[test]
    fn parse_datetime_empty_string_returns_none() {
        let parsed = parse_datetime("");
        assert!(parsed.is_none());
    }
}

// ────────────────────────────────────────────────────────────────────────────
// ADVANCED SEARCH (Issue #51)
// ────────────────────────────────────────────────────────────────────────────

/// Advanced contract search using a recursive Query DSL
#[allow(dead_code)]
#[utoipa::path(
    post,
    path = "/api/contracts/search",
    request_body = AdvancedSearchRequest,
    responses(
        (status = 200, description = "Search results", body = PaginatedResponse<Contract>),
        (status = 400, description = "Invalid query DSL")
    ),
    tag = "Contracts"
)]
pub async fn advanced_search_contracts(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<AdvancedSearchRequest>,
) -> ApiResult<Json<PaginatedResponse<Contract>>> {
    let limit = req.limit.unwrap_or(20).clamp(1, 100);
    let offset = req.offset.unwrap_or(0).max(0);
    let page = (offset / limit) + 1;

    let mut query_builder: sqlx::QueryBuilder<'_, sqlx::Postgres> =
        sqlx::QueryBuilder::new("SELECT c.* FROM contracts c ");

    // Add joins for sorting/filtering if needed
    query_builder.push("LEFT JOIN contract_interactions ci ON c.id = ci.contract_id ");
    query_builder.push("LEFT JOIN contract_versions cv ON c.id = cv.contract_id ");
    query_builder.push("LEFT JOIN contract_tags ct ON c.id = ct.contract_id ");
    query_builder.push("LEFT JOIN tags t ON t.id = ct.tag_id ");
    query_builder.push("WHERE 1=1 ");

    // Recursively build the WHERE clause
    build_where_clause(&mut query_builder, &req.query)?;

    query_builder.push(" GROUP BY c.id ");

    // Sorting
    let sort_by = req.sort_by.unwrap_or(shared::SortBy::CreatedAt);
    let sort_order = req.sort_order.unwrap_or(shared::SortOrder::Desc);
    let direction = if sort_order == shared::SortOrder::Asc {
        "ASC"
    } else {
        "DESC"
    };

    query_builder.push(" ORDER BY ");
    match sort_by {
        shared::SortBy::CreatedAt => {
            query_builder.push("c.created_at ");
        }
        shared::SortBy::UpdatedAt => {
            query_builder.push("c.updated_at ");
        }
        shared::SortBy::Popularity | shared::SortBy::Interactions => {
            query_builder.push("COUNT(DISTINCT ci.id) ");
        }
        shared::SortBy::Deployments => {
            query_builder.push("COUNT(DISTINCT cv.id) ");
        }
        shared::SortBy::Relevance => {
            query_builder.push("c.created_at "); // Default relevance if no query term
        }
        _ => {
            query_builder.push("c.created_at ");
        }
    }
    query_builder.push(direction);
    query_builder.push(", c.id DESC ");

    // Pagination
    query_builder.push(" LIMIT ");
    query_builder.push_bind(limit);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(offset);

    let query = query_builder.build_query_as::<Contract>();
    let mut contracts = query
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_internal_error("advanced search contracts", err))?;

    // Fetch tags for these contracts (keeps output consistent with list_contracts)
    let contract_ids: Vec<Uuid> = contracts.iter().map(|c| c.id).collect();
    if !contract_ids.is_empty() {
        #[derive(sqlx::FromRow)]
        struct TagRow {
            contract_id: Uuid,
            id: Uuid,
            name: String,
            color: String,
        }
        let tag_rows = sqlx::query_as::<_, TagRow>(
            r#"
            SELECT ct.contract_id, t.id, t.name, t.color
            FROM tags t
            JOIN contract_tags ct ON t.id = ct.tag_id
            WHERE ct.contract_id = ANY($1)
            "#,
        )
        .bind(&contract_ids)
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch tags (advanced search)", err))?;

        let mut tags_map: HashMap<Uuid, Vec<shared::Tag>> = HashMap::new();
        for row in tag_rows {
            tags_map
                .entry(row.contract_id)
                .or_default()
                .push(shared::Tag {
                    id: row.id,
                    name: row.name,
                    color: row.color,
                });
        }

        for contract in &mut contracts {
            if let Some(tags) = tags_map.remove(&contract.id) {
                contract.tags = tags;
            }
        }
    }

    // Count total matches (naively for now, same filters)
    let mut count_builder: sqlx::QueryBuilder<'_, sqlx::Postgres> =
        sqlx::QueryBuilder::new("SELECT COUNT(DISTINCT c.id) FROM contracts c ");
    count_builder.push("LEFT JOIN contract_tags ct ON c.id = ct.contract_id ");
    count_builder.push("LEFT JOIN tags t ON t.id = ct.tag_id ");
    count_builder.push("WHERE 1=1 ");
    build_where_clause(&mut count_builder, &req.query)?;

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count advanced search", err))?;

    Ok(Json(PaginatedResponse::new(contracts, total, page, limit)))
}

#[allow(dead_code)]
fn build_where_clause<'a>(
    builder: &mut sqlx::QueryBuilder<'a, sqlx::Postgres>,
    node: &'a QueryNode,
) -> ApiResult<()> {
    match node {
        QueryNode::Condition(cond) => {
            builder.push(" AND ");
            apply_condition(builder, cond)?;
        }
        QueryNode::Group {
            operator,
            conditions,
        } => {
            if conditions.is_empty() {
                return Ok(());
            }
            builder.push(" AND (");
            for (i, child) in conditions.iter().enumerate() {
                if i > 0 {
                    match operator {
                        QueryOperator::And => builder.push(" AND "),
                        QueryOperator::Or => builder.push(" OR "),
                    };
                }

                // For groups we need to wrap children
                match child {
                    QueryNode::Condition(c) => apply_condition(builder, c)?,
                    QueryNode::Group { .. } => {
                        builder.push(" (1=1 ");
                        build_where_clause(builder, child)?;
                        builder.push(") ");
                    }
                }
            }
            builder.push(") ");
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn apply_condition<'a>(
    builder: &mut sqlx::QueryBuilder<'a, sqlx::Postgres>,
    cond: &'a QueryCondition,
) -> ApiResult<()> {
    let field = match cond.field.as_str() {
        "name" => "c.name",
        "description" => "c.description",
        "category" => "c.category",
        "network" => "c.network",
        "verified" => "c.is_verified",
        "publisher" => "c.publisher_id",
        "tag" => "t.name",
        _ => {
            return Err(ApiError::bad_request_with(
                "InvalidField",
                format!("Field '{}' is not searchable", cond.field),
            ))
        }
    };

    let string_value = || {
        cond.value.as_str().ok_or_else(|| {
            ApiError::bad_request_with(
                "InvalidValue",
                format!(
                    "Field '{}' expects a string value for operator '{:?}'",
                    cond.field, cond.operator
                ),
            )
        })
    };

    builder.push(field);
    match cond.operator {
        FieldOperator::Eq => {
            builder.push(" = ");
            if cond.field == "verified" {
                let value = cond.value.as_bool().ok_or_else(|| {
                    ApiError::bad_request_with(
                        "InvalidValue",
                        "Field 'verified' expects a boolean value",
                    )
                })?;
                builder.push_bind(value);
            } else {
                builder.push_bind(string_value()?);
            }
        }
        FieldOperator::Ne => {
            builder.push(" != ");
            if cond.field == "verified" {
                let value = cond.value.as_bool().ok_or_else(|| {
                    ApiError::bad_request_with(
                        "InvalidValue",
                        "Field 'verified' expects a boolean value",
                    )
                })?;
                builder.push_bind(value);
            } else {
                builder.push_bind(string_value()?);
            }
        }
        FieldOperator::Gt => {
            builder.push(" > ");
            builder.push_bind(string_value()?);
        }
        FieldOperator::Lt => {
            builder.push(" < ");
            builder.push_bind(string_value()?);
        }
        FieldOperator::In => {
            builder.push(" IN (");
            let arr = cond.value.as_array().ok_or_else(|| {
                ApiError::bad_request_with(
                    "InvalidValue",
                    format!(
                        "Field '{}' expects an array value for operator 'in'",
                        cond.field
                    ),
                )
            })?;

            let mut separated = builder.separated(", ");
            for val in arr {
                if cond.field == "verified" {
                    let b = val.as_bool().ok_or_else(|| {
                        ApiError::bad_request_with(
                            "InvalidValue",
                            "Field 'verified' expects a boolean array value",
                        )
                    })?;
                    separated.push_bind(b);
                } else {
                    let s = val.as_str().ok_or_else(|| {
                        ApiError::bad_request_with(
                            "InvalidValue",
                            format!("Field '{}' expects a string array value", cond.field),
                        )
                    })?;
                    separated.push_bind(s.to_string());
                }
            }
            builder.push(")");
        }
        FieldOperator::Contains => {
            builder.push(" ILIKE ");
            let val = format!("%{}%", string_value()?);
            builder.push_bind(val);
        }
        FieldOperator::StartsWith => {
            builder.push(" ILIKE ");
            let val = format!("{}%", string_value()?);
            builder.push_bind(val);
        }
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// FAVORITE SEARCHES (Issue #51)
// ────────────────────────────────────────────────────────────────────────────

/// List favorite searches for the current user
#[utoipa::path(
    get,
    path = "/api/favorites/search",
    responses(
        (status = 200, description = "List of favorite searches", body = [FavoriteSearch])
    ),
    tag = "Favorites"
)]
pub async fn list_favorite_searches(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<FavoriteSearch>>> {
    // For now, return all since we don't have a strict user_id auth yet
    let favorites: Vec<FavoriteSearch> =
        sqlx::query_as("SELECT * FROM favorite_searches ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await
            .map_err(|err| db_internal_error("list favorite searches", err))?;

    Ok(Json(favorites))
}

/// Save a new favorite search
#[utoipa::path(
    post,
    path = "/api/favorites/search",
    request_body = SaveFavoriteSearchRequest,
    responses(
        (status = 201, description = "Favorite search saved", body = FavoriteSearch)
    ),
    tag = "Favorites"
)]
pub async fn save_favorite_search(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<SaveFavoriteSearchRequest>,
) -> ApiResult<Json<FavoriteSearch>> {
    let favorite: FavoriteSearch = sqlx::query_as(
        "INSERT INTO favorite_searches (name, query_json) VALUES ($1, $2) RETURNING *",
    )
    .bind(&req.name)
    .bind(&req.query_json)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("save favorite search", err))?;

    Ok(Json(favorite))
}

/// Delete a favorite search
#[utoipa::path(
    delete,
    path = "/api/favorites/search/{id}",
    params(
        ("id" = String, Path, description = "Favorite search ID")
    ),
    responses(
        (status = 204, description = "Favorite search deleted"),
        (status = 404, description = "Favorite search not found")
    ),
    tag = "Favorites"
)]
pub async fn delete_favorite_search(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request("InvalidId", "Invalid favorite search ID format"))?;

    let result = sqlx::query("DELETE FROM favorite_searches WHERE id = $1")
        .bind(uuid)
        .execute(&state.db)
        .await
        .map_err(|err| db_internal_error("delete favorite search", err))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::not_found(
            "FavoriteNotFound",
            "Favorite search not found",
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}

// Axum Handler to Export Logs using project native structs
pub async fn handle_export_audit(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<crate::audit::AuditRow>>> {
    let logs = sqlx::query_as::<_, crate::audit::AuditRow>(
        r#"
        SELECT id, actor_id, actor_email, operation, resource_type,
               resource_id, metadata, status, error_message, chain_hash,
               created_at
        FROM audit_logs
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DatabaseError",
            format!("Failed to export audit logs: {e}"),
        )
    })?;

    Ok(Json(logs))
}

// Axum Handler to enforce retention criteria
pub async fn handle_retention_cleanup(State(state): State<AppState>) -> ApiResult<String> {
    let result = sqlx::query("DELETE FROM audit_logs WHERE created_at < NOW() - INTERVAL '1 year'")
        .execute(&state.db)
        .await
        .map_err(|e| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DatabaseError",
                format!("Failed to prune logs: {e}"),
            )
        })?;

    Ok(format!("Pruned rows: {}", result.rows_affected()))
}
