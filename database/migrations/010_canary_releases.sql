-- contract_deployments is needed here but defined later in 047; create it early
CREATE TYPE deployment_environment AS ENUM ('blue', 'green');
CREATE TYPE deployment_status AS ENUM ('active', 'inactive', 'testing', 'failed');

CREATE TABLE contract_deployments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    environment deployment_environment NOT NULL,
    status deployment_status NOT NULL DEFAULT 'inactive',
    wasm_hash VARCHAR(64) NOT NULL,
    deployed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    activated_at TIMESTAMPTZ,
    health_checks_passed INTEGER DEFAULT 0,
    health_checks_failed INTEGER DEFAULT 0,
    last_health_check_at TIMESTAMPTZ,
    error_message TEXT,
    UNIQUE(contract_id, environment)
);

CREATE INDEX idx_contract_deployments_contract_id ON contract_deployments(contract_id);
CREATE INDEX idx_contract_deployments_status ON contract_deployments(status);
CREATE INDEX idx_contract_deployments_environment ON contract_deployments(environment);
CREATE INDEX idx_contract_deployments_active ON contract_deployments(contract_id, status) WHERE status = 'active';

CREATE TYPE canary_status AS ENUM ('pending', 'active', 'paused', 'completed', 'rolled_back', 'failed');
CREATE TYPE rollout_stage AS ENUM ('stage_1', 'stage_2', 'stage_3', 'stage_4', 'complete');

CREATE TABLE canary_releases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    from_deployment_id UUID REFERENCES contract_deployments(id),
    to_deployment_id UUID NOT NULL REFERENCES contract_deployments(id),
    status canary_status NOT NULL DEFAULT 'pending',
    current_stage rollout_stage NOT NULL DEFAULT 'stage_1',
    current_percentage INTEGER NOT NULL DEFAULT 1 CHECK (current_percentage >= 0 AND current_percentage <= 100),
    target_percentage INTEGER NOT NULL DEFAULT 100 CHECK (target_percentage >= 0 AND target_percentage <= 100),
    error_rate_threshold DECIMAL(5,2) NOT NULL DEFAULT 5.0,
    current_error_rate DECIMAL(5,2) DEFAULT 0.0,
    total_requests INTEGER DEFAULT 0,
    error_count INTEGER DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    created_by VARCHAR(255)
);

CREATE UNIQUE INDEX idx_canary_releases_one_active_per_contract ON canary_releases(contract_id, status) WHERE status IN ('pending', 'active');

CREATE INDEX idx_canary_releases_contract_id ON canary_releases(contract_id);
CREATE INDEX idx_canary_releases_status ON canary_releases(status);
CREATE INDEX idx_canary_releases_active ON canary_releases(contract_id, status) WHERE status = 'active';

CREATE TABLE canary_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    canary_id UUID NOT NULL REFERENCES canary_releases(id) ON DELETE CASCADE,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    requests INTEGER NOT NULL DEFAULT 0,
    errors INTEGER NOT NULL DEFAULT 0,
    error_rate DECIMAL(5,2) NOT NULL DEFAULT 0.0,
    avg_response_time_ms DECIMAL(10,2),
    p95_response_time_ms DECIMAL(10,2),
    p99_response_time_ms DECIMAL(10,2)
);

CREATE INDEX idx_canary_metrics_canary_id ON canary_metrics(canary_id);
CREATE INDEX idx_canary_metrics_timestamp ON canary_metrics(timestamp DESC);

CREATE TABLE canary_user_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    canary_id UUID NOT NULL REFERENCES canary_releases(id) ON DELETE CASCADE,
    user_address VARCHAR(56) NOT NULL,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    notified BOOLEAN NOT NULL DEFAULT FALSE,
    notified_at TIMESTAMPTZ,
    UNIQUE(canary_id, user_address)
);

CREATE INDEX idx_canary_user_assignments_canary_id ON canary_user_assignments(canary_id);
CREATE INDEX idx_canary_user_assignments_user ON canary_user_assignments(user_address);

CREATE TABLE canary_stage_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    canary_id UUID NOT NULL REFERENCES canary_releases(id) ON DELETE CASCADE,
    from_stage rollout_stage NOT NULL,
    to_stage rollout_stage NOT NULL,
    from_percentage INTEGER NOT NULL,
    to_percentage INTEGER NOT NULL,
    transitioned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    transitioned_by VARCHAR(255),
    metrics_at_transition JSONB
);

CREATE INDEX idx_canary_stage_history_canary_id ON canary_stage_history(canary_id);
CREATE INDEX idx_canary_stage_history_transitioned_at ON canary_stage_history(transitioned_at DESC);

CREATE OR REPLACE FUNCTION check_canary_error_rate()
RETURNS TRIGGER AS $$
DECLARE
    canary_record canary_releases%ROWTYPE;
    error_rate DECIMAL(5,2);
BEGIN
    SELECT * INTO canary_record FROM canary_releases WHERE id = NEW.canary_id;
    
    IF canary_record.status = 'active' THEN
        error_rate := (canary_record.error_count::DECIMAL / NULLIF(canary_record.total_requests, 0)) * 100.0;
        
        IF error_rate > canary_record.error_rate_threshold THEN
            UPDATE canary_releases 
            SET status = 'rolled_back', completed_at = NOW()
            WHERE id = NEW.canary_id;
            
            INSERT INTO canary_stage_history (canary_id, from_stage, to_stage, from_percentage, to_percentage, transitioned_by)
            VALUES (canary_record.id, canary_record.current_stage, 'complete', canary_record.current_percentage, 0, 'auto-rollback');
        END IF;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER canary_auto_rollback_trigger
AFTER UPDATE OF error_count, total_requests ON canary_releases
FOR EACH ROW
WHEN (NEW.status = 'active')
EXECUTE FUNCTION check_canary_error_rate();
