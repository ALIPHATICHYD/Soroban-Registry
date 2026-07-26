-- Track dependency vulnerability scan runs (backend dependency scanning feature).
--
-- Builds on 029_dependency_scanning.sql, which created
-- `contract_package_dependencies`, `cve_vulnerabilities` and
-- `contract_scan_results` but left nothing populating or reading them.
-- This table records when each scan ran and by whom, so contract pages can
-- show "last scanned" status and publishers get an auditable re-scan trigger.

CREATE TABLE contract_dependency_scan_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    triggered_by VARCHAR(20) NOT NULL DEFAULT 'manual',
    dependencies_scanned INT NOT NULL DEFAULT 0,
    vulnerabilities_found INT NOT NULL DEFAULT 0,
    scanned_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_contract_dependency_scan_runs_contract_id
    ON contract_dependency_scan_runs(contract_id, scanned_at DESC);
