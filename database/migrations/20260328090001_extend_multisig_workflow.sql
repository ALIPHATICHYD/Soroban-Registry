-- Extend multisig deployment workflow with approvals, ordering, queueing, audit trail,
-- and notification tracking.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'approval_decision_type') THEN
        CREATE TYPE approval_decision_type AS ENUM ('approved', 'rejected');
    END IF;
END$$;

ALTER TABLE multisig_policies
    ADD COLUMN IF NOT EXISTS ordered_approvals BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE deploy_proposals
    ADD COLUMN IF NOT EXISTS required_approvals INT,
    ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS rejected_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS rejection_reason TEXT;

UPDATE deploy_proposals p
SET required_approvals = mp.threshold
FROM multisig_policies mp
WHERE p.policy_id = mp.id
  AND p.required_approvals IS NULL;

ALTER TABLE deploy_proposals
    ALTER COLUMN required_approvals SET NOT NULL;

ALTER TABLE proposal_signatures
    ADD COLUMN IF NOT EXISTS decision approval_decision_type NOT NULL DEFAULT 'approved',
    ADD COLUMN IF NOT EXISTS comment TEXT,
    ADD COLUMN IF NOT EXISTS step_index INT,
    ADD COLUMN IF NOT EXISTS reviewed_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

CREATE TABLE IF NOT EXISTS multisig_approval_audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proposal_id UUID NOT NULL REFERENCES deploy_proposals(id) ON DELETE CASCADE,
    actor_address VARCHAR(56),
    action VARCHAR(64) NOT NULL,
    decision approval_decision_type,
    comment TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_multisig_audit_events_proposal
    ON multisig_approval_audit_events(proposal_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_multisig_audit_events_action
    ON multisig_approval_audit_events(action);

CREATE TABLE IF NOT EXISTS multisig_approval_notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    proposal_id UUID NOT NULL REFERENCES deploy_proposals(id) ON DELETE CASCADE,
    signer_address VARCHAR(56) NOT NULL,
    notification_type VARCHAR(64) NOT NULL,
    payload JSONB,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sent_at TIMESTAMPTZ,
    acknowledged_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_multisig_notifications_proposal
    ON multisig_approval_notifications(proposal_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_multisig_notifications_signer_status
    ON multisig_approval_notifications(signer_address, status, created_at DESC);
