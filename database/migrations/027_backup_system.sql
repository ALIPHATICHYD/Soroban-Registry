-- Contract backups
CREATE TABLE contract_backups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    backup_date DATE NOT NULL,
    wasm_hash VARCHAR(64) NOT NULL,
    metadata JSONB NOT NULL,
    state_snapshot JSONB,
    storage_size_bytes BIGINT NOT NULL,
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    primary_region VARCHAR(50) NOT NULL DEFAULT 'us-east-1',
    backup_regions TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, backup_date)
);

CREATE INDEX idx_contract_backups_contract_id ON contract_backups(contract_id);
CREATE INDEX idx_contract_backups_backup_date ON contract_backups(backup_date);
CREATE INDEX idx_contract_backups_verified ON contract_backups(verified);

-- Backup restoration log
CREATE TABLE backup_restorations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    backup_id UUID NOT NULL REFERENCES contract_backups(id),
    restored_by UUID NOT NULL REFERENCES publishers(id),
    restore_duration_ms INTEGER NOT NULL,
    success BOOLEAN NOT NULL,
    error_message TEXT,
    restored_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_backup_restorations_backup_id ON backup_restorations(backup_id);
CREATE INDEX idx_backup_restorations_restored_at ON backup_restorations(restored_at);
