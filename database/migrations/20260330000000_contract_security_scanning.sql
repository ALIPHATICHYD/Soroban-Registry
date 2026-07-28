-- Migration: 20260330000000_contract_security_scanning
-- Features: #498 Build Automated Contract Security Scanning Integration

BEGIN;

-- Create enum for security scan status
CREATE TYPE scan_status_type AS ENUM ('pending', 'running', 'completed', 'failed');

-- Create enum for security issue severity
CREATE TYPE issue_severity_type AS ENUM ('low', 'medium', 'high', 'critical');

-- Create enum for security issue status
CREATE TYPE issue_status_type AS ENUM ('open', 'acknowledged', 'resolved', 'false_positive');

-- ═══════════════════════════════════════════════════════════════════════════
-- Security Scanners Configuration
-- ═══════════════════════════════════════════════════════════════════════════

-- Table to track configured security scanners/tools
CREATE TABLE security_scanners (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL UNIQUE,
    description TEXT,
    scanner_type VARCHAR(100) NOT NULL, -- e.g., 'static_analysis', 'formal_verification', 'dependency_check'
    api_endpoint VARCHAR(500),
    api_key_encrypted BYTEA, -- Encrypted API key if needed
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    configuration JSONB DEFAULT '{}',
    timeout_seconds INTEGER NOT NULL DEFAULT 300,
    max_concurrent_scans INTEGER NOT NULL DEFAULT 5,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_security_scanners_active ON security_scanners(is_active);

-- ═══════════════════════════════════════════════════════════════════════════
-- Security Scan Results
-- ═══════════════════════════════════════════════════════════════════════════

-- Main table for security scan results
CREATE TABLE security_scans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    contract_version_id UUID REFERENCES contract_versions(id) ON DELETE SET NULL,
    scanner_id UUID REFERENCES security_scanners(id) ON DELETE SET NULL,
    status scan_status_type NOT NULL DEFAULT 'pending',
    scan_type VARCHAR(100) NOT NULL DEFAULT 'full', -- 'full', 'quick', 'incremental'
    triggered_by UUID REFERENCES auth_users(id) ON DELETE SET NULL, -- Who triggered the scan
    triggered_by_event VARCHAR(100), -- e.g., 'upload', 'manual', 'scheduled', 'version_create'
    
    -- Scan results summary
    total_issues INTEGER NOT NULL DEFAULT 0,
    critical_issues INTEGER NOT NULL DEFAULT 0,
    high_issues INTEGER NOT NULL DEFAULT 0,
    medium_issues INTEGER NOT NULL DEFAULT 0,
    low_issues INTEGER NOT NULL DEFAULT 0,
    
    -- Scan metadata
    scan_duration_ms INTEGER, -- Duration in milliseconds
    scanner_version VARCHAR(100), -- Version of the scanner used
    scan_parameters JSONB, -- Parameters passed to the scanner
    scan_result_raw JSONB, -- Raw result from the scanner
    
    -- Timing
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_security_scans_contract ON security_scans(contract_id);
CREATE INDEX idx_security_scans_version ON security_scans(contract_version_id);
CREATE INDEX idx_security_scans_status ON security_scans(status);
CREATE INDEX idx_security_scans_created ON security_scans(created_at);
CREATE INDEX idx_security_scans_triggered_by ON security_scans(triggered_by);

-- ═══════════════════════════════════════════════════════════════════════════
-- Security Issues
-- ═══════════════════════════════════════════════════════════════════════════

-- Individual security issues found during scans
CREATE TABLE security_issues (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    scan_id UUID NOT NULL REFERENCES security_scans(id) ON DELETE CASCADE,
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    contract_version_id UUID REFERENCES contract_versions(id) ON DELETE SET NULL,
    
    -- Issue details
    title VARCHAR(500) NOT NULL,
    description TEXT NOT NULL,
    severity issue_severity_type NOT NULL,
    status issue_status_type NOT NULL DEFAULT 'open',
    category VARCHAR(200), -- e.g., 'reentrancy', 'overflow', 'access_control', 'logic_error'
    cwe_id VARCHAR(50), -- Common Weakness Enumeration ID
    cve_id VARCHAR(50), -- Common Vulnerabilities and Exposures ID (if applicable)
    
    -- Location information
    source_file VARCHAR(500),
    source_line_start INTEGER,
    source_line_end INTEGER,
    function_name VARCHAR(255),
    code_snippet TEXT,
    
    -- Remediation
    remediation TEXT,
    remediation_code_example TEXT,
    reference_urls TEXT[], -- Array of reference URLs
    
    -- Tracking
    external_issue_id VARCHAR(255), -- ID from external scanner
    is_false_positive BOOLEAN NOT NULL DEFAULT FALSE,
    false_positive_reason TEXT,
    resolved_by UUID REFERENCES auth_users(id) ON DELETE SET NULL,
    resolved_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_security_issues_scan ON security_issues(scan_id);
CREATE INDEX idx_security_issues_contract ON security_issues(contract_id);
CREATE INDEX idx_security_issues_version ON security_issues(contract_version_id);
CREATE INDEX idx_security_issues_severity ON security_issues(severity);
CREATE INDEX idx_security_issues_status ON security_issues(status);
CREATE INDEX idx_security_issues_category ON security_issues(category);
CREATE INDEX idx_security_issues_created ON security_issues(created_at);

-- ═══════════════════════════════════════════════════════════════════════════
-- Security Scan History & Version Tracking
-- ═══════════════════════════════════════════════════════════════════════════

-- Track security score evolution across versions
CREATE TABLE security_score_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    contract_version_id UUID NOT NULL REFERENCES contract_versions(id) ON DELETE CASCADE,
    
    -- Security score (0-100)
    overall_score INTEGER NOT NULL CHECK (overall_score >= 0 AND overall_score <= 100),
    score_breakdown JSONB, -- Breakdown by category
    
    -- Issue counts at time of scan
    critical_count INTEGER NOT NULL DEFAULT 0,
    high_count INTEGER NOT NULL DEFAULT 0,
    medium_count INTEGER NOT NULL DEFAULT 0,
    low_count INTEGER NOT NULL DEFAULT 0,
    
    -- Scan reference
    scan_id UUID REFERENCES security_scans(id) ON DELETE SET NULL,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_security_score_history_contract ON security_score_history(contract_id);
CREATE INDEX idx_security_score_history_version ON security_score_history(contract_version_id);
CREATE INDEX idx_security_score_history_created ON security_score_history(created_at);

-- ═══════════════════════════════════════════════════════════════════════════
-- Issue Resolution Tracking
-- ═══════════════════════════════════════════════════════════════════════════

-- Track actions taken on security issues
CREATE TABLE security_issue_actions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    issue_id UUID NOT NULL REFERENCES security_issues(id) ON DELETE CASCADE,
    action_type VARCHAR(100) NOT NULL, -- 'acknowledged', 'resolved', 'marked_false_positive', 'reopened', 'comment_added'
    performed_by UUID REFERENCES auth_users(id) ON DELETE SET NULL,
    notes TEXT,
    previous_status issue_status_type,
    new_status issue_status_type,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_security_issue_actions_issue ON security_issue_actions(issue_id);
CREATE INDEX idx_security_issue_actions_performed_by ON security_issue_actions(performed_by);

-- ═══════════════════════════════════════════════════════════════════════════
-- Scheduled Scans Configuration
-- ═══════════════════════════════════════════════════════════════════════════

-- Configure automatic security scans
CREATE TABLE security_scan_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID REFERENCES contracts(id) ON DELETE CASCADE,
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    schedule_type VARCHAR(50) NOT NULL, -- 'daily', 'weekly', 'monthly', 'on_version'
    scanner_ids UUID[] NOT NULL, -- Which scanners to use
    scan_type VARCHAR(100) NOT NULL DEFAULT 'full',
    notification_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_run_at TIMESTAMPTZ,
    next_run_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_security_scan_schedules_contract ON security_scan_schedules(contract_id);
CREATE INDEX idx_security_scan_schedules_organization ON security_scan_schedules(organization_id);
CREATE INDEX idx_security_scan_schedules_active ON security_scan_schedules(is_active);

-- ═══════════════════════════════════════════════════════════════════════════
-- Triggers
-- ═══════════════════════════════════════════════════════════════════════════

-- Trigger to update updated_at timestamps
CREATE TRIGGER update_security_scanners_updated_at
    BEFORE UPDATE ON security_scanners
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_security_scans_updated_at
    BEFORE UPDATE ON security_scans
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_security_issues_updated_at
    BEFORE UPDATE ON security_issues
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_security_scan_schedules_updated_at
    BEFORE UPDATE ON security_scan_schedules
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ═══════════════════════════════════════════════════════════════════════════
-- Comments for Documentation
-- ═══════════════════════════════════════════════════════════════════════════

COMMENT ON TABLE security_scanners IS 'Configured security scanning tools and integrations (#498)';
COMMENT ON TABLE security_scans IS 'Security scan results for contracts (#498)';
COMMENT ON TABLE security_issues IS 'Individual security issues found during scans (#498)';
COMMENT ON TABLE security_score_history IS 'Historical security scores across contract versions (#498)';
COMMENT ON TABLE security_issue_actions IS 'Audit trail of actions taken on security issues (#498)';
COMMENT ON TABLE security_scan_schedules IS 'Automated security scan scheduling configuration (#498)';

COMMENT ON COLUMN security_scans.triggered_by_event IS 'Event that triggered the scan: upload, manual, scheduled, version_create (#498)';
COMMENT ON COLUMN security_scans.scan_result_raw IS 'Raw JSON result from the security scanner (#498)';
COMMENT ON COLUMN security_issues.cwe_id IS 'Common Weakness Enumeration identifier (#498)';
COMMENT ON COLUMN security_issues.cve_id IS 'Common Vulnerabilities and Exposures identifier (#498)';
COMMENT ON COLUMN security_score_history.overall_score IS 'Security score from 0-100, higher is better (#498)';

COMMIT;
