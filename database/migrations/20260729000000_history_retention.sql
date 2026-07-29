-- History table retention: archive storage for terminal-state proposal records.
--
-- Terminal proposals accumulate indefinitely and degrade the indexed status /
-- timestamp lookups on the live tables. The retention job moves records past the
-- retention window into these archive tables rather than hard-deleting them.
--
-- Archive tables intentionally store enum-backed columns (status, network,
-- governance_model) as TEXT so archived history stays readable if the live enum
-- definitions change, and carry no foreign keys so archived rows survive deletion
-- of the contracts, publishers, and policies they referenced.

CREATE TABLE deploy_proposals_archive (
    id                    UUID PRIMARY KEY,
    contract_name         VARCHAR(255) NOT NULL,
    contract_id           VARCHAR(56)  NOT NULL,
    wasm_hash             VARCHAR(64)  NOT NULL,
    network               TEXT         NOT NULL,
    description           TEXT,
    policy_id             UUID,
    status                TEXT         NOT NULL,
    expires_at            TIMESTAMPTZ  NOT NULL,
    executed_at           TIMESTAMPTZ,
    proposer              VARCHAR(56)  NOT NULL,
    created_at            TIMESTAMPTZ  NOT NULL,
    updated_at            TIMESTAMPTZ  NOT NULL,
    archived_at           TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_deploy_proposals_archive_archived_at ON deploy_proposals_archive(archived_at);
CREATE INDEX idx_deploy_proposals_archive_contract_id ON deploy_proposals_archive(contract_id);
CREATE INDEX idx_deploy_proposals_archive_status      ON deploy_proposals_archive(status);

-- proposal_signatures cascade-delete from deploy_proposals, so they are archived
-- alongside the parent to avoid losing the approval trail.
CREATE TABLE proposal_signatures_archive (
    id                    UUID PRIMARY KEY,
    proposal_id           UUID         NOT NULL,
    signer_address        VARCHAR(56)  NOT NULL,
    signature_data        TEXT,
    signed_at             TIMESTAMPTZ  NOT NULL,
    archived_at           TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_proposal_signatures_archive_proposal_id ON proposal_signatures_archive(proposal_id);
CREATE INDEX idx_proposal_signatures_archive_archived_at ON proposal_signatures_archive(archived_at);

CREATE TABLE governance_proposals_archive (
    id                    UUID PRIMARY KEY,
    contract_id           UUID,
    title                 VARCHAR(255) NOT NULL,
    description           TEXT         NOT NULL,
    governance_model      TEXT         NOT NULL,
    proposer              UUID,
    status                TEXT         NOT NULL,
    voting_starts_at      TIMESTAMPTZ  NOT NULL,
    voting_ends_at        TIMESTAMPTZ  NOT NULL,
    execution_delay_hours INTEGER,
    quorum_required       INTEGER      NOT NULL,
    approval_threshold    INTEGER      NOT NULL,
    created_at            TIMESTAMPTZ  NOT NULL,
    executed_at           TIMESTAMPTZ,
    archived_at           TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_governance_proposals_archive_archived_at ON governance_proposals_archive(archived_at);
CREATE INDEX idx_governance_proposals_archive_contract_id ON governance_proposals_archive(contract_id);
CREATE INDEX idx_governance_proposals_archive_status      ON governance_proposals_archive(status);

-- governance_votes cascade-delete from governance_proposals, so they are archived
-- alongside the parent to preserve the voting record.
CREATE TABLE governance_votes_archive (
    id                    UUID PRIMARY KEY,
    proposal_id           UUID         NOT NULL,
    voter                 UUID         NOT NULL,
    vote_choice           TEXT         NOT NULL,
    voting_power          BIGINT       NOT NULL,
    delegated_from        UUID,
    created_at            TIMESTAMPTZ  NOT NULL,
    archived_at           TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_governance_votes_archive_proposal_id ON governance_votes_archive(proposal_id);
CREATE INDEX idx_governance_votes_archive_archived_at ON governance_votes_archive(archived_at);

-- Supporting indexes for the purge scan. The retention anchor is the point a
-- record entered its terminal state: deploy_proposals maintains updated_at via
-- trigger, governance_proposals has no updated_at so it falls back to created_at
-- for records that never executed.
CREATE INDEX idx_deploy_proposals_updated_at ON deploy_proposals(updated_at);

CREATE INDEX idx_governance_proposals_terminal_at
    ON governance_proposals ((COALESCE(executed_at, created_at)));
