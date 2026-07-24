-- Issue #616: key management table for multisig publisher actions.

CREATE TABLE IF NOT EXISTS publisher_multisig_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    publisher_id UUID NOT NULL REFERENCES publishers(id) ON DELETE CASCADE,
    key_name VARCHAR(128) NOT NULL,
    public_key TEXT NOT NULL,
    algorithm VARCHAR(32) NOT NULL DEFAULT 'ed25519',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (publisher_id, public_key)
);

CREATE INDEX IF NOT EXISTS idx_publisher_multisig_keys_publisher
    ON publisher_multisig_keys(publisher_id);

CREATE INDEX IF NOT EXISTS idx_publisher_multisig_keys_active
    ON publisher_multisig_keys(publisher_id, is_active);

DROP TRIGGER IF EXISTS update_publisher_multisig_keys_updated_at ON publisher_multisig_keys;
CREATE TRIGGER update_publisher_multisig_keys_updated_at
    BEFORE UPDATE ON publisher_multisig_keys
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
