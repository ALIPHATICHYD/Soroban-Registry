-- Issue #1094
-- Two-phase ownership transfer with signature-anchored confirmation.
--
-- Hardens the issue #1058 ownership-transfer tables so a contract's publisher can only
-- change after two distinct ed25519 signatures have been verified against the on-chain
-- Stellar accounts of the outgoing and incoming publishers.
--
-- Flow under this migration:
--   phase 1 (initiate) is signed by the current owner; the row is created directly with
--            status = 'pending' and from_confirmation = TRUE (one signature verified).
--   phase 2 (accept / reject) is signed by the recipient; accept moves the row to
--            'completed' and moves contracts.publisher_id in the same transaction.
--
-- Both signed payloads are stored verbatim so any third party can re-verify a completed
-- transfer offline from the row alone. That is the provenance half of the issue: the
-- history table records who acted, and these columns record what they actually signed.

-- ── Signature columns ────────────────────────────────────────────────────────

ALTER TABLE ownership_transfers
    ADD COLUMN IF NOT EXISTS signature_algorithm     VARCHAR(20) NOT NULL DEFAULT 'ed25519',
    -- Client-supplied nonce binding the phase-1 signature. The sender must sign before
    -- the row (and therefore its id) exists, so the nonce is what makes that signature
    -- single-use; see uq_ownership_transfers_request_nonce below.
    ADD COLUMN IF NOT EXISTS request_nonce           VARCHAR(128),
    ADD COLUMN IF NOT EXISTS from_signature          TEXT,
    ADD COLUMN IF NOT EXISTS from_signer_address     VARCHAR(56),
    ADD COLUMN IF NOT EXISTS from_signed_at          TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS from_signed_payload     TEXT,
    ADD COLUMN IF NOT EXISTS decision_nonce          VARCHAR(128),
    ADD COLUMN IF NOT EXISTS decision_signature      TEXT,
    ADD COLUMN IF NOT EXISTS decision_signer_address VARCHAR(56),
    ADD COLUMN IF NOT EXISTS decision_signed_at      TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS decision_signed_payload TEXT,
    ADD COLUMN IF NOT EXISTS decision_by             UUID REFERENCES publishers(id);

COMMENT ON COLUMN ownership_transfers.from_signed_payload IS
    'Exact ASCII payload the outgoing publisher signed. Retained for offline re-verification.';
COMMENT ON COLUMN ownership_transfers.decision_signed_payload IS
    'Exact ASCII payload the recipient signed when accepting or rejecting.';

-- ── Invalidate pre-#1094 in-flight transfers ─────────────────────────────────
--
-- Rows created before this migration carry no signatures and therefore cannot satisfy
-- the new invariants. Expiring them is consistent with the append-only history model:
-- nothing is deleted, the rows simply reach a terminal state and the parties re-initiate.

UPDATE ownership_transfers
SET status       = 'expired',
    completed_at = COALESCE(completed_at, NOW())
WHERE status IN ('pending', 'confirmed');

-- ── Confirmation-state invariants ────────────────────────────────────────────
--
-- Redefined for the new flow: 'pending' now means "the sender has signed", so
-- from_confirmation is TRUE from the moment the row is inserted. Under #1058 'pending'
-- required both flags FALSE, which is what made the partial-confirm UPDATE in
-- handlers.rs violate this very constraint at runtime.

ALTER TABLE ownership_transfers DROP CONSTRAINT IF EXISTS chk_confirmation_logic;

-- NOT VALID: enforced for every insert and every update from here on, but not
-- retro-checked against legacy rows, so this migration cannot fail the API's
-- boot-time migration run on an unknown data set.
ALTER TABLE ownership_transfers
    ADD CONSTRAINT chk_confirmation_logic CHECK (
        (status = 'pending'   AND from_confirmation = TRUE AND to_confirmation = FALSE) OR
        (status = 'confirmed' AND from_confirmation = TRUE AND to_confirmation = TRUE)  OR
        (status = 'completed' AND from_confirmation = TRUE AND to_confirmation = TRUE)  OR
        (status IN ('expired', 'rejected', 'duplicate'))
    ) NOT VALID;

