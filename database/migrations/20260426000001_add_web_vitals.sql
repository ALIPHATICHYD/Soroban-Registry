CREATE TABLE web_vitals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metric_id VARCHAR(50) NOT NULL,
    name VARCHAR(50) NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    rating VARCHAR(20),
    delta DOUBLE PRECISION,
    navigation_type VARCHAR(50),
    url TEXT,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_web_vitals_name ON web_vitals(name);
CREATE INDEX idx_web_vitals_created_at ON web_vitals(created_at);
