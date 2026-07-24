-- Migration: 076_contract_state_history.sql
-- Issue #647: Real-Time Contract State Monitor
-- Purpose: Track contract state changes and anomalies

-- Contract state history table (stores historical state values)
CREATE TABLE IF NOT EXISTS contract_state_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    state_key VARCHAR(255) NOT NULL,
    old_value TEXT,
    new_value TEXT,
    value_type VARCHAR(50), -- string, u64, i64, bool, json, bytes
    transaction_hash VARCHAR(64),
    ledger_index BIGINT,
    contract_version INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB DEFAULT '{}' -- additional context like invoker, gas used, etc.
);

CREATE INDEX idx_contract_state_history_contract_id ON contract_state_history(contract_id);
CREATE INDEX idx_contract_state_history_key ON contract_state_history(contract_id, state_key);
CREATE INDEX idx_contract_state_history_created_at ON contract_state_history(created_at DESC);
CREATE INDEX idx_contract_state_history_ledger ON contract_state_history(ledger_index);

-- Anomaly detection table (stores detected anomalies)
CREATE TABLE IF NOT EXISTS state_anomalies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    anomaly_type VARCHAR(100) NOT NULL, -- spike, unusual_pattern, unexpected_change, etc.
    severity VARCHAR(20) NOT NULL, -- low, medium, high, critical
    description TEXT NOT NULL,
    state_key VARCHAR(255),
    old_value TEXT,
    new_value TEXT,
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_resolved BOOLEAN NOT NULL DEFAULT FALSE,
    resolved_at TIMESTAMPTZ,
    resolution_notes TEXT,
    metadata JSONB DEFAULT '{}' -- threshold values, statistics, etc.
);

CREATE INDEX idx_state_anomalies_contract_id ON state_anomalies(contract_id);
CREATE INDEX idx_state_anomalies_detected_at ON state_anomalies(detected_at DESC);
CREATE INDEX idx_state_anomalies_severity ON state_anomalies(severity);
CREATE INDEX idx_state_anomalies_is_resolved ON state_anomalies(is_resolved);

-- Contract event history (for recent events with TTL for efficient cleanup)
CREATE TABLE IF NOT EXISTS contract_event_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB NOT NULL,
    ledger_index BIGINT,
    transaction_hash VARCHAR(64),
    published_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '30 days')
);

-- Index for cleanup and queries
CREATE INDEX idx_contract_event_history_contract_id ON contract_event_history(contract_id);
CREATE INDEX idx_contract_event_history_published_at ON contract_event_history(published_at DESC);
CREATE INDEX idx_contract_event_history_expires_at ON contract_event_history(expires_at);

-- Function to clean up old event history (run periodically)
CREATE OR REPLACE FUNCTION cleanup_expired_events()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM contract_event_history 
    WHERE expires_at < NOW();
    
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- View for recent state changes (last 24h)
CREATE OR REPLACE VIEW recent_state_changes AS
SELECT 
    c.id as contract_id,
    c.name as contract_name,
    c.network,
    sh.state_key,
    sh.old_value,
    sh.new_value,
    sh.created_at,
    sh.transaction_hash
FROM contract_state_history sh
JOIN contracts c ON c.id = sh.contract_id
WHERE sh.created_at > NOW() - INTERVAL '24 hours'
ORDER BY sh.created_at DESC;

-- View for active anomalies
CREATE OR REPLACE VIEW active_anomalies AS
SELECT 
    a.id,
    c.name as contract_name,
    c.network,
    a.anomaly_type,
    a.severity,
    a.description,
    a.detected_at,
    a.state_key,
    a.new_value
FROM state_anomalies a
JOIN contracts c ON c.id = a.contract_id
WHERE a.is_resolved = FALSE
ORDER BY 
    CASE a.severity
        WHEN 'critical' THEN 1
        WHEN 'high' THEN 2
        WHEN 'medium' THEN 3
        WHEN 'low' THEN 4
        ELSE 5
    END,
    a.detected_at DESC;
