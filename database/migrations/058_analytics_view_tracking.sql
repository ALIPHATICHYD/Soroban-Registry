-- Contract interaction analytics tracking (issue #415)
--
-- Adds a denormalised view_count column to contracts so profile pages can
-- display popularity without an extra aggregate query.  Views are incremented
-- asynchronously in the background (fire-and-forget tokio::spawn) so the
-- read path is never blocked waiting for a write.
--
-- The contract_interaction_daily_aggregates table introduced in migration
-- 20260223010000 is the authoritative source for deployment/interaction counts
-- and the 7-day trend; this migration extends the schema for the analytics
-- endpoint enhancements described in the issue.

-- ── View count on contracts ───────────────────────────────────────────────────

ALTER TABLE contracts
    ADD COLUMN IF NOT EXISTS view_count BIGINT NOT NULL DEFAULT 0;

-- Used for "sort by most viewed" and trending-by-views queries.
CREATE INDEX IF NOT EXISTS idx_contracts_view_count
    ON contracts(view_count DESC);

-- Composite index for category-scoped analytics summary query.
-- Covers: GROUP BY category + SUM(view_count)
CREATE INDEX IF NOT EXISTS idx_contracts_category_view_count
    ON contracts(category, view_count);

-- Composite index for network-scoped analytics summary query.
CREATE INDEX IF NOT EXISTS idx_contracts_network_view_count
    ON contracts(network, view_count);
