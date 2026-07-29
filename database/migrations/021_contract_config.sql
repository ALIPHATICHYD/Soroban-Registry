-- Migration to add contract_configs table

CREATE TABLE contract_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    environment VARCHAR(50) NOT NULL,
    version INT NOT NULL DEFAULT 1,
    config_data JSONB NOT NULL DEFAULT '{}',
    secrets_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255) NOT NULL,
    UNIQUE(contract_id, environment, version)
);

CREATE INDEX idx_contract_configs_contract_id ON contract_configs(contract_id);
CREATE INDEX idx_contract_configs_environment ON contract_configs(environment);
