-- Migration: 20260330000001_contract_notification_system
-- Features: #493 Add Contract Notification/Alert System for Updates

BEGIN;

-- Create enum for notification types
CREATE TYPE notification_type AS ENUM (
    'new_version',
    'verification_status',
    'security_issue',
    'security_scan_completed',
    'breaking_change',
    'deprecation',
    'maintenance',
    'compatibility_issue'
);

-- Create enum for notification channels
CREATE TYPE notification_channel AS ENUM ('email', 'webhook', 'push', 'in_app');

-- Create enum for notification frequency
CREATE TYPE notification_frequency AS ENUM ('realtime', 'daily_digest', 'weekly_digest');

-- Create enum for subscription status
CREATE TYPE subscription_status AS ENUM ('active', 'paused', 'unsubscribed');

-- ═══════════════════════════════════════════════════════════════════════════
-- Contract Subscriptions
-- ═══════════════════════════════════════════════════════════════════════════

-- Main subscriptions table for users following contracts
CREATE TABLE contract_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    
    -- Subscription settings
    status subscription_status NOT NULL DEFAULT 'active',
    notification_types notification_type[] NOT NULL DEFAULT ARRAY['new_version', 'verification_status', 'security_issue']::notification_type[],
    channels notification_channel[] NOT NULL DEFAULT ARRAY['in_app']::notification_channel[],
    frequency notification_frequency NOT NULL DEFAULT 'realtime',
    
    -- Filters for granular control
    min_severity issue_severity_type DEFAULT 'low', -- Minimum severity for security issue notifications
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Ensure unique subscription per user-contract pair
    CONSTRAINT unique_user_contract_subscription UNIQUE (user_id, contract_id)
);

CREATE INDEX idx_contract_subscriptions_user ON contract_subscriptions(user_id);
CREATE INDEX idx_contract_subscriptions_contract ON contract_subscriptions(contract_id);
CREATE INDEX idx_contract_subscriptions_status ON contract_subscriptions(status);

-- ═══════════════════════════════════════════════════════════════════════════
-- Notification Queue
-- ═══════════════════════════════════════════════════════════════════════════

-- Queue for pending notifications
CREATE TABLE notification_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id UUID NOT NULL REFERENCES contract_subscriptions(id) ON DELETE CASCADE,
    
    -- Notification content
    notification_type notification_type NOT NULL,
    title VARCHAR(500) NOT NULL,
    message TEXT NOT NULL,
    
    -- Context data
    contract_id UUID NOT NULL REFERENCES contracts(id) ON DELETE CASCADE,
    contract_version_id UUID REFERENCES contract_versions(id) ON DELETE SET NULL,
    security_issue_id UUID REFERENCES security_issues(id) ON DELETE SET NULL,
    metadata JSONB DEFAULT '{}', -- Additional context data
    
    -- Delivery
    channels notification_channel[] NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'processing', 'sent', 'failed', 'cancelled'
    priority INTEGER NOT NULL DEFAULT 5, -- 1-10, lower is higher priority
    scheduled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Delivery tracking
    sent_at TIMESTAMPTZ,
    delivered_channels notification_channel[],
    delivery_errors TEXT,
    
    -- Retry logic
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    next_retry_at TIMESTAMPTZ,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notification_queue_status ON notification_queue(status);
CREATE INDEX idx_notification_queue_scheduled ON notification_queue(scheduled_at);
CREATE INDEX idx_notification_queue_subscription ON notification_queue(subscription_id);
CREATE INDEX idx_notification_queue_contract ON notification_queue(contract_id);
CREATE INDEX idx_notification_queue_retry ON notification_queue(next_retry_at);

-- ═══════════════════════════════════════════════════════════════════════════
-- Notification Delivery Logs
-- ═══════════════════════════════════════════════════════════════════════════

-- Track individual notification deliveries
CREATE TABLE notification_delivery_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    notification_id UUID NOT NULL REFERENCES notification_queue(id) ON DELETE CASCADE,
    channel notification_channel NOT NULL,
    
    -- Recipient info
    recipient_id UUID REFERENCES auth_users(id) ON DELETE SET NULL,
    recipient_address VARCHAR(500), -- Email address or webhook URL
    
    -- Delivery status
    status VARCHAR(50) NOT NULL, -- 'sent', 'delivered', 'failed', 'bounced'
    error_message TEXT,
    response_code INTEGER,
    response_body TEXT,
    
    -- Timing
    sent_at TIMESTAMPTZ,
    delivered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notification_delivery_logs_notification ON notification_delivery_logs(notification_id);
CREATE INDEX idx_notification_delivery_logs_channel ON notification_delivery_logs(channel);
CREATE INDEX idx_notification_delivery_logs_status ON notification_delivery_logs(status);

