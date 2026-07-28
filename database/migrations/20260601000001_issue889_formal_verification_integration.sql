-- Issue #889: Formal verification integration for contract validation.
--
-- Builds an integration layer on top of the existing built-in WASM analyzer
-- (formal_verification.rs): pluggable verifier backends (built-in or an external
-- service), configurable properties, per-category optional/mandatory policy,
-- timeout-aware runs with stored results, and a result cache keyed by bytecode.

-- ── Configurable properties to verify ─────────────────────────────────────────
-- formal_verification_properties already exists from migration 032, with an
-- unrelated per-session schema (session_id, invariant, counterexample, ...) that
-- is still used by formal_verification_handlers.rs, AND is an FK target
-- (032 line 26 references it), so it cannot be renamed aside. The original
-- CREATE TABLE IF NOT EXISTS here silently bound to 032's table, which lacks the
-- config-catalog columns this feature (#889, formal_verification_integration.rs)
-- needs — the index on `category` then failed with:
--   column "category" does not exist
-- Augment the existing table with the #889 columns instead so both the
-- per-session results (032) and the property catalog (#889) share it. property_key
-- is added with a partial UNIQUE index so legacy 032 rows (no property_key) coexist.
ALTER TABLE formal_verification_properties ADD COLUMN IF NOT EXISTS property_key TEXT;
ALTER TABLE formal_verification_properties ADD COLUMN IF NOT EXISTS category     TEXT;
ALTER TABLE formal_verification_properties ADD COLUMN IF NOT EXISTS spec         JSONB       NOT NULL DEFAULT '{}';
ALTER TABLE formal_verification_properties ADD COLUMN IF NOT EXISTS enabled      BOOLEAN     NOT NULL DEFAULT TRUE;
ALTER TABLE formal_verification_properties ADD COLUMN IF NOT EXISTS updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE UNIQUE INDEX IF NOT EXISTS idx_fv_properties_property_key
    ON formal_verification_properties (property_key) WHERE property_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_fv_properties_category ON formal_verification_properties (category);

-- ── Optional / mandatory policy by category ───────────────────────────────────
CREATE TABLE IF NOT EXISTS formal_verification_policies (
    category       TEXT        PRIMARY KEY,
    -- 'mandatory' | 'optional' | 'disabled'.
    requirement    TEXT        NOT NULL DEFAULT 'optional',
    -- Minimum overall confidence for a run to count as satisfying the policy.
    min_confidence DOUBLE PRECISION NOT NULL DEFAULT 0.8,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Verification runs ─────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS formal_verification_runs (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id             UUID        NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    version                 TEXT,
    wasm_hash               TEXT        NOT NULL DEFAULT '',
    -- 'builtin' | 'external'.
    backend                 TEXT        NOT NULL,
    -- 'completed' | 'timeout' | 'failed'.
    status                  TEXT        NOT NULL,
    properties_proved       INTEGER     NOT NULL DEFAULT 0,
    properties_violated     INTEGER     NOT NULL DEFAULT 0,
    properties_inconclusive INTEGER     NOT NULL DEFAULT 0,
    overall_confidence      DOUBLE PRECISION NOT NULL DEFAULT 0,
    report                  JSONB       NOT NULL DEFAULT '{}',
    duration_ms             BIGINT      NOT NULL DEFAULT 0,
    cache_hit               BOOLEAN     NOT NULL DEFAULT FALSE,
    error_message           TEXT,
    started_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at            TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_fv_runs_contract  ON formal_verification_runs (contract_id, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_fv_runs_status    ON formal_verification_runs (status);
CREATE INDEX IF NOT EXISTS idx_fv_runs_wasm_hash ON formal_verification_runs (wasm_hash);

-- ── Result cache (keyed by bytecode + backend + property set) ──────────────────
CREATE TABLE IF NOT EXISTS formal_verification_run_cache (
    cache_key               TEXT        PRIMARY KEY,
    wasm_hash               TEXT        NOT NULL,
    backend                 TEXT        NOT NULL,
    status                  TEXT        NOT NULL,
    properties_proved       INTEGER     NOT NULL DEFAULT 0,
    properties_violated     INTEGER     NOT NULL DEFAULT 0,
    properties_inconclusive INTEGER     NOT NULL DEFAULT 0,
    overall_confidence      DOUBLE PRECISION NOT NULL DEFAULT 0,
    report                  JSONB       NOT NULL DEFAULT '{}',
    hits                    BIGINT      NOT NULL DEFAULT 0,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_fv_cache_wasm_hash ON formal_verification_run_cache (wasm_hash);

COMMENT ON TABLE formal_verification_properties IS
    'Configurable properties to verify, optionally scoped by category (issue #889).';
COMMENT ON TABLE formal_verification_policies IS
    'Per-category optional/mandatory formal-verification policy (issue #889).';
COMMENT ON TABLE formal_verification_runs IS
    'Formal verification runs with backend, status (incl. timeout), and stored report (issue #889).';
COMMENT ON TABLE formal_verification_run_cache IS
    'Cache of formal verification results keyed by bytecode + backend + property set (issue #889).';
