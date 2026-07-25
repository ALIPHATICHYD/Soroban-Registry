-- Migration: Contract deprecation state with lineage pointer (Issue #1090)
-- Additive-only: do not modify previously applied migration files.
-- Denormalizes deprecation onto contracts so list/search/trending can filter
-- and surface status without joining contract_deprecations.

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

CREATE INDEX IF NOT EXISTS idx_contracts_is_deprecated
    ON contracts (is_deprecated)
    WHERE is_deprecated = TRUE;

CREATE INDEX IF NOT EXISTS idx_contracts_replacement_contract_id
    ON contracts (replacement_contract_id)
    WHERE replacement_contract_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_contracts_deprecation_status
    ON contracts (deprecation_status)
    WHERE deprecation_status <> 'active';

COMMENT ON COLUMN contracts.deprecated_at IS
    'When this contract version was marked deprecated (Issue #1090). NULL means active.';
COMMENT ON COLUMN contracts.deprecation_reason IS
    'Human-readable reason for deprecation; immutable once set unless override undeprecate.';
COMMENT ON COLUMN contracts.replacement_contract_id IS
    'FK-style pointer to the recommended successor contract (lineage).';
COMMENT ON COLUMN contracts.is_deprecated IS
    'Denormalized flag for trending/similar/search filters; true iff deprecated_at IS NOT NULL.';
COMMENT ON COLUMN contracts.deprecation_status IS
    'Generated: active | deprecated | superseded — superseded when a replacement_contract_id is set.';

-- Backfill from existing contract_deprecations side-table (Issue #65) when present.
UPDATE contracts c
SET
    deprecated_at = cd.deprecated_at,
    deprecation_reason = cd.notes,
    replacement_contract_id = cd.replacement_contract_id,
    is_deprecated = TRUE
FROM contract_deprecations cd
WHERE cd.contract_id = c.id
  AND c.deprecated_at IS NULL;
