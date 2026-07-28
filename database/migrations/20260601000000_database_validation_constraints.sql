-- Database-level validation and integrity constraints
-- This migration strengthens the schema with format checks, range checks,
-- and trigger-based validation for cross-column rules that are awkward to
-- express as plain CHECK constraints.

BEGIN;

-- ---------------------------------------------------------------------------
-- Publishers
-- ---------------------------------------------------------------------------

ALTER TABLE publishers
    ADD CONSTRAINT chk_publishers_stellar_address_format
        CHECK (stellar_address ~ '^G[A-Z0-9]{55}$'),
    ADD CONSTRAINT chk_publishers_username_not_blank
        CHECK (username IS NULL OR char_length(btrim(username)) > 0),
    ADD CONSTRAINT chk_publishers_email_format
        CHECK (
            email IS NULL OR (
                char_length(btrim(email)) BETWEEN 1 AND 320
                AND email ~ '^[^[:space:]@]+@[^[:space:]@]+\.[^[:space:]@]+$'
            )
        ),
    ADD CONSTRAINT chk_publishers_github_url_format
        CHECK (
            github_url IS NULL OR (
                char_length(btrim(github_url)) <= 500
                AND github_url ~ '^https?://[^[:space:]]+$'
            )
        ),
    ADD CONSTRAINT chk_publishers_website_url_format
        CHECK (
            website IS NULL OR (
                char_length(btrim(website)) <= 500
                AND website ~ '^https?://[^[:space:]]+$'
            )
        );

-- ---------------------------------------------------------------------------
-- Contracts
-- ---------------------------------------------------------------------------

ALTER TABLE contracts
    ADD CONSTRAINT chk_contracts_contract_id_format
        CHECK (char_length(btrim(contract_id)) = 56 AND contract_id ~ '^C[A-Z0-9]{55}$'),
    ADD CONSTRAINT chk_contracts_wasm_hash_format
        CHECK (char_length(btrim(wasm_hash)) = 64 AND wasm_hash ~ '^[A-Fa-f0-9]{64}$'),
    ADD CONSTRAINT chk_contracts_name_length
        CHECK (char_length(btrim(name)) BETWEEN 1 AND 255),
    ADD CONSTRAINT chk_contracts_slug_format
        CHECK (
            slug IS NULL OR (
                char_length(btrim(slug)) BETWEEN 1 AND 100
                AND slug ~ '^[a-z0-9]+(?:-[a-z0-9]+)*$'
            )
        ),
    ADD CONSTRAINT chk_contracts_category_length
        CHECK (category IS NULL OR char_length(btrim(category)) BETWEEN 1 AND 100),
    ADD CONSTRAINT chk_contracts_health_score_range
        CHECK (health_score BETWEEN 0 AND 100),
    ADD CONSTRAINT chk_contracts_deployment_count_non_negative
        CHECK (deployment_count >= 0),
    ADD CONSTRAINT chk_contracts_network_configs_object
        CHECK (network_configs IS NULL OR jsonb_typeof(network_configs) = 'object'),
    ADD CONSTRAINT chk_contracts_current_version_length
        CHECK (current_version IS NULL OR char_length(btrim(current_version)) BETWEEN 1 AND 50);

-- Keep verification columns consistent without breaking import paths.
CREATE OR REPLACE FUNCTION validate_contract_verification_state()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.is_verified THEN
        NEW.verification_status := 'verified';
        IF NEW.verified_at IS NULL THEN
            NEW.verified_at := NOW();
        END IF;
    ELSIF NEW.verification_status = 'verified' THEN
        NEW.is_verified := TRUE;
        IF NEW.verified_at IS NULL THEN
            NEW.verified_at := NOW();
        END IF;
    ELSIF NEW.verification_status = 'failed' THEN
        NEW.is_verified := FALSE;
        NEW.verified_at := NULL;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_validate_contract_verification_state ON contracts;
CREATE TRIGGER trg_validate_contract_verification_state
    BEFORE INSERT OR UPDATE ON contracts
    FOR EACH ROW
    EXECUTE FUNCTION validate_contract_verification_state();

-- ---------------------------------------------------------------------------
-- Contract versions
-- ---------------------------------------------------------------------------

