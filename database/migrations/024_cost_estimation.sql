-- Cost estimation data
CREATE TABLE cost_estimates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    method_name VARCHAR(255) NOT NULL,
    avg_gas_cost BIGINT NOT NULL,
    avg_storage_bytes BIGINT NOT NULL,
    sample_count INTEGER NOT NULL DEFAULT 1,
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, method_name)
);

CREATE INDEX idx_cost_estimates_contract_id ON cost_estimates(contract_id);
CREATE INDEX idx_cost_estimates_method_name ON cost_estimates(method_name);
