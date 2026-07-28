-- Issue #881: Data archival and cleanup strategy for old records.
--
-- Creates the tables that track archival policies, run history, and
-- the audit trail of every archived record.

-- ── Retention policies (configurable per data type) ───────────────────────

CREATE TABLE IF NOT EXISTS archival_policies (
    id              BIGSERIAL       PRIMARY KEY,
    data_type       TEXT            NOT NULL UNIQUE,
    source_table    TEXT            NOT NULL,
    retention_days  INT             NOT NULL DEFAULT 365,
    archive_storage TEXT            NOT NULL DEFAULT 'database',
    is_enabled      BOOLEAN         NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

-- Default policies
INSERT INTO archival_policies (data_type, source_table, retention_days, archive_storage)
VALUES
    ('interactions',    'contract_interactions', 365,  'database'),
    ('audit_logs',      'audit_log',             1825, 'database'),
    ('query_perf_log',  'query_performance_log',  90,  'database')
ON CONFLICT (data_type) DO NOTHING;

-- ── Archival run log ──────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS archival_runs (
    id              BIGSERIAL       PRIMARY KEY,
    policy_id       BIGINT          REFERENCES archival_policies(id),
    data_type       TEXT            NOT NULL,
    status          TEXT            NOT NULL DEFAULT 'pending',
    rows_archived   BIGINT          NOT NULL DEFAULT 0,
    rows_deleted    BIGINT          NOT NULL DEFAULT 0,
    error_message   TEXT,
    started_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    completed_at    TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_archival_runs_policy
    ON archival_runs (policy_id, started_at DESC);

CREATE INDEX IF NOT EXISTS idx_archival_runs_status
    ON archival_runs (status, started_at DESC);

-- ── Archived record trail (audit trail of what was archived) ──────────────

CREATE TABLE IF NOT EXISTS archival_audit_trail (
    id              BIGSERIAL       PRIMARY KEY,
    run_id          BIGINT          REFERENCES archival_runs(id),
    source_table    TEXT            NOT NULL,
    source_id       TEXT            NOT NULL,
    archived_data   JSONB,
    archive_ref     TEXT,
    archived_at     TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_archival_trail_run
    ON archival_audit_trail (run_id);

CREATE INDEX IF NOT EXISTS idx_archival_trail_source
    ON archival_audit_trail (source_table, source_id);

COMMENT ON TABLE archival_policies     IS 'Configurable retention policies per data type (issue #881).';
COMMENT ON TABLE archival_runs         IS 'Execution history of archival jobs (issue #881).';
COMMENT ON TABLE archival_audit_trail  IS 'Per-record archive audit trail enabling restore (issue #881).';
