use async_graphql::{
    dataloader::DataLoader, Context, Enum, Error, Object, Result, SimpleObject, Union,
};
use chrono::{DateTime, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde_json::Value;
use shared::models::{
    AlertSeverity, AuditActionType, Contract, ContractAuditLog, ContractInteraction,
    ContractPerformanceSummaryResponse, ContractVersion, DependencyNode, DependencyResponse,
    MetricType, Network, Organization, PerformanceAlert, PerformanceBenchmark,
    PerformanceMetricSnapshot, PerformanceRegression, PerformanceTrendPoint, Publisher,
    VisibilityType,
};

use crate::handlers::ContractChangelogEntry;
use uuid::Uuid;

use crate::state::AppState;

// ─── ContractType ────────────────────────────────────────────────────────────

pub struct ContractType {
    pub id: Uuid,
    pub contract_id: String,
    pub wasm_hash: String,
    pub name: String,
    pub description: Option<String>,
    pub publisher_id: Uuid,
    pub network: Network,
    pub is_verified: bool,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub health_score: i32,
    pub visibility: VisibilityType,
    pub organization_id: Option<Uuid>,
}

#[Object]
impl ContractType {
    async fn id(&self) -> Uuid {
        self.id
    }
    async fn contract_id(&self) -> &str {
        &self.contract_id
    }
    async fn wasm_hash(&self) -> &str {
        &self.wasm_hash
    }
    async fn name(&self) -> &str {
        &self.name
    }
    async fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    async fn network(&self) -> NetworkType {
        self.network.clone().into()
    }
    async fn is_verified(&self) -> bool {
        self.is_verified
    }
    async fn category(&self) -> Option<&str> {
        self.category.as_deref()
    }
    async fn tags(&self) -> &[String] {
        &self.tags
    }
    async fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    async fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
    async fn health_score(&self) -> i32 {
        self.health_score
    }
    async fn visibility(&self) -> VisibilityTypeGraphQL {
        self.visibility.clone().into()
    }

    /// Resolve the publisher for this contract (uses DataLoader to avoid N+1)
    async fn publisher(&self, ctx: &Context<'_>) -> Result<PublisherType> {
        let loader = ctx.data_unchecked::<DataLoader<crate::graphql::loaders::PublisherLoader>>();
        let publisher = loader
            .load_one(self.publisher_id)
            .await?
            .ok_or_else(|| Error::new("Publisher not found"))?;
        Ok(PublisherType::from(publisher))
    }

    /// Resolve the owning organisation (if any)
    async fn organization(&self, ctx: &Context<'_>) -> Result<Option<OrganizationType>> {
        if let Some(org_id) = self.organization_id {
            let loader =
                ctx.data_unchecked::<DataLoader<crate::graphql::loaders::OrganizationLoader>>();
            let org = loader.load_one(org_id).await?;
            Ok(org.map(OrganizationType::from))
        } else {
            Ok(None)
        }
    }

    /// Resolve versions for this contract (uses DataLoader; optional limit)
    async fn versions(
        &self,
        ctx: &Context<'_>,
        limit: Option<usize>,
    ) -> Result<Vec<ContractVersionType>> {
        let loader =
            ctx.data_unchecked::<DataLoader<crate::graphql::loaders::ContractVersionsLoader>>();
        let versions = loader.load_one(self.id).await?.unwrap_or_default();
        let items = versions
            .into_iter()
            .take(limit.unwrap_or(usize::MAX))
            .map(ContractVersionType::from)
            .collect();
        Ok(items)
    }

    /// Fetch audit log entries for this contract
    async fn audit_log(&self, ctx: &Context<'_>) -> Result<Vec<AuditLogType>> {
        let state = ctx.data::<AppState>()?;
        let logs: Vec<ContractAuditLog> = sqlx::query_as(
            "SELECT * FROM contract_audit_log WHERE contract_id = $1 ORDER BY created_at DESC",
        )
        .bind(self.id)
        .fetch_all(&state.db)
        .await?;
        Ok(logs.into_iter().map(AuditLogType::from).collect())
    }

    /// Fetch recent metadata/status interactions
    async fn interactions(
        &self,
        ctx: &Context<'_>,
        limit: Option<i64>,
    ) -> Result<Vec<InteractionType>> {
        let state = ctx.data::<AppState>()?;
        let interactions: Vec<ContractInteraction> = sqlx::query_as(
            "SELECT * FROM contract_interactions WHERE contract_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(self.id)
        .bind(limit.unwrap_or(50))
        .fetch_all(&state.db)
        .await?;
        Ok(interactions
            .into_iter()
            .map(InteractionType::from)
            .collect())
    }

    /// Get aggregated performance summary (benchmarks, trends, etc.)
    async fn performance(&self, ctx: &Context<'_>) -> Result<PerformanceSummaryType> {
        let state = ctx.data::<AppState>()?;
        let summary =
            crate::performance_handlers::build_performance_summary_internal(state, self.id).await?;
        Ok(PerformanceSummaryType::from(summary))
    }

    /// Build the dependency tree for this contract
    async fn dependencies(&self, ctx: &Context<'_>) -> Result<DependencyResponseType> {
        let state = ctx.data::<AppState>()?;
        // We'll call the internal logic from dependency_handlers if available, or re-implement logic
        // For simplicity and to avoid circular deps if they occur, we assume it's exposed or we use similar logic.
        // Actually, let's just use the logic from dependency_handlers by making it accessible or duplicating logic for the resolver.
        // Given we are in the same crate, we can just call the logic.

        let response =
            crate::dependency_handlers::get_contract_dependencies_internal(state, self.id).await?;
        Ok(DependencyResponseType::from(response))
    }
}

impl From<Contract> for ContractType {
    fn from(c: Contract) -> Self {
        Self {
            id: c.id,
            contract_id: c.contract_id,
            wasm_hash: c.wasm_hash,
            name: c.name,
            description: c.description,
            publisher_id: c.publisher_id,
            network: c.network,
            is_verified: c.is_verified,
            category: c.category,
            tags: c.tags.into_iter().map(|tag| tag.name).collect(),
            created_at: c.created_at,
            updated_at: c.updated_at,
            health_score: c.health_score,
            visibility: c.visibility,
            organization_id: c.organization_id,
        }
    }
}

// ─── Network / Visibility enums ──────────────────────────────────────────────

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum NetworkType {
    Mainnet,
    Testnet,
    Futurenet,
}

impl From<Network> for NetworkType {
    fn from(n: Network) -> Self {
        match n {
            Network::Mainnet => Self::Mainnet,
            Network::Testnet => Self::Testnet,
            Network::Futurenet => Self::Futurenet,
        }
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum VisibilityTypeGraphQL {
    Public,
    Private,
}

impl From<VisibilityType> for VisibilityTypeGraphQL {
    fn from(v: VisibilityType) -> Self {
        match v {
            VisibilityType::Public => Self::Public,
            VisibilityType::Private => Self::Private,
        }
    }
}

// ─── ContractVersionType ─────────────────────────────────────────────────────

pub struct ContractVersionType {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub wasm_hash: String,
    pub source_url: Option<String>,
    pub commit_hash: Option<String>,
    pub release_notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[Object]
impl ContractVersionType {
    async fn id(&self) -> Uuid {
        self.id
    }
    async fn contract_id(&self) -> Uuid {
        self.contract_id
    }
    async fn version(&self) -> &str {
        &self.version
    }
    async fn wasm_hash(&self) -> &str {
        &self.wasm_hash
    }
    async fn source_url(&self) -> Option<&str> {
        self.source_url.as_deref()
    }
    async fn commit_hash(&self) -> Option<&str> {
        self.commit_hash.as_deref()
    }
    async fn release_notes(&self) -> Option<&str> {
        self.release_notes.as_deref()
    }
    async fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Resolve the parent contract via DataLoader
    async fn contract(&self, ctx: &Context<'_>) -> Result<ContractType> {
        let loader = ctx.data_unchecked::<DataLoader<crate::graphql::loaders::DbLoader>>();
        let contract = loader
            .load_one(self.contract_id)
            .await?
            .ok_or_else(|| Error::new("Contract not found"))?;
        Ok(ContractType::from(contract))
    }
}

impl From<ContractVersion> for ContractVersionType {
    fn from(v: ContractVersion) -> Self {
        Self {
            id: v.id,
            contract_id: v.contract_id,
            version: v.version,
            wasm_hash: v.wasm_hash,
            source_url: v.source_url,
            commit_hash: v.commit_hash,
            release_notes: v.release_notes,
            created_at: v.created_at,
        }
    }
}

// ─── PublisherType ───────────────────────────────────────────────────────────

pub struct PublisherType {
    pub id: Uuid,
    pub stellar_address: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub github_url: Option<String>,
    pub website: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[Object]
impl PublisherType {
    async fn id(&self) -> Uuid {
        self.id
    }
    async fn stellar_address(&self) -> &str {
        &self.stellar_address
    }
    async fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }
    async fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }
    async fn github_url(&self) -> Option<&str> {
        self.github_url.as_deref()
    }
    async fn website(&self) -> Option<&str> {
        self.website.as_deref()
    }
    async fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// All contracts published by this publisher
    async fn contracts(&self, ctx: &Context<'_>) -> Result<Vec<ContractType>> {
        let state = ctx.data::<AppState>()?;
        let contracts: Vec<Contract> = sqlx::query_as(
            "SELECT * FROM contracts WHERE publisher_id = $1 ORDER BY created_at DESC",
        )
        .bind(self.id)
        .fetch_all(&state.db)
        .await?;
        Ok(contracts.into_iter().map(ContractType::from).collect())
    }
}

impl From<Publisher> for PublisherType {
    fn from(p: Publisher) -> Self {
        Self {
            id: p.id,
            stellar_address: p.stellar_address,
            username: p.username,
            email: p.email,
            github_url: p.github_url,
            website: p.website,
            created_at: p.created_at,
        }
    }
}

// ─── OrganizationType ────────────────────────────────────────────────────────

pub struct OrganizationType {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub is_private: bool,
    pub created_at: DateTime<Utc>,
}

#[Object]
impl OrganizationType {
    async fn id(&self) -> Uuid {
        self.id
    }
    async fn name(&self) -> &str {
        &self.name
    }
    async fn slug(&self) -> &str {
        &self.slug
    }
    async fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    async fn is_private(&self) -> bool {
        self.is_private
    }
    async fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// All contracts owned by this organisation
    async fn contracts(&self, ctx: &Context<'_>) -> Result<Vec<ContractType>> {
        let state = ctx.data::<AppState>()?;
        let contracts: Vec<Contract> = sqlx::query_as(
            "SELECT * FROM contracts WHERE organization_id = $1 ORDER BY created_at DESC",
        )
        .bind(self.id)
        .fetch_all(&state.db)
        .await?;
        Ok(contracts.into_iter().map(ContractType::from).collect())
    }
}

impl From<Organization> for OrganizationType {
    fn from(o: Organization) -> Self {
        Self {
            id: o.id,
            name: o.name,
            slug: o.slug,
            description: o.description,
            is_private: o.is_private,
            created_at: o.created_at,
        }
    }
}

// ─── Paginated response ───────────────────────────────────────────────────────

/// Paginated list of contracts returned by the `contracts` query
#[derive(SimpleObject)]
pub struct PaginatedContracts {
    pub items: Vec<ContractType>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

// ─── CategoryType ─────────────────────────────────────────────────────────────

pub struct CategoryType {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub is_default: bool,
    pub usage_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[Object]
impl CategoryType {
    async fn id(&self) -> Uuid {
        self.id
    }
    async fn name(&self) -> &str {
        &self.name
    }
    async fn slug(&self) -> &str {
        &self.slug
    }
    async fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    async fn is_default(&self) -> bool {
        self.is_default
    }
    async fn usage_count(&self) -> i64 {
        self.usage_count
    }
    async fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    async fn parent(&self, ctx: &Context<'_>) -> Result<Option<CategoryType>> {
        if let Some(pid) = self.parent_id {
            let loader =
                ctx.data_unchecked::<DataLoader<crate::graphql::loaders::CategoryLoader>>();
            let row = loader.load_one(pid).await?;
            Ok(row.map(CategoryType::from))
        } else {
            Ok(None)
        }
    }
}

impl From<crate::category_handlers::CategoryRow> for CategoryType {
    fn from(row: crate::category_handlers::CategoryRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            slug: row.slug,
            description: row.description,
            parent_id: row.parent_id,
            is_default: row.is_default,
            usage_count: row.usage_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

// ─── AuditLogType ─────────────────────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct AuditLogType {
    pub id: Uuid,
    pub action_type: String,
    pub actor: String,
    pub target_id: Uuid,
    pub before_state: Option<Value>,
    pub after_state: Option<Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<ContractAuditLog> for AuditLogType {
    fn from(l: ContractAuditLog) -> Self {
        Self {
            id: l.id,
            action_type: l.action_type.to_string(),
            actor: l.changed_by,
            target_id: l.contract_id,
            before_state: l.old_value,
            after_state: l.new_value,
            ip_address: None,
            user_agent: None,
            created_at: l.timestamp,
        }
    }
}

// ─── InteractionType ──────────────────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct InteractionType {
    pub id: Uuid,
    pub name: String,
    pub details: Option<Value>,
    pub created_at: DateTime<Utc>,
}

impl From<ContractInteraction> for InteractionType {
    fn from(i: ContractInteraction) -> Self {
        Self {
            id: i.id,
            name: i.interaction_type,
            details: i.parameters.or(i.return_value),
            created_at: i.created_at,
        }
    }
}

// ─── PerformanceSummaryType ───────────────────────────────────────────────────

fn decimal_to_f64(value: Decimal) -> f64 {
    value.to_f64().unwrap_or(0.0)
}

fn option_decimal_to_f64(value: Option<Decimal>) -> Option<f64> {
    value.and_then(|v| v.to_f64())
}

#[derive(SimpleObject)]
pub struct PerformanceSummaryType {
    pub latest_benchmarks: Vec<PerformanceBenchmarkType>,
    pub metric_snapshots: Vec<PerformanceMetricSnapshotType>,
    pub trends: Vec<PerformanceTrendPointType>,
    pub regressions: Vec<PerformanceRegressionType>,
    pub unresolved_alerts: Vec<PerformanceAlertType>,
}

impl From<ContractPerformanceSummaryResponse> for PerformanceSummaryType {
    fn from(s: ContractPerformanceSummaryResponse) -> Self {
        Self {
            latest_benchmarks: s
                .latest_benchmarks
                .into_iter()
                .map(PerformanceBenchmarkType::from)
                .collect(),
            metric_snapshots: s
                .metric_snapshots
                .into_iter()
                .map(PerformanceMetricSnapshotType::from)
                .collect(),
            trends: s
                .trends
                .into_iter()
                .map(PerformanceTrendPointType::from)
                .collect(),
            regressions: s
                .regressions
                .into_iter()
                .map(PerformanceRegressionType::from)
                .collect(),
            unresolved_alerts: s
                .unresolved_alerts
                .into_iter()
                .map(PerformanceAlertType::from)
                .collect(),
        }
    }
}

#[derive(SimpleObject)]
pub struct PerformanceBenchmarkType {
    pub id: Uuid,
    pub version: Option<String>,
    pub benchmark_name: String,
    pub execution_time_ms: f64,
    pub gas_used: i64,
    pub sample_size: i32,
    pub recorded_at: DateTime<Utc>,
}

impl From<PerformanceBenchmark> for PerformanceBenchmarkType {
    fn from(b: PerformanceBenchmark) -> Self {
        Self {
            id: b.id,
            version: b.version,
            benchmark_name: b.benchmark_name,
            execution_time_ms: decimal_to_f64(b.execution_time_ms),
            gas_used: b.gas_used,
            sample_size: b.sample_size,
            recorded_at: b.recorded_at,
        }
    }
}

#[derive(SimpleObject)]
pub struct PerformanceMetricSnapshotType {
    pub metric_type: String,
    pub latest_value: f64,
    pub change_percent: Option<f64>,
}

impl From<PerformanceMetricSnapshot> for PerformanceMetricSnapshotType {
    fn from(s: PerformanceMetricSnapshot) -> Self {
        Self {
            metric_type: s.metric_type,
            latest_value: decimal_to_f64(s.latest_value),
            change_percent: option_decimal_to_f64(s.change_percent),
        }
    }
}

#[derive(SimpleObject)]
pub struct PerformanceTrendPointType {
    pub bucket_start: DateTime<Utc>,
    pub avg_execution_time_ms: f64,
    pub avg_gas_used: f64,
}

impl From<PerformanceTrendPoint> for PerformanceTrendPointType {
    fn from(p: PerformanceTrendPoint) -> Self {
        Self {
            bucket_start: p.bucket_start,
            avg_execution_time_ms: decimal_to_f64(p.avg_execution_time_ms),
            avg_gas_used: decimal_to_f64(p.avg_gas_used),
        }
    }
}

#[derive(SimpleObject)]
pub struct PerformanceRegressionType {
    pub benchmark_name: String,
    pub regression_percent: Option<f64>,
    pub severity: String,
    pub detected_at: DateTime<Utc>,
}

impl From<PerformanceRegression> for PerformanceRegressionType {
    fn from(r: PerformanceRegression) -> Self {
        Self {
            benchmark_name: r.benchmark_name,
            regression_percent: option_decimal_to_f64(r.execution_time_regression_percent),
            severity: r.severity,
            detected_at: r.detected_at,
        }
    }
}

#[derive(SimpleObject)]
pub struct PerformanceAlertType {
    pub id: Uuid,
    pub metric_type: String,
    pub current_value: f64,
    pub severity: String,
    pub triggered_at: DateTime<Utc>,
}

impl From<PerformanceAlert> for PerformanceAlertType {
    fn from(a: PerformanceAlert) -> Self {
        Self {
            id: a.id,
            metric_type: format!("{:?}", a.metric_type),
            current_value: decimal_to_f64(a.current_value),
            severity: format!("{:?}", a.severity),
            triggered_at: a.triggered_at,
        }
    }
}

// ─── DependencyType ───────────────────────────────────────────────────────────

#[derive(SimpleObject)]
pub struct DependencyResponseType {
    pub root: DependencyNodeType,
    pub total_dependencies: i32,
    pub max_depth: i32,
    pub has_circular: bool,
}

impl From<DependencyResponse> for DependencyResponseType {
    fn from(d: DependencyResponse) -> Self {
        Self {
            root: DependencyNodeType::from(d.root),
            total_dependencies: d.total_dependencies as i32,
            max_depth: d.max_depth as i32,
            has_circular: d.has_circular,
        }
    }
}

#[derive(SimpleObject)]
pub struct DependencyNodeType {
    pub contract_id: String,
    pub name: Option<String>,
    pub status: String,
    pub is_circular: bool,
    pub dependencies: Vec<DependencyNodeType>,
}

impl From<DependencyNode> for DependencyNodeType {
    fn from(n: DependencyNode) -> Self {
        Self {
            contract_id: n.contract_id,
            name: n.name,
            status: n.status,
            is_circular: n.is_circular,
            dependencies: n
                .dependencies
                .into_iter()
                .map(DependencyNodeType::from)
                .collect(),
        }
    }
}
