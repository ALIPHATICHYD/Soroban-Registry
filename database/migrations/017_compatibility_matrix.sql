-- Add a table for contract version compatibility matrix
CREATE TABLE contract_version_compatibility (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    source_version VARCHAR(50) NOT NULL,
    target_contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    target_version VARCHAR(50) NOT NULL,
    stellar_version VARCHAR(50),
    is_compatible BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(source_contract_id, source_version, target_contract_id, target_version)
);

CREATE INDEX idx_compat_source_contract ON contract_version_compatibility(source_contract_id);
CREATE INDEX idx_compat_target_contract ON contract_version_compatibility(target_contract_id);

CREATE TRIGGER update_compatibility_updated_at BEFORE UPDATE ON contract_version_compatibility
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
