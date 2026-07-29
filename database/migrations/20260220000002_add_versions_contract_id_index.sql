-- no-transaction

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_versions_contract_id
    ON contract_versions (contract_id);
