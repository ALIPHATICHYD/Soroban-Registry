-- Indexer state tracking table
-- Tracks the last indexed ledger height for each network to enable safe resume after restarts

CREATE TABLE indexer_state (
    id SERIAL PRIMARY KEY,
    network network_type NOT NULL UNIQUE,
    last_indexed_ledger_height BIGINT NOT NULL DEFAULT 0,
    last_checkpoint_ledger_height BIGINT NOT NULL DEFAULT 0,
    indexed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    checkpoint_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    error_message TEXT,
    consecutive_failures INT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_indexer_state_network ON indexer_state(network);
CREATE INDEX idx_indexer_state_updated_at ON indexer_state(updated_at);

-- Function to update updated_at for indexer_state
CREATE OR REPLACE FUNCTION update_indexer_state_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger to auto-update updated_at
CREATE TRIGGER update_indexer_state_timestamp BEFORE UPDATE ON indexer_state
    FOR EACH ROW EXECUTE FUNCTION update_indexer_state_timestamp();

-- Initialize state for all networks
INSERT INTO indexer_state (network, last_indexed_ledger_height, last_checkpoint_ledger_height)
VALUES 
    ('mainnet', 0, 0),
    ('testnet', 0, 0),
    ('futurenet', 0, 0)
ON CONFLICT (network) DO NOTHING;
