-- Migration: Contract deprecation state with lineage pointer (Issue #1090)
-- Additive-only: do not modify previously applied migration files.
-- Builds on 20260725000000_issue1061_deprecation_grace_period.sql, which already
-- added contracts.is_deprecated. This migration adds the lifecycle timestamp,
-- reason and replacement pointer so list/search responses can surface deprecation
-- state without joining contract_deprecations.

ALTER TABLE contracts
    ADD COLUMN IF NOT EXISTS deprecated_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS deprecation_reason TEXT,
    ADD COLUMN IF NOT EXISTS replacement_contract_id UUID REFERENCES contracts(id),
    ADD COLUMN IF NOT EXISTS is_deprecated BOOLEAN NOT NULL DEFAULT FALSE;

-- Generated lifecycle status so list/search always stay consistent with columns.
ALTER TABLE contracts
    ADD COLUMN IF NOT EXISTS deprecation_status TEXT
    GENERATED ALWAYS AS (
        CASE
            WHEN deprecated_at IS NULL THEN 'active'
            WHEN replacement_contract_id IS NOT NULL THEN 'superseded'
            ELSE 'deprecated'
        END
    ) STORED;

-- Backfill from the contract_deprecations side-table (Issue #65 / #1061) before
-- the consistency constraint is applied: #1061 already set is_deprecated = TRUE
-- for those rows, so deprecated_at must be populated to match.
UPDATE contracts c
SET
    deprecated_at = cd.deprecated_at,
    deprecation_reason = COALESCE(cd.deprecated_reason, cd.notes),
    replacement_contract_id = cd.replacement_contract_id,
    is_deprecated = TRUE
FROM contract_deprecations cd
WHERE cd.contract_id = c.id
  AND c.deprecated_at IS NULL;

-- Any row still flagged without a schedule row gets a timestamp so the flag and
-- the timestamp can never disagree.
UPDATE contracts
SET deprecated_at = NOW()
WHERE is_deprecated = TRUE
  AND deprecated_at IS NULL;

-- Keep is_deprecated aligned with deprecated_at for callers that filter on the flag.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'chk_contracts_deprecation_flag_consistency'
    ) THEN
        ALTER TABLE contracts
            ADD CONSTRAINT chk_contracts_deprecation_flag_consistency
            CHECK (
                (is_deprecated = FALSE AND deprecated_at IS NULL)
                OR
                (is_deprecated = TRUE AND deprecated_at IS NOT NULL)
            );
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_contracts_replacement_contract_id
    ON contracts (replacement_contract_id)
    WHERE replacement_contract_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_contracts_deprecation_status
    ON contracts (deprecation_status)
    WHERE deprecation_status <> 'active';

COMMENT ON COLUMN contracts.deprecated_at IS
    'When this contract version was marked deprecated (Issue #1090). NULL means active.';
COMMENT ON COLUMN contracts.deprecation_reason IS
    'Denormalized copy of contract_deprecations.deprecated_reason for list/search responses.';
COMMENT ON COLUMN contracts.replacement_contract_id IS
    'FK-style pointer to the recommended successor contract (lineage).';
COMMENT ON COLUMN contracts.deprecation_status IS
    'Generated: active | deprecated | superseded — superseded when a replacement_contract_id is set.';
