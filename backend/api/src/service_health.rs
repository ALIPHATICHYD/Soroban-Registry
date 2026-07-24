//! Per-service health checks for the registry's core request paths
//! (publish, search, verification), exposed at `GET /health/services`.
//!
//! Unlike `/health/detailed` (which reports raw dependency status), this
//! module classifies each core service as healthy/degraded/unhealthy using
//! explicit latency and error-rate thresholds, so it can back uptime and
//! error-rate alerting (see observability/prometheus/alert_rules.yml).

use crate::metrics;
use crate::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// Dependency checks that take longer than this are still "up" but degraded.
const DEGRADED_LATENCY_MS: u128 = 200;
/// Dependency checks that take longer than this are treated as failed.
const CHECK_TIMEOUT: Duration = Duration::from_millis(500);

/// Verification failure ratio (of success+failure since process start) above
/// which the verification service is considered degraded / unhealthy.
const VERIFICATION_FAILURE_RATIO_WARN: f64 = 0.10;
const VERIFICATION_FAILURE_RATIO_CRIT: f64 = 0.30;
/// Pending verification jobs above which the queue is considered backed up.
const VERIFICATION_QUEUE_DEPTH_WARN: i64 = 50;
const VERIFICATION_QUEUE_DEPTH_CRIT: i64 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl ServiceStatus {
    /// Combines two statuses, keeping the more severe one.
    fn worst(self, other: Self) -> Self {
        use ServiceStatus::*;
        match (self, other) {
            (Unhealthy, _) | (_, Unhealthy) => Unhealthy,
            (Degraded, _) | (_, Degraded) => Degraded,
            (Healthy, Healthy) => Healthy,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            ServiceStatus::Healthy => "healthy",
            ServiceStatus::Degraded => "degraded",
            ServiceStatus::Unhealthy => "unhealthy",
        }
    }
}

/// Classifies a timed dependency check into a service status.
///
/// `failed` covers both connection errors and query errors; `timed_out`
/// covers checks that exceeded `CHECK_TIMEOUT`. Both are treated as
/// unhealthy since the dependency did not answer successfully in time.
fn classify_latency(elapsed_ms: u128, failed: bool, timed_out: bool) -> ServiceStatus {
    if failed || timed_out {
        ServiceStatus::Unhealthy
    } else if elapsed_ms > DEGRADED_LATENCY_MS {
        ServiceStatus::Degraded
    } else {
        ServiceStatus::Healthy
    }
}

/// Classifies verification health from cumulative success/failure counters
/// and the current queue depth. Returns the status plus the failure ratio
/// so callers can surface it for debugging.
fn classify_verification(success: u64, failure: u64, queue_depth: i64) -> (ServiceStatus, f64) {
    let total = success + failure;
    let failure_ratio = if total == 0 {
        0.0
    } else {
        failure as f64 / total as f64
    };

    let status = if failure_ratio >= VERIFICATION_FAILURE_RATIO_CRIT
        || queue_depth >= VERIFICATION_QUEUE_DEPTH_CRIT
    {
        ServiceStatus::Unhealthy
    } else if failure_ratio >= VERIFICATION_FAILURE_RATIO_WARN
        || queue_depth >= VERIFICATION_QUEUE_DEPTH_WARN
    {
        ServiceStatus::Degraded
    } else {
        ServiceStatus::Healthy
    };

    (status, failure_ratio)
}

async fn check_db_dependency(
    pool: &sqlx::PgPool,
    query: &str,
) -> (ServiceStatus, u128, Option<String>) {
    let start = Instant::now();
    let outcome = tokio::time::timeout(CHECK_TIMEOUT, sqlx::query(query).execute(pool)).await;
    let elapsed_ms = start.elapsed().as_millis();

    match outcome {
        Ok(Ok(_)) => (classify_latency(elapsed_ms, false, false), elapsed_ms, None),
        Ok(Err(e)) => (
            classify_latency(elapsed_ms, true, false),
            elapsed_ms,
            Some(e.to_string()),
        ),
        Err(_) => (
            classify_latency(elapsed_ms, false, true),
            elapsed_ms,
            Some("dependency check timed out".to_string()),
        ),
    }
}

