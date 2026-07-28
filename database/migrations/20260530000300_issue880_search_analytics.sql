-- Issue #880: Full-text search integration with Elasticsearch.
--
-- Adds the search_analytics table for tracking query popularity and
-- the search_synonyms table for the synonym dictionary.

-- ── Search event log ──────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS search_analytics (
    id              BIGSERIAL       PRIMARY KEY,
    query_text      TEXT            NOT NULL,
    backend         TEXT            NOT NULL DEFAULT 'postgres',
    result_count    INT             NOT NULL DEFAULT 0,
    took_ms         INT             NOT NULL DEFAULT 0,
    filters         JSONB,
    user_agent      TEXT,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_search_analytics_created
    ON search_analytics (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_search_analytics_query
    ON search_analytics (query_text, created_at DESC);

-- ── Synonym dictionary ────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS search_synonyms (
    id              BIGSERIAL       PRIMARY KEY,
    term            TEXT            NOT NULL UNIQUE,
    synonyms        TEXT[]          NOT NULL DEFAULT '{}',
    is_active       BOOLEAN         NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

-- Seed a handful of domain-relevant synonyms
INSERT INTO search_synonyms (term, synonyms) VALUES
    ('token',    ARRAY['coin', 'asset', 'currency']),
    ('nft',      ARRAY['non-fungible', 'collectible']),
    ('dex',      ARRAY['exchange', 'swap', 'amm']),
    ('defi',     ARRAY['decentralized finance', 'yield', 'liquidity']),
    ('oracle',   ARRAY['price feed', 'data feed']),
    ('bridge',   ARRAY['cross-chain', 'interop'])
ON CONFLICT (term) DO NOTHING;

COMMENT ON TABLE search_analytics IS 'Per-query analytics log for popular-term tracking and performance measurement (issue #880).';
COMMENT ON TABLE search_synonyms  IS 'User-maintained synonym dictionary applied at search time (issue #880).';
