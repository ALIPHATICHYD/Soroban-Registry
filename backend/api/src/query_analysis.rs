// Issue #887: Application-side database query logging and analysis.
//
// Where issue #878 reads the *server's* view via pg_stat_statements, this module
// captures the *application's* own queries as they execute and analyses them:
//
//   • Logs every query with its execution time (via the sqlx tracing layer), so
//     no call-site changes are needed and overhead is a single in-memory update.
//   • Logs the *normalized* statement only — literals and bind placeholders are
//     collapsed to `?`, so bound parameter values (and any secrets) are never
//     recorded.
//   • Tracks per-pattern frequency, mean/max timing, and slow-call counts.
//   • Detects N+1 problems: the same normalized pattern firing many times within
//     a short sliding window.
//   • Emits slow-query alerts and Prometheus counters above a threshold.
//   • Flushes per-pattern deltas to `query_pattern_log` for historical trends and
//     persists detected N+1 bursts to `query_nplus1_incidents`.
//   • Serves frequency / slow / trend / N+1 reports and an EXPLAIN endpoint.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::{Context, Layer};

use crate::error::ApiError;
use crate::state::AppState;

// ── Configuration ─────────────────────────────────────────────────────────────

/// Slow-query threshold in milliseconds (env `DB_QUERY_SLOW_THRESHOLD_MS`).
fn slow_threshold_ms() -> f64 {
    std::env::var("DB_QUERY_SLOW_THRESHOLD_MS")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| *v > 0.0)
        .unwrap_or(500.0)
}

/// Max distinct events retained for N+1 sliding-window analysis.
const RING_CAPACITY: usize = 20_000;
/// Default N+1 window and occurrence threshold.
const DEFAULT_NPLUS1_WINDOW_MS: u64 = 1_000;
const DEFAULT_NPLUS1_THRESHOLD: usize = 10;
/// Truncate stored statement samples to this many chars.
const SAMPLE_MAX_LEN: usize = 500;

// ── Normalization ─────────────────────────────────────────────────────────────

static RE_STRING: Lazy<Regex> = Lazy::new(|| Regex::new(r"'(?:[^']|'')*'").unwrap());
static RE_DOLLAR: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\d+").unwrap());
static RE_NUMBER: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b\d+(\.\d+)?\b").unwrap());
static RE_WS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s+").unwrap());
static RE_IN_LIST: Lazy<Regex> = Lazy::new(|| Regex::new(r"\(\s*\?(\s*,\s*\?)+\s*\)").unwrap());

/// Collapse a SQL statement to a stable, parameter-free pattern. This both
/// produces a fingerprint for aggregation and guarantees no literal values
/// (potential secrets) are retained.
pub fn normalize_sql(sql: &str) -> String {
    let sql = sql.trim();
    let s = RE_STRING.replace_all(sql, "?");
    let s = RE_DOLLAR.replace_all(&s, "?");
    let s = RE_NUMBER.replace_all(&s, "?");
    let s = RE_IN_LIST.replace_all(&s, "(?)");
    let s = RE_WS.replace_all(&s, " ");
    s.trim().to_string()
}

/// Deterministic 64-bit FNV-1a hash (stable across process restarts, unlike the
/// default SipHash), hex-encoded for use as a persisted fingerprint.
fn fingerprint(normalized: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in normalized.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut end = max;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

// ── In-memory analyzer ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct PatternStat {
    pub fingerprint: String,
    pub pattern: String,
    pub calls: u64,
    pub total_time_ms: f64,
    pub mean_time_ms: f64,
    pub max_time_ms: f64,
    pub slow_calls: u64,
}

struct PatternEntry {
    pattern: String,
    calls: u64,
    total_time_ms: f64,
    max_time_ms: f64,
    slow_calls: u64,
    // Counts already flushed to the DB, so we can compute deltas.
    flushed_calls: u64,
    flushed_time_ms: f64,
    flushed_slow: u64,
}

struct RingItem {
    fp: String,
    at: Instant,
}

#[derive(Default)]
struct AnalyzerInner {
    patterns: HashMap<String, PatternEntry>,
    recent: VecDeque<RingItem>,
    total_calls: u64,
    total_slow: u64,
}

