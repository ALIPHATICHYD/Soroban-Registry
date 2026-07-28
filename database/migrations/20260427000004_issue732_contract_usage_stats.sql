-- Migration: Contract Usage Statistics and Metrics (Issue #732)
-- Purpose: Add dedicated stats tracking table and optimize queries for
--          GET /contracts/{id}/stats with time-series support.

-- 1) Contract usage stats snapshot table
--    Stores periodic snapshots of cumulative stats per contract.
--    Updated by the hourly aggregation job.
CREATE TABLE IF NOT EXISTS contract_usage_stats (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    deployment_count BIGINT NOT NULL DEFAULT 0,
    call_count BIGINT NOT NULL DEFAULT 0,
    error_count BIGINT NOT NULL DEFAULT 0,
    unique_callers BIGINT NOT NULL DEFAULT 0,
    unique_deployers BIGINT NOT NULL DEFAULT 0,
    total_interactions BIGINT NOT NULL DEFAULT 0,
    avg_calls_per_day NUMERIC(12, 2) NOT NULL DEFAULT 0,
    error_rate NUMERIC(5, 4) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, period_start, period_end)
);

CREATE INDEX IF NOT EXISTS idx_contract_usage_stats_contract_period
    ON contract_usage_stats(contract_id, period_start DESC);

CREATE INDEX IF NOT EXISTS idx_contract_usage_stats_period
    ON contract_usage_stats(period_start DESC, period_end DESC);

-- Trigger to auto-update updated_at
CREATE TRIGGER update_contract_usage_stats_updated_at
    BEFORE UPDATE ON contract_usage_stats
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 2) Trending contracts materialized view
--    Refreshed hourly alongside the aggregation job.
--    Ranks contracts by interaction velocity over configurable windows.
CREATE MATERIALIZED VIEW IF NOT EXISTS trending_contracts_mv AS
SELECT
    c.id AS contract_id,
    c.name,
    c.network::TEXT,
    c.category,
    c.is_verified,
    COALESCE(stats_7d.total_interactions, 0) AS interactions_7d,
    COALESCE(stats_30d.total_interactions, 0) AS interactions_30d,
    COALESCE(stats_90d.total_interactions, 0) AS interactions_90d,
    COALESCE(stats_7d.deployment_count, 0) AS deployments_7d,
    COALESCE(stats_7d.error_count, 0) AS errors_7d,
    COALESCE(stats_7d.unique_callers, 0) AS unique_callers_7d,
    -- Trending score: weighted combination of recent activity
    (
        COALESCE(stats_7d.total_interactions, 0) * 1.0 +
        COALESCE(stats_30d.total_interactions, 0) * 0.3 +
        COALESCE(stats_90d.total_interactions, 0) * 0.1
    ) AS trending_score,
    NOW() AS calculated_at
FROM contracts c
LEFT JOIN LATERAL (
    SELECT
        SUM(count) AS total_interactions,
        SUM(count) FILTER (WHERE agg.interaction_type = 'deploy') AS deployment_count,
        SUM(count) FILTER (WHERE agg.interaction_type = 'publish_failed') AS error_count,
        COUNT(DISTINCT ci.user_address) FILTER (WHERE ci.user_address IS NOT NULL) AS unique_callers
    FROM contract_interaction_daily_aggregates agg
    LEFT JOIN contract_interactions ci ON ci.contract_id = agg.contract_id
        AND DATE(ci.interaction_timestamp) >= CURRENT_DATE - INTERVAL '7 days'
        AND DATE(ci.interaction_timestamp) <= CURRENT_DATE
    WHERE agg.contract_id = c.id
      AND agg.day >= CURRENT_DATE - INTERVAL '7 days'
      AND agg.day <= CURRENT_DATE
) stats_7d ON true
LEFT JOIN LATERAL (
    SELECT SUM(count) AS total_interactions
    FROM contract_interaction_daily_aggregates
    WHERE contract_id = c.id
      AND day >= CURRENT_DATE - INTERVAL '30 days'
      AND day <= CURRENT_DATE
) stats_30d ON true
LEFT JOIN LATERAL (
    SELECT SUM(count) AS total_interactions
    FROM contract_interaction_daily_aggregates
    WHERE contract_id = c.id
      AND day >= CURRENT_DATE - INTERVAL '90 days'
      AND day <= CURRENT_DATE
) stats_90d ON true
ORDER BY trending_score DESC;

CREATE UNIQUE INDEX IF NOT EXISTS idx_trending_contracts_mv_contract_id
    ON trending_contracts_mv(contract_id);

CREATE INDEX IF NOT EXISTS idx_trending_contracts_mv_score
    ON trending_contracts_mv(trending_score DESC);

