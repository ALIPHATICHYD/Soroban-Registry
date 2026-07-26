use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;

use crate::metrics;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/metrics",
    responses(
        (status = 200, description = "Prometheus metrics", body = String)
    ),
    tag = "Observability"
)]
pub async fn metrics_endpoint(State(state): State<AppState>) -> impl IntoResponse {
    let body = metrics::gather_metrics(&state.registry);
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{CacheConfig, CacheLayer};
    use crate::contract_events::ContractEventHub;
    use crate::rate_limit::RateLimitState;
    use crate::search_client::SearchClient;
    use crate::search_postgres::PostgresSearchService;
    use axum::extract::State;
    use axum::response::IntoResponse;
    use prometheus::Registry;
    use std::sync::{Arc, RwLock};
    use std::time::Instant;

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
            is_shutting_down: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            health_monitor_status: crate::health_monitor::HealthMonitorStatus::default(),
            auth_mgr: Arc::new(RwLock::new(crate::auth::AuthManager::new(
                "test-secret-test-secret-test-se".to_string(),
            ))),
            resource_mgr: Arc::new(RwLock::new(crate::resource_tracking::ResourceManager::new())),
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

    fn create_test_pool() -> sqlx::PgPool {
        sqlx::pool::PoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://localhost/test")
            .expect("lazy pool")
    }

    #[tokio::test]
    async fn test_metrics_endpoint_returns_200() {
        let state = test_state().await;
        let resp = metrics_endpoint(State(state)).await.into_response();

        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(ct.contains("text/plain"));
    }

    #[tokio::test]
    async fn test_metrics_endpoint_contains_metric_families() {
        let state = test_state().await;
        metrics::CONTRACTS_PUBLISHED.inc();
        metrics::observe_http("GET", "/health", 200, 0.001);

        let resp = metrics_endpoint(State(state)).await.into_response();

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("http_requests_total"));
        assert!(text.contains("contracts_published_total"));
        assert!(text.contains("# TYPE"));
    }
}
// TODO: resource tracking metrics
