CREATE TABLE IF NOT EXISTS sla_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    uptime_percentage DOUBLE PRECISION NOT NULL,
    avg_latency_ms DOUBLE PRECISION NOT NULL,
    error_rate DOUBLE PRECISION NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sla_metrics_contract ON sla_metrics(contract_id);
CREATE INDEX idx_sla_metrics_recorded ON sla_metrics(recorded_at);

CREATE TABLE IF NOT EXISTS sla_status (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL UNIQUE REFERENCES contracts(id) ON DELETE CASCADE,
    total_records INTEGER NOT NULL DEFAULT 0,
    violations_count INTEGER NOT NULL DEFAULT 0,
    penalty_accrued DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    credits_issued DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    compliant BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
