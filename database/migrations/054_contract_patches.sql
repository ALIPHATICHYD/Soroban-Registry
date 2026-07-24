-- Differential contract update pipeline (Issue #501)
-- Stores deltas between consecutive contract versions so that only changed
-- fields are persisted, reducing storage for frequently-updated contracts.

CREATE TABLE contract_patches (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id     UUID        NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    -- Empty string ('') means this patch is the baseline for the first version
    -- (carries the full initial state).  Non-empty values are the semver string
    -- of the preceding version.  We use '' instead of NULL so that the UNIQUE
    -- constraint works correctly (PostgreSQL treats two NULLs as distinct).
    from_version    TEXT        NOT NULL DEFAULT '',
    to_version      TEXT        NOT NULL,
    -- JSON object with only the fields that changed, plus an ABI change summary.
    -- Structure:
    --   { "fields": { "<field>": <new_value>, ... },
    --     "abi_changes": { "added": [...], "removed": [...], "modified": [...] } }
    patch           JSONB       NOT NULL DEFAULT '{}',
    -- Sizes in bytes for storage-savings reporting.
    patch_size_bytes    INT     NOT NULL DEFAULT 0,
    full_size_bytes     INT     NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, from_version, to_version)
);

CREATE INDEX idx_contract_patches_contract_id ON contract_patches(contract_id);
CREATE INDEX idx_contract_patches_to_version  ON contract_patches(contract_id, to_version);
