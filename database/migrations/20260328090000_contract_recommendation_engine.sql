-- Issue #492: contract recommendation engine support tables.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'recommendation_event_type') THEN
        CREATE TYPE recommendation_event_type AS ENUM (
            'impression',
            'click',
            'dismiss',
            'conversion'
        );
    END IF;
END $$;

CREATE TABLE IF NOT EXISTS contract_recommendation_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    recommended_contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    event_type recommendation_event_type NOT NULL DEFAULT 'impression',
    algorithm_key VARCHAR(64) NOT NULL,
    ab_variant VARCHAR(32) NOT NULL DEFAULT 'baseline',
    reason_codes JSONB NOT NULL DEFAULT '[]'::jsonb,
    score DOUBLE PRECISION,
    subject_hash VARCHAR(64),
    context JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_contract_recommendation_events_source_time
    ON contract_recommendation_events(source_contract_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_contract_recommendation_events_recommended_time
    ON contract_recommendation_events(recommended_contract_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_contract_recommendation_events_algorithm_variant
    ON contract_recommendation_events(algorithm_key, ab_variant, created_at DESC);
