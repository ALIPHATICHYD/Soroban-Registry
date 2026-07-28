CREATE TABLE contract_templates (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug          TEXT UNIQUE NOT NULL,
    name          TEXT NOT NULL,
    description   TEXT,
    category      TEXT NOT NULL,
    version       TEXT NOT NULL DEFAULT '1.0.0',
    source_code   TEXT NOT NULL,
    parameters    JSONB NOT NULL DEFAULT '[]',
    install_count BIGINT NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE template_installs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_id UUID NOT NULL REFERENCES contract_templates(id) ON DELETE CASCADE,
    user_address TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_template_installs_template_id ON template_installs(template_id);
CREATE INDEX idx_contract_templates_category   ON contract_templates(category);
