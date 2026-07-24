-- Multi-Signature Contract Deployment
-- Supports M-of-N approval flows before a contract is deployed

-- Status lifecycle: pending -> approved -> executed
--                                       -> expired
--                                       -> rejected
CREATE TYPE proposal_status AS ENUM ('pending', 'approved', 'executed', 'expired', 'rejected');

-- ─────────────────────────────────────────────────────────────────────────────
-- multisig_policies
-- Defines who can sign and how many signatures are required
-- ─────────────────────────────────────────────────────────────────────────────
CREATE TABLE multisig_policies (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(255) NOT NULL,
    -- Number of signatures required to approve (M-of-N)
    threshold       INT NOT NULL CHECK (threshold >= 1),
    -- Ordered list of Stellar addresses authorised to sign
    signer_addresses TEXT[] NOT NULL CHECK (array_length(signer_addresses, 1) >= threshold),
    -- How long (in seconds) a proposal using this policy remains valid
    expiry_seconds  INT NOT NULL DEFAULT 86400 CHECK (expiry_seconds >= 60),
    created_by      VARCHAR(56) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_multisig_policies_created_by ON multisig_policies(created_by);

-- ─────────────────────────────────────────────────────────────────────────────
-- deploy_proposals
-- A pending deployment request waiting to collect signatures
-- ─────────────────────────────────────────────────────────────────────────────
CREATE TABLE deploy_proposals (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Human-readable name/label for the contract being proposed
    contract_name   VARCHAR(255) NOT NULL,
    -- On-chain contract ID (may be new/upcoming address)
    contract_id     VARCHAR(56)  NOT NULL,
    -- WASM hash of the binary to be deployed
    wasm_hash       VARCHAR(64)  NOT NULL,
    network         network_type NOT NULL,
    description     TEXT,
    policy_id       UUID         NOT NULL REFERENCES multisig_policies(id) ON DELETE RESTRICT,
    status          proposal_status NOT NULL DEFAULT 'pending',
    -- Computed at creation time: NOW() + policy.expiry_seconds
    expires_at      TIMESTAMPTZ  NOT NULL,
    -- Set when status transitions to 'executed'
    executed_at     TIMESTAMPTZ,
    -- The Stellar address of whoever created the proposal
    proposer        VARCHAR(56)  NOT NULL,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_deploy_proposals_policy_id   ON deploy_proposals(policy_id);
CREATE INDEX idx_deploy_proposals_status       ON deploy_proposals(status);
CREATE INDEX idx_deploy_proposals_contract_id  ON deploy_proposals(contract_id);
CREATE INDEX idx_deploy_proposals_expires_at   ON deploy_proposals(expires_at);

-- Auto-update updated_at
CREATE TRIGGER update_deploy_proposals_updated_at
    BEFORE UPDATE ON deploy_proposals
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ─────────────────────────────────────────────────────────────────────────────
-- proposal_signatures
-- Tracks each individual approval collected for a proposal
-- ─────────────────────────────────────────────────────────────────────────────
CREATE TABLE proposal_signatures (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proposal_id     UUID         NOT NULL REFERENCES deploy_proposals(id) ON DELETE CASCADE,
    signer_address  VARCHAR(56)  NOT NULL,
    -- Optional: the actual cryptographic signature payload for off-chain verification
    signature_data  TEXT,
    signed_at       TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    -- Each address may only sign a proposal once
    UNIQUE (proposal_id, signer_address)
);

CREATE INDEX idx_proposal_signatures_proposal_id ON proposal_signatures(proposal_id);
CREATE INDEX idx_proposal_signatures_signer      ON proposal_signatures(signer_address);
