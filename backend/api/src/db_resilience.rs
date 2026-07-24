//! Database resilience layer: connection queue management + circuit breaker (issue #595).
//!
//! ## Overview
//!
//! This module implements two complementary mechanisms to handle database connection
//! pool exhaustion during traffic spikes:
//!
//! 1. **[`CircuitBreaker`]** — three-state FSM (`Closed → Open → Half-Open`) backed
//!    by a background health-ping task.  While `Open`, requests are fast-failed with
//!    `503 Service Unavailable` so the database gets breathing room.
//!
//! 2. **[`DbQueue`]** — tokio [`Semaphore`]-based concurrency limiter that caps the
//!    number of requests simultaneously executing DB queries.  Requests that cannot
//!    obtain a permit within `queue_timeout` receive `503`; if the waiting queue
//!    itself exceeds `queue_limit` the request is rejected immediately.
//!
//! ## Environment variables
//!
//! | Variable                      | Default                   | Description                            |
//! |-------------------------------|---------------------------|----------------------------------------|
//! | `DB_CONCURRENCY_LIMIT`        | `max_pool - 2` (min 1)    | Concurrent DB-executing requests       |
//! | `DB_QUEUE_LIMIT`              | `50`                      | Max requests waiting for a permit      |
//! | `DB_QUEUE_TIMEOUT_MS`         | `5000`                    | How long to wait for a permit (ms)     |
//! | `CIRCUIT_BREAKER_FAILURES`    | `5`                       | Consecutive ping failures to open      |
//! | `CIRCUIT_BREAKER_RECOVERY_SECS` | `10`                   | Seconds in Open before Half-Open probe |

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use axum::{
    body::Body,
    extract::State,
    http::{header::RETRY_AFTER, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::error::ApiError;
use crate::state::AppState;

// ── Circuit Breaker ──────────────────────────────────────────────────────────

/// Three-state circuit breaker for database health.
///
/// * `Closed`   – normal operation; pings are executed each cycle.
/// * `Open`     – database deemed unhealthy; all DB-bound requests are fast-failed.
/// * `HalfOpen` – a single probe ping is fired; success → `Closed`, failure → `Open`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerState {
    Closed,
    Open,
    HalfOpen,
}

impl BreakerState {
    pub fn to_gauge_value(self) -> i64 {
        match self {
            Self::Closed => 0,
            Self::Open => 1,
            Self::HalfOpen => 2,
        }
    }
}

/// Thread-safe circuit breaker.  All state mutations are protected by an
/// `RwLock`; hot-path reads (`is_open`) acquire only a shared read lock.
#[derive(Debug)]
pub struct CircuitBreaker {
    state: RwLock<BreakerState>,
    consecutive_failures: AtomicU32,
    last_state_change: RwLock<Instant>,
    /// Number of consecutive ping failures required to trip from `Closed` → `Open`.
    failure_threshold: u32,
    /// How long to stay in `Open` before transitioning to `HalfOpen`.
    recovery_time: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_time: Duration) -> Self {
        Self {
            state: RwLock::new(BreakerState::Closed),
            consecutive_failures: AtomicU32::new(0),
            last_state_change: RwLock::new(Instant::now()),
            failure_threshold,
            recovery_time,
        }
    }

    /// Returns the current breaker state without acquiring a write lock.
    pub fn state(&self) -> BreakerState {
        *self
            .state
            .read()
            .expect("circuit breaker state RwLock poisoned")
    }

    /// `true` when the breaker is `Open` and requests should be fast-failed.
    pub fn is_open(&self) -> bool {
        self.state() == BreakerState::Open
    }

    /// Called after a successful ping — resets failure count and closes the circuit.
    pub fn record_success(&self) {
        self.consecutive_failures.store(0, Ordering::SeqCst);
        let mut state = self
            .state
            .write()
            .expect("circuit breaker state RwLock poisoned");
        if *state != BreakerState::Closed {
            tracing::info!("Database circuit breaker → CLOSED (healthy)");
            *state = BreakerState::Closed;
            *self
                .last_state_change
                .write()
                .expect("last_state_change RwLock poisoned") = Instant::now();
        }
    }

    /// Called after a failed or timed-out ping.
    pub fn record_failure(&self) {
        let failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;
        let mut state = self
            .state
            .write()
            .expect("circuit breaker state RwLock poisoned");
        match *state {
            BreakerState::Closed if failures >= self.failure_threshold => {
                tracing::error!(
                    consecutive_failures = failures,
                    "Database circuit breaker → OPEN (unhealthy)"
                );
                *state = BreakerState::Open;
                *self
                    .last_state_change
                    .write()
                    .expect("last_state_change RwLock poisoned") = Instant::now();
                crate::metrics::DB_RESILIENCE_BREAKER_TRIPS.inc();
            }
            BreakerState::HalfOpen => {
                tracing::error!("Database circuit breaker probe failed → OPEN");
                *state = BreakerState::Open;
                *self
                    .last_state_change
                    .write()
                    .expect("last_state_change RwLock poisoned") = Instant::now();
                crate::metrics::DB_RESILIENCE_BREAKER_TRIPS.inc();
            }
            _ => {}
        }
    }

    /// Called once per background-ping cycle to check whether the recovery
    /// timeout has elapsed and the breaker should move to `HalfOpen`.
    pub fn check_recovery(&self) {
        let mut state = self
            .state
            .write()
            .expect("circuit breaker state RwLock poisoned");
        if *state == BreakerState::Open {
            let elapsed = self
                .last_state_change
                .read()
                .expect("last_state_change RwLock poisoned")
                .elapsed();
            if elapsed >= self.recovery_time {
                tracing::warn!(
                    elapsed_ms = elapsed.as_millis(),
                    "Database circuit breaker → HALF-OPEN (probing)"
                );
                *state = BreakerState::HalfOpen;
                self.consecutive_failures.store(0, Ordering::SeqCst);
                *self
                    .last_state_change
                    .write()
                    .expect("last_state_change RwLock poisoned") = Instant::now();
            }
        }
    }
}

