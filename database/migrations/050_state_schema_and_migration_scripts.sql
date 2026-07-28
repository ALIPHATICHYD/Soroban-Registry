-- Add state_schema to contract_versions and create migration_scripts table

ALTER TABLE contract_versions
    ADD COLUMN state_schema JSONB NULL;

CREATE TABLE migration_scripts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_version UUID NOT NULL REFERENCES contract_versions(id) ON DELETE CASCADE,
    to_version UUID NOT NULL REFERENCES contract_versions(id) ON DELETE CASCADE,
    script_path VARCHAR(1024) NOT NULL,
    checksum VARCHAR(128) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_migration_scripts_from_to ON migration_scripts(from_version, to_version);
