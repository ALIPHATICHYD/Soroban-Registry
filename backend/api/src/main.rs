#![warn(unused_imports)]

use anyhow::Result;
use axum::extract::{MatchedPath, Request, State};
use axum::http::StatusCode;
use axum::middleware;
use axum::response::Response;
use prometheus::Registry;
use sqlx::postgres::PgPoolOptions;
use sqlx::ConnectOptions;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

use api::aggregation;
use api::ai::service::AIService;
use api::config;
use api::db_monitoring;
use api::error::ApiError;
use api::graphql;
use api::handlers;
use api::health_monitor;
use api::metrics;
use api::migration_handlers;
use api::rate_limit;
use api::rate_limit::RateLimitState;
use api::request_tracing;
use api::routes;
use api::security::WebSecurityConfig;
use api::state::AppState;
use api::state_monitor::StateMonitorService;
use api::validation;
use api::webhook_delivery;

async fn http_metrics_middleware(
    State(state): State<AppState>,
    req: Request,
    next: middleware::Next,
) -> Result<Response, ApiError> {
    if state.is_shutting_down.load(Ordering::Relaxed) {
        return Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "SERVICE_UNAVAILABLE",
            "Service is shutting down and temporarily unavailable",
        ));
    }
    let method = req.method().as_str().to_owned();
    // Use the matched route template (e.g. "/api/contracts/:id") rather than the
    // concrete URI so per-id paths don't explode the metric label cardinality.
    let path = req
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map(|p| p.as_str().to_owned())
        .unwrap_or_else(|| req.uri().path().to_owned());
    let request_size = content_length_bytes(req.headers());

    metrics::HTTP_IN_FLIGHT.inc();
    let start = std::time::Instant::now();
    let res = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();
    metrics::HTTP_IN_FLIGHT.dec();

    metrics::observe_http(&method, &path, res.status().as_u16(), elapsed);
    if let Some(bytes) = request_size {
        metrics::HTTP_REQUEST_SIZE
            .with_label_values(&[&method])
            .observe(bytes);
    }
    if let Some(bytes) = content_length_bytes(res.headers()) {
        metrics::HTTP_RESPONSE_SIZE
            .with_label_values(&[&method])
            .observe(bytes);
    }
    Ok(res)
}

