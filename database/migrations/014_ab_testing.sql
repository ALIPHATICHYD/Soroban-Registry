CREATE TYPE ab_test_status AS ENUM ('draft', 'running', 'paused', 'completed', 'cancelled');
CREATE TYPE variant_type AS ENUM ('control', 'treatment');

CREATE TABLE ab_tests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    status ab_test_status NOT NULL DEFAULT 'draft',
    traffic_split DECIMAL(5,2) NOT NULL DEFAULT 50.0 CHECK (traffic_split >= 0 AND traffic_split <= 100),
    variant_a_deployment_id UUID NOT NULL REFERENCES contract_deployments(id),
    variant_b_deployment_id UUID NOT NULL REFERENCES contract_deployments(id),
    primary_metric VARCHAR(100) NOT NULL,
    hypothesis TEXT,
    significance_threshold DECIMAL(5,2) NOT NULL DEFAULT 95.0,
    min_sample_size INTEGER NOT NULL DEFAULT 1000,
    started_at TIMESTAMPTZ,
    ended_at TIMESTAMPTZ,
    created_by VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_ab_tests_one_running_per_contract ON ab_tests(contract_id, status) WHERE status = 'running';

CREATE INDEX idx_ab_tests_contract_id ON ab_tests(contract_id);
CREATE INDEX idx_ab_tests_status ON ab_tests(status);
CREATE INDEX idx_ab_tests_running ON ab_tests(contract_id, status) WHERE status = 'running';

CREATE TABLE ab_test_variants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    test_id UUID NOT NULL REFERENCES ab_tests(id) ON DELETE CASCADE,
    variant_type variant_type NOT NULL,
    deployment_id UUID NOT NULL REFERENCES contract_deployments(id),
    traffic_percentage DECIMAL(5,2) NOT NULL DEFAULT 50.0,
    UNIQUE(test_id, variant_type)
);

CREATE INDEX idx_ab_test_variants_test_id ON ab_test_variants(test_id);

CREATE TABLE ab_test_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    test_id UUID NOT NULL REFERENCES ab_tests(id) ON DELETE CASCADE,
    user_address VARCHAR(56) NOT NULL,
    variant_type variant_type NOT NULL,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(test_id, user_address)
);

CREATE INDEX idx_ab_test_assignments_test_id ON ab_test_assignments(test_id);
CREATE INDEX idx_ab_test_assignments_user ON ab_test_assignments(user_address);

CREATE TABLE ab_test_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    test_id UUID NOT NULL REFERENCES ab_tests(id) ON DELETE CASCADE,
    variant_type variant_type NOT NULL,
    metric_name VARCHAR(100) NOT NULL,
    metric_value DECIMAL(15,4) NOT NULL,
    user_address VARCHAR(56),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_ab_test_metrics_test_id ON ab_test_metrics(test_id);
CREATE INDEX idx_ab_test_metrics_variant ON ab_test_metrics(test_id, variant_type);
CREATE INDEX idx_ab_test_metrics_timestamp ON ab_test_metrics(timestamp DESC);

CREATE TABLE ab_test_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    test_id UUID NOT NULL REFERENCES ab_tests(id) ON DELETE CASCADE,
    variant_type variant_type NOT NULL,
    sample_size INTEGER NOT NULL DEFAULT 0,
    mean_value DECIMAL(15,4),
    std_deviation DECIMAL(15,4),
    confidence_interval_lower DECIMAL(15,4),
    confidence_interval_upper DECIMAL(15,4),
    p_value DECIMAL(10,6),
    statistical_significance DECIMAL(5,2),
    is_winner BOOLEAN DEFAULT FALSE,
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(test_id, variant_type)
);

CREATE INDEX idx_ab_test_results_test_id ON ab_test_results(test_id);

CREATE OR REPLACE FUNCTION assign_variant(test_uuid UUID, user_addr VARCHAR(56))
RETURNS variant_type AS $$
DECLARE
    test_record ab_tests%ROWTYPE;
    existing_assignment variant_type;
    hash_value INTEGER;
    split_percentage DECIMAL(5,2);
