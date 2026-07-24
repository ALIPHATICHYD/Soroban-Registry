-- Issue #888: Contract signature verification system.
--
-- Cryptographic authentication of contracts via deployer signatures, with
-- multi-algorithm support (Ed25519, secp256k1/ECDSA), certificate chains,
-- a revocation list, timestamp validity windows, and key rotation.
--
-- Relationship to the existing `package_signatures` (signing_handlers.rs):
-- that table is the Ed25519-only package-signing + transparency-log subsystem.
-- This migration adds the broader verification system (#888) under distinct
-- table names so the two coexist.
--
-- However, the #888 `signing_keys` table below is NOT actually distinct from the
-- one created by migration 034_package_signing.sql, which already owns the name
-- with an unrelated schema (publisher_id, key_fingerprint, is_active). Because
-- the CREATE below uses IF NOT EXISTS, on a fresh database it silently binds to
-- 034's table and the subsequent index on `owner` fails:
--   column "owner" does not exist
-- The backend's signature_verification.rs expects THIS migration's schema
-- (key_id, owner, status, rotated_to), and 034's table has no backend consumers,
-- so move 034's table (and its indexes) aside and let #888 own `signing_keys`.
ALTER TABLE IF EXISTS signing_keys RENAME TO package_signing_keys;
ALTER INDEX IF EXISTS idx_signing_keys_publisher_id RENAME TO idx_package_signing_keys_publisher_id;
ALTER INDEX IF EXISTS idx_signing_keys_public_key    RENAME TO idx_package_signing_keys_public_key;
ALTER INDEX IF EXISTS idx_signing_keys_is_active     RENAME TO idx_package_signing_keys_is_active;

-- ── Signing keys (deployer keys + certificate-chain authorities) ──────────────
CREATE TABLE IF NOT EXISTS signing_keys (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Deterministic fingerprint: hex(sha256(algorithm || ':' || raw public key)).
    key_id        TEXT        NOT NULL UNIQUE,
    -- Who controls the key (deployer Stellar address, org id, or CA name).
    owner         TEXT        NOT NULL,
    -- 'ed25519' | 'secp256k1'.
    algorithm     TEXT        NOT NULL,
    -- Base64-encoded public key (32 bytes ed25519; SEC1 compressed/uncompressed secp256k1).
    public_key    TEXT        NOT NULL,
    -- Issuer fingerprint for certificate chains (NULL for self-issued/root).
    parent_key_id TEXT,
    -- Parent's base64 signature over this key's raw public-key bytes (the cert).
    cert_signature TEXT,
    -- Trusted anchor: a root may terminate a chain.
    is_root       BOOLEAN     NOT NULL DEFAULT FALSE,
    -- Validity window for timestamp checks.
    not_before    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    not_after     TIMESTAMPTZ,
    -- 'active' | 'revoked' | 'rotated'.
    status        TEXT        NOT NULL DEFAULT 'active',
    -- Replacement key fingerprint after rotation (old key stays for historical sigs).
    rotated_to    TEXT,
    metadata      JSONB       NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_signing_keys_owner       ON signing_keys (owner);
CREATE INDEX IF NOT EXISTS idx_signing_keys_parent      ON signing_keys (parent_key_id);
CREATE INDEX IF NOT EXISTS idx_signing_keys_status      ON signing_keys (status);

-- ── Stored contract signatures ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS contract_signatures (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Optional link to a registry contract row.
    contract_id     UUID        REFERENCES contracts(id) ON DELETE CASCADE,
    -- Free-form subject reference (on-chain id, package coordinate, etc.).
    contract_ref    TEXT        NOT NULL,
    -- The exact message/hash that was signed (e.g. the wasm hash).
    subject_hash    TEXT        NOT NULL,
    algorithm       TEXT        NOT NULL,
    -- Base64 signature bytes.
    signature       TEXT        NOT NULL,
    -- Fingerprint of the signing key (joins signing_keys.key_id).
    key_id          TEXT        NOT NULL,
    -- Claimed signing time, and optional validity window.
    signed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    not_before      TIMESTAMPTZ,
    expires_at      TIMESTAMPTZ,
    -- Result of the most recent verification.
    verified        BOOLEAN     NOT NULL DEFAULT FALSE,
    last_verified_at TIMESTAMPTZ,
    metadata        JSONB       NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_contract_signatures_contract  ON contract_signatures (contract_id);
CREATE INDEX IF NOT EXISTS idx_contract_signatures_key       ON contract_signatures (key_id);
CREATE INDEX IF NOT EXISTS idx_contract_signatures_subject   ON contract_signatures (subject_hash);

-- ── Revocation list ───────────────────────────────────────────────────────────
-- Unlike signing_keys, signature_revocations already exists from migration 034
-- AND is still used by the package-signing path (signing_handlers.rs inserts
-- signature_id/revoked_by/reason), so it cannot be renamed aside. The original
-- CREATE TABLE IF NOT EXISTS here silently bound to 034's table, which lacks the
-- key_id column the #888 index and signature_verification.rs need, failing with:
--   column "key_id" does not exist
-- Augment the existing table with the key-based-revocation column instead so both
-- the package-signing and #888 verification paths share it.
-- NOTE (follow-up): 034 declares signature_id NOT NULL with an FK to
-- package_signatures; the #888 key-based revocation path inserts rows with a NULL
-- signature_id, so fully enabling that path also requires relaxing that
-- constraint. That is a schema-reconciliation decision left for a dedicated PR.
ALTER TABLE signature_revocations ADD COLUMN IF NOT EXISTS key_id TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_signature_revocations_key
    ON signature_revocations (key_id) WHERE key_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_signature_revocations_sig
    ON signature_revocations (signature_id) WHERE signature_id IS NOT NULL;

COMMENT ON TABLE signing_keys IS
    'Deployer/CA keys for the contract signature verification system, incl. cert chains and rotation (issue #888).';
COMMENT ON TABLE contract_signatures IS
    'Stored contract signatures with algorithm, validity window, and verification metadata (issue #888).';
COMMENT ON TABLE signature_revocations IS
    'Revocation list for signing keys and individual signatures (issue #888).';
