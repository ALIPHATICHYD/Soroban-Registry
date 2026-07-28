-- Migration: Bulk Import Jobs Table
-- Description: Create table for tracking async bulk import operations

-- Create enum for import job status
CREATE TYPE import_job_status AS ENUM (
    'pending',
    'processing', 
    'completed',
    'failed',
    'partial'
);

-- Create table for bulk import jobs
CREATE TABLE bulk_import_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    requested_by UUID REFERENCES publishers(id),
    status import_job_status NOT NULL DEFAULT 'pending',
    fail_safe BOOLEAN NOT NULL DEFAULT false,
    skip_existing BOOLEAN NOT NULL DEFAULT false,
    total_count INTEGER NOT NULL DEFAULT 0,
    processed_count INTEGER NOT NULL DEFAULT 0,
    imported_count INTEGER NOT NULL DEFAULT 0,
    failed_count INTEGER NOT NULL DEFAULT 0,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    results JSONB DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for efficient querying
CREATE INDEX idx_bulk_import_jobs_status ON bulk_import_jobs(status);
CREATE INDEX idx_bulk_import_jobs_requested_by ON bulk_import_jobs(requested_by);
CREATE INDEX idx_bulk_import_jobs_requested_at ON bulk_import_jobs(requested_at);
CREATE INDEX idx_bulk_import_jobs_created_at ON bulk_import_jobs(created_at);

-- Create function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_import_job_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create trigger to automatically update updated_at
CREATE TRIGGER update_bulk_import_jobs_updated_at 
    BEFORE UPDATE ON bulk_import_jobs 
    FOR EACH ROW 
    EXECUTE FUNCTION update_import_job_updated_at();

-- Create a view for import job summary
CREATE VIEW bulk_import_job_summary AS
SELECT 
    id,
    status,
    fail_safe,
    skip_existing,
    total_count,
    processed_count,
    imported_count,
    failed_count,
    ROUND(
        CASE 
            WHEN total_count > 0 THEN (imported_count::numeric / total_count::numeric) * 100 
            ELSE 0 
        END, 2
    ) as success_rate,
    requested_at,
    started_at,
    completed_at,
    CASE 
        WHEN completed_at IS NOT NULL THEN EXTRACT(EPOCH FROM (completed_at - requested_at))::integer
        ELSE NULL 
    END as duration_seconds,
    created_at
FROM bulk_import_jobs;

-- Add comment for documentation
COMMENT ON TABLE bulk_import_jobs IS 'Tracks async bulk contract import operations';
COMMENT ON COLUMN bulk_import_jobs.results IS 'JSON array of individual contract import results with success/error details';
