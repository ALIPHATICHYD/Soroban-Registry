-- Migration Audit Log (Issue #877)
-- Records every migration framework operation for a complete audit trail.

CREATE TABLE IF NOT EXISTS migration_audit_log (
    id          BIGSERIAL PRIMARY KEY,
    operation   VARCHAR(64)  NOT NULL,  -- 'apply', 'rollback', 'register', 'dry_run', 'validate', 'lock_acquired', 'lock_released'
    version     INTEGER,
    actor       VARCHAR(255) NOT NULL DEFAULT current_user,
    success     BOOLEAN      NOT NULL,
    detail      TEXT,
    error_msg   TEXT,
    duration_ms INTEGER,
    occurred_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_migration_audit_occurred_at ON migration_audit_log(occurred_at DESC);
CREATE INDEX idx_migration_audit_version     ON migration_audit_log(version)     WHERE version IS NOT NULL;
CREATE INDEX idx_migration_audit_operation   ON migration_audit_log(operation);
