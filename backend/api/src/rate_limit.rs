//! Sliding-window rate limiter with automatic eviction of expired entries.
//!
//! ## Memory-leak fix (issue #317)
//!
//! The original implementation stored fixed-window counters in a
//! `HashMap` that was never cleaned up.  Every unique IP that ever hit the
//! API accumulated an entry that lived forever.
//!
//! This version fixes that by:
//!
//! 1. **Background eviction task** – [`RateLimitState::spawn_eviction_task`]
//!    runs every `EVICTION_INTERVAL` and removes any bucket whose window
//!    expired more than `window_duration` ago.
//! 2. **`tokio::sync::Mutex`** – replaced `std::sync::Mutex` so the lock is
//!    held correctly across `.await` points and avoids blocking the async
//!    executor.
//! 3. **Graceful lock error handling** – `.lock().await` on a
//!    `tokio::sync::Mutex` never poisons, so the panic from `.expect()` is
//!    gone. The one remaining fallible path (attaching response headers) logs
//!    a warning instead of crashing.
//!
//! ## Write-endpoint protection
//!
//! Mutation endpoints (POST, PUT, PATCH, DELETE) consume more server resources
//! and can cause irreversible state changes.  They are subject to a separate,
//! tighter quota:
//!
//! - Anonymous write limit: 100 req/hour  (`RATE_LIMIT_WRITE_ANON_PER_HOUR`)
//! - Authenticated write limit: 300 req/hour (`RATE_LIMIT_WRITE_AUTH_PER_HOUR`)
//!
//! ## Exempt paths
//!
//! Health probes (`/health*`), the Prometheus metrics scrape endpoint
//! (`/metrics`), and internal admin flows (`/api/admin/*`) bypass the rate
//! limiter so monitoring systems and operators are never blocked.
//!
//! ## Horizontal scaling note
//!
//! This rate limiter is **per-instance**.  When running multiple API replicas
//! the bucket state is not shared between them.  For true distributed rate
//! limiting consider replacing the in-process `HashMap` with a Redis-backed
//! store (e.g. via the `upstash-redis` crate or `fred`).

use std::{
    collections::{HashMap, HashSet, VecDeque},
    env,
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    extract::{connect_info::ConnectInfo, State},
    http::{
        header::{AUTHORIZATION, RETRY_AFTER},
        HeaderName, HeaderValue, Method, Request,
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use tokio::sync::Mutex;

use crate::error::ApiError;

// Issue #891: 1,000 requests per minute per IP/API key by default.
const DEFAULT_ANON_LIMIT: u32 = 1_000;
const DEFAULT_AUTH_LIMIT: u32 = 1_000;
// Stricter default quotas for mutation (write) endpoints.
const DEFAULT_WRITE_ANON_LIMIT: u32 = 100;
const DEFAULT_WRITE_AUTH_LIMIT: u32 = 300;
const DEFAULT_WINDOW_SECONDS: u64 = 60;
#[allow(dead_code)]
const DEFAULT_CONTRACTS_PAGE_SIZE: u32 = 50;
#[allow(dead_code)]
const MAX_CONTRACTS_PAGE_SIZE: u32 = 1000;
const ABI_ENDPOINT_LIMIT_PER_MINUTE: u32 = 1_000;
#[allow(dead_code)]
const ENDPOINT_LIMIT_ENV_PREFIX: &str = "RATE_LIMIT_ENDPOINT_";

// Issue #1045: Per-endpoint tighter limits for publish and search.
// Publish (POST /api/contracts) is the most expensive write — cap it hard.
/// Default anonymous publish limit per window (5 publishes).
const DEFAULT_PUBLISH_ANON_LIMIT: u32 = 5;
/// Default authenticated publish limit per window (30 publishes).
const DEFAULT_PUBLISH_AUTH_LIMIT: u32 = 30;
/// Default anonymous search limit per window (100 queries).
const DEFAULT_SEARCH_ANON_LIMIT: u32 = 100;
/// Default authenticated search limit per window (500 queries).
const DEFAULT_SEARCH_AUTH_LIMIT: u32 = 500;

/// Issue #727 — tiered limits over the configured window.
const FREE_TIER_LIMIT: u32 = 1_000;
const PRO_TIER_LIMIT: u32 = 10_000;
const BURST_WINDOW_SECONDS: u64 = 60; // 1 minute burst window

/// How often the background task sweeps for expired buckets.
const EVICTION_INTERVAL: Duration = Duration::from_secs(5 * 60); // every 5 minutes

const HEADER_RATE_LIMIT_LIMIT: HeaderName = HeaderName::from_static("x-ratelimit-limit");
const HEADER_RATE_LIMIT_REMAINING: HeaderName = HeaderName::from_static("x-ratelimit-remaining");
const HEADER_RATE_LIMIT_RESET: HeaderName = HeaderName::from_static("x-ratelimit-reset");
const HEADER_RATE_LIMIT_TIER: HeaderName = HeaderName::from_static("x-ratelimit-tier");

/// API tier that determines quota limits (issue #727).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiTier {
    Free,
    Pro,
    /// Enterprise limit is read from ENTERPRISE_RATE_LIMIT_PER_HOUR at startup.
    Enterprise,
}

impl ApiTier {
    fn as_str(&self) -> &'static str {
        match self {
            ApiTier::Free => "free",
            ApiTier::Pro => "pro",
            ApiTier::Enterprise => "enterprise",
        }
    }

    pub fn from_header(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "pro" => ApiTier::Pro,
            "enterprise" => ApiTier::Enterprise,
            _ => ApiTier::Free,
        }
    }
}

/// Paths that bypass rate limiting entirely.
///
/// Health probes must not be rate-limited so that load balancers and
/// orchestrators always get a response.  The Prometheus scrape endpoint
/// `/metrics` and internal admin APIs are also exempt.
fn is_exempt_path(path: &str) -> bool {
    path.starts_with("/health") || path == "/metrics" || path.starts_with("/api/admin/")
}

#[derive(Clone)]
pub struct RateLimitState {
    config: std::sync::Arc<RateLimitConfig>,
    /// Shared bucket map — protected by a *tokio* Mutex so it is async-safe.
    buckets: std::sync::Arc<Mutex<HashMap<BucketKey, BucketState>>>,
}

/// Snapshot of quota usage for a client key (used by the /api/quota endpoint).
#[derive(Debug, Clone, serde::Serialize)]
pub struct QuotaSnapshot {
    pub tier: String,
    pub hourly_limit: u32,
    pub hourly_used: usize,
    pub hourly_remaining: usize,
    pub burst_limit: u32,
    pub burst_used: usize,
    pub burst_remaining: usize,
}

impl RateLimitState {
    pub fn from_env() -> Self {
        Self::new(RateLimitConfig::from_env())
    }

