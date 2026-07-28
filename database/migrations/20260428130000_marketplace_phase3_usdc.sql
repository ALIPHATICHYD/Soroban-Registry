-- Marketplace Phase 3: USDC payments on Stellar.
--
-- Stripe (Phase 2) issues licenses when Stripe webhooks confirm a
-- checkout. USDC payments follow the same pattern but use Stellar as
-- the source of truth: the buyer sends USDC to a platform-controlled
-- receiving address, with the platform-generated memo as the binding
-- between on-chain transaction and DB row. A confirm endpoint (called
-- by the indexer crate, or manually by an operator) closes the loop
-- and issues the license.

CREATE TABLE marketplace_usdc_payments (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id         UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    plan_id             UUID NOT NULL REFERENCES contract_pricing_plans(id) ON DELETE RESTRICT,
    payer_id            UUID NOT NULL REFERENCES publishers(id) ON DELETE CASCADE,
    -- Pricing snapshot at intent-creation time. Stored in cents to
    -- match contract_pricing_plans; conversion to USDC stroops happens
    -- in the application (USDC has 7 decimals on Stellar).
    amount_cents        BIGINT NOT NULL CHECK (amount_cents > 0),
    -- The Stellar receiving address the payer must send to. Stored
    -- with the row so a later env-var rotation doesn't orphan
    -- in-flight intents.
    receiving_address   VARCHAR(56) NOT NULL,
    -- USDC asset issuer on the target network. Stellar identifies a
    -- non-native asset by (code, issuer); the code is fixed to USDC.
    asset_issuer        VARCHAR(56) NOT NULL,
    -- Stellar network ∈ {testnet, public}. We deliberately don't
    -- support futurenet here for v1 — USDC issuers vary.
    network             VARCHAR(20) NOT NULL CHECK (network IN ('testnet', 'public')),
    -- Memo binding on-chain payment to this DB row. Stellar memos
    -- limited to 28 bytes for MEMO_TEXT; we use a 22-char base32
    -- prefix so it fits.
    memo                VARCHAR(28) NOT NULL UNIQUE,
    -- Filled by the confirmation step
    tx_hash             VARCHAR(64) UNIQUE,
    confirmed_amount    BIGINT,           -- amount actually received, in cents
    license_id          UUID REFERENCES contract_licenses(id) ON DELETE SET NULL,
    status              VARCHAR(20) NOT NULL DEFAULT 'pending'
                         CHECK (status IN ('pending', 'confirmed', 'expired', 'failed')),
    expires_at          TIMESTAMPTZ NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at        TIMESTAMPTZ,
    metadata            JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX idx_usdc_payments_payer    ON marketplace_usdc_payments(payer_id);
CREATE INDEX idx_usdc_payments_contract ON marketplace_usdc_payments(contract_id);
CREATE INDEX idx_usdc_payments_status   ON marketplace_usdc_payments(status);
CREATE INDEX idx_usdc_payments_memo     ON marketplace_usdc_payments(memo);
CREATE INDEX idx_usdc_payments_expires  ON marketplace_usdc_payments(expires_at) WHERE status = 'pending';

CREATE TRIGGER trg_usdc_payments_touch
    BEFORE UPDATE ON marketplace_usdc_payments
    FOR EACH ROW EXECUTE FUNCTION marketplace_touch_updated_at();
