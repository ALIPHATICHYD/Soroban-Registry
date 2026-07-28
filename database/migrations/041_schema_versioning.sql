-- Database Migration Versioning and Rollback Support (Issue #252)
-- Tracks schema migrations with checksums, rollback scripts, and advisory locking.

-- Schema versions table: the single source of truth for applied migrations
CREATE TABLE IF NOT EXISTS schema_versions (
    id SERIAL PRIMARY KEY,
    version INTEGER NOT NULL UNIQUE,
    description TEXT NOT NULL,
    filename VARCHAR(255) NOT NULL,
    checksum VARCHAR(64) NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    applied_by VARCHAR(255) NOT NULL DEFAULT current_user,
    execution_time_ms INTEGER,
    rolled_back_at TIMESTAMPTZ,
    rollback_by VARCHAR(255)
);

CREATE INDEX idx_schema_versions_version ON schema_versions(version);
CREATE INDEX idx_schema_versions_applied_at ON schema_versions(applied_at);

-- Rollback scripts table: stores DOWN SQL for each migration
CREATE TABLE IF NOT EXISTS schema_rollback_scripts (
    id SERIAL PRIMARY KEY,
    version INTEGER NOT NULL UNIQUE REFERENCES schema_versions(version) ON DELETE CASCADE,
    down_sql TEXT NOT NULL,
    checksum VARCHAR(64) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Migration lock table: prevents concurrent migrations via advisory lock
CREATE TABLE IF NOT EXISTS schema_migration_locks (
    id INTEGER PRIMARY KEY DEFAULT 1,
    locked_by VARCHAR(255),
    locked_at TIMESTAMPTZ,
    CONSTRAINT single_lock CHECK (id = 1)
);

INSERT INTO schema_migration_locks (id) VALUES (1) ON CONFLICT DO NOTHING;
