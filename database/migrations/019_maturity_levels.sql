-- Add maturity level to contracts
CREATE TYPE maturity_level AS ENUM ('alpha', 'beta', 'stable', 'mature', 'legacy');

ALTER TABLE contracts ADD COLUMN maturity maturity_level NOT NULL DEFAULT 'alpha';
CREATE INDEX idx_contracts_maturity ON contracts(maturity);

-- Track maturity level changes
CREATE TABLE maturity_changes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    from_level maturity_level,
    to_level maturity_level NOT NULL,
    reason TEXT,
    changed_by UUID NOT NULL REFERENCES publishers(id),
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_maturity_changes_contract_id ON maturity_changes(contract_id);
CREATE INDEX idx_maturity_changes_changed_at ON maturity_changes(changed_at);
