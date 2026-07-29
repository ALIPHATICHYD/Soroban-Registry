-- no-transaction

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_contracts_created
    ON contracts (created_at DESC);
