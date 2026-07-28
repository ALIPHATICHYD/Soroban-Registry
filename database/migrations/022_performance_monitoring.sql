CREATE TYPE metric_type AS ENUM ('execution_time', 'memory_usage', 'storage_io', 'gas_consumption', 'error_rate');
CREATE TYPE alert_severity AS ENUM ('info', 'warning', 'critical');

CREATE TABLE performance_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    metric_type metric_type NOT NULL,
    function_name VARCHAR(255),
    value DECIMAL(15,4) NOT NULL,
    p50 DECIMAL(15,4),
    p95 DECIMAL(15,4),
    p99 DECIMAL(15,4),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_performance_metrics_contract_id ON performance_metrics(contract_id);
CREATE INDEX idx_performance_metrics_type ON performance_metrics(metric_type);
CREATE INDEX idx_performance_metrics_timestamp ON performance_metrics(timestamp DESC);
CREATE INDEX idx_performance_metrics_contract_timestamp ON performance_metrics(contract_id, timestamp DESC);

CREATE TABLE performance_anomalies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    metric_type metric_type NOT NULL,
    function_name VARCHAR(255),
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    baseline_value DECIMAL(15,4),
    current_value DECIMAL(15,4),
    deviation_percent DECIMAL(5,2),
    severity alert_severity NOT NULL,
    resolved BOOLEAN NOT NULL DEFAULT FALSE,
    resolved_at TIMESTAMPTZ,
    description TEXT
);

CREATE INDEX idx_performance_anomalies_contract_id ON performance_anomalies(contract_id);
CREATE INDEX idx_performance_anomalies_detected_at ON performance_anomalies(detected_at DESC);
CREATE INDEX idx_performance_anomalies_resolved ON performance_anomalies(resolved, detected_at DESC);

CREATE TABLE performance_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    metric_type metric_type NOT NULL,
    threshold_type VARCHAR(50) NOT NULL,
    threshold_value DECIMAL(15,4) NOT NULL,
    current_value DECIMAL(15,4) NOT NULL,
    severity alert_severity NOT NULL,
    triggered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    acknowledged BOOLEAN NOT NULL DEFAULT FALSE,
    acknowledged_at TIMESTAMPTZ,
    acknowledged_by VARCHAR(255),
    resolved BOOLEAN NOT NULL DEFAULT FALSE,
    resolved_at TIMESTAMPTZ,
    message TEXT
);

CREATE INDEX idx_performance_alerts_contract_id ON performance_alerts(contract_id);
CREATE INDEX idx_performance_alerts_triggered_at ON performance_alerts(triggered_at DESC);
CREATE INDEX idx_performance_alerts_resolved ON performance_alerts(resolved, triggered_at DESC);

CREATE TABLE performance_trends (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    function_name VARCHAR(255),
    metric_type metric_type NOT NULL,
    timeframe_start TIMESTAMPTZ NOT NULL,
    timeframe_end TIMESTAMPTZ NOT NULL,
    avg_value DECIMAL(15,4),
    min_value DECIMAL(15,4),
    max_value DECIMAL(15,4),
    p50_value DECIMAL(15,4),
    p95_value DECIMAL(15,4),
    p99_value DECIMAL(15,4),
    sample_count INTEGER NOT NULL DEFAULT 0,
    trend_direction VARCHAR(20),
    change_percent DECIMAL(5,2),
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_performance_trends_contract_id ON performance_trends(contract_id);
CREATE INDEX idx_performance_trends_timeframe ON performance_trends(timeframe_start DESC, timeframe_end DESC);

CREATE TABLE performance_alert_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    metric_type metric_type NOT NULL,
    threshold_type VARCHAR(50) NOT NULL,
    threshold_value DECIMAL(15,4) NOT NULL,
    severity alert_severity NOT NULL DEFAULT 'warning',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id, metric_type, threshold_type)
);

CREATE INDEX idx_performance_alert_configs_contract_id ON performance_alert_configs(contract_id);
CREATE INDEX idx_performance_alert_configs_enabled ON performance_alert_configs(enabled, contract_id);