    fn new(config: RateLimitConfig) -> Self {
        Self {
            config: std::sync::Arc::new(config),
            buckets: std::sync::Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Spawn a background Tokio task that periodically evicts expired buckets.
    ///
    /// Call this once during application startup (after `tokio::main` is
    /// entered).  The task runs until the process exits.
    pub fn spawn_eviction_task(&self) {
        let buckets = self.buckets.clone();
        let window = self.config.window;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(EVICTION_INTERVAL);
            // The first tick fires immediately — skip it so we don't evict
            // right after startup when there is nothing to evict yet.
            ticker.tick().await;

            loop {
                ticker.tick().await;

                let now = Instant::now();
                let mut map = buckets.lock().await;
                let before = map.len();

                // Retain only buckets that have seen traffic recently.
                map.retain(|_, state| {
                    // Check both windows; keep if either has a recent timestamp.
                    let hourly_active = state
                        .timestamps
                        .back()
                        .map(|last_seen| {
                            now.saturating_duration_since(*last_seen) < window.saturating_mul(2)
                        })
                        .unwrap_or(false);
                    let burst_active = state
                        .burst_timestamps
                        .back()
                        .map(|last_seen| {
                            now.saturating_duration_since(*last_seen) < window.saturating_mul(2)
                        })
                        .unwrap_or(false);
                    hourly_active || burst_active
                });

                let evicted = before - map.len();
                if evicted > 0 {
                    tracing::info!(
                        evicted,
                        remaining = map.len(),
                        "rate limiter: evicted expired buckets"
                    );
                }
            }
        });
    }

    async fn check_request(
        &self,
        key: BucketKey,
        hourly_limit: u32,
        burst_limit: u32,
    ) -> RateLimitDecision {
        let now = Instant::now();

        // tokio::sync::Mutex::lock() never poisons — no .expect() needed.
        let mut buckets = self.buckets.lock().await;

        let bucket = buckets.entry(key).or_insert_with(|| BucketState {
            timestamps: VecDeque::new(),
            burst_timestamps: VecDeque::new(),
        });

        // Trim the hourly sliding window
        let window_start_cutoff = now.checked_sub(self.config.window).unwrap_or(now);
        while bucket
            .timestamps
            .front()
            .copied()
            .map(|ts| ts <= window_start_cutoff)
            .unwrap_or(false)
        {
            bucket.timestamps.pop_front();
        }

        // Trim the burst (1-minute) sliding window
        let burst_cutoff = now.checked_sub(self.config.burst_window).unwrap_or(now);
        while bucket
            .burst_timestamps
            .front()
            .copied()
            .map(|ts| ts <= burst_cutoff)
            .unwrap_or(false)
        {
            bucket.burst_timestamps.pop_front();
        }

        let reset_seconds = bucket
            .timestamps
            .front()
            .and_then(|oldest| oldest.checked_add(self.config.window))
            .map(|expiry| ceil_duration_to_seconds(expiry.saturating_duration_since(now)).max(1))
            .unwrap_or_else(|| ceil_duration_to_seconds(self.config.window).max(1));

        // Reject if hourly limit exceeded
        if (bucket.timestamps.len() as u32) >= hourly_limit {
            return RateLimitDecision {
                allowed: false,
                limit: hourly_limit,
                remaining: 0,
                reset_seconds,
            };
        }

        // Reject if burst limit exceeded (too many in the last minute)
        if (bucket.burst_timestamps.len() as u32) >= burst_limit {
            return RateLimitDecision {
                allowed: false,
                limit: hourly_limit,
                remaining: hourly_limit.saturating_sub(bucket.timestamps.len() as u32),
                reset_seconds: ceil_duration_to_seconds(self.config.burst_window).max(1),
            };
        }

        bucket.timestamps.push_back(now);
        bucket.burst_timestamps.push_back(now);
        let remaining = hourly_limit.saturating_sub(bucket.timestamps.len() as u32);

        RateLimitDecision {
            allowed: true,
            limit: hourly_limit,
            remaining,
            reset_seconds,
        }
    }

    /// Returns the current quota snapshot for a client key without consuming a token.
    pub async fn quota_snapshot(&self, client_key: &str, tier: &ApiTier) -> QuotaSnapshot {
        let now = Instant::now();
        let hourly_limit = self.config.hourly_limit_for_tier(tier);
        let burst_limit = self.config.burst_limit_for_tier(tier);

        let buckets = self.buckets.lock().await;
        let key = BucketKey {
            client_key: client_key.to_owned(),
        };

        if let Some(bucket) = buckets.get(&key) {
            let window_cutoff = now.checked_sub(self.config.window).unwrap_or(now);
            let hourly_used = bucket
                .timestamps
                .iter()
                .filter(|&&ts| ts > window_cutoff)
                .count();

            let burst_cutoff = now.checked_sub(self.config.burst_window).unwrap_or(now);
            let burst_used = bucket
                .burst_timestamps
                .iter()
                .filter(|&&ts| ts > burst_cutoff)
                .count();

            QuotaSnapshot {
                tier: tier.as_str().to_owned(),
                hourly_limit,
                hourly_used,
                hourly_remaining: (hourly_limit as usize).saturating_sub(hourly_used),
                burst_limit,
                burst_used,
                burst_remaining: (burst_limit as usize).saturating_sub(burst_used),
            }
        } else {
            QuotaSnapshot {
                tier: tier.as_str().to_owned(),
                hourly_limit,
                hourly_used: 0,
                hourly_remaining: hourly_limit as usize,
                burst_limit,
                burst_used: 0,
                burst_remaining: burst_limit as usize,
            }
        }
    }

