-- Contributors table: richer profile system for contract creators
CREATE TABLE contributors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stellar_address VARCHAR(56) NOT NULL UNIQUE,
    name VARCHAR(100),
    avatar_url TEXT,
    bio TEXT,
    links JSONB NOT NULL DEFAULT '{}',
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_contributors_stellar_address ON contributors(stellar_address);
CREATE INDEX idx_contributors_is_verified ON contributors(is_verified);

CREATE TRIGGER update_contributors_updated_at BEFORE UPDATE ON contributors
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
