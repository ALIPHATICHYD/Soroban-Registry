-- Migration: 20260727000001_issue1110_webhook_subscriptions
-- Issue #1110: Add webhook_subscriptions table with publisher_id, url, event_types[], secret.
-- Backs the push-notification system for contract.deprecated, ownership.transferred,
-- and vulnerability.found events.

BEGIN;

-- ── 1. webhook_subscriptions ─────────────────────────────────────────────────
-- Distinct from webhook_configurations (which uses user_id and supports
-- per-notification-type subscriptions).  This table is publisher-centric and
-- stores signed endpoints for lifecycle events.

CREATE TABLE IF NOT EXISTS webhook_subscriptions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    publisher_id  UUID NOT NULL REFERENCES publishers(id) ON DELETE CASCADE,
    url           TEXT NOT NULL,
    -- Array of event type strings, e.g.
    -- 'contract.deprecated', 'ownership.transferred', 'vulnerability.found'
    event_types   TEXT[] NOT NULL DEFAULT ARRAY['contract.deprecated','ownership.transferred','vulnerability.found'],
    -- Plain-text HMAC-SHA256 signing secret.  Only returned on creation.
    secret        TEXT NOT NULL,
    is_active     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_webhook_subscription_url_scheme
        CHECK (url LIKE 'https://%' OR url LIKE 'http://localhost%')
);

COMMENT ON TABLE webhook_subscriptions IS
    'Publisher-scoped webhook endpoints for lifecycle push events (#1110)';

COMMENT ON COLUMN webhook_subscriptions.secret IS
    'Plain-text HMAC-SHA256 signing secret; returned only on webhook creation (#1110)';

COMMENT ON COLUMN webhook_subscriptions.event_types IS
    'Subset of lifecycle events this endpoint is subscribed to (#1110)';

-- ── 2. Indexes ────────────────────────────────────────────────────────────────
CREATE INDEX IF NOT EXISTS idx_webhook_subs_publisher_id
    ON webhook_subscriptions(publisher_id);

CREATE INDEX IF NOT EXISTS idx_webhook_subs_active
    ON webhook_subscriptions(publisher_id, is_active)
    WHERE is_active = TRUE;

-- ── 3. webhook_subscription_deliveries ───────────────────────────────────────
-- Dead-letter / delivery log table for webhook_subscriptions deliveries.
-- Mirrors the structure of notification_delivery_logs but is keyed to
-- webhook_subscriptions.id instead of webhook_configurations.id so both
-- delivery paths can coexist without schema conflicts.

CREATE TABLE IF NOT EXISTS webhook_subscription_deliveries (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id     UUID NOT NULL REFERENCES webhook_subscriptions(id) ON DELETE CASCADE,
    event_type          TEXT NOT NULL,
    payload             JSONB NOT NULL,
    status              TEXT NOT NULL DEFAULT 'pending'
                        CHECK (status IN ('pending','processing','delivered','failed')),
    attempt_number      INTEGER NOT NULL DEFAULT 0,
    response_code       INTEGER,
    response_body       TEXT,
    error_message       TEXT,
    delivery_duration_ms BIGINT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE webhook_subscription_deliveries IS
    'Delivery log for webhook_subscriptions push events (#1110)';

CREATE INDEX IF NOT EXISTS idx_wsd_subscription_id
    ON webhook_subscription_deliveries(subscription_id);

CREATE INDEX IF NOT EXISTS idx_wsd_status_pending
    ON webhook_subscription_deliveries(status, created_at ASC)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_wsd_subscription_status
    ON webhook_subscription_deliveries(subscription_id, status);

COMMIT;
