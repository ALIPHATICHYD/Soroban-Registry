CREATE TYPE deployment_environment AS ENUM ('blue', 'green');
CREATE TYPE deployment_status AS ENUM ('active', 'inactive', 'testing', 'failed');

CREATE TABLE contract_deployments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    environment deployment_environment NOT NULL,
    status deployment_status NOT NULL DEFAULT 'inactive',
    wasm_hash VARCHAR(64) NOT NULL,
    deployed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    activated_at TIMESTAMPTZ,
    health_checks_passed INTEGER DEFAULT 0,
    health_checks_failed INTEGER DEFAULT 0,
    last_health_check_at TIMESTAMPTZ,
    error_message TEXT,
    UNIQUE(contract_id, environment)
);

CREATE INDEX idx_contract_deployments_contract_id ON contract_deployments(contract_id);
CREATE INDEX idx_contract_deployments_status ON contract_deployments(status);
CREATE INDEX idx_contract_deployments_environment ON contract_deployments(environment);
CREATE INDEX idx_contract_deployments_active ON contract_deployments(contract_id, status) WHERE status = 'active';

CREATE TABLE deployment_switches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    from_environment deployment_environment NOT NULL,
    to_environment deployment_environment NOT NULL,
    switched_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    switched_by VARCHAR(255),
    rollback BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_deployment_switches_contract_id ON deployment_switches(contract_id);
CREATE INDEX idx_deployment_switches_switched_at ON deployment_switches(switched_at DESC);
