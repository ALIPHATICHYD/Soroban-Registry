-- Migration: Contract Event Indexing
-- Issue #44: Implement Contract Event Indexing and Queries

-- Create enum for event types
CREATE TYPE contract_event_type AS (
    topic TEXT,
    data JSONB
);

-- Create events table for indexing contract-emitted events
CREATE TABLE IF NOT EXISTS contract_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id TEXT NOT NULL,
    topic TEXT NOT NULL,
    data JSONB,
    ledger_sequence BIGINT NOT NULL,
    transaction_hash TEXT,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    network network_type NOT NULL DEFAULT 'testnet',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_contract_events_contract_id ON contract_events(contract_id);
CREATE INDEX IF NOT EXISTS idx_contract_events_topic ON contract_events(topic);
CREATE INDEX IF NOT EXISTS idx_contract_events_ledger ON contract_events(ledger_sequence);
CREATE INDEX IF NOT EXISTS idx_contract_events_timestamp ON contract_events(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_contract_events_network ON contract_events(network);
CREATE INDEX IF NOT EXISTS idx_contract_events_contract_topic ON contract_events(contract_id, topic);
CREATE INDEX IF NOT EXISTS idx_contract_events_data ON contract_events USING GIN (data);

-- Create composite index for common query patterns
CREATE INDEX IF NOT EXISTS idx_contract_events_query ON contract_events(contract_id, topic, timestamp DESC);

-- Add constraints
ALTER TABLE contract_events ADD CONSTRAINT unique_event_per_ledger UNIQUE (contract_id, ledger_sequence, transaction_hash);

-- Create retention policy table
CREATE TABLE IF NOT EXISTS event_retention_policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id TEXT,
    network network_type,
    retention_days INTEGER NOT NULL DEFAULT 365,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default retention policy (1 year)
INSERT INTO event_retention_policies (retention_days) VALUES (365) ON CONFLICT DO NOTHING;

-- Create function to clean old events based on retention policy
CREATE OR REPLACE FUNCTION clean_old_events()
RETURNS void AS $$
BEGIN
    DELETE FROM contract_events
    WHERE timestamp < NOW() - INTERVAL '1 year';
END;
$$ LANGUAGE plpgsql;

-- Comment on table
COMMENT ON TABLE contract_events IS 'Indexed contract events for querying and analytics';
COMMENT ON COLUMN contract_events.contract_id IS 'The on-chain contract address that emitted the event';
COMMENT ON COLUMN contract_events.topic IS 'The event topic/name (e.g., swap, transfer)';
COMMENT ON COLUMN contract_events.data IS 'JSON payload of event data';
COMMENT ON COLUMN contract_events.ledger_sequence IS 'Stellar ledger number for reorg safety';
COMMENT ON COLUMN contract_events.transaction_hash IS 'Transaction hash that triggered the event';
