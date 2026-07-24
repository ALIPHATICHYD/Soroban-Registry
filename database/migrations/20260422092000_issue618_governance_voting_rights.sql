-- Issue #618: DAO voting-rights integration for governance voting.

CREATE TABLE IF NOT EXISTS governance_voting_rights (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    publisher_id UUID NOT NULL REFERENCES publishers(id) ON DELETE CASCADE,
    voting_power BIGINT NOT NULL CHECK (voting_power >= 0),
    source VARCHAR(32) NOT NULL DEFAULT 'manual',
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (contract_id, publisher_id)
);

CREATE INDEX IF NOT EXISTS idx_governance_voting_rights_contract
    ON governance_voting_rights(contract_id);

CREATE INDEX IF NOT EXISTS idx_governance_voting_rights_publisher
    ON governance_voting_rights(publisher_id);

DROP TRIGGER IF EXISTS update_governance_voting_rights_updated_at ON governance_voting_rights;
CREATE TRIGGER update_governance_voting_rights_updated_at
    BEFORE UPDATE ON governance_voting_rights
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
