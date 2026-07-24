use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════
// EXISTING REGISTRY TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Represents a tag that can be attached to a contract
#[derive(
    Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema, PartialEq, sqlx::Type,
)]
#[sqlx(type_name = "tag")]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}

/// Represents a smart contract in the registry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
#[schema(example = json!({
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "contract_id": "C...1234",
    "wasm_hash": "a1b2c3d4...",
    "name": "YieldOptimizer",
    "description": "Optimizes yield across protocols",
    "publisher_id": "550e8400-e29b-41d4-a716-446655440001",
    "network": "mainnet",
    "is_verified": true,
    "category": "DeFi",
    "tags": [
        {"id": "550e8400-e29b-41d4-a716-446655440005", "name": "yield", "color": "#888888"},
        {"id": "550e8400-e29b-41d4-a716-446655440006", "name": "optimization", "color": "#888888"}
    ],
    "created_at": "2023-10-27T10:00:00Z",
    "updated_at": "2023-10-27T10:00:00Z"
}))]
pub struct Contract {
    pub id: Uuid,
    pub contract_id: String,
    pub wasm_hash: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub publisher_id: Uuid,
    pub network: Network,
    pub is_verified: bool,
    /// Overall verification status for the contract (unverified, pending, verified, failed)
    pub verification_status: VerificationStatus,
    pub category: Option<String>,
    #[sqlx(skip)]
    #[serde(default)]
    pub tags: Vec<Tag>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub deployed_at: Option<DateTime<Utc>>,
    /// Who verified the contract (publisher/user id)
    pub verified_by: Option<Uuid>,
    /// Optional notes attached to the verification
    pub verification_notes: Option<String>,
    pub last_accessed_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub health_score: i32,
    #[sqlx(default)]
    pub is_maintenance: bool,
    /// Groups rows that represent the same logical contract across networks (Issue #43)
    #[sqlx(default)]
    pub logical_id: Option<Uuid>,
    /// Per-network config: { "mainnet": { contract_id, is_verified, min_version, max_version }, ... }
    #[sqlx(default)]
    pub network_configs: Option<serde_json::Value>,
    /// Search relevance score (calculated at runtime)
    #[sqlx(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relevance_score: Option<f64>,
    /// Organization that owns this contract (for private registries)
    pub organization_id: Option<Uuid>,
    /// Visibility level
    pub visibility: VisibilityType,
    /// The currently active version string for this contract (Issue #486)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<String>,
    /// Number of times this contract has been accessed via API
    #[serde(default)]
    pub usage_count: i64,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq, Default,
)]
#[sqlx(type_name = "visibility_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum VisibilityType {
    #[default]
    Public,
    Private,
}

/// Response for GET /contracts/:id with optional network-specific slice (Issue #43)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractGetResponse {
    #[serde(flatten)]
    pub contract: Contract,
    /// When ?network= is set, the requested network
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_network: Option<Network>,
    /// When ?network= is set, that network's config slice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_config: Option<NetworkConfig>,
}

/// Per-network config: address, verified status, min/max version (Issue #43)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NetworkConfig {
    pub contract_id: String,
    pub is_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum NetworkStatus {
    Online,
    Offline,
    Degraded,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NetworkEndpoints {
    pub rpc_url: String,
    pub health_url: String,
    pub explorer_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub friendbot_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NetworkInfo {
    pub id: String,
    pub name: String,
    pub network_type: Network,
    pub status: NetworkStatus,
    pub endpoints: NetworkEndpoints,
    pub last_checked_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_indexed_ledger_height: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_indexed_at: Option<DateTime<Utc>>,
    pub consecutive_failures: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NetworkListResponse {
    pub networks: Vec<NetworkInfo>,
    pub cached_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NetworkHealth {
    pub network_id: String,
    pub name: String,
    pub status: NetworkStatus,
    pub rpc_available: bool,
    pub last_indexed_ledger: Option<i64>,
    pub current_ledger: Option<u32>,
    pub indexer_lag: Option<i64>,
    pub last_checked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct NetworkHealthResponse {
    pub health: Vec<NetworkHealth>,
    pub timestamp: DateTime<Utc>,
}

/// Network where the contract is deployed
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq, Eq)]
#[sqlx(type_name = "network_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    Futurenet,
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Testnet => write!(f, "testnet"),
            Network::Futurenet => write!(f, "futurenet"),
        }
    }
}

fn parse_network_value<E: de::Error>(value: &str) -> Result<Network, E> {
    match value.trim().to_ascii_lowercase().as_str() {
        "mainnet" => Ok(Network::Mainnet),
        "testnet" => Ok(Network::Testnet),
        "futurenet" => Ok(Network::Futurenet),
        _ => Err(E::custom(format!(
            "invalid network `{value}`; expected mainnet, testnet, or futurenet"
        ))),
    }
}

fn deserialize_optional_networks<'de, D>(deserializer: D) -> Result<Option<Vec<Network>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct NetworksVisitor;

    impl<'de> Visitor<'de> for NetworksVisitor {
        type Value = Option<Vec<Network>>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a comma-separated string or sequence of network names")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut networks = Vec::new();
            while let Some(network) = seq.next_element::<Network>()? {
                networks.push(network);
            }

            if networks.is_empty() {
                Ok(None)
            } else {
                Ok(Some(networks))
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let networks: Result<Vec<_>, _> = value
                .split(',')
                .map(str::trim)
                .filter(|network| !network.is_empty())
                .map(parse_network_value)
                .collect();

            let networks = networks?;
            if networks.is_empty() {
                Ok(None)
            } else {
                Ok(Some(networks))
            }
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }

        fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(value)
        }
    }

    deserializer.deserialize_any(NetworksVisitor)
}

/// Deserialize a free-form string filter that may arrive either as a
/// comma-separated string (`?categories=DeFi,NFT`) or as repeated query
/// parameters (`?categories=DeFi&categories=NFT`), so both callers agree.
///
/// Blank entries are dropped and an all-blank filter deserializes to `None`,
/// matching `deserialize_optional_networks`.
pub fn deserialize_optional_string_list<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct StringListVisitor;

    impl<'de> Visitor<'de> for StringListVisitor {
        type Value = Option<Vec<String>>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a comma-separated string or sequence of values")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut values = Vec::new();
            // Repeated params may themselves be comma-separated, so split again.
            while let Some(value) = seq.next_element::<String>()? {
                values.extend(
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned),
                );
            }

            if values.is_empty() {
                Ok(None)
            } else {
                Ok(Some(values))
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let values: Vec<String> = value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect();

            if values.is_empty() {
                Ok(None)
            } else {
                Ok(Some(values))
            }
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }

        fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(value)
        }
    }

    deserializer.deserialize_any(StringListVisitor)
}

/// Upgrade strategy for contract upgrades
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "upgrade_strategy_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UpgradeStrategy {
    Proxy,
    Uups,
    DataMigration,
    ShadowContract,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq)]
#[sqlx(type_name = "source_format_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SourceFormat {
    Rust,
    Wasm,
}

/// Supported source storage backends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackend {
    Local,
    S3,
    Gcs,
}

impl std::fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageBackend::Local => write!(f, "local"),
            StorageBackend::S3 => write!(f, "s3"),
            StorageBackend::Gcs => write!(f, "gcs"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractSource {
    pub id: Uuid,
    pub contract_version_id: Uuid,
    pub source_format: SourceFormat,
    pub storage_backend: String,
    pub storage_key: String,
    pub source_hash: String,
    pub source_size: i64,
    pub created_at: DateTime<Utc>,
}

/// Contract version information
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractVersion {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub wasm_hash: String,
    pub source_url: Option<String>,
    pub commit_hash: Option<String>,
    pub release_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    #[sqlx(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_schema: Option<serde_json::Value>,
    /// Optional Ed25519 signature over "{contract_id}:{version}:{wasm_hash}"
    #[sqlx(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Publisher's public key corresponding to the signature (base64-encoded ed25519 key)
    #[sqlx(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_key: Option<String>,
    /// Signature algorithm identifier (e.g. "ed25519")
    #[sqlx(default)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_algorithm: Option<String>,
    /// Structured notes describing what changed in this version (Issue #486)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_notes: Option<String>,
    /// True when this version was created by reverting to a previous version (Issue #486)
    #[serde(default)]
    pub is_revert: bool,
    /// The version string that was reverted to, when is_revert = true (Issue #486)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverted_from: Option<String>,
}

/// Represents a historical version of contract metadata (#729)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractMetadataVersion {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub user_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub change_summary: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Represents a difference in a single field of metadata (#729)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MetadataDiff {
    pub field: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

/// Response containing the metadata history for a contract (#729)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MetadataHistoryResponse {
    pub contract_id: Uuid,
    pub versions: Vec<ContractMetadataVersion>,
}

// ═══════════════════════════════════════════════════════════════════════════
// MULTI-TENANCY TYPES (Issue #420)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq)]
#[sqlx(type_name = "organization_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum OrganizationRole {
    Admin,
    Member,
    Viewer,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub is_private: bool,
    pub quota_contracts: i32,
    pub rate_limit_requests: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct OrganizationMember {
    pub organization_id: Uuid,
    pub publisher_id: Uuid,
    pub role: OrganizationRole,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct OrganizationInvitation {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub email: String,
    pub role: OrganizationRole,
    pub token: String,
    pub inviter_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub is_private: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InviteMemberRequest {
    pub email: String,
    pub role: OrganizationRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateOrganizationRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_private: Option<bool>,
}

/// Verification status and details
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct Verification {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub status: VerificationStatus,
    pub source_code: Option<String>,
    pub build_params: Option<serde_json::Value>,
    pub compiler_version: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Verification status enum
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "verification_status", rename_all = "lowercase")]
pub enum VerificationStatus {
    #[serde(rename = "unverified")]
    Unverified,
    Pending,
    Verified,
    Failed,
}

/// Security audit status of the contract (Issue #401)
#[derive(
    Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq, Default,
)]
#[sqlx(type_name = "audit_status_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditStatus {
    #[default]
    None,
    Pending,
    Passed,
    Failed,
}

/// Contract maturity level - indicates stability and production readiness
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub enum MaturityLevel {
    Experimental,
    Beta,
    Stable,
    Production,
}

/// Publisher/developer information
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
#[schema(example = json!({
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "stellar_address": "GABC...",
    "username": "SorobanDev",
    "email": "dev@soroban.io",
    "github_url": "https://github.com/sorobandev",
    "website": "https://soroban.io",
    "created_at": "2023-10-27T10:00:00Z"
}))]
pub struct Publisher {
    pub id: Uuid,
    pub stellar_address: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub github_url: Option<String>,
    pub website: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// User preferences and settings
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
#[schema(example = json!({
    "id": "550e8400-e29b-41d4-a716-446655440002",
    "publisher_id": "550e8400-e29b-41d4-a716-446655440001",
    "theme": "dark",
    "language": "en",
    "default_network": "testnet",
    "favorites": ["550e8400-e29b-41d4-a716-446655440000"],
    "extensible_settings": {},
    "created_at": "2023-10-27T10:00:00Z",
    "updated_at": "2023-10-27T10:00:00Z"
}))]
pub struct UserPreferences {
    pub id: Uuid,
    pub publisher_id: Uuid,
    pub theme: String,
    pub language: String,
    pub default_network: Network,
    pub favorites: serde_json::Value,
    pub extensible_settings: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Contract interaction statistics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractStats {
    pub contract_id: Uuid,
    pub total_deployments: i64,
    pub total_interactions: i64,
    pub unique_users: i64,
    pub last_interaction: Option<DateTime<Utc>>,
}

/// GraphNode (minimal contract info for graph rendering)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct GraphNode {
    pub id: Uuid,
    pub contract_id: String,
    pub name: String,
    pub network: Network,
    pub is_verified: bool,
    pub category: Option<String>,
    #[sqlx(skip)]
    pub tags: Vec<Tag>,
}

/// Graph edge (dependency relationship)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct GraphEdge {
    pub source: Uuid,
    pub target: Uuid,
    pub dependency_type: String,
    pub call_frequency: Option<i64>,
    pub call_volume: Option<i64>,
    pub is_estimated: bool,
    pub is_circular: bool,
}

/// Full graph response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GraphResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

// ── Graph analysis types ──────────────────────────────────────────────────────

/// A detected sub-network (community) within the contract interaction graph.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GraphCluster {
    /// Stable cluster identifier (integer label from community detection).
    pub cluster_id: usize,
    /// Contract UUIDs that belong to this cluster.
    pub members: Vec<Uuid>,
    /// The highest-degree node in the cluster — acts as the cluster hub.
    pub hub_contract_id: Option<Uuid>,
    /// Average internal edge weight (call frequency) within the cluster.
    pub cohesion: f64,
    /// Number of edges crossing into other clusters.
    pub external_edges: usize,
}

