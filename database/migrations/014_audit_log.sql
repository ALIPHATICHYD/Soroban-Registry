-- 004_audit_log.sql
-- Comprehensive audit log tracking all contract modifications with full
-- version history.  Records are NEVER deleted (no cascade from contracts).

-- ─────────────────────────────────────────────────────────
-- Action type enum
-- ─────────────────────────────────────────────────────────
DO $$ BEGIN
    CREATE TYPE audit_action_type AS ENUM (
        'contract_published',
        'metadata_updated',
        'verification_changed',
        'publisher_changed',
        'version_created',
        'rollback'
    );
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

-- ─────────────────────────────────────────────────────────
-- contract_audit_log
-- One row per mutation.  Immutable — never deleted.
-- ─────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS contract_audit_log (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL,
    action_type audit_action_type NOT NULL,
    old_value   JSONB,                          -- NULL for initial publish
    new_value   JSONB,                          -- NULL for hard-delete (future)
    changed_by  VARCHAR(56) NOT NULL,           -- Stellar address or service ID
    timestamp   TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Intentional: NO FK to contracts so the log survives contract deletions
    CONSTRAINT chk_audit_log_has_value
        CHECK (old_value IS NOT NULL OR new_value IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_audit_log_contract_id
    ON contract_audit_log(contract_id);

CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp
    ON contract_audit_log(timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_audit_log_action_type
    ON contract_audit_log(action_type);

-- Composite index for the sidebar query (latest 10 per contract)
CREATE INDEX IF NOT EXISTS idx_audit_log_contract_ts
    ON contract_audit_log(contract_id, timestamp DESC);

-- ─────────────────────────────────────────────────────────
-- contract_snapshots
-- Full serialised contract state at each audited point.
-- ─────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS contract_snapshots (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id     UUID NOT NULL,
    version_number  INTEGER NOT NULL,           -- monotonically increasing per contract
    snapshot_data   JSONB NOT NULL,             -- full contracts row as JSON
    audit_log_id    UUID NOT NULL REFERENCES contract_audit_log(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Each contract has at most one snapshot per version number
    UNIQUE(contract_id, version_number)
);

CREATE INDEX IF NOT EXISTS idx_snapshots_contract_id
    ON contract_snapshots(contract_id);

CREATE INDEX IF NOT EXISTS idx_snapshots_contract_version
    ON contract_snapshots(contract_id, version_number DESC);

-- ─────────────────────────────────────────────────────────
-- Helper function: next version number for a given contract
-- ─────────────────────────────────────────────────────────
CREATE OR REPLACE FUNCTION next_contract_version(p_contract_id UUID)
RETURNS INTEGER AS $$
DECLARE
    v_max INTEGER;
BEGIN
    SELECT COALESCE(MAX(version_number), 0)
      INTO v_max
      FROM contract_snapshots
     WHERE contract_id = p_contract_id;
    RETURN v_max + 1;
END;
$$ LANGUAGE plpgsql;
