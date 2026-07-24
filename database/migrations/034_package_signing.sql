-- Contract Package Signing (Issue #67)
-- Cryptographic signing of contract packages for authenticity verification

CREATE TYPE signature_status AS ENUM ('valid', 'revoked', 'expired');

CREATE TYPE transparency_entry_type AS ENUM (
    'package_signed',
    'signature_verified',
    'signature_revoked',
    'key_rotated'
);

CREATE TABLE package_signatures (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id     UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    version         VARCHAR(50) NOT NULL,
    wasm_hash       VARCHAR(64) NOT NULL,
    signature       TEXT NOT NULL,
    signing_address VARCHAR(56) NOT NULL,
    public_key      TEXT NOT NULL,
    algorithm       VARCHAR(32) NOT NULL DEFAULT 'ed25519',
    status          signature_status NOT NULL DEFAULT 'valid',
    signed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ,
    revoked_at      TIMESTAMPTZ,
    revoked_reason  TEXT,
    revoked_by      VARCHAR(56),
    metadata        JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, version, signing_address)
);

CREATE INDEX idx_package_signatures_contract_id ON package_signatures(contract_id);
CREATE INDEX idx_package_signatures_signing_address ON package_signatures(signing_address);
CREATE INDEX idx_package_signatures_status ON package_signatures(status);
CREATE INDEX idx_package_signatures_wasm_hash ON package_signatures(wasm_hash);
CREATE INDEX idx_package_signatures_signed_at ON package_signatures(signed_at);

CREATE TRIGGER update_package_signatures_updated_at
    BEFORE UPDATE ON package_signatures
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TABLE signature_revocations (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    signature_id    UUID NOT NULL REFERENCES package_signatures(id) ON DELETE CASCADE,
    revoked_by      VARCHAR(56) NOT NULL,
    reason          TEXT NOT NULL,
    revoked_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_signature_revocations_signature_id ON signature_revocations(signature_id);
CREATE INDEX idx_signature_revocations_revoked_by ON signature_revocations(revoked_by);

CREATE TABLE signing_keys (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    publisher_id    UUID NOT NULL REFERENCES publishers(id) ON DELETE CASCADE,
    public_key      TEXT NOT NULL,
    key_fingerprint VARCHAR(128) NOT NULL,
    algorithm       VARCHAR(32) NOT NULL DEFAULT 'ed25519',
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deactivated_at  TIMESTAMPTZ,
    UNIQUE(publisher_id, key_fingerprint)
);

CREATE INDEX idx_signing_keys_publisher_id ON signing_keys(publisher_id);
CREATE INDEX idx_signing_keys_public_key ON signing_keys(public_key);
CREATE INDEX idx_signing_keys_is_active ON signing_keys(is_active);

CREATE TABLE transparency_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entry_type      transparency_entry_type NOT NULL,
    contract_id     UUID REFERENCES contracts(id) ON DELETE SET NULL,
    signature_id    UUID REFERENCES package_signatures(id) ON DELETE SET NULL,
    actor_address   VARCHAR(56) NOT NULL,
    previous_hash   VARCHAR(64),
    entry_hash      VARCHAR(64) NOT NULL,
    payload         JSONB,
    timestamp       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    immutable       BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_transparency_log_contract_id ON transparency_log(contract_id);
CREATE INDEX idx_transparency_log_signature_id ON transparency_log(signature_id);
CREATE INDEX idx_transparency_log_entry_type ON transparency_log(entry_type);
CREATE INDEX idx_transparency_log_timestamp ON transparency_log(timestamp);
CREATE UNIQUE INDEX idx_transparency_log_entry_hash ON transparency_log(entry_hash);
