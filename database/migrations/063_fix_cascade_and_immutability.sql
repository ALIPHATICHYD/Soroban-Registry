-- Fix ON DELETE CASCADE issues and add transparency log immutability
-- 
-- This migration addresses data integrity issues:
-- 1. Prevents accidental deletion of immutable transparency log references
-- 2. Prevents cascade deletion of contract production data
-- 3. Adds soft-delete columns for publishers and contracts
-- 4. Adds immutability trigger for transparency_log
--
-- OPTIONAL PRE-MIGRATION CHECKS:
-- 
-- 1. Check for existing orphaned transparency_log entries:
--   SELECT 
--     COUNT(*) FILTER (WHERE contract_id IS NULL) AS orphaned_contracts,
--     COUNT(*) FILTER (WHERE signature_id IS NULL) AS orphaned_signatures,
--     COUNT(*) AS total_entries
--   FROM transparency_log;
--
-- 2. Verify constraint names (PostgreSQL auto-generates as {table}_{column}_fkey):
--   SELECT conname, conrelid::regclass AS table_name, confrelid::regclass AS referenced_table
--   FROM pg_constraint
--   WHERE conname IN (
--     'contracts_publisher_id_fkey',
--     'contract_versions_contract_id_fkey',
--     'verifications_contract_id_fkey',
--     'contract_interactions_contract_id_fkey',
--     'package_signatures_contract_id_fkey',
--     'transparency_log_contract_id_fkey',
--     'transparency_log_signature_id_fkey'
--   );
--

-- ============================================================================
-- STEP 1: Add immutability trigger for transparency_log
-- ============================================================================

CREATE OR REPLACE FUNCTION enforce_transparency_log_immutability()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'Updates and deletions are strictly prohibited on transparency_log to ensure immutability.';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER prevent_transparency_log_modification
    BEFORE UPDATE OR DELETE ON transparency_log
    FOR EACH ROW EXECUTE FUNCTION enforce_transparency_log_immutability();

-- ============================================================================
-- STEP 2: Change ON DELETE CASCADE to ON DELETE RESTRICT
-- ============================================================================

-- 2.1 Publishers → Contracts
-- Prevent publisher deletion when contracts exist
ALTER TABLE contracts
    DROP CONSTRAINT contracts_publisher_id_fkey,
    ADD CONSTRAINT contracts_publisher_id_fkey
        FOREIGN KEY (publisher_id) REFERENCES publishers(id)
        ON DELETE RESTRICT;

-- 2.2 Contracts → Contract Versions
-- Prevent contract deletion when versions exist
ALTER TABLE contract_versions
    DROP CONSTRAINT contract_versions_contract_id_fkey,
    ADD CONSTRAINT contract_versions_contract_id_fkey
        FOREIGN KEY (contract_id) REFERENCES contracts(id)
        ON DELETE RESTRICT;

-- 2.3 Contracts → Verifications
-- Prevent contract deletion when verifications exist
ALTER TABLE verifications
    DROP CONSTRAINT verifications_contract_id_fkey,
    ADD CONSTRAINT verifications_contract_id_fkey
        FOREIGN KEY (contract_id) REFERENCES contracts(id)
        ON DELETE RESTRICT;

-- 2.4 Contracts → Contract Interactions
-- Prevent contract deletion when interaction history exists
ALTER TABLE contract_interactions
    DROP CONSTRAINT contract_interactions_contract_id_fkey,
    ADD CONSTRAINT contract_interactions_contract_id_fkey
        FOREIGN KEY (contract_id) REFERENCES contracts(id)
        ON DELETE RESTRICT;

-- 2.5 Contracts → Package Signatures
-- Prevent contract deletion when signatures exist
ALTER TABLE package_signatures
    DROP CONSTRAINT package_signatures_contract_id_fkey,
    ADD CONSTRAINT package_signatures_contract_id_fkey
        FOREIGN KEY (contract_id) REFERENCES contracts(id)
        ON DELETE RESTRICT;

-- ============================================================================
-- STEP 3: Change ON DELETE SET NULL to ON DELETE RESTRICT for transparency_log
-- ============================================================================

-- 3.1 Transparency Log → Contracts
-- Prevent contract deletion from orphaning transparency log entries
ALTER TABLE transparency_log
    DROP CONSTRAINT transparency_log_contract_id_fkey,
    ADD CONSTRAINT transparency_log_contract_id_fkey
        FOREIGN KEY (contract_id) REFERENCES contracts(id)
        ON DELETE RESTRICT;

-- 3.2 Transparency Log → Package Signatures
-- Prevent signature deletion from orphaning transparency log entries
ALTER TABLE transparency_log
    DROP CONSTRAINT transparency_log_signature_id_fkey,
    ADD CONSTRAINT transparency_log_signature_id_fkey
        FOREIGN KEY (signature_id) REFERENCES package_signatures(id)
        ON DELETE RESTRICT;

-- ============================================================================
-- STEP 4: Add soft-delete columns
-- ============================================================================

-- 4.1 Add deleted_at to publishers
ALTER TABLE publishers
    ADD COLUMN deleted_at TIMESTAMPTZ;

CREATE INDEX idx_publishers_deleted_at ON publishers(deleted_at)
    WHERE deleted_at IS NULL;

-- 4.2 Add deleted_at to contracts
ALTER TABLE contracts
    ADD COLUMN deleted_at TIMESTAMPTZ;

CREATE INDEX idx_contracts_deleted_at ON contracts(deleted_at)
    WHERE deleted_at IS NULL;

-- ============================================================================
-- Migration complete
-- ============================================================================
-- 
-- Next steps for application code:
-- 1. Update queries to filter WHERE deleted_at IS NULL for active records
-- 2. Implement soft-delete API endpoints (SET deleted_at = NOW())
-- 3. Create admin API for controlled contract archival/deprecation
-- 4. Consider creating views for active_publishers and active_contracts
-- 
-- Rollback is possible but will restore the CASCADE behavior. To rollback:
-- - Drop the triggers and function
-- - Change constraints back to CASCADE/SET NULL
-- - Drop the deleted_at columns and indexes
