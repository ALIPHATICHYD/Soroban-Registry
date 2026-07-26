-- Issue #1058
-- Ownership transfer table for contract/package publisher transfers.
-- Supports two-party confirmation, expiring requests, and audit logging.

CREATE TABLE IF NOT EXISTS ownership_transfers (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id     UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    from_publisher_id  UUID NOT NULL REFERENCES publishers(id),
    to_publisher_id    UUID NOT NULL REFERENCES publishers(id),
    from_confirmation BOOLEAN NOT NULL DEFAULT FALSE,
    to_confirmation   BOOLEAN NOT NULL DEFAULT FALSE,
    status            VARCHAR(20) NOT NULL DEFAULT 'pending',
    expires_at        TIMESTAMPTZ NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at      TIMESTAMPTZ,
    completed_at      TIMESTAMPTZ,
    created_by        UUID NOT NULL REFERENCES publishers(id),
    CONSTRAINT chk_status CHECK (status IN ('pending', 'confirmed', 'completed', 'expired', 'rejected', 'duplicate')),
    CONSTRAINT chk_different_publishers CHECK (from_publisher_id <> to_publisher_id),
    CONSTRAINT chk_confirmation_logic CHECK (
        (status = 'pending' AND from_confirmation = FALSE AND to_confirmation = FALSE) OR
        (status = 'confirmed' AND to_confirmation = TRUE) OR
        (status = 'completed' AND from_confirmation = TRUE AND to_confirmation = TRUE) OR
        (status IN ('expired', 'rejected', 'duplicate'))
    )
);

CREATE INDEX IF NOT EXISTS idx_ownership_transfers_contract_id
    ON ownership_transfers(contract_id);

CREATE INDEX IF NOT EXISTS idx_ownership_transfers_from_publisher
    ON ownership_transfers(from_publisher_id);

CREATE INDEX IF NOT EXISTS idx_ownership_transfers_to_publisher
    ON ownership_transfers(to_publisher_id);

CREATE INDEX IF NOT EXISTS idx_ownership_transfers_status
    ON ownership_transfers(status);

CREATE INDEX IF NOT EXISTS idx_ownership_transfers_expires_at
    ON ownership_transfers(expires_at);

CREATE TABLE IF NOT EXISTS ownership_transfer_logs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transfer_id     UUID NOT NULL REFERENCES ownership_transfers(id) ON DELETE CASCADE,
    actor_id        UUID NOT NULL REFERENCES publishers(id),
    actor_type      VARCHAR(20) NOT NULL CHECK (actor_type IN ('publisher', 'system')),
    action          VARCHAR(30) NOT NULL,
    details         JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ownership_transfer_logs_transfer_id
    ON ownership_transfer_logs(transfer_id);

CREATE INDEX IF NOT EXISTS idx_ownership_transfer_logs_created_at
    ON ownership_transfer_logs(created_at DESC);
