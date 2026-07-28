CREATE TABLE contract_call_dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    caller_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    callee_contract_id VARCHAR(56) NOT NULL,
    call_volume INT NOT NULL DEFAULT 0,
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(caller_id, callee_contract_id)
);

CREATE INDEX idx_contract_call_dependencies_caller ON contract_call_dependencies(caller_id);
CREATE INDEX idx_contract_call_dependencies_callee ON contract_call_dependencies(callee_contract_id);

CREATE TRIGGER update_contract_call_dependencies_updated_at BEFORE UPDATE ON contract_call_dependencies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
