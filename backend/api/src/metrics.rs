use once_cell::sync::Lazy;
use prometheus::{
    opts, Encoder, GaugeVec, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge,
    IntGaugeVec, Registry, TextEncoder,
};

#[allow(dead_code)]
pub static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

macro_rules! counter_vec {
    ($name:expr, $help:expr, $labels:expr) => {
        Lazy::new(|| IntCounterVec::new(opts!($name, $help), $labels).unwrap())
    };
}
macro_rules! histogram_vec {
    ($name:expr, $help:expr, $labels:expr) => {
        Lazy::new(|| {
            HistogramVec::new(
                HistogramOpts::new($name, $help).buckets(LATENCY_BUCKETS.to_vec()),
                $labels,
            )
            .unwrap()
        })
    };
}
macro_rules! counter {
    ($name:expr, $help:expr) => {
        Lazy::new(|| IntCounter::new($name, $help).unwrap())
    };
}
macro_rules! gauge {
    ($name:expr, $help:expr) => {
        Lazy::new(|| IntGauge::new($name, $help).unwrap())
    };
}
macro_rules! gauge_vec {
    ($name:expr, $help:expr, $labels:expr) => {
        Lazy::new(|| IntGaugeVec::new(opts!($name, $help), $labels).unwrap())
    };
}
macro_rules! gauge_f64_vec {
    ($name:expr, $help:expr, $labels:expr) => {
        Lazy::new(|| GaugeVec::new(opts!($name, $help), $labels).unwrap())
    };
}

const LATENCY_BUCKETS: [f64; 14] = [
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0,
];

// ── HTTP ────────────────────────────────────────────────────────────────────
pub static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = counter_vec!(
    "http_requests_total",
    "Total HTTP requests",
    &["method", "path", "status"]
);
pub static HTTP_REQUEST_DURATION: Lazy<HistogramVec> = histogram_vec!(
    "http_request_duration_seconds",
    "HTTP request latency",
    &["method", "path"]
);
pub static HTTP_IN_FLIGHT: Lazy<IntGauge> =
    gauge!("http_requests_in_flight", "In-flight HTTP requests");
pub static HTTP_REQUEST_SIZE: Lazy<HistogramVec> = histogram_vec!(
    "http_request_size_bytes",
    "HTTP request body size",
    &["method"]
);
pub static HTTP_RESPONSE_SIZE: Lazy<HistogramVec> = histogram_vec!(
    "http_response_size_bytes",
    "HTTP response body size",
    &["method"]
);

// ── Contracts ───────────────────────────────────────────────────────────────
pub static CONTRACTS_TOTAL: Lazy<IntGauge> =
    gauge!("contracts_total", "Total registered contracts");
pub static CONTRACTS_PUBLISHED: Lazy<IntCounter> =
    counter!("contracts_published_total", "Contracts published");
pub static CONTRACTS_VERIFIED: Lazy<IntCounter> =
    counter!("contracts_verified_total", "Contracts verified");
pub static CONTRACTS_PER_PUBLISHER: Lazy<IntGaugeVec> = gauge_vec!(
    "contracts_per_publisher",
    "Contracts per publisher",
    &["publisher"]
);
pub static CONTRACT_DEPLOY_TOTAL: Lazy<IntCounter> =
    counter!("contract_deploy_total", "Contract deployments");
pub static CONTRACT_DEPLOY_ERRORS: Lazy<IntCounter> = counter!(
    "contract_deploy_errors_total",
    "Failed contract deployments"
);
pub static CONTRACT_STATE_READS: Lazy<IntCounter> =
    counter!("contract_state_reads_total", "State reads");
pub static CONTRACT_STATE_WRITES: Lazy<IntCounter> =
    counter!("contract_state_writes_total", "State writes");
pub static CONTRACT_SIZE_BYTES: Lazy<HistogramVec> = histogram_vec!(
    "contract_size_bytes",
    "Contract binary size",
    &["publisher"]
);
pub static CONTRACTS_BY_CATEGORY: Lazy<IntCounterVec> = counter_vec!(
    "contracts_by_category_total",
    "Contracts by category",
    &["category"]
);