-- ═══════════════════════════════════════════════════════════════════════════
-- User Notification Preferences (Global Settings)
-- ═══════════════════════════════════════════════════════════════════════════

-- Extend user preferences with notification settings
ALTER TABLE user_preferences
    ADD COLUMN notification_frequency notification_frequency NOT NULL DEFAULT 'realtime',
    ADD COLUMN notification_channels notification_channel[] DEFAULT ARRAY['in_app']::notification_channel[],
    ADD COLUMN email_notifications_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN webhook_url VARCHAR(500),
    ADD COLUMN webhook_secret_encrypted BYTEA,
    ADD COLUMN quiet_hours_start TIME,
    ADD COLUMN quiet_hours_end TIME,
    ADD COLUMN timezone VARCHAR(100) DEFAULT 'UTC';

-- ═══════════════════════════════════════════════════════════════════════════
-- Webhook Configurations
-- ═══════════════════════════════════════════════════════════════════════════

-- Store webhook configurations for users/organizations
CREATE TABLE webhook_configurations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES auth_users(id) ON DELETE CASCADE,
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    
    -- Webhook details
    name VARCHAR(255) NOT NULL,
    url VARCHAR(500) NOT NULL,
    secret_encrypted BYTEA, -- For signing webhook payloads
    
    -- Configuration
    notification_types notification_type[] NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    verify_ssl BOOLEAN NOT NULL DEFAULT TRUE,
    custom_headers JSONB DEFAULT '{}',
    
    -- Rate limiting
    rate_limit_per_minute INTEGER DEFAULT 60,
    
    -- Statistics
    total_deliveries INTEGER NOT NULL DEFAULT 0,
    failed_deliveries INTEGER NOT NULL DEFAULT 0,
    last_delivery_at TIMESTAMPTZ,
    last_success_at TIMESTAMPTZ,
    last_failure_at TIMESTAMPTZ,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhook_configurations_user ON webhook_configurations(user_id);
CREATE INDEX idx_webhook_configurations_organization ON webhook_configurations(organization_id);
CREATE INDEX idx_webhook_configurations_active ON webhook_configurations(is_active);

-- Ensure either user_id or organization_id is set, but not both
ALTER TABLE webhook_configurations
    ADD CONSTRAINT webhook_owner_check CHECK (
        (user_id IS NOT NULL AND organization_id IS NULL) OR
        (user_id IS NULL AND organization_id IS NOT NULL)
    );

-- ═══════════════════════════════════════════════════════════════════════════
-- Notification Templates
-- ═══════════════════════════════════════════════════════════════════════════

-- NOTE: `notification_templates` is already created by migration
-- 044_add_disaster_recovery_tables.sql with columns
-- (name, subject, message_template, channel). Re-creating it here failed with
-- "relation \"notification_templates\" already exists", and the definition that
-- previously lived here declared an entirely different, unused schema (the seed
-- INSERT further down, the backend in notification_handlers.rs, and migration
-- 069 all use the 044 columns). The duplicate CREATE TABLE and its indexes on
-- columns that don't exist in the 044 table have been removed; this migration
-- now reuses the existing table (see the seed INSERT below).

-- ═══════════════════════════════════════════════════════════════════════════
-- Batch Notification Processing
-- ═══════════════════════════════════════════════════════════════════════════

-- Track batch notification jobs
CREATE TABLE notification_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    batch_type VARCHAR(100) NOT NULL, -- 'daily_digest', 'weekly_digest', 'bulk_send'
    
    -- Scope
    user_id UUID REFERENCES auth_users(id) ON DELETE CASCADE,
    contract_id UUID REFERENCES contracts(id) ON DELETE CASCADE,
    subscription_ids UUID[],
    
    -- Processing
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'processing', 'completed', 'failed'
    total_notifications INTEGER NOT NULL DEFAULT 0,
    processed_notifications INTEGER NOT NULL DEFAULT 0,
    failed_notifications INTEGER NOT NULL DEFAULT 0,
    
    -- Timing
    scheduled_for TIMESTAMPTZ NOT NULL,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notification_batches_status ON notification_batches(status);
CREATE INDEX idx_notification_batches_scheduled ON notification_batches(scheduled_for);
CREATE INDEX idx_notification_batches_user ON notification_batches(user_id);

-- ═══════════════════════════════════════════════════════════════════════════
-- Notification Statistics
-- ═══════════════════════════════════════════════════════════════════════════

