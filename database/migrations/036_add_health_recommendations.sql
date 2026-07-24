-- Add recommendations array to contract_health
ALTER TABLE contract_health ADD COLUMN IF NOT EXISTS recommendations TEXT[] NOT NULL DEFAULT '{}';
