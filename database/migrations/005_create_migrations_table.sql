CREATE TYPE migration_status AS ENUM ('pending', 'success', 'failed', 'rolled_back');

CREATE TABLE migrations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id VARCHAR(255) NOT NULL,
    status migration_status NOT NULL DEFAULT 'pending',
    wasm_hash VARCHAR(64) NOT NULL,
    log_output TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER update_migrations_timestamp
BEFORE UPDATE ON migrations
FOR EACH ROW
EXECUTE FUNCTION update_updated_at_column();
