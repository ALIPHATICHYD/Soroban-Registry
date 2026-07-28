-- 051_source_storage_and_access_log.sql
-- Store verified contract source artifacts and access logging for compliance

DO $$ BEGIN
    CREATE TYPE source_format_type AS ENUM ('rust', 'wasm');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS contract_sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_version_id UUID NOT NULL REFERENCES contract_versions(id) ON DELETE CASCADE,
    source_format source_format_type NOT NULL,
    storage_backend VARCHAR(50) NOT NULL,
    storage_key TEXT NOT NULL,
    source_hash VARCHAR(64) NOT NULL,
    source_size BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_version_id, source_format)
);

CREATE INDEX IF NOT EXISTS idx_contract_sources_contract_version_id
    ON contract_sources(contract_version_id);

CREATE TABLE IF NOT EXISTS source_access_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_source_id UUID NOT NULL REFERENCES contract_sources(id) ON DELETE CASCADE,
    action VARCHAR(20) NOT NULL,
    actor VARCHAR(56) NULL,
    request_ip VARCHAR(64) NULL,
    user_agent TEXT NULL,
    details JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_source_access_logs_contract_source_id
    ON source_access_logs(contract_source_id);

CREATE INDEX IF NOT EXISTS idx_source_access_logs_created_at
    ON source_access_logs(created_at DESC);
