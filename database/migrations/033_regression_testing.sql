-- 010_regression_testing.sql
-- Comprehensive regression testing framework for contract versions
-- Tracks baselines, test results, and regression detection

CREATE TYPE test_status AS ENUM ('pending', 'running', 'passed', 'failed', 'skipped');
CREATE TYPE regression_severity AS ENUM ('none', 'minor', 'major', 'critical');

-- ─────────────────────────────────────────────────────────
-- regression_test_baselines
-- Establishes performance/behavior baselines for major versions
-- ─────────────────────────────────────────────────────────
CREATE TABLE regression_test_baselines (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    test_suite_name VARCHAR(255) NOT NULL,
    function_name VARCHAR(255) NOT NULL,
    
    -- Performance baselines
    baseline_execution_time_ms DECIMAL(15,4) NOT NULL,
    baseline_memory_bytes BIGINT,
    baseline_cpu_instructions BIGINT,
    baseline_storage_reads INTEGER,
    baseline_storage_writes INTEGER,
    
    -- Output snapshot for comparison
    output_snapshot JSONB NOT NULL,
    output_hash VARCHAR(64) NOT NULL,
    
    -- Metadata
    established_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    established_by VARCHAR(255),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    notes TEXT,
    
    UNIQUE(contract_id, version, test_suite_name, function_name)
);

CREATE INDEX idx_regression_baselines_contract ON regression_test_baselines(contract_id);
CREATE INDEX idx_regression_baselines_active ON regression_test_baselines(contract_id, is_active) WHERE is_active = TRUE;
CREATE INDEX idx_regression_baselines_version ON regression_test_baselines(contract_id, version);

-- ─────────────────────────────────────────────────────────
-- regression_test_runs
-- Records each automated test execution
-- ─────────────────────────────────────────────────────────
CREATE TABLE regression_test_runs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    version VARCHAR(50) NOT NULL,
    baseline_id UUID REFERENCES regression_test_baselines(id),
    
    -- Test execution details
    test_suite_name VARCHAR(255) NOT NULL,
    function_name VARCHAR(255) NOT NULL,
    status test_status NOT NULL DEFAULT 'pending',
    
    -- Performance measurements
    execution_time_ms DECIMAL(15,4),
    memory_bytes BIGINT,
    cpu_instructions BIGINT,
    storage_reads INTEGER,
    storage_writes INTEGER,
    
    -- Output comparison
    output_data JSONB,
    output_hash VARCHAR(64),
    output_matches_baseline BOOLEAN,
    
    -- Regression detection
    regression_detected BOOLEAN NOT NULL DEFAULT FALSE,
    regression_severity regression_severity NOT NULL DEFAULT 'none',
    performance_degradation_percent DECIMAL(5,2),
    
    -- Execution metadata
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    duration_seconds INTEGER,
    error_message TEXT,
    triggered_by VARCHAR(50) NOT NULL, -- 'deployment', 'manual', 'scheduled'
    
    -- Environment context
    deployment_id UUID REFERENCES contract_deployments(id),
    test_environment VARCHAR(50) NOT NULL DEFAULT 'testing'
);

CREATE INDEX idx_regression_runs_contract ON regression_test_runs(contract_id);
CREATE INDEX idx_regression_runs_status ON regression_test_runs(status);
CREATE INDEX idx_regression_runs_regression ON regression_test_runs(regression_detected, regression_severity) WHERE regression_detected = TRUE;
CREATE INDEX idx_regression_runs_started ON regression_test_runs(started_at DESC);
CREATE INDEX idx_regression_runs_version ON regression_test_runs(contract_id, version, started_at DESC);

-- ─────────────────────────────────────────────────────────
-- regression_test_suites
-- Defines comprehensive test suites for contracts
-- ─────────────────────────────────────────────────────────
CREATE TABLE regression_test_suites (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    
    -- Test configuration
    test_functions JSONB NOT NULL, -- Array of function names and parameters
    performance_thresholds JSONB, -- Max acceptable degradation percentages
    auto_run_on_deploy BOOLEAN NOT NULL DEFAULT TRUE,
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(255),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    
    UNIQUE(contract_id, name)
);

CREATE INDEX idx_regression_suites_contract ON regression_test_suites(contract_id);
CREATE INDEX idx_regression_suites_active ON regression_test_suites(contract_id, is_active) WHERE is_active = TRUE;