fn content_length_bytes(headers: &axum::http::HeaderMap) -> Option<f64> {
    headers
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<f64>().ok())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load and validate configuration (#768)
    let config = config::load_config()?;

    // Initialize structured JSON tracing (ELK/Splunk compatible)
    request_tracing::init_json_tracing();

    let logical_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    // Issue #876: Connection pool configuration
    let min_pool_size: u32 = std::env::var("DB_MIN_POOL_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let max_pool_size: u32 = std::env::var("DB_MAX_POOL_SIZE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or_else(|| (logical_cores * 2).max(10) as u32)
        .min(50); // hard cap per spec

    let acquire_timeout_secs: u64 = std::env::var("DB_ACQUIRE_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let idle_timeout_secs: u64 = std::env::var("DB_IDLE_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600); // 10 minutes

    let max_lifetime_secs: u64 = std::env::var("DB_MAX_LIFETIME_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800); // 30 minutes

    let query_timeout_ms: u64 = std::env::var("DB_QUERY_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30000); // 30 seconds

    let statement_cache_capacity: usize = std::env::var("DB_STATEMENT_CACHE_CAPACITY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(250);

    let slow_query_threshold_ms: f64 = std::env::var("DB_SLOW_QUERY_THRESHOLD_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100.0);

    tracing::info!(
        min_pool_size,
        max_pool_size,
        acquire_timeout_secs,
        idle_timeout_secs,
        max_lifetime_secs,
        query_timeout_ms,
        statement_cache_capacity,
        slow_query_threshold_ms,
        logical_cores,
        "Initializing database connection pool"
    );

    // Prepared statement cache + query timeout (statement_timeout) per connection
    let connect_options = config
        .database_url
        .parse::<sqlx::postgres::PgConnectOptions>()?
        .statement_cache_capacity(statement_cache_capacity)
        .log_slow_statements(
            log::LevelFilter::Warn,
            Duration::from_millis(slow_query_threshold_ms as u64),
        )
        .options([("statement_timeout", query_timeout_ms)]);

    let pool = PgPoolOptions::new()
        .min_connections(min_pool_size)
        .max_connections(max_pool_size)
        .acquire_timeout(Duration::from_secs(acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(idle_timeout_secs))
        .max_lifetime(Duration::from_secs(max_lifetime_secs))
        .connect_with(connect_options)
        .await?;

    // Run migrations (skip if SKIP_MIGRATIONS=true, useful when migrations were applied manually)
    let skip_migrations = std::env::var("SKIP_MIGRATIONS")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    if skip_migrations {
        tracing::info!("Skipping automatic migrations (SKIP_MIGRATIONS=true)");
    } else {
        sqlx::migrate!("../../database/migrations")
            .run(&pool)
            .await?;
    }

    tracing::info!("Database connected and migrations applied");

    // Check migration versioning state on startup (Issue #252)
    migration_handlers::check_migrations_on_startup(&pool).await;

    // Spawn the hourly analytics aggregation background task
    aggregation::spawn_aggregation_task(pool.clone());

    // Spawn the periodic data integrity verification task (Issue #886)
    api::integrity::spawn_integrity_verification_task(pool.clone());

    // Spawn the query-analysis flush + N+1 persistence task (Issue #887)
    api::query_analysis::spawn_query_analysis_task(pool.clone());

    // Create prometheus registry for metrics
    let registry = Registry::new();
    if let Err(e) = api::metrics::register_all(&registry) {
        tracing::error!("Failed to register metrics: {}", e);
    }

    // Job engine omitted: optional dependency; add soroban_batch and uncomment to enable.
    // let (job_engine, job_rx) = soroban_batch::engine::JobEngine::new();
    // let job_engine = Arc::new(job_engine);
    // tokio::spawn(async move { job_engine.clone().run_worker(job_rx).await });

    // Issue #727: create rate limiter before AppState so it can be shared
    let rate_limit_state = std::sync::Arc::new(RateLimitState::from_env());
    rate_limit_state.spawn_eviction_task();

    // Initialize AI service (optional - graceful if not configured)
    let ai_service = match AIService::from_env() {
        Ok(service) => {
            tracing::info!("AI service initialized successfully");
            Some(Arc::new(service))
        }
        Err(e) => {
            tracing::warn!("AI service not initialized: {}", e);
            None
        }
    };

    // Create event broadcaster for real-time updates
    let (event_broadcaster, _) = broadcast::channel(100);

    // Database Concurrency & Queue configuration (#595)
    let concurrency_limit = std::env::var("DB_CONCURRENCY_LIMIT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(((max_pool_size as usize).saturating_sub(2)).max(1));

    let queue_limit = std::env::var("DB_QUEUE_LIMIT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(50);

    let queue_timeout_ms = std::env::var("DB_QUEUE_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5000);

    let breaker_failures = std::env::var("CIRCUIT_BREAKER_FAILURES")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(5);

    let breaker_recovery_secs = std::env::var("CIRCUIT_BREAKER_RECOVERY_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(10);

    tracing::info!(
        concurrency_limit = concurrency_limit,
        queue_limit = queue_limit,
        queue_timeout_ms = queue_timeout_ms,
        breaker_failures = breaker_failures,
        breaker_recovery_secs = breaker_recovery_secs,
        "Initializing database resilience layer"
    );

    let db_breaker = Arc::new(api::db_resilience::CircuitBreaker::new(
        breaker_failures,
        Duration::from_secs(breaker_recovery_secs),
    ));

    let db_queue = Arc::new(api::db_resilience::DbQueue::new(
        concurrency_limit,
        queue_limit,
        Duration::from_millis(queue_timeout_ms),
    ));

    // Spawn background database health ping task
    api::db_resilience::spawn_background_ping_task(
        pool.clone(),
        db_breaker.clone(),
        Duration::from_secs(2), // Ping every 2 seconds
        Duration::from_secs(1), // Timeout after 1 second
    );
    // Initialize feature flags manager from configuration (#1007)
    let flag_entries = api::config::parse_feature_flags(&config.feature_flags_json);
    let feature_flags = Arc::new(api::feature_flags::FeatureFlagManager::from_config(
        &flag_entries,
    ));
    if !flag_entries.is_empty() {
        tracing::info!(
            count = flag_entries.len(),
            "Feature flags loaded from configuration"
        );
    }

    // Create app state
    let is_shutting_down = Arc::new(AtomicBool::new(false));
    // Job engine: initialize for background batch processing
    let (job_engine, job_rx) = soroban_batch::engine::JobEngine::new();
    let job_engine = Arc::new(job_engine);
    let je = job_engine.clone();
    tokio::spawn(async move { je.run_worker(job_rx).await });

    let mut state = AppState::new(
        pool.clone(),
        registry,
        job_engine,
        is_shutting_down.clone(),
        rate_limit_state.clone(),
        ai_service.clone(),
        event_broadcaster.clone(),
        db_breaker,
        db_queue,
        feature_flags,
    )
    .await?;

    // Initialize state monitor service (optional)
    if let Some(monitor) = state.state_monitor.clone() {
        // Spawn state monitor background task
        tokio::spawn(async move {
            if let Err(e) = monitor.run().await {
                tracing::error!("State monitor error: {}", e);
            }
        });
    }

    // Initialize state monitor service (optional)
    let _state_monitor = match StateMonitorService::new(pool.clone(), event_broadcaster.clone()) {
        Ok(service) => {
            info!("State monitor service initialized");
            let monitor = Arc::new(service);
            state.state_monitor = Some(monitor.clone());

            // Spawn state monitor background task
            let monitor_clone = monitor.clone();
            tokio::spawn(async move {
                if let Err(e) = monitor_clone.run().await {
                    error!("State monitor error: {}", e);
                }
            });

            Some(monitor)
        }
        Err(e) => {
            warn!("State monitor service not initialized: {}", e);
            None
        }
    };

    // Initialize GraphQL schema
    let schema = graphql::schema::build_schema(state.clone());

    // Spawn webhook delivery background task
    webhook_delivery::spawn_webhook_delivery_task(pool.clone());

    // Spawn the background DB and cache monitoring task
    let replication_monitor = match (
        std::env::var("DATABASE_PRIMARY_URL"),
        std::env::var("DATABASE_REPLICA_URL"),
    ) {
        (Ok(primary_url), Ok(replica_url)) => {
            let lag_threshold_ms = std::env::var("DATABASE_REPLICATION_LAG_THRESHOLD_MS")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(100);
            Some(api::db_monitoring::ReplicationMonitorConfig {
                primary_url,
                replica_url,
                lag_threshold_ms,
                check_interval: Duration::from_secs(5),
            })
        }
        _ => None,
    };
    db_monitoring::spawn_db_monitoring_task(pool.clone(), state.cache.clone(), replication_monitor);

    // Spawn query monitor: snapshots pg_stat_statements and logs slow queries (Issue #876)
    api::query_monitor::spawn_query_monitor_task(pool.clone(), slow_query_threshold_ms);

    // Spawn the health monitor background task (Issue #333)
    let hm_state = state.clone();
    let hm_status = state.health_monitor_status.clone();
    tokio::spawn(async move {
        health_monitor::run_health_monitor(hm_state, hm_status).await;
    });

    // Create alert manager and spawn system health monitor
    let alert_mgr = Arc::new(api::alerting::AlertManager::new());
    api::system_health::spawn_system_health_monitor(pool.clone(), state.cache.clone(), alert_mgr);

    let network_state = state.clone();
    tokio::spawn(async move {
        handlers::run_network_catalog_refresh(network_state).await;
    });

    // Warm up the cache
    state.cache.clone().warm_up(pool.clone());

    let web_security = WebSecurityConfig::from_env();
    let cors = web_security.build_cors_layer();
    let https_config = api::security::HttpsConfig::from_env();

    // Build router
    let app = routes::application_routes(schema)
        .fallback(handlers::route_not_found)
        .layer(middleware::from_fn(
            validation::payload_size::payload_size_validation_middleware,
        ))
        .layer(middleware::from_fn(
            validation::enhanced_extractors::validation_failure_tracking_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            api::db_resilience::db_resilience_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            http_metrics_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            (*rate_limit_state).clone(),
            rate_limit::rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            web_security,
            api::security::csrf_and_origin_middleware,
        ))
        .layer(cors)
        .layer(middleware::from_fn_with_state(
            https_config,
            api::security::https_enforcement_middleware,
        ))
        .layer(middleware::from_fn(request_tracing::tracing_middleware))
        .with_state(state.clone());

    // Start server (port configurable via PORT env var, default 3001)
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3001);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        tracing::info!(
            "SIGTERM/SIGINT received. Failing health checks and stopping new requests..."
        );
        let _ = tx.send(()).await;
    });

    let server_task = tokio::spawn(async move {
        if let Err(e) = server.await {
            tracing::error!("Server error: {}", e);
        }
    });

    if let Some(()) = rx.recv().await {
        is_shutting_down.store(true, Ordering::SeqCst);
        let initial_in_flight = metrics::HTTP_IN_FLIGHT.get();
        tracing::info!(
            "Graceful shutdown initiated. In-flight requests: {}",
            initial_in_flight
        );

        let timeout_secs = std::env::var("SHUTDOWN_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .unwrap_or(30);

        let start_time = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_secs(timeout_secs);

        let mut success = false;
        loop {
            let in_flight = metrics::HTTP_IN_FLIGHT.get();
            if in_flight == 0 {
                tracing::info!(
                    "All in-flight requests completed in {}ms. In-flight: 0",
                    start_time.elapsed().as_millis()
                );
                success = true;
                break;
            }
            if start_time.elapsed() > timeout_duration {
                tracing::error!(
                    "Graceful shutdown timeout ({}s) reached. {} requests still in-flight.",
                    timeout_secs,
                    in_flight
                );
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        tracing::info!("Closing database connections cleanly...");
        pool.close().await;

        let shutdown_duration = start_time.elapsed();
        tracing::info!(
            "Shutdown complete. Duration: {}ms",
            shutdown_duration.as_millis()
        );

        if success {
            std::process::exit(0);
        } else {
            std::process::exit(1);
        }
    } else {
        let _ = server_task.await;
        tracing::info!("Closing database connections cleanly...");
        pool.close().await;
        tracing::info!("Shutdown complete");
    }

    Ok(())
}