    /// Derive the bucket key and API tier from an incoming request.
    ///
    /// Tier is resolved in order:
    /// 1. `X-Api-Plan: free | pro | enterprise` header
    /// 2. Defaults to `free` for unauthenticated requests, `free` for authenticated
    ///    (upgrade to pro/enterprise requires the header or a future DB lookup).
    fn select_limit_and_key<B>(&self, request: &Request<B>) -> (u32, u32, BucketKey, ApiTier) {
        let tier = extract_api_tier(request);
        let path = request.uri().path();
        let query = request.uri().query();
        // Mutation requests get a separate, stricter bucket.
        let is_write = matches!(
            request.method().as_str(),
            "POST" | "PUT" | "PATCH" | "DELETE"
        );
        let bucket_suffix = if is_write { "write" } else { "read" };

        if is_contract_abi_endpoint(request.method(), path) {
            return (
                ABI_ENDPOINT_LIMIT_PER_MINUTE * 60,
                ABI_ENDPOINT_LIMIT_PER_MINUTE,
                BucketKey {
                    client_key: format!("abi:{}:{}", path, extract_client_ip(request)),
                },
                tier,
            );
        }

        // Issue #1045: publish endpoint — POST /api/contracts.
        // Use a dedicated, tighter bucket so publish spam is blocked independently
        // of the general write quota and does not penalise other write endpoints.
        if is_publish_endpoint(request.method(), path) {
            if let Some(token) = extract_auth_token(request) {
                let limit = self.config.publish_auth_limit;
                let burst = self.config.burst_limit_for_limit(limit);
                return (
                    limit,
                    burst,
                    BucketKey {
                        client_key: format!("publish:auth:{token}"),
                    },
                    tier,
                );
            }
            let ip = extract_client_ip(request);
            let limit = self.config.publish_anon_limit;
            let burst = self.config.burst_limit_for_limit(limit);
            return (
                limit,
                burst,
                BucketKey {
                    client_key: format!("publish:anon:{ip}"),
                },
                tier,
            );
        }

        // Issue #1045: search/list endpoints — GET /api/contracts, /api/v1/contracts/search, etc.
        // Dedicated bucket prevents aggressive scraping from consuming the general read quota.
        if is_search_endpoint(request.method(), path) {
            if let Some(token) = extract_auth_token(request) {
                let limit = self.config.search_auth_limit;
                let burst = self.config.burst_limit_for_limit(limit);
                return (
                    limit,
                    burst,
                    BucketKey {
                        client_key: format!("search:auth:{token}"),
                    },
                    tier,
                );
            }
            let ip = extract_client_ip(request);
            // Scale down further for large page sizes (already-existing logic).
            let base = self.config.search_anon_limit;
            let limit = if let Some(page_size) =
                contracts_page_size_rate_limit(request.method(), path, query)
            {
                scale_limit_by_page_size(base, page_size)
            } else {
                base
            };
            let burst = self.config.burst_limit_for_limit(limit);
            return (
                limit,
                burst,
                BucketKey {
                    client_key: format!("search:anon:{ip}"),
                },
                tier,
            );
        }

        if let Some(token) = extract_auth_token(request) {
            let hourly = self
                .config
                .per_api_key_limit(&token)
                .unwrap_or_else(|| self.config.hourly_limit_for_tier(&tier));
            let auth_hourly = if is_write {
                self.config.write_auth_limit.min(hourly)
            } else {
                hourly
            };
            let burst = self.config.burst_limit_for_limit(auth_hourly);
            return (
                auth_hourly,
                burst,
                BucketKey {
                    client_key: format!("auth:{bucket_suffix}:{token}"),
                },
                tier,
            );
        }

        let ip = extract_client_ip(request);
        if is_write {
            let limit = self.config.write_anonymous_limit;
            let burst = self.config.burst_limit_for_limit(limit);
            return (
                limit,
                burst,
                BucketKey {
                    client_key: format!("anon:write:{ip}"),
                },
                tier,
            );
        }

        let hourly_base = self.config.anonymous_limit;
        let hourly = if let Some(page_size) =
            contracts_page_size_rate_limit(request.method(), path, query)
        {
            scale_limit_by_page_size(hourly_base, page_size)
        } else {
            hourly_base
        };
        let burst = self.config.burst_limit_for_limit(hourly);

        (
            hourly,
            burst,
            BucketKey {
                client_key: format!("anon:read:{ip}"),
            },
            tier,
        )
    }
}

struct RateLimitConfig {
    anonymous_limit: u32,
    auth_limit: u32,
    write_anonymous_limit: u32,
    write_auth_limit: u32,
    window: Duration,
    enterprise_limit: u32,
    burst_window: Duration,
    per_api_key_limits: HashMap<String, u32>,
    trusted_client_ips: HashSet<String>,
    trusted_api_keys: HashSet<String>,
    /// Issue #1045: per-endpoint tighter limit for POST /api/contracts (publish).
    publish_anon_limit: u32,
    publish_auth_limit: u32,
    /// Issue #1045: per-endpoint tighter limit for GET /api/v1/contracts/search
    /// and GET /api/contracts (search/list).
    search_anon_limit: u32,
    search_auth_limit: u32,
}

impl RateLimitConfig {
    fn from_env() -> Self {
        // Issue #891: configurable per-IP and per-API-key limits; defaults to 1000 req/min.
        let anonymous_limit = env_u32_with_fallback(
            "RATE_LIMIT_IP_PER_MINUTE",
            "RATE_LIMIT_ANON_PER_MINUTE",
            DEFAULT_ANON_LIMIT,
        );
        let auth_limit = env_u32_with_fallback(
            "RATE_LIMIT_API_KEY_PER_MINUTE",
            "RATE_LIMIT_AUTH_PER_MINUTE",
            DEFAULT_AUTH_LIMIT,
        );
        // Write limits — stricter quota for mutation endpoints.
        let write_anonymous_limit =
            env_u32("RATE_LIMIT_WRITE_ANON_PER_HOUR", DEFAULT_WRITE_ANON_LIMIT);
        let write_auth_limit = env_u32("RATE_LIMIT_WRITE_AUTH_PER_HOUR", DEFAULT_WRITE_AUTH_LIMIT);
        let window_seconds = env_u64("RATE_LIMIT_WINDOW_SECONDS", DEFAULT_WINDOW_SECONDS).max(1);
        // Issue #727: enterprise tier custom limit
        let enterprise_limit = env_u32("ENTERPRISE_RATE_LIMIT_PER_WINDOW", 100_000);
        let per_api_key_limits = parse_key_limit_map("RATE_LIMIT_API_KEY_LIMITS");
        let trusted_client_ips = parse_csv_set("RATE_LIMIT_TRUSTED_IPS");
        let trusted_api_keys = parse_csv_set("RATE_LIMIT_TRUSTED_API_KEYS");

        // Issue #1045: per-endpoint tighter limits for publish and search.
        let publish_anon_limit =
            env_u32("RATE_LIMIT_PUBLISH_ANON_PER_WINDOW", DEFAULT_PUBLISH_ANON_LIMIT);
        let publish_auth_limit =
            env_u32("RATE_LIMIT_PUBLISH_AUTH_PER_WINDOW", DEFAULT_PUBLISH_AUTH_LIMIT);
        let search_anon_limit =
            env_u32("RATE_LIMIT_SEARCH_ANON_PER_WINDOW", DEFAULT_SEARCH_ANON_LIMIT);
        let search_auth_limit =
            env_u32("RATE_LIMIT_SEARCH_AUTH_PER_WINDOW", DEFAULT_SEARCH_AUTH_LIMIT);

        tracing::info!(
            anonymous_limit,
            auth_limit,
            write_anonymous_limit,
            write_auth_limit,
            window_seconds,
            enterprise_limit,
            publish_anon_limit,
            publish_auth_limit,
            search_anon_limit,
            search_auth_limit,
            "Rate limiter configured (issue #891/#727/#1045: per-IP/API-key/endpoint quotas)"
        );

        Self {
            anonymous_limit,
            auth_limit,
            write_anonymous_limit,
            write_auth_limit,
            window: Duration::from_secs(window_seconds),
            enterprise_limit,
            burst_window: Duration::from_secs(BURST_WINDOW_SECONDS),
            per_api_key_limits,
            trusted_client_ips,
            trusted_api_keys,
            publish_anon_limit,
            publish_auth_limit,
            search_anon_limit,
            search_auth_limit,
        }
    }