fn service_json(
    status: ServiceStatus,
    latency_ms: u128,
    error: Option<String>,
    extra: Value,
) -> Value {
    let mut obj = json!({
        "status": status.as_str(),
        "latency_ms": latency_ms,
    });
    if let Some(err) = error {
        obj["error"] = json!(err);
    }
    if let Value::Object(extra_map) = extra {
        if let Value::Object(obj_map) = &mut obj {
            obj_map.extend(extra_map);
        }
    }
    obj
}

/// `GET /health/services` — reports the health of the registry's core
/// request paths (publish, search, verification) individually, so outages
/// or degradation in one subsystem can be alerted on and diagnosed without
/// digging through generic dependency checks.
#[utoipa::path(
    get,
    path = "/health/services",
    responses(
        (status = 200, description = "All core services healthy or degraded", body = Object),
        (status = 503, description = "One or more core services are unhealthy", body = Object)
    ),
    tag = "Observability"
)]
pub async fn health_check_services(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let now = chrono::Utc::now().to_rfc3339();

    if state.is_shutting_down.load(Ordering::SeqCst) {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "unhealthy",
                "timestamp": now,
                "reason": "shutting_down"
            })),
        );
    }

    // Publish depends on being able to write/read the registry's primary store.
    let (publish_status, publish_latency, publish_err) =
        check_db_dependency(&state.db, "SELECT 1 FROM contracts LIMIT 1").await;

    // Search currently runs through PostgreSQL full-text search on the same store;
    // checked separately so a slow search index doesn't get masked by a fast publish path.
    let (search_status, search_latency, search_err) = check_db_dependency(
        &state.db,
        "SELECT 1 FROM contracts WHERE search_vector IS NOT NULL LIMIT 1",
    )
    .await;

    // Verification health is derived from live counters rather than a synthetic
    // check, since a real verification run is too expensive to do on every probe.
    let verification_success = metrics::VERIFICATION_SUCCESS.get();
    let verification_failure = metrics::VERIFICATION_FAILURE.get();
    let verification_queue_depth = metrics::VERIFICATION_QUEUE_DEPTH.get();
    let (verification_status, verification_failure_ratio) = classify_verification(
        verification_success,
        verification_failure,
        verification_queue_depth,
    );

    let overall = publish_status
        .worst(search_status)
        .worst(verification_status);

    let status_code = match overall {
        ServiceStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
        ServiceStatus::Degraded | ServiceStatus::Healthy => StatusCode::OK,
    };

    (
        status_code,
        Json(json!({
            "status": overall.as_str(),
            "timestamp": now,
            "services": {
                "publish": service_json(publish_status, publish_latency, publish_err, json!({})),
                "search": service_json(search_status, search_latency, search_err, json!({})),
                "verification": service_json(
                    verification_status,
                    0,
                    None,
                    json!({
                        "queue_depth": verification_queue_depth,
                        "failure_ratio": verification_failure_ratio,
                    })
                ),
            }
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_latency_healthy_when_fast() {
        assert_eq!(classify_latency(10, false, false), ServiceStatus::Healthy);
    }

    #[test]
    fn classify_latency_degraded_when_slow_but_ok() {
        assert_eq!(
            classify_latency(DEGRADED_LATENCY_MS + 1, false, false),
            ServiceStatus::Degraded
        );
    }

    #[test]
    fn classify_latency_unhealthy_on_error() {
        assert_eq!(classify_latency(5, true, false), ServiceStatus::Unhealthy);
    }

    #[test]
    fn classify_latency_unhealthy_on_timeout() {
        assert_eq!(classify_latency(500, false, true), ServiceStatus::Unhealthy);
    }

    #[test]
    fn classify_verification_healthy_with_no_traffic() {
        let (status, ratio) = classify_verification(0, 0, 0);
        assert_eq!(status, ServiceStatus::Healthy);
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn classify_verification_healthy_below_thresholds() {
        let (status, ratio) = classify_verification(95, 5, 3);
        assert_eq!(status, ServiceStatus::Healthy);
        assert!((ratio - 0.05).abs() < 1e-9);
    }

    #[test]
    fn classify_verification_degraded_on_failure_ratio() {
        // 15% failures — above the 10% warn threshold, below the 30% critical one.
        let (status, _) = classify_verification(85, 15, 0);
        assert_eq!(status, ServiceStatus::Degraded);
    }

    #[test]
    fn classify_verification_degraded_on_queue_backlog() {
        let (status, _) = classify_verification(100, 0, VERIFICATION_QUEUE_DEPTH_WARN);
        assert_eq!(status, ServiceStatus::Degraded);
    }

    #[test]
    fn classify_verification_unhealthy_on_high_failure_ratio() {
        // 40% failures — a simulated verification outage.
        let (status, ratio) = classify_verification(60, 40, 0);
        assert_eq!(status, ServiceStatus::Unhealthy);
        assert!((ratio - 0.4).abs() < 1e-9);
    }

    #[test]
    fn classify_verification_unhealthy_on_severe_queue_backlog() {
        let (status, _) = classify_verification(100, 0, VERIFICATION_QUEUE_DEPTH_CRIT);
        assert_eq!(status, ServiceStatus::Unhealthy);
    }

    #[test]
    fn worst_status_picks_most_severe() {
        assert_eq!(
            ServiceStatus::Healthy.worst(ServiceStatus::Degraded),
            ServiceStatus::Degraded
        );
        assert_eq!(
            ServiceStatus::Degraded.worst(ServiceStatus::Unhealthy),
            ServiceStatus::Unhealthy
        );
        assert_eq!(
            ServiceStatus::Healthy.worst(ServiceStatus::Healthy),
            ServiceStatus::Healthy
        );
        assert_eq!(
            ServiceStatus::Unhealthy.worst(ServiceStatus::Healthy),
            ServiceStatus::Unhealthy
        );
    }

    use crate::cache::{CacheConfig, CacheLayer};
    use crate::contract_events::ContractEventHub;
    use crate::rate_limit::RateLimitState;
    use crate::search_client::SearchClient;
    use crate::search_postgres::PostgresSearchService;
    use prometheus::Registry;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, RwLock};

    /// Builds an `AppState` backed by a lazily-connected (never-touched) pool.
    /// Mirrors the harness in `metrics_handler.rs`'s tests. Suitable only for
    /// paths that short-circuit before hitting the database, such as the
    /// shutting-down check below.
    async fn test_state(shutting_down: bool) -> AppState {
        let registry = Registry::new_custom(Some("t".into()), None).unwrap();
        metrics::register_all(&registry).unwrap();
        let db = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost:5432/soroban_registry")
            .unwrap();
        let (job_engine, _rx) = soroban_batch::engine::JobEngine::new();
        let (event_broadcaster, _) = tokio::sync::broadcast::channel(100);
        AppState {
            db: db.clone(),
            started_at: Instant::now(),
            cache: Arc::new(CacheLayer::new(CacheConfig::default()).await),
            registry,
            job_engine: Arc::new(job_engine),
            is_shutting_down: Arc::new(AtomicBool::new(shutting_down)),
            health_monitor_status: crate::health_monitor::HealthMonitorStatus::default(),
            auth_mgr: Arc::new(RwLock::new(crate::auth::AuthManager::new(
                "test-secret-test-secret-test-se".to_string(),
            ))),
            resource_mgr: Arc::new(RwLock::new(crate::resource_tracking::ResourceManager::new())),
            contract_events: Arc::new(ContractEventHub::from_env()),
            source_storage: Arc::new(shared::source_storage::SourceStorage::new().await.unwrap()),
            event_broadcaster,
            search: Arc::new(SearchClient::new("http://localhost:9200").unwrap()),
            pg_search: Arc::new(PostgresSearchService::new(db)),
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
    async fn health_check_services_returns_unhealthy_when_shutting_down() {
        // A service mid-shutdown must fail fast without touching the database —
        // the lazily-connected pool here is never queried because the
        // shutting-down branch short-circuits first.
        let state = test_state(true).await;

        let (status, body) = health_check_services(State(state)).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body.0["status"], "unhealthy");
        assert_eq!(body.0["reason"], "shutting_down");
    }
}