// ── Connection Queue ─────────────────────────────────────────────────────────

/// Semaphore-based concurrency limiter that caps simultaneous DB-executing
/// requests and enforces a maximum waiting-queue depth.
#[derive(Debug)]
pub struct DbQueue {
    semaphore: Arc<Semaphore>,
    /// Number of requests currently waiting for a permit.
    queued_requests: AtomicUsize,
    /// Number of requests currently holding a permit (executing a DB query).
    active_requests: AtomicUsize,
    /// Maximum requests allowed to wait before new ones are rejected immediately.
    queue_limit: usize,
    /// Maximum time a request will wait for a permit before it receives `503`.
    queue_timeout: Duration,
}

impl DbQueue {
    pub fn new(concurrency_limit: usize, queue_limit: usize, queue_timeout: Duration) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(concurrency_limit)),
            queued_requests: AtomicUsize::new(0),
            active_requests: AtomicUsize::new(0),
            queue_limit,
            queue_timeout,
        }
    }

    pub fn queued_count(&self) -> usize {
        self.queued_requests.load(Ordering::SeqCst)
    }

    pub fn active_count(&self) -> usize {
        self.active_requests.load(Ordering::SeqCst)
    }

    pub fn is_queue_full(&self) -> bool {
        self.queued_count() >= self.queue_limit
    }

    /// Attempt to obtain a [`DbQueuePermit`].
    ///
    /// Returns `Err(DbQueueError::QueueFull)` immediately if the queue is at
    /// capacity, or `Err(DbQueueError::Timeout)` when the wait exceeds
    /// `queue_timeout`.
    pub async fn acquire(self: &Arc<Self>) -> Result<DbQueuePermit, DbQueueError> {
        if self.is_queue_full() {
            crate::metrics::DB_RESILIENCE_REJECTIONS
                .with_label_values(&["queue_full"])
                .inc();
            return Err(DbQueueError::QueueFull);
        }

        // Account for this request in the waiting count.
        self.queued_requests.fetch_add(1, Ordering::SeqCst);
        crate::metrics::DB_RESILIENCE_QUEUE_DEPTH.set(self.queued_count() as i64);

        let sem_clone = self.semaphore.clone();
        let result = tokio::time::timeout(self.queue_timeout, sem_clone.acquire_owned()).await;

        self.queued_requests.fetch_sub(1, Ordering::SeqCst);
        crate::metrics::DB_RESILIENCE_QUEUE_DEPTH.set(self.queued_count() as i64);

        match result {
            Ok(Ok(permit)) => {
                self.active_requests.fetch_add(1, Ordering::SeqCst);
                crate::metrics::DB_RESILIENCE_ACTIVE_REQS.set(self.active_count() as i64);
                Ok(DbQueuePermit {
                    _permit: permit,
                    queue: self.clone(),
                })
            }
            Ok(Err(_closed)) => {
                crate::metrics::DB_RESILIENCE_REJECTIONS
                    .with_label_values(&["semaphore_closed"])
                    .inc();
                Err(DbQueueError::SemaphoreClosed)
            }
            Err(_timeout) => {
                crate::metrics::DB_RESILIENCE_REJECTIONS
                    .with_label_values(&["queue_timeout"])
                    .inc();
                Err(DbQueueError::Timeout)
            }
        }
    }

    fn release_permit(&self) {
        self.active_requests.fetch_sub(1, Ordering::SeqCst);
        crate::metrics::DB_RESILIENCE_ACTIVE_REQS.set(self.active_count() as i64);
    }
}

