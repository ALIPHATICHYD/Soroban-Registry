-- Issue #1118: configurable retention and purge policy for terminal-state
-- history records.
--
-- The archival engine from #881 selects rows purely by age. History tables like
-- ownership_transfers hold live workflow state alongside finished records, so an
-- age-only policy would archive pending transfers. These columns let a policy
-- restrict itself to records in a state they can no longer transition out of,
-- and to measure age against a column other than created_at.

ALTER TABLE archival_policies
    ADD COLUMN IF NOT EXISTS timestamp_column TEXT NOT NULL DEFAULT 'created_at',
    ADD COLUMN IF NOT EXISTS terminal_states  TEXT[];

COMMENT ON COLUMN archival_policies.timestamp_column IS
    'Column whose value is compared against the retention cutoff.';
COMMENT ON COLUMN archival_policies.terminal_states IS
    'When set, only rows whose status column matches one of these values are eligible. NULL means age-only retention.';

-- ownership_transfers.status is constrained to
-- ('pending','confirmed','completed','expired','rejected','duplicate').
-- Only the last three are terminal: pending is awaiting confirmation, confirmed
-- is awaiting completion, and completed is the successful end state that the
-- ownership audit trail depends on.
INSERT INTO archival_policies
    (data_type, source_table, retention_days, archive_storage, timestamp_column, terminal_states)
VALUES
    (
        'ownership_transfers',
        'ownership_transfers',
        90,
        'database',
        'created_at',
        ARRAY['expired', 'rejected', 'duplicate']
    ),
    -- Scan runs are an append-only log with no status column, so retention is
    -- age-only and measured from scanned_at.
    (
        'dependency_scan_runs',
        'contract_dependency_scan_runs',
        90,
        'database',
        'scanned_at',
        NULL
    )
ON CONFLICT (data_type) DO NOTHING;

-- Two policies seeded by #881 have never archived anything. Both failed inside
-- the engine, which swallowed the error and recorded the run as completed, so
-- they reported success while doing nothing. Now that errors propagate they
-- would fail loudly on every sweep, so correct them here.
--
--   query_perf_log  -> query_performance_log timestamps its rows recorded_at,
--                      not created_at.
--   audit_logs      -> the table is audit_logs; audit_log does not exist.
UPDATE archival_policies
SET timestamp_column = 'recorded_at'
WHERE data_type = 'query_perf_log'
  AND source_table = 'query_performance_log';

UPDATE archival_policies
SET source_table = 'audit_logs'
WHERE data_type = 'audit_logs'
  AND source_table = 'audit_log';

-- Supports the retention scan on each newly covered table.
CREATE INDEX IF NOT EXISTS idx_ownership_transfers_status_created_at
    ON ownership_transfers (status, created_at);

CREATE INDEX IF NOT EXISTS idx_contract_dependency_scan_runs_scanned_at
    ON contract_dependency_scan_runs (scanned_at);