// ── Verification ────────────────────────────────────────────────────────────
pub static VERIFICATION_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    HistogramVec::new(
        HistogramOpts::new("verification_latency_seconds", "Verification latency")
            .buckets(vec![0.01, 0.05, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 10.0]),
        &["result"],
    )
    .unwrap()
});
pub static VERIFICATION_QUEUE_DEPTH: Lazy<IntGauge> =
    gauge!("verification_queue_depth", "Verification queue depth");
pub static VERIFICATION_SUCCESS: Lazy<IntCounter> =
    counter!("verification_success_total", "Successful verifications");
pub static VERIFICATION_FAILURE: Lazy<IntCounter> =
    counter!("verification_failure_total", "Failed verifications");

// ── Database ────────────────────────────────────────────────────────────────
pub static DB_QUERY_DURATION: Lazy<HistogramVec> = histogram_vec!(
    "db_query_duration_seconds",
    "Database query latency",
    &["query"]
);
pub static DB_CONNECTIONS_ACTIVE: Lazy<IntGauge> =
    gauge!("db_connections_active", "Active DB connections");
pub static DB_CONNECTIONS_IDLE: Lazy<IntGauge> =
    gauge!("db_connections_idle", "Idle DB connections");
pub static DB_QUERY_ERRORS: Lazy<IntCounter> = counter!("db_query_errors_total", "DB query errors");
// Issue #887: application-side query observation counters.
pub static DB_QUERIES_OBSERVED: Lazy<IntCounter> = counter!(
    "db_queries_observed_total",
    "Queries observed by the app-side analyzer"
);
pub static DB_SLOW_QUERIES: Lazy<IntCounter> = counter!(
    "db_slow_queries_total",
    "Queries exceeding the slow threshold"
);
pub static DB_TRANSACTIONS_TOTAL: Lazy<IntCounter> =
    counter!("db_transactions_total", "Total DB transactions");
pub static DB_POOL_SIZE: Lazy<IntGauge> = gauge!("db_pool_size", "DB connection pool size");
pub static DB_CONNECTION_WAIT_MS: Lazy<HistogramVec> = Lazy::new(|| {
    HistogramVec::new(
        HistogramOpts::new(
            "db_connection_wait_milliseconds",
            "DB connection acquisition latency",
        )
        .buckets(vec![
            1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0,
        ]),
        &["pool"],
    )
    .unwrap()
});
pub static DB_POOL_TIMEOUTS: Lazy<IntCounter> =
    counter!("db_pool_timeouts_total", "DB pool acquisition timeouts");
pub static DB_POOL_UTILIZATION: Lazy<GaugeVec> = gauge_f64_vec!(
    "db_pool_utilization",
    "DB pool utilization ratio",
    &["pool"]
);
pub static DB_REPLICATION_LAG_MS: Lazy<IntGauge> = gauge!(
    "db_replication_lag_ms",
    "Replica replay lag in milliseconds"
);
pub static DB_REPLICATION_WAL_LAG_BYTES: Lazy<IntGauge> = gauge!(
    "db_replication_wal_lag_bytes",
    "Estimated WAL lag between the primary and replica in bytes"
);
pub static DB_REPLICATION_HEALTH: Lazy<IntGauge> = gauge!(
    "db_replication_health",
    "Replication health state (1=healthy, 0=unhealthy)"
);
pub static DB_REPLICATION_CHECKS_TOTAL: Lazy<IntCounter> = counter!(
    "db_replication_checks_total",
    "Replication health checks performed"
);
pub static DB_REPLICATION_CHECK_FAILURES_TOTAL: Lazy<IntCounter> = counter!(
    "db_replication_check_failures_total",
    "Replication health checks that failed"
);
pub static SEARCH_QUERY_DURATION: Lazy<HistogramVec> = histogram_vec!(
    "search_query_duration_seconds",
    "Search query latency",
    &["type"]
);
pub static SEARCH_SLOW_QUERIES: Lazy<IntCounterVec> = counter_vec!(
    "search_slow_queries_total",
    "Search queries slower than the threshold",
    &["type"]
);