CREATE OR REPLACE FUNCTION detect_performance_anomaly()
RETURNS TRIGGER AS $$
DECLARE
    baseline_avg DECIMAL(15,4);
    baseline_std DECIMAL(15,4);
    deviation DECIMAL(5,2);
    severity_level alert_severity;
    anomaly_desc TEXT;
BEGIN
    SELECT AVG(value), STDDEV(value)
    INTO baseline_avg, baseline_std
    FROM performance_metrics
    WHERE contract_id = NEW.contract_id
      AND metric_type = NEW.metric_type
      AND (function_name = NEW.function_name OR (function_name IS NULL AND NEW.function_name IS NULL))
      AND timestamp > NOW() - INTERVAL '1 hour'
      AND timestamp < NEW.timestamp - INTERVAL '5 minutes';
    
    IF baseline_avg IS NULL OR baseline_std IS NULL THEN
        RETURN NEW;
    END IF;
    
    IF baseline_std = 0 THEN
        baseline_std := baseline_avg * 0.1;
    END IF;
    
    deviation := ABS((NEW.value - baseline_avg) / baseline_std) * 100.0;
    
    IF deviation > 200 THEN
        severity_level := 'critical';
    ELSIF deviation > 100 THEN
        severity_level := 'warning';
    ELSIF deviation > 50 THEN
        severity_level := 'info';
    ELSE
        RETURN NEW;
    END IF;
    
    anomaly_desc := format('Anomaly detected: %s metric for %s deviated by %.2f%% from baseline',
                          NEW.metric_type, COALESCE(NEW.function_name, 'contract'), deviation);
    
    INSERT INTO performance_anomalies (
        contract_id, metric_type, function_name, baseline_value, current_value,
        deviation_percent, severity, description
    ) VALUES (
        NEW.contract_id, NEW.metric_type, NEW.function_name, baseline_avg, NEW.value,
        deviation, severity_level, anomaly_desc
    );
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER performance_anomaly_detection
AFTER INSERT ON performance_metrics
FOR EACH ROW
EXECUTE FUNCTION detect_performance_anomaly();

CREATE OR REPLACE FUNCTION check_performance_thresholds()
RETURNS TRIGGER AS $$
DECLARE
    alert_config RECORD;
    threshold_met BOOLEAN;
    alert_msg TEXT;
BEGIN
    FOR alert_config IN
        SELECT * FROM performance_alert_configs
        WHERE contract_id = NEW.contract_id
          AND metric_type = NEW.metric_type
          AND enabled = TRUE
    LOOP
        threshold_met := FALSE;
        
        CASE alert_config.threshold_type
            WHEN 'p99_exceeds' THEN
                threshold_met := NEW.p99 IS NOT NULL AND NEW.p99 > alert_config.threshold_value;
            WHEN 'p95_exceeds' THEN
                threshold_met := NEW.p95 IS NOT NULL AND NEW.p95 > alert_config.threshold_value;
            WHEN 'value_exceeds' THEN
                threshold_met := NEW.value > alert_config.threshold_value;
            WHEN 'value_below' THEN
                threshold_met := NEW.value < alert_config.threshold_value;
            ELSE
                threshold_met := FALSE;
        END CASE;
        
        IF threshold_met THEN
            alert_msg := format('%s metric %s threshold: %.2f (current: %.2f)',
                              NEW.metric_type, alert_config.threshold_type,
                              alert_config.threshold_value, NEW.value);
            
            INSERT INTO performance_alerts (
                contract_id, metric_type, threshold_type, threshold_value,
                current_value, severity, message
            ) VALUES (
                NEW.contract_id, NEW.metric_type, alert_config.threshold_type,
                alert_config.threshold_value, NEW.value, alert_config.severity, alert_msg
            )
            ON CONFLICT DO NOTHING;
        END IF;
    END LOOP;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER performance_threshold_check
AFTER INSERT ON performance_metrics
FOR EACH ROW
EXECUTE FUNCTION check_performance_thresholds();
