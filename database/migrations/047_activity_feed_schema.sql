-- Add new event types
ALTER TYPE analytics_event_type ADD VALUE 'contract_updated';
ALTER TYPE analytics_event_type ADD VALUE 'publisher_created';

-- Make contract_id nullable and add publisher_id
ALTER TABLE analytics_events 
    ALTER COLUMN contract_id DROP NOT NULL,
    ADD COLUMN publisher_id UUID REFERENCES publishers(id) ON DELETE CASCADE;

-- Index for publisher-based filtering
CREATE INDEX idx_analytics_events_publisher_id ON analytics_events(publisher_id);
