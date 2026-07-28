BEGIN;

ALTER TABLE contracts
    ADD COLUMN artifact_scan_status VARCHAR(32) NOT NULL DEFAULT 'pending'
        CHECK (artifact_scan_status IN ('pending', 'passed', 'quarantined')),
    ADD COLUMN artifact_scan_findings JSONB NOT NULL DEFAULT '[]'::jsonb;

CREATE INDEX idx_contracts_artifact_scan_status ON contracts(artifact_scan_status);

COMMIT;
