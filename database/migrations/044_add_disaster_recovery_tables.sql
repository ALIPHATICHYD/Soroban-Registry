-- Add disaster recovery tables and related entities

-- Disaster Recovery Plans Table
CREATE TABLE disaster_recovery_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    rto_minutes INTEGER NOT NULL DEFAULT 60,  -- Recovery Time Objective in minutes
    rpo_minutes INTEGER NOT NULL DEFAULT 5,   -- Recovery Point Objective in minutes
    recovery_strategy VARCHAR(100) NOT NULL,  -- 'automated', 'manual', 'hybrid'
    backup_frequency_minutes INTEGER NOT NULL DEFAULT 15,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(contract_id)
);

-- Notification Templates Table
CREATE TABLE notification_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,        -- e.g., 'recovery_started', 'recovery_completed'
    subject TEXT NOT NULL,
    message_template TEXT NOT NULL,            -- Template with placeholders
    channel VARCHAR(20) NOT NULL,             -- 'email', 'sms', 'push', 'webhook'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- User Notification Preferences Table
CREATE TABLE user_notification_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES publishers(id), -- Assuming publishers table represents users
    contract_id UUID REFERENCES contracts(id) ON DELETE CASCADE,  -- NULL means all contracts
    notification_types TEXT[] NOT NULL DEFAULT '{}', -- ['recovery_started', 'recovery_completed', 'incident_detected']
    channels TEXT[] NOT NULL DEFAULT '{}',          -- ['email', 'sms', 'push']
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Notification Logs Table
CREATE TABLE notification_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    notification_type VARCHAR(50) NOT NULL,         -- 'recovery_started', 'recovery_completed', etc.
    recipients TEXT[] NOT NULL,                     -- Array of recipient addresses/user IDs
    message TEXT NOT NULL,
    sent_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status VARCHAR(20) NOT NULL DEFAULT 'sent',     -- 'sent', 'delivered', 'failed'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Post-Incident Reports Table
CREATE TABLE post_incident_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    incident_id UUID,                             -- Reference to incident if applicable
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    root_cause TEXT NOT NULL,
    impact_assessment TEXT NOT NULL,
    recovery_steps TEXT[] NOT NULL DEFAULT '{}',
    lessons_learned TEXT[] NOT NULL DEFAULT '{}',
    created_by VARCHAR(255) NOT NULL,             -- User who created the report
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Action Items Table (for tracking improvements after incidents)
CREATE TABLE action_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id UUID NOT NULL REFERENCES post_incident_reports(id) ON DELETE CASCADE,
    description TEXT NOT NULL,
    owner VARCHAR(255) NOT NULL,                  -- Who is responsible
    due_date TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'todo',   -- 'todo', 'in_progress', 'completed'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Recovery Metrics Table (for tracking RTO/RPO compliance)
CREATE TABLE recovery_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    drp_id UUID REFERENCES disaster_recovery_plans(id) ON DELETE SET NULL,
    recovery_type VARCHAR(50) NOT NULL,           -- 'disaster_recovery', 'drill', 'test'
    rto_achieved_seconds INTEGER NOT NULL,        -- Actual RTO achieved
    rpo_achieved_seconds INTEGER NOT NULL,        -- Actual RPO achieved
    recovery_duration_seconds INTEGER NOT NULL,   -- Total time to complete recovery
    data_loss_seconds INTEGER NOT NULL,           -- Amount of data lost
    recovery_success BOOLEAN NOT NULL,
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX idx_disaster_recovery_plans_contract_id ON disaster_recovery_plans(contract_id);
CREATE INDEX idx_notification_logs_contract_id ON notification_logs(contract_id);
CREATE INDEX idx_notification_logs_sent_at ON notification_logs(sent_at);
CREATE INDEX idx_post_incident_reports_contract_id ON post_incident_reports(contract_id);
CREATE INDEX idx_post_incident_reports_created_at ON post_incident_reports(created_at);
CREATE INDEX idx_action_items_report_id ON action_items(report_id);
CREATE INDEX idx_action_items_status ON action_items(status);
CREATE INDEX idx_recovery_metrics_contract_id ON recovery_metrics(contract_id);
CREATE INDEX idx_recovery_metrics_executed_at ON recovery_metrics(executed_at);

-- Insert default notification templates
INSERT INTO notification_templates (name, subject, message_template, channel) VALUES
('recovery_started', 'Disaster Recovery Started for Contract {{contract_id}}', 'Disaster recovery has been initiated for contract {{contract_id}}. Current status: {{status}}.', 'email'),
('recovery_completed', 'Disaster Recovery Completed for Contract {{contract_id}}', 'Disaster recovery for contract {{contract_id}} has been completed successfully. RTO: {{rto_seconds}}s, RPO: {{rpo_seconds}}s.', 'email'),
('recovery_failed', 'Disaster Recovery Failed for Contract {{contract_id}}', 'Disaster recovery for contract {{contract_id}} has failed. Error: {{error_message}}', 'email'),
('drill_scheduled', 'Quarterly DR Drill Scheduled', 'A quarterly disaster recovery drill has been scheduled for contract {{contract_id}} on {{scheduled_date}}.', 'email'),
('drill_completed', 'Quarterly DR Drill Completed', 'The quarterly disaster recovery drill for contract {{contract_id}} has been completed. Results: {{results_summary}}', 'email');