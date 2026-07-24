-- Contract Compatibility Testing Matrix (Issue #261)
-- Tracks and tests contract compatibility across Soroban SDK versions,
-- Stellar networks, and Wasm runtime versions.

-- Compatibility status enum
CREATE TYPE compatibility_status AS ENUM ('compatible', 'warning', 'incompatible');

-- Main compatibility testing table
CREATE TABLE contract_compatibility (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    sdk_version VARCHAR(50) NOT NULL,
    wasm_runtime VARCHAR(50) NOT NULL,
    network VARCHAR(50) NOT NULL,
    compatible compatibility_status NOT NULL DEFAULT 'compatible',
    tested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    test_duration_ms INTEGER,
    test_output TEXT,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, sdk_version, wasm_runtime, network)
);

CREATE INDEX idx_compat_test_contract ON contract_compatibility(contract_id);
CREATE INDEX idx_compat_test_sdk ON contract_compatibility(sdk_version);
CREATE INDEX idx_compat_test_network ON contract_compatibility(network);
CREATE INDEX idx_compat_test_status ON contract_compatibility(compatible);
CREATE INDEX idx_compat_test_tested_at ON contract_compatibility(tested_at);

-- Historical compatibility changes for trend analysis
CREATE TABLE contract_compatibility_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    sdk_version VARCHAR(50) NOT NULL,
    wasm_runtime VARCHAR(50) NOT NULL,
    network VARCHAR(50) NOT NULL,
    previous_status compatibility_status,
    new_status compatibility_status NOT NULL,
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    change_reason TEXT
);

CREATE INDEX idx_compat_history_contract ON contract_compatibility_history(contract_id);
CREATE INDEX idx_compat_history_changed_at ON contract_compatibility_history(changed_at);

-- Publisher notifications for compatibility changes
CREATE TABLE compatibility_notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    sdk_version VARCHAR(50) NOT NULL,
    message TEXT NOT NULL,
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_compat_notif_contract ON compatibility_notifications(contract_id);
CREATE INDEX idx_compat_notif_unread ON compatibility_notifications(contract_id, is_read) WHERE NOT is_read;

-- Trigger for updated_at on contract_compatibility
CREATE TRIGGER update_contract_compatibility_updated_at
    BEFORE UPDATE ON contract_compatibility
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
