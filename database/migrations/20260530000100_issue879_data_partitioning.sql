-- Issue #879: Data partitioning strategy for contract tables.
--
-- Creates declarative-partitioned variants alongside existing tables:
--   contracts_partitioned      — LIST partitioned by network
--   interactions_partitioned   — RANGE partitioned by month
--   audit_logs_partitioned     — RANGE partitioned by year
--
-- Existing tables are left untouched.  The partition_manager service creates
-- child partitions automatically as new periods arrive.

-- ── contracts_partitioned (LIST by network) ────────────────────────────────

CREATE TABLE IF NOT EXISTS contracts_partitioned (
    id                  UUID            NOT NULL DEFAULT gen_random_uuid(),
    contract_id         TEXT            NOT NULL,
    name                TEXT            NOT NULL,
    description         TEXT,
    category            TEXT,
    network             TEXT            NOT NULL,
    is_verified         BOOLEAN         NOT NULL DEFAULT false,
    verification_status TEXT,
    current_version     TEXT,
    slug                TEXT,
    publisher_id        UUID,
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, network)
) PARTITION BY LIST (network);

CREATE TABLE IF NOT EXISTS contracts_partitioned_mainnet
    PARTITION OF contracts_partitioned FOR VALUES IN ('mainnet');

CREATE TABLE IF NOT EXISTS contracts_partitioned_testnet
    PARTITION OF contracts_partitioned FOR VALUES IN ('testnet');

CREATE TABLE IF NOT EXISTS contracts_partitioned_futurenet
    PARTITION OF contracts_partitioned FOR VALUES IN ('futurenet');

CREATE TABLE IF NOT EXISTS contracts_partitioned_default
    PARTITION OF contracts_partitioned DEFAULT;

-- ── interactions_partitioned (RANGE by month) ─────────────────────────────

CREATE TABLE IF NOT EXISTS interactions_partitioned (
    id              BIGSERIAL,
    contract_id     UUID            NOT NULL,
    caller          TEXT,
    method          TEXT,
    args            JSONB,
    result          JSONB,
    gas_used        BIGINT,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- Seed two initial monthly partitions (current month and next month).
-- The partition_manager background task creates additional partitions automatically.
DO $$
DECLARE
    cur_start TIMESTAMPTZ := date_trunc('month', NOW());
    cur_end   TIMESTAMPTZ := date_trunc('month', NOW()) + INTERVAL '1 month';
    nxt_end   TIMESTAMPTZ := date_trunc('month', NOW()) + INTERVAL '2 months';
    tbl       TEXT;
BEGIN
    tbl := 'interactions_p_' || to_char(cur_start, 'YYYY_MM');
    IF NOT EXISTS (
        SELECT 1 FROM pg_class WHERE relname = tbl
    ) THEN
        EXECUTE format(
            'CREATE TABLE %I PARTITION OF interactions_partitioned FOR VALUES FROM (%L) TO (%L)',
            tbl, cur_start, cur_end
        );
    END IF;

    tbl := 'interactions_p_' || to_char(cur_end, 'YYYY_MM');
    IF NOT EXISTS (
        SELECT 1 FROM pg_class WHERE relname = tbl
    ) THEN
        EXECUTE format(
            'CREATE TABLE %I PARTITION OF interactions_partitioned FOR VALUES FROM (%L) TO (%L)',
            tbl, cur_end, nxt_end
        );
    END IF;
END
$$;

-- ── audit_logs_partitioned (RANGE by year) ────────────────────────────────

CREATE TABLE IF NOT EXISTS audit_logs_partitioned (
    id              BIGSERIAL,
    table_name      TEXT            NOT NULL,
    record_id       UUID,
    action          TEXT            NOT NULL,
    actor           TEXT,
    old_data        JSONB,
    new_data        JSONB,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

DO $$
DECLARE
    yr_start TIMESTAMPTZ := date_trunc('year', NOW());
    yr_end   TIMESTAMPTZ := date_trunc('year', NOW()) + INTERVAL '1 year';
    tbl      TEXT;
BEGIN
    tbl := 'audit_logs_p_' || to_char(yr_start, 'YYYY');
    IF NOT EXISTS (
        SELECT 1 FROM pg_class WHERE relname = tbl
    ) THEN
        EXECUTE format(
            'CREATE TABLE %I PARTITION OF audit_logs_partitioned FOR VALUES FROM (%L) TO (%L)',
            tbl, yr_start, yr_end
        );
    END IF;
END
$$;

-- ── Partition registry ────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS partition_registry (
    id              BIGSERIAL       PRIMARY KEY,
    parent_table    TEXT            NOT NULL,
    partition_name  TEXT            NOT NULL UNIQUE,
    partition_key   TEXT            NOT NULL,
    range_start     TIMESTAMPTZ,
    range_end       TIMESTAMPTZ,
    list_value      TEXT,
    row_count       BIGINT,
    size_bytes      BIGINT,
    status          TEXT            NOT NULL DEFAULT 'active',
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    archived_at     TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_partition_registry_parent
    ON partition_registry (parent_table, status);

COMMENT ON TABLE partition_registry IS
    'Catalogue of all managed partitions for automated lifecycle management (issue #879).';
