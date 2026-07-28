-- Create User Preferences table
CREATE TABLE user_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    publisher_id UUID NOT NULL UNIQUE REFERENCES publishers(id) ON DELETE CASCADE,
    theme VARCHAR(20) NOT NULL DEFAULT 'dark',
    language VARCHAR(10) NOT NULL DEFAULT 'en',
    default_network network_type NOT NULL DEFAULT 'testnet',
    favorites JSONB NOT NULL DEFAULT '[]',
    extensible_settings JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for publisher lookup
CREATE INDEX idx_user_preferences_publisher_id ON user_preferences(publisher_id);

-- Trigger to automatically update updated_at
CREATE TRIGGER update_user_preferences_updated_at BEFORE UPDATE ON user_preferences
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function to initialize preferences for new publishers
CREATE OR REPLACE FUNCTION initialize_publisher_preferences()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO user_preferences (publisher_id)
    VALUES (NEW.id);
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger to create preferences on publisher insertion
CREATE TRIGGER on_publisher_created
    AFTER INSERT ON publishers
    FOR EACH ROW EXECUTE FUNCTION initialize_publisher_preferences();

-- Initialize preferences for existing publishers (backfill)
INSERT INTO user_preferences (publisher_id)
SELECT id FROM publishers
ON CONFLICT (publisher_id) DO NOTHING;
