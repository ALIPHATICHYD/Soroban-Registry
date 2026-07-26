-- Migration for #949 Composite Index Optimization
--
-- CREATE INDEX CONCURRENTLY cannot run inside a transaction block, and sqlx wraps
-- each migration in one, so the original CONCURRENTLY form failed with:
--   CREATE INDEX CONCURRENTLY cannot run inside a transaction block
-- CONCURRENTLY only exists to avoid locking a table under live production
-- traffic; during a migration a plain CREATE INDEX is correct (and instant on an
-- empty/small table). Dropped CONCURRENTLY so the migration applies.

-- (network, category)
CREATE INDEX IF NOT EXISTS idx_contracts_network_category ON contracts (network, category);

-- (network, verified)
CREATE INDEX IF NOT EXISTS idx_contracts_network_verified ON contracts (network, is_verified);

-- (network, created_at DESC)
CREATE INDEX IF NOT EXISTS idx_contracts_network_created_at ON contracts (network, created_at DESC);

-- (network, updated_at DESC)
CREATE INDEX IF NOT EXISTS idx_contracts_network_updated_at ON contracts (network, updated_at DESC);

-- (category, verified)
CREATE INDEX IF NOT EXISTS idx_contracts_category_verified ON contracts (category, is_verified);

-- (publisher_id, created_at DESC)
CREATE INDEX IF NOT EXISTS idx_contracts_publisher_created_at ON contracts (publisher_id, created_at DESC);
