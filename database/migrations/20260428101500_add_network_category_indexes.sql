-- Migration to add indexes on network and category columns for optimized filtering.
-- Target tables: contracts, contract_metadata_versions

-- 1. Ensure indexes on contracts table
CREATE INDEX IF NOT EXISTS idx_contracts_network ON contracts(network);
CREATE INDEX IF NOT EXISTS idx_contracts_category ON contracts(category);

-- 2. Add composite index for combined filtering on contracts
CREATE INDEX IF NOT EXISTS idx_contracts_network_category ON contracts(network, category);

-- 3. Add index on category for metadata history lookups
CREATE INDEX IF NOT EXISTS idx_contract_metadata_versions_category ON contract_metadata_versions(category);

-- 4. Add index on network for daily interaction aggregates
CREATE INDEX IF NOT EXISTS idx_interaction_daily_network ON contract_interaction_daily_aggregates(network);
