-- Contract Version Tracking (Issue #486)
-- Adds current_version field to contracts for easy access to the active version,
-- and enriches contract_versions with change_notes, revert tracking, and
-- a direct version-lookup index.

-- Add current_version to contracts: reflects the latest active version string.
ALTER TABLE contracts ADD COLUMN IF NOT EXISTS current_version VARCHAR(50);

-- Add structured change notes to contract_versions (distinct from user-facing release_notes).
ALTER TABLE contract_versions ADD COLUMN IF NOT EXISTS change_notes TEXT;

-- Track whether a version was created by reverting to a previous version.
ALTER TABLE contract_versions ADD COLUMN IF NOT EXISTS is_revert BOOLEAN NOT NULL DEFAULT FALSE;

-- Store the version string that was reverted to (when is_revert = true).
ALTER TABLE contract_versions ADD COLUMN IF NOT EXISTS reverted_from VARCHAR(50);

-- Back-fill current_version for existing contracts from the most recently created version.
UPDATE contracts
SET current_version = (
    SELECT version
    FROM contract_versions
    WHERE contract_id = contracts.id
    ORDER BY created_at DESC
    LIMIT 1
)
WHERE EXISTS (
    SELECT 1 FROM contract_versions WHERE contract_id = contracts.id
);

-- Composite index to make single-version lookups fast.
CREATE INDEX IF NOT EXISTS idx_contract_versions_lookup
    ON contract_versions(contract_id, version);