pub struct QueryAnalyzer {
    inner: Mutex<AnalyzerInner>,
    enabled: bool,
}

/// Process-wide analyzer fed by the sqlx capture layer.
pub static ANALYZER: Lazy<QueryAnalyzer> = Lazy::new(|| QueryAnalyzer {
    inner: Mutex::new(AnalyzerInner::default()),
    enabled: std::env::var("DB_QUERY_ANALYSIS_ENABLED")
        .map(|v| v != "false" && v != "0")
        .unwrap_or(true),
});

impl QueryAnalyzer {
    /// Record one observed query execution. Cheap: one short lock + map update.
    pub fn record(&self, sql: &str, elapsed_ms: f64, _rows: Option<i64>) {
        if !self.enabled {
            return;
        }
        let normalized = normalize_sql(sql);
        if normalized.is_empty() {
            return;
        }
        let fp = fingerprint(&normalized);
        let threshold = slow_threshold_ms();
        let is_slow = elapsed_ms >= threshold;

        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.total_calls += 1;
        if is_slow {
            inner.total_slow += 1;
        }
        let entry = inner
            .patterns
            .entry(fp.clone())
            .or_insert_with(|| PatternEntry {
                pattern: truncate(&normalized, SAMPLE_MAX_LEN),
                calls: 0,
                total_time_ms: 0.0,
                max_time_ms: 0.0,
                slow_calls: 0,
                flushed_calls: 0,
                flushed_time_ms: 0.0,
                flushed_slow: 0,
            });
        entry.calls += 1;
        entry.total_time_ms += elapsed_ms;
        if elapsed_ms > entry.max_time_ms {
            entry.max_time_ms = elapsed_ms;
        }
        if is_slow {
            entry.slow_calls += 1;
        }

        if inner.recent.len() >= RING_CAPACITY {
            inner.recent.pop_front();
        }
        inner.recent.push_back(RingItem {
            fp,
            at: Instant::now(),
        });

        // Drop the lock before doing the (rare) slow-query alert side effects.
        drop(inner);

        if is_slow {
            crate::metrics::DB_SLOW_QUERIES.inc();
            tracing::warn!(
                target: "query_analysis",
                elapsed_ms = elapsed_ms,
                threshold_ms = threshold,
                pattern = %truncate(&normalized, 200),
                "slow query detected"
            );
        }
        crate::metrics::DB_QUERIES_OBSERVED.inc();
    }

    fn snapshot(&self) -> Vec<PatternStat> {
        let Ok(inner) = self.inner.lock() else {
            return Vec::new();
        };
        inner
            .patterns
            .iter()
            .map(|(fp, e)| PatternStat {
                fingerprint: fp.clone(),
                pattern: e.pattern.clone(),
                calls: e.calls,
                total_time_ms: e.total_time_ms,
                mean_time_ms: if e.calls > 0 {
                    e.total_time_ms / e.calls as f64
                } else {
                    0.0
                },
                max_time_ms: e.max_time_ms,
                slow_calls: e.slow_calls,
            })
            .collect()
    }

    fn totals(&self) -> (u64, u64, usize) {
        let Ok(inner) = self.inner.lock() else {
            return (0, 0, 0);
        };
        (inner.total_calls, inner.total_slow, inner.patterns.len())
    }

    /// Detect N+1 candidates: the same fingerprint firing `>= threshold` times
    /// inside any `window` sliding window across the recent-event ring.
    fn detect_nplus1(&self, window: Duration, threshold: usize) -> Vec<NPlusOneFinding> {
        let Ok(inner) = self.inner.lock() else {
            return Vec::new();
        };
        // Group recent timestamps by fingerprint.
        let mut by_fp: HashMap<&str, Vec<Instant>> = HashMap::new();
        for item in &inner.recent {
            by_fp.entry(item.fp.as_str()).or_default().push(item.at);
        }
        let mut findings = Vec::new();
        for (fp, mut times) in by_fp {
            if times.len() < threshold {
                continue;
            }
            times.sort();
            // Sliding window: max count of timestamps within `window`.
            let mut max_in_window = 0usize;
            let mut start = 0usize;
            for end in 0..times.len() {
                while times[end].duration_since(times[start]) > window {
                    start += 1;
                }
                max_in_window = max_in_window.max(end - start + 1);
            }
            if max_in_window >= threshold {
                let pattern = inner
                    .patterns
                    .get(fp)
                    .map(|e| e.pattern.clone())
                    .unwrap_or_default();
                findings.push(NPlusOneFinding {
                    fingerprint: fp.to_string(),
                    pattern,
                    occurrence_count: max_in_window as i64,
                    window_ms: window.as_millis() as i64,
                });
            }
        }
        findings.sort_by(|a, b| b.occurrence_count.cmp(&a.occurrence_count));
        findings
    }

