-- Migration: Contract Deprecation Management
-- Issue #65: Add Contract Deprecation Management

CREATE TABLE IF NOT EXISTS contract_deprecations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    deprecated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retirement_at TIMESTAMPTZ NOT NULL,
    replacement_contract_id UUID REFERENCES contracts(id),
    migration_guide_url TEXT,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (contract_id)
);

CREATE INDEX IF NOT EXISTS idx_contract_deprecations_contract_id ON contract_deprecations(contract_id);
CREATE INDEX IF NOT EXISTS idx_contract_deprecations_retirement_at ON contract_deprecations(retirement_at);

CREATE TABLE IF NOT EXISTS contract_deprecation_notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    deprecated_contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    message TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    acknowledged_at TIMESTAMPTZ,
    UNIQUE (contract_id, deprecated_contract_id)
);

CREATE INDEX IF NOT EXISTS idx_deprecation_notifications_contract_id ON contract_deprecation_notifications(contract_id);
CREATE INDEX IF NOT EXISTS idx_deprecation_notifications_deprecated_contract_id ON contract_deprecation_notifications(deprecated_contract_id);
