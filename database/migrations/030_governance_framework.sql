-- Governance framework
CREATE TYPE governance_model AS ENUM ('token_weighted', 'quadratic', 'multisig', 'timelock');
-- Use governance_proposal_status to avoid conflict with proposal_status in multisig_deployment
CREATE TYPE governance_proposal_status AS ENUM ('pending', 'active', 'passed', 'rejected', 'executed', 'cancelled');
CREATE TYPE vote_choice AS ENUM ('for', 'against', 'abstain');

-- Governance proposals
CREATE TABLE governance_proposals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    governance_model governance_model NOT NULL,
    proposer UUID NOT NULL REFERENCES publishers(id),
    status governance_proposal_status NOT NULL DEFAULT 'pending',
    voting_starts_at TIMESTAMPTZ NOT NULL,
    voting_ends_at TIMESTAMPTZ NOT NULL,
    execution_delay_hours INTEGER DEFAULT 0,
    quorum_required INTEGER NOT NULL DEFAULT 50,
    approval_threshold INTEGER NOT NULL DEFAULT 50,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    executed_at TIMESTAMPTZ
);

CREATE INDEX idx_governance_proposals_contract_id ON governance_proposals(contract_id);
CREATE INDEX idx_governance_proposals_status ON governance_proposals(status);

-- Votes
CREATE TABLE governance_votes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proposal_id UUID NOT NULL REFERENCES governance_proposals(id) ON DELETE CASCADE,
    voter UUID NOT NULL REFERENCES publishers(id),
    vote_choice vote_choice NOT NULL,
    voting_power BIGINT NOT NULL DEFAULT 1,
    delegated_from UUID REFERENCES publishers(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(proposal_id, voter)
);

CREATE INDEX idx_governance_votes_proposal_id ON governance_votes(proposal_id);
CREATE INDEX idx_governance_votes_voter ON governance_votes(voter);

-- Vote delegation
CREATE TABLE vote_delegations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    delegator UUID NOT NULL REFERENCES publishers(id),
    delegate UUID NOT NULL REFERENCES publishers(id),
    contract_id UUID REFERENCES contracts(id) ON DELETE CASCADE,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    UNIQUE(delegator, contract_id, active)
);

CREATE INDEX idx_vote_delegations_delegator ON vote_delegations(delegator);
CREATE INDEX idx_vote_delegations_delegate ON vote_delegations(delegate);
