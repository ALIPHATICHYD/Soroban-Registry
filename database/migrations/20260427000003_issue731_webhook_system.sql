-- Migration: 20260427000000_issue731_webhook_system
-- Issue #731: Implement Webhook System for Contract Events

BEGIN;

-- ── 1. Add plain-text secret to webhook_configurations ───────────────────────
-- The delivery worker needs to sign payloads with HMAC-SHA256 using a plain
-- text secret. The existing secret_encrypted BYTEA column is not usable for
-- this without the encryption key at query time.
ALTER TABLE webhook_configurations
    ADD COLUMN IF NOT EXISTS secret TEXT;

-- ── 2. Extend notification_delivery_logs for webhook delivery tracking ────────
-- The webhook_delivery worker queries these columns; they don't exist yet.
ALTER TABLE notification_delivery_logs
    ADD COLUMN IF NOT EXISTS webhook_id UUID REFERENCES webhook_configurations(id) ON DELETE CASCADE,
    ADD COLUMN IF NOT EXISTS event_type TEXT,
    ADD COLUMN IF NOT EXISTS attempt_number INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS delivery_duration_ms BIGINT,
    ADD COLUMN IF NOT EXISTS response_body TEXT;

-- Allow notification_id to be optional (webhook-triggered deliveries have no
-- associated notification_queue entry).
ALTER TABLE notification_delivery_logs
    ALTER COLUMN notification_id DROP NOT NULL;

-- ── 3. Indexes for the new columns ───────────────────────────────────────────
CREATE INDEX IF NOT EXISTS idx_ndl_webhook_id
    ON notification_delivery_logs(webhook_id);

CREATE INDEX IF NOT EXISTS idx_ndl_webhook_status
    ON notification_delivery_logs(webhook_id, status);

CREATE INDEX IF NOT EXISTS idx_ndl_webhook_attempt
    ON notification_delivery_logs(webhook_id, attempt_number);

-- ── 4. Make webhook_configurations aware of contract event types ──────────────
-- The existing notification_types enum covers subscription types (new_version,
-- verification_status, etc.). Add a companion text[] for raw contract event
-- types (contract_created, verified, updated, deprecated) used by the
-- webhook-only delivery path.
ALTER TABLE webhook_configurations
    ADD COLUMN IF NOT EXISTS event_types TEXT[] NOT NULL DEFAULT ARRAY['contract_created','verified','updated','deprecated'];

COMMENT ON COLUMN webhook_configurations.secret IS
    'Plain-text HMAC-SHA256 signing secret returned only on webhook creation (#731)';

COMMENT ON COLUMN notification_delivery_logs.webhook_id IS
    'Foreign key to the webhook_configurations that triggered this delivery (#731)';

COMMENT ON COLUMN notification_delivery_logs.attempt_number IS
    'Zero-based retry attempt counter used by the delivery worker (#731)';

COMMIT;