-- ─────────────────────────────────────────────────────────
-- regression_alerts
-- Notifications when regressions are detected
-- ─────────────────────────────────────────────────────────
CREATE TABLE regression_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    test_run_id UUID NOT NULL REFERENCES regression_test_runs(id) ON DELETE CASCADE,
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    
    severity regression_severity NOT NULL,
    alert_type VARCHAR(50) NOT NULL, -- 'performance_degradation', 'output_mismatch', 'test_failure'
    
    message TEXT NOT NULL,
    details JSONB,
    
    -- Alert lifecycle
    triggered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    acknowledged BOOLEAN NOT NULL DEFAULT FALSE,
    acknowledged_at TIMESTAMPTZ,
    acknowledged_by VARCHAR(255),
    resolved BOOLEAN NOT NULL DEFAULT FALSE,
    resolved_at TIMESTAMPTZ,
    resolution_notes TEXT,
    
    -- Notification tracking
    notification_sent BOOLEAN NOT NULL DEFAULT FALSE,
    notification_sent_at TIMESTAMPTZ,
    notification_channels TEXT[] -- ['email', 'slack', 'webhook']
);

CREATE INDEX idx_regression_alerts_contract ON regression_alerts(contract_id);
CREATE INDEX idx_regression_alerts_severity ON regression_alerts(severity, triggered_at DESC);
CREATE INDEX idx_regression_alerts_unresolved ON regression_alerts(resolved, triggered_at DESC) WHERE resolved = FALSE;
CREATE INDEX idx_regression_alerts_test_run ON regression_alerts(test_run_id);

-- ─────────────────────────────────────────────────────────
-- regression_test_statistics
-- Aggregated statistics for monitoring test health
-- ─────────────────────────────────────────────────────────
CREATE TABLE regression_test_statistics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    
    -- Time window
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    
    -- Test execution stats
    total_runs INTEGER NOT NULL DEFAULT 0,
    passed_runs INTEGER NOT NULL DEFAULT 0,
    failed_runs INTEGER NOT NULL DEFAULT 0,
    
    -- Regression detection stats
    regressions_detected INTEGER NOT NULL DEFAULT 0,
    false_positives INTEGER NOT NULL DEFAULT 0,
    true_positives INTEGER NOT NULL DEFAULT 0,
    
    -- Accuracy metrics
    detection_accuracy_percent DECIMAL(5,2),
    false_positive_rate_percent DECIMAL(5,2),
    
    -- Performance trends
    avg_execution_time_ms DECIMAL(15,4),
    avg_degradation_percent DECIMAL(5,2),
    
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE(contract_id, period_start, period_end)
);

CREATE INDEX idx_regression_stats_contract ON regression_test_statistics(contract_id);
CREATE INDEX idx_regression_stats_period ON regression_test_statistics(period_start DESC, period_end DESC);

-- ─────────────────────────────────────────────────────────
-- Trigger: Auto-create alert on regression detection
-- ─────────────────────────────────────────────────────────
CREATE OR REPLACE FUNCTION create_regression_alert()
RETURNS TRIGGER AS $$
DECLARE
    alert_msg TEXT;
    alert_type VARCHAR(50);
    details_json JSONB;
BEGIN
    IF NEW.regression_detected AND NEW.status = 'failed' THEN
        -- Determine alert type and message
        IF NEW.performance_degradation_percent IS NOT NULL AND NEW.performance_degradation_percent > 10 THEN
            alert_type := 'performance_degradation';
            alert_msg := format(
                'Performance regression detected in %s.%s: %.2f%% slower than baseline',
                NEW.test_suite_name, NEW.function_name, NEW.performance_degradation_percent
            );
        ELSIF NEW.output_matches_baseline = FALSE THEN
            alert_type := 'output_mismatch';
            alert_msg := format(
                'Output mismatch detected in %s.%s: results differ from baseline',
                NEW.test_suite_name, NEW.function_name
            );
        ELSE
            alert_type := 'test_failure';
            alert_msg := format(
                'Test failure in %s.%s',
                NEW.test_suite_name, NEW.function_name
            );
        END IF;
        
        -- Build details JSON
        details_json := jsonb_build_object(
            'version', NEW.version,
            'execution_time_ms', NEW.execution_time_ms,
            'performance_degradation_percent', NEW.performance_degradation_percent,
            'output_matches_baseline', NEW.output_matches_baseline,
            'error_message', NEW.error_message
        );
        
        -- Insert alert
        INSERT INTO regression_alerts (
            test_run_id,
            contract_id,
            severity,
            alert_type,
            message,
            details
        ) VALUES (
            NEW.id,
            NEW.contract_id,
            NEW.regression_severity,
            alert_type,
            alert_msg,
            details_json
        );
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_create_regression_alert
    AFTER INSERT OR UPDATE ON regression_test_runs
    FOR EACH ROW
    WHEN (NEW.regression_detected = TRUE AND NEW.status = 'failed')
    EXECUTE FUNCTION create_regression_alert();

