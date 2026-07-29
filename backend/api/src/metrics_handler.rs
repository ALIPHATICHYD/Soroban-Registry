use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;

use crate::metrics;
use crate::state::AppState;

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
    use axum::extract::State;
    use axum::response::IntoResponse;
    use prometheus::Registry;
    use std::sync::Arc;
    use std::time::Instant;

    fn test_state() -> AppState {
        let registry = Registry::new_custom(Some("test".into()), None).unwrap();
        metrics::register_all(&registry).unwrap();
        AppState {
            db: create_test_pool(),
            started_at: Instant::now(),
            cache: Arc::new(CacheLayer::new(CacheConfig::default())),
            registry,
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
        let state = test_state();
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
        let state = test_state();
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