    /// Compute and consume per-pattern deltas since the last flush.
    fn drain_deltas(&self) -> Vec<PatternDelta> {
        let Ok(mut inner) = self.inner.lock() else {
            return Vec::new();
        };
        let mut deltas = Vec::new();
        for (fp, e) in inner.patterns.iter_mut() {
            let calls_delta = e.calls - e.flushed_calls;
            if calls_delta == 0 {
                continue;
            }
            let time_delta = e.total_time_ms - e.flushed_time_ms;
            let slow_delta = e.slow_calls - e.flushed_slow;
            deltas.push(PatternDelta {
                fingerprint: fp.clone(),
                pattern: e.pattern.clone(),
                calls_delta: calls_delta as i64,
                total_time_ms: time_delta,
                mean_time_ms: if calls_delta > 0 {
                    time_delta / calls_delta as f64
                } else {
                    0.0
                },
                max_time_ms: e.max_time_ms,
                slow_calls: slow_delta as i64,
            });
            e.flushed_calls = e.calls;
            e.flushed_time_ms = e.total_time_ms;
            e.flushed_slow = e.slow_calls;
        }
        deltas
    }

    fn reset(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            *inner = AnalyzerInner::default();
        }
    }
}

struct PatternDelta {
    fingerprint: String,
    pattern: String,
    calls_delta: i64,
    total_time_ms: f64,
    mean_time_ms: f64,
    max_time_ms: f64,
    slow_calls: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct NPlusOneFinding {
    pub fingerprint: String,
    pub pattern: String,
    pub occurrence_count: i64,
    pub window_ms: i64,
}

// ── sqlx tracing capture layer ────────────────────────────────────────────────

#[derive(Default)]
struct QueryVisitor {
    statement: Option<String>,
    summary: Option<String>,
    elapsed_secs: Option<f64>,
    elapsed_dbg: Option<String>,
    rows: Option<i64>,
}

impl Visit for QueryVisitor {
    fn record_f64(&mut self, field: &Field, value: f64) {
        if field.name() == "elapsed_secs" {
            self.elapsed_secs = Some(value);
        }
    }
    fn record_u64(&mut self, field: &Field, value: u64) {
        match field.name() {
            "rows_returned" => self.rows = Some(value as i64),
            "rows_affected" if self.rows.is_none() => self.rows = Some(value as i64),
            _ => {}
        }
    }
    fn record_i64(&mut self, field: &Field, value: i64) {
        match field.name() {
            "rows_returned" => self.rows = Some(value),
            "rows_affected" if self.rows.is_none() => self.rows = Some(value),
            _ => {}
        }
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "db.statement" => self.statement = Some(value.to_string()),
            "summary" if self.summary.is_none() => self.summary = Some(value.to_string()),
            _ => {}
        }
    }
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let s = format!("{value:?}");
        // Debug of a Display-wrapped field has no surrounding quotes; trim any.
        let s = s.trim_matches('"').to_string();
        match field.name() {
            "db.statement" if self.statement.is_none() => self.statement = Some(s),
            "summary" if self.summary.is_none() => self.summary = Some(s),
            "elapsed" => self.elapsed_dbg = Some(s),
            _ => {}
        }
    }
}

