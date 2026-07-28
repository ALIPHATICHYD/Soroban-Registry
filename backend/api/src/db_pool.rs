//! Database connection pool statistics and monitoring (Issue #876).
//!
//! Exposes a single handler that returns a live snapshot of the pool state,
//! including utilization, idle/active connection counts, and the configured
//! pool bounds and timeouts.  Metrics are also pushed to the existing
//! Prometheus gauges so the data is available in dashboards.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::error::ApiError;
use crate::state::AppState;

/// Live snapshot of the connection pool configuration and utilisation.
#[derive(Debug, Serialize)]
pub struct PoolStats {
    /// Connections currently open (active + idle).
    pub total_connections: u32,
    /// Connections idle and available to serve requests.
    pub idle_connections: u32,
    /// Connections currently executing a query.
    pub active_connections: u32,
    /// Maximum connections the pool will open.
    pub max_connections: u32,
    /// Minimum connections the pool maintains when idle.
    pub min_connections: u32,
    /// Fraction of the pool in active use (0.0–1.0).
    pub utilization: f64,
    /// Fraction expressed as a percentage (0.0–100.0).
    pub utilization_pct: f64,
    /// How long (seconds) the pool waits for a free connection before erroring.
    pub acquire_timeout_secs: f64,
    /// Idle connections are closed after this many seconds (if set).
    pub idle_timeout_secs: Option<f64>,
    /// Connections are recycled after this many seconds (if set).
    pub max_lifetime_secs: Option<f64>,
}

/// GET /api/admin/db/pool-stats
///
/// Returns a live snapshot of the database connection pool.
/// Requires admin authentication (enforced by the route layer in routes.rs).
pub async fn get_pool_stats(State(state): State<AppState>) -> Result<Json<PoolStats>, ApiError> {
    let total = state.db.size();
    let idle = state.db.num_idle() as u32;
    let active = total.saturating_sub(idle);
    let opts = state.db.options();
    let max = opts.get_max_connections();
    let min = opts.get_min_connections();
    let utilization = if max > 0 {
        active as f64 / max as f64
    } else {
        0.0
    };
    let acquire_timeout_secs = opts.get_acquire_timeout().as_secs_f64();
    let idle_timeout_secs = opts.get_idle_timeout().map(|d| d.as_secs_f64());
    let max_lifetime_secs = opts.get_max_lifetime().map(|d| d.as_secs_f64());

    // Keep Prometheus gauges in sync with the point-in-time snapshot.
    crate::metrics::DB_CONNECTIONS_ACTIVE.set(active as i64);
    crate::metrics::DB_CONNECTIONS_IDLE.set(idle as i64);
    crate::metrics::DB_POOL_SIZE.set(total as i64);
    crate::metrics::DB_POOL_UTILIZATION
        .with_label_values(&["default"])
        .set(utilization);

    Ok(Json(PoolStats {
        total_connections: total,
        idle_connections: idle,
        active_connections: active,
        max_connections: max,
        min_connections: min,
        utilization,
        utilization_pct: utilization * 100.0,
        acquire_timeout_secs,
        idle_timeout_secs,
        max_lifetime_secs,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_stats_utilization_zero_max() {
        // Guard against division by zero when max is 0.
        let utilization = {
            let max: u32 = 0;
            let active: u32 = 0;
            if max > 0 {
                active as f64 / max as f64
            } else {
                0.0
            }
        };
        assert_eq!(utilization, 0.0);
    }

    #[test]
    fn pool_stats_utilization_calculation() {
        let max: u32 = 50;
        let active: u32 = 25;
        let utilization = active as f64 / max as f64;
        assert!((utilization - 0.5).abs() < f64::EPSILON);
        assert!((utilization * 100.0 - 50.0).abs() < f64::EPSILON);
    }
}
