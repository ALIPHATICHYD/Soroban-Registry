-- Add popularity scoring to contracts

ALTER TABLE contracts
    ADD COLUMN popularity_score DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    ADD COLUMN score_updated_at TIMESTAMPTZ;

-- Index for efficient trending queries (sorted by score descending)
CREATE INDEX idx_contracts_popularity_score ON contracts (popularity_score DESC);

-- Composite index for the trending filter (exclude low-quality + sort by score)
CREATE INDEX idx_contracts_trending
    ON contracts (popularity_score DESC)
    WHERE is_verified = true OR popularity_score > 0;
