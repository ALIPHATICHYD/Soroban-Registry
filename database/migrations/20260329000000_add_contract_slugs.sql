-- Add slug column to contracts table
ALTER TABLE contracts ADD COLUMN slug VARCHAR(255);

-- Generate slugs for existing contracts (simple lowercase and hyphenate)
UPDATE contracts 
SET slug = LOWER(REGEXP_REPLACE(name, '[^a-zA-Z0-9]+', '-', 'g'))
WHERE slug IS NULL;

-- Handle potential duplicate slugs for existing data by appending ID suffix if needed
-- (In a real migration, we might want more sophisticated logic, but this is a good start)
UPDATE contracts
SET slug = slug || '-' || SUBSTR(id::text, 1, 8)
WHERE slug IN (
    SELECT slug FROM contracts GROUP BY slug, network HAVING COUNT(*) > 1
);

-- Make slug NOT NULL and add unique constraint per network
ALTER TABLE contracts ALTER COLUMN slug SET NOT NULL;
ALTER TABLE contracts ADD CONSTRAINT contracts_slug_network_key UNIQUE (slug, network);

-- Add index for slug searches
CREATE INDEX idx_contracts_slug ON contracts(slug);