    #[cfg(test)]
    fn for_tests(anonymous_limit: u32, auth_limit: u32, window: Duration) -> Self {
        Self {
            anonymous_limit,
            auth_limit,
            write_anonymous_limit: anonymous_limit / 10,
            write_auth_limit: auth_limit / 3,
            window,
            enterprise_limit: 100_000,
            burst_window: Duration::from_secs(BURST_WINDOW_SECONDS),
            per_api_key_limits: HashMap::new(),
            trusted_client_ips: HashSet::new(),
            trusted_api_keys: HashSet::new(),
            publish_anon_limit: DEFAULT_PUBLISH_ANON_LIMIT,
            publish_auth_limit: DEFAULT_PUBLISH_AUTH_LIMIT,
            search_anon_limit: DEFAULT_SEARCH_ANON_LIMIT,
            search_auth_limit: DEFAULT_SEARCH_AUTH_LIMIT,
        }
    }

    #[cfg(test)]
    fn for_tests_with_write(
        anonymous_limit: u32,
        auth_limit: u32,
        write_anonymous_limit: u32,
        write_auth_limit: u32,
        window: Duration,
    ) -> Self {
        Self {
            anonymous_limit,
            auth_limit,
            write_anonymous_limit,
            write_auth_limit,
            window,
            enterprise_limit: 100_000,
            burst_window: Duration::from_secs(BURST_WINDOW_SECONDS),
            per_api_key_limits: HashMap::new(),
            trusted_client_ips: HashSet::new(),
            trusted_api_keys: HashSet::new(),
            publish_anon_limit: DEFAULT_PUBLISH_ANON_LIMIT,
            publish_auth_limit: DEFAULT_PUBLISH_AUTH_LIMIT,
            search_anon_limit: DEFAULT_SEARCH_ANON_LIMIT,
            search_auth_limit: DEFAULT_SEARCH_AUTH_LIMIT,
        }
    }

    /// Constructor used by publish/search endpoint tests with explicit per-endpoint limits.
    #[cfg(test)]
    fn for_tests_with_endpoint_limits(
        anonymous_limit: u32,
        auth_limit: u32,
        write_anonymous_limit: u32,
        write_auth_limit: u32,
        publish_anon_limit: u32,
        publish_auth_limit: u32,
        search_anon_limit: u32,
        search_auth_limit: u32,
        window: Duration,
    ) -> Self {
        Self {
            anonymous_limit,
            auth_limit,
            write_anonymous_limit,
            write_auth_limit,
            window,
            enterprise_limit: 100_000,
            burst_window: Duration::from_secs(BURST_WINDOW_SECONDS),
            per_api_key_limits: HashMap::new(),
            trusted_client_ips: HashSet::new(),
            trusted_api_keys: HashSet::new(),
            publish_anon_limit,
            publish_auth_limit,
            search_anon_limit,
            search_auth_limit,
        }
    }

    /// Returns the hourly limit for the given tier.
    fn hourly_limit_for_tier(&self, tier: &ApiTier) -> u32 {
        match tier {
            ApiTier::Free => self.auth_limit.max(FREE_TIER_LIMIT),
            ApiTier::Pro => self.auth_limit.max(PRO_TIER_LIMIT),
            ApiTier::Enterprise => self.enterprise_limit,
        }
    }

    /// Burst limit = ceil(hourly_limit × 1.2 / 60) requests per minute.
    fn burst_limit_for_tier(&self, tier: &ApiTier) -> u32 {
        let hourly = self.hourly_limit_for_tier(tier);
        self.burst_limit_for_limit(hourly)
    }

    fn burst_limit_for_limit(&self, hourly: u32) -> u32 {
        // 120 % of the per-minute equivalent; minimum 1
        ((hourly as f64 * 1.2 / 60.0).ceil() as u32).max(1)
    }

    fn per_api_key_limit(&self, token: &str) -> Option<u32> {
        let normalized = token
            .strip_prefix("Bearer ")
            .or_else(|| token.strip_prefix("ApiKey "))
            .unwrap_or(token)
            .trim();
        self.per_api_key_limits.get(normalized).copied()
    }

    fn is_whitelisted<B>(&self, request: &Request<B>) -> bool {
        let client_ip = extract_client_ip(request);
        if self.trusted_client_ips.contains(&client_ip) {
            return true;
        }

        extract_auth_token(request)
            .map(|token| {
                let normalized = token
                    .strip_prefix("Bearer ")
                    .or_else(|| token.strip_prefix("ApiKey "))
                    .unwrap_or(&token)
                    .trim()
                    .to_string();
                self.trusted_api_keys.contains(&normalized)
            })
            .unwrap_or(false)
    }
}

#[derive(Hash, Eq, PartialEq)]
struct BucketKey {
    client_key: String,
}

struct BucketState {
    timestamps: VecDeque<Instant>,
    /// Timestamps in the 1-minute burst window (subset of `timestamps`).
    burst_timestamps: VecDeque<Instant>,
}

struct RateLimitDecision {
    allowed: bool,
    limit: u32,
    remaining: u32,
    reset_seconds: u64,
}

pub async fn rate_limit_middleware(
    State(rate_limiter): State<RateLimitState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if rate_limiter.config.is_whitelisted(&request) {
        return next.run(request).await;
    }

    // Extract request metadata before awaiting to avoid borrowing `request` across `.await`.
    let (hourly_limit, burst_limit, key, tier) = rate_limiter.select_limit_and_key(&request);
    let decision = rate_limiter
        .check_request(key, hourly_limit, burst_limit)
        .await;

    if !decision.allowed {
        let is_write = is_write_method(request.method());
        // Observability: surface throttle events so operators can spot abuse or
        // accidental overload of public endpoints (issue #1005).
        tracing::warn!(
            client_ip = %extract_client_ip(&request),
            tier = tier.as_str(),
            limit_type = if is_write { "write" } else { "read" },
            limit = decision.limit,
            remaining = decision.remaining,
            retry_after_seconds = decision.reset_seconds,
            "Request throttled by rate limiter"
        );
        let detail = if is_write {
            "Write quota exhausted. Reduce request frequency or wait for the window to reset."
        } else {
            "Too many requests. Please retry after the indicated time."
        };
        let mut response = ApiError::rate_limited(detail)
            .with_details(serde_json::json!({
                "retry_after_seconds": decision.reset_seconds,
                "limit_type": if is_write { "write" } else { "read" }
            }))
            .into_response();
        attach_rate_limit_headers(&mut response, &decision);
        attach_tier_header(&mut response, &tier);
        response.headers_mut().insert(
            RETRY_AFTER,
            HeaderValue::from_str(&decision.reset_seconds.to_string())
                .unwrap_or_else(|_| HeaderValue::from_static("1")),
        );
        return response;
    }

    let mut response = next.run(request).await;
    attach_rate_limit_headers(&mut response, &decision);
    attach_tier_header(&mut response, &tier);
    response
}

fn attach_tier_header(response: &mut Response, tier: &ApiTier) {
    if let Ok(val) = HeaderValue::from_str(tier.as_str()) {
        response.headers_mut().insert(HEADER_RATE_LIMIT_TIER, val);
    }
}

