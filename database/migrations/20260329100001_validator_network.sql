-- Validator Network Schema

-- Validator Status Enum
DO $$ BEGIN
    CREATE TYPE validator_status AS ENUM ('active', 'inactive', 'slashed');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Task Status Enum
DO $$ BEGIN
    CREATE TYPE verification_task_status AS ENUM ('pending', 'processing', 'completed', 'failed');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Attestation Decision Enum
DO $$ BEGIN
    CREATE TYPE attestation_decision AS ENUM ('valid', 'invalid', 'unknown');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Validators Table
CREATE TABLE IF NOT EXISTS validators (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    stellar_address TEXT NOT NULL UNIQUE,
    name TEXT,
    status validator_status NOT NULL DEFAULT 'active',
    stake_amount NUMERIC(20, 7) NOT NULL DEFAULT 0, -- XLM stake
    reputation_score INT NOT NULL DEFAULT 100,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Verification Tasks Table
CREATE TABLE IF NOT EXISTS verification_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    status verification_task_status NOT NULL DEFAULT 'pending',
    assigned_to UUID REFERENCES validators(id),
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Attestations Table
CREATE TABLE IF NOT EXISTS attestations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id UUID NOT NULL REFERENCES verification_tasks(id) ON DELETE CASCADE,
    validator_id UUID NOT NULL REFERENCES validators(id) ON DELETE CASCADE,
    decision attestation_decision NOT NULL,
    compiled_wasm_hash TEXT,
    error_message TEXT,
    signature TEXT, -- Proof of attestation
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(task_id, validator_id)
);

-- Validator Performance Logs
CREATE TABLE IF NOT EXISTS validator_performance (
    validator_id UUID PRIMARY KEY REFERENCES validators(id) ON DELETE CASCADE,
    total_verifications INT NOT NULL DEFAULT 0,
    successful_verifications INT NOT NULL DEFAULT 0,
    failed_verifications INT NOT NULL DEFAULT 0,
    slashed_count INT NOT NULL DEFAULT 0,
    last_active_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_validators_status ON validators(status);
CREATE INDEX IF NOT EXISTS idx_verification_tasks_status ON verification_tasks(status);
CREATE INDEX IF NOT EXISTS idx_attestations_task_id ON attestations(task_id);