/// RAII guard returned by [`DbQueue::acquire`].  Releases the semaphore permit
/// and decrements the active-request counter on drop.
#[derive(Debug)]
pub struct DbQueuePermit {
    _permit: OwnedSemaphorePermit,
    queue: Arc<DbQueue>,
}

impl Drop for DbQueuePermit {
    fn drop(&mut self) {
        self.queue.release_permit();
    }
}

/// Errors returned by [`DbQueue::acquire`].
#[derive(Debug, thiserror::Error)]
pub enum DbQueueError {
    #[error("database request queue is full")]
    QueueFull,
    #[error("timeout waiting for a database connection permit")]
    Timeout,
    #[error("database connection semaphore was closed")]
    SemaphoreClosed,
}

// ── Background Ping Task ─────────────────────────────────────────────────────

/// Spawns a Tokio task that periodically pings the database and drives the
/// circuit breaker state machine.
///
/// * `ping_interval` — how often to attempt a health check (e.g. 2 s).
/// * `ping_timeout`  — per-ping deadline before a failure is recorded (e.g. 1 s).
pub fn spawn_background_ping_task(
    pool: sqlx::PgPool,
    breaker: Arc<CircuitBreaker>,
    ping_interval: Duration,
    ping_timeout: Duration,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(ping_interval);
        // Skip the very first immediate tick so we don't ping before the server
        // has fully started.
        ticker.tick().await;

        loop {
            ticker.tick().await;

            // Possibly transition Open → HalfOpen if recovery time elapsed.
            breaker.check_recovery();

            let current = breaker.state();
            crate::metrics::DB_RESILIENCE_BREAKER_STATE.set(current.to_gauge_value());

            match current {
                BreakerState::Open => {
                    tracing::debug!("DB circuit breaker OPEN — skipping ping");
                }
                BreakerState::Closed | BreakerState::HalfOpen => {
                    let fut = sqlx::query("SELECT 1").execute(&pool);
                    match tokio::time::timeout(ping_timeout, fut).await {
                        Ok(Ok(_)) => {
                            if current == BreakerState::HalfOpen {
                                tracing::info!("DB circuit breaker probe succeeded → CLOSED");
                            }
                            breaker.record_success();
                        }
                        Ok(Err(e)) => {
                            tracing::warn!(error = %e, "DB health ping query failed");
                            breaker.record_failure();
                        }
                        Err(_) => {
                            tracing::warn!("DB health ping timed out");
                            breaker.record_failure();
                        }
                    }
                }
            }
        }
    });
}

// ── Axum Middleware ──────────────────────────────────────────────────────────

/// Axum middleware that enforces the circuit breaker and connection queue on
/// every incoming request that targets a database-backed endpoint.
///
/// Health/metrics endpoints are bypassed so they remain reachable even when
/// the database is unavailable.
pub async fn db_resilience_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let path = request.uri().path();

    // Always allow health and observability endpoints through.
    if matches!(
        path,
        "/health" | "/health/live" | "/health/ready" | "/health/detailed" | "/metrics"
    ) {
        return Ok(next.run(request).await);
    }

    // ── 1. Circuit breaker check ───────────────────────────────────────────
    if state.db_breaker.is_open() {
        crate::metrics::DB_RESILIENCE_REJECTIONS
            .with_label_values(&["circuit_open"])
            .inc();
        let mut response = ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "Database is temporarily unavailable. Please retry shortly.",
        )
        .into_response();
        response
            .headers_mut()
            .insert(RETRY_AFTER, axum::http::HeaderValue::from_static("10"));
        return Ok(response);
    }

    // ── 2. Connection queue permit ─────────────────────────────────────────
    match state.db_queue.acquire().await {
        Ok(permit) => {
            let response = next.run(request).await;
            drop(permit); // explicit; permit released here, not at response send
            Ok(response)
        }
        Err(DbQueueError::QueueFull) => {
            let mut response = ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "QUEUE_FULL",
                "Server is under extremely high load. Please try again later.",
            )
            .into_response();
            response
                .headers_mut()
                .insert(RETRY_AFTER, axum::http::HeaderValue::from_static("5"));
            Ok(response)
        }
        Err(DbQueueError::Timeout) => {
            let mut response = ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "QUEUE_TIMEOUT",
                "Server is busy. Please try again in a moment.",
            )
            .into_response();
            response
                .headers_mut()
                .insert(RETRY_AFTER, axum::http::HeaderValue::from_static("2"));
            Ok(response)
        }
        Err(DbQueueError::SemaphoreClosed) => Ok(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "SEMAPHORE_CLOSED",
            "Internal database service is restarting.",
        )
        .into_response()),
    }
}