/// Per-contract criticality ranking combining multiple centrality measures.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CriticalContractScore {
    pub contract_id: Uuid,
    pub contract_name: String,
    /// Combined criticality score in [0, 1]; higher = more critical.
    pub criticality_score: f64,
    /// PageRank score — measures influence propagated through in-edges.
    pub pagerank: f64,
    /// Betweenness centrality — fraction of shortest paths passing through.
    pub betweenness: f64,
    /// In-degree: number of contracts that directly depend on this one.
    pub in_degree: usize,
    /// Out-degree: number of contracts this one directly depends on.
    pub out_degree: usize,
    /// Cluster this contract belongs to.
    pub cluster_id: Option<usize>,
}

/// One hop in a vulnerability propagation path.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PropagationHop {
    pub contract_id: Uuid,
    pub contract_name: String,
    /// Hops from the vulnerable source contract.
    pub depth: usize,
    /// Accumulated risk score at this node (decays with distance).
    pub risk_score: f64,
    /// Direct dependents that are also at risk.
    pub propagates_to: Vec<Uuid>,
}

/// Result of a vulnerability propagation analysis from one or more source contracts.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VulnerabilityPropagationResult {
    /// The contract(s) where the vulnerability originates.
    pub source_contracts: Vec<Uuid>,
    /// All contracts reachable from sources, ordered by risk score.
    pub affected_contracts: Vec<PropagationHop>,
    /// Total number of contracts at risk (any depth).
    pub total_affected: usize,
    /// Maximum propagation depth reached.
    pub max_depth: usize,
    /// True if the propagation path contains a cycle.
    pub has_cycles: bool,
}

