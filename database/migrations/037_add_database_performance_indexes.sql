-- Add performance indexes for high-traffic contract and verification queries.
-- (CONCURRENTLY omitted so migration runs inside SQLx transaction.)

CREATE INDEX IF NOT EXISTS idx_contracts_publisher_network
    ON contracts (publisher_id, network);

CREATE INDEX IF NOT EXISTS idx_contracts_created
    ON contracts (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_versions_contract_id
    ON contract_versions (contract_id);

CREATE INDEX IF NOT EXISTS idx_verifications_status
    ON verifications (status);
