-- Generated contract documentation artifacts.

CREATE TABLE IF NOT EXISTS contract_generated_docs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    contract_version_id UUID REFERENCES contract_versions(id) ON DELETE SET NULL,
    version_label VARCHAR(50) NOT NULL DEFAULT 'latest',
    format VARCHAR(20) NOT NULL DEFAULT 'markdown',
    template_name VARCHAR(100) NOT NULL DEFAULT 'default',
    template_body TEXT,
    content TEXT NOT NULL,
    source_checksum VARCHAR(64),
    generated_by VARCHAR(56),
    is_custom_override BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT contract_generated_docs_format_check CHECK (format IN ('markdown')),
    CONSTRAINT contract_generated_docs_unique UNIQUE (contract_id, version_label, template_name, format)
);

CREATE INDEX IF NOT EXISTS idx_contract_generated_docs_contract_created
    ON contract_generated_docs(contract_id, created_at DESC);

CREATE TRIGGER update_contract_generated_docs_updated_at
    BEFORE UPDATE ON contract_generated_docs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
