CREATE TABLE contract_performance_benchmarks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    contract_version_id UUID REFERENCES contract_versions(id) ON DELETE SET NULL,
    benchmark_name VARCHAR(255) NOT NULL,
    execution_time_ms DECIMAL(15,4) NOT NULL CHECK (execution_time_ms >= 0),
    gas_used BIGINT NOT NULL CHECK (gas_used >= 0),
    sample_size INTEGER NOT NULL DEFAULT 1 CHECK (sample_size > 0),
    source VARCHAR(50) NOT NULL DEFAULT 'manual',
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_contract_perf_benchmarks_contract ON contract_performance_benchmarks(contract_id, recorded_at DESC);
CREATE INDEX idx_contract_perf_benchmarks_version ON contract_performance_benchmarks(contract_version_id, recorded_at DESC);
CREATE INDEX idx_contract_perf_benchmarks_name ON contract_performance_benchmarks(contract_id, benchmark_name, recorded_at DESC);

CREATE OR REPLACE FUNCTION sync_contract_benchmark_metrics()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO performance_metrics (
        contract_id,
        metric_type,
        function_name,
        value,
        metadata,
        timestamp
    ) VALUES (
        NEW.contract_id,
        'execution_time',
        NEW.benchmark_name,
        NEW.execution_time_ms,
        jsonb_build_object(
            'contract_version_id', NEW.contract_version_id,
            'benchmark_name', NEW.benchmark_name,
            'source', NEW.source
        ) || COALESCE(NEW.metadata, '{}'::jsonb),
        NEW.recorded_at
    );

    INSERT INTO performance_metrics (
        contract_id,
        metric_type,
        function_name,
        value,
        metadata,
        timestamp
    ) VALUES (
        NEW.contract_id,
        'gas_consumption',
        NEW.benchmark_name,
        NEW.gas_used,
        jsonb_build_object(
            'contract_version_id', NEW.contract_version_id,
            'benchmark_name', NEW.benchmark_name,
            'source', NEW.source
        ) || COALESCE(NEW.metadata, '{}'::jsonb),
        NEW.recorded_at
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_sync_contract_benchmark_metrics
AFTER INSERT ON contract_performance_benchmarks
FOR EACH ROW
EXECUTE FUNCTION sync_contract_benchmark_metrics();

CREATE OR REPLACE FUNCTION create_contract_performance_regression_alert()
RETURNS TRIGGER AS $$
DECLARE
    previous_row RECORD;
    exec_regression DECIMAL(10,2);
    gas_regression DECIMAL(10,2);
    regression_severity alert_severity;
BEGIN
    SELECT
        b.execution_time_ms,
        b.gas_used,
        cv.version
    INTO previous_row
    FROM contract_performance_benchmarks b
    LEFT JOIN contract_versions cv ON cv.id = b.contract_version_id
    WHERE b.contract_id = NEW.contract_id
      AND b.benchmark_name = NEW.benchmark_name
      AND b.id <> NEW.id
      AND (
        NEW.contract_version_id IS NULL
        OR b.contract_version_id IS DISTINCT FROM NEW.contract_version_id
      )
    ORDER BY b.recorded_at DESC
    LIMIT 1;

    IF previous_row.execution_time_ms IS NULL AND previous_row.gas_used IS NULL THEN
        RETURN NEW;
    END IF;

    IF previous_row.execution_time_ms IS NOT NULL AND previous_row.execution_time_ms > 0 THEN
        exec_regression := ((NEW.execution_time_ms - previous_row.execution_time_ms) / previous_row.execution_time_ms) * 100;
    END IF;

    IF previous_row.gas_used IS NOT NULL AND previous_row.gas_used > 0 THEN
        gas_regression := ((NEW.gas_used - previous_row.gas_used) / previous_row.gas_used::DECIMAL) * 100;
    END IF;

    IF COALESCE(exec_regression, 0) <= 10 AND COALESCE(gas_regression, 0) <= 10 THEN
        RETURN NEW;
    END IF;

    regression_severity := CASE
        WHEN COALESCE(exec_regression, 0) >= 30 OR COALESCE(gas_regression, 0) >= 30 THEN 'critical'
        WHEN COALESCE(exec_regression, 0) >= 20 OR COALESCE(gas_regression, 0) >= 20 THEN 'warning'
        ELSE 'info'
    END;

    INSERT INTO performance_alerts (
        contract_id,
        metric_type,
        threshold_type,
        threshold_value,
        current_value,
        severity,
        message,
        triggered_at
    ) VALUES (
        NEW.contract_id,
        CASE
            WHEN COALESCE(exec_regression, 0) >= COALESCE(gas_regression, 0) THEN 'execution_time'
            ELSE 'gas_consumption'
        END,
        'version_regression',
        GREATEST(COALESCE(previous_row.execution_time_ms, 0), COALESCE(previous_row.gas_used, 0)),
        GREATEST(NEW.execution_time_ms, NEW.gas_used::DECIMAL),
        regression_severity,
        format(
            'Performance regression detected for %s: execution %.2f%%, gas %.2f%% compared to previous version %s',
            NEW.benchmark_name,
            COALESCE(exec_regression, 0),
            COALESCE(gas_regression, 0),
            COALESCE(previous_row.version, 'unknown')
        ),
        NEW.recorded_at
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_contract_performance_regression_alert
AFTER INSERT ON contract_performance_benchmarks
FOR EACH ROW
EXECUTE FUNCTION create_contract_performance_regression_alert();
