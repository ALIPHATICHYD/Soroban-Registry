-- Marketplace Phase 2: Stripe payment integration.
--
-- Stores Stripe Checkout Sessions and the webhook events that drive
-- their state changes. License issuance happens server-side on
-- `checkout.session.completed`; the resulting license id is back-
-- linked here for traceability.

CREATE TABLE marketplace_stripe_payments (
    id                       UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id              UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    plan_id                  UUID NOT NULL REFERENCES contract_pricing_plans(id) ON DELETE RESTRICT,
    payer_id                 UUID NOT NULL REFERENCES publishers(id) ON DELETE CASCADE,
    -- Stripe identifiers
    stripe_checkout_session  VARCHAR(200) NOT NULL UNIQUE,
    stripe_payment_intent    VARCHAR(200),
    stripe_customer          VARCHAR(200),
    -- Pricing snapshot at session-creation time (audit trail; price on
    -- the plan can drift after the payment is started).
    amount_cents             BIGINT NOT NULL CHECK (amount_cents >= 0),
    currency                 CHAR(3) NOT NULL,
    status                   VARCHAR(20) NOT NULL DEFAULT 'pending'
                              CHECK (status IN ('pending', 'completed', 'failed', 'refunded', 'expired')),
    -- License issued on completion; back-linked once status='completed'.
    license_id               UUID REFERENCES contract_licenses(id) ON DELETE SET NULL,
    checkout_url             TEXT NOT NULL,
    metadata                 JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at             TIMESTAMPTZ
);

CREATE INDEX idx_stripe_payments_payer        ON marketplace_stripe_payments(payer_id);
CREATE INDEX idx_stripe_payments_contract     ON marketplace_stripe_payments(contract_id);
CREATE INDEX idx_stripe_payments_status       ON marketplace_stripe_payments(status);

-- Idempotency ledger for Stripe webhook events. Stripe retries on
-- non-2xx responses, so we MUST refuse to re-process by event id.
CREATE TABLE marketplace_stripe_webhook_events (
    event_id      VARCHAR(200) PRIMARY KEY,    -- Stripe `evt_…` id
    event_type    VARCHAR(100) NOT NULL,       -- e.g. checkout.session.completed
    received_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    payload       JSONB NOT NULL,
    -- nullable so we can record events that aren't tied to a payment
    -- (e.g. account.* events) without violating FK
    payment_id    UUID REFERENCES marketplace_stripe_payments(id) ON DELETE SET NULL
);

CREATE INDEX idx_stripe_webhook_events_received_at ON marketplace_stripe_webhook_events(received_at DESC);

CREATE TRIGGER trg_stripe_payments_touch
    BEFORE UPDATE ON marketplace_stripe_payments
    FOR EACH ROW EXECUTE FUNCTION marketplace_touch_updated_at();
