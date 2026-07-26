-- The audit_logs table is already created by earlier migrations
-- (025_data_residency.sql and 20260427000000_audit_logs.sql, both using
-- CREATE TABLE IF NOT EXISTS with an equivalent schema). The original form of
-- this migration used a bare CREATE TABLE / CREATE INDEX and therefore failed on
-- any database that already had the table:
--   relation "audit_logs" already exists
-- Make every statement idempotent so this migration is a safe no-op where the
-- table exists and still creates it where it does not.
CREATE TABLE IF NOT EXISTS audit_logs (
    id BIGSERIAL PRIMARY KEY,
    actor_id VARCHAR(255),
    actor_email VARCHAR(255),
    operation VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    resource_id VARCHAR(255) NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    status VARCHAR(50) NOT NULL,
    error_message TEXT,
    chain_hash VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_actor ON audit_logs(actor_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at);
