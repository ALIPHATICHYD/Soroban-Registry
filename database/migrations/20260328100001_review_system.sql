-- Review System Implementation
-- Adds status column for moderation workflow and improves review schema

-- Add review_status enum type if not exists
DO $$ BEGIN
    CREATE TYPE review_status AS ENUM ('pending', 'approved', 'rejected');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Add status column to reviews table
ALTER TABLE reviews
    ADD COLUMN IF NOT EXISTS status review_status NOT NULL DEFAULT 'pending';

-- Add index for status filtering (approved reviews are fetched most often)
CREATE INDEX IF NOT EXISTS idx_reviews_status ON reviews(status);

-- Add index for contract_id + status combination (common query pattern)
CREATE INDEX IF NOT EXISTS idx_reviews_contract_status ON reviews(contract_id, status);

-- Add unique constraint to prevent duplicate reviews per user per contract (optional, can be enabled)
-- This enforces one review per user per contract
CREATE UNIQUE INDEX IF NOT EXISTS idx_reviews_user_contract_unique 
    ON reviews(contract_id, user_id) 
    WHERE status != 'rejected';

-- Add updated_at trigger for reviews table
CREATE OR REPLACE FUNCTION update_reviews_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_reviews_updated_at 
    BEFORE UPDATE ON reviews
    FOR EACH ROW 
    EXECUTE FUNCTION update_reviews_updated_at_column();

-- Add constraint name for rating validation (for better error messages)
ALTER TABLE reviews DROP CONSTRAINT IF EXISTS reviews_rating_check;
ALTER TABLE reviews ADD CONSTRAINT reviews_rating_check 
    CHECK (rating >= 1.0 AND rating <= 5.0);

-- Comment columns for documentation
COMMENT ON COLUMN reviews.status IS 'Moderation status: pending (awaiting approval), approved (visible), rejected (hidden)';
COMMENT ON COLUMN reviews.is_flagged IS 'True if review has been flagged for inappropriate content';
COMMENT ON COLUMN reviews.helpful_count IS 'Net helpful votes (helpful - unhelpful)';
COMMENT ON TABLE review_votes IS 'Tracks user votes on review helpfulness (prevents duplicate votes)';
COMMENT ON TABLE review_flags IS 'Tracks flagged reviews for moderation review';