BEGIN
    SELECT * INTO test_record FROM ab_tests WHERE id = test_uuid AND status = 'running';
    
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;
    
    SELECT variant_type INTO existing_assignment 
    FROM ab_test_assignments 
    WHERE test_id = test_uuid AND user_address = user_addr;
    
    IF existing_assignment IS NOT NULL THEN
        RETURN existing_assignment;
    END IF;
    
    hash_value := abs(hashtext(test_uuid::text || user_addr)) % 100;
    split_percentage := test_record.traffic_split;
    
    IF hash_value < split_percentage THEN
        INSERT INTO ab_test_assignments (test_id, user_address, variant_type)
        VALUES (test_uuid, user_addr, 'control')
        ON CONFLICT (test_id, user_address) DO NOTHING;
        RETURN 'control';
    ELSE
        INSERT INTO ab_test_assignments (test_id, user_address, variant_type)
        VALUES (test_uuid, user_addr, 'treatment')
        ON CONFLICT (test_id, user_address) DO NOTHING;
        RETURN 'treatment';
    END IF;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION calculate_statistical_significance(test_uuid UUID)
RETURNS TABLE (
    variant variant_type,
    sample_size INTEGER,
    mean_val DECIMAL(15,4),
    std_dev DECIMAL(15,4),
    p_value DECIMAL(10,6),
    significance DECIMAL(5,2)
) AS $$
DECLARE
    control_mean DECIMAL(15,4);
    treatment_mean DECIMAL(15,4);
    control_std DECIMAL(15,4);
    treatment_std DECIMAL(15,4);
    control_n INTEGER;
    treatment_n INTEGER;
    pooled_std DECIMAL(15,4);
    t_statistic DECIMAL(15,4);
    degrees_of_freedom INTEGER;
    p_val DECIMAL(10,6);
    sig_level DECIMAL(5,2);
BEGIN
    SELECT 
        COUNT(*),
        AVG(metric_value),
        STDDEV(metric_value)
    INTO control_n, control_mean, control_std
    FROM ab_test_metrics
    WHERE test_id = test_uuid AND variant_type = 'control';
    
    SELECT 
        COUNT(*),
        AVG(metric_value),
        STDDEV(metric_value)
    INTO treatment_n, treatment_mean, treatment_std
    FROM ab_test_metrics
    WHERE test_id = test_uuid AND variant_type = 'treatment';
    
    IF control_n < 30 OR treatment_n < 30 THEN
        RETURN QUERY SELECT 
            'control'::variant_type,
            control_n,
            control_mean,
            control_std,
            1.0::DECIMAL(10,6) as p_value,
            0.0::DECIMAL(5,2) as significance;
        RETURN QUERY SELECT 
            'treatment'::variant_type,
            treatment_n,
            treatment_mean,
            treatment_std,
            1.0::DECIMAL(10,6) as p_value,
            0.0::DECIMAL(5,2) as significance;
        RETURN;
    END IF;
    
    pooled_std := SQRT(((control_n - 1) * POWER(control_std, 2) + (treatment_n - 1) * POWER(treatment_std, 2)) / (control_n + treatment_n - 2));
    t_statistic := (treatment_mean - control_mean) / (pooled_std * SQRT(1.0/control_n + 1.0/treatment_n));
    degrees_of_freedom := control_n + treatment_n - 2;
    
    p_val := 2 * (1 - normal_cdf(ABS(t_statistic)));
    sig_level := CASE 
        WHEN p_val < 0.01 THEN 99.0
        WHEN p_val < 0.05 THEN 95.0
        WHEN p_val < 0.10 THEN 90.0
        ELSE 0.0
    END;
    
    RETURN QUERY SELECT 
        'control'::variant_type,
        control_n,
        control_mean,
        control_std,
        p_val,
        sig_level;
    RETURN QUERY SELECT 
        'treatment'::variant_type,
        treatment_n,
        treatment_mean,
        treatment_std,
        p_val,
        sig_level;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION normal_cdf(x DECIMAL)
RETURNS DECIMAL AS $$
BEGIN
    RETURN 0.5 * (1 + erf(x / SQRT(2.0)));
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION erf(x DECIMAL)
RETURNS DECIMAL AS $$
DECLARE
    a1 DECIMAL := 0.254829592;
    a2 DECIMAL := -0.284496736;
    a3 DECIMAL := 1.421413741;
    a4 DECIMAL := -1.453152027;
    a5 DECIMAL := 1.061405429;
    p DECIMAL := 0.3275911;
    sign DECIMAL;
    t DECIMAL;
    y DECIMAL;
BEGIN
    sign := SIGN(x);
    x := ABS(x);
    t := 1.0 / (1.0 + p * x);
    y := 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * EXP(-x * x);
    RETURN sign * y;
END;
$$ LANGUAGE plpgsql;
