use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use crate::error::ApiError;
use crate::state::AppState;

pub async fn get_contract_resources(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mgr = state.resource_mgr.read().unwrap();
    match mgr.summary(&id) {
        Some(summary) => (StatusCode::OK, Json(summary)).into_response(),
        None => ApiError::not_found("RESOURCE_DATA_NOT_FOUND", "No resource data for contract")
            .with_details(serde_json::json!({ "contract_id": id }))
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthManager;
    use crate::cache::{CacheConfig, CacheLayer};
    use crate::contract_events::ContractEventHub;
    use crate::metrics;
    use crate::rate_limit::RateLimitState;
    use crate::resource_tracking::{ResourceManager, ResourceUsage};
    use crate::search_client::SearchClient;
    use crate::search_postgres::PostgresSearchService;
    use axum::extract::{Path, State};
    use axum::response::IntoResponse;
    use chrono::{TimeZone, Utc};
    use prometheus::Registry;
    use std::sync::{Arc, RwLock};
    use std::time::Instant;

    fn create_test_pool() -> sqlx::PgPool {
        sqlx::pool::PoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://localhost/test")
            .expect("lazy pool")
    }

    async fn test_state() -> AppState {
        let registry = Registry::new_custom(Some("test".into()), None).unwrap();
        metrics::register_all(&registry).unwrap();
        let (job_engine, _rx) = soroban_batch::engine::JobEngine::new();
        let (event_broadcaster, _) = tokio::sync::broadcast::channel(100);
        AppState {
            db: create_test_pool(),
            started_at: Instant::now(),
            cache: Arc::new(CacheLayer::new(CacheConfig::default()).await),
            registry,
            job_engine: Arc::new(job_engine),
            is_shutting_down: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            health_monitor_status: crate::health_monitor::HealthMonitorStatus::default(),
            resource_mgr: Arc::new(RwLock::new(ResourceManager::new())),
            auth_mgr: Arc::new(RwLock::new(AuthManager::new(
                "test-secret-test-secret-test-se".to_string(),
            ))),
            contract_events: Arc::new(ContractEventHub::from_env()),
            source_storage: Arc::new(shared::source_storage::SourceStorage::new().await.unwrap()),
            event_broadcaster,
            search: Arc::new(SearchClient::new("http://localhost:9200").unwrap()),
            pg_search: Arc::new(PostgresSearchService::new(create_test_pool())),
            ai_service: None,
            state_monitor: None,
            rate_limit_state: Arc::new(RateLimitState::from_env()),
            db_breaker: Arc::new(crate::db_resilience::CircuitBreaker::new(
                3,
                std::time::Duration::from_secs(10),
            )),
            db_queue: Arc::new(crate::db_resilience::DbQueue::new(
                10,
                10,
                std::time::Duration::from_secs(1),
            )),
            feature_flags: Arc::new(crate::feature_flags::FeatureFlagManager::new()),
            encryption: Arc::new(crate::crypto::EncryptionService::disabled()),
        }
    }

    #[tokio::test]
    async fn returns_forecast_payload_for_alias_route() {
        let state = test_state().await;
        {
            let base = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
            let mut mgr = state.resource_mgr.write().unwrap();
            for i in 0..48_u64 {
                let _ = mgr.record_usage(
                    "c-resource",
                    ResourceUsage {
                        cpu_instructions: 2_000_000 + i * 1_500_000,
                        mem_bytes: 4_000_000 + i * 200_000,
                        storage_bytes: i * 1024,
                        timestamp: base + chrono::Duration::hours(i as i64),
                    },
                );
            }
        }

        let resp = get_contract_resources(State(state), Path("c-resource".to_string()))
            .await
            .into_response();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["contract_id"], "c-resource");
        assert!(json["history"].as_array().unwrap().len() >= 2);
        assert!(json["forecast"]["cpu_exhaustion_ts"].is_string());
        assert!(json["forecast"]["cpu_exhaustion_ts_p90"].is_string());
        assert!(json["forecast"]["mem_exhaustion_ts_p90"].is_string());
    }
}
