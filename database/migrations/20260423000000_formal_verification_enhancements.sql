-- Enhance formal verification tables to support full WASM bytecode analysis,
-- property proofs, vulnerability findings, and proof certificates.

-- ── Extend formal_verification_sessions ─────────────────────────────────────

ALTER TABLE formal_verification_sessions
    ALTER COLUMN version DROP NOT NULL,
    ALTER COLUMN verifier_version DROP NOT NULL;

ALTER TABLE formal_verification_sessions
    ADD COLUMN IF NOT EXISTS status VARCHAR NOT NULL DEFAULT 'pending',
    ADD COLUMN IF NOT EXISTS properties_proved INTEGER,
    ADD COLUMN IF NOT EXISTS properties_violated INTEGER,
    ADD COLUMN IF NOT EXISTS properties_inconclusive INTEGER,
    ADD COLUMN IF NOT EXISTS overall_confidence DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS analysis_duration_ms BIGINT,
    ADD COLUMN IF NOT EXISTS analyzer_version VARCHAR,
    ADD COLUMN IF NOT EXISTS completed_at TIMESTAMPTZ;

-- ── Extend formal_verification_properties ───────────────────────────────────

ALTER TABLE formal_verification_properties
    ADD COLUMN IF NOT EXISTS name VARCHAR,
    ADD COLUMN IF NOT EXISTS status VARCHAR NOT NULL DEFAULT 'Unknown',
    ADD COLUMN IF NOT EXISTS method VARCHAR NOT NULL DEFAULT 'PatternMatching',
    ADD COLUMN IF NOT EXISTS confidence DOUBLE PRECISION NOT NULL DEFAULT 0.5,
    ADD COLUMN IF NOT EXISTS evidence JSONB NOT NULL DEFAULT '[]';

-- ── Vulnerability findings ───────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS formal_verification_findings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES formal_verification_sessions(id) ON DELETE CASCADE,
    finding_id VARCHAR NOT NULL,
    title VARCHAR NOT NULL,
    description TEXT NOT NULL,
    severity VARCHAR NOT NULL,
    category VARCHAR NOT NULL,
    cwe_id VARCHAR,
    affected_functions TEXT[] NOT NULL DEFAULT '{}',
    remediation TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_fv_findings_session_id
    ON formal_verification_findings(session_id);

CREATE INDEX IF NOT EXISTS idx_fv_findings_severity
    ON formal_verification_findings(severity);

-- ── Proof certificates ───────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS formal_verification_certificates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES formal_verification_sessions(id) ON DELETE CASCADE,
    certificate_id UUID NOT NULL UNIQUE,
    properties_proved INTEGER NOT NULL DEFAULT 0,
    properties_violated INTEGER NOT NULL DEFAULT 0,
    properties_inconclusive INTEGER NOT NULL DEFAULT 0,
    overall_confidence DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    summary TEXT NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_fv_certificates_session_id
    ON formal_verification_certificates(session_id);

-- ── Additional indexes on sessions ──────────────────────────────────────────

CREATE INDEX IF NOT EXISTS idx_fv_sessions_status
    ON formal_verification_sessions(status);

CREATE INDEX IF NOT EXISTS idx_fv_sessions_completed_at
    ON formal_verification_sessions(completed_at DESC);

-- ── Update formal_verification_properties to store name and counterexample ──

ALTER TABLE formal_verification_properties
    ADD COLUMN IF NOT EXISTS counterexample TEXT;