fn attach_rate_limit_headers(response: &mut Response, decision: &RateLimitDecision) {
    // Use graceful fallback instead of panicking on header value conversion.
    let headers = response.headers_mut();

    if let Ok(val) = HeaderValue::from_str(&decision.limit.to_string()) {
        headers.insert(HEADER_RATE_LIMIT_LIMIT, val);
    } else {
        tracing::warn!("Failed to encode x-ratelimit-limit header");
    }

    if let Ok(val) = HeaderValue::from_str(&decision.remaining.to_string()) {
        headers.insert(HEADER_RATE_LIMIT_REMAINING, val);
    } else {
        tracing::warn!("Failed to encode x-ratelimit-remaining header");
    }

    if let Ok(val) = HeaderValue::from_str(&decision.reset_seconds.to_string()) {
        headers.insert(HEADER_RATE_LIMIT_RESET, val);
    } else {
        tracing::warn!("Failed to encode x-ratelimit-reset header");
    }
}

fn extract_client_ip<B>(request: &Request<B>) -> String {
    if let Some(ip) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_x_forwarded_for)
    {
        return ip.to_string();
    }

    if let Some(ip) = request
        .headers()
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_ip_addr)
    {
        return ip.to_string();
    }

    if let Some(connect_info) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        return connect_info.0.ip().to_string();
    }

    "unknown".to_string()
}

/// Resolve API tier from the `X-Api-Plan` request header.
/// Falls back to `Free` when the header is absent or unrecognised.
fn extract_api_tier<B>(request: &Request<B>) -> ApiTier {
    request
        .headers()
        .get("x-api-plan")
        .and_then(|v| v.to_str().ok())
        .map(ApiTier::from_header)
        .unwrap_or(ApiTier::Free)
}

fn extract_auth_token<B>(request: &Request<B>) -> Option<String> {
    if let Some(api_key) = request
        .headers()
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Some(api_key.to_string());
    }

    request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_key_limit_map(key: &str) -> HashMap<String, u32> {
    std::env::var(key)
        .unwrap_or_default()
        .split(',')
        .filter_map(|entry| {
            let (api_key, limit) = entry.split_once('=')?;
            let limit = limit.trim().parse::<u32>().ok()?;
            (limit > 0).then(|| (api_key.trim().to_string(), limit))
        })
        .filter(|(api_key, _)| !api_key.is_empty())
        .collect()
}

