-- Migration: Custom Contract Metrics
-- Issue #89: Implement Contract Custom Metrics

-- Metric type enum for contract-emitted custom metrics
CREATE TYPE custom_metric_type AS ENUM ('counter', 'gauge', 'histogram');

-- Append-only metrics log
CREATE TABLE IF NOT EXISTS contract_custom_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id TEXT NOT NULL,
    metric_name TEXT NOT NULL,
    metric_type custom_metric_type NOT NULL,
    value NUMERIC NOT NULL,
    unit TEXT,
    metadata JSONB,
    ledger_sequence BIGINT,
    transaction_hash TEXT,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    network network_type NOT NULL DEFAULT 'testnet',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_custom_metrics_contract_id ON contract_custom_metrics(contract_id);
CREATE INDEX IF NOT EXISTS idx_custom_metrics_metric_name ON contract_custom_metrics(metric_name);
CREATE INDEX IF NOT EXISTS idx_custom_metrics_timestamp ON contract_custom_metrics(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_custom_metrics_contract_metric_time ON contract_custom_metrics(contract_id, metric_name, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_custom_metrics_metadata ON contract_custom_metrics USING GIN (metadata);

-- Hourly aggregates
CREATE TABLE IF NOT EXISTS contract_custom_metrics_hourly (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id TEXT NOT NULL,
    metric_name TEXT NOT NULL,
    metric_type custom_metric_type NOT NULL,
    bucket_start TIMESTAMPTZ NOT NULL,
    bucket_end TIMESTAMPTZ NOT NULL,
    sample_count INTEGER NOT NULL,
    sum_value NUMERIC,
    avg_value NUMERIC,
    min_value NUMERIC,
    max_value NUMERIC,
    p50_value NUMERIC,
    p95_value NUMERIC,
    p99_value NUMERIC,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (contract_id, metric_name, metric_type, bucket_start)
);

CREATE INDEX IF NOT EXISTS idx_custom_metrics_hourly_contract_metric_time
    ON contract_custom_metrics_hourly(contract_id, metric_name, bucket_start DESC);

-- Daily aggregates
CREATE TABLE IF NOT EXISTS contract_custom_metrics_daily (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id TEXT NOT NULL,
    metric_name TEXT NOT NULL,
    metric_type custom_metric_type NOT NULL,
    bucket_start TIMESTAMPTZ NOT NULL,
    bucket_end TIMESTAMPTZ NOT NULL,
    sample_count INTEGER NOT NULL,
    sum_value NUMERIC,
    avg_value NUMERIC,
    min_value NUMERIC,
    max_value NUMERIC,
    p50_value NUMERIC,
    p95_value NUMERIC,
    p99_value NUMERIC,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (contract_id, metric_name, metric_type, bucket_start)
);

CREATE INDEX IF NOT EXISTS idx_custom_metrics_daily_contract_metric_time
    ON contract_custom_metrics_daily(contract_id, metric_name, bucket_start DESC);

COMMENT ON TABLE contract_custom_metrics IS 'Append-only log of contract-emitted custom metrics.';
COMMENT ON COLUMN contract_custom_metrics.metric_name IS 'Metric identifier, e.g. custom_trades_volume';
COMMENT ON COLUMN contract_custom_metrics.metric_type IS 'Metric type: counter, gauge, or histogram.';
