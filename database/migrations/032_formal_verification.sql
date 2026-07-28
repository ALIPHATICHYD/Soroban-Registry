-- Add tables for formal verification runs

-- Use formal_verification_status to avoid conflict with verification_status in 001_initial
CREATE TYPE formal_verification_status AS ENUM ('Proved', 'Violated', 'Unknown', 'Skipped');

CREATE TABLE formal_verification_sessions (
    id UUID PRIMARY KEY,
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    version VARCHAR NOT NULL,
    verifier_version VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE formal_verification_properties (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES formal_verification_sessions(id) ON DELETE CASCADE,
    property_id VARCHAR NOT NULL,
    description TEXT,
    invariant TEXT NOT NULL,
    severity VARCHAR NOT NULL
);

CREATE TABLE formal_verification_results (
    id UUID PRIMARY KEY,
    property_id UUID NOT NULL REFERENCES formal_verification_properties(id) ON DELETE CASCADE UNIQUE,
    status formal_verification_status NOT NULL,
    counterexample TEXT,
    details TEXT
);

CREATE INDEX idx_formal_verification_sessions_contract_id ON formal_verification_sessions(contract_id);
