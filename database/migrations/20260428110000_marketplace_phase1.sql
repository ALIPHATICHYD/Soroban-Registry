-- Marketplace Phase 1: paid contract pricing, licensing, and usage metering.
--
-- Phase 1 covers schema + license issuance/validation + metering only.
-- Payment-provider integration (Stripe, USDC) lands in later phases and
-- will reference the tables introduced here.

-- ── Pricing plans ─────────────────────────────────────────────────────
-- A pricing plan describes how a single contract can be licensed. A
-- contract may have many plans (e.g. Free / Pro / Enterprise).
CREATE TABLE contract_pricing_plans (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id     UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    name            VARCHAR(100) NOT NULL,
    description     TEXT,
    -- Price in the smallest currency unit (cents for USD). 0 means free.
    price_cents     BIGINT NOT NULL CHECK (price_cents >= 0),
    currency        CHAR(3) NOT NULL DEFAULT 'USD',
    -- 'monthly' = recurring subscription, 'one_time' = single payment.
    -- Phase 1 only issues licenses for 'monthly' and 'one_time'; metered
    -- tiers arrive with the billing pipeline.
    billing_period  VARCHAR(20) NOT NULL DEFAULT 'monthly'
                     CHECK (billing_period IN ('monthly', 'one_time')),
    -- Maximum metered calls per billing period; NULL means unlimited.
    call_quota      BIGINT CHECK (call_quota IS NULL OR call_quota >= 0),
    features        JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (contract_id, name)
);

CREATE INDEX idx_contract_pricing_plans_contract ON contract_pricing_plans(contract_id);
CREATE INDEX idx_contract_pricing_plans_active   ON contract_pricing_plans(contract_id) WHERE is_active;

-- ── Issued licenses ──────────────────────────────────────────────────
-- Each row is a license issued to a publisher for a specific plan.
-- The signed JWT given to the client carries `jti` so we can revoke
-- it server-side without re-issuing or rotating the signing key.
CREATE TABLE contract_licenses (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    jti           UUID NOT NULL UNIQUE,
    contract_id   UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    plan_id       UUID NOT NULL REFERENCES contract_pricing_plans(id) ON DELETE RESTRICT,
    owner_id      UUID NOT NULL REFERENCES publishers(id) ON DELETE CASCADE,
    issued_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ,
    revoked_at    TIMESTAMPTZ,
    status        VARCHAR(20) NOT NULL DEFAULT 'active'
                   CHECK (status IN ('active', 'revoked', 'expired')),
    metadata      JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_contract_licenses_owner    ON contract_licenses(owner_id);
CREATE INDEX idx_contract_licenses_contract ON contract_licenses(contract_id);
CREATE INDEX idx_contract_licenses_jti      ON contract_licenses(jti);
CREATE INDEX idx_contract_licenses_status   ON contract_licenses(status);

-- ── Usage metering events ────────────────────────────────────────────
-- Append-only ledger of metered usage. Aggregations roll up at query
-- time; we keep the raw events so the billing run (Phase 2/3) can
-- reconcile against on-chain or Stripe-reported counts.
CREATE TABLE contract_usage_events (
    id          BIGSERIAL PRIMARY KEY,
    license_id  UUID NOT NULL REFERENCES contract_licenses(id) ON DELETE CASCADE,
    -- Denormalised so we can aggregate per-contract without a join.
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    ts          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    call_count  INTEGER NOT NULL DEFAULT 1 CHECK (call_count > 0),
    metadata    JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX idx_contract_usage_events_license ON contract_usage_events(license_id, ts DESC);
CREATE INDEX idx_contract_usage_events_contract ON contract_usage_events(contract_id, ts DESC);

-- Touch trigger to keep updated_at honest on pricing plans.
CREATE OR REPLACE FUNCTION marketplace_touch_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at := NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_contract_pricing_plans_touch
    BEFORE UPDATE ON contract_pricing_plans
    FOR EACH ROW EXECUTE FUNCTION marketplace_touch_updated_at();
