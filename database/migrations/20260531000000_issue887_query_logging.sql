-- Issue #887: Application-side database query logging and analysis.
--
-- Complements issue #878 (which snapshots pg_stat_statements, the *server* view)
-- by persisting the *application's* own view of query execution: normalized
-- query patterns with timing/frequency deltas, and detected N+1 incidents.
--
-- The application captures every query it issues (via the sqlx tracing layer),
-- aggregates them in-process with negligible overhead, and the background task
-- flushes per-pattern deltas here on an interval so trends survive restarts and
-- can be reported on historically.
--
-- Safety: only the *normalized* query shape is stored (literals and bind
-- placeholders are collapsed to `?`), never bound parameter values, so secrets
-- are never written to this table.

-- ── Per-pattern execution deltas (time-bucketed) ──────────────────────────────
CREATE TABLE IF NOT EXISTS query_pattern_log (
    id              BIGSERIAL    PRIMARY KEY,
    -- Stable hash of the normalized statement (hex of 64-bit FNV).
    fingerprint     TEXT         NOT NULL,
    -- Truncated normalized statement, for human-readable reports.
    pattern_sample  TEXT         NOT NULL,
    -- Calls observed since the previous flush.
    calls_delta     BIGINT       NOT NULL DEFAULT 0,
    total_time_ms   DOUBLE PRECISION NOT NULL DEFAULT 0,
    mean_time_ms    DOUBLE PRECISION NOT NULL DEFAULT 0,
    max_time_ms     DOUBLE PRECISION NOT NULL DEFAULT 0,
    -- Calls in this window that exceeded the slow threshold.
    slow_calls      BIGINT       NOT NULL DEFAULT 0,
    recorded_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_query_pattern_log_recorded_at
    ON query_pattern_log (recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_query_pattern_log_fingerprint
    ON query_pattern_log (fingerprint, recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_query_pattern_log_slow
    ON query_pattern_log (recorded_at DESC)
    WHERE slow_calls > 0;

-- ── Detected N+1 query incidents ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS query_nplus1_incidents (
    id                BIGSERIAL   PRIMARY KEY,
    fingerprint       TEXT        NOT NULL,
    pattern_sample    TEXT        NOT NULL,
    -- How many times the pattern fired inside the detection window.
    occurrence_count  INTEGER     NOT NULL,
    -- Width of the burst window, in milliseconds.
    window_ms         BIGINT      NOT NULL,
    detected_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_query_nplus1_detected_at
    ON query_nplus1_incidents (detected_at DESC);
CREATE INDEX IF NOT EXISTS idx_query_nplus1_fingerprint
    ON query_nplus1_incidents (fingerprint, detected_at DESC);

COMMENT ON TABLE query_pattern_log IS
    'Application-side normalized query execution deltas for frequency/slow/trend analysis (issue #887).';
COMMENT ON TABLE query_nplus1_incidents IS
    'Detected N+1 query bursts: the same normalized pattern firing many times within a short window (issue #887).';