ALTER TABLE contract_versions
    ADD CONSTRAINT chk_contract_versions_version_length
        CHECK (char_length(btrim(version)) BETWEEN 1 AND 50),
    ADD CONSTRAINT chk_contract_versions_wasm_hash_format
        CHECK (char_length(btrim(wasm_hash)) = 64 AND wasm_hash ~ '^[A-Fa-f0-9]{64}$'),
    ADD CONSTRAINT chk_contract_versions_commit_hash_format
        CHECK (
            commit_hash IS NULL OR (
                char_length(btrim(commit_hash)) = 40
                AND commit_hash ~ '^[A-Fa-f0-9]{40}$'
            )
        ),
    ADD CONSTRAINT chk_contract_versions_source_url_format
        CHECK (
            source_url IS NULL OR (
                char_length(btrim(source_url)) <= 500
                AND source_url ~ '^https?://[^[:space:]]+$'
            )
        ),
    ADD CONSTRAINT chk_contract_versions_signature_length
        CHECK (signature IS NULL OR char_length(btrim(signature)) BETWEEN 1 AND 4096),
    ADD CONSTRAINT chk_contract_versions_publisher_key_length
        CHECK (publisher_key IS NULL OR char_length(btrim(publisher_key)) BETWEEN 1 AND 256),
    ADD CONSTRAINT chk_contract_versions_signature_algorithm
        CHECK (signature_algorithm IS NULL OR signature_algorithm = 'ed25519'),
    ADD CONSTRAINT chk_contract_versions_reverted_from_length
        CHECK (reverted_from IS NULL OR char_length(btrim(reverted_from)) BETWEEN 1 AND 50);

CREATE OR REPLACE FUNCTION validate_contract_version_integrity()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.is_revert AND NEW.reverted_from IS NULL THEN
        RAISE EXCEPTION USING
            MESSAGE = 'contract_versions.reverted_from is required when is_revert is true';
    ELSIF NOT NEW.is_revert AND NEW.reverted_from IS NOT NULL THEN
        RAISE EXCEPTION USING
            MESSAGE = 'contract_versions.reverted_from must be null when is_revert is false';
    END IF;

    IF NEW.signature IS NOT NULL AND (
        NEW.publisher_key IS NULL OR NEW.signature_algorithm IS NULL
    ) THEN
        RAISE EXCEPTION USING
            MESSAGE = 'contract_versions.signature requires publisher_key and signature_algorithm';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_validate_contract_version_integrity ON contract_versions;
CREATE TRIGGER trg_validate_contract_version_integrity
    BEFORE INSERT OR UPDATE ON contract_versions
    FOR EACH ROW
    EXECUTE FUNCTION validate_contract_version_integrity();

-- ---------------------------------------------------------------------------
-- Verifications
-- ---------------------------------------------------------------------------

ALTER TABLE verifications
    ADD CONSTRAINT chk_verifications_compiler_version_length
        CHECK (compiler_version IS NULL OR char_length(btrim(compiler_version)) BETWEEN 1 AND 50),
    ADD CONSTRAINT chk_verifications_error_message_length
        CHECK (error_message IS NULL OR char_length(btrim(error_message)) BETWEEN 1 AND 10000);

CREATE OR REPLACE FUNCTION validate_verification_integrity()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status = 'verified' AND NEW.verified_at IS NULL THEN
        RAISE EXCEPTION USING
            MESSAGE = 'verifications.verified_at is required when status is verified';
    ELSIF NEW.status = 'failed' AND (NEW.error_message IS NULL OR btrim(NEW.error_message) = '') THEN
        RAISE EXCEPTION USING
            MESSAGE = 'verifications.error_message is required when status is failed';
    ELSIF NEW.status = 'pending' AND NEW.verified_at IS NOT NULL THEN
        RAISE EXCEPTION USING
            MESSAGE = 'verifications.verified_at must be null while status is pending';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_validate_verification_integrity ON verifications;
CREATE TRIGGER trg_validate_verification_integrity
    BEFORE INSERT OR UPDATE ON verifications
    FOR EACH ROW
    EXECUTE FUNCTION validate_verification_integrity();

-- ---------------------------------------------------------------------------
-- Reviews
-- ---------------------------------------------------------------------------

ALTER TABLE reviews
    ADD CONSTRAINT chk_reviews_review_text_length
        CHECK (review_text IS NULL OR char_length(review_text) <= 5000),
    ADD CONSTRAINT chk_reviews_version_length
        CHECK (char_length(btrim(version)) BETWEEN 1 AND 50);

-- ---------------------------------------------------------------------------
-- Tags
-- ---------------------------------------------------------------------------

ALTER TABLE tags
    ADD CONSTRAINT chk_tags_prefix_not_blank
        CHECK (char_length(btrim(prefix)) BETWEEN 1 AND 100),
    ADD CONSTRAINT chk_tags_name_not_blank
        CHECK (char_length(btrim(name)) BETWEEN 1 AND 255),
    ADD CONSTRAINT chk_tags_usage_count_non_negative
        CHECK (usage_count >= 0);

