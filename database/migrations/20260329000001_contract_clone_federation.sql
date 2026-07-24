-- Migration: 20260329000000_contract_clone_federation
-- Features: #487 Contract Mirror/Clone, #499 Federated Contract Registry Protocol

-- No migration in this repo ever creates the `auth_users` table, yet this
-- migration (and 20260330000000 / 20260330000001) declare foreign keys against
-- auth_users(id). On a fresh database that FK fails with:
--   relation "auth_users" does not exist
-- Create a minimal stub so the referencing constraints resolve. This is only an
-- FK target (id) — the full auth/user schema is missing from the migration set
-- and should be added properly as a follow-up.
CREATE TABLE IF NOT EXISTS auth_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ═══════════════════════════════════════════════════════════════════════════
-- #487: Contract Clone/Mirror Support
-- ═══════════════════════════════════════════════════════════════════════════

-- Add clone tracking to contracts table
ALTER TABLE contracts 
    ADD COLUMN cloned_from_id UUID REFERENCES contracts(id) ON DELETE SET NULL,
    ADD COLUMN clone_count INTEGER NOT NULL DEFAULT 0;

CREATE INDEX idx_contracts_cloned_from ON contracts(cloned_from_id);

-- Track clone relationships for analytics
CREATE TABLE contract_clone_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    parent_contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    cloned_contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    cloned_by UUID REFERENCES auth_users(id) ON DELETE SET NULL,
    cloned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata_overrides JSONB,
    network Network_type,
    CONSTRAINT unique_clone_pair UNIQUE (parent_contract_id, cloned_contract_id)
);

CREATE INDEX idx_contract_clone_history_parent ON contract_clone_history(parent_contract_id);
CREATE INDEX idx_contract_clone_history_cloned ON contract_clone_history(cloned_contract_id);

-- ═══════════════════════════════════════════════════════════════════════════
-- #499: Federated Registry Protocol Support
-- ═══════════════════════════════════════════════════════════════════════════

-- Registry peers for federation
CREATE TABLE federated_registries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    base_url VARCHAR(500) NOT NULL UNIQUE,
    public_key VARCHAR(255),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    federation_protocol_version VARCHAR(50) NOT NULL DEFAULT '1.0',
    last_synced_at TIMESTAMPTZ,
    sync_status VARCHAR(50) NOT NULL DEFAULT 'pending',
    sync_error TEXT,
    contracts_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_sync_status CHECK (sync_status IN ('pending', 'syncing', 'synced', 'error'))
);

CREATE INDEX idx_federated_registries_active ON federated_registries(is_active);
CREATE INDEX idx_federated_registries_sync_status ON federated_registries(sync_status);

-- Trigger to update updated_at
CREATE TRIGGER update_federated_registries_updated_at 
    BEFORE UPDATE ON federated_registries
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Contracts synced from federated registries
ALTER TABLE contracts
    ADD COLUMN federated_from_id UUID REFERENCES federated_registries(id) ON DELETE SET NULL,
    ADD COLUMN original_registry_contract_id VARCHAR(255),
    ADD COLUMN federation_metadata JSONB;

CREATE INDEX idx_contracts_federated_from ON contracts(federated_from_id);

-- Federation sync jobs
CREATE TABLE federation_sync_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    registry_id UUID NOT NULL REFERENCES federated_registries(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    contracts_synced INTEGER NOT NULL DEFAULT 0,
    contracts_failed INTEGER NOT NULL DEFAULT 0,
    duplicates_detected INTEGER NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_job_status CHECK (status IN ('pending', 'running', 'completed', 'failed'))
);

CREATE INDEX idx_federation_sync_jobs_registry ON federation_sync_jobs(registry_id);
CREATE INDEX idx_federation_sync_jobs_status ON federation_sync_jobs(status);

-- Federation sync results (individual contract sync tracking)
CREATE TABLE federation_sync_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES federation_sync_jobs(id) ON DELETE CASCADE,
    source_registry_id UUID NOT NULL REFERENCES federated_registries(id) ON DELETE CASCADE,
    source_contract_id VARCHAR(255) NOT NULL,
    local_contract_id UUID REFERENCES contracts(id) ON DELETE SET NULL,
    sync_action VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    error_message TEXT,
    synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT valid_sync_action CHECK (sync_action IN ('created', 'updated', 'skipped_duplicate', 'failed')),
    CONSTRAINT valid_sync_result_status CHECK (status IN ('success', 'failed', 'duplicate'))
);

CREATE INDEX idx_federation_sync_results_job ON federation_sync_results(job_id);
CREATE INDEX idx_federation_sync_results_source ON federation_sync_results(source_registry_id);
CREATE INDEX idx_federation_sync_results_local ON federation_sync_results(local_contract_id);

-- Federation protocol configuration
CREATE TABLE federation_protocol_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    config_key VARCHAR(255) NOT NULL UNIQUE,
    config_value JSONB NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_federation_protocol_config_key ON federation_protocol_config(config_key);

-- Trigger to update updated_at
CREATE TRIGGER update_federation_protocol_config_updated_at 
    BEFORE UPDATE ON federation_protocol_config
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert default federation protocol configuration
INSERT INTO federation_protocol_config (config_key, config_value, description) VALUES
    ('protocol_version', '{"version": "1.0", "supported_features": ["sync", "discovery", "attribution"]}', 'Federation protocol version and features'),
    ('sync_settings', '{"batch_size": 100, "timeout_seconds": 300, "retry_attempts": 3}', 'Sync operation settings'),
    ('duplicate_detection', '{"strategy": "wasm_hash", "fallback": "contract_id"}', 'How to detect duplicate contracts'),
    ('discovery', '{"enabled": true, "broadcast_interval_hours": 24}', 'Registry discovery settings'),
    ('attribution', '{"preserve_source": true, "add_watermark": true}', 'Attribution handling settings');

-- Comments for documentation
COMMENT ON COLUMN contracts.cloned_from_id IS 'References the original contract if this is a clone (#487)';
COMMENT ON COLUMN contracts.clone_count IS 'Number of times this contract has been cloned (#487)';
COMMENT ON TABLE contract_clone_history IS 'Tracks clone relationships between contracts (#487)';
COMMENT ON TABLE federated_registries IS 'External registries participating in federation (#499)';
COMMENT ON COLUMN contracts.federated_from_id IS 'References the federated registry this contract was synced from (#499)';
COMMENT ON COLUMN contracts.original_registry_contract_id IS 'Original contract ID in the source registry (#499)';
COMMENT ON TABLE federation_sync_jobs IS 'Tracks federation sync operations (#499)';
COMMENT ON TABLE federation_sync_results IS 'Individual contract sync results from federation (#499)';
COMMENT ON TABLE federation_protocol_config IS 'Configuration for federation protocol behavior (#499)';
