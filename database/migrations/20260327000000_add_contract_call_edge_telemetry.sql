-- Exact per-edge call telemetry for dependency graph metrics.
CREATE TABLE IF NOT EXISTS contract_call_edge_daily_aggregates (
    source_contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    target_contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    network network_type NOT NULL,
    day DATE NOT NULL,
    call_count BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (source_contract_id, target_contract_id, network, day),
    CONSTRAINT contract_call_edge_daily_aggregates_no_self_call
        CHECK (source_contract_id <> target_contract_id)
);

CREATE INDEX IF NOT EXISTS idx_contract_call_edge_daily_source_day
    ON contract_call_edge_daily_aggregates(source_contract_id, day DESC);

CREATE INDEX IF NOT EXISTS idx_contract_call_edge_daily_target_day
    ON contract_call_edge_daily_aggregates(target_contract_id, day DESC);