ALTER TABLE tag_aliases
    ADD CONSTRAINT chk_tag_aliases_alias_not_blank
        CHECK (char_length(btrim(alias)) BETWEEN 1 AND 255);

ALTER TABLE tag_usage_log
    ADD CONSTRAINT chk_tag_usage_log_usage_count_non_negative
        CHECK (usage_count >= 0);

-- ---------------------------------------------------------------------------
-- Organizations and invitations
-- ---------------------------------------------------------------------------

ALTER TABLE organizations
    ADD CONSTRAINT chk_organizations_name_not_blank
        CHECK (char_length(btrim(name)) BETWEEN 1 AND 255),
    ADD CONSTRAINT chk_organizations_slug_format
        CHECK (
            char_length(btrim(slug)) BETWEEN 1 AND 255
            AND slug ~ '^[a-z0-9]+(?:-[a-z0-9]+)*$'
        ),
    ADD CONSTRAINT chk_organizations_quota_contracts_positive
        CHECK (quota_contracts > 0),
    ADD CONSTRAINT chk_organizations_rate_limit_requests_positive
        CHECK (rate_limit_requests > 0);

ALTER TABLE organization_invitations
    ADD CONSTRAINT chk_organization_invitations_email_format
        CHECK (
            char_length(btrim(email)) BETWEEN 1 AND 320
            AND email ~ '^[^[:space:]@]+@[^[:space:]@]+\.[^[:space:]@]+$'
        ),
    ADD CONSTRAINT chk_organization_invitations_token_not_blank
        CHECK (char_length(btrim(token)) BETWEEN 1 AND 255),
    ADD CONSTRAINT chk_organization_invitations_time_order
        CHECK (expires_at > created_at),
    ADD CONSTRAINT chk_organization_invitations_accepted_at_order
        CHECK (accepted_at IS NULL OR accepted_at >= created_at);

-- ---------------------------------------------------------------------------
-- Contributors
-- ---------------------------------------------------------------------------

ALTER TABLE contributors
    ALTER COLUMN name TYPE VARCHAR(255);

ALTER TABLE contributors
    ADD CONSTRAINT chk_contributors_stellar_address_format
        CHECK (stellar_address ~ '^G[A-Z0-9]{55}$'),
    ADD CONSTRAINT chk_contributors_name_length
        CHECK (name IS NULL OR char_length(btrim(name)) BETWEEN 1 AND 255),
    ADD CONSTRAINT chk_contributors_links_object
        CHECK (jsonb_typeof(links) = 'object');

-- ---------------------------------------------------------------------------
-- User preferences
-- ---------------------------------------------------------------------------

ALTER TABLE user_preferences
    ADD CONSTRAINT chk_user_preferences_theme
        CHECK (theme IN ('dark', 'light', 'system')),
    ADD CONSTRAINT chk_user_preferences_language_not_blank
        CHECK (char_length(btrim(language)) BETWEEN 2 AND 10),
    ADD CONSTRAINT chk_user_preferences_favorites_json_array
        CHECK (jsonb_typeof(favorites) = 'array'),
    ADD CONSTRAINT chk_user_preferences_extensible_settings_json_object
        CHECK (jsonb_typeof(extensible_settings) = 'object'),
    ADD CONSTRAINT chk_user_preferences_timezone_not_blank
        CHECK (timezone IS NULL OR char_length(btrim(timezone)) BETWEEN 1 AND 100),
    ADD CONSTRAINT chk_user_preferences_webhook_url_format
        CHECK (
            webhook_url IS NULL OR (
                char_length(btrim(webhook_url)) <= 500
                AND webhook_url ~ '^https?://[^[:space:]]+$'
            )
        );

-- ---------------------------------------------------------------------------
-- Notification queues and delivery logs
-- ---------------------------------------------------------------------------

ALTER TABLE notification_queue
    ADD CONSTRAINT chk_notification_queue_status
        CHECK (status IN ('pending', 'processing', 'sent', 'failed', 'cancelled')),
    ADD CONSTRAINT chk_notification_queue_priority_range
        CHECK (priority BETWEEN 1 AND 10),
    ADD CONSTRAINT chk_notification_queue_retry_count_non_negative
        CHECK (retry_count >= 0),
    ADD CONSTRAINT chk_notification_queue_max_retries_non_negative
        CHECK (max_retries >= 0);

ALTER TABLE notification_delivery_logs
    ADD CONSTRAINT chk_notification_delivery_logs_status
        CHECK (status IN ('sent', 'delivered', 'failed', 'bounced'));

COMMIT;
