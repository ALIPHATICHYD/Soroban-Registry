-- Migration: Contract Interaction History (Issue #46)
-- Extend contract_interactions with method, parameters, return_value and add indexes for fast queries.

-- Add columns for invocation details
ALTER TABLE contract_interactions
    ADD COLUMN IF NOT EXISTS method TEXT,
    ADD COLUMN IF NOT EXISTS parameters JSONB,
    ADD COLUMN IF NOT EXISTS return_value JSONB;

-- Allow interaction_type to be nullable for backward compatibility (new rows can set method instead)
-- Keep NOT NULL for now to avoid breaking existing rows; new inserts should set interaction_type to method or 'invocation'

-- Index for list + timeline: contract + time descending
CREATE INDEX IF NOT EXISTS idx_contract_interactions_contract_created_desc
    ON contract_interactions(contract_id, created_at DESC);

-- Index for filter by account
CREATE INDEX IF NOT EXISTS idx_contract_interactions_contract_user
    ON contract_interactions(contract_id, user_address);

-- Index for filter by method
CREATE INDEX IF NOT EXISTS idx_contract_interactions_contract_method
    ON contract_interactions(contract_id, method) WHERE method IS NOT NULL;

COMMENT ON COLUMN contract_interactions.method IS 'Contract method name invoked';
COMMENT ON COLUMN contract_interactions.parameters IS 'Invocation parameters (JSON)';
COMMENT ON COLUMN contract_interactions.return_value IS 'Invocation return value (JSON)';
