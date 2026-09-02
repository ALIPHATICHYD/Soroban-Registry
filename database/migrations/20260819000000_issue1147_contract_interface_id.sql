-- Issue #1147: persist the deterministic interface fingerprint of each contract.
--
-- `shared::interface_fingerprint::fingerprint_spec` already derives a stable
-- `soroban-interface-v1` identifier from a parsed contractspecv0 section, but
-- until now its only consumer was the CLI, computing it from a locally supplied
-- WASM file. Nothing on the server recorded it, so "the interface changed" was
-- not a question the registry could answer.
--
-- The dependency graph needs exactly that: an edge records the interface id its
-- target had when the edge was declared, and an incompatibility is drift between
-- that recorded id and the target's current one. Without a persisted column
-- every recorded id would be NULL and the rule could never fire.
--
-- Both columns are nullable. A publish without `wasm_artifact_base64` has no
-- bytes to fingerprint, matching the existing `artifact_scan_status = 'pending'`
-- semantics; so does an artifact that embeds no contract spec section.

ALTER TABLE contracts
    ADD COLUMN IF NOT EXISTS interface_id        TEXT,
    ADD COLUMN IF NOT EXISTS interface_algorithm TEXT;

COMMENT ON COLUMN contracts.interface_id IS
    'Deterministic interface fingerprint derived from the contractspecv0 section at publish time. NULL when no artifact or no spec section was supplied.';
COMMENT ON COLUMN contracts.interface_algorithm IS
    'Algorithm that produced interface_id (currently soroban-interface-v1). Stored alongside the id so fingerprints from a future algorithm are never compared against v1 ones.';

-- Comparisons are always equality against a known id, never a range scan, and
-- the column is NULL for every artifact-less contract, so a partial index keeps
-- it small.
CREATE INDEX IF NOT EXISTS idx_contracts_interface_id
    ON contracts (interface_id)
    WHERE interface_id IS NOT NULL;
