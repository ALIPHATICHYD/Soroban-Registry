-- Create contract_health table
CREATE TABLE IF NOT EXISTS contract_health (
    contract_id UUID PRIMARY KEY REFERENCES contracts(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('healthy', 'warning', 'critical')),
    last_activity TIMESTAMP WITH TIME ZONE NOT NULL,
    security_score INTEGER NOT NULL DEFAULT 0,
    audit_date TIMESTAMP WITH TIME ZONE,
    total_score INTEGER NOT NULL DEFAULT 0 CHECK (total_score >= 0 AND total_score <= 100),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create index for faster querying by status and score
CREATE INDEX idx_contract_health_status ON contract_health(status);
CREATE INDEX idx_contract_health_total_score ON contract_health(total_score);
