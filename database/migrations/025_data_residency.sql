-- Issue #100: Contract Data Residency Controls
-- Enforces where contract data may be stored for regulatory compliance.

CREATE TYPE residency_decision AS ENUM ('allowed', 'denied');

CREATE TABLE residency_policies (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id     VARCHAR(56)  NOT NULL,
    allowed_regions TEXT[]       NOT NULL CHECK (array_length(allowed_regions, 1) >= 1),
    description     TEXT,
    is_active       BOOLEAN      NOT NULL DEFAULT TRUE,
    created_by      VARCHAR(56)  NOT NULL,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_residency_policies_contract_id ON residency_policies(contract_id);
CREATE INDEX idx_residency_policies_is_active   ON residency_policies(is_active);

CREATE TRIGGER update_residency_policies_updated_at
    BEFORE UPDATE ON residency_policies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE residency_audit_logs (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id        UUID         NOT NULL REFERENCES residency_policies(id) ON DELETE CASCADE,
    contract_id      VARCHAR(56)  NOT NULL,
    requested_region VARCHAR(64)  NOT NULL,
    decision         residency_decision NOT NULL,
    action           VARCHAR(64)  NOT NULL,
    requested_by     VARCHAR(56),
    reason           TEXT,
    created_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_residency_audit_logs_policy_id    ON residency_audit_logs(policy_id);
CREATE INDEX idx_residency_audit_logs_contract_id  ON residency_audit_logs(contract_id);
CREATE INDEX idx_residency_audit_logs_decision     ON residency_audit_logs(decision);
CREATE INDEX idx_residency_audit_logs_created_at   ON residency_audit_logs(created_at);

CREATE TABLE residency_violations (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id        UUID         NOT NULL REFERENCES residency_policies(id) ON DELETE CASCADE,
    contract_id      VARCHAR(56)  NOT NULL,
    attempted_region VARCHAR(64)  NOT NULL,
    action           VARCHAR(64)  NOT NULL,
    attempted_by     VARCHAR(56),
    prevented_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_residency_violations_policy_id    ON residency_violations(policy_id);
CREATE INDEX idx_residency_violations_contract_id  ON residency_violations(contract_id);
CREATE INDEX idx_residency_violations_prevented_at ON residency_violations(prevented_at);
