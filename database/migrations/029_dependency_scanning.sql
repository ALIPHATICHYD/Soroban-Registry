-- Dependency Scanning Tables

CREATE TABLE cve_vulnerabilities (
    cve_id VARCHAR(50) PRIMARY KEY,
    description TEXT,
    severity VARCHAR(20) NOT NULL,
    package_name VARCHAR(255) NOT NULL,
    patched_versions TEXT[] NOT NULL DEFAULT '{}',
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_cve_package_name ON cve_vulnerabilities(package_name);
CREATE INDEX idx_cve_severity ON cve_vulnerabilities(severity);

-- Named contract_package_dependencies to avoid conflict with contract_dependencies (006)
CREATE TABLE contract_package_dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    package_name VARCHAR(255) NOT NULL,
    version VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, package_name)
);

CREATE INDEX idx_contract_package_dependencies_contract_id ON contract_package_dependencies(contract_id);
CREATE INDEX idx_contract_package_dependencies_package_name ON contract_package_dependencies(package_name);

CREATE TABLE contract_scan_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    cve_id VARCHAR(50) NOT NULL REFERENCES cve_vulnerabilities(cve_id) ON DELETE CASCADE,
    package_name VARCHAR(255) NOT NULL,
    current_version VARCHAR(100) NOT NULL,
    recommended_version VARCHAR(100),
    is_false_positive BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, cve_id)
);

CREATE INDEX idx_scan_results_contract_id ON contract_scan_results(contract_id);
CREATE INDEX idx_scan_results_cve_id ON contract_scan_results(cve_id);

-- Trigger to automatically update cve_vulnerabilities updated_at
CREATE TRIGGER update_cve_vulnerabilities_updated_at BEFORE UPDATE ON cve_vulnerabilities
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
