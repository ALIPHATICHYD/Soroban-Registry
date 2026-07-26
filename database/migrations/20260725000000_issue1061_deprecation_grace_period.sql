-- Migration: Add soft-delete and grace-period support for contract deprecation
-- Issue #1061: Backend – Add soft-delete and grace period for contract deprecation

-- 1. Add a fast `is_deprecated` flag directly on the contracts table so that
--    every query referencing contracts can cheaply filter deprecated ones without
--    joining to contract_deprecations.
ALTER TABLE contracts
    ADD COLUMN IF NOT EXISTS is_deprecated BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX IF NOT EXISTS idx_contracts_is_deprecated
    ON contracts(is_deprecated)
    WHERE is_deprecated = TRUE;

-- 2. Extend contract_deprecations with:
--    * deprecated_reason  – human-readable reason for the deprecation (mirrors npm deprecate)
--    * grace_period_days  – how many days after deprecated_at the contract is still
--                           resolvable before a background job hard-deletes it.
--                           NULL means "keep forever" (soft-delete only).
ALTER TABLE contract_deprecations
    ADD COLUMN IF NOT EXISTS deprecated_reason TEXT,
    ADD COLUMN IF NOT EXISTS grace_period_days INTEGER CHECK (grace_period_days IS NULL OR grace_period_days > 0);

COMMENT ON COLUMN contract_deprecations.deprecated_reason IS
    'Human-readable message displayed in API deprecation warnings (issue #1061)';

COMMENT ON COLUMN contract_deprecations.grace_period_days IS
    'Number of days after deprecated_at before the contract is eligible for hard deletion. '
    'NULL means the contract is soft-deleted indefinitely (never hard-deleted automatically).';

-- 3. Populate is_deprecated on contracts for any rows that already have a
--    deprecation record, so the flag stays consistent after the migration.
UPDATE contracts c
SET    is_deprecated = TRUE
WHERE  EXISTS (
    SELECT 1 FROM contract_deprecations cd WHERE cd.contract_id = c.id
);

-- 4. Index to make the background purge query fast:
--    find all contracts whose grace period has elapsed.
CREATE INDEX IF NOT EXISTS idx_contract_deprecations_grace_expiry
    ON contract_deprecations(
        deprecated_at,
        grace_period_days
    )
    WHERE grace_period_days IS NOT NULL;
