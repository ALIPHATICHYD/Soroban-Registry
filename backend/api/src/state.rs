use crate::ai::service::AIService;
use crate::auth::AuthManager;
use crate::cache::{CacheConfig, CacheLayer};
use crate::contract_events::ContractEventHub;
use crate::crypto::EncryptionService;
use crate::feature_flags::FeatureFlagManager;
use crate::health_monitor::HealthMonitorStatus;
use crate::rate_limit::RateLimitState;
use crate::resource_tracking::ResourceManager;
use crate::search_client::SearchClient;
use crate::search_postgres::PostgresSearchService;
use crate::state_monitor::StateMonitorService;
use shared::source_storage::SourceStorage;

use prometheus::Registry;
use sqlx::PgPool;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tracing::{info, warn};

use serde_json::Value;
use shared::models::Network;
use tokio::sync::broadcast;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractEventVisibility {
    Public,
    Private,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum RealtimeEvent {
    ContractDeployed {
        contract_id: String,
        contract_name: String,
        publisher: String,
        version: String,
        timestamp: String,
        network: Network,
    },
    ContractUpdated {
        contract_id: String,
        update_type: String,
        details: Value,
        timestamp: String,
    },
    CicdPipeline {
        contract_id: String,
        status: String,
        steps_completed: u32,
        total_steps: u32,
        timestamp: String,
    },
    VersionCreated {
        contract_id: String,
        version: String,
        network: Network,
        timestamp: String,
    },
    MetadataUpdated {
        contract_id: String,
        timestamp: String,
        changes: Value,
        visibility: ContractEventVisibility,
    },
    StatusUpdated {
        contract_id: String,
        status: String,
        timestamp: String,
        is_verified: bool,
        details: Option<Value>,
        visibility: ContractEventVisibility,
    },
}

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub started_at: Instant,
    pub cache: Arc<CacheLayer>,
    pub contract_events: Arc<ContractEventHub>,
    pub registry: Registry,
    pub job_engine: Arc<soroban_batch::engine::JobEngine>,
    pub is_shutting_down: Arc<AtomicBool>,
    pub health_monitor_status: HealthMonitorStatus,
    pub auth_mgr: Arc<RwLock<AuthManager>>,
    pub resource_mgr: Arc<RwLock<ResourceManager>>,
    pub source_storage: Arc<SourceStorage>,
    pub event_broadcaster: broadcast::Sender<RealtimeEvent>,
    pub search: Arc<SearchClient>,
    pub pg_search: Arc<PostgresSearchService>,
    pub ai_service: Option<Arc<AIService>>,
    pub state_monitor: Option<Arc<StateMonitorService>>,
    pub rate_limit_state: Arc<RateLimitState>,
    pub db_breaker: Arc<crate::db_resilience::CircuitBreaker>,
    pub db_queue: Arc<crate::db_resilience::DbQueue>,
    pub feature_flags: Arc<FeatureFlagManager>,
    /// At-rest field encryption service (#895).
    pub encryption: Arc<EncryptionService>,
}

impl AppState {
    pub async fn new(
        db: PgPool,
        registry: Registry,
        job_engine: Arc<soroban_batch::engine::JobEngine>,
        is_shutting_down: Arc<AtomicBool>,
        rate_limit_state: Arc<RateLimitState>,
        ai_service: Option<Arc<AIService>>,
        event_broadcaster: broadcast::Sender<RealtimeEvent>,
        db_breaker: Arc<crate::db_resilience::CircuitBreaker>,
        db_queue: Arc<crate::db_resilience::DbQueue>,
        feature_flags: Arc<FeatureFlagManager>,
    ) -> Result<Self, shared::error::RegistryError> {
        let config = CacheConfig::from_env();
        let auth_manager = match AuthManager::from_env() {
            Ok(manager) => manager,
            Err(err) => {
                #[cfg(test)]
                {
                    let _ = err;
                    AuthManager::new("test-jwt-secret-at-least-32-chars".to_string())
                }
                #[cfg(not(test))]
                {
                    panic!("JWT config validated at startup: {:?}", err)
                }
            }
        };
        let auth_mgr = Arc::new(RwLock::new(auth_manager));
        let resource_mgr = Arc::new(RwLock::new(ResourceManager::new()));
        let contract_events = Arc::new(ContractEventHub::from_env());
        let source_storage = Arc::new(SourceStorage::new().await?);

        let elasticsearch_url = std::env::var("ELASTICSEARCH_URL")
            .unwrap_or_else(|_| "http://localhost:9200".to_string());
        let search = Arc::new(SearchClient::new(&elasticsearch_url).expect("Search client init"));
        let pg_search = Arc::new(PostgresSearchService::new(db.clone()));

        // Initialize state monitor (optional)
        let state_monitor = match StateMonitorService::new(db.clone(), event_broadcaster.clone()) {
            Ok(service) => {
                info!("State monitor service initialized");
                Some(Arc::new(service))
            }
            Err(e) => {
                warn!("State monitor service not initialized: {}", e);
                None
            }
        };

        Ok(Self {
            db,
            started_at: Instant::now(),
            cache: Arc::new(CacheLayer::new(config).await),
            contract_events,
            registry,
            job_engine,
            is_shutting_down,
            health_monitor_status: HealthMonitorStatus::default(),
            auth_mgr,
            resource_mgr,
            source_storage,
            event_broadcaster,
            search,
            pg_search,
            ai_service,
            state_monitor,
            rate_limit_state,
            db_breaker,
            db_queue,
            feature_flags,
            encryption: Arc::new(EncryptionService::from_env()),
        })
    }
}
