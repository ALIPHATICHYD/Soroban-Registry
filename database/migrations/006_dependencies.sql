-- Create contract dependencies table
CREATE TABLE contract_dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    dependency_name VARCHAR(255) NOT NULL,
    dependency_contract_id UUID REFERENCES contracts(id),
    version_constraint VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, dependency_name)
);

-- Indexes for efficient lookups
CREATE INDEX idx_contract_dependencies_contract_id ON contract_dependencies(contract_id);
CREATE INDEX idx_contract_dependencies_dependency_contract_id ON contract_dependencies(dependency_contract_id);