// ── Cache ───────────────────────────────────────────────────────────────────
pub static CACHE_HITS: Lazy<IntCounter> = counter!("cache_hits_total", "Cache hits");
pub static CACHE_MISSES: Lazy<IntCounter> = counter!("cache_misses_total", "Cache misses");
pub static CACHE_EVICTIONS: Lazy<IntCounter> = counter!("cache_evictions_total", "Cache evictions");
pub static CACHE_SIZE_BYTES: Lazy<IntGauge> = gauge!("cache_size_bytes", "Cache size in bytes");
pub static CACHE_ENTRIES: Lazy<IntGauge> = gauge!("cache_entries", "Number of cached entries");

pub static ABI_CACHE_HITS: Lazy<IntCounter> = counter!("abi_cache_hits_total", "ABI cache hits");
pub static ABI_CACHE_MISSES: Lazy<IntCounter> =
    counter!("abi_cache_misses_total", "ABI cache misses");
pub static VERIFICATION_CACHE_HITS: Lazy<IntCounter> =
    counter!("verification_cache_hits_total", "Verification cache hits");
pub static VERIFICATION_CACHE_MISSES: Lazy<IntCounter> = counter!(
    "verification_cache_misses_total",
    "Verification cache misses"
);
pub static CONTRACTS_CACHE_HITS: Lazy<IntCounter> =
    counter!("contracts_cache_hits_total", "Contracts cache hits");
pub static CONTRACTS_CACHE_MISSES: Lazy<IntCounter> =
    counter!("contracts_cache_misses_total", "Contracts cache misses");

pub static REDIS_CACHE_HITS: Lazy<IntCounter> =
    counter!("redis_cache_hits_total", "Redis cache hits");
pub static REDIS_CACHE_MISSES: Lazy<IntCounter> =
    counter!("redis_cache_misses_total", "Redis cache misses");

// ── Resources ────────────────────────────────────────────────────────────────────
pub static RESOURCE_RECORDINGS: Lazy<IntCounter> =
    counter!("resource_recordings_total", "Resource usage recordings");
pub static RESOURCE_ALERTS_FIRED: Lazy<IntCounter> =
    counter!("resource_alerts_total", "Resource alerts fired");
pub static RESOURCE_FORECAST_RUNS: Lazy<IntCounter> = counter!(
    "resource_forecast_runs_total",
    "Resource forecast computations"
);

// ── Migration ───────────────────────────────────────────────────────────────
pub static MIGRATION_TOTAL: Lazy<IntCounter> = counter!("migration_total", "Total migrations");
pub static MIGRATION_FAILURES: Lazy<IntCounter> =
    counter!("migration_failures_total", "Migration failures");
pub static MIGRATION_DURATION: Lazy<HistogramVec> = histogram_vec!(
    "migration_duration_seconds",
    "Migration duration",
    &["status"]
);

// ── Canary ──────────────────────────────────────────────────────────────────
pub static CANARY_ACTIVE: Lazy<IntGauge> =
    gauge!("canary_deployments_active", "Active canary deployments");
pub static CANARY_ROLLBACKS: Lazy<IntCounter> =
    counter!("canary_rollbacks_total", "Canary rollbacks");
pub static CANARY_PROMOTIONS: Lazy<IntCounter> =
    counter!("canary_promotions_total", "Canary promotions");

// ── AB Test ─────────────────────────────────────────────────────────────────
pub static AB_TESTS_ACTIVE: Lazy<IntGauge> = gauge!("ab_tests_active", "Active AB tests");
pub static AB_TEST_IMPRESSIONS: Lazy<IntCounterVec> = counter_vec!(
    "ab_test_impressions_total",
    "AB test impressions",
    &["test_id", "variant"]
);
pub static AB_TEST_CONVERSIONS: Lazy<IntCounterVec> = counter_vec!(
    "ab_test_conversions_total",
    "AB test conversions",
    &["test_id", "variant"]
);