// ── Unit Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    // ── CircuitBreaker tests ─────────────────────────────────────────────────

    #[test]
    fn breaker_starts_closed() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(10));
        assert_eq!(cb.state(), BreakerState::Closed);
        assert!(!cb.is_open());
    }

    #[test]
    fn breaker_trips_after_threshold_failures() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(10));
        cb.record_failure();
        assert_eq!(cb.state(), BreakerState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), BreakerState::Closed);
        cb.record_failure(); // 3rd failure → trips
        assert_eq!(cb.state(), BreakerState::Open);
        assert!(cb.is_open());
    }

    #[test]
    fn breaker_success_resets_failure_count() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(10));
        cb.record_failure();
        cb.record_failure();
        cb.record_success(); // resets count
        cb.record_failure();
        // Only 1 failure after reset — still closed
        assert_eq!(cb.state(), BreakerState::Closed);
    }

    #[test]
    fn breaker_does_not_transition_before_recovery_time() {
        let cb = CircuitBreaker::new(1, Duration::from_secs(9999));
        cb.record_failure(); // → Open
        cb.check_recovery(); // should NOT move to HalfOpen yet
        assert_eq!(cb.state(), BreakerState::Open);
    }

    #[tokio::test]
    async fn breaker_transitions_to_half_open_after_recovery() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(50));
        cb.record_failure(); // → Open
        assert_eq!(cb.state(), BreakerState::Open);
        sleep(Duration::from_millis(60)).await;
        cb.check_recovery(); // elapsed > 50 ms → HalfOpen
        assert_eq!(cb.state(), BreakerState::HalfOpen);
    }

    #[tokio::test]
    async fn breaker_closes_on_success_from_half_open() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(50));
        cb.record_failure(); // → Open
        sleep(Duration::from_millis(60)).await;
        cb.check_recovery(); // → HalfOpen
        cb.record_success(); // → Closed
        assert_eq!(cb.state(), BreakerState::Closed);
    }

    #[tokio::test]
    async fn breaker_reopens_on_failure_from_half_open() {
        let cb = CircuitBreaker::new(1, Duration::from_millis(50));
        cb.record_failure(); // → Open
        sleep(Duration::from_millis(60)).await;
        cb.check_recovery(); // → HalfOpen
        cb.record_failure(); // probe failed → Open again
        assert_eq!(cb.state(), BreakerState::Open);
    }

    // ── DbQueue tests ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn queue_permits_acquired_and_released() {
        let queue = Arc::new(DbQueue::new(2, 10, Duration::from_secs(1)));
        let p1 = queue.acquire().await.expect("permit 1");
        let p2 = queue.acquire().await.expect("permit 2");
        assert_eq!(queue.active_count(), 2);
        drop(p1);
        assert_eq!(queue.active_count(), 1);
        drop(p2);
        assert_eq!(queue.active_count(), 0);
    }

    #[tokio::test]
    async fn queue_rejects_when_full() {
        // concurrency_limit=1, queue_limit=0 → any waiter is immediately rejected
        let queue = Arc::new(DbQueue::new(1, 0, Duration::from_secs(1)));
        let _p = queue.acquire().await.expect("first permit");
        // queue is now saturated (queue_limit = 0 means no waiting allowed)
        let err = queue.acquire().await.unwrap_err();
        assert!(matches!(err, DbQueueError::QueueFull));
    }

    #[tokio::test]
    async fn queue_times_out_when_all_permits_held() {
        // concurrency_limit=1, large enough queue_limit, very short timeout
        let queue = Arc::new(DbQueue::new(1, 100, Duration::from_millis(50)));
        let _p = queue.acquire().await.expect("first permit");
        let err = queue.acquire().await.unwrap_err();
        assert!(matches!(err, DbQueueError::Timeout));
    }

    #[tokio::test]
    async fn queue_depth_gauge_tracks_waiters() {
        let queue = Arc::new(DbQueue::new(1, 100, Duration::from_millis(200)));
        let _permit = queue.acquire().await.expect("hold the only permit");

        // Spawn a waiter in the background
        let q = queue.clone();
        let waiter = tokio::spawn(async move { q.acquire().await });

        // Give the waiter a moment to park on the semaphore
        sleep(Duration::from_millis(30)).await;
        assert_eq!(queue.queued_count(), 1);

        drop(_permit); // release so waiter can proceed
        let _ = waiter.await;
    }
}
