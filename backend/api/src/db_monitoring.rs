use crate::cache::CacheLayer;
use crate::metrics;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use sqlx::Row;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Clone, Debug)]
pub struct ReplicationMonitorConfig {
    pub primary_url: String,
    pub replica_url: String,
    pub lag_threshold_ms: i64,
    pub check_interval: Duration,
}

pub fn spawn_db_monitoring_task(
    pool: PgPool,
    cache: Arc<CacheLayer>,
    replication_monitor: Option<ReplicationMonitorConfig>,
) {
    if let Some(config) = replication_monitor {
        spawn_replication_monitor_task(config);
    }

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let max_connections = pool.options().get_max_connections();

        loop {
            interval.tick().await;

            // Database Pool Metrics
            let total_connections = pool.size();
            let idle_connections = pool.num_idle() as u32;
            let active_connections = total_connections.saturating_sub(idle_connections);

            metrics::DB_CONNECTIONS_ACTIVE.set(active_connections as i64);
            metrics::DB_CONNECTIONS_IDLE.set(idle_connections as i64);
            metrics::DB_POOL_SIZE.set(total_connections as i64);

            let utilization = if max_connections > 0 {
                active_connections as f64 / max_connections as f64
            } else {
                0.0
            };

            metrics::DB_POOL_UTILIZATION
                .with_label_values(&["default"])
                .set(utilization);

            if utilization >= 0.8 {
                tracing::warn!(
                    utilization = %format!("{:.1}%", utilization * 100.0),
                    active = active_connections,
                    idle = idle_connections,
                    max = max_connections,
                    "High database pool utilization detected"
                );
            }

            // Moka Cache Metrics
            let abi_entries = cache.abi_cache.entry_count();
            let abi_size = cache.abi_cache.weighted_size();
            let ver_entries = cache.verification_cache.entry_count();
            let ver_size = cache.verification_cache.weighted_size();

            metrics::CACHE_ENTRIES.set(abi_entries.saturating_add(ver_entries) as i64);
            metrics::CACHE_SIZE_BYTES.set(abi_size.saturating_add(ver_size) as i64);

            tracing::debug!(
                db_active = active_connections,
                db_idle = idle_connections,
                cache_entries = abi_entries + ver_entries,
                "Resource monitoring update"
            );
        }
    });
}

fn spawn_replication_monitor_task(config: ReplicationMonitorConfig) {
    let primary_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&config.primary_url)
        .expect("failed to create lazy primary replication pool");
    let replica_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy(&config.replica_url)
        .expect("failed to create lazy replica replication pool");

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(config.check_interval);

        loop {
            interval.tick().await;
            metrics::DB_REPLICATION_CHECKS_TOTAL.inc();

            let replica_status = match sqlx::query(
                r#"
                SELECT
                    pg_is_in_recovery() AS in_recovery,
                    pg_last_wal_replay_lsn()::text AS replay_lsn,
                    COALESCE(EXTRACT(EPOCH FROM now() - pg_last_xact_replay_timestamp()) * 1000.0, 0) AS lag_ms
                "#,
            )
            .fetch_one(&replica_pool)
            .await
            {
                Ok(row) => {
                    let in_recovery: bool = row.try_get("in_recovery").unwrap_or(false);
                    let replay_lsn: Option<String> = row.try_get("replay_lsn").ok();
                    let lag_ms: f64 = row.try_get("lag_ms").unwrap_or(0.0);
                    Some((in_recovery, replay_lsn, lag_ms))
                }
                Err(err) => {
                    metrics::DB_REPLICATION_CHECK_FAILURES_TOTAL.inc();
                    metrics::DB_REPLICATION_HEALTH.set(0);
                    warn!(error = %err, "Replication monitor failed to query replica status");
                    None
                }
            };

            let Some((in_recovery, replay_lsn, lag_ms)) = replica_status else {
                continue;
            };

            let mut wal_lag_bytes = 0_i64;
            let mut healthy = in_recovery;

            if let Some(replay_lsn) = replay_lsn {
                match sqlx::query(
                    r#"
                    SELECT COALESCE(pg_wal_lsn_diff(pg_current_wal_lsn(), $1::pg_lsn), 0)::bigint AS wal_lag_bytes
                    "#,
                )
                .bind(&replay_lsn)
                .fetch_one(&primary_pool)
                .await
                {
                    Ok(row) => {
                        wal_lag_bytes = row.try_get::<i64, _>("wal_lag_bytes").unwrap_or(0);
                    }
                    Err(err) => {
                        metrics::DB_REPLICATION_CHECK_FAILURES_TOTAL.inc();
                        metrics::DB_REPLICATION_HEALTH.set(0);
                        warn!(
                            error = %err,
                            "Replication monitor failed to compute WAL lag"
                        );
                        continue;
                    }
                }
            } else {
                healthy = false;
            }

            metrics::DB_REPLICATION_LAG_MS.set(lag_ms.round() as i64);
            metrics::DB_REPLICATION_WAL_LAG_BYTES.set(wal_lag_bytes);

            if lag_ms > config.lag_threshold_ms as f64 {
                healthy = false;
                warn!(
                    lag_ms = lag_ms.round() as i64,
                    threshold_ms = config.lag_threshold_ms,
                    wal_lag_bytes,
                    "Replication lag exceeded target"
                );
            } else {
                info!(
                    lag_ms = lag_ms.round() as i64,
                    threshold_ms = config.lag_threshold_ms,
                    wal_lag_bytes,
                    "Replication lag within target"
                );
            }

            metrics::DB_REPLICATION_HEALTH.set(if healthy { 1 } else { 0 });
        }
    });
}

/// Helper to acquire a connection with latency tracking and slow acquisition logging
#[allow(dead_code)]
pub async fn acquire_with_metrics(
    pool: &PgPool,
) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, sqlx::Error> {
    let start = std::time::Instant::now();
    let res = pool.acquire().await;
    let duration = start.elapsed();
    let duration_ms = duration.as_millis() as f64;

    metrics::DB_CONNECTION_WAIT_MS
        .with_label_values(&["default"])
        .observe(duration_ms);

    if res.is_err() {
        metrics::DB_POOL_TIMEOUTS.inc();
    }

    if duration_ms > 100.0 {
        tracing::warn!(
            duration_ms = duration_ms,
            "Slow database connection acquisition"
        );
    }

    res
}