-- 3) Function to refresh trending contracts materialized view
CREATE OR REPLACE FUNCTION refresh_trending_contracts()
RETURNS VOID AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY trending_contracts_mv;
END;
$$ LANGUAGE plpgsql;

-- 4) Function to upsert contract usage stats for a given period
CREATE OR REPLACE FUNCTION upsert_contract_usage_stats(
    p_contract_id UUID,
    p_period_start TIMESTAMPTZ,
    p_period_end TIMESTAMPTZ
)
RETURNS VOID AS $$
DECLARE
    v_deployment_count BIGINT;
    v_call_count BIGINT;
    v_error_count BIGINT;
    v_unique_callers BIGINT;
    v_unique_deployers BIGINT;
    v_total_interactions BIGINT;
    v_days NUMERIC;
    v_avg_calls_per_day NUMERIC(12, 2);
    v_error_rate NUMERIC(5, 4);
BEGIN
    -- Get stats from daily aggregates for the period
    SELECT
        COALESCE(SUM(count) FILTER (WHERE agg.interaction_type = 'deploy'), 0),
        COALESCE(SUM(count) FILTER (WHERE agg.interaction_type IN ('invoke', 'transfer', 'query')), 0),
        COALESCE(SUM(count) FILTER (WHERE agg.interaction_type = 'publish_failed'), 0),
        COUNT(DISTINCT ci.user_address) FILTER (WHERE ci.user_address IS NOT NULL AND ci.interaction_type IN ('invoke', 'transfer', 'query')),
        COUNT(DISTINCT ci.user_address) FILTER (WHERE ci.user_address IS NOT NULL AND ci.interaction_type = 'deploy'),
        COALESCE(SUM(count), 0)
    INTO
        v_deployment_count,
        v_call_count,
        v_error_count,
        v_unique_callers,
        v_unique_deployers,
        v_total_interactions
    FROM contract_interaction_daily_aggregates agg
    LEFT JOIN contract_interactions ci ON ci.contract_id = agg.contract_id
        AND DATE(ci.interaction_timestamp) BETWEEN DATE(p_period_start) AND DATE(p_period_end)
    WHERE agg.contract_id = p_contract_id
      AND agg.day >= DATE(p_period_start)
      AND agg.day <= DATE(p_period_end);

    -- Calculate derived metrics
    v_days = GREATEST(EXTRACT(EPOCH FROM (p_period_end - p_period_start)) / 86400.0, 1.0);
    v_avg_calls_per_day = CASE WHEN v_days > 0 THEN v_call_count::NUMERIC / v_days ELSE 0 END;
    v_error_rate = CASE WHEN v_total_interactions > 0 THEN v_error_count::NUMERIC / v_total_interactions ELSE 0 END;

    -- Upsert the stats
    INSERT INTO contract_usage_stats (
        contract_id, period_start, period_end,
        deployment_count, call_count, error_count,
        unique_callers, unique_deployers, total_interactions,
        avg_calls_per_day, error_rate
    ) VALUES (
        p_contract_id, p_period_start, p_period_end,
        v_deployment_count, v_call_count, v_error_count,
        COALESCE(v_unique_callers, 0), COALESCE(v_unique_deployers, 0), v_total_interactions,
        v_avg_calls_per_day, v_error_rate
    )
    ON CONFLICT (contract_id, period_start, period_end) DO UPDATE SET
        deployment_count = EXCLUDED.deployment_count,
        call_count = EXCLUDED.call_count,
        error_count = EXCLUDED.error_count,
        unique_callers = EXCLUDED.unique_callers,
        unique_deployers = EXCLUDED.unique_deployers,
        total_interactions = EXCLUDED.total_interactions,
        avg_calls_per_day = EXCLUDED.avg_calls_per_day,
        error_rate = EXCLUDED.error_rate,
        updated_at = NOW();
END;
$$ LANGUAGE plpgsql;

-- 5) Seed initial stats for all contracts (last 90 days)
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN SELECT id FROM contracts LOOP
        PERFORM upsert_contract_usage_stats(r.id, NOW() - INTERVAL '90 days', NOW());
        PERFORM upsert_contract_usage_stats(r.id, NOW() - INTERVAL '30 days', NOW());
        PERFORM upsert_contract_usage_stats(r.id, NOW() - INTERVAL '7 days', NOW());
    END LOOP;
END $$;

COMMENT ON TABLE contract_usage_stats IS 'Periodic snapshots of contract usage metrics (deployments, calls, errors, unique callers).';
COMMENT ON MATERIALIZED VIEW trending_contracts_mv IS 'Materialized view ranking contracts by interaction velocity for trending display.';
