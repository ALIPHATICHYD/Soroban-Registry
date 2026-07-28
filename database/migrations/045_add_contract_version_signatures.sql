-- Add signature metadata to contract_versions for Ed25519 verification (Issue #255)

ALTER TABLE contract_versions
    ADD COLUMN signature TEXT,
    ADD COLUMN publisher_key TEXT,
    ADD COLUMN signature_algorithm TEXT DEFAULT 'ed25519';

