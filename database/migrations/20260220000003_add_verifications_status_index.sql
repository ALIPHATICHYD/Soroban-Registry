-- no-transaction

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_verifications_status
    ON verifications (status);
