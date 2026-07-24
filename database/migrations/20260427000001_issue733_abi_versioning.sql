-- Migration: 20260427000001_issue733_abi_versioning
-- Issue #733: Contract ABI Versioning and Compatibility Checking

BEGIN;

-- Add deprecation flag and changelog to contract_abis (table created in 002_add_abi.sql)
ALTER TABLE contract_abis
    ADD COLUMN IF NOT EXISTS is_deprecated BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS deprecated_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS changelog TEXT;

-- Index for non-deprecated ABI lookups
CREATE INDEX IF NOT EXISTS idx_contract_abis_not_deprecated
    ON contract_abis(contract_id, created_at DESC)
    WHERE is_deprecated = FALSE;

COMMENT ON COLUMN contract_abis.is_deprecated IS
    'Marks an ABI version as deprecated; deprecated versions are excluded from the latest lookup (#733)';

COMMIT;