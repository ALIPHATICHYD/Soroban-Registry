-- 061_add_contract_deployed_at.sql
ALTER TABLE contracts
    ADD COLUMN deployed_at TIMESTAMPTZ;

-- Backfill deployed_at from created_at for existing contracts
-- (Since we don't have historical deployment timestamps in the DB yet)
UPDATE contracts SET deployed_at = created_at WHERE deployed_at IS NULL;