-- Ownership must not be transferable without both signatures on record. Enforcing this
-- in the database as well as the handler means a future code path cannot quietly skip it.
DO $$ BEGIN
    ALTER TABLE ownership_transfers
        ADD CONSTRAINT chk_ownership_transfer_signature_completeness CHECK (
            (
                status = 'pending'
                AND request_nonce IS NOT NULL
                AND from_signature IS NOT NULL
                AND from_signer_address IS NOT NULL
                AND from_signed_at IS NOT NULL
                AND from_signed_payload IS NOT NULL
            ) OR (
                status IN ('confirmed', 'completed', 'rejected')
                AND request_nonce IS NOT NULL
                AND from_signature IS NOT NULL
                AND from_signer_address IS NOT NULL
                AND from_signed_at IS NOT NULL
                AND from_signed_payload IS NOT NULL
                AND decision_nonce IS NOT NULL
                AND decision_signature IS NOT NULL
                AND decision_signer_address IS NOT NULL
                AND decision_signed_at IS NOT NULL
                AND decision_signed_payload IS NOT NULL
                AND decision_by IS NOT NULL
            ) OR (
                -- Expiry is a system action with no counter-signature, and 'duplicate'
                -- is retained only for legacy rows.
                status IN ('expired', 'duplicate')
            )
        ) NOT VALID;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    ALTER TABLE ownership_transfers
        ADD CONSTRAINT chk_ownership_transfers_signature_algorithm
        CHECK (signature_algorithm = 'ed25519') NOT VALID;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

-- NOT VALID for the same reason as the constraints above, and this one is the likeliest to
-- have a non-compliant legacy row: #1058 validated the caller's expiry against the Rust
-- clock, while created_at defaults to the SQL transaction timestamp a moment later, so a
-- transfer created with a near-instant expiry could have expires_at <= created_at. A
-- validated constraint would abort the boot-time migration run over such a row.
DO $$ BEGIN
    ALTER TABLE ownership_transfers
        ADD CONSTRAINT chk_ownership_transfers_expiry_after_creation
        CHECK (expires_at > created_at) NOT VALID;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;

-- ── Single-use signatures, single live transfer ──────────────────────────────

-- A captured create body replays at most once: the second attempt trips this index and
-- the handler maps 23505 to 409 rather than opening a second transfer.
CREATE UNIQUE INDEX IF NOT EXISTS uq_ownership_transfers_request_nonce
    ON ownership_transfers(request_nonce)
    WHERE request_nonce IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS uq_ownership_transfers_decision_nonce
    ON ownership_transfers(decision_nonce)
    WHERE decision_nonce IS NOT NULL;

-- At most one live transfer per contract, enforced by the database rather than by a
-- read-then-insert in the handler, which two concurrent initiations could both pass.
CREATE UNIQUE INDEX IF NOT EXISTS uq_ownership_transfers_one_live_per_contract
    ON ownership_transfers(contract_id)
    WHERE status IN ('pending', 'confirmed');

-- Supports the expiry sweeper's claim query, which only ever scans live rows.
CREATE INDEX IF NOT EXISTS idx_ownership_transfers_due_expiry
    ON ownership_transfers(expires_at)
    WHERE status IN ('pending', 'confirmed');

-- ── History table: allow system-authored rows ────────────────────────────────
--
-- The expiry sweeper has no acting publisher. actor_id was NOT NULL, which forced the
-- #1058 code to invent an actor; the duplicate branch invented a transfer_id too and
-- so tripped the foreign key, turning an intended 409 into a 500.

ALTER TABLE ownership_transfer_logs ALTER COLUMN actor_id DROP NOT NULL;

DO $$ BEGIN
    ALTER TABLE ownership_transfer_logs
        ADD CONSTRAINT chk_ownership_transfer_logs_actor CHECK (
            actor_type = 'system' OR (actor_type = 'publisher' AND actor_id IS NOT NULL)
        ) NOT VALID;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;
