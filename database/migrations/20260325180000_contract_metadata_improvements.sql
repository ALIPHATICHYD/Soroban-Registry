-- Migration: Contract Metadata Improvements
-- Description: Add last_verified_at, deployment_count, and audit_status to contracts table
-- Created: 2026-03-25

-- 1. Create audit_status_type enum
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'audit_status_type') THEN
        CREATE TYPE audit_status_type AS ENUM ('NONE', 'PENDING', 'PASSED', 'FAILED');
    END IF;
END$$;

-- 2. Add columns to contracts table
ALTER TABLE contracts 
    ADD COLUMN IF NOT EXISTS last_verified_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS deployment_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS audit_status audit_status_type NOT NULL DEFAULT 'NONE';

-- 3. Create indexes for the new columns
CREATE INDEX IF NOT EXISTS idx_contracts_last_verified_at ON contracts(last_verified_at);
CREATE INDEX IF NOT EXISTS idx_contracts_deployment_count ON contracts(deployment_count);
CREATE INDEX IF NOT EXISTS idx_contracts_audit_status ON contracts(audit_status);

-- 4. Add comments for clarity
COMMENT ON COLUMN contracts.last_verified_at IS 'Timestamp of the last successful source code verification';
COMMENT ON COLUMN contracts.deployment_count IS 'Total number of times this contract has been deployed/upgraded';
COMMENT ON COLUMN contracts.audit_status IS 'Current security audit status of the contract';

/*
-- ROLLBACK SCRIPT
DROP INDEX IF EXISTS idx_contracts_audit_status;
DROP INDEX IF EXISTS idx_contracts_deployment_count;
DROP INDEX IF EXISTS idx_contracts_last_verified_at;

ALTER TABLE contracts 
    DROP COLUMN IF EXISTS audit_status,
    DROP COLUMN IF EXISTS deployment_count,
    DROP COLUMN IF EXISTS last_verified_at;

DROP TYPE IF EXISTS audit_status_type;
*/
