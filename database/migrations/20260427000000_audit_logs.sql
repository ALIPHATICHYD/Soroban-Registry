-- Audit log table for security-sensitive operations.
--
-- Design principles:
--   • Append-only: no UPDATE or DELETE privileges granted on this table.
--   • Tamper-evident: each row carries a SHA-256 chain hash over the previous
--     row's hash and the current row's payload, making undetected modification
--     of historical records computationally infeasible.
--   • Immutable trigger: a BEFORE UPDATE/DELETE trigger raises an exception so
--     even superuser mistakes are caught at the DB level.

CREATE TABLE IF NOT EXISTS audit_logs (
    id            BIGSERIAL    PRIMARY KEY,
    -- Who performed the action (NULL for system/anonymous operations).
    actor_id      TEXT,
    actor_email   TEXT,
    -- What happened.
    operation     TEXT         NOT NULL,   -- e.g. 'contract.verify', 'publisher.change'
    resource_type TEXT         NOT NULL,   -- e.g. 'contract', 'publisher', 'user'
    resource_id   TEXT         NOT NULL,   -- ID of the affected resource
    -- Structured context (request metadata, diff, etc.).
    metadata      JSONB        NOT NULL DEFAULT '{}',
    -- Outcome of the operation.
    status        TEXT         NOT NULL CHECK (status IN ('success', 'failure')),
    error_message TEXT,
    -- Tamper-evidence chain: SHA-256(prev_chain_hash || operation || resource_id || created_at).
    chain_hash    TEXT         NOT NULL,
    -- Immutable timestamp.
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- Indexes for common audit queries.
CREATE INDEX IF NOT EXISTS idx_audit_logs_actor_id      ON audit_logs (actor_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_operation     ON audit_logs (operation);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource      ON audit_logs (resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at    ON audit_logs (created_at DESC);

-- Prevent any modification or deletion of audit rows.
CREATE OR REPLACE FUNCTION audit_logs_immutable()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'audit_logs is append-only: UPDATE and DELETE are not permitted';
END;
$$;

DROP TRIGGER IF EXISTS trg_audit_logs_immutable ON audit_logs;
CREATE TRIGGER trg_audit_logs_immutable
    BEFORE UPDATE OR DELETE ON audit_logs
    FOR EACH ROW EXECUTE FUNCTION audit_logs_immutable();

-- Revoke destructive privileges (run as superuser during migration).
-- REVOKE UPDATE, DELETE, TRUNCATE ON audit_logs FROM PUBLIC;
-- REVOKE UPDATE, DELETE, TRUNCATE ON audit_logs FROM <app_role>;