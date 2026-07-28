-- Issue #878: Database query monitoring and performance analysis
-- Creates tables for persisting query performance snapshots and slow query history.

CREATE EXTENSION IF NOT EXISTS pg_stat_statements;

CREATE TABLE IF NOT EXISTS query_performance_log (
    id              BIGSERIAL PRIMARY KEY,
    query_hash      TEXT        NOT NULL,
    query_sample    TEXT        NOT NULL,
    calls_delta     BIGINT      NOT NULL DEFAULT 0,
    mean_exec_time_ms DOUBLE PRECISION NOT NULL DEFAULT 0,
    max_exec_time_ms  DOUBLE PRECISION NOT NULL DEFAULT 0,
    total_rows      BIGINT      NOT NULL DEFAULT 0,
    is_slow         BOOLEAN     NOT NULL DEFAULT false,
    recorded_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_query_perf_log_recorded_at
    ON query_performance_log (recorded_at DESC);

CREATE INDEX IF NOT EXISTS idx_query_perf_log_hash
    ON query_performance_log (query_hash);

CREATE INDEX IF NOT EXISTS idx_query_perf_log_slow
    ON query_performance_log (is_slow, recorded_at DESC)
    WHERE is_slow = true;

-- Retention: keep rolling 90 days; older rows are archived by the archival job.
COMMENT ON TABLE query_performance_log IS
    'Periodic snapshots of pg_stat_statements for historical query performance analysis (issue #878).';