/// Parse sqlx's human-readable `elapsed` Debug string (e.g. "1.23ms", "950µs",
/// "2.1s") into milliseconds, as a fallback when `elapsed_secs` is absent.
fn parse_elapsed_dbg(s: &str) -> Option<f64> {
    let s = s.trim();
    let (num, unit) = if let Some(v) = s.strip_suffix("ns") {
        (v, 1e-6)
    } else if let Some(v) = s.strip_suffix("µs").or_else(|| s.strip_suffix("us")) {
        (v, 1e-3)
    } else if let Some(v) = s.strip_suffix("ms") {
        (v, 1.0)
    } else if let Some(v) = s.strip_suffix('s') {
        (v, 1000.0)
    } else {
        return None;
    };
    num.trim().parse::<f64>().ok().map(|n| n * unit)
}

struct QueryCaptureLayer;

impl<S> Layer<S> for QueryCaptureLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut v = QueryVisitor::default();
        event.record(&mut v);

        let elapsed_ms = v
            .elapsed_secs
            .map(|s| s * 1000.0)
            .or_else(|| v.elapsed_dbg.as_deref().and_then(parse_elapsed_dbg));
        let (Some(elapsed_ms), Some(stmt)) = (elapsed_ms, v.statement.or(v.summary)) else {
            return;
        };
        ANALYZER.record(stmt.trim(), elapsed_ms, v.rows);
    }
}

/// Build the tracing layer that captures sqlx query events for analysis. It is
/// filtered to the `sqlx::query` target only, independent of `RUST_LOG`, so the
/// analyzer always sees every query while console/OTEL output stays unchanged.
pub fn capture_layer<S>() -> impl Layer<S>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    use tracing_subscriber::filter::{LevelFilter, Targets};
    // TRACE so the capture catches sqlx query events at whatever level sqlx emits
    // them (DEBUG for normal, WARN for slow), independent of RUST_LOG.
    QueryCaptureLayer.with_filter(Targets::new().with_target("sqlx::query", LevelFilter::TRACE))
}