-- Aggregate notification statistics
CREATE TABLE notification_statistics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES auth_users(id) ON DELETE CASCADE,
    contract_id UUID REFERENCES contracts(id) ON DELETE CASCADE,
    
    -- Time period
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    
    -- Counts by type
    new_version_count INTEGER NOT NULL DEFAULT 0,
    verification_status_count INTEGER NOT NULL DEFAULT 0,
    security_issue_count INTEGER NOT NULL DEFAULT 0,
    security_scan_completed_count INTEGER NOT NULL DEFAULT 0,
    breaking_change_count INTEGER NOT NULL DEFAULT 0,
    deprecation_count INTEGER NOT NULL DEFAULT 0,
    maintenance_count INTEGER NOT NULL DEFAULT 0,
    compatibility_issue_count INTEGER NOT NULL DEFAULT 0,
    
    -- Delivery stats
    total_sent INTEGER NOT NULL DEFAULT 0,
    total_delivered INTEGER NOT NULL DEFAULT 0,
    total_failed INTEGER NOT NULL DEFAULT 0,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notification_statistics_user ON notification_statistics(user_id);
CREATE INDEX idx_notification_statistics_contract ON notification_statistics(contract_id);
CREATE INDEX idx_notification_statistics_period ON notification_statistics(period_start, period_end);

-- ═══════════════════════════════════════════════════════════════════════════
-- Triggers
-- ═══════════════════════════════════════════════════════════════════════════

-- Trigger to update updated_at timestamps
CREATE TRIGGER update_contract_subscriptions_updated_at
    BEFORE UPDATE ON contract_subscriptions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_notification_queue_updated_at
    BEFORE UPDATE ON notification_queue
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_webhook_configurations_updated_at
    BEFORE UPDATE ON webhook_configurations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_notification_templates_updated_at
    BEFORE UPDATE ON notification_templates
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Trigger to create notification when contract is updated
CREATE OR REPLACE FUNCTION notify_contract_update()
RETURNS TRIGGER AS $$
DECLARE
    sub RECORD;
    notif_title TEXT;
    notif_message TEXT;
BEGIN
    -- Determine notification type and message based on what changed
    IF (OLD.is_verified IS DISTINCT FROM NEW.is_verified OR 
        OLD.verification_status IS DISTINCT FROM NEW.verification_status) THEN
        notif_title := 'Verification Status Updated';
        notif_message := format('Contract "%s" verification status changed to %s', NEW.name, NEW.verification_status);
        
        -- Queue notifications for subscribers interested in verification status changes
        FOR sub IN SELECT * FROM contract_subscriptions 
                   WHERE contract_id = NEW.id 
                   AND status = 'active'
                   AND 'verification_status' = ANY(notification_types)
        LOOP
            INSERT INTO notification_queue (
                subscription_id, notification_type, title, message, 
                contract_id, channels, priority
            ) VALUES (
                sub.id, 'verification_status', notif_title, notif_message,
                NEW.id, sub.channels, 3
            );
        END LOOP;
        
    ELSIF (TG_OP = 'INSERT' OR EXISTS (
        SELECT 1 FROM contract_versions WHERE contract_id = NEW.id 
        ORDER BY created_at DESC LIMIT 1
    )) THEN
        -- New version notification would be triggered from contract_versions table
        NULL;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_notify_contract_update ON contracts;
CREATE TRIGGER trg_notify_contract_update
    AFTER UPDATE ON contracts
    FOR EACH ROW EXECUTE FUNCTION notify_contract_update();

-- Trigger to create notification when new version is created
CREATE OR REPLACE FUNCTION notify_new_version()
RETURNS TRIGGER AS $$
DECLARE
    sub RECORD;
    notif_title TEXT;
    notif_message TEXT;
    contract_name TEXT;
BEGIN
    -- Get contract name
    SELECT name INTO contract_name FROM contracts WHERE id = NEW.contract_id;
    
    notif_title := 'New Version Available';
    notif_message := format('Contract "%s" has a new version: %s', contract_name, NEW.version);
    
    -- Queue notifications for subscribers interested in new versions
    FOR sub IN SELECT * FROM contract_subscriptions 
               WHERE contract_id = NEW.contract_id 
               AND status = 'active'
               AND 'new_version' = ANY(notification_types)
    LOOP
        INSERT INTO notification_queue (
            subscription_id, notification_type, title, message, 
            contract_id, contract_version_id, channels, priority
        ) VALUES (
            sub.id, 'new_version', notif_title, notif_message,
            NEW.contract_id, NEW.id, sub.channels, 2
        );
    END LOOP;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_notify_new_version ON contract_versions;
CREATE TRIGGER trg_notify_new_version
    AFTER INSERT ON contract_versions
    FOR EACH ROW EXECUTE FUNCTION notify_new_version();

