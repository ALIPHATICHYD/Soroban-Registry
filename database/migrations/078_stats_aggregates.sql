-- Migration: 078_stats_aggregates.sql
-- Issue #526: Contract Statistics CLI Command
-- Purpose: Pre-computed aggregates for fast statistics queries

-- The contract<->tag junction table is referenced throughout the backend
-- (INSERT INTO contract_tags ..., and the mv_tag_stats view below) but no
-- earlier migration ever created it, so `mv_tag_stats` fails on a fresh DB with:
-- relation "contract_tags" does not exist. Create the standard many-to-many
-- junction here (idempotent) so the view builds and tag features work.
CREATE TABLE IF NOT EXISTS contract_tags (
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (contract_id, tag_id)
);

CREATE INDEX IF NOT EXISTS idx_contract_tags_contract_id ON contract_tags(contract_id);
CREATE INDEX IF NOT EXISTS idx_contract_tags_tag_id ON contract_tags(tag_id);

-- Create materialized view for contract statistics (refresh periodically)
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_contract_stats AS
SELECT 
    COUNT(*) as total_contracts,
    COUNT(DISTINCT publisher_id) as total_publishers,
    COUNT(*) FILTER (WHERE is_verified = TRUE) as verified_contracts,
    ROUND(
        (COUNT(*) FILTER (WHERE is_verified = TRUE)::numeric / NULLIF(COUNT(*), 0) * 100), 
        2
    ) as verification_percentage,
    COUNT(DISTINCT category) as total_categories,
    COUNT(DISTINCT network) as total_networks,
    MIN(created_at) as first_contract_date,
    MAX(created_at) as latest_contract_date,
    NOW() as stats_refreshed_at
FROM contracts;

-- Create index on materialized view for potential refresh queries
CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_contract_stats_dummy ON mv_contract_stats((1));

-- Network-specific statistics
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_network_stats AS
SELECT 
    network,
    COUNT(*) as contract_count,
    COUNT(*) FILTER (WHERE is_verified = TRUE) as verified_count,
    ROUND(
        (COUNT(*) FILTER (WHERE is_verified = TRUE)::numeric / NULLIF(COUNT(*), 0) * 100), 
        2
    ) as verification_rate,
    COUNT(DISTINCT publisher_id) as publisher_count,
    COUNT(DISTINCT category) as category_count,
    MIN(created_at) as first_deployed,
    MAX(created_at) as last_deployed
FROM contracts
GROUP BY network;

CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_network_stats_network ON mv_network_stats(network);

-- Category statistics
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_category_stats AS
SELECT 
    COALESCE(category, 'Uncategorized') as category,
    COUNT(*) as contract_count,
    COUNT(*) FILTER (WHERE is_verified = TRUE) as verified_count,
    ROUND(
        (COUNT(*) FILTER (WHERE is_verified = TRUE)::numeric / NULLIF(COUNT(*), 0) * 100), 
        2
    ) as verification_rate,
    COUNT(DISTINCT publisher_id) as publisher_count,
    COUNT(DISTINCT network) as network_coverage,
    RANK() OVER (ORDER BY COUNT(*) DESC) as popularity_rank
FROM contracts
GROUP BY category;

CREATE INDEX IF NOT EXISTS idx_mv_category_stats_count ON mv_category_stats(contract_count DESC);

-- Top contracts by interactions (for trending/leaderboard)
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_top_contracts AS
SELECT 
    c.id,
    c.name,
    c.contract_id as slug,
    c.contract_id,
    c.network,
    c.is_verified,
    COALESCE(ci.interaction_count, 0) as total_interactions,
    COALESCE(ci.unique_users, 0) as unique_users,
    c.created_at,
    NULL::timestamptz as verified_at
FROM contracts c
LEFT JOIN (
    SELECT 
        contract_id,
        COUNT(*) as interaction_count,
        COUNT(DISTINCT user_address) as unique_users
    FROM contract_interactions 
    WHERE created_at > NOW() - INTERVAL '30 days'
    GROUP BY contract_id
) ci ON c.id = ci.contract_id
ORDER BY ci.interaction_count DESC NULLS LAST
LIMIT 100;

CREATE INDEX IF NOT EXISTS idx_mv_top_contracts_interactions ON mv_top_contracts(total_interactions DESC);

-- Monthly growth trends
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_monthly_growth AS
SELECT 
    DATE_TRUNC('month', created_at) as month,
    COUNT(*) as contracts_created,
    COUNT(*) FILTER (WHERE is_verified = TRUE) as contracts_verified,
    COUNT(DISTINCT publisher_id) as new_publishers,
    LAG(COUNT(*)) OVER (ORDER BY DATE_TRUNC('month', created_at)) as prev_month_count,
    ROUND(
        ((COUNT(*) - LAG(COUNT(*)) OVER (ORDER BY DATE_TRUNC('month', created_at)))::numeric 
         / LAG(COUNT(*)) OVER (ORDER BY DATE_TRUNC('month', created_at)) * 100), 
        2
    ) as growth_percentage
FROM contracts
GROUP BY DATE_TRUNC('month', created_at)
ORDER BY month DESC;

CREATE INDEX IF NOT EXISTS idx_mv_monthly_growth_month ON mv_monthly_growth(month);

-- Tag popularity statistics
CREATE MATERIALIZED VIEW IF NOT EXISTS mv_tag_stats AS
SELECT 
    t.name as tag_name,
    COUNT(DISTINCT ct.contract_id) as contract_count,
    COUNT(DISTINCT ct.contract_id) FILTER (WHERE c.is_verified = TRUE) as verified_contract_count,
    ROUND(
        (COUNT(DISTINCT ct.contract_id) FILTER (WHERE c.is_verified = TRUE)::numeric 
         / NULLIF(COUNT(DISTINCT ct.contract_id), 0) * 100), 
        2
    ) as verification_rate
FROM tags t
JOIN contract_tags ct ON t.id = ct.tag_id
JOIN contracts c ON c.id = ct.contract_id
GROUP BY t.name
ORDER BY contract_count DESC;

CREATE INDEX IF NOT EXISTS idx_mv_tag_stats_count ON mv_tag_stats(contract_count DESC);

-- Function to refresh all stats materialized views
CREATE OR REPLACE FUNCTION refresh_stats_views()
RETURNS VOID AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_contract_stats;
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_network_stats;
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_category_stats;
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_top_contracts;
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_monthly_growth;
    REFRESH MATERIALIZED VIEW CONCURRENTLY mv_tag_stats;
END;
$$ LANGUAGE plpgsql;

-- Grant permissions.
-- `registry_user` is the runtime role in the Docker/production deployment; it is
-- not created by any migration, so an unguarded GRANT fails on databases that do
-- not provision it (e.g. local dev). Grant only when the role exists. (Migration
-- 077 side-steps the same issue by commenting its grants out entirely.)
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'registry_user') THEN
    GRANT SELECT ON mv_contract_stats TO registry_user;
    GRANT SELECT ON mv_network_stats TO registry_user;
    GRANT SELECT ON mv_category_stats TO registry_user;
    GRANT SELECT ON mv_top_contracts TO registry_user;
    GRANT SELECT ON mv_monthly_growth TO registry_user;
    GRANT SELECT ON mv_tag_stats TO registry_user;
  END IF;
END $$;
