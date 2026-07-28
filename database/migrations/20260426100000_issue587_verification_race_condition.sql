-- Migration: Fix race condition in contract verification status updates (Issue #587)
--
-- Adds optimistic locking (version column) to the verifications table and a
-- composite unique index that prevents duplicate in-flight verifications for
-- the same contract.  The application layer uses SELECT … FOR UPDATE inside
-- explicit transactions to serialise concurrent status writes.

BEGIN;

-- 1. Add a version counter to verifications for optimistic locking.
--    Every UPDATE must increment this column; a stale writer will see
--    rowcount = 0 and can retry or surface a conflict error.
ALTER TABLE verifications
    ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 0;

-- 2. Prevent two concurrent requests from both inserting a 'pending' row for
--    the same contract at the same time.  Only one pending verification per
--    contract is meaningful; subsequent requests should wait for the current
--    one to finish.
CREATE UNIQUE INDEX IF NOT EXISTS idx_verifications_one_pending_per_contract
    ON verifications (contract_id)
    WHERE status = 'pending';

-- 3. Composite index used by the FOR UPDATE row-lock query in the handler.
CREATE INDEX IF NOT EXISTS idx_verifications_contract_status_created
    ON verifications (contract_id, status, created_at DESC);

-- 4. Add a version counter to contracts as well so the application can detect
--    a lost update when writing verification_status back to the contracts row.
ALTER TABLE contracts
    ADD COLUMN IF NOT EXISTS verification_version INTEGER NOT NULL DEFAULT 0;

COMMIT;
