-- Issue #856: Batch notification delivery and read tracking

BEGIN;

CREATE TABLE IF NOT EXISTS batch_notification_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_type VARCHAR(32) NOT NULL CHECK (
        message_type IN ('info', 'warning', 'critical', 'action-required')
    ),
    message TEXT NOT NULL,
    channels TEXT[] NOT NULL DEFAULT '{}',
    scheduled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status VARCHAR(32) NOT NULL DEFAULT 'scheduled' CHECK (
        status IN ('scheduled', 'sent', 'partial', 'failed')
    ),
    total_recipients INTEGER NOT NULL DEFAULT 0,
    delivered_count INTEGER NOT NULL DEFAULT 0,
    failed_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS batch_notification_deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES batch_notification_jobs(id) ON DELETE CASCADE,
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    contract_address VARCHAR(56) NOT NULL,
    recipient TEXT NOT NULL,
    channel VARCHAR(32) NOT NULL CHECK (channel IN ('email', 'in-app', 'webhook')),
    delivery_status VARCHAR(32) NOT NULL DEFAULT 'pending' CHECK (
        delivery_status IN ('pending', 'sent', 'failed')
    ),
    read_at TIMESTAMPTZ,
    error_message TEXT,
    sent_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_batch_notification_jobs_status
    ON batch_notification_jobs(status, scheduled_at);

CREATE INDEX IF NOT EXISTS idx_batch_notification_deliveries_job
    ON batch_notification_deliveries(job_id);

CREATE INDEX IF NOT EXISTS idx_batch_notification_deliveries_contract
    ON batch_notification_deliveries(contract_id);

CREATE INDEX IF NOT EXISTS idx_batch_notification_deliveries_read
    ON batch_notification_deliveries(job_id, read_at);

COMMIT;
