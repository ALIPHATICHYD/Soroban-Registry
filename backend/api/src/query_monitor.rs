// Issue #878: Database query monitoring and performance analysis tools.
//
// Reads from pg_stat_statements (requires the extension) and from the
// query_performance_log table populated by the background task.  All
// pg_stat_statements queries degrade gracefully when the extension is absent.

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Duration;

use crate::error::ApiError;
use crate::state::AppState;

const DEFAULT_SLOW_THRESHOLD_MS: f64 = 1000.0;
const SNAPSHOT_INTERVAL_SECS: u64 = 60;

// ── Request params ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SlowQueryParams {
    pub threshold_ms: Option<f64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TrendParams {
    pub hours: Option<i64>,
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SlowQuery {
    pub query: Option<String>,
    pub calls: Option<i64>,
    pub mean_exec_time_ms: Option<f64>,
    pub max_exec_time_ms: Option<f64>,
    pub total_exec_time_ms: Option<f64>,
    pub rows: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IndexStat {
    pub schema_name: Option<String>,
    pub table_name: Option<String>,
    pub index_name: Option<String>,
    pub index_scans: Option<i64>,
    pub tuples_read: Option<i64>,
    pub tuples_fetched: Option<i64>,
    pub index_size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    pub pid: Option<i32>,
    pub relation: Option<String>,
    pub lock_type: Option<String>,
    pub lock_mode: Option<String>,
    pub granted: Option<bool>,
    pub query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PerformanceTrend {
    pub hour: Option<DateTime<Utc>>,
    pub total_calls: Option<i64>,
    pub avg_exec_time_ms: Option<f64>,
    pub slow_query_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub generated_at: DateTime<Utc>,
    pub slow_query_threshold_ms: f64,
    pub slow_queries: Vec<SlowQuery>,
    pub index_stats: Vec<IndexStat>,
    pub top_queries_by_frequency: Vec<SlowQuery>,
    pub recommendations: Vec<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_recommendations(slow: &[SlowQuery], indexes: &[IndexStat]) -> Vec<String> {
    let mut recs = Vec::new();

    for idx in indexes {
        if idx.index_scans == Some(0) {
            recs.push(format!(
                "Index '{}' on table '{}' has 0 scans. Consider dropping it to reduce write overhead.",
                idx.index_name.as_deref().unwrap_or("?"),
                idx.table_name.as_deref().unwrap_or("?"),
            ));
        }
    }

    for sq in slow {
        if sq.mean_exec_time_ms.unwrap_or(0.0) > 5000.0 {
            recs.push(format!(
                "Query with mean execution time {:.0} ms exceeds 5 s. Review for missing index or query rewrite.",
                sq.mean_exec_time_ms.unwrap_or(0.0),
            ));
        }
    }

    if recs.is_empty() {
        recs.push(
            "No immediate optimization recommendations. Continue monitoring query patterns."
                .to_string(),
        );
    }

    recs
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/admin/db/slow-queries
/// Lists queries whose mean execution time exceeds the threshold.
/// Requires pg_stat_statements extension; returns empty list if unavailable.
pub async fn get_slow_queries(
    State(state): State<AppState>,
    Query(params): Query<SlowQueryParams>,
) -> Result<Json<Vec<SlowQuery>>, ApiError> {
    let threshold = params.threshold_ms.unwrap_or(DEFAULT_SLOW_THRESHOLD_MS);
    let limit = params.limit.unwrap_or(50).clamp(1, 200);

    let rows = sqlx::query_as::<_, SlowQuery>(
        r#"
        SELECT
            query,
            calls,
            mean_exec_time    AS mean_exec_time_ms,
            max_exec_time     AS max_exec_time_ms,
            total_exec_time   AS total_exec_time_ms,
            rows
        FROM pg_stat_statements
        WHERE calls > 0
          AND mean_exec_time >= $1
        ORDER BY mean_exec_time DESC
        LIMIT $2
        "#,
    )
    .bind(threshold)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Ok(Json(rows))
}

/// GET /api/admin/db/index-stats
/// Returns index scan counts and sizes for all user indexes.
pub async fn get_index_stats(
    State(state): State<AppState>,
) -> Result<Json<Vec<IndexStat>>, ApiError> {
    let rows = sqlx::query_as::<_, IndexStat>(
        r#"
        SELECT
            schemaname                                    AS schema_name,
            relname                                       AS table_name,
            indexrelname                                  AS index_name,
            idx_scan                                      AS index_scans,
            idx_tup_read                                  AS tuples_read,
            idx_tup_fetch                                 AS tuples_fetched,
            pg_size_pretty(pg_relation_size(indexrelid))  AS index_size
        FROM pg_stat_user_indexes
        ORDER BY idx_scan DESC
        LIMIT 100
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("INDEX_STATS_ERROR", e.to_string()))?;

    Ok(Json(rows))
}

/// GET /api/admin/db/lock-monitor
/// Returns current locks and any contention visible in pg_locks.
pub async fn get_lock_monitor(
    State(state): State<AppState>,
) -> Result<Json<Vec<LockInfo>>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT
            l.pid,
            c.relname   AS relation,
            l.locktype  AS lock_type,
            l.mode      AS lock_mode,
            l.granted,
            a.query
        FROM pg_locks l
        LEFT JOIN pg_class c        ON l.relation = c.oid
        LEFT JOIN pg_stat_activity a ON l.pid     = a.pid
        WHERE l.pid != pg_backend_pid()
        ORDER BY l.granted, l.pid
        LIMIT 50
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("LOCK_MONITOR_ERROR", e.to_string()))?;

    use sqlx::Row as _;
    let locks = rows
        .into_iter()
        .map(|r| LockInfo {
            pid: r.try_get("pid").ok(),
            relation: r.try_get("relation").ok(),
            lock_type: r.try_get("lock_type").ok(),
            lock_mode: r.try_get("lock_mode").ok(),
            granted: r.try_get("granted").ok(),
            query: r.try_get("query").ok(),
        })
        .collect();

    Ok(Json(locks))
}

/// GET /api/admin/db/performance-trends
/// Returns hourly aggregated call counts and mean exec times from the
/// query_performance_log snapshot table.
pub async fn get_performance_trends(
    State(state): State<AppState>,
    Query(params): Query<TrendParams>,
) -> Result<Json<Vec<PerformanceTrend>>, ApiError> {
    let hours = params.hours.unwrap_or(24).clamp(1, 168) as f64;

    let rows = sqlx::query_as::<_, PerformanceTrend>(
        r#"
        SELECT
            date_trunc('hour', recorded_at)            AS hour,
            SUM(calls_delta)                           AS total_calls,
            AVG(mean_exec_time_ms)                     AS avg_exec_time_ms,
            COUNT(*) FILTER (WHERE is_slow = true)     AS slow_query_count
        FROM query_performance_log
        WHERE recorded_at >= NOW() - ($1 * INTERVAL '1 hour')
        GROUP BY date_trunc('hour', recorded_at)
        ORDER BY date_trunc('hour', recorded_at) ASC
        "#,
    )
    .bind(hours)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("TREND_ERROR", e.to_string()))?;

    Ok(Json(rows))
}

/// GET /api/admin/db/performance-report
/// Aggregate report: slow queries, index stats, top queries, and recommendations.
pub async fn get_performance_report(
    State(state): State<AppState>,
    Query(params): Query<SlowQueryParams>,
) -> Result<Json<PerformanceReport>, ApiError> {
    let threshold = params.threshold_ms.unwrap_or(DEFAULT_SLOW_THRESHOLD_MS);

    let slow_queries = sqlx::query_as::<_, SlowQuery>(
        r#"
        SELECT
            query,
            calls,
            mean_exec_time  AS mean_exec_time_ms,
            max_exec_time   AS max_exec_time_ms,
            total_exec_time AS total_exec_time_ms,
            rows
        FROM pg_stat_statements
        WHERE calls > 0
          AND mean_exec_time >= $1
        ORDER BY mean_exec_time DESC
        LIMIT 20
        "#,
    )
    .bind(threshold)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let top_queries = sqlx::query_as::<_, SlowQuery>(
        r#"
        SELECT
            query,
            calls,
            mean_exec_time  AS mean_exec_time_ms,
            max_exec_time   AS max_exec_time_ms,
            total_exec_time AS total_exec_time_ms,
            rows
        FROM pg_stat_statements
        WHERE calls > 0
        ORDER BY calls DESC
        LIMIT 10
        "#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let index_stats = sqlx::query_as::<_, IndexStat>(
        r#"
        SELECT
            schemaname                                    AS schema_name,
            relname                                       AS table_name,
            indexrelname                                  AS index_name,
            idx_scan                                      AS index_scans,
            idx_tup_read                                  AS tuples_read,
            idx_tup_fetch                                 AS tuples_fetched,
            pg_size_pretty(pg_relation_size(indexrelid))  AS index_size
        FROM pg_stat_user_indexes
        ORDER BY idx_scan ASC
        LIMIT 20
        "#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let recommendations = build_recommendations(&slow_queries, &index_stats);

    Ok(Json(PerformanceReport {
        generated_at: Utc::now(),
        slow_query_threshold_ms: threshold,
        slow_queries,
        index_stats,
        top_queries_by_frequency: top_queries,
        recommendations,
    }))
}

/// POST /api/admin/db/performance-report/export
/// Returns the full performance report as an exportable JSON envelope.
pub async fn export_performance_report(
    state: State<AppState>,
    query: Query<SlowQueryParams>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Json(report) = get_performance_report(state, query).await?;
    let json = serde_json::to_value(&report)
        .map_err(|e| ApiError::internal_error("EXPORT_ERROR", e.to_string()))?;

    Ok(Json(serde_json::json!({
        "format": "json",
        "exported_at": Utc::now(),
        "report": json
    })))
}

// ── Background snapshot task ──────────────────────────────────────────────────

/// Spawns a background task that periodically snapshots pg_stat_statements into
/// query_performance_log and logs a warning when slow queries are detected.
pub fn spawn_query_monitor_task(pool: PgPool, threshold_ms: f64) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(SNAPSHOT_INTERVAL_SECS));
        loop {
            interval.tick().await;

            let snapshot_result = sqlx::query(
                r#"
                INSERT INTO query_performance_log (
                    query_hash,
                    query_sample,
                    calls_delta,
                    mean_exec_time_ms,
                    max_exec_time_ms,
                    total_rows,
                    is_slow,
                    recorded_at
                )
                SELECT
                    md5(query)        AS query_hash,
                    LEFT(query, 500)  AS query_sample,
                    calls             AS calls_delta,
                    mean_exec_time    AS mean_exec_time_ms,
                    max_exec_time     AS max_exec_time_ms,
                    rows              AS total_rows,
                    mean_exec_time >= $1 AS is_slow,
                    NOW()
                FROM pg_stat_statements
                WHERE calls > 0
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(threshold_ms)
            .execute(&pool)
            .await;

            if let Err(e) = snapshot_result {
                tracing::debug!(error = %e, "pg_stat_statements snapshot skipped (extension may be unavailable)");
                continue;
            }

            let slow_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM pg_stat_statements WHERE mean_exec_time >= $1 AND calls > 0",
            )
            .bind(threshold_ms)
            .fetch_one(&pool)
            .await
            .unwrap_or(0);

            if slow_count > 0 {
                tracing::warn!(
                    slow_query_count = slow_count,
                    threshold_ms = threshold_ms,
                    "Slow queries detected above threshold"
                );
                crate::metrics::DB_QUERY_ERRORS.inc();
            }
        }
    });
}
