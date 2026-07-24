-- Migration 014: Search Optimization
-- Adds full-text search capabilities and ranking signals

-- 1. Add search_vector column to contracts
ALTER TABLE contracts ADD COLUMN IF NOT EXISTS search_vector tsvector;

-- 2. Create function to update search_vector
CREATE OR REPLACE FUNCTION contracts_search_vector_update() RETURNS trigger AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('english', coalesce(NEW.name, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(NEW.description, '')), 'B');
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

-- 3. Create trigger to keep search_vector updated
DROP TRIGGER IF EXISTS trg_contracts_search_vector_update ON contracts;
CREATE TRIGGER trg_contracts_search_vector_update
    BEFORE INSERT OR UPDATE OF name, description
    ON contracts
    FOR EACH ROW
    EXECUTE FUNCTION contracts_search_vector_update();

-- 4. Update existing rows
UPDATE contracts SET search_vector = 
    setweight(to_tsvector('english', coalesce(name, '')), 'A') ||
    setweight(to_tsvector('english', coalesce(description, '')), 'B');

-- 5. Create GIN index for fast search
CREATE INDEX IF NOT EXISTS contracts_search_vector_idx ON contracts USING GIN(search_vector);

-- 6. Add search_click to analytics_event_type if not exists
-- PostgreSQL doesn't support ALTER TYPE ADD VALUE IF NOT EXISTS easily in a transaction
-- We'll use a DO block to handle it
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type t JOIN pg_enum e ON t.oid = e.enumtypid WHERE t.typname = 'analytics_event_type' AND e.enumlabel = 'search_click') THEN
        ALTER TYPE analytics_event_type ADD VALUE 'search_click';
    END IF;
END
$$;
