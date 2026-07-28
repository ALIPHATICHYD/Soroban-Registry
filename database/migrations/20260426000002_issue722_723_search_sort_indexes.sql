-- Issue #722: Add database indexes on network and category columns for fast filtering.
-- Issue #723: Add indexes to support efficient sorting by updated_at, verified_at,
--             and interaction/deployment counts.

-- Single-column indexes for WHERE filters
CREATE INDEX IF NOT EXISTS idx_contracts_network
    ON contracts (network);

CREATE INDEX IF NOT EXISTS idx_contracts_category
    ON contracts (category);

-- Composite index for the common (network, category) combination
CREATE INDEX IF NOT EXISTS idx_contracts_network_category
    ON contracts (network, category);

-- Sorting indexes
CREATE INDEX IF NOT EXISTS idx_contracts_updated_at_desc
    ON contracts (updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_contracts_verified_at_desc
    ON contracts (verified_at DESC);

CREATE INDEX IF NOT EXISTS idx_contracts_last_accessed_at_desc
    ON contracts (last_accessed_at DESC);

-- Verification status filter index (used by the verification-status endpoint #724)
CREATE INDEX IF NOT EXISTS idx_verifications_contract_status
    ON verifications (contract_id, status);

-- Verification history (used by the verification-history endpoint #724)
CREATE INDEX IF NOT EXISTS idx_verification_events_contract_created
    ON verification_events (contract_id, created_at DESC);

-- Analytics interaction daily aggregates: fast per-contract time-range lookups (#725)
CREATE INDEX IF NOT EXISTS idx_interaction_daily_contract_day
    ON contract_interaction_daily_aggregates (contract_id, day DESC);

CREATE INDEX IF NOT EXISTS idx_interaction_daily_contract_type_day
    ON contract_interaction_daily_aggregates (contract_id, interaction_type, day DESC);
