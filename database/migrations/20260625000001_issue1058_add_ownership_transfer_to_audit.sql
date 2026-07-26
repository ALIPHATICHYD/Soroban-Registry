-- Issue #1058
-- Add ownership_transferred to the audit_action_type enum
-- so that ownership transfer events are tracked in contract_audit_log.

DO $$ BEGIN
    ALTER TYPE audit_action_type ADD VALUE 'ownership_transferred';
EXCEPTION
    WHEN duplicate_object THEN NULL;
END $$;
