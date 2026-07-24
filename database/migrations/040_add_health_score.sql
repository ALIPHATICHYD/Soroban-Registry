-- Add health_score column to contracts table
ALTER TABLE contracts ADD COLUMN health_score INTEGER NOT NULL DEFAULT 0;

-- Create index for health_score sorting
CREATE INDEX idx_contracts_health_score ON contracts(health_score);

-- Comment explaining the score range
COMMENT ON COLUMN contracts.health_score IS 'Contract health score from 0-100 calculated daily based on verification, activity, and security.';