/// Complete graph analysis report.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GraphAnalysisReport {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub clusters: Vec<GraphCluster>,
    pub critical_contracts: Vec<CriticalContractScore>,
    /// IDs of contracts that belong to strongly-connected components (cycles).
    pub cyclic_contracts: Vec<Uuid>,
    pub analysis_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolComplianceStatus {
    Compliant,
    Partial,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InteroperabilityCapabilityKind {
    Bridge,
    Adapter,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InteroperabilityProtocolMatch {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub status: ProtocolComplianceStatus,
    pub matched_functions: Vec<String>,
    pub missing_functions: Vec<String>,
    pub optional_matches: Vec<String>,
    pub compliance_score: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InteroperabilityCapability {
    pub kind: InteroperabilityCapabilityKind,
    pub label: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InteroperabilitySuggestion {
    pub contract_id: Uuid,
    pub contract_address: String,
    pub contract_name: String,
    pub network: Network,
    pub category: Option<String>,
    pub is_verified: bool,
    pub score: f64,
    pub reason: String,
    pub shared_protocols: Vec<String>,
    pub shared_functions: Vec<String>,
    pub relation_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InteroperabilitySummary {
    pub protocol_matches: usize,
    pub compatible_contracts: usize,
    pub suggested_contracts: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub bridge_signals: usize,
    pub adapter_signals: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractInteroperabilityResponse {
    pub contract_id: Uuid,
    pub contract_address: String,
    pub contract_name: String,
    pub network: Network,
    pub analyzed_at: DateTime<Utc>,
    pub has_abi: bool,
    pub analyzed_functions: Vec<String>,
    pub warnings: Vec<String>,
    pub protocols: Vec<InteroperabilityProtocolMatch>,
    pub capabilities: Vec<InteroperabilityCapability>,
    pub suggestions: Vec<InteroperabilitySuggestion>,
    pub graph: GraphResponse,
    pub summary: InteroperabilitySummary,
}

/// Request to publish a new contract
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublishRequest {
    pub contract_id: String,
    pub wasm_hash: String,
    pub name: String,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub network: Network,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub source_url: Option<String>,
    pub publisher_address: String,
    // Dependencies (new field)
    #[serde(default)]
    pub dependencies: Vec<DependencyDeclaration>,
    /// Whether this was published via CI/CD (Issue #529)
    #[serde(default)]
    pub is_cicd: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateContractMetadataRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub user_id: Option<Uuid>,
}

/// Item in a batch metadata update request (#849)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchMetadataUpdateItem {
    pub contract_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub change_summary: Option<String>,
}

/// Request body for POST /api/contracts/metadata/batch (#849)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchMetadataUpdateRequest {
    pub items: Vec<BatchMetadataUpdateItem>,
    pub user_id: Option<Uuid>,
    /// Client-supplied UUID for correlating audit log entries across the batch
    pub batch_id: Option<String>,
}

/// Per-item result within a batch metadata update response (#849)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchMetadataUpdateItemResult {
    pub contract_id: String,
    pub ok: bool,
    pub rollback_version_id: Option<Uuid>,
    pub error: Option<String>,
}

/// Response body for POST /api/contracts/metadata/batch (#849)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchMetadataUpdateResponse {
    pub batch_id: String,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<BatchMetadataUpdateItemResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ChangePublisherRequest {
    pub publisher_address: String,
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateContractStatusRequest {
    pub status: String,
    pub error_message: Option<String>,
    pub user_id: Option<Uuid>,
}

/// Item for bulk contract status updates
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BulkStatusUpdateItem {
    pub id: Uuid,
    pub status: String,
    pub error_message: Option<String>,
    pub user_id: Option<Uuid>,
}

/// Bulk status update request body
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BulkStatusUpdateRequest {
    pub items: Vec<BulkStatusUpdateItem>,
}

/// Request to create a new contract version with ABI
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateContractVersionRequest {
    pub contract_id: String,
    pub version: String,
    pub wasm_hash: String,
    pub abi: serde_json::Value,
    pub source_url: Option<String>,
    pub commit_hash: Option<String>,
    pub release_notes: Option<String>,
    /// Structured change notes for this version (Issue #486)
    #[serde(default)]
    pub change_notes: Option<String>,
    /// Optional Ed25519 signature and publisher key metadata for this version
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub publisher_key: Option<String>,
    #[serde(default)]
    pub signature_algorithm: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// VERSION TRACKING TYPES (Issue #486)
// ═══════════════════════════════════════════════════════════════════════════

/// A single field-level difference between two contract versions
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VersionFieldDiff {
    /// Name of the field that changed
    pub field: String,
    /// Value in the `from` version (null if the field was absent)
    pub from_value: Option<serde_json::Value>,
    /// Value in the `to` version (null if the field was removed)
    pub to_value: Option<serde_json::Value>,
}

/// Response for GET /api/contracts/:id/versions/compare
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VersionCompareResponse {
    pub contract_id: Uuid,
    pub from_version: ContractVersion,
    pub to_version: ContractVersion,
    /// List of fields that differ between the two versions
    pub differences: Vec<VersionFieldDiff>,
    /// Whether the WASM hash changed (indicates a code change)
    pub wasm_changed: bool,
}

/// Request body for POST /api/admin/contracts/:id/versions/:version/revert
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RevertVersionRequest {
    /// Optional notes explaining why the revert was performed
    pub change_notes: Option<String>,
    /// UUID of the admin performing the revert (used for audit log)
    pub admin_id: Uuid,
}

// ────────────────────────────────────────────────────────────────────────────
// Deprecation management (issue #65)
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeprecationStatus {
    Active,
    Deprecated,
    Retired,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DeprecationInfo {
    pub contract_id: String,
    pub status: DeprecationStatus,
    pub deprecated_at: Option<DateTime<Utc>>,
    pub retirement_at: Option<DateTime<Utc>>,
    pub replacement_contract_id: Option<String>,
    pub migration_guide_url: Option<String>,
    pub notes: Option<String>,
    pub days_remaining: Option<i64>,
    pub dependents_notified: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DeprecateContractRequest {
    pub retirement_at: DateTime<Utc>,
    pub replacement_contract_id: Option<String>,
    pub migration_guide_url: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct DeprecationNotification {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub deprecated_contract_id: Uuid,
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
}

/// Response for impact analysis
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ImpactAnalysisResponse {
    pub contract_id: Uuid,
    pub change_type: Option<String>,
    pub affected_count: usize,
    pub affected_contracts: Vec<Contract>,
    pub has_cycles: bool,
}
/// Dependency declaration in publish request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DependencyDeclaration {
    pub name: String,
    pub version_constraint: String,
}

/// Contract dependency record (database row)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractDependency {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub dependency_name: String,
    pub dependency_contract_id: Option<Uuid>,
    pub version_constraint: String,
    pub created_at: DateTime<Utc>,
}

/// Tracks migration scripts between contract versions
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct MigrationScript {
    pub id: Uuid,
    pub from_version: Uuid,
    pub to_version: Uuid,
    pub script_path: String,
    pub checksum: String,
    pub created_at: DateTime<Utc>,
}

/// Recursive dependency tree node for API response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DependencyTreeNode {
    pub contract_id: String, // Public key ID
    pub name: String,
    pub current_version: String,
    pub constraint_to_parent: String,
    pub dependencies: Vec<DependencyTreeNode>,
}

/// Request to verify a contract
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VerifyRequest {
    pub contract_id: String,
    pub source_code: String,
    pub build_params: serde_json::Value,
    pub compiler_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchVerifyItem {
    pub contract_id: String,
    #[serde(default)]
    pub source_code: Option<String>,
    #[serde(default)]
    pub build_params: Option<serde_json::Value>,
    #[serde(default)]
    pub compiler_version: Option<String>,
    /// "basic" | "standard" | "strict"
    #[serde(default)]
    pub level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchVerifyRequest {
    pub contracts: Vec<BatchVerifyItem>,
}

/// Lifecycle state of an asynchronous batch verification job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BatchVerifyJobStatus {
    Pending,
    Processing,
    Completed,
    /// Job finished but at least one contract could not be verified.
    PartialFailure,
    Failed,
}

/// Per-contract result inside a batch job.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchVerifyJobResult {
    pub contract_id: String,
    pub verified: bool,
    /// Human-readable failure reason; absent on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wasm_hash_matches: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abi_valid: Option<bool>,
}

/// Response returned when a batch job is submitted or polled.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchVerifyJobResponse {
    pub job_id: Uuid,
    pub status: BatchVerifyJobStatus,
    pub total: usize,
    pub verified: usize,
    pub failed: usize,
    pub submitted_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Per-contract results; populated once the job reaches Completed or PartialFailure.
    #[serde(default)]
    pub results: Vec<BatchVerifyJobResult>,
    pub status_url: String,
}

/// Sorting options for contracts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub enum SortBy {
    #[serde(rename = "created_at", alias = "createdat")]
    CreatedAt,
    #[serde(rename = "updated_at", alias = "updatedat")]
    UpdatedAt,
    VerifiedAt,
    LastAccessedAt,
    Popularity,
    #[serde(rename = "deployments")]
    Deployments,
    // Kept for backwards/UX compatibility: the frontend supports "downloads".
    #[serde(rename = "interactions", alias = "downloads")]
    Interactions,
    #[serde(rename = "relevance")]
    Relevance,
}

/// Sorting order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// Search/filter parameters for contracts
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct ContractSearchParams {
    pub query: Option<String>,
    pub network: Option<Network>,
    /// Multiple networks filter, comma-separated: `?networks=mainnet,testnet`.
    /// (Repeating the key is rejected by the query extractor as a duplicate
    /// field; a sequence is still accepted when deserializing from JSON.)
    #[serde(default, deserialize_with = "deserialize_optional_networks")]
    pub networks: Option<Vec<Network>>,
    pub verified_only: Option<bool>,
    /// Filter by verification_status (unverified, pending, verified, failed)
    pub verification_status: Option<VerificationStatus>,
    pub category: Option<String>,
    /// Multiple categories filter, comma-separated: `?categories=DeFi,NFT`.
    #[serde(default, deserialize_with = "deserialize_optional_string_list")]
    pub categories: Option<Vec<String>>,
    /// Multiple tags filter, comma-separated: `?tags=defi,amm`.
    #[serde(default, deserialize_with = "deserialize_optional_string_list")]
    pub tags: Option<Vec<String>>,
    pub maturity: Option<MaturityLevel>,
    pub page: Option<i64>,
    #[serde(alias = "page_size")]
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<SortBy>,
    pub sort_order: Option<SortOrder>,
    pub cursor: Option<String>,
    pub created_from: Option<DateTime<Utc>>,
    pub created_to: Option<DateTime<Utc>>,
    pub updated_from: Option<DateTime<Utc>>,
    pub updated_to: Option<DateTime<Utc>>,
    pub verified_from: Option<DateTime<Utc>>,
    pub verified_to: Option<DateTime<Utc>>,
    pub last_accessed_from: Option<DateTime<Utc>>,
    pub last_accessed_to: Option<DateTime<Utc>>,
    // Weights for ranking
    pub w_text: Option<f64>,
    pub w_pop: Option<f64>,
    pub w_rec: Option<f64>,
    pub w_rat: Option<f64>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ContractExportFormat {
    Json,
    /// Newline-delimited JSON (one object per line). Suitable for streaming
    /// and processing with tools such as `jq` and `dbt`.
    Jsonl,
    Csv,
    Yaml,
}

impl std::fmt::Display for ContractExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json => write!(f, "json"),
            Self::Jsonl => write!(f, "jsonl"),
            Self::Csv => write!(f, "csv"),
            Self::Yaml => write!(f, "yaml"),
        }
    }
}

impl ContractExportFormat {
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Json | Self::Jsonl => "application/json",
            Self::Csv => "text/csv; charset=utf-8",
            Self::Yaml => "application/yaml",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Jsonl => "jsonl",
            Self::Csv => "csv",
            Self::Yaml => "yaml",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractExportRequest {
    pub format: ContractExportFormat,
    #[serde(default)]
    pub filters: ContractSearchParams,
    #[serde(default)]
    pub async_mode: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ContractMetadataExportRecord {
    pub id: Uuid,
    pub logical_id: Option<Uuid>,
    pub contract_id: String,
    pub wasm_hash: String,
    pub name: String,
    pub description: Option<String>,
    pub publisher_id: Uuid,
    pub publisher_stellar_address: String,
    pub publisher_username: Option<String>,
    pub network: String,
    pub is_verified: bool,
    pub category: Option<String>,
    #[sqlx(skip)]
    pub tags: Vec<Tag>,
    pub maturity: Option<String>,
    pub health_score: i32,
    pub is_maintenance: bool,
    pub deployment_count: i32,
    pub audit_status: Option<String>,
    pub visibility: String,
    pub organization_id: Option<Uuid>,
    pub network_configs: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub verified_at: Option<DateTime<Utc>>,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub last_accessed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractExportMetadata {
    pub exported_at: DateTime<Utc>,
    pub format: ContractExportFormat,
    pub total_count: i64,
    pub async_export: bool,
    pub filters: ContractSearchParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractMetadataExportEnvelope {
    pub metadata: ContractExportMetadata,
    pub contracts: Vec<ContractMetadataExportRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContractExportJobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractExportAcceptedResponse {
    pub job_id: Uuid,
    pub status: ContractExportJobStatus,
    pub status_url: String,
    pub download_url: Option<String>,
    pub total_count: i64,
    pub format: ContractExportFormat,
    pub requested_at: DateTime<Utc>,
    pub filters: ContractSearchParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractExportStatusResponse {
    pub job_id: Uuid,
    pub status: ContractExportJobStatus,
    pub status_url: String,
    pub download_url: Option<String>,
    pub total_count: i64,
    pub format: ContractExportFormat,
    pub requested_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub filters: ContractSearchParams,
    pub error: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
// BULK IMPORT TYPES
// ═══════════════════════════════════════════════════════════════════════════

/// Single contract record for bulk import
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractImportRecord {
    pub contract_id: String,
    pub wasm_hash: String,
    pub name: String,
    pub description: Option<String>,
    pub publisher_address: String,
    pub network: Network,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub is_verified: Option<bool>,
    pub maturity: Option<String>,
    pub visibility: Option<String>,
    pub organization_id: Option<Uuid>,
    pub network_configs: Option<serde_json::Value>,
    pub versions: Option<Vec<ContractImportVersion>>,
}

/// Version information for import
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractImportVersion {
    pub version: String,
    pub wasm_hash: String,
    pub source_url: Option<String>,
    pub commit_hash: Option<String>,
    pub release_notes: Option<String>,
}

/// Request to import contracts in bulk
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractImportRequest {
    pub contracts: Vec<ContractImportRecord>,
    #[serde(default)]
    pub fail_safe: bool,
    #[serde(default)]
    pub async_mode: Option<bool>,
    #[serde(default)]
    pub skip_existing: Option<bool>,
}

/// Result of a single contract import
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractImportItemResult {
    pub index: usize,
    pub contract_id: String,
    pub success: bool,
    pub imported_id: Option<Uuid>,
    pub error: Option<String>,
}

/// Response for synchronous bulk import
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractImportResponse {
    pub total_count: usize,
    pub imported_count: usize,
    pub failed_count: usize,
    pub results: Vec<ContractImportItemResult>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContractImportJobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Partial,
}

/// Response when import is accepted for async processing
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractImportAcceptedResponse {
    pub job_id: Uuid,
    pub status: ContractImportJobStatus,
    pub status_url: String,
    pub total_count: i64,
    pub requested_at: DateTime<Utc>,
    pub fail_safe: bool,
}

/// Status response for an async import job
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractImportStatusResponse {
    pub job_id: Uuid,
    pub status: ContractImportJobStatus,
    pub status_url: String,
    pub total_count: i64,
    pub processed_count: i64,
    pub imported_count: i64,
    pub failed_count: i64,
    pub requested_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub fail_safe: bool,
    pub error: Option<String>,
    pub results: Option<Vec<ContractImportItemResult>>,
}

// ═══════════════════════════════════════════════════════════════════════════
// BULK EXPORT GET REQUEST (Query params)
// ═══════════════════════════════════════════════════════════════════════════

/// Query parameters for GET /contracts/export
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::IntoParams)]
pub struct ContractExportQueryParams {
    pub format: Option<ContractExportFormat>,
    pub networks: Option<Vec<Network>>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub verified_only: Option<bool>,
    pub maturity: Option<String>,
    pub query: Option<String>,
    pub async_mode: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum FieldOperator {
    Eq,
    Ne,
    Gt,
    Lt,
    In,
    Contains,
    StartsWith,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct QueryCondition {
    pub field: String,
    pub operator: FieldOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum QueryOperator {
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub enum QueryNode {
    Condition(QueryCondition),
    Group {
        operator: QueryOperator,
        conditions: Vec<QueryNode>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AdvancedSearchRequest {
    pub query: QueryNode,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<SortBy>,
    pub sort_order: Option<SortOrder>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct FavoriteSearch {
    pub id: Uuid,
    pub user_address: String,
    pub name: String,
    pub query_json: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SaveFavoriteSearchRequest {
    pub user_address: String,
    pub name: String,
    pub query_json: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchSuggestion {
    pub text: String,
    pub kind: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchSuggestionsResponse {
    pub items: Vec<SearchSuggestion>,
}

/// Pagination params for contract versions (limit/offset style)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VersionPaginationParams {
    #[serde(default = "default_version_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_version_limit() -> i64 {
    20
}

/// Paginated version response (limit/offset style per issue #32 spec)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PaginatedVersionResponse {
    pub items: Vec<ContractVersion>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
}

/// Active search/list filter metadata included in paginated responses
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchFilterMetadata {
    /// Applied network filters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub networks: Option<Vec<String>>,
    /// Applied category filters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    /// Whether only verified contracts are shown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_only: Option<bool>,
    /// Verification status filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_status: Option<String>,
    /// Applied tag filters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// Maturity level filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maturity: Option<String>,
    /// Text query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    /// Number of items per page (serialised as `per_page`).
    #[serde(rename = "per_page")]
    pub page_size: i64,
    /// Total number of pages (serialised as `pages`).
    #[serde(rename = "pages")]
    pub total_pages: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<String>,
    /// Active filters applied to this result set (only present for search/list endpoints)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<SearchFilterMetadata>,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: i64, page: i64, limit: i64) -> Self {
        let total_pages = if limit > 0 {
            (total as f64 / limit as f64).ceil() as i64
        } else {
            0
        };
        Self {
            items,
            total,
            page,
            page_size: limit,
            total_pages,
            next_cursor: None,
            prev_cursor: None,
            filters: None,
        }
    }

    pub fn with_filters(mut self, filters: SearchFilterMetadata) -> Self {
        self.filters = Some(filters);
        self
    }

    pub fn with_cursors(mut self, next: Option<String>, prev: Option<String>) -> Self {
        self.next_cursor = next;
        self.prev_cursor = prev;
        self
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT INTERACTION HISTORY (Issue #46)
// ═══════════════════════════════════════════════════════════════════════════

/// One contract invocation row (DB)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractInteraction {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub user_address: Option<String>,
    pub interaction_type: String,
    pub transaction_hash: Option<String>,
    pub method: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub return_value: Option<serde_json::Value>,
    pub interaction_timestamp: Option<DateTime<Utc>>,
    pub interaction_count: Option<i64>,
    pub network: Option<Network>,
    pub created_at: DateTime<Utc>,
}

/// Response item for GET /api/contracts/:id/interactions
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractInteractionResponse {
    pub id: Uuid,
    pub account: Option<String>,
    pub method: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub return_value: Option<serde_json::Value>,
    pub transaction_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct InteractionsQueryParams {
    #[serde(default = "default_interactions_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub account: Option<String>,
    pub method: Option<String>,
    pub from_timestamp: Option<String>,
    pub to_timestamp: Option<String>,
    pub days: Option<i64>,
    pub interaction_type: Option<String>,
    pub network: Option<Network>,
    pub cursor: Option<String>,
}

fn default_interactions_limit() -> i64 {
    50
}

/// Request body for POST /api/contracts/:id/interactions (single)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateInteractionRequest {
    pub account: Option<String>,
    pub interaction_type: Option<String>,
    pub method: Option<String>,
    pub transaction_hash: Option<String>,
    pub target_contract_id: Option<String>,
    pub parameters: Option<serde_json::Value>,
    pub return_value: Option<serde_json::Value>,
    pub timestamp: Option<DateTime<Utc>>,
    pub network: Option<Network>,
}

/// Request body for POST /api/contracts/:id/interactions/batch
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateInteractionBatchRequest {
    pub interactions: Vec<CreateInteractionRequest>,
}

/// Paginated interactions response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InteractionsListResponse {
    pub items: Vec<ContractInteractionResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InteractionTimeSeriesPoint {
    pub date: chrono::NaiveDate,
    pub interaction_type: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InteractionTimeSeriesResponse {
    pub contract_id: Uuid,
    pub days: i64,
    pub interactions_this_week: i64,
    pub interactions_last_week: i64,
    pub is_trending: bool,
    pub series: Vec<InteractionTimeSeriesPoint>,
}

/// Migration status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, utoipa::ToSchema)]
#[sqlx(type_name = "migration_status", rename_all = "snake_case")]
pub enum MigrationStatus {
    Pending,
    Success,
    Failed,
    RolledBack,
}

/// Represents a contract state migration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct Migration {
    pub id: Uuid,
    pub contract_id: String,
    pub status: MigrationStatus,
    pub wasm_hash: String,
    pub log_output: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a new migration record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMigrationRequest {
    pub contract_id: String,
    pub wasm_hash: String,
}

/// Request to update a migration's status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMigrationStatusRequest {
    pub status: MigrationStatus,
    pub log_output: Option<String>,
}

// ────────────────────────────────────────────────────────────────────────────
// Contributor models
// ────────────────────────────────────────────────────────────────────────────

/// A contract creator profile with optional verification badge
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Contributor {
    pub id: Uuid,
    pub stellar_address: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub links: serde_json::Value,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Contributor profile with aggregated contract stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorWithStats {
    pub id: Uuid,
    pub stellar_address: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub links: serde_json::Value,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub contract_count: i64,
}

impl ContributorWithStats {
    pub fn from_contributor(c: Contributor, contract_count: i64) -> Self {
        Self {
            id: c.id,
            stellar_address: c.stellar_address,
            name: c.name,
            avatar_url: c.avatar_url,
            bio: c.bio,
            links: c.links,
            is_verified: c.is_verified,
            created_at: c.created_at,
            updated_at: c.updated_at,
            contract_count,
        }
    }
}

/// Request to create a contributor profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateContributorRequest {
    pub stellar_address: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub links: Option<serde_json::Value>,
}

/// Request to update a contributor profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateContributorRequest {
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub links: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, utoipa::ToSchema)]
#[sqlx(type_name = "deployment_environment", rename_all = "lowercase")]
pub enum DeploymentEnvironment {
    Blue,
    Green,
}

impl std::fmt::Display for DeploymentEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentEnvironment::Blue => write!(f, "blue"),
            DeploymentEnvironment::Green => write!(f, "green"),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, utoipa::ToSchema)]
#[sqlx(type_name = "deployment_status", rename_all = "lowercase")]
pub enum DeploymentStatus {
    Active,
    Inactive,
    Testing,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractDeploymentHistory {
    pub deployed_at: DateTime<Utc>,
    pub network: Network,
    pub deployer_address: Option<String>,
    pub transaction_hash: Option<String>,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct DeploymentHistoryQueryParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractDeployment {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub environment: DeploymentEnvironment,
    pub status: DeploymentStatus,
    pub wasm_hash: String,
    pub deployed_at: DateTime<Utc>,
    pub activated_at: Option<DateTime<Utc>>,
    pub health_checks_passed: i32,
    pub health_checks_failed: i32,
    pub last_health_check_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct DeploymentSwitch {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub from_environment: DeploymentEnvironment,
    pub to_environment: DeploymentEnvironment,
    pub switched_at: DateTime<Utc>,
    pub switched_by: Option<String>,
    pub rollback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "canary_status", rename_all = "snake_case")]
pub enum CanaryStatus {
    Pending,
    Active,
    Paused,
    Completed,
    RolledBack,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "rollout_stage", rename_all = "snake_case")]
pub enum RolloutStage {
    Stage1,
    Stage2,
    Stage3,
    Stage4,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct CanaryRelease {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub from_deployment_id: Option<Uuid>,
    pub to_deployment_id: Uuid,
    pub status: CanaryStatus,
    pub current_stage: RolloutStage,
    pub current_percentage: i32,
    pub target_percentage: i32,
    pub error_rate_threshold: Decimal,
    pub current_error_rate: Option<Decimal>,
    pub total_requests: i32,
    pub error_count: i32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct CanaryMetric {
    pub id: Uuid,
    pub canary_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub requests: i32,
    pub errors: i32,
    pub error_rate: rust_decimal::Decimal,
    pub avg_response_time_ms: Option<Decimal>,
    pub p95_response_time_ms: Option<Decimal>,
    pub p99_response_time_ms: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct CanaryUserAssignment {
    pub id: Uuid,
    pub canary_id: Uuid,
    pub user_address: String,
    pub assigned_at: DateTime<Utc>,
    pub notified: bool,
    pub notified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateCanaryRequest {
    pub contract_id: String,
    pub to_deployment_id: String,
    pub error_rate_threshold: Option<f64>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AdvanceCanaryRequest {
    pub canary_id: String,
    pub target_percentage: Option<i32>,
    pub advanced_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecordCanaryMetricRequest {
    pub canary_id: String,
    pub requests: i32,
    pub errors: i32,
    pub avg_response_time_ms: Option<f64>,
    pub p95_response_time_ms: Option<f64>,
    pub p99_response_time_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "ab_test_status", rename_all = "snake_case")]
pub enum AbTestStatus {
    Draft,
    Running,
    Paused,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "variant_type", rename_all = "snake_case")]
pub enum VariantType {
    Control,
    Treatment,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct AbTest {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: AbTestStatus,
    pub traffic_split: Decimal,
    pub variant_a_deployment_id: Uuid,
    pub variant_b_deployment_id: Uuid,
    pub primary_metric: String,
    pub hypothesis: Option<String>,
    pub significance_threshold: Decimal,
    pub min_sample_size: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct AbTestVariant {
    pub id: Uuid,
    pub test_id: Uuid,
    pub variant_type: VariantType,
    pub deployment_id: Uuid,
    pub traffic_percentage: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct AbTestAssignment {
    pub id: Uuid,
    pub test_id: Uuid,
    pub user_address: String,
    pub variant_type: VariantType,
    pub assigned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct AbTestMetric {
    pub id: Uuid,
    pub test_id: Uuid,
    pub variant_type: VariantType,
    pub metric_name: String,
    pub metric_value: Decimal,
    pub user_address: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct AbTestResult {
    pub id: Uuid,
    pub test_id: Uuid,
    pub variant_type: VariantType,
    pub sample_size: i32,
    pub mean_value: Option<Decimal>,
    pub std_deviation: Option<Decimal>,
    pub confidence_interval_lower: Option<Decimal>,
    pub confidence_interval_upper: Option<Decimal>,
    pub p_value: Option<Decimal>,
    pub statistical_significance: Option<Decimal>,
    pub is_winner: bool,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateAbTestRequest {
    pub contract_id: String,
    pub name: String,
    pub description: Option<String>,
    pub traffic_split: Option<f64>,
    pub variant_a_deployment_id: String,
    pub variant_b_deployment_id: String,
    pub primary_metric: String,
    pub hypothesis: Option<String>,
    pub significance_threshold: Option<f64>,
    pub min_sample_size: Option<i32>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecordAbTestMetricRequest {
    pub test_id: String,
    pub user_address: Option<String>,
    pub metric_name: String,
    pub metric_value: f64,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GetVariantRequest {
    pub test_id: String,
    pub user_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "metric_type", rename_all = "snake_case")]
pub enum MetricType {
    ExecutionTime,
    MemoryUsage,
    StorageIo,
    GasConsumption,
    ErrorRate,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "alert_severity", rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct PerformanceMetric {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub metric_type: MetricType,
    pub function_name: Option<String>,
    pub value: Decimal,
    pub p50: Option<Decimal>,
    pub p95: Option<Decimal>,
    pub p99: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct PerformanceAnomaly {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub metric_type: MetricType,
    pub function_name: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub baseline_value: Option<Decimal>,
    pub current_value: Option<Decimal>,
    pub deviation_percent: Option<Decimal>,
    pub severity: AlertSeverity,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct PerformanceAlert {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub metric_type: MetricType,
    pub threshold_type: String,
    pub threshold_value: Decimal,
    pub current_value: Decimal,
    pub severity: AlertSeverity,
    pub triggered_at: DateTime<Utc>,
    pub acknowledged: bool,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<String>,
    pub resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct PerformanceTrend {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub function_name: Option<String>,
    pub metric_type: MetricType,
    pub timeframe_start: DateTime<Utc>,
    pub timeframe_end: DateTime<Utc>,
    pub avg_value: Option<Decimal>,
    pub min_value: Option<Decimal>,
    pub max_value: Option<Decimal>,
    pub p50_value: Option<Decimal>,
    pub p95_value: Option<Decimal>,
    pub p99_value: Option<Decimal>,
    pub sample_count: i32,
    pub trend_direction: Option<String>,
    pub change_percent: Option<Decimal>,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct PerformanceAlertConfig {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub metric_type: MetricType,
    pub threshold_type: String,
    pub threshold_value: Decimal,
    pub severity: AlertSeverity,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecordPerformanceMetricRequest {
    pub contract_id: String,
    pub metric_type: MetricType,
    pub function_name: Option<String>,
    pub value: f64,
    pub p50: Option<f64>,
    pub p95: Option<f64>,
    pub p99: Option<f64>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecordPerformanceBenchmarkRequest {
    pub contract_id: Option<String>,
    pub contract_version_id: Option<String>,
    pub benchmark_name: String,
    pub execution_time_ms: f64,
    pub gas_used: i64,
    pub sample_size: Option<i32>,
    pub source: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PerformanceBenchmark {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub contract_version_id: Option<Uuid>,
    pub version: Option<String>,
    pub benchmark_name: String,
    pub execution_time_ms: Decimal,
    pub gas_used: i64,
    pub sample_size: i32,
    pub source: String,
    pub recorded_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PerformanceMetricSnapshot {
    pub metric_type: String,
    pub benchmark_name: Option<String>,
    pub latest_value: Decimal,
    pub previous_value: Option<Decimal>,
    pub change_percent: Option<Decimal>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PerformanceTrendPoint {
    pub bucket_start: DateTime<Utc>,
    pub bucket_end: DateTime<Utc>,
    pub benchmark_name: String,
    pub avg_execution_time_ms: Decimal,
    pub avg_gas_used: Decimal,
    pub sample_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PerformanceRegression {
    pub benchmark_name: String,
    pub current_version: Option<String>,
    pub previous_version: Option<String>,
    pub execution_time_regression_percent: Option<Decimal>,
    pub gas_regression_percent: Option<Decimal>,
    pub severity: String,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PerformanceComparisonEntry {
    pub contract_id: Uuid,
    pub contract_name: String,
    pub category: Option<String>,
    pub benchmark_name: String,
    pub avg_execution_time_ms: Decimal,
    pub avg_gas_used: Decimal,
    pub sample_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractPerformanceSummaryResponse {
    pub contract_id: Uuid,
    pub latest_benchmarks: Vec<PerformanceBenchmark>,
    pub metric_snapshots: Vec<PerformanceMetricSnapshot>,
    pub trends: Vec<PerformanceTrendPoint>,
    pub regressions: Vec<PerformanceRegression>,
    pub comparisons: Vec<PerformanceComparisonEntry>,
    pub unresolved_alerts: Vec<PerformanceAlert>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateAlertConfigRequest {
    pub contract_id: String,
    pub metric_type: MetricType,
    pub threshold_type: String,
    pub threshold_value: f64,
    pub severity: Option<AlertSeverity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "similarity_match_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SimilarityMatchType {
    ExactClone,
    NearDuplicate,
    Similar,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "similarity_review_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SimilarityReviewStatus {
    None,
    Pending,
    Reviewed,
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractSimilaritySignature {
    pub contract_id: Uuid,
    pub representation_type: String,
    pub exact_hash: String,
    pub simhash: i64,
    pub token_count: i32,
    pub source_length: i32,
    pub wasm_hash: String,
    pub computed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractSimilarityReport {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub similar_contract_id: Uuid,
    pub similarity_score: Decimal,
    pub exact_clone: bool,
    pub match_type: SimilarityMatchType,
    pub suspicious: bool,
    pub flagged_for_review: bool,
    pub review_status: SimilarityReviewStatus,
    pub reasons: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractSimilarityResult {
    pub contract_id: Uuid,
    pub similar_contract_id: Uuid,
    pub similar_contract_name: String,
    pub similar_contract_address: String,
    pub similarity_score: f64,
    pub exact_clone: bool,
    pub match_type: SimilarityMatchType,
    pub suspicious: bool,
    pub flagged_for_review: bool,
    pub review_status: SimilarityReviewStatus,
    pub reasons: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractSimilarityResponse {
    pub contract_id: Uuid,
    pub total_matches: usize,
    pub suspicious_matches: usize,
    pub items: Vec<ContractSimilarityResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchSimilarityAnalysisRequest {
    #[serde(default)]
    pub contract_ids: Vec<String>,
    pub limit_per_contract: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchSimilarityAnalysisItem {
    pub contract_id: Uuid,
    pub analyzed_contracts: usize,
    pub suspicious_matches: usize,
    pub items: Vec<ContractSimilarityResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchSimilarityAnalysisResponse {
    pub analyzed_contracts: usize,
    pub total_flagged_for_review: usize,
    pub items: Vec<BatchSimilarityAnalysisItem>,
}

// ────────────────────────────────────────────────────────────────────────────
// Custom contract metrics (issue #89)
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "custom_metric_type", rename_all = "snake_case")]
pub enum CustomMetricType {
    Counter,
    Gauge,
    Histogram,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct CustomMetric {
    pub id: Uuid,
    pub contract_id: String,
    pub metric_name: String,
    pub metric_type: CustomMetricType,
    pub value: Decimal,
    pub unit: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub ledger_sequence: Option<i64>,
    pub transaction_hash: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub network: Network,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecordCustomMetricRequest {
    pub contract_id: String,
    pub metric_name: String,
    pub metric_type: CustomMetricType,
    pub value: f64,
    pub unit: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub ledger_sequence: Option<i64>,
    pub transaction_hash: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub network: Option<Network>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct CustomMetricAggregate {
    pub contract_id: String,
    pub metric_name: String,
    pub metric_type: CustomMetricType,
    pub bucket_start: DateTime<Utc>,
    pub bucket_end: DateTime<Utc>,
    pub sample_count: i32,
    pub sum_value: Option<Decimal>,
    pub avg_value: Option<Decimal>,
    pub min_value: Option<Decimal>,
    pub max_value: Option<Decimal>,
    pub p50_value: Option<Decimal>,
    pub p95_value: Option<Decimal>,
    pub p99_value: Option<Decimal>,
}

// ────────────────────────────────────────────────────────────────────────────
// Analytics models
// ────────────────────────────────────────────────────────────────────────────

/// Types of analytics events tracked by the system
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, utoipa::ToSchema)]
#[sqlx(type_name = "analytics_event_type", rename_all = "snake_case")]
pub enum AnalyticsEventType {
    ContractPublished,
    ContractVerified,
    ContractDeployed,
    VersionCreated,
    ContractUpdated,
    PublisherCreated,
    SearchClick,
}

impl std::fmt::Display for AnalyticsEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContractPublished => write!(f, "contract_published"),
            Self::ContractVerified => write!(f, "contract_verified"),
            Self::ContractDeployed => write!(f, "contract_deployed"),
            Self::VersionCreated => write!(f, "version_created"),
            Self::ContractUpdated => write!(f, "contract_updated"),
            Self::PublisherCreated => write!(f, "publisher_created"),
            Self::SearchClick => write!(f, "search_click"),
        }
    }
}

/// A simplified entry for the activity feed API
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActivityFeedEntry {
    pub id: Uuid,
    pub event_type: AnalyticsEventType,
    pub contract_id: Option<Uuid>,
    pub contract_name: Option<String>,
    pub contract_stellar_id: Option<String>,
    pub publisher_id: Option<Uuid>,
    pub publisher_name: Option<String>,
    pub network: Option<Network>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// A raw analytics event recorded when a contract lifecycle action occurs
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct AnalyticsEvent {
    pub id: Uuid,
    pub event_type: AnalyticsEventType,
    pub contract_id: Uuid,
    pub user_address: Option<String>,
    pub network: Option<Network>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Pre-computed daily aggregate for a single contract
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct DailyAggregate {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub date: chrono::NaiveDate,
    pub deployment_count: i32,
    pub unique_deployers: i32,
    pub verification_count: i32,
    pub publish_count: i32,
    pub version_count: i32,
    pub update_count: i32,
    pub total_events: i32,
    pub unique_users: i32,
    pub network_breakdown: serde_json::Value,
    pub top_users: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ────────────────────────────────────────────────────────────────────────────
// Analytics API response DTOs
// ────────────────────────────────────────────────────────────────────────────

/// Top-level response for GET /api/contracts/:id/analytics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractAnalyticsResponse {
    pub contract_id: Uuid,
    pub deployments: DeploymentStats,
    pub interactors: InteractorStats,
    pub timeline: Vec<TimelineEntry>,
}

/// Deployment statistics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DeploymentStats {
    pub count: i64,
    pub unique_users: i64,
    pub by_network: serde_json::Value,
}

/// Interactor / unique-user statistics
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InteractorStats {
    pub unique_count: i64,
    pub top_users: Vec<TopUser>,
}

/// A user ranked by interaction count
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TopUser {
    pub address: String,
    pub count: i64,
}

/// One data-point in the 30-day timeline
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TimelineEntry {
    pub date: chrono::NaiveDate,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DeployGreenRequest {
    pub contract_id: String,
    pub wasm_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SwitchDeploymentRequest {
    pub contract_id: String,
    pub force: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HealthCheckRequest {
    pub contract_id: String,
    pub environment: DeploymentEnvironment,
    pub passed: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// POPULARITY / TRENDING
// ═══════════════════════════════════════════════════════════════════════════

/// Query parameters for the trending contracts endpoint
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct TrendingParams {
    /// Max results to return (default 10, max 50)
    pub limit: Option<i64>,
    /// Timeframe for trending calculation: "7d", "30d", "90d" (default "7d")
    pub timeframe: Option<String>,
}

/// Response DTO for a trending contract
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct TrendingContract {
    // Core contract fields
    pub id: Uuid,
    pub contract_id: String,
    pub name: String,
    pub description: Option<String>,
    pub network: Network,
    pub is_verified: bool,
    pub category: Option<String>,
    #[sqlx(skip)]
    #[serde(default)]
    pub tags: Vec<Tag>,
    pub created_at: DateTime<Utc>,
    // Popularity metrics
    pub popularity_score: f64,
    pub deployment_count: i64,
    pub interaction_count: i64,
}

// MULTI-SIGNATURE DEPLOYMENT TYPES  (issue #47)
// ═══════════════════════════════════════════════════════════════════════════
// ════════════════════════════════════════════════════════════════════════════
// Audit Log & Version History types
// ════════════════════════════════════════════════════════════════════════════

/// The type of mutation that triggered an audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "audit_action_type", rename_all = "snake_case")]
pub enum AuditActionType {
    ContractPublished,
    MetadataUpdated,
    VerificationChanged,
    PublisherChanged,
    VersionCreated,
    Rollback,
}

impl std::fmt::Display for AuditActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::ContractPublished => "contract_published",
            Self::MetadataUpdated => "metadata_updated",
            Self::VerificationChanged => "verification_changed",
            Self::PublisherChanged => "publisher_changed",
            Self::VersionCreated => "version_created",
            Self::Rollback => "rollback",
        };
        write!(f, "{}", s)
    }
}

/// One immutable row in `contract_audit_log`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractAuditLog {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub action_type: AuditActionType,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub changed_by: String,
    pub timestamp: DateTime<Utc>,
    pub previous_hash: Option<String>,
    pub hash: Option<String>,
    pub signature: Option<String>,
}

/// Full contract state captured at each audited change in `contract_snapshots`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractSnapshot {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version_number: i32,
    pub snapshot_data: serde_json::Value,
    pub audit_log_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// A single field-level change between two snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FieldChange {
    pub field: String,
    pub from: serde_json::Value,
    pub to: serde_json::Value,
}

/// Response for GET /api/contracts/:id/versions/:v1/diff/:v2
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct VersionDiff {
    pub contract_id: Uuid,
    pub from_version: i32,
    pub to_version: i32,
    /// Fields present in v2 but not v1
    pub added: Vec<FieldChange>,
    /// Fields present in v1 but not v2
    pub removed: Vec<FieldChange>,
    /// Fields present in both but with different values
    pub modified: Vec<FieldChange>,
}

/// Request body for POST /api/contracts/:id/rollback/:snapshot_id
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RollbackRequest {
    /// Stellar address (or admin service ID) authorising the rollback
    pub changed_by: String,
}

// Multisig deployment types
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct MultisigPolicy {
    pub id: Uuid,
    pub name: String,
    pub threshold: i32,
    pub required_signatures: i32,
    pub signer_addresses: Vec<String>,
    pub expiry_seconds: i32,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct DeployProposal {
    pub id: Uuid,
    pub contract_name: String,
    pub contract_id: Uuid,
    pub wasm_hash: String,
    pub network: String,
    pub description: Option<String>,
    pub policy_id: Uuid,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub proposer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ProposalSignature {
    pub id: Uuid,
    pub proposal_id: Uuid,
    pub signer_address: String,
}

/// Paginated response for audit log
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ProposalWithSignatures {
    pub proposal: DeployProposal,
    pub policy: MultisigPolicy,
    pub signatures: Vec<ProposalSignature>,
    pub signatures_needed: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AuditLogPage {
    pub items: Vec<ContractAuditLog>,
    pub total: i64,
    pub page: i64,
    pub total_pages: i64,
}

// ────────────────────────────────────────────────────────────────────────────
// Cursor-based pagination  (issue #337)
// Used by activity-feed and any future cursor-paginated endpoints.
// PaginatedResponse (offset-based) above is left completely untouched.
// ────────────────────────────────────────────────────────────────────────────

/// Query parameters for the activity feed endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityFeedParams {
    /// ISO-8601 timestamp. Only events older than this are returned.
    /// Omit on the first request; use `next_cursor` from the previous
    /// response on subsequent requests.
    pub cursor: Option<DateTime<Utc>>,

    /// How many events to return (default 20, max 100).
    #[serde(default = "default_activity_limit")]
    pub limit: i64,

    /// Optionally filter by event type.
    pub event_type: Option<AnalyticsEventType>,

    /// Optionally filter by contract ID.
    pub contract_id: Option<Uuid>,
}

fn default_activity_limit() -> i64 {
    20
}

/// Response for cursor-paginated endpoints.
/// `next_cursor` is `None` when `has_more` is false (last page).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    /// Real total matching the applied filters — from COUNT(*).
    pub total: i64,
    /// True when more results exist beyond this page.
    pub has_more: bool,
    /// Pass this value as `cursor` to fetch the next page.
    /// `None` on the last page.
    pub next_cursor: Option<DateTime<Utc>>,
}

impl<T: Serialize> CursorPaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, limit: i64, next_cursor: Option<DateTime<Utc>>) -> Self {
        let has_more = data.len() as i64 == limit;
        Self {
            has_more,
            // Only emit a cursor when there really are more pages.
            next_cursor: if has_more { next_cursor } else { None },
            data,
            total,
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Config Management types
// ════════════════════════════════════════════════════════════════════════════

/// Represents a contract configuration version in the registry
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractConfig {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub environment: String,
    pub version: i32,
    pub config_data: serde_json::Value,
    pub secrets_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

/// Request to create a new configuration version
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ConfigCreateRequest {
    pub environment: String,
    pub config_data: serde_json::Value,
    pub secrets_data: Option<serde_json::Value>,
    pub created_by: String,
}

/// Request to rollback to an old configuration version
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ConfigRollbackRequest {
    pub roll_back_to_version: i32,
    pub created_by: String,
}

/// Response object for returning configurations (without secrets_data when returning publicly)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractConfigResponse {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub environment: String,
    pub version: i32,
    pub config_data: serde_json::Value,
    pub has_secrets: bool, // Indicator instead of returning actual secrets
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

impl From<ContractConfig> for ContractConfigResponse {
    fn from(config: ContractConfig) -> Self {
        Self {
            id: config.id,
            contract_id: config.contract_id,
            environment: config.environment,
            version: config.version,
            config_data: config.config_data,
            has_secrets: config.secrets_data.is_some(),
            created_at: config.created_at,
            created_by: config.created_by,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DATA RESIDENCY CONTROLS  (issue #100)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "residency_decision", rename_all = "lowercase")]
pub enum ResidencyDecision {
    Allowed,
    Denied,
}

impl std::fmt::Display for ResidencyDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allowed => write!(f, "allowed"),
            Self::Denied => write!(f, "denied"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ResidencyPolicy {
    pub id: Uuid,
    pub contract_id: String,
    pub allowed_regions: Vec<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ResidencyAuditLog {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub contract_id: String,
    pub requested_region: String,
    pub decision: ResidencyDecision,
    pub action: String,
    pub requested_by: Option<String>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ResidencyViolation {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub contract_id: String,
    pub attempted_region: String,
    pub action: String,
    pub attempted_by: Option<String>,
    pub prevented_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateResidencyPolicyRequest {
    pub contract_id: String,
    pub allowed_regions: Vec<String>,
    pub description: Option<String>,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResidencyPolicyRequest {
    pub allowed_regions: Option<Vec<String>>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResidencyRequest {
    pub policy_id: Uuid,
    pub contract_id: String,
    pub requested_region: String,
    pub action: String,
    pub requested_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResidencyLogsParams {
    pub contract_id: Option<String>,
    pub limit: Option<i64>,
    pub page: Option<i64>,
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT EVENT TYPES (issue #44)
// ═══════════════════════════════════════════════════════════════════════════

/// A contract event emitted during execution
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractEvent {
    pub id: Uuid,
    pub contract_id: String,
    pub topic: String,
    pub data: Option<serde_json::Value>,
    pub ledger_sequence: i64,
    pub transaction_hash: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub network: Network,
    pub created_at: DateTime<Utc>,
}

/// Query parameters for searching events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventQueryParams {
    pub topic: Option<String>,
    pub data_pattern: Option<String>,
    pub from_timestamp: Option<DateTime<Utc>>,
    pub to_timestamp: Option<DateTime<Utc>>,
    pub from_ledger: Option<i64>,
    pub to_ledger: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Request to index a new event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEventRequest {
    pub contract_id: String,
    pub topic: String,
    pub data: Option<serde_json::Value>,
    pub ledger_sequence: i64,
    pub transaction_hash: Option<String>,
    pub network: Network,
}

/// Event statistics for a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStats {
    pub contract_id: String,
    pub total_events: i64,
    pub unique_topics: i64,
    pub first_event: Option<DateTime<Utc>>,
    pub last_event: Option<DateTime<Utc>>,
    pub events_by_topic: serde_json::Value,
}

/// CSV export response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventExport {
    pub contract_id: String,
    pub events: Vec<ContractEvent>,
    pub exported_at: DateTime<Utc>,
    pub total_count: i64,
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT PACKAGE SIGNING (Issue #67)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "signature_status", rename_all = "lowercase")]
pub enum SignatureStatus {
    Valid,
    Revoked,
    Expired,
}

impl std::fmt::Display for SignatureStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Valid => write!(f, "valid"),
            Self::Revoked => write!(f, "revoked"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "transparency_entry_type", rename_all = "snake_case")]
pub enum TransparencyEntryType {
    PackageSigned,
    SignatureVerified,
    SignatureRevoked,
    KeyRotated,
}

impl std::fmt::Display for TransparencyEntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PackageSigned => write!(f, "package_signed"),
            Self::SignatureVerified => write!(f, "signature_verified"),
            Self::SignatureRevoked => write!(f, "signature_revoked"),
            Self::KeyRotated => write!(f, "key_rotated"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PackageSignature {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub wasm_hash: String,
    pub signature: String,
    pub signing_address: String,
    pub public_key: String,
    pub algorithm: String,
    pub status: SignatureStatus,
    pub signed_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_reason: Option<String>,
    pub revoked_by: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignPackageRequest {
    pub contract_id: String,
    pub version: String,
    pub wasm_hash: String,
    pub private_key: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifySignatureRequest {
    pub contract_id: String,
    pub version: String,
    pub wasm_hash: String,
    pub signature: String,
    pub signing_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifySignatureResponse {
    pub valid: bool,
    pub signature_id: Option<Uuid>,
    pub signing_address: String,
    pub signed_at: Option<DateTime<Utc>>,
    pub status: SignatureStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeSignatureRequest {
    pub signature_id: String,
    pub revoked_by: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SignatureRevocation {
    pub id: Uuid,
    pub signature_id: Uuid,
    pub revoked_by: String,
    pub reason: String,
    pub revoked_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SigningKey {
    pub id: Uuid,
    pub publisher_id: Uuid,
    pub public_key: String,
    pub key_fingerprint: String,
    pub algorithm: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub deactivated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSigningKeyRequest {
    pub publisher_id: String,
    pub public_key: String,
    pub algorithm: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TransparencyLogEntry {
    pub id: Uuid,
    pub entry_type: TransparencyEntryType,
    pub contract_id: Option<Uuid>,
    pub signature_id: Option<Uuid>,
    pub actor_address: String,
    pub previous_hash: Option<String>,
    pub entry_hash: String,
    pub payload: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub immutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainOfCustodyEntry {
    pub action: String,
    pub actor: String,
    pub timestamp: DateTime<Utc>,
    pub signature_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainOfCustodyResponse {
    pub contract_id: String,
    pub entries: Vec<ChainOfCustodyEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransparencyLogQueryParams {
    pub contract_id: Option<String>,
    pub entry_type: Option<TransparencyEntryType>,
    pub actor_address: Option<String>,
    pub from_timestamp: Option<DateTime<Utc>>,
    pub to_timestamp: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for HealthStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "healthy" => Ok(Self::Healthy),
            "warning" => Ok(Self::Warning),
            "critical" => Ok(Self::Critical),
            other => Err(format!("unknown health status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractHealth {
    pub contract_id: Uuid,
    pub status: String,
    pub last_activity: DateTime<Utc>,
    pub security_score: i32,
    pub audit_date: Option<DateTime<Utc>>,
    pub total_score: i32,
    pub recommendations: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

impl ContractHealth {
    /// Returns the typed `HealthStatus` from the stored string.
    pub fn health_status(&self) -> Result<HealthStatus, String> {
        self.status.parse()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT RECOMMENDATIONS (Issue #492)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecommendationReason {
    /// Stable machine-readable reason code (e.g. same_category, similar_functionality)
    pub code: String,
    /// Human-readable reason explanation
    pub message: String,
    /// Relative contribution weight for this reason in the final score
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecommendedContract {
    pub id: Uuid,
    pub contract_id: String,
    pub name: String,
    pub description: Option<String>,
    pub network: Network,
    pub category: Option<String>,
    pub popularity_score: i64,
    pub similarity_score: f64,
    pub recommendation_score: f64,
    pub reasons: Vec<RecommendationReason>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractRecommendationsResponse {
    pub contract_id: Uuid,
    pub algorithm: String,
    pub ab_variant: String,
    pub cached: bool,
    pub generated_at: DateTime<Utc>,
    pub recommendations: Vec<RecommendedContract>,
}

// ═══════════════════════════════════════════════════════════════════════════
// COLLABORATIVE REVIEWS (Issue #502)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, utoipa::ToSchema)]
#[sqlx(type_name = "collaborative_review_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CollaborativeReviewStatus {
    Pending,
    Approved,
    ChangesRequested,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct CollaborativeReview {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub status: CollaborativeReviewStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct CollaborativeReviewer {
    pub id: Uuid,
    pub review_id: Uuid,
    pub user_id: Uuid,
    pub status: CollaborativeReviewStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct CollaborativeComment {
    pub id: Uuid,
    pub review_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub line_number: Option<i32>,
    pub file_path: Option<String>,
    pub abi_path: Option<String>,
    pub parent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateCollaborativeReviewRequest {
    pub contract_id: Uuid,
    pub version: String,
    pub reviewer_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AddCollaborativeCommentRequest {
    pub content: String,
    pub line_number: Option<i32>,
    pub file_path: Option<String>,
    pub abi_path: Option<String>,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateReviewerStatusRequest {
    pub status: CollaborativeReviewStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CollaborativeReviewDetails {
    pub review: CollaborativeReview,
    pub reviewers: Vec<CollaborativeReviewer>,
    pub comments: Vec<CollaborativeComment>,
}

// ═══════════════════════════════════════════════════════════════════════════
// ADVANCED CONTRACT DEPENDENCIES (issue #417)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyNode {
    pub contract_id: String,
    pub resolved_id: Option<Uuid>,
    pub name: Option<String>,
    pub call_volume: i32,
    pub status: String,
    pub is_circular: bool,
    pub dependencies: Vec<DependencyNode>,
    pub visualization_hints: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResponse {
    pub root: DependencyNode,
    pub total_dependencies: usize,
    pub max_depth: usize,
    pub has_circular: bool,
}

// Backup and disaster recovery types
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractBackup {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub backup_date: chrono::NaiveDate,
    pub wasm_hash: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub state_snapshot: Option<serde_json::Value>,
    pub storage_size_bytes: Option<i64>,
    pub primary_region: Option<String>,
    pub backup_regions: Option<Vec<String>>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackupRestoration {
    pub id: Uuid,
    pub backup_id: Uuid,
    pub restored_by: Uuid,
    pub restore_duration_ms: i32,
    pub success: bool,
    pub restored_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBackupRequest {
    pub include_state: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreBackupRequest {
    pub backup_date: String,
}

/// AUTOMATED REALEASE NOTE GENERATOR
/// Status of auto-generated release notes (draft allows editing before publish)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "release_notes_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ReleaseNotesStatus {
    Draft,
    Published,
}

impl std::fmt::Display for ReleaseNotesStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReleaseNotesStatus::Draft => write!(f, "draft"),
            ReleaseNotesStatus::Published => write!(f, "published"),
        }
    }
}

/// A detected function change in a code diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionChange {
    pub name: String,
    pub change_type: String,
    pub old_signature: Option<String>,
    pub new_signature: Option<String>,
    pub is_breaking: bool,
}

/// Summary of a code diff between two contract versions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiffSummary {
    pub files_changed: i32,
    pub lines_added: i32,
    pub lines_removed: i32,
    pub function_changes: Vec<FunctionChange>,
    pub has_breaking_changes: bool,
    pub features_count: i32,
    pub fixes_count: i32,
    pub breaking_count: i32,
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT DEPLOYMENT SIMULATION (Issue #256)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateDeployRequest {
    pub wasm_binary: String,
    pub contract_id: String,
    pub name: String,
    pub description: Option<String>,
    pub network: Network,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub publisher_address: String,
    #[serde(default)]
    pub dependencies: Vec<DependencyDeclaration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub valid: bool,
    pub errors: Vec<SimulationError>,
    pub warnings: Vec<SimulationWarning>,
    pub gas_estimate: GasEstimate,
    pub performance_metrics: PerformanceMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abi_preview: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_functions: Option<Vec<ContractFunctionInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationWarning {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimate {
    pub total_cost_stroops: i64,
    pub total_cost_xlm: f64,
    pub wasm_size_kb: f64,
    pub complexity_factor: f64,
    pub deployment_cost_stroops: i64,
    pub storage_cost_stroops: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub estimated_execution_time_ms: u64,
    pub memory_estimate_kb: u64,
    pub function_count: u32,
    pub table_size_bytes: u32,
    pub data_section_bytes: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractFunctionInfo {
    pub name: String,
    pub param_count: u32,
    pub return_type: Option<String>,
    pub is_view: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// GAS USAGE ESTIMATION TYPES (Issue #496)
// ═══════════════════════════════════════════════════════════════════════════

/// Confidence level of a gas estimate based on available historical data.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GasEstimateConfidence {
    /// Derived from ≥10 real invocations.
    High,
    /// Derived from 1–9 real invocations.
    Medium,
    /// No historical data; purely heuristic.
    Low,
}

/// Gas estimate for a single contract method.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MethodGasEstimate {
    /// Name of the contract method.
    pub method_name: String,
    /// Lower bound of expected gas cost in stroops.
    pub min_gas_stroops: i64,
    /// Upper bound of expected gas cost in stroops.
    pub max_gas_stroops: i64,
    /// Average (or heuristic mid-point) gas cost in stroops.
    pub avg_gas_stroops: i64,
    /// Convenience field: average cost converted to XLM.
    pub avg_gas_xlm: f64,
    /// How confident the estimate is based on available samples.
    pub confidence: GasEstimateConfidence,
    /// Number of real invocations the estimate is based on (0 = heuristic only).
    pub sample_count: i64,
    /// Whether historical data was used for this estimate.
    pub from_history: bool,
    /// Unix timestamp (seconds) when the underlying data was last refreshed.
    pub last_updated: Option<DateTime<Utc>>,
}

/// Optional method-level parameters that make estimates more accurate.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct MethodParamHint {
    /// Parameter name (as declared in the ABI).
    pub name: String,
    /// JSON-serialisable value used only for heuristic sizing.
    pub value: serde_json::Value,
}

/// Query parameters for `GET /api/contracts/:id/methods/:method/gas-estimate`.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::IntoParams)]
pub struct GasEstimateQuery {
    /// Optional JSON-encoded array of `MethodParamHint` for a more realistic estimate.
    #[serde(default)]
    pub params: Option<String>,
}

/// Request body for batch gas estimation.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchGasEstimateRequest {
    /// List of method names to estimate.  Must be non-empty (max 50).
    pub methods: Vec<BatchMethodEntry>,
}

/// One entry in a batch gas-estimate request.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchMethodEntry {
    /// Name of the method to estimate.
    pub method_name: String,
    /// Optional parameter hints for this method.
    #[serde(default)]
    pub params: Vec<MethodParamHint>,
}

/// Response for batch gas estimation.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct BatchGasEstimateResponse {
    /// Estimates in the same order as the request.
    pub estimates: Vec<MethodGasEstimate>,
    /// Methods that were requested but could not be estimated (e.g., not in ABI).
    pub not_found: Vec<String>,
}

// ────────────────────────────────────────────────────────────────────────────
// Contract changelog (release history)
// ────────────────────────────────────────────────────────────────────────────

// ═══════════════════════════════════════════════════════════════════════════
// ANALYTICS DASHBOARD (issue #430)
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct NetworkCount {
    pub network: Network,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct DeploymentTrend {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DashboardAnalyticsResponse {
    pub category_distribution: Vec<CategoryCount>,
    pub network_usage: Vec<NetworkCount>,
    pub deployment_trends: Vec<DeploymentTrend>,
    pub recent_additions: Vec<Contract>,
}

// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT REVIEW SYSTEM (Issue: Review System Implementation)
// ═══════════════════════════════════════════════════════════════════════════

/// Review status for moderation workflow
/// New reviews start as "pending" and must be approved before becoming visible
#[derive(
    Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq, Default,
)]
#[sqlx(type_name = "review_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    /// Review is pending approval (default for new reviews)
    #[default]
    Pending,
    /// Review has been approved and is visible to users
    Approved,
    /// Review has been rejected and is hidden
    Rejected,
}

/// Sort options for fetching reviews
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReviewSortBy {
    /// Sort by helpful votes (highest first)
    MostHelpful,
    /// Sort by creation date (newest first)
    MostRecent,
    /// Sort by rating (highest first)
    HighestRated,
    /// Sort by rating (lowest first)
    LowestRated,
}

/// Request to create a new contract review
/// Rating must be between 1.0 and 5.0 (inclusive)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateReviewRequest {
    /// Rating from 1.0 to 5.0 (one decimal place allowed)
    #[schema(example = 4.5, minimum = 1.0, maximum = 5.0)]
    pub rating: f64,
    /// Optional review text
    #[schema(example = "Great contract! Very well optimized and easy to integrate.")]
    pub review_text: Option<String>,
    /// Contract version being reviewed (optional)
    #[schema(example = "1.0.0")]
    pub version: Option<String>,
}

/// Review response with full details
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ReviewResponse {
    pub id: i32,
    pub contract_id: Uuid,
    pub user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[schema(example = 4.5, minimum = 1.0, maximum = 5.0)]
    pub rating: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_text: Option<String>,
    #[serde(default)]
    pub helpful_count: i32,
    #[serde(default)]
    pub is_flagged: bool,
    #[serde(default)]
    pub status: ReviewStatus,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

/// Aggregated rating statistics for a contract
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractRatingStats {
    /// Average rating (0.0 if no reviews)
    #[schema(example = 4.3)]
    pub average_rating: f64,
    /// Total number of approved reviews
    #[schema(example = 42)]
    pub total_reviews: i64,
    /// Distribution of ratings (1-5 stars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating_distribution: Option<RatingDistribution>,
}

/// Distribution of ratings by star count
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RatingDistribution {
    /// Number of 1-star reviews
    #[schema(example = 2)]
    pub stars_1: i64,
    /// Number of 2-star reviews
    #[schema(example = 3)]
    pub stars_2: i64,
    /// Number of 3-star reviews
    #[schema(example = 5)]
    pub stars_3: i64,
    /// Number of 4-star reviews
    #[schema(example = 12)]
    pub stars_4: i64,
    /// Number of 5-star reviews
    #[schema(example = 20)]
    pub stars_5: i64,
}

/// Request to vote on review helpfulness
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReviewVoteRequest {
    /// true = helpful, false = unhelpful
    pub helpful: bool,
}

/// Request to flag a review for moderation
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FlagReviewRequest {
    /// Reason for flagging the review
    #[schema(example = "Spam or misleading content")]
    pub reason: String,
}

/// Request to approve or reject a review (moderation endpoint)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ModerateReviewRequest {
    /// Action to take: "approve" or "reject"
    #[schema(example = "approve")]
    pub action: String,
}

/// Query parameters for GET /contracts/:id/reviews
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::IntoParams)]
pub struct GetReviewsQuery {
    /// Sort order: most_helpful, most_recent, highest_rated, lowest_rated
    #[serde(default = "default_review_sort")]
    pub sort_by: ReviewSortBy,
    /// Maximum number of reviews to return (default: 20, max: 100)
    #[serde(default = "default_review_limit")]
    pub limit: i64,
    /// Offset for pagination
    #[serde(default)]
    pub offset: i64,
}

fn default_review_sort() -> ReviewSortBy {
    ReviewSortBy::MostRecent
}

fn default_review_limit() -> i64 {
    20
}

/// Response for review voting endpoint
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReviewVoteResponse {
    pub review_id: i32,
    pub helpful_count: i32,
    pub vote_recorded: bool,
}

// ═══════════════════════════════════════════════════════════════════════════
// #487: Contract Clone/Mirror Types
// ═══════════════════════════════════════════════════════════════════════════

/// Request to clone an existing contract
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CloneContractRequest {
    /// New name for the cloned contract (optional, defaults to original name + " Clone")
    #[schema(example = "MyYieldOptimizer V2")]
    pub name: Option<String>,
    /// New description for the cloned contract (optional)
    #[schema(example = "A modified version of the original yield optimizer")]
    pub description: Option<String>,
    /// Target network for the clone (optional, defaults to original network)
    pub network: Option<Network>,
    /// New contract ID/address for the clone (required)
    #[schema(example = "C...5678")]
    pub contract_id: String,
    /// New wasm hash for the clone (optional, defaults to original)
    #[schema(example = "a1b2c3d4e5f6...")]
    pub wasm_hash: Option<String>,
    /// Override publisher (optional, defaults to current user)
    pub publisher_id: Option<Uuid>,
    /// Override category (optional)
    #[schema(example = "DeFi")]
    pub category: Option<String>,
    /// Override tags (optional)
    #[schema(example = json!(["yield", "fork", "optimized"]))]
    pub tags: Option<Vec<String>>,
}

/// Response from cloning a contract
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CloneContractResponse {
    /// ID of the newly created clone
    pub id: Uuid,
    /// Contract ID (address) of the clone
    pub contract_id: String,
    /// Name of the cloned contract
    pub name: String,
    /// Link to the original contract
    pub original_contract_id: Uuid,
    /// Original contract name
    pub original_contract_name: String,
    /// Clone link (API endpoint)
    pub clone_link: String,
    /// Network where the clone is deployed
    pub network: Network,
    /// Whether the clone inherited ABI from original
    pub inherited_abi: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Clone history record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ContractCloneHistory {
    pub id: Uuid,
    pub parent_contract_id: Uuid,
    pub cloned_contract_id: Uuid,
    pub cloned_by: Option<Uuid>,
    pub cloned_at: DateTime<Utc>,
    pub metadata_overrides: Option<serde_json::Value>,
    pub network: Network,
}

// ═══════════════════════════════════════════════════════════════════════════
// #499: Federated Registry Protocol Types
// ═══════════════════════════════════════════════════════════════════════════

/// Federation protocol version
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FederationProtocolVersion {
    pub version: String,
    pub supported_features: Vec<String>,
}

/// Federated registry information
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct FederatedRegistry {
    pub id: Uuid,
    pub name: String,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    pub is_active: bool,
    pub federation_protocol_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<DateTime<Utc>>,
    pub sync_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_error: Option<String>,
    pub contracts_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to register a new federated registry
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RegisterFederatedRegistryRequest {
    /// Name of the registry
    #[schema(example = "Stellar Community Registry")]
    pub name: String,
    /// Base URL of the registry API
    #[schema(example = "https://registry.example.com")]
    pub base_url: String,
    /// Public key for signature verification (optional)
    #[schema(example = "ed25519:base64encodedkey...")]
    pub public_key: Option<String>,
    /// Federation protocol version (defaults to "1.0")
    #[schema(example = "1.0")]
    pub federation_protocol_version: Option<String>,
}

/// Response from registering a federated registry
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FederatedRegistryResponse {
    pub id: Uuid,
    pub name: String,
    pub base_url: String,
    pub is_active: bool,
    pub federation_protocol_version: String,
    pub registration_link: String,
    pub created_at: DateTime<Utc>,
}

/// Federation sync job status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct FederationSyncJob {
    pub id: Uuid,
    pub registry_id: Uuid,
    pub status: String,
    pub contracts_synced: i32,
    pub contracts_failed: i32,
    pub duplicates_detected: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request to sync contracts from a federated registry
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SyncFederatedRegistryRequest {
    /// Registry ID to sync from
    pub registry_id: Uuid,
    /// Sync only new contracts (default: false)
    #[serde(default)]
    pub incremental: bool,
    /// Batch size for sync operations (default: 100)
    #[serde(default = "default_sync_batch_size")]
    pub batch_size: i32,
}

fn default_sync_batch_size() -> i32 {
    100
}

/// Response from federation sync operation
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FederationSyncResponse {
    pub job_id: Uuid,
    pub registry_id: Uuid,
    pub registry_name: String,
    pub status: String,
    pub contracts_synced: i32,
    pub contracts_failed: i32,
    pub duplicates_detected: i32,
    pub sync_link: String,
    pub started_at: Option<DateTime<Utc>>,
}

/// Individual sync result record
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct FederationSyncResult {
    pub id: Uuid,
    pub job_id: Uuid,
    pub source_registry_id: Uuid,
    pub source_contract_id: String,
    pub local_contract_id: Option<Uuid>,
    pub sync_action: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub synced_at: DateTime<Utc>,
}

/// Federation discovery response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FederationDiscoveryResponse {
    pub registries: Vec<FederatedRegistrySummary>,
    pub total_count: i64,
    pub discovered_at: DateTime<Utc>,
}

/// Summary of a federated registry for discovery
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FederatedRegistrySummary {
    pub id: Uuid,
    pub name: String,
    pub base_url: String,
    pub contracts_count: i32,
    pub protocol_version: String,
    pub is_active: bool,
}

/// Duplicate detection result
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct DuplicateDetectionResult {
    pub source_contract_id: String,
    pub source_registry: String,
    pub local_match: Option<ContractDuplicateMatch>,
    pub is_duplicate: bool,
    pub detection_method: String,
}

/// Matched duplicate contract info
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractDuplicateMatch {
    pub contract_id: Uuid,
    pub contract_address: String,
    pub name: String,
    pub match_confidence: f64,
    pub match_method: String,
}

/// Federation attribution info
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FederationAttribution {
    pub source_registry_id: Uuid,
    pub source_registry_name: String,
    pub original_contract_id: String,
    pub synced_at: DateTime<Utc>,
    pub attribution_link: String,
}

/// Request to opt-in/out of federation for a contract
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FederationOptRequest {
    /// Whether to allow this contract to be federated
    pub allow_federation: bool,
    /// Optional list of specific registries to allow/deny
    pub registry_filters: Option<Vec<Uuid>>,
}

/// Federation configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct FederationProtocolConfig {
    pub id: Uuid,
    pub config_key: String,
    pub config_value: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// List response for federated registries
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FederatedRegistryListResponse {
    pub registries: Vec<FederatedRegistry>,
    pub total_count: i64,
}

/// Sync history response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FederationSyncHistoryResponse {
    pub jobs: Vec<FederationSyncJob>,
    pub total_count: i64,
}

// ═══════════════════════════════════════════════════════════════════════════
// SECURITY SCANNING TYPES (#498)
// ═══════════════════════════════════════════════════════════════════════════

/// Security scanner configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct SecurityScanner {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub scanner_type: String,
    pub api_endpoint: Option<String>,
    pub is_active: bool,
    pub configuration: serde_json::Value,
    pub timeout_seconds: i32,
    pub max_concurrent_scans: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Security scan status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq)]
#[sqlx(type_name = "scan_status_type", rename_all = "snake_case")]
pub enum ScanStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Security issue severity
#[derive(
    Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq, PartialOrd,
)]
#[sqlx(type_name = "issue_severity_type", rename_all = "lowercase")]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// Security issue status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq)]
#[sqlx(type_name = "issue_status_type", rename_all = "snake_case")]
pub enum IssueStatus {
    Open,
    Acknowledged,
    Resolved,
    FalsePositive,
}

/// Security scan result
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct SecurityScan {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub contract_version_id: Option<Uuid>,
    pub scanner_id: Option<Uuid>,
    pub status: ScanStatus,
    pub scan_type: String,
    pub triggered_by: Option<Uuid>,
    pub triggered_by_event: Option<String>,
    pub total_issues: i32,
    pub critical_issues: i32,
    pub high_issues: i32,
    pub medium_issues: i32,
    pub low_issues: i32,
    pub scan_duration_ms: Option<i32>,
    pub scanner_version: Option<String>,
    pub scan_parameters: Option<serde_json::Value>,
    pub scan_result_raw: Option<serde_json::Value>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Security issue found during a scan
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct SecurityIssue {
    pub id: Uuid,
    pub scan_id: Uuid,
    pub contract_id: Uuid,
    pub contract_version_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub severity: IssueSeverity,
    pub status: IssueStatus,
    pub category: Option<String>,
    pub cwe_id: Option<String>,
    pub cve_id: Option<String>,
    pub source_file: Option<String>,
    pub source_line_start: Option<i32>,
    pub source_line_end: Option<i32>,
    pub function_name: Option<String>,
    pub code_snippet: Option<String>,
    pub remediation: Option<String>,
    pub remediation_code_example: Option<String>,
    pub references: Option<Vec<String>>,
    pub external_issue_id: Option<String>,
    pub is_false_positive: bool,
    pub false_positive_reason: Option<String>,
    pub resolved_by: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Security score history for version tracking
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct SecurityScoreHistory {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub contract_version_id: Uuid,
    pub overall_score: i32,
    pub score_breakdown: Option<serde_json::Value>,
    pub critical_count: i32,
    pub high_count: i32,
    pub medium_count: i32,
    pub low_count: i32,
    pub scan_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Request to trigger a security scan
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TriggerSecurityScanRequest {
    pub contract_id: Uuid,
    pub version: Option<String>,
    pub scanner_ids: Option<Vec<Uuid>>,
    pub scan_type: Option<String>,
}

/// Request to create/update a security scanner configuration
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateSecurityScannerRequest {
    pub name: String,
    pub description: Option<String>,
    pub scanner_type: String,
    pub api_endpoint: Option<String>,
    pub api_key: Option<String>,
    pub configuration: Option<serde_json::Value>,
    pub timeout_seconds: Option<i32>,
}

/// Request to update security issue status
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateSecurityIssueRequest {
    pub status: IssueStatus,
    pub notes: Option<String>,
}

/// Security scan summary for a contract
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractSecuritySummary {
    pub contract_id: Uuid,
    pub contract_name: String,
    pub latest_scan: Option<SecurityScanSummary>,
    pub total_scans: i64,
    pub open_issues: i64,
    pub critical_open: i64,
    pub high_open: i64,
    pub security_score: Option<i32>,
}

/// Summary of a single security scan
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct SecurityScanSummary {
    pub id: Uuid,
    pub status: ScanStatus,
    pub scan_type: String,
    pub total_issues: i32,
    pub critical_issues: i32,
    pub high_issues: i32,
    pub medium_issues: i32,
    pub low_issues: i32,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Security scan history response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SecurityScanHistoryResponse {
    pub scans: Vec<SecurityScanSummary>,
    pub total_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "audit_status", rename_all = "snake_case")]
pub enum AuditScanStatus {
    Passed,
    Issues,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema)]
#[sqlx(type_name = "audit_type", rename_all = "snake_case")]
pub enum AuditType {
    Formal,
    Informal,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractAuditFinding {
    pub severity: String,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ContractAuditResponse {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub audit_type: AuditType,
    pub status: AuditScanStatus,
    pub auditor: Option<String>,
    pub audit_date: DateTime<Utc>,
    pub findings_summary: Vec<ContractAuditFinding>,
    pub total_issues: i32,
    pub report_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PaginatedAuditsResponse {
    pub audits: Vec<ContractAuditResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

// ═══════════════════════════════════════════════════════════════════════════
// NOTIFICATION/SUBSCRIPTION TYPES (#493)
// ═══════════════════════════════════════════════════════════════════════════

/// Notification type
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq)]
#[sqlx(type_name = "notification_type", rename_all = "snake_case")]
pub enum NotificationType {
    NewVersion,
    VerificationStatus,
    SecurityIssue,
    SecurityScanCompleted,
    BreakingChange,
    Deprecation,
    Maintenance,
    CompatibilityIssue,
}

/// Notification channel
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq)]
#[sqlx(type_name = "notification_channel", rename_all = "snake_case")]
pub enum NotificationChannel {
    Email,
    Webhook,
    Push,
    InApp,
}

/// Notification frequency
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq)]
#[sqlx(type_name = "notification_frequency", rename_all = "snake_case")]
pub enum NotificationFrequency {
    Realtime,
    DailyDigest,
    WeeklyDigest,
}

impl std::fmt::Display for NotificationFrequency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Realtime => write!(f, "realtime"),
            Self::DailyDigest => write!(f, "daily_digest"),
            Self::WeeklyDigest => write!(f, "weekly_digest"),
        }
    }
}

/// Subscription status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq)]
#[sqlx(type_name = "subscription_status", rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Active,
    Paused,
    Unsubscribed,
}

/// Contract subscription
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ContractSubscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub contract_id: Uuid,
    pub status: SubscriptionStatus,
    pub notification_types: Vec<NotificationType>,
    pub channels: Vec<NotificationChannel>,
    pub frequency: NotificationFrequency,
    pub min_severity: Option<IssueSeverity>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to subscribe to a contract
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SubscribeRequest {
    pub contract_id: Uuid,
    pub notification_types: Option<Vec<NotificationType>>,
    pub channels: Option<Vec<NotificationChannel>>,
    pub frequency: Option<NotificationFrequency>,
    pub min_severity: Option<IssueSeverity>,
}

/// Request to update subscription preferences
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateSubscriptionRequest {
    pub status: Option<SubscriptionStatus>,
    pub notification_types: Option<Vec<NotificationType>>,
    pub channels: Option<Vec<NotificationChannel>>,
    pub frequency: Option<NotificationFrequency>,
    pub min_severity: Option<IssueSeverity>,
}

/// User notification preferences
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct UserNotificationPreferences {
    pub id: Uuid,
    pub publisher_id: Uuid,
    pub notification_frequency: NotificationFrequency,
    pub notification_channels: Vec<NotificationChannel>,
    pub email_notifications_enabled: bool,
    pub webhook_url: Option<String>,
    pub quiet_hours_start: Option<String>,
    pub quiet_hours_end: Option<String>,
    pub timezone: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to update user notification preferences
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateUserNotificationPreferencesRequest {
    pub notification_frequency: Option<NotificationFrequency>,
    pub notification_channels: Option<Vec<NotificationChannel>>,
    pub email_notifications_enabled: Option<bool>,
    pub webhook_url: Option<String>,
    pub webhook_secret: Option<String>,
    pub quiet_hours_start: Option<String>,
    pub quiet_hours_end: Option<String>,
    pub timezone: Option<String>,
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct WebhookConfiguration {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub name: String,
    pub url: String,
    pub notification_types: Vec<NotificationType>,
    pub is_active: bool,
    pub verify_ssl: bool,
    pub custom_headers: Option<serde_json::Value>,
    pub rate_limit_per_minute: Option<i32>,
    pub total_deliveries: i32,
    pub failed_deliveries: i32,
    pub last_delivery_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub consecutive_failures: i32,
    /// Signing secret — only populated in the creation response, never in reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a webhook
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateWebhookRequest {
    pub name: String,
    pub url: String,
    pub notification_types: Vec<NotificationType>,
    pub secret: Option<String>,
    pub verify_ssl: Option<bool>,
    pub custom_headers: Option<serde_json::Value>,
}

/// Notification queue item
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct NotificationQueueItem {
    pub id: Uuid,
    pub subscription_id: Uuid,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub contract_id: Uuid,
    pub contract_version_id: Option<Uuid>,
    pub security_issue_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub channels: Vec<NotificationChannel>,
    pub status: String,
    pub priority: i32,
    pub scheduled_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// User's subscriptions list response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UserSubscriptionsResponse {
    pub subscriptions: Vec<ContractSubscriptionSummary>,
    pub total_count: i64,
}

/// Summary of a contract subscription
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ContractSubscriptionSummary {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub contract_name: String,
    pub contract_slug: Option<String>,
    pub status: SubscriptionStatus,
    pub notification_types: Vec<NotificationType>,
    pub channels: Vec<NotificationChannel>,
    pub frequency: NotificationFrequency,
    pub created_at: DateTime<Utc>,
}

/// Notification statistics
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct NotificationStatistics {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub contract_id: Option<Uuid>,
    pub period_start: chrono::NaiveDate,
    pub period_end: chrono::NaiveDate,
    pub new_version_count: i32,
    pub verification_status_count: i32,
    pub security_issue_count: i32,
    pub security_scan_completed_count: i32,
    pub breaking_change_count: i32,
    pub deprecation_count: i32,
    pub maintenance_count: i32,
    pub compatibility_issue_count: i32,
    pub total_sent: i32,
    pub total_delivered: i32,
    pub total_failed: i32,
}

// ═══════════════════════════════════════════════════════════════════════════
// ZERO-KNOWLEDGE PROOF VALIDATION SYSTEM (Issue #624)
// ═══════════════════════════════════════════════════════════════════════════

/// Supported ZK proof systems
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq)]
#[sqlx(type_name = "zk_proof_system", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ZkProofSystem {
    Groth16,
    Plonk,
    Stark,
    Marlin,
    Fflonk,
}

impl std::fmt::Display for ZkProofSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ZkProofSystem::Groth16 => "groth16",
            ZkProofSystem::Plonk => "plonk",
            ZkProofSystem::Stark => "stark",
            ZkProofSystem::Marlin => "marlin",
            ZkProofSystem::Fflonk => "fflonk",
        };
        write!(f, "{}", s)
    }
}

/// Supported circuit languages / DSLs
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq)]
#[sqlx(type_name = "zk_circuit_language", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ZkCircuitLanguage {
    Circom,
    Noir,
    Leo,
    Cairo,
    Halo2,
}

impl std::fmt::Display for ZkCircuitLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ZkCircuitLanguage::Circom => "circom",
            ZkCircuitLanguage::Noir => "noir",
            ZkCircuitLanguage::Leo => "leo",
            ZkCircuitLanguage::Cairo => "cairo",
            ZkCircuitLanguage::Halo2 => "halo2",
        };
        write!(f, "{}", s)
    }
}

/// ZK proof validation status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, utoipa::ToSchema, PartialEq)]
#[sqlx(type_name = "zk_proof_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ZkProofStatus {
    Pending,
    Valid,
    Invalid,
    Error,
}

impl std::fmt::Display for ZkProofStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ZkProofStatus::Pending => "pending",
            ZkProofStatus::Valid => "valid",
            ZkProofStatus::Invalid => "invalid",
            ZkProofStatus::Error => "error",
        };
        write!(f, "{}", s)
    }
}

/// A compiled ZK circuit registered for a contract
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ZkCircuit {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub language: ZkCircuitLanguage,
    pub proof_system: ZkProofSystem,
    /// Circuit source code / program text
    pub circuit_source: String,
    /// SHA-256 hash of the compiled circuit artifact
    pub circuit_hash: String,
    /// Serialised verification key (base64)
    pub verification_key: String,
    pub num_public_inputs: i32,
    pub num_constraints: Option<i64>,
    pub metadata: Option<serde_json::Value>,
    pub compiled_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request body to register / compile a new circuit
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RegisterCircuitRequest {
    pub name: String,
    pub description: Option<String>,
    pub language: ZkCircuitLanguage,
    pub proof_system: ZkProofSystem,
    pub circuit_source: String,
    /// Pre-computed verification key (base64)
    pub verification_key: String,
    pub num_public_inputs: i32,
    pub num_constraints: Option<i64>,
    pub metadata: Option<serde_json::Value>,
    pub created_by_address: Option<String>,
}

/// A ZK proof submitted for validation
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ZkProofSubmission {
    pub id: Uuid,
    pub circuit_id: Uuid,
    pub contract_id: Uuid,
    pub proof_data: String,
    pub public_inputs: serde_json::Value,
    pub status: ZkProofStatus,
    pub prover_address: String,
    pub purpose: Option<String>,
    pub error_message: Option<String>,
    pub verification_ms: Option<i64>,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Request body to submit a ZK proof for validation
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SubmitProofRequest {
    pub circuit_id: Uuid,
    pub proof_data: String,
    pub public_inputs: Vec<String>,
    pub prover_address: String,
    pub purpose: Option<String>,
}

/// Result returned after proof validation
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ZkProofValidationResult {
    pub proof_id: Uuid,
    pub circuit_id: Uuid,
    pub contract_id: Uuid,
    pub status: ZkProofStatus,
    pub valid: bool,
    pub message: String,
    pub verification_ms: Option<i64>,
    pub verified_at: Option<DateTime<Utc>>,
}

/// Privacy-preserving aggregate analytics for a contract's ZK proof activity
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ZkAnalyticsAggregate {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub circuit_id: Option<Uuid>,
    pub bucket_hour: DateTime<Utc>,
    pub proof_system: ZkProofSystem,
    pub total_proofs: i64,
    pub valid_proofs: i64,
    pub invalid_proofs: i64,
    pub error_proofs: i64,
    pub avg_verify_ms: Option<rust_decimal::Decimal>,
    pub p99_verify_ms: Option<rust_decimal::Decimal>,
}

/// Aggregated ZK analytics response (no individual prover data exposed)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ZkAnalyticsSummary {
    pub contract_id: Uuid,
    pub total_proofs: i64,
    pub valid_proofs: i64,
    pub invalid_proofs: i64,
    pub error_proofs: i64,
    pub success_rate_pct: f64,
    pub avg_verify_ms: Option<f64>,
    pub circuits: Vec<ZkCircuitStats>,
}

/// Per-circuit statistics within the analytics summary
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ZkCircuitStats {
    pub circuit_id: Uuid,
    pub circuit_name: String,
    pub proof_system: ZkProofSystem,
    pub total_proofs: i64,
    pub valid_proofs: i64,
    pub success_rate_pct: f64,
    pub avg_verify_ms: Option<f64>,
}

/// Circuit summary (safe to expose publicly — omits circuit_source & verification_key)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, sqlx::FromRow)]
pub struct ZkCircuitSummary {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub language: ZkCircuitLanguage,
    pub proof_system: ZkProofSystem,
    pub circuit_hash: String,
    pub num_public_inputs: i32,
    pub num_constraints: Option<i64>,
    pub compiled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_contract_usage_count_serialization() {
        // Test that Contract can be serialized with usage_count
        let contract = Contract {
            id: Uuid::new_v4(),
            contract_id: "C123".to_string(),
            wasm_hash: "hash123".to_string(),
            name: "Test Contract".to_string(),
            slug: "test-contract".to_string(),
            description: Some("A test contract".to_string()),
            publisher_id: Uuid::new_v4(),
            network: Network::Mainnet,
            is_verified: true,
            verification_status: VerificationStatus::Verified,
            category: Some("DeFi".to_string()),
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            verified_at: Some(Utc::now()),
            deployed_at: None,
            verified_by: Some(Uuid::new_v4()),
            verification_notes: None,
            last_accessed_at: Some(Utc::now()),
            health_score: 100,
            is_maintenance: false,
            logical_id: None,
            network_configs: None,
            relevance_score: None,
            organization_id: None,
            visibility: VisibilityType::Public,
            current_version: Some("1.0.0".to_string()),
            usage_count: 42,
        };

        let json = serde_json::to_string(&contract).expect("Failed to serialize contract");
        assert!(json.contains("\"usage_count\":42"));

        let deserialized: Contract =
            serde_json::from_str(&json).expect("Failed to deserialize contract");
        assert_eq!(deserialized.usage_count, 42);
    }

    #[test]
    fn test_contract_usage_count_backward_compatibility() {
        // Test that Contract can be deserialized from JSON without usage_count field
        // This simulates backward compatibility with existing API responses
        let json_without_usage_count = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "contract_id": "C123",
            "wasm_hash": "hash123",
            "name": "Test Contract",
            "slug": "test-contract",
            "description": "A test contract",
            "publisher_id": "550e8400-e29b-41d4-a716-446655440001",
            "network": "mainnet",
            "is_verified": true,
            "verification_status": "Verified",
            "category": "DeFi",
            "tags": [],
            "created_at": "2023-10-27T10:00:00Z",
            "updated_at": "2023-10-27T10:00:00Z",
            "verified_at": "2023-10-27T10:00:00Z",
            "verified_by": "550e8400-e29b-41d4-a716-446655440002",
            "verification_notes": null,
            "last_accessed_at": "2023-10-27T10:00:00Z",
            "health_score": 100,
            "is_maintenance": false,
            "logical_id": null,
            "network_configs": null,
            "relevance_score": null,
            "organization_id": null,
            "visibility": "public",
            "current_version": "1.0.0"
        }"#;

        let contract: Contract = serde_json::from_str(json_without_usage_count)
            .expect("Failed to deserialize contract without usage_count");

        // usage_count should default to 0 due to #[serde(default)]
        assert_eq!(contract.usage_count, 0);
        assert_eq!(contract.name, "Test Contract");
        assert_eq!(contract.contract_id, "C123");
    }

    #[test]
    fn test_contract_usage_count_with_explicit_zero() {
        // Test that explicit zero value works correctly
        let json_with_zero_usage = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "contract_id": "C123",
            "wasm_hash": "hash123",
            "name": "Test Contract",
            "slug": "test-contract",
            "description": "A test contract",
            "publisher_id": "550e8400-e29b-41d4-a716-446655440001",
            "network": "mainnet",
            "is_verified": true,
            "verification_status": "Verified",
            "category": "DeFi",
            "tags": [],
            "created_at": "2023-10-27T10:00:00Z",
            "updated_at": "2023-10-27T10:00:00Z",
            "verified_at": "2023-10-27T10:00:00Z",
            "verified_by": "550e8400-e29b-41d4-a716-446655440002",
            "verification_notes": null,
            "last_accessed_at": "2023-10-27T10:00:00Z",
            "health_score": 100,
            "is_maintenance": false,
            "logical_id": null,
            "network_configs": null,
            "relevance_score": null,
            "organization_id": null,
            "visibility": "public",
            "current_version": "1.0.0",
            "usage_count": 0
        }"#;

        let contract: Contract = serde_json::from_str(json_with_zero_usage)
            .expect("Failed to deserialize contract with zero usage_count");

        assert_eq!(contract.usage_count, 0);
    }
}
