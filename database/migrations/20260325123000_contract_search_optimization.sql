CREATE EXTENSION IF NOT EXISTS pg_trgm;

ALTER TABLE contracts
  ADD COLUMN IF NOT EXISTS category_search tsvector
    GENERATED ALWAYS AS (
      to_tsvector('english', COALESCE(category, ''))
    ) STORED,
  ADD COLUMN IF NOT EXISTS search_document tsvector
    GENERATED ALWAYS AS (
      setweight(to_tsvector('english', COALESCE(name, '')), 'A') ||
      setweight(to_tsvector('english', COALESCE(category, '')), 'B') ||
      setweight(to_tsvector('english', COALESCE(description, '')), 'C')
    ) STORED;

CREATE INDEX IF NOT EXISTS idx_contracts_category_search
  ON contracts USING GIN (category_search);

CREATE INDEX IF NOT EXISTS idx_contracts_search_document
  ON contracts USING GIN (search_document);

CREATE INDEX IF NOT EXISTS idx_contracts_name_trgm
  ON contracts USING GIN (lower(name) gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_contracts_category_trgm
  ON contracts USING GIN (lower(category) gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_contracts_name_prefix
  ON contracts (lower(name) text_pattern_ops);

CREATE INDEX IF NOT EXISTS idx_contracts_category_prefix
  ON contracts (lower(category) text_pattern_ops);

ANALYZE contracts;
