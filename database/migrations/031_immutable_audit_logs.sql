-- 009_immutable_audit_logs.sql
-- Add cryptographic hash-chaining and immutability guarantees to contract_audit_log

-- 1. Add new columns for hash-chain and signatures
ALTER TABLE contract_audit_log
    ADD COLUMN previous_hash VARCHAR(64),
    ADD COLUMN hash VARCHAR(64),
    ADD COLUMN signature VARCHAR(128);

-- 2. Create the append-only trigger function to guarantee immutability
CREATE OR REPLACE FUNCTION enforce_append_only_audit_log()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'Updates and deletions are strictly prohibited on contract_audit_log to ensure immutability.';
END;
$$ LANGUAGE plpgsql;

-- 3. Attach the trigger for UPDATE and DELETE operations
CREATE TRIGGER prevent_audit_log_modification
BEFORE UPDATE OR DELETE ON contract_audit_log
FOR EACH ROW
EXECUTE FUNCTION enforce_append_only_audit_log();
