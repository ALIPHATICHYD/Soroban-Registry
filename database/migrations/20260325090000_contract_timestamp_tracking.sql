ALTER TABLE contracts
    ADD COLUMN verified_at TIMESTAMPTZ,
    ADD COLUMN last_accessed_at TIMESTAMPTZ;

UPDATE contracts c
SET verified_at = latest.verified_at
FROM (
    SELECT DISTINCT ON (contract_id) contract_id, verified_at
    FROM verifications
    WHERE status = 'verified' AND verified_at IS NOT NULL
    ORDER BY contract_id, verified_at DESC
) AS latest
WHERE c.id = latest.contract_id;

CREATE INDEX idx_contracts_updated_at_desc ON contracts (updated_at DESC);
CREATE INDEX idx_contracts_verified_at_desc ON contracts (verified_at DESC);
CREATE INDEX idx_contracts_last_accessed_at_desc ON contracts (last_accessed_at DESC);
