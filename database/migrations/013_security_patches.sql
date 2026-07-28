CREATE TYPE patch_severity AS ENUM ('critical', 'high', 'medium', 'low');

CREATE TABLE security_patches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    target_version VARCHAR(50) NOT NULL,
    severity patch_severity NOT NULL,
    new_wasm_hash VARCHAR(64) NOT NULL,
    rollout_percentage INTEGER NOT NULL DEFAULT 100 CHECK (rollout_percentage BETWEEN 0 AND 100),
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_security_patches_target_version ON security_patches(target_version);
CREATE INDEX idx_security_patches_severity ON security_patches(severity);

CREATE TABLE patch_audits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    patch_id UUID NOT NULL REFERENCES security_patches(id) ON DELETE CASCADE,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, patch_id)
);

CREATE INDEX idx_patch_audits_contract_id ON patch_audits(contract_id);
CREATE INDEX idx_patch_audits_patch_id ON patch_audits(patch_id);
