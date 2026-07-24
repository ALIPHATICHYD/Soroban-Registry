-- ╔══════════════════════════════════════════════════════════════════════════╗
-- ║  ZK Proof Validation System (Issue #624)                                ║
-- ║  Adds tables for storing zero-knowledge proof submissions, circuit       ║
-- ║  definitions, and privacy-preserving analytics aggregates.              ║
-- ╚══════════════════════════════════════════════════════════════════════════╝

-- ── 1. Proof system enum ────────────────────────────────────────────────────
DO $$ BEGIN
    CREATE TYPE zk_proof_system AS ENUM ('groth16', 'plonk', 'stark', 'marlin', 'fflonk');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- ── 2. Circuit language enum ────────────────────────────────────────────────
DO $$ BEGIN
    CREATE TYPE zk_circuit_language AS ENUM ('circom', 'noir', 'leo', 'cairo', 'halo2');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- ── 3. ZK proof status enum ─────────────────────────────────────────────────
DO $$ BEGIN
    CREATE TYPE zk_proof_status AS ENUM ('pending', 'valid', 'invalid', 'error');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- ── 4. Circuit registry ─────────────────────────────────────────────────────
-- Stores compiled circuit definitions that verifiers reference.
CREATE TABLE IF NOT EXISTS zk_circuits (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id         UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    description         TEXT,
    language            zk_circuit_language NOT NULL DEFAULT 'circom',
    proof_system        zk_proof_system NOT NULL DEFAULT 'groth16',
    -- The circuit source (e.g., Circom file content, Noir program)
    circuit_source      TEXT NOT NULL,
    -- SHA-256 hash of the compiled circuit artifact
    circuit_hash        TEXT NOT NULL,
    -- Verification key (base64 or hex depending on proof system)
    verification_key    TEXT NOT NULL,
    -- Public circuit parameters / constraints count
    num_public_inputs   INTEGER NOT NULL DEFAULT 0,
    num_constraints     BIGINT,
    -- Metadata blob (trusted setup info, curve parameters, etc.)
    metadata            JSONB,
    compiled_at         TIMESTAMPTZ,
    created_by          UUID REFERENCES publishers(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_zk_circuits_contract ON zk_circuits(contract_id);
CREATE INDEX IF NOT EXISTS idx_zk_circuits_proof_system ON zk_circuits(proof_system);
CREATE UNIQUE INDEX IF NOT EXISTS idx_zk_circuits_contract_name
    ON zk_circuits(contract_id, name);

-- ── 5. ZK proof submissions ─────────────────────────────────────────────────
-- Records every proof submitted for validation against a circuit.
CREATE TABLE IF NOT EXISTS zk_proof_submissions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    circuit_id      UUID NOT NULL REFERENCES zk_circuits(id) ON DELETE CASCADE,
    contract_id     UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    -- The raw proof encoded as hex/base64
    proof_data      TEXT NOT NULL,
    -- Public inputs supplied by the prover (JSON array)
    public_inputs   JSONB NOT NULL DEFAULT '[]',
    -- Validation result
    status          zk_proof_status NOT NULL DEFAULT 'pending',
    -- Prover address (Stellar / publisher address)
    prover_address  TEXT NOT NULL,
    -- Optional human-readable purpose for the proof
    purpose         TEXT,
    error_message   TEXT,
    -- Milliseconds the verifier took
    verification_ms BIGINT,
    verified_at     TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_zk_proofs_circuit    ON zk_proof_submissions(circuit_id);
CREATE INDEX IF NOT EXISTS idx_zk_proofs_contract   ON zk_proof_submissions(contract_id);
CREATE INDEX IF NOT EXISTS idx_zk_proofs_status     ON zk_proof_submissions(status);
CREATE INDEX IF NOT EXISTS idx_zk_proofs_prover     ON zk_proof_submissions(prover_address);
CREATE INDEX IF NOT EXISTS idx_zk_proofs_created    ON zk_proof_submissions(created_at DESC);

-- ── 6. Privacy-preserving analytics ─────────────────────────────────────────
-- Aggregated, non-attributable analytics so individual prover data is never
-- exposed.  Data is bucketed by hour to prevent timing correlation.
CREATE TABLE IF NOT EXISTS zk_analytics_aggregates (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id     UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    circuit_id      UUID REFERENCES zk_circuits(id) ON DELETE SET NULL,
    -- Truncated to the hour for privacy
    bucket_hour     TIMESTAMPTZ NOT NULL,
    proof_system    zk_proof_system NOT NULL,
    total_proofs    BIGINT NOT NULL DEFAULT 0,
    valid_proofs    BIGINT NOT NULL DEFAULT 0,
    invalid_proofs  BIGINT NOT NULL DEFAULT 0,
    error_proofs    BIGINT NOT NULL DEFAULT 0,
    avg_verify_ms   NUMERIC(12, 2),
    p99_verify_ms   NUMERIC(12, 2),
    UNIQUE (contract_id, circuit_id, bucket_hour, proof_system)
);

CREATE INDEX IF NOT EXISTS idx_zk_analytics_contract
    ON zk_analytics_aggregates(contract_id, bucket_hour DESC);

-- ── 7. Update trigger ───────────────────────────────────────────────────────
CREATE OR REPLACE FUNCTION update_zk_circuits_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS trg_zk_circuits_updated_at ON zk_circuits;
CREATE TRIGGER trg_zk_circuits_updated_at
    BEFORE UPDATE ON zk_circuits
    FOR EACH ROW EXECUTE FUNCTION update_zk_circuits_updated_at();
