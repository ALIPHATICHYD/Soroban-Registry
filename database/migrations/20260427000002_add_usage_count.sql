-- Migration: Add usage counter to contracts table
-- Issue #607: Implement Contract Usage Counter Feature
-- Description: Add usage_count field to track API access frequency

-- Add usage_count column to contracts table
ALTER TABLE contracts 
    ADD COLUMN usage_count BIGINT NOT NULL DEFAULT 0;

-- Add database constraint to ensure non-negative values
ALTER TABLE contracts 
    ADD CONSTRAINT chk_contracts_usage_count_non_negative 
    CHECK (usage_count >= 0);

-- Create index for efficient statistics queries
CREATE INDEX idx_contracts_usage_count ON contracts(usage_count DESC);

-- Update existing contracts to have zero usage count (already handled by DEFAULT)
-- No additional data migration needed as DEFAULT 0 handles existing rows