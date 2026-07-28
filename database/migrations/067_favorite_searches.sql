-- Migration: 051_favorite_searches.sql
-- Issue: Implement Advanced Search with Query Builder
-- Description: Table to store user's favorite searches for easy recall.

CREATE TABLE IF NOT EXISTS favorite_searches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID, -- Optional for now, can be linked to a publisher or user profile later
    name VARCHAR(255) NOT NULL,
    query_json JSONB NOT NULL, -- Stores the structured Query DSL
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for faster lookup by user (if implemented later)
CREATE INDEX IF NOT EXISTS idx_favorite_searches_user_id ON favorite_searches(user_id);

-- Trigger for updated_at
CREATE OR REPLACE FUNCTION update_favorite_searches_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_update_favorite_searches_updated_at
BEFORE UPDATE ON favorite_searches
FOR EACH ROW
EXECUTE FUNCTION update_favorite_searches_updated_at();
