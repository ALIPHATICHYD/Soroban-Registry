-- Gas Usage Estimation (Issue #496)
-- Extends cost_estimates with min/max range columns and adds a method_gas_history
-- table for recording per-invocation gas observations that feed into future estimates.

-- Extend the existing cost_estimates table with range columns.
-- min/max allow the endpoint to return an accurate low-to-high estimate range.
ALTER TABLE cost_estimates ADD COLUMN IF NOT EXISTS min_gas_cost BIGINT;
ALTER TABLE cost_estimates ADD COLUMN IF NOT EXISTS max_gas_cost BIGINT;

-- Back-fill min/max from avg for existing rows so the range is non-null.
UPDATE cost_estimates
SET min_gas_cost = (avg_gas_cost * 0.8)::BIGINT,
    max_gas_cost = (avg_gas_cost * 1.2)::BIGINT
WHERE min_gas_cost IS NULL OR max_gas_cost IS NULL;

-- Per-invocation gas observations.
-- Populated externally (indexer, client SDK) and consumed by the estimation logic
-- to refine min/max/avg over time.
CREATE TABLE IF NOT EXISTS method_gas_history (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id     UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    method_name     VARCHAR(255) NOT NULL,
    gas_used        BIGINT NOT NULL,          -- actual stroops consumed
    success         BOOLEAN NOT NULL DEFAULT TRUE,
    recorded_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_method_gas_history_contract_method
    ON method_gas_history(contract_id, method_name);
CREATE INDEX IF NOT EXISTS idx_method_gas_history_recorded_at
    ON method_gas_history(recorded_at);

-- Trigger: after each new gas observation, refresh the cost_estimates aggregate row.
CREATE OR REPLACE FUNCTION refresh_cost_estimate()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO cost_estimates (contract_id, method_name, avg_gas_cost, min_gas_cost, max_gas_cost, avg_storage_bytes, sample_count, last_updated)
    SELECT
        NEW.contract_id,
        NEW.method_name,
        AVG(gas_used)::BIGINT,
        MIN(gas_used),
        MAX(gas_used),
        0,
        COUNT(*),
        NOW()
    FROM method_gas_history
    WHERE contract_id = NEW.contract_id
      AND method_name = NEW.method_name
    ON CONFLICT (contract_id, method_name) DO UPDATE
        SET avg_gas_cost  = EXCLUDED.avg_gas_cost,
            min_gas_cost  = EXCLUDED.min_gas_cost,
            max_gas_cost  = EXCLUDED.max_gas_cost,
            sample_count  = EXCLUDED.sample_count,
            last_updated  = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_refresh_cost_estimate ON method_gas_history;
CREATE TRIGGER trg_refresh_cost_estimate
    AFTER INSERT ON method_gas_history
    FOR EACH ROW EXECUTE FUNCTION refresh_cost_estimate();
