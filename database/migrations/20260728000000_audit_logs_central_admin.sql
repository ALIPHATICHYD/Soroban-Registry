ALTER TABLE audit_logs
    ADD COLUMN IF NOT EXISTS actor_ip TEXT,
    ADD COLUMN IF NOT EXISTS request_id TEXT;

ALTER TABLE audit_logs
    ADD COLUMN IF NOT EXISTS action_type TEXT GENERATED ALWAYS AS (operation) STORED,
    ADD COLUMN IF NOT EXISTS target_resource_type TEXT GENERATED ALWAYS AS (resource_type) STORED,
    ADD COLUMN IF NOT EXISTS target_resource_id TEXT GENERATED ALWAYS AS (resource_id) STORED;

CREATE INDEX IF NOT EXISTS idx_audit_logs_actor_ip ON audit_logs(actor_ip);
CREATE INDEX IF NOT EXISTS idx_audit_logs_request_id ON audit_logs(request_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action_type ON audit_logs(action_type);
CREATE INDEX IF NOT EXISTS idx_audit_logs_target_resource ON audit_logs(target_resource_type, target_resource_id);

CREATE OR REPLACE FUNCTION audit_logs_immutable()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' AND current_setting('audit_logs.allow_delete', true) = 'on' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'audit_logs is append-only: UPDATE and DELETE are not permitted';
END;
$$;

DROP TRIGGER IF EXISTS trg_audit_logs_immutable ON audit_logs;
CREATE TRIGGER trg_audit_logs_immutable
    BEFORE UPDATE OR DELETE ON audit_logs
    FOR EACH ROW EXECUTE FUNCTION audit_logs_immutable();