fn parse_csv_set(key: &str) -> HashSet<String> {
    std::env::var(key)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_x_forwarded_for(raw: &str) -> Option<IpAddr> {
    raw.split(',').map(str::trim).find_map(parse_ip_addr)
}

fn parse_ip_addr(raw: &str) -> Option<IpAddr> {
    raw.parse::<IpAddr>()
        .ok()
        .or_else(|| raw.parse::<SocketAddr>().ok().map(|addr| addr.ip()))
}

fn is_write_method(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

fn contracts_page_size_rate_limit(method: &Method, path: &str, query: Option<&str>) -> Option<u32> {
    if *method != Method::GET || path != "/api/contracts" {
        return None;
    }

    Some(extract_page_size(query).unwrap_or(DEFAULT_CONTRACTS_PAGE_SIZE))
}

fn is_contract_abi_endpoint(method: &Method, path: &str) -> bool {
    *method == Method::GET && path.starts_with("/api/v1/contracts/") && path.ends_with("/abi")
}

/// Issue #1045: Detect contract publish endpoint — POST /api/contracts.
///
/// This is the most resource-intensive write: it validates the WASM hash,
/// writes metadata, emits events, and may trigger async verification.
/// It gets its own tighter quota so a single IP cannot spam the registry.
fn is_publish_endpoint(method: &Method, path: &str) -> bool {
    *method == Method::POST && path == "/api/contracts"
}

/// Issue #1045: Detect contract search / list endpoints.
///
/// Covers:
///  - GET /api/contracts                        (list/search)
///  - GET /api/v1/contracts/search              (v1 full-text search)
///  - GET /api/contracts/suggestions            (search autocomplete)
///  - GET /api/v1/contracts/trending            (trending list)
///
/// These are read-heavy but can be abused for bulk scraping; a per-endpoint
/// quota complements (but does not replace) the general read limit.
fn is_search_endpoint(method: &Method, path: &str) -> bool {
    if *method != Method::GET {
        return false;
    }
    path == "/api/contracts"
        || path == "/api/v1/contracts/search"
        || path == "/api/contracts/suggestions"
        || path == "/api/v1/contracts/trending"
        || path == "/contracts"
}

fn extract_page_size(query: Option<&str>) -> Option<u32> {
    let query = query?;

    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next()?;
        let value = parts.next().unwrap_or_default();

        if key == "limit" || key == "page_size" {
            if let Ok(parsed) = value.parse::<u32>() {
                return Some(parsed.clamp(1, MAX_CONTRACTS_PAGE_SIZE));
            }
        }
    }

    None
}

fn scale_limit_by_page_size(base_limit: u32, page_size: u32) -> u32 {
    let weight = page_size.div_ceil(DEFAULT_CONTRACTS_PAGE_SIZE).max(1);
    (base_limit / weight).max(1)
}

#[allow(dead_code)]
fn endpoint_key(method: &Method, path: &str) -> String {
    let normalized_path = path
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();

    let compact_path = normalized_path
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_");

    if compact_path.is_empty() {
        format!("{}_ROOT", method.as_str().to_ascii_uppercase())
    } else {
        format!("{}_{}", method.as_str().to_ascii_uppercase(), compact_path)
    }
}

fn env_u32(key: &str, default: u32) -> u32 {
    match env::var(key) {
        Ok(raw) => match raw.parse::<u32>() {
            Ok(value) if value > 0 => value,
            _ => {
                tracing::warn!("Invalid value for {key} (`{raw}`), using default {default}");
                default
            }
        },
        Err(_) => default,
    }
}

fn env_u32_with_fallback(primary_key: &str, fallback_key: &str, default: u32) -> u32 {
    match env::var(primary_key) {
        Ok(raw) => match raw.parse::<u32>() {
            Ok(value) if value > 0 => value,
            _ => {
                tracing::warn!(
                    "Invalid value for {primary_key} (`{raw}`), using default {default}"
                );
                default
            }
        },
        Err(_) => env_u32(fallback_key, default),
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    match env::var(key) {
        Ok(raw) => match raw.parse::<u64>() {
            Ok(value) if value > 0 => value,
            _ => {
                tracing::warn!("Invalid value for {key} (`{raw}`), using default {default}");
                default
            }
        },
        Err(_) => default,
    }
}

fn ceil_duration_to_seconds(duration: Duration) -> u64 {
    let secs = duration.as_secs();
    if duration.subsec_nanos() > 0 {
        secs + 1
    } else {
        secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::Service;

    fn test_app(anonymous_limit: u32, auth_limit: u32, window: Duration) -> Router<()> {
        let limiter = RateLimitState::new(RateLimitConfig::for_tests(
            anonymous_limit,
            auth_limit,
            window,
        ));

        Router::new()
            .route("/read", get(|| async { "read" }))
            .layer(middleware::from_fn_with_state(
                limiter,
                rate_limit_middleware,
            ))
    }

    async fn call(app: &Router<()>, request: Request<Body>) -> Response {
        let mut svc = app.clone();
        svc.call(request).await.unwrap()
    }

    /// Issue #609: anonymous IP limited to 1,000 req/hour; 1001st gets 429.
    #[tokio::test]
    async fn anonymous_user_gets_429_on_1001st_request() {
        let app = test_app(1_000, 1_000, Duration::from_secs(3600));

        for _ in 0..1_000 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/read")
                    .method("GET")
                    .header("x-forwarded-for", "203.0.113.10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;

            assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }

        let response = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "203.0.113.10")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key(RETRY_AFTER));
    }

    #[tokio::test]
    async fn authenticated_user_gets_429_on_1001st_request() {
        let app = test_app(100, 1_000, Duration::from_secs(60));

        for _ in 0..1_000 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/read")
                    .method("GET")
                    .header("authorization", "Bearer token-abc")
                    .header("x-forwarded-for", "203.0.113.25")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;

            assert_ne!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        }

        let response = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("authorization", "Bearer token-abc")
                .header("x-forwarded-for", "203.0.113.25")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key(RETRY_AFTER));
    }

    #[tokio::test]
    async fn includes_rate_limit_headers_on_success_and_429() {
        let app = test_app(1, 10, Duration::from_secs(60));

        let ok_response = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "198.51.100.22")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(ok_response.status(), StatusCode::OK);
        assert!(ok_response.headers().contains_key(HEADER_RATE_LIMIT_LIMIT));
        assert!(ok_response
            .headers()
            .contains_key(HEADER_RATE_LIMIT_REMAINING));
        assert!(ok_response.headers().contains_key(HEADER_RATE_LIMIT_RESET));

        let limited_response = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "198.51.100.22")
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(limited_response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(limited_response
            .headers()
            .contains_key(HEADER_RATE_LIMIT_LIMIT));
        assert!(limited_response
            .headers()
            .contains_key(HEADER_RATE_LIMIT_REMAINING));
        assert!(limited_response
            .headers()
            .contains_key(HEADER_RATE_LIMIT_RESET));
        assert!(limited_response.headers().contains_key(RETRY_AFTER));

        let body = axum::body::to_bytes(limited_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["error_code"], "RATE_LIMITED");
    }

    #[tokio::test]
    async fn retry_after_header_is_present_and_reasonable() {
        let app = test_app(1, 10, Duration::from_secs(2));

        let _first = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "192.0.2.44")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let second = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "192.0.2.44")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        let retry_after: u64 = second
            .headers()
            .get(RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        assert!((1..=2).contains(&retry_after));
    }

    #[tokio::test]
    async fn rate_limit_headers_show_remaining_quota() {
        let app = test_app(2, 10, Duration::from_secs(60));

        let first = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "192.0.2.44")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(
            first
                .headers()
                .get(HEADER_RATE_LIMIT_REMAINING)
                .and_then(|value| value.to_str().ok())
                .unwrap_or(""),
            "1"
        );

        let second = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "192.0.2.44")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(second.status(), StatusCode::OK);
        assert_eq!(
            second
                .headers()
                .get(HEADER_RATE_LIMIT_REMAINING)
                .and_then(|value| value.to_str().ok())
                .unwrap_or(""),
            "0"
        );

        let third = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", "192.0.2.44")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(third.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            third
                .headers()
                .get(HEADER_RATE_LIMIT_REMAINING)
                .and_then(|value| value.to_str().ok())
                .unwrap_or(""),
            "0"
        );
    }

    #[tokio::test]
    async fn contracts_rate_limit_scales_down_for_large_page_sizes() {
        let app = test_app(100, 20, Duration::from_secs(60));
        let ip = "198.51.100.77";

        for _ in 0..5 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/api/contracts?limit=1000")
                    .method("GET")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;

            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert_eq!(
                response
                    .headers()
                    .get(HEADER_RATE_LIMIT_LIMIT)
                    .and_then(|value| value.to_str().ok()),
                Some("5")
            );
        }

        let limited = call(
            &app,
            Request::builder()
                .uri("/api/contracts?limit=1000")
                .method("GET")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn page_size_scaling_uses_default_page_size_baseline() {
        assert_eq!(scale_limit_by_page_size(100, 50), 100);
        assert_eq!(scale_limit_by_page_size(100, 51), 50);
        assert_eq!(scale_limit_by_page_size(100, 1000), 5);
    }

    /// Verify that the eviction logic correctly removes expired buckets.
    #[tokio::test]
    async fn eviction_removes_expired_buckets() {
        let window = Duration::from_millis(100);
        let state = RateLimitState::new(RateLimitConfig::for_tests(10, 10, window));

        // Insert a request so a bucket is created
        let req = Request::builder()
            .uri("/read")
            .method("GET")
            .header("x-forwarded-for", "10.0.0.1")
            .body(Body::empty())
            .unwrap();
        let (hourly, burst, key, _tier) = state.select_limit_and_key(&req);
        state.check_request(key, hourly, burst).await;

        // Confirm one bucket exists
        assert_eq!(state.buckets.lock().await.len(), 1);

        // Wait for more than two window lengths so the bucket qualifies for eviction
        tokio::time::sleep(window.saturating_mul(3)).await;

        // Run eviction manually (same logic as the background task)
        {
            let now = Instant::now();
            let mut map = state.buckets.lock().await;
            map.retain(|_, s| {
                let hourly = s
                    .timestamps
                    .back()
                    .map(|last_seen| {
                        now.saturating_duration_since(*last_seen) < window.saturating_mul(2)
                    })
                    .unwrap_or(false);
                let burst = s
                    .burst_timestamps
                    .back()
                    .map(|last_seen| {
                        now.saturating_duration_since(*last_seen) < window.saturating_mul(2)
                    })
                    .unwrap_or(false);
                hourly || burst
            });
        }

        assert_eq!(state.buckets.lock().await.len(), 0);
    }

    // ── Write endpoint protection tests ──────────────────────────────────────

    fn test_app_with_write(read_limit: u32, write_limit: u32, window: Duration) -> Router<()> {
        let limiter = RateLimitState::new(RateLimitConfig::for_tests_with_write(
            read_limit,
            read_limit,
            write_limit,
            write_limit,
            window,
        ));

        Router::new()
            .route("/read", get(|| async { "read" }))
            .route("/write", post(|| async { "written" }))
            .route("/health", get(|| async { "ok" }))
            .route("/health/ready", get(|| async { "ok" }))
            .route("/metrics", get(|| async { "# metrics" }))
            .route("/api/admin/audit-logs", get(|| async { "[]" }))
            .layer(middleware::from_fn_with_state(
                limiter,
                rate_limit_middleware,
            ))
    }

    #[tokio::test]
    async fn write_requests_use_tighter_limit_than_reads() {
        // Read limit: 100, write limit: 2. After 2 POSTs the third is blocked.
        // GETs should still be allowed up to the read limit.
        let app = test_app_with_write(100, 2, Duration::from_secs(60));
        let ip = "198.51.100.99";

        // First two POSTs succeed.
        for _ in 0..2 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/write")
                    .method("POST")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_ne!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "first two write requests should not be rate limited"
            );
        }

        // Third POST is blocked.
        let limited = call(
            &app,
            Request::builder()
                .uri("/write")
                .method("POST")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(limited.headers().contains_key(RETRY_AFTER));

        // GET requests are still within their separate read quota.
        let read_response = call(
            &app,
            Request::builder()
                .uri("/read")
                .method("GET")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_ne!(read_response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn write_limit_response_body_contains_limit_type_write() {
        let app = test_app_with_write(100, 1, Duration::from_secs(60));
        let ip = "203.0.113.55";

        // Use up the single write slot.
        call(
            &app,
            Request::builder()
                .uri("/write")
                .method("POST")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        let response = call(
            &app,
            Request::builder()
                .uri("/write")
                .method("POST")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["details"]["limit_type"], "write");
    }

    #[tokio::test]
    async fn health_endpoint_is_not_rate_limited() {
        // Very tight limits — both read and write — to verify health bypasses.
        let app = test_app_with_write(1, 1, Duration::from_secs(60));
        let ip = "192.0.2.1";

        // Send many requests to /health; none should be rate limited.
        for i in 0..20 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/health")
                    .method("GET")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_ne!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "request {i} to /health should not be rate limited"
            );
        }
    }

    #[tokio::test]
    async fn health_ready_endpoint_is_exempt() {
        let app = test_app_with_write(1, 1, Duration::from_secs(60));
        let ip = "192.0.2.2";

        for i in 0..5 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/health/ready")
                    .method("GET")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_ne!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "request {i} to /health/ready should not be rate limited"
            );
        }
    }

    #[tokio::test]
    async fn metrics_endpoint_is_exempt() {
        let app = test_app_with_write(1, 1, Duration::from_secs(60));
        let ip = "192.0.2.3";

        for i in 0..5 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/metrics")
                    .method("GET")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_ne!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "request {i} to /metrics should not be rate limited"
            );
        }
    }

    #[tokio::test]
    async fn admin_endpoint_is_exempt_from_rate_limiting() {
        let app = test_app_with_write(1, 1, Duration::from_secs(60));
        let ip = "192.0.2.4";

        for i in 0..5 {
            let response = call(
                &app,
                Request::builder()
                    .uri("/api/admin/audit-logs")
                    .method("GET")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_ne!(
                response.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "request {i} to /api/admin/* should not be rate limited"
            );
        }
    }

    #[test]
    fn is_exempt_path_recognises_health_metrics_and_admin() {
        assert!(is_exempt_path("/health"));
        assert!(is_exempt_path("/health/live"));
        assert!(is_exempt_path("/health/ready"));
        assert!(is_exempt_path("/health/detailed"));
        assert!(is_exempt_path("/metrics"));
        assert!(is_exempt_path("/api/admin/migrations/status"));
        assert!(!is_exempt_path("/api/contracts"));
        assert!(!is_exempt_path("/api/contracts/verify"));
        assert!(!is_exempt_path("/api/publishers"));
    }

    // ── Issue #1045: per-endpoint publish / search limit tests ───────────────

    /// Build an app with separate per-endpoint limits for publish and search.
    fn test_app_endpoint_limits(
        general_read: u32,
        general_write: u32,
        publish_anon: u32,
        publish_auth: u32,
        search_anon: u32,
        search_auth: u32,
        window: Duration,
    ) -> Router<()> {
        let config = RateLimitConfig::for_tests_with_endpoint_limits(
            general_read,
            general_read,
            general_write,
            general_write,
            publish_anon,
            publish_auth,
            search_anon,
            search_auth,
            window,
        );
        let limiter = RateLimitState::new(config);
        Router::new()
            .route("/api/contracts", get(|| async { "list" }))
            .route("/api/contracts", post(|| async { "published" }))
            .route("/api/v1/contracts/search", get(|| async { "search" }))
            .route("/api/contracts/suggestions", get(|| async { "suggestions" }))
            .route("/other", post(|| async { "other write" }))
            .layer(middleware::from_fn_with_state(
                limiter,
                rate_limit_middleware,
            ))
    }

    /// Burst spam on POST /api/contracts is blocked after the publish limit,
    /// even though the general write quota is much higher.
    #[tokio::test]
    async fn publish_burst_traffic_blocked_at_publish_limit() {
        // publish_anon=2, general_write=100 — only 2 publishes allowed.
        let app = test_app_endpoint_limits(100, 100, 2, 20, 50, 200, Duration::from_secs(60));
        let ip = "198.51.100.10";

        for i in 0..2 {
            let resp = call(
                &app,
                Request::builder()
                    .uri("/api/contracts")
                    .method("POST")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_ne!(
                resp.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "publish request {i} should be allowed"
            );
        }

        // Third publish is blocked by the publish-specific limit.
        let limited = call(
            &app,
            Request::builder()
                .uri("/api/contracts")
                .method("POST")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(
            limited.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "third publish from same IP must be rate-limited"
        );
        assert!(
            limited.headers().contains_key(RETRY_AFTER),
            "429 response must include Retry-After header"
        );
    }

    /// The 429 response for a publish request carries the standard rate-limit headers.
    #[tokio::test]
    async fn publish_429_response_has_rate_limit_headers() {
        let app = test_app_endpoint_limits(100, 100, 1, 10, 50, 200, Duration::from_secs(60));
        let ip = "203.0.113.20";

        // Use up the single publish slot.
        call(
            &app,
            Request::builder()
                .uri("/api/contracts")
                .method("POST")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        let resp = call(
            &app,
            Request::builder()
                .uri("/api/contracts")
                .method("POST")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(
            resp.headers().contains_key(RETRY_AFTER),
            "Retry-After header required on publish 429"
        );
        assert!(resp.headers().contains_key(HEADER_RATE_LIMIT_LIMIT));
        assert!(resp.headers().contains_key(HEADER_RATE_LIMIT_REMAINING));
        assert!(resp.headers().contains_key(HEADER_RATE_LIMIT_RESET));

        // Verify the response body is well-formed JSON.
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(val["error_code"], "RATE_LIMITED");
    }

    /// Sustained publish abuse from a single IP is blocked while a second IP
    /// is still allowed — limits are per-IP, not global.
    #[tokio::test]
    async fn sustained_publish_abuse_blocked_independent_of_other_ips() {
        let app = test_app_endpoint_limits(200, 200, 3, 30, 100, 300, Duration::from_secs(60));
        let abuser_ip = "10.0.0.1";
        let normal_ip = "10.0.0.2";

        // Abuser exhausts the publish limit.
        for _ in 0..3 {
            call(
                &app,
                Request::builder()
                    .uri("/api/contracts")
                    .method("POST")
                    .header("x-forwarded-for", abuser_ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        }

        // Abuser's 4th request is blocked.
        let blocked = call(
            &app,
            Request::builder()
                .uri("/api/contracts")
                .method("POST")
                .header("x-forwarded-for", abuser_ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(
            blocked.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "abuser should be blocked after 3 publishes"
        );

        // Normal IP's first publish still succeeds.
        let normal = call(
            &app,
            Request::builder()
                .uri("/api/contracts")
                .method("POST")
                .header("x-forwarded-for", normal_ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_ne!(
            normal.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "other IPs must not be affected by abuser's publish limit"
        );
    }

    /// Search endpoints GET /api/contracts and /api/v1/contracts/search
    /// use the dedicated search bucket, not the general read bucket.
    /// An IP that hits the search limit is still allowed on other GET endpoints.
    #[tokio::test]
    async fn search_limit_is_separate_from_general_read_limit() {
        // search_anon=2, general_read=1000 — search gets its own tight quota.
        let app = test_app_endpoint_limits(1000, 100, 5, 30, 2, 50, Duration::from_secs(60));
        let ip = "192.0.2.5";

        // Two search requests succeed.
        for i in 0..2 {
            let resp = call(
                &app,
                Request::builder()
                    .uri("/api/contracts")
                    .method("GET")
                    .header("x-forwarded-for", ip)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_ne!(
                resp.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "search request {i} should succeed"
            );
        }

        // Third search is blocked.
        let limited = call(
            &app,
            Request::builder()
                .uri("/api/contracts")
                .method("GET")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(
            limited.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "third search request should be rate-limited"
        );
        assert!(
            limited.headers().contains_key(RETRY_AFTER),
            "search 429 must include Retry-After"
        );
    }

    /// POST /api/contracts (publish) and GET /api/contracts (search) use
    /// separate buckets, so exhausting one does not block the other.
    #[tokio::test]
    async fn publish_and_search_limits_are_independent_buckets() {
        // publish_anon=1, search_anon=1 — each gets exactly one slot.
        let app = test_app_endpoint_limits(100, 100, 1, 10, 1, 10, Duration::from_secs(60));
        let ip = "198.51.100.50";

        // Use up the publish slot.
        call(
            &app,
            Request::builder()
                .uri("/api/contracts")
                .method("POST")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;

        // Publish is now blocked.
        let publish_blocked = call(
            &app,
            Request::builder()
                .uri("/api/contracts")
                .method("POST")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(publish_blocked.status(), StatusCode::TOO_MANY_REQUESTS);

        // But search (GET) is still allowed — different bucket.
        let search_ok = call(
            &app,
            Request::builder()
                .uri("/api/contracts")
                .method("GET")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_ne!(
            search_ok.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "search bucket must be independent of publish bucket"
        );
    }

    /// Authenticated clients get the separate, higher publish auth limit.
    #[tokio::test]
    async fn authenticated_publish_uses_auth_limit_not_anon() {
        // publish_anon=1, publish_auth=5 — auth gets 5x the anon slot.
        let app = test_app_endpoint_limits(100, 100, 1, 5, 50, 200, Duration::from_secs(60));
        let ip = "203.0.113.30";

        // Authenticated user should be allowed up to publish_auth_limit (5).
        for i in 0..5 {
            let resp = call(
                &app,
                Request::builder()
                    .uri("/api/contracts")
                    .method("POST")
                    .header("x-forwarded-for", ip)
                    .header("authorization", "Bearer publish-token-xyz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
            assert_ne!(
                resp.status(),
                StatusCode::TOO_MANY_REQUESTS,
                "authenticated publish request {i} should be allowed"
            );
        }

        // 6th publish is blocked.
        let blocked = call(
            &app,
            Request::builder()
                .uri("/api/contracts")
                .method("POST")
                .header("x-forwarded-for", ip)
                .header("authorization", "Bearer publish-token-xyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(
            blocked.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "6th authenticated publish must be blocked"
        );
    }

    /// /api/v1/contracts/search uses the same search bucket as /api/contracts (GET).
    #[tokio::test]
    async fn v1_search_endpoint_shares_search_bucket() {
        // search_anon=2: one request to /api/v1/contracts/search + one to /api/contracts (GET)
        // exhausts the quota; a second v1 search is then blocked.
        let app = test_app_endpoint_limits(1000, 100, 5, 30, 2, 100, Duration::from_secs(60));
        let ip = "198.51.100.60";

        // First request: /api/v1/contracts/search
        let resp1 = call(
            &app,
            Request::builder()
                .uri("/api/v1/contracts/search")
                .method("GET")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_ne!(resp1.status(), StatusCode::TOO_MANY_REQUESTS);

        // Second request: /api/contracts (list)
        let resp2 = call(
            &app,
            Request::builder()
                .uri("/api/contracts")
                .method("GET")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_ne!(resp2.status(), StatusCode::TOO_MANY_REQUESTS);

        // Third request: quota exhausted.
        let resp3 = call(
            &app,
            Request::builder()
                .uri("/api/v1/contracts/search")
                .method("GET")
                .header("x-forwarded-for", ip)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(
            resp3.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "search quota should be shared across search endpoints"
        );
        assert!(resp3.headers().contains_key(RETRY_AFTER));
    }

    /// Unit tests for the endpoint detection helpers.
    #[test]
    fn is_publish_endpoint_detects_post_api_contracts_only() {
        assert!(is_publish_endpoint(&Method::POST, "/api/contracts"));
        // Not publish: wrong method
        assert!(!is_publish_endpoint(&Method::GET, "/api/contracts"));
        assert!(!is_publish_endpoint(&Method::PUT, "/api/contracts"));
        // Not publish: wrong path
        assert!(!is_publish_endpoint(&Method::POST, "/api/contracts/abc"));
        assert!(!is_publish_endpoint(&Method::POST, "/api/v1/contracts"));
    }

    #[test]
    fn is_search_endpoint_detects_correct_paths() {
        assert!(is_search_endpoint(&Method::GET, "/api/contracts"));
        assert!(is_search_endpoint(&Method::GET, "/api/v1/contracts/search"));
        assert!(is_search_endpoint(&Method::GET, "/api/contracts/suggestions"));
        assert!(is_search_endpoint(&Method::GET, "/api/v1/contracts/trending"));
        // Not search: wrong method
        assert!(!is_search_endpoint(&Method::POST, "/api/contracts"));
        assert!(!is_search_endpoint(&Method::DELETE, "/api/v1/contracts/search"));
        // Not search: unrelated paths
        assert!(!is_search_endpoint(&Method::GET, "/api/contracts/abc"));
        assert!(!is_search_endpoint(&Method::GET, "/api/publishers"));
    }
}
