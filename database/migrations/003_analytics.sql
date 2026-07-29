-- Analytics event type enum
CREATE TYPE analytics_event_type AS ENUM (
    'contract_published',
    'contract_verified',
    'contract_deployed',
    'version_created'
);

-- Raw analytics events table
CREATE TABLE analytics_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type analytics_event_type NOT NULL,
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    user_address VARCHAR(56),
    network network_type,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_analytics_events_contract_created
    ON analytics_events(contract_id, created_at);
CREATE INDEX idx_analytics_events_type_created
    ON analytics_events(event_type, created_at);
CREATE INDEX idx_analytics_events_created_at
    ON analytics_events(created_at);

-- Daily aggregate summaries (permanent retention)
CREATE TABLE analytics_daily_aggregates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    deployment_count INTEGER NOT NULL DEFAULT 0,
    unique_deployers INTEGER NOT NULL DEFAULT 0,
    verification_count INTEGER NOT NULL DEFAULT 0,
    publish_count INTEGER NOT NULL DEFAULT 0,
    version_count INTEGER NOT NULL DEFAULT 0,
    total_events INTEGER NOT NULL DEFAULT 0,
    unique_users INTEGER NOT NULL DEFAULT 0,
    network_breakdown JSONB NOT NULL DEFAULT '{}',
    top_users JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, date)
);

CREATE INDEX idx_analytics_aggregates_contract_date
    ON analytics_daily_aggregates(contract_id, date);

-- Reuse the updated_at trigger for aggregates
CREATE TRIGGER update_analytics_daily_aggregates_updated_at
    BEFORE UPDATE ON analytics_daily_aggregates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