-- ─────────────────────────────────────────────────────────
-- Function: Calculate regression statistics
-- ─────────────────────────────────────────────────────────
CREATE OR REPLACE FUNCTION calculate_regression_statistics(
    p_contract_id UUID,
    p_period_start TIMESTAMPTZ,
    p_period_end TIMESTAMPTZ
)
RETURNS void AS $$
DECLARE
    v_total_runs INTEGER;
    v_passed_runs INTEGER;
    v_failed_runs INTEGER;
    v_regressions INTEGER;
    v_false_positives INTEGER;
    v_true_positives INTEGER;
    v_accuracy DECIMAL(5,2);
    v_fpr DECIMAL(5,2);
    v_avg_time DECIMAL(15,4);
    v_avg_degradation DECIMAL(5,2);
BEGIN
    -- Count test runs
    SELECT 
        COUNT(*),
        COUNT(*) FILTER (WHERE status = 'passed'),
        COUNT(*) FILTER (WHERE status = 'failed')
    INTO v_total_runs, v_passed_runs, v_failed_runs
    FROM regression_test_runs
    WHERE contract_id = p_contract_id
        AND started_at >= p_period_start
        AND started_at < p_period_end;
    
    -- Count regressions
    SELECT COUNT(*)
    INTO v_regressions
    FROM regression_test_runs
    WHERE contract_id = p_contract_id
        AND started_at >= p_period_start
        AND started_at < p_period_end
        AND regression_detected = TRUE;
    
    -- Calculate false positives (alerts marked as resolved without code changes)
    SELECT COUNT(*)
    INTO v_false_positives
    FROM regression_alerts ra
    JOIN regression_test_runs rtr ON ra.test_run_id = rtr.id
    WHERE rtr.contract_id = p_contract_id
        AND ra.triggered_at >= p_period_start
        AND ra.triggered_at < p_period_end
        AND ra.resolved = TRUE
        AND ra.resolution_notes ILIKE '%false positive%';
    
    v_true_positives := v_regressions - v_false_positives;
    
    -- Calculate accuracy (true positives / total regressions detected)
    IF v_regressions > 0 THEN
        v_accuracy := (v_true_positives::DECIMAL / v_regressions) * 100;
        v_fpr := (v_false_positives::DECIMAL / v_regressions) * 100;
    ELSE
        v_accuracy := 100.0;
        v_fpr := 0.0;
    END IF;
    
    -- Calculate average metrics
    SELECT 
        AVG(execution_time_ms),
        AVG(performance_degradation_percent)
    INTO v_avg_time, v_avg_degradation
    FROM regression_test_runs
    WHERE contract_id = p_contract_id
        AND started_at >= p_period_start
        AND started_at < p_period_end
        AND status = 'passed';
    
    -- Insert or update statistics
    INSERT INTO regression_test_statistics (
        contract_id,
        period_start,
        period_end,
        total_runs,
        passed_runs,
        failed_runs,
        regressions_detected,
        false_positives,
        true_positives,
        detection_accuracy_percent,
        false_positive_rate_percent,
        avg_execution_time_ms,
        avg_degradation_percent
    ) VALUES (
        p_contract_id,
        p_period_start,
        p_period_end,
        v_total_runs,
        v_passed_runs,
        v_failed_runs,
        v_regressions,
        v_false_positives,
        v_true_positives,
        v_accuracy,
        v_fpr,
        v_avg_time,
        v_avg_degradation
    )
    ON CONFLICT (contract_id, period_start, period_end)
    DO UPDATE SET
        total_runs = EXCLUDED.total_runs,
        passed_runs = EXCLUDED.passed_runs,
        failed_runs = EXCLUDED.failed_runs,
        regressions_detected = EXCLUDED.regressions_detected,
        false_positives = EXCLUDED.false_positives,
        true_positives = EXCLUDED.true_positives,
        detection_accuracy_percent = EXCLUDED.detection_accuracy_percent,
        false_positive_rate_percent = EXCLUDED.false_positive_rate_percent,
        avg_execution_time_ms = EXCLUDED.avg_execution_time_ms,
        avg_degradation_percent = EXCLUDED.avg_degradation_percent,
        calculated_at = NOW();
END;
$$ LANGUAGE plpgsql;