// ── Request / response DTOs ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LimitParams {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct NPlusOneParams {
    pub window_ms: Option<u64>,
    pub threshold: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct TrendParams {
    pub hours: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct QueryStatsSummary {
    pub total_queries: i64,
    pub slow_queries: i64,
    pub distinct_patterns: i64,
    pub slow_threshold_ms: f64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct QueryTrendBucket {
    pub hour: Option<DateTime<Utc>>,
    pub total_calls: Option<i64>,
    pub avg_time_ms: Option<f64>,
    pub slow_calls: Option<i64>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct NPlusOneIncident {
    pub id: i64,
    pub fingerprint: String,
    pub pattern_sample: String,
    pub occurrence_count: i32,
    pub window_ms: i64,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct QueryReport {
    pub generated_at: DateTime<Utc>,
    pub summary: QueryStatsSummary,
    pub most_frequent: Vec<PatternStat>,
    pub slowest: Vec<PatternStat>,
    pub n_plus_one: Vec<NPlusOneFinding>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExplainRequest {
    pub sql: String,
    #[serde(default)]
    pub analyze: bool,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/admin/db/queries/stats
pub async fn get_query_stats(State(_state): State<AppState>) -> Json<QueryStatsSummary> {
    let (total, slow, distinct) = ANALYZER.totals();
    Json(QueryStatsSummary {
        total_queries: total as i64,
        slow_queries: slow as i64,
        distinct_patterns: distinct as i64,
        slow_threshold_ms: slow_threshold_ms(),
    })
}

/// GET /api/admin/db/queries/frequent
pub async fn get_frequent_queries(
    State(_state): State<AppState>,
    Query(p): Query<LimitParams>,
) -> Json<Vec<PatternStat>> {
    let limit = p.limit.unwrap_or(20).clamp(1, 200);
    let mut stats = ANALYZER.snapshot();
    stats.sort_by(|a, b| b.calls.cmp(&a.calls));
    stats.truncate(limit);
    Json(stats)
}

/// GET /api/admin/db/queries/slow
pub async fn get_slow_queries(
    State(_state): State<AppState>,
    Query(p): Query<LimitParams>,
) -> Json<Vec<PatternStat>> {
    let limit = p.limit.unwrap_or(20).clamp(1, 200);
    let mut stats: Vec<PatternStat> = ANALYZER
        .snapshot()
        .into_iter()
        .filter(|s| s.slow_calls > 0)
        .collect();
    stats.sort_by(|a, b| {
        b.mean_time_ms
            .partial_cmp(&a.mean_time_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    stats.truncate(limit);
    Json(stats)
}

/// GET /api/admin/db/queries/n-plus-one
pub async fn get_nplus1(
    State(_state): State<AppState>,
    Query(p): Query<NPlusOneParams>,
) -> Json<Vec<NPlusOneFinding>> {
    let window = Duration::from_millis(
        p.window_ms
            .unwrap_or(DEFAULT_NPLUS1_WINDOW_MS)
            .clamp(10, 60_000),
    );
    let threshold = p
        .threshold
        .unwrap_or(DEFAULT_NPLUS1_THRESHOLD)
        .clamp(2, 10_000);
    Json(ANALYZER.detect_nplus1(window, threshold))
}

/// GET /api/admin/db/queries/trends
pub async fn get_query_trends(
    State(state): State<AppState>,
    Query(p): Query<TrendParams>,
) -> Result<Json<Vec<QueryTrendBucket>>, ApiError> {
    let hours = p.hours.unwrap_or(24).clamp(1, 168) as f64;
    let rows = sqlx::query_as::<_, QueryTrendBucket>(
        r#"
        SELECT
            date_trunc('hour', recorded_at)         AS hour,
            SUM(calls_delta)                        AS total_calls,
            CASE WHEN SUM(calls_delta) > 0
                 THEN SUM(total_time_ms) / SUM(calls_delta)
                 ELSE 0 END                         AS avg_time_ms,
            SUM(slow_calls)                         AS slow_calls
        FROM query_pattern_log
        WHERE recorded_at >= NOW() - ($1 * INTERVAL '1 hour')
        GROUP BY date_trunc('hour', recorded_at)
        ORDER BY date_trunc('hour', recorded_at) ASC
        "#,
    )
    .bind(hours)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("QUERY_TREND_ERROR", e.to_string()))?;
    Ok(Json(rows))
}

/// GET /api/admin/db/queries/incidents
pub async fn get_nplus1_incidents(
    State(state): State<AppState>,
    Query(p): Query<LimitParams>,
) -> Result<Json<Vec<NPlusOneIncident>>, ApiError> {
    let limit = p.limit.unwrap_or(50).clamp(1, 500) as i64;
    let rows = sqlx::query_as::<_, NPlusOneIncident>(
        "SELECT id, fingerprint, pattern_sample, occurrence_count, window_ms, detected_at
         FROM query_nplus1_incidents ORDER BY detected_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("INCIDENT_LIST_ERROR", e.to_string()))?;
    Ok(Json(rows))
}

/// GET /api/admin/db/queries/report
pub async fn get_query_report(State(_state): State<AppState>) -> Json<QueryReport> {
    let (total, slow, distinct) = ANALYZER.totals();
    let snapshot = ANALYZER.snapshot();

    let mut most_frequent = snapshot.clone();
    most_frequent.sort_by(|a, b| b.calls.cmp(&a.calls));
    most_frequent.truncate(10);

    let mut slowest: Vec<PatternStat> = snapshot;
    slowest.sort_by(|a, b| {
        b.mean_time_ms
            .partial_cmp(&a.mean_time_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    slowest.truncate(10);

    let n_plus_one = ANALYZER.detect_nplus1(
        Duration::from_millis(DEFAULT_NPLUS1_WINDOW_MS),
        DEFAULT_NPLUS1_THRESHOLD,
    );

    let recommendations = build_recommendations(&slowest, &n_plus_one, slow);

    Json(QueryReport {
        generated_at: Utc::now(),
        summary: QueryStatsSummary {
            total_queries: total as i64,
            slow_queries: slow as i64,
            distinct_patterns: distinct as i64,
            slow_threshold_ms: slow_threshold_ms(),
        },
        most_frequent,
        slowest,
        n_plus_one,
        recommendations,
    })
}

/// POST /api/admin/db/queries/reset — clear in-memory analysis state.
pub async fn reset_query_stats(State(_state): State<AppState>) -> Json<serde_json::Value> {
    ANALYZER.reset();
    Json(serde_json::json!({ "status": "reset" }))
}

/// POST /api/admin/db/queries/explain — capture the execution plan for a query.
///
/// Read-only: only a single SELECT/WITH statement is permitted. `analyze=true`
/// actually executes the query (inside a rolled-back transaction) for real
/// timings; the default is plan-only and never executes.
pub async fn explain_query(
    State(state): State<AppState>,
    Json(req): Json<ExplainRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let sql = req.sql.trim().trim_end_matches(';').trim();
    let lowered = sql.to_ascii_lowercase();

    if sql.is_empty() {
        return Err(ApiError::bad_request("EMPTY_SQL", "sql is required"));
    }
    // Single statement only.
    if sql.contains(';') {
        return Err(ApiError::bad_request(
            "MULTIPLE_STATEMENTS",
            "only a single statement may be explained",
        ));
    }
    // Read-only: must start with SELECT or WITH.
    if !(lowered.starts_with("select") || lowered.starts_with("with")) {
        return Err(ApiError::bad_request(
            "READ_ONLY_ONLY",
            "only SELECT/WITH statements can be explained",
        ));
    }
    // Defense-in-depth against data-modifying keywords slipping through a CTE.
    for kw in [
        "insert ",
        "update ",
        "delete ",
        "drop ",
        "alter ",
        "truncate ",
        "create ",
        "grant ",
    ] {
        if lowered.contains(kw) {
            return Err(ApiError::bad_request(
                "WRITE_KEYWORD_REJECTED",
                "data-modifying statements cannot be explained",
            ));
        }
    }

    let explain = if req.analyze {
        format!("EXPLAIN (FORMAT JSON, ANALYZE, BUFFERS) {sql}")
    } else {
        format!("EXPLAIN (FORMAT JSON) {sql}")
    };

    // Run inside a transaction so ANALYZE side effects are always rolled back.
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| ApiError::internal_error("EXPLAIN_TX_ERROR", e.to_string()))?;

    let plan: serde_json::Value = sqlx::query_scalar(&explain)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ApiError::bad_request("EXPLAIN_FAILED", e.to_string()))?;

    let _ = tx.rollback().await;

    Ok(Json(serde_json::json!({
        "analyzed": req.analyze,
        "normalized_pattern": normalize_sql(sql),
        "plan": plan,
    })))
}

fn build_recommendations(
    slowest: &[PatternStat],
    nplus1: &[NPlusOneFinding],
    slow_total: u64,
) -> Vec<String> {
    let mut recs = Vec::new();
    for f in nplus1 {
        recs.push(format!(
            "Possible N+1: pattern fired {} times within {} ms — consider batching or a JOIN. Pattern: {}",
            f.occurrence_count, f.window_ms, f.pattern
        ));
    }
    for s in slowest.iter().take(3) {
        if s.mean_time_ms >= slow_threshold_ms() {
            recs.push(format!(
                "Slow pattern (mean {:.1} ms over {} calls) — review indexes or rewrite. Pattern: {}",
                s.mean_time_ms, s.calls, s.pattern
            ));
        }
    }
    if recs.is_empty() {
        recs.push(format!(
            "No query issues detected ({slow_total} slow calls so far). Continue monitoring."
        ));
    }
    recs
}

// ── Background flush + N+1 persistence task ────────────────────────────────────

/// Spawn the background task that periodically flushes per-pattern deltas to
/// `query_pattern_log` and persists detected N+1 bursts to
/// `query_nplus1_incidents`. Interval is `DB_QUERY_FLUSH_INTERVAL_SECS`
/// (default 60s).
pub fn spawn_query_analysis_task(pool: PgPool) {
    let interval_secs = std::env::var("DB_QUERY_FLUSH_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(60);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        interval.tick().await; // skip immediate first tick
        loop {
            interval.tick().await;

            // 1) Flush per-pattern deltas.
            let deltas = ANALYZER.drain_deltas();
            for d in &deltas {
                let res = sqlx::query(
                    r#"
                    INSERT INTO query_pattern_log
                        (fingerprint, pattern_sample, calls_delta, total_time_ms,
                         mean_time_ms, max_time_ms, slow_calls)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    "#,
                )
                .bind(&d.fingerprint)
                .bind(&d.pattern)
                .bind(d.calls_delta)
                .bind(d.total_time_ms)
                .bind(d.mean_time_ms)
                .bind(d.max_time_ms)
                .bind(d.slow_calls)
                .execute(&pool)
                .await;
                if let Err(e) = res {
                    tracing::debug!(target: "query_analysis", error = %e, "query_pattern_log flush failed");
                }
            }

            // 2) Persist N+1 incidents (deduped by recent window in SQL).
            let findings = ANALYZER.detect_nplus1(
                Duration::from_millis(DEFAULT_NPLUS1_WINDOW_MS),
                DEFAULT_NPLUS1_THRESHOLD,
            );
            for f in &findings {
                tracing::warn!(
                    target: "query_analysis",
                    fingerprint = %f.fingerprint,
                    occurrences = f.occurrence_count,
                    window_ms = f.window_ms,
                    "N+1 query pattern detected"
                );
                // Avoid duplicate rows for the same fingerprint within one window.
                let res = sqlx::query(
                    r#"
                    INSERT INTO query_nplus1_incidents
                        (fingerprint, pattern_sample, occurrence_count, window_ms)
                    SELECT $1, $2, $3, $4
                    WHERE NOT EXISTS (
                        SELECT 1 FROM query_nplus1_incidents
                        WHERE fingerprint = $1
                          AND detected_at >= NOW() - INTERVAL '5 minutes'
                    )
                    "#,
                )
                .bind(&f.fingerprint)
                .bind(&f.pattern)
                .bind(f.occurrence_count as i32)
                .bind(f.window_ms)
                .execute(&pool)
                .await;
                if let Err(e) = res {
                    tracing::debug!(target: "query_analysis", error = %e, "n+1 incident insert failed");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_collapses_literals_and_params() {
        let a = normalize_sql("SELECT * FROM contracts WHERE id = '123' AND n = 5");
        let b = normalize_sql("SELECT * FROM contracts WHERE id = '999' AND n = 42");
        assert_eq!(a, b);
        assert!(a.contains("id = ?"));
    }

    #[test]
    fn normalize_collapses_dollar_params_and_in_lists() {
        let a = normalize_sql("SELECT * FROM t WHERE id = $1 AND x IN (1, 2, 3, 4)");
        assert_eq!(a, "SELECT * FROM t WHERE id = ? AND x IN (?)");
    }

    #[test]
    fn fingerprint_is_stable_and_distinct() {
        assert_eq!(fingerprint("select ?"), fingerprint("select ?"));
        assert_ne!(fingerprint("select ?"), fingerprint("select ? from t"));
        assert_eq!(fingerprint("select ?").len(), 16);
    }

    #[test]
    fn parse_elapsed_handles_units() {
        assert_eq!(parse_elapsed_dbg("1.5ms"), Some(1.5));
        assert_eq!(parse_elapsed_dbg("2s"), Some(2000.0));
        assert!((parse_elapsed_dbg("500µs").unwrap() - 0.5).abs() < 1e-9);
        assert!(parse_elapsed_dbg("garbage").is_none());
    }

    #[test]
    fn analyzer_records_and_detects_nplus1() {
        let analyzer = QueryAnalyzer {
            inner: Mutex::new(AnalyzerInner::default()),
            enabled: true,
        };
        for _ in 0..15 {
            analyzer.record("SELECT * FROM contracts WHERE id = $1", 2.0, Some(1));
        }
        let (total, _slow, distinct) = analyzer.totals();
        assert_eq!(total, 15);
        assert_eq!(distinct, 1);
        let findings = analyzer.detect_nplus1(Duration::from_secs(1), 10);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].occurrence_count >= 10);
    }

    #[test]
    fn analyzer_drain_deltas_is_incremental() {
        let analyzer = QueryAnalyzer {
            inner: Mutex::new(AnalyzerInner::default()),
            enabled: true,
        };
        analyzer.record("SELECT 1", 1.0, None);
        analyzer.record("SELECT 1", 3.0, None);
        let d1 = analyzer.drain_deltas();
        assert_eq!(d1.len(), 1);
        assert_eq!(d1[0].calls_delta, 2);
        // No new calls → no deltas.
        let d2 = analyzer.drain_deltas();
        assert!(d2.is_empty());
    }
}
