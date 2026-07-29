-- Migration: 007_full_text_search.sql
-- Issue #19: Implement Full-Text Search with PostgreSQL
--
-- Strategy:
--   • Two STORED generated tsvector columns (name_search, description_search)
--     so the tsvectors are always in sync with their source columns at zero
--     runtime cost — PostgreSQL recomputes them automatically on INSERT/UPDATE.
--   • Separate GIN indexes on each column for index-only scans.
--   • A combined GIN index on the weighted concatenation used by the query.
--   • 'english' dictionary provides stemming ("tokens" ↔ "tokenization").

-- ── 1. Add stored tsvector columns ─────────────────────────────────────────
ALTER TABLE contracts
  ADD COLUMN IF NOT EXISTS name_search tsvector
    GENERATED ALWAYS AS (
      to_tsvector('english', name)
    ) STORED,

  ADD COLUMN IF NOT EXISTS description_search tsvector
    GENERATED ALWAYS AS (
      to_tsvector('english', COALESCE(description, ''))
    ) STORED;

-- ── 2. GIN indexes ──────────────────────────────────────────────────────────
-- Individual column indexes (used when filtering on one field only)
CREATE INDEX IF NOT EXISTS idx_contracts_name_search
  ON contracts USING GIN (name_search);

CREATE INDEX IF NOT EXISTS idx_contracts_description_search
  ON contracts USING GIN (description_search);

-- Combined weighted index — this is what the ranking query hits most often.
-- Weight A (name) ranks higher than weight B (description) via ts_rank.
CREATE INDEX IF NOT EXISTS idx_contracts_fts_combined
  ON contracts USING GIN (
    (
      setweight(name_search, 'A') ||
      setweight(description_search, 'B')
    )
  );

-- ── 3. Helper function: sanitise a raw user query into a tsquery ────────────
--
-- Supports:
--   • Bare words          → stemmed prefix match:  "dex"    → dex:*
--   • Quoted phrases      → exact phrase match:    "my dex"
--   • AND / OR / NOT ops  → passed through to websearch_to_tsquery
--   • Trailing *          → explicit prefix query: "token*" → token:*
--
-- We use websearch_to_tsquery for its safe, operator-aware parsing, which
-- handles AND / OR / NOT / phrase quoting and never throws on bad input.
-- For bare single-word queries we additionally append :* to get prefix
-- matching so "dex" also finds "dexterity", "dexter", etc.
CREATE OR REPLACE FUNCTION contracts_build_tsquery(raw_query TEXT)
RETURNS tsquery
LANGUAGE plpgsql
IMMUTABLE STRICT
AS $$
DECLARE
  cleaned TEXT;
  q       tsquery;
BEGIN
  cleaned := btrim(raw_query);

  -- Null / empty guard
  IF cleaned = '' THEN
    RETURN NULL;
  END IF;

  -- If the caller explicitly ended with * treat as prefix for every lexeme
  IF cleaned LIKE '%*' THEN
    -- Strip the trailing * and let to_tsquery handle it
    cleaned := left(cleaned, length(cleaned) - 1);
    BEGIN
      q := to_tsquery('english', replace(btrim(cleaned), ' ', ' & ') || ':*');
    EXCEPTION WHEN OTHERS THEN
      q := websearch_to_tsquery('english', cleaned);
    END;
    RETURN q;
  END IF;

  -- Use websearch_to_tsquery for multi-word / operator queries
  q := websearch_to_tsquery('english', cleaned);

  -- For single-word queries with no spaces and no operators, append prefix
  -- matching so partial words also match.
  IF cleaned NOT LIKE '% %'
     AND cleaned NOT LIKE '%&%'
     AND cleaned NOT LIKE '%|%'
     AND cleaned NOT LIKE '%-% '
  THEN
    BEGIN
      q := to_tsquery('english', cleaned || ':*');
    EXCEPTION WHEN OTHERS THEN
      -- Fall back to the websearch result already in q
      NULL;
    END;
  END IF;

  RETURN q;
END;
$$;

-- ── 4. Backfill statistics ──────────────────────────────────────────────────
-- The GENERATED columns are populated automatically when the ALTER TABLE
-- above executes, but ANALYZE helps the planner choose the new indexes.
ANALYZE contracts;