// ── Multisig ────────────────────────────────────────────────────────────────
pub static MULTISIG_PROPOSALS: Lazy<IntCounter> =
    counter!("multisig_proposals_total", "Multisig proposals created");
pub static MULTISIG_SIGNATURES: Lazy<IntCounter> =
    counter!("multisig_signatures_total", "Multisig signatures collected");
pub static MULTISIG_EXECUTIONS: Lazy<IntCounter> =
    counter!("multisig_executions_total", "Multisig executions completed");
pub static MULTISIG_REJECTIONS: Lazy<IntCounter> =
    counter!("multisig_rejections_total", "Multisig proposals rejected");

// ── Job Queue ───────────────────────────────────────────────────────────────
pub static JOB_QUEUE_DEPTH: Lazy<IntGaugeVec> = gauge_vec!(
    "job_queue_depth",
    "Current number of jobs in the queue",
    &["type", "status"]
);
pub static JOB_PROCESSING_DURATION: Lazy<HistogramVec> = histogram_vec!(
    "job_processing_duration_seconds",
    "Job processing latency",
    &["type"]
);
pub static JOB_FAILURES_TOTAL: Lazy<IntCounterVec> = counter_vec!(
    "job_failures_total",
    "Total number of job failures",
    &["type"]
);

// ── System ──────────────────────────────────────────────────────────────────
pub static PROCESS_START_TIME: Lazy<IntGauge> =
    gauge!("process_start_time_seconds", "Process start time");
pub static BUILD_INFO: Lazy<IntGaugeVec> =
    gauge_vec!("build_info", "Build information", &["version", "commit"]);

// ── Client-side breaker metrics ─────────────────────────────────────────────
pub static CLIENT_BREAKER_OPEN: Lazy<IntGaugeVec> = gauge_vec!(
    "client_breaker_open",
    "Client-side circuit breaker open state (1=open, 0=closed)",
    &["endpoint"]
);

// ── Database Resilience Metrics ─────────────────────────────────────────────
pub static DB_RESILIENCE_QUEUE_DEPTH: Lazy<IntGauge> = gauge!(
    "db_resilience_queue_depth",
    "Current requests waiting in the DB connection queue"
);
pub static DB_RESILIENCE_ACTIVE_REQS: Lazy<IntGauge> = gauge!(
    "db_resilience_active_reqs",
    "Active requests holding a DB concurrency permit"
);
pub static DB_RESILIENCE_BREAKER_STATE: Lazy<IntGauge> = gauge!(
    "db_resilience_breaker_state",
    "Database circuit breaker state (0=Closed, 1=Open, 2=HalfOpen)"
);
pub static DB_RESILIENCE_BREAKER_TRIPS: Lazy<IntCounter> = counter!(
    "db_resilience_breaker_trips_total",
    "Total database circuit breaker trips"
);
pub static DB_RESILIENCE_REJECTIONS: Lazy<IntCounterVec> = counter_vec!(
    "db_resilience_rejections_total",
    "Total requests rejected by database resilience",
    &["reason"]
);

// ── SLO ─────────────────────────────────────────────────────────────────────
pub static SLO_ERROR_BUDGET: Lazy<GaugeVec> = gauge_f64_vec!(
    "slo_error_budget_remaining",
    "SLO error budget remaining",
    &["slo"]
);
pub static SLO_BURN_RATE: Lazy<GaugeVec> =
    gauge_f64_vec!("slo_burn_rate", "SLO burn rate", &["slo"]);
pub static SLO_AVAILABILITY: Lazy<GaugeVec> = gauge_f64_vec!(
    "slo_availability",
    "Service availability ratio",
    &["window"]
);