-- Trigger to create notification when security issue is found
CREATE OR REPLACE FUNCTION notify_security_issue()
RETURNS TRIGGER AS $$
DECLARE
    sub RECORD;
    notif_title TEXT;
    notif_message TEXT;
    contract_name TEXT;
BEGIN
    -- Get contract name
    SELECT name INTO contract_name FROM contracts WHERE id = NEW.contract_id;
    
    notif_title := format('Security Issue Detected: %s', NEW.title);
    notif_message := format(
        'A %s severity security issue was found in contract "%s": %s',
        NEW.severity, contract_name, NEW.description
    );
    
    -- Queue notifications for subscribers interested in security issues
    -- Only notify if severity meets user's minimum threshold
    FOR sub IN SELECT * FROM contract_subscriptions 
               WHERE contract_id = NEW.contract_id 
               AND status = 'active'
               AND 'security_issue' = ANY(notification_types)
               AND (
                   (sub.min_severity IS NULL) OR
                   (NEW.severity = 'critical') OR
                   (NEW.severity = 'high' AND sub.min_severity IN ('low', 'medium', 'high')) OR
                   (NEW.severity = 'medium' AND sub.min_severity IN ('low', 'medium')) OR
                   (NEW.severity = 'low' AND sub.min_severity = 'low')
               )
    LOOP
        INSERT INTO notification_queue (
            subscription_id, notification_type, title, message, 
            contract_id, security_issue_id, channels, priority
        ) VALUES (
            sub.id, 'security_issue', notif_title, notif_message,
            NEW.contract_id, NEW.id, sub.channels, 
            CASE NEW.severity
                WHEN 'critical' THEN 1
                WHEN 'high' THEN 2
                WHEN 'medium' THEN 4
                ELSE 6
            END
        );
    END LOOP;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_notify_security_issue ON security_issues;
CREATE TRIGGER trg_notify_security_issue
    AFTER INSERT ON security_issues
    FOR EACH ROW EXECUTE FUNCTION notify_security_issue();

-- ═══════════════════════════════════════════════════════════════════════════
-- Insert Default Notification Templates
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO notification_templates (name, subject, message_template, channel) VALUES
    ('new_version', 'New version available for {{contract_name}}', 'A new version ({{version}}) of {{contract_name}} has been published.\n\nRelease notes: {{release_notes}}', 'in_app'),
    ('verification_status', '{{contract_name}} verification status: {{status}}', 'The verification status of {{contract_name}} has been updated to {{status}}.\n\n{{notes}}', 'in_app'),
    ('security_issue', 'Security issue detected in {{contract_name}}', 'A {{severity}} severity security issue has been detected in {{contract_name}}.\n\nTitle: {{issue_title}}\nDescription: {{description}}\n\nRemediation: {{remediation}}', 'in_app'),
    ('security_scan_completed', 'Security scan completed for {{contract_name}}', 'A security scan has completed for {{contract_name}}.\n\nResults: {{total_issues}} issues found ({{critical_issues}} critical, {{high_issues}} high)', 'in_app'),
    ('deprecation', '{{contract_name}} has been deprecated', '{{contract_name}} has been deprecated and will be retired on {{retirement_date}}.\n\nMigration guide: {{migration_guide}}', 'in_app')
ON CONFLICT DO NOTHING;

-- ═══════════════════════════════════════════════════════════════════════════
-- Comments for Documentation
-- ═══════════════════════════════════════════════════════════════════════════

COMMENT ON TABLE contract_subscriptions IS 'User subscriptions to contract updates (#493)';
COMMENT ON TABLE notification_queue IS 'Queue for pending notifications (#493)';
COMMENT ON TABLE notification_delivery_logs IS 'Delivery tracking for notifications (#493)';
COMMENT ON TABLE webhook_configurations IS 'User/organization webhook configurations (#493)';
COMMENT ON TABLE notification_templates IS 'Templates for different notification types (#493)';
COMMENT ON TABLE notification_batches IS 'Batch notification job tracking (#493)';
COMMENT ON TABLE notification_statistics IS 'Aggregated notification statistics (#493)';

COMMENT ON COLUMN contract_subscriptions.notification_types IS 'Types of notifications the user wants to receive (#493)';
COMMENT ON COLUMN contract_subscriptions.frequency IS 'How often notifications are sent (realtime, daily, weekly) (#493)';
COMMENT ON COLUMN contract_subscriptions.min_severity IS 'Minimum severity level for security issue notifications (#493)';
COMMENT ON COLUMN notification_queue.priority IS 'Priority 1-10, lower is higher priority (#493)';
COMMENT ON COLUMN webhook_configurations.secret_encrypted IS 'Encrypted secret for signing webhook payloads (#493)';

COMMIT;
