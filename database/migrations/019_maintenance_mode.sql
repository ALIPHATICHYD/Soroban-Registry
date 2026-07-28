-- Maintenance windows table
CREATE TABLE maintenance_windows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    scheduled_end_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    created_by UUID NOT NULL REFERENCES publishers(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_maintenance_windows_contract_id ON maintenance_windows(contract_id);
CREATE INDEX idx_maintenance_windows_ended_at ON maintenance_windows(ended_at);
CREATE INDEX idx_maintenance_windows_scheduled_end_at ON maintenance_windows(scheduled_end_at);

-- Add maintenance status to contracts
ALTER TABLE contracts ADD COLUMN is_maintenance BOOLEAN NOT NULL DEFAULT FALSE;
CREATE INDEX idx_contracts_is_maintenance ON contracts(is_maintenance);