// ── Patch ───────────────────────────────────────────────────────────────────
pub static PATCHES_CREATED: Lazy<IntCounter> =
    counter!("patches_created_total", "Security patches created");
pub static PATCHES_APPLIED: Lazy<IntCounter> =
    counter!("patches_applied_total", "Security patches applied");
pub static PATCHES_FAILED: Lazy<IntCounter> =
    counter!("patches_failed_total", "Security patches failed");

// ── Publisher ───────────────────────────────────────────────────────────────
pub static PUBLISHERS_TOTAL: Lazy<IntGauge> =
    gauge!("publishers_total", "Total registered publishers");
pub static PUBLISHER_REGISTRATIONS: Lazy<IntCounter> =
    counter!("publisher_registrations_total", "Publisher registrations");

pub fn register_all(r: &Registry) -> prometheus::Result<()> {
    r.register(Box::new(HTTP_REQUESTS_TOTAL.clone()))?;
    r.register(Box::new(HTTP_REQUEST_DURATION.clone()))?;
    r.register(Box::new(HTTP_IN_FLIGHT.clone()))?;
    r.register(Box::new(HTTP_REQUEST_SIZE.clone()))?;
    r.register(Box::new(HTTP_RESPONSE_SIZE.clone()))?;
    r.register(Box::new(CONTRACTS_TOTAL.clone()))?;
    r.register(Box::new(CONTRACTS_PUBLISHED.clone()))?;
    r.register(Box::new(CONTRACTS_VERIFIED.clone()))?;
    r.register(Box::new(CONTRACTS_PER_PUBLISHER.clone()))?;
    r.register(Box::new(CONTRACT_DEPLOY_TOTAL.clone()))?;
    r.register(Box::new(CONTRACT_DEPLOY_ERRORS.clone()))?;
    r.register(Box::new(CONTRACT_STATE_READS.clone()))?;
    r.register(Box::new(CONTRACT_STATE_WRITES.clone()))?;
    r.register(Box::new(CONTRACT_SIZE_BYTES.clone()))?;
    r.register(Box::new(CONTRACTS_BY_CATEGORY.clone()))?;
    r.register(Box::new(VERIFICATION_LATENCY.clone()))?;
    r.register(Box::new(VERIFICATION_QUEUE_DEPTH.clone()))?;
    r.register(Box::new(VERIFICATION_SUCCESS.clone()))?;
    r.register(Box::new(VERIFICATION_FAILURE.clone()))?;
    r.register(Box::new(DB_QUERY_DURATION.clone()))?;
    r.register(Box::new(DB_CONNECTIONS_ACTIVE.clone()))?;
    r.register(Box::new(DB_CONNECTIONS_IDLE.clone()))?;
    r.register(Box::new(DB_QUERY_ERRORS.clone()))?;
    r.register(Box::new(DB_QUERIES_OBSERVED.clone()))?;
    r.register(Box::new(DB_SLOW_QUERIES.clone()))?;
    r.register(Box::new(DB_TRANSACTIONS_TOTAL.clone()))?;
    r.register(Box::new(DB_POOL_SIZE.clone()))?;
    r.register(Box::new(DB_CONNECTION_WAIT_MS.clone()))?;
    r.register(Box::new(DB_POOL_TIMEOUTS.clone()))?;
    r.register(Box::new(DB_POOL_UTILIZATION.clone()))?;
    r.register(Box::new(DB_REPLICATION_LAG_MS.clone()))?;
    r.register(Box::new(DB_REPLICATION_WAL_LAG_BYTES.clone()))?;
    r.register(Box::new(DB_REPLICATION_HEALTH.clone()))?;
    r.register(Box::new(DB_REPLICATION_CHECKS_TOTAL.clone()))?;
    r.register(Box::new(DB_REPLICATION_CHECK_FAILURES_TOTAL.clone()))?;
    r.register(Box::new(SEARCH_QUERY_DURATION.clone()))?;
    r.register(Box::new(SEARCH_SLOW_QUERIES.clone()))?;

    r.register(Box::new(CACHE_HITS.clone()))?;
    r.register(Box::new(CACHE_MISSES.clone()))?;
    r.register(Box::new(CACHE_EVICTIONS.clone()))?;
    r.register(Box::new(CACHE_SIZE_BYTES.clone()))?;
    r.register(Box::new(CACHE_ENTRIES.clone()))?;
    r.register(Box::new(ABI_CACHE_HITS.clone()))?;
    r.register(Box::new(ABI_CACHE_MISSES.clone()))?;
    r.register(Box::new(VERIFICATION_CACHE_HITS.clone()))?;
    r.register(Box::new(VERIFICATION_CACHE_MISSES.clone()))?;
    r.register(Box::new(CONTRACTS_CACHE_HITS.clone()))?;
    r.register(Box::new(CONTRACTS_CACHE_MISSES.clone()))?;
    r.register(Box::new(REDIS_CACHE_HITS.clone()))?;
    r.register(Box::new(REDIS_CACHE_MISSES.clone()))?;
    r.register(Box::new(RESOURCE_RECORDINGS.clone()))?;
    r.register(Box::new(RESOURCE_ALERTS_FIRED.clone()))?;
    r.register(Box::new(RESOURCE_FORECAST_RUNS.clone()))?;
    r.register(Box::new(MIGRATION_TOTAL.clone()))?;
    r.register(Box::new(MIGRATION_FAILURES.clone()))?;
    r.register(Box::new(MIGRATION_DURATION.clone()))?;
    r.register(Box::new(CANARY_ACTIVE.clone()))?;
    r.register(Box::new(CANARY_ROLLBACKS.clone()))?;
    r.register(Box::new(CANARY_PROMOTIONS.clone()))?;
    r.register(Box::new(AB_TESTS_ACTIVE.clone()))?;
    r.register(Box::new(AB_TEST_IMPRESSIONS.clone()))?;
    r.register(Box::new(AB_TEST_CONVERSIONS.clone()))?;
    r.register(Box::new(MULTISIG_PROPOSALS.clone()))?;
    r.register(Box::new(MULTISIG_SIGNATURES.clone()))?;
    r.register(Box::new(MULTISIG_EXECUTIONS.clone()))?;
    r.register(Box::new(MULTISIG_REJECTIONS.clone()))?;
    r.register(Box::new(PROCESS_START_TIME.clone()))?;
    r.register(Box::new(BUILD_INFO.clone()))?;
    r.register(Box::new(CLIENT_BREAKER_OPEN.clone()))?;
    r.register(Box::new(DB_RESILIENCE_QUEUE_DEPTH.clone()))?;
    r.register(Box::new(DB_RESILIENCE_ACTIVE_REQS.clone()))?;
    r.register(Box::new(DB_RESILIENCE_BREAKER_STATE.clone()))?;
    r.register(Box::new(DB_RESILIENCE_BREAKER_TRIPS.clone()))?;
    r.register(Box::new(DB_RESILIENCE_REJECTIONS.clone()))?;
    r.register(Box::new(SLO_ERROR_BUDGET.clone()))?;
    r.register(Box::new(SLO_BURN_RATE.clone()))?;
    r.register(Box::new(SLO_AVAILABILITY.clone()))?;
    r.register(Box::new(PATCHES_CREATED.clone()))?;
    r.register(Box::new(PATCHES_APPLIED.clone()))?;
    r.register(Box::new(PATCHES_FAILED.clone()))?;
    r.register(Box::new(PUBLISHERS_TOTAL.clone()))?;
    r.register(Box::new(PUBLISHER_REGISTRATIONS.clone()))?;
    r.register(Box::new(JOB_QUEUE_DEPTH.clone()))?;
    r.register(Box::new(JOB_PROCESSING_DURATION.clone()))?;
    r.register(Box::new(JOB_FAILURES_TOTAL.clone()))?;
    Ok(())
}

