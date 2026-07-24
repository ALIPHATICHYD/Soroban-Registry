//! Validation failure rate limiting
//!
//! This module tracks validation failures per client IP address and rate limits
//! clients that repeatedly send invalid data. This helps prevent attackers from
//! probing the API with malicious payloads.
//!
//! Configuration via environment variables:
//! - VALIDATION_FAILURE_LIMIT: Max failures before rate limiting (default: 20)
//! - VALIDATION_FAILURE_WINDOW_SECONDS: Time window for counting failures (default: 60)

use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const DEFAULT_VALIDATION_FAILURE_LIMIT: u32 = 20;
const DEFAULT_VALIDATION_FAILURE_WINDOW_SECONDS: u64 = 60;

/// State for tracking validation failures per IP
#[derive(Clone)]
pub struct ValidationFailureRateLimiter {
    config: ValidationFailureConfig,
    /// Map of IP -> (failures in current window, window start time)
    buckets: Arc<Mutex<HashMap<IpAddr, FailureBucket>>>,
}

#[derive(Debug, Clone)]
struct ValidationFailureConfig {
    max_failures: u32,
    window: Duration,
}

#[derive(Debug, Clone)]
struct FailureBucket {
    count: u32,
    window_start: Instant,
}

impl ValidationFailureRateLimiter {
    /// Create a new rate limiter from environment configuration
    pub fn from_env() -> Self {
        let max_failures = std::env::var("VALIDATION_FAILURE_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_VALIDATION_FAILURE_LIMIT);

        let window_seconds = std::env::var("VALIDATION_FAILURE_WINDOW_SECONDS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_VALIDATION_FAILURE_WINDOW_SECONDS);

        Self::new(max_failures, Duration::from_secs(window_seconds))
    }

    /// Create a new rate limiter with explicit configuration
    pub fn new(max_failures: u32, window: Duration) -> Self {
        Self {
            config: ValidationFailureConfig {
                max_failures,
                window,
            },
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if an IP has exceeded validation failure rate limit
    ///
    /// Returns:
    /// - `Ok(failure_count)` - Request is allowed, failure_count is current count
    /// - `Err(failure_count)` - Rate limit exceeded, failure_count is current count
    ///
    /// This should be called when a validation error occurs.
    pub fn check_and_increment(&self, ip: IpAddr) -> Result<u32, u32> {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().expect("rate limiter mutex poisoned");

        let bucket = buckets.entry(ip).or_insert_with(|| FailureBucket {
            count: 0,
            window_start: now,
        });

        // Reset bucket if window has expired
        if now.duration_since(bucket.window_start) >= self.config.window {
            bucket.window_start = now;
            bucket.count = 0;
        }

        bucket.count += 1;
        let current_count = bucket.count;

        if current_count > self.config.max_failures {
            Err(current_count)
        } else {
            Ok(current_count)
        }
    }

    /// Get current failure count for an IP (used for testing/monitoring)
    pub fn get_failure_count(&self, ip: IpAddr) -> u32 {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().expect("rate limiter mutex poisoned");

        if let Some(bucket) = buckets.get_mut(&ip) {
            // Reset if window expired
            if now.duration_since(bucket.window_start) >= self.config.window {
                bucket.window_start = now;
                bucket.count = 0;
            }
            bucket.count
        } else {
            0
        }
    }

    /// Clear an IP's failure count (useful for admin operations)
    pub fn reset_ip(&self, ip: IpAddr) {
        let mut buckets = self.buckets.lock().expect("rate limiter mutex poisoned");
        buckets.remove(&ip);
    }

    /// Clear all failure counts
    pub fn reset_all(&self) {
        let mut buckets = self.buckets.lock().expect("rate limiter mutex poisoned");
        buckets.clear();
    }

    /// Get configuration
    pub fn config(&self) -> (u32, Duration) {
        (self.config.max_failures, self.config.window)
    }
}

/// Response when validation failure rate limit is exceeded
#[derive(Debug, serde::Serialize)]
pub struct ValidationRateLimitExceeded {
    pub error: String,
    pub message: String,
    pub code: u16,
    pub retry_after_seconds: u64,
    pub correlation_id: String,
    pub timestamp: String,
}

impl ValidationRateLimitExceeded {
    pub fn new(retry_after_seconds: u64) -> Self {
        use chrono::{SecondsFormat, Utc};

        Self {
            error: "TooManyValidationFailures".to_string(),
            message: format!(
                "Too many validation failures. Please try again in {} seconds",
                retry_after_seconds
            ),
            code: 429,
            retry_after_seconds,
            correlation_id: crate::request_tracing::current_request_id()
                .unwrap_or_else(crate::request_tracing::generate_request_id),
            timestamp: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_validation_failure_count_increments() {
        let limiter = ValidationFailureRateLimiter::new(5, Duration::from_secs(60));
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        assert_eq!(limiter.check_and_increment(ip), Ok(1));
        assert_eq!(limiter.check_and_increment(ip), Ok(2));
        assert_eq!(limiter.check_and_increment(ip), Ok(3));
    }

    #[test]
    fn test_validation_failure_rate_limit_exceeded() {
        let limiter = ValidationFailureRateLimiter::new(3, Duration::from_secs(60));
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

        assert_eq!(limiter.check_and_increment(ip), Ok(1));
        assert_eq!(limiter.check_and_increment(ip), Ok(2));
        assert_eq!(limiter.check_and_increment(ip), Ok(3));
        assert_eq!(limiter.check_and_increment(ip), Err(4));
        assert_eq!(limiter.check_and_increment(ip), Err(5));
    }

    #[test]
    fn test_different_ips_independent() {
        let limiter = ValidationFailureRateLimiter::new(2, Duration::from_secs(60));
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

        assert_eq!(limiter.check_and_increment(ip1), Ok(1));
        assert_eq!(limiter.check_and_increment(ip1), Ok(2));
        assert_eq!(limiter.check_and_increment(ip1), Err(3));

        // ip2 should start fresh
        assert_eq!(limiter.check_and_increment(ip2), Ok(1));
        assert_eq!(limiter.check_and_increment(ip2), Ok(2));
        assert_eq!(limiter.check_and_increment(ip2), Err(3));
    }

    #[test]
    fn test_get_failure_count() {
        let limiter = ValidationFailureRateLimiter::new(5, Duration::from_secs(60));
        let ip = IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1));

        assert_eq!(limiter.get_failure_count(ip), 0);
        limiter.check_and_increment(ip).ok();
        assert_eq!(limiter.get_failure_count(ip), 1);
        limiter.check_and_increment(ip).ok();
        assert_eq!(limiter.get_failure_count(ip), 2);
    }

    #[test]
    fn test_reset_ip() {
        let limiter = ValidationFailureRateLimiter::new(5, Duration::from_secs(60));
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 2, 1));

        limiter.check_and_increment(ip).ok();
        limiter.check_and_increment(ip).ok();
        assert_eq!(limiter.get_failure_count(ip), 2);

        limiter.reset_ip(ip);
        assert_eq!(limiter.get_failure_count(ip), 0);
    }
}
