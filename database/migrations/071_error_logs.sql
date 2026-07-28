-- Migration: 071_error_logs.sql
-- Purpose: Store sanitized frontend/backend error reports for operational debugging.
-- Issue: #767

CREATE TABLE IF NOT EXISTS error_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source VARCHAR(32) NOT NULL,
    category VARCHAR(32) NOT NULL,
    severity VARCHAR(16) NOT NULL,
    message TEXT NOT NULL,
    stack_trace TEXT,
    route TEXT,
    request_id TEXT,
    user_agent TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_error_logs_created_at
    ON error_logs(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_error_logs_severity_created
    ON error_logs(severity, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_error_logs_category_created
    ON error_logs(category, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_error_logs_request_id
    ON error_logs(request_id)
    WHERE request_id IS NOT NULL;