pub fn gather_metrics(r: &Registry) -> String {
    let encoder = TextEncoder::new();
    let families = r.gather();
    let mut buf = Vec::new();
    encoder.encode(&families, &mut buf).unwrap_or_default();
    String::from_utf8(buf).unwrap_or_default()
}

pub fn observe_http(method: &str, path: &str, status: u16, duration_secs: f64) {
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, path, &status.to_string()])
        .inc();
    HTTP_REQUEST_DURATION
        .with_label_values(&[method, path])
        .observe(duration_secs);
}

pub fn observe_verification_latency(result: &str, duration_secs: f64) {
    VERIFICATION_LATENCY
        .with_label_values(&[result])
        .observe(duration_secs);
    match result {
        "success" => VERIFICATION_SUCCESS.inc(),
        _ => VERIFICATION_FAILURE.inc(),
    }
}

#[allow(dead_code)]
pub fn set_contracts_per_publisher(publisher: &str, count: i64) {
    CONTRACTS_PER_PUBLISHER
        .with_label_values(&[publisher])
        .set(count);
}

#[allow(dead_code)]
pub fn observe_db_query(query: &str, duration_secs: f64) {
    DB_QUERY_DURATION
        .with_label_values(&[query])
        .observe(duration_secs);
    DB_TRANSACTIONS_TOTAL.inc();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_registry() -> Registry {
        let r = Registry::new_custom(Some("t".into()), None).unwrap();
        register_all(&r).unwrap();
        r
    }

    #[test]
    fn test_http_request_counter() {
        let r = fresh_registry();
        HTTP_REQUESTS_TOTAL
            .with_label_values(&["GET", "/health", "200"])
            .inc();
        HTTP_REQUESTS_TOTAL
            .with_label_values(&["GET", "/health", "200"])
            .inc();
        let out = gather_metrics(&r);
        assert!(out.contains("http_requests_total"));
    }

    #[test]
    fn test_verification_latency_observe() {
        let r = fresh_registry();
        observe_verification_latency("success", 0.42);
        observe_verification_latency("failure", 1.5);
        let out = gather_metrics(&r);
        assert!(out.contains("verification_latency_seconds"));
        assert!(out.contains("verification_success_total"));
        assert!(out.contains("verification_failure_total"));
    }

    #[test]
    fn test_contracts_per_publisher() {
        let r = fresh_registry();
        set_contracts_per_publisher("alice", 5);
        set_contracts_per_publisher("bob", 3);
        let out = gather_metrics(&r);
        assert!(out.contains("contracts_per_publisher"));
        assert!(out.contains("alice"));
        assert!(out.contains("bob"));
    }

    #[test]
    fn test_db_query_observation() {
        let r = fresh_registry();
        observe_db_query("select_contracts", 0.012);
        let out = gather_metrics(&r);
        assert!(out.contains("db_query_duration_seconds"));
        assert!(out.contains("db_transactions_total"));
    }

    #[test]
    fn test_gather_returns_valid_prometheus_format() {
        let r = fresh_registry();
        CONTRACTS_PUBLISHED.inc();
        let out = gather_metrics(&r);
        assert!(out.contains("# HELP"));
        assert!(out.contains("# TYPE"));
        assert!(out.contains("contracts_published_total"));
    }

    #[test]
    fn test_at_least_45_metric_families() {
        let r = fresh_registry();
        CONTRACTS_PUBLISHED.inc();
        observe_http("GET", "/test", 200, 0.01);
        observe_verification_latency("success", 0.1);
        set_contracts_per_publisher("x", 1);
        observe_db_query("q", 0.001);
        let families = r.gather();
        assert!(
            families.len() >= 45,
            "expected ≥45 metric families, got {}",
            families.len()
        );
    }

    #[test]
    fn test_observe_http_records_duration() {
        let _r = fresh_registry();
        observe_http("POST", "/api/contracts", 201, 0.055);
        let sample_count = HTTP_REQUEST_DURATION
            .with_label_values(&["POST", "/api/contracts"])
            .get_sample_count();
        assert!(sample_count >= 1);
    }
}
