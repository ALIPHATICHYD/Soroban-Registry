//! Validatable implementations for handler-local request types (Issue #893).

use rust_decimal::Decimal;
use serde::Deserialize;
use shared::models::{
    AddCollaborativeCommentRequest, AdvanceCanaryRequest, BatchGasEstimateRequest,
    BatchMetadataUpdateRequest, BatchMethodEntry, BatchSimilarityAnalysisRequest, BatchVerifyItem,
    BatchVerifyRequest, CloneContractRequest, CreateAbTestRequest, CreateAlertConfigRequest,
    CreateBackupRequest, CreateCanaryRequest, CreateCollaborativeReviewRequest,
    CreateContributorRequest, CreateOrganizationRequest, CreateReviewRequest,
    CreateSecurityScannerRequest, CreateWebhookRequest, DeprecateContractRequest,
    FederationOptRequest, FlagReviewRequest, InviteMemberRequest, MethodParamHint,
    ModerateReviewRequest, RecordAbTestMetricRequest, RecordCanaryMetricRequest,
    RecordCustomMetricRequest, RecordPerformanceBenchmarkRequest, RecordPerformanceMetricRequest,
    RegisterFederatedRegistryRequest, RestoreBackupRequest, RevertVersionRequest,
    ReviewVoteRequest, SimulateDeployRequest, SubscribeRequest, SyncFederatedRegistryRequest,
    TriggerSecurityScanRequest, UpdateContributorRequest, UpdateOrganizationRequest,
    UpdateReviewerStatusRequest, UpdateSecurityIssueRequest, UpdateSubscriptionRequest,
    UpdateUserNotificationPreferencesRequest,
};

use crate::abi_versioning_handlers::{CheckCompatibilityRequest, PublishAbiRequest};
use crate::ai::handlers::{ChatRequest, SuggestRequest};
use crate::ai::service::ChatMessage;
use crate::analytics_handlers::WebVitalMetric;
use crate::auth_handlers;
use crate::category_handlers::{CreateCategoryRequest, UpdateCategoryRequest};
use crate::client_observability_handlers::ClientBreakerReport;
use crate::compatibility_testing_handlers::RunCompatibilityTestRequest;
use crate::dependency_handlers::DeclareDependenciesRequest;
use crate::disaster_recovery_models::{
    CreateActionItemRequest, CreateDisasterRecoveryPlanRequest, CreateNotificationTemplateRequest,
    CreatePostIncidentReportRequest, CreateUserNotificationPreferenceRequest,
    ExecuteRecoveryRequest, SendNotificationRequest,
};
use crate::error_logging::ErrorReportRequest;
use crate::favorites_handlers::UpdateFavoritesRequest;
use crate::formal_verification_handlers::TriggerVerificationRequest;
use crate::governance_handlers::{
    CastVoteRequest, CreateProposalRequest, UpsertVotingRightsRequest,
};
use crate::handlers::compatibility::AddCompatibilityRequest;
use crate::handlers::validators::{RegisterValidatorRequest, SubmitAttestationRequest};
use crate::handlers::UploadContractSourceRequest;
use crate::incident_handlers::{
    AddAffectedContractRequest, AddIncidentUpdateRequest, NotifyAffectedUsersRequest,
    PublishAdvisoryRequest, ReportIncidentRequest, UpdateIncidentStatusRequest,
};
use crate::migration_handlers::RegisterMigrationRequest;
use crate::multisig_handlers::{
    CreateDeployProposalRequest, CreateMultisigPolicyRequest, CreatePublisherKeyRequest,
    SignProposalRequest,
};
use crate::mutation_testing_handlers::RunMutationTestRequest;
use crate::patch_handlers::{BulkApplyRequest, BulkTarget, ReconstructRequest};
use crate::publisher_verification_handlers::PublisherVerifyRequest;
use crate::release_notes_handlers::{
    GenerateReleaseNotesRequest, PublishReleaseNotesRequest, UpdateReleaseNotesRequest,
};
use crate::state_monitor::handlers::ResolveAnomalyRequest;
use crate::verification_handlers::ContractVerifyRequest;

use super::extractors::{FieldError, Validatable, ValidationBuilder};
use super::sanitizers::{
    normalize_contract_id, normalize_stellar_address, sanitize_description_optional, sanitize_name,
    sanitize_tags, sanitize_url_optional, trim, trim_optional,
};
use super::validators::{
    validate_base64_size, validate_collection_size, validate_contract_id, validate_email,
    validate_hex_length, validate_json_depth, validate_length, validate_name_format,
    validate_no_xss, validate_one_of, validate_one_of_optional, validate_percentage,
    validate_rating, validate_semver, validate_slug, validate_source_code_size,
    validate_stellar_address, validate_stellar_address_optional, validate_tags,
    validate_url_optional, validate_wasm_hash,
};

const MAX_NAME_LENGTH: usize = 255;
const MAX_DESCRIPTION_LENGTH: usize = 5000;
const MAX_TAGS_COUNT: usize = 10;
const MAX_TAG_LENGTH: usize = 50;
const MAX_SOURCE_CODE_BYTES: usize = 1024 * 1024;
const MAX_JSON_DEPTH: usize = 10;
const MAX_BATCH_SIZE: usize = 100;
const MAX_MESSAGE_LENGTH: usize = 10_000;
const ALLOWED_CATEGORIES: &[&str] = &["DEX", "Lending", "Bridge", "Oracle", "Token", "Other"];

fn validate_positive_decimal(value: &Decimal) -> Result<(), String> {
    if *value <= Decimal::ZERO {
        return Err("must be greater than zero".to_string());
    }
    Ok(())
}

fn validate_optional_text(
    builder: &mut ValidationBuilder,
    field: &str,
    value: &Option<String>,
    max: usize,
) {
    if let Some(v) = value {
        builder.check(field, || validate_length(v, 1, max));
        builder.check(field, || validate_no_xss(v));
    }
}

fn validate_text(
    builder: &mut ValidationBuilder,
    field: &str,
    value: &str,
    min: usize,
    max: usize,
) {
    builder.check(field, || validate_length(value, min, max));
    builder.check(field, || validate_no_xss(value));
}

// ── Wrapper types for untyped JSON bodies ────────────────────────────────────

/// Batch contract ID lookup request.
#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct BatchContractIdsRequest(pub Vec<String>);

fn validate_batch_contract_lookup_id(id: &str) -> Result<(), String> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err("contract ID cannot be empty".to_string());
    }
    if uuid::Uuid::parse_str(trimmed).is_ok() {
        return Ok(());
    }
    validate_contract_id(trimmed)
}

impl Validatable for BatchContractIdsRequest {
    fn sanitize(&mut self) {
        self.0 = self
            .0
            .iter()
            .map(|id| {
                let trimmed = id.trim();
                if uuid::Uuid::parse_str(trimmed).is_ok() {
                    trimmed.to_string()
                } else {
                    normalize_contract_id(trimmed)
                }
            })
            .collect();
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("contract_ids", || {
            validate_collection_size(self.0.len(), 0, MAX_BATCH_SIZE)
        });
        for (i, id) in self.0.iter().enumerate() {
            builder.check(&format!("contract_ids[{i}]"), || {
                validate_batch_contract_lookup_id(id)
            });
        }
        builder.build()
    }
}

impl std::ops::Deref for BatchContractIdsRequest {
    type Target = Vec<String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Validatable for BatchMetadataUpdateRequest {
    fn sanitize(&mut self) {
        if let Some(batch_id) = self.batch_id.as_mut() {
            *batch_id = batch_id.trim().to_string();
        }
        for item in self.items.iter_mut() {
            item.contract_id = item.contract_id.trim().to_string();
            for field in [
                item.name.as_mut(),
                item.description.as_mut(),
                item.category.as_mut(),
                item.change_summary.as_mut(),
            ]
            .into_iter()
            .flatten()
            {
                *field = field.trim().to_string();
            }
            if let Some(tags) = item.tags.as_mut() {
                for tag in tags.iter_mut() {
                    *tag = tag.trim().to_string();
                }
            }
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("items", || {
            validate_collection_size(self.items.len(), 1, MAX_BATCH_SIZE)
        });
        for (i, item) in self.items.iter().enumerate() {
            builder.check(&format!("items[{i}].contract_id"), || {
                validate_batch_contract_lookup_id(&item.contract_id)
            });
        }
        builder.build()
    }
}

impl IntoIterator for BatchContractIdsRequest {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Generic JSON body with depth validation.
#[derive(Debug, Deserialize)]
pub struct ValidatedJsonBody(pub serde_json::Value);

impl Validatable for ValidatedJsonBody {
    fn sanitize(&mut self) {
        super::sanitizers::sanitize_json_value(&mut self.0);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("body", || validate_json_depth(&self.0, MAX_JSON_DEPTH));
        builder.build()
    }
}

impl std::ops::Deref for ValidatedJsonBody {
    type Target = serde_json::Value;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Batch custom metrics request.
#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct RecordMetricsBatchRequest(pub Vec<RecordCustomMetricRequest>);

impl Validatable for RecordMetricsBatchRequest {
    fn sanitize(&mut self) {
        for item in &mut self.0 {
            item.sanitize();
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("metrics", || {
            validate_collection_size(self.0.len(), 1, MAX_BATCH_SIZE)
        });
        for (i, item) in self.0.iter().enumerate() {
            if let Err(errors) = item.validate() {
                for err in errors {
                    builder.add_error(format!("metrics[{i}].{}", err.field), err.message);
                }
            }
        }
        builder.build()
    }
}

impl std::ops::Deref for RecordMetricsBatchRequest {
    type Target = Vec<RecordCustomMetricRequest>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for RecordMetricsBatchRequest {
    type Item = RecordCustomMetricRequest;
    type IntoIter = std::vec::IntoIter<RecordCustomMetricRequest>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

// ── Validator network ────────────────────────────────────────────────────────

impl Validatable for RegisterValidatorRequest {
    fn sanitize(&mut self) {
        self.stellar_address = normalize_stellar_address(&self.stellar_address);
        self.name = sanitize_name(&self.name);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("stellar_address", || {
            validate_stellar_address(&self.stellar_address)
        });
        builder.check("name", || validate_length(&self.name, 1, MAX_NAME_LENGTH));
        builder.check("name", || validate_name_format(&self.name));
        builder.check("name", || validate_no_xss(&self.name));
        builder.check("stake_amount", || {
            validate_positive_decimal(&self.stake_amount)
        });
        builder.build()
    }
}

impl Validatable for SubmitAttestationRequest {
    fn sanitize(&mut self) {
        self.decision = trim(&self.decision).to_lowercase();
        if let Some(ref mut hash) = self.compiled_wasm_hash {
            *hash = trim(hash);
        }
        trim_optional(&mut self.error_message);
        trim_optional(&mut self.signature);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("decision", || {
            validate_one_of(&self.decision, &["valid", "invalid"])
        });
        if let Some(ref hash) = self.compiled_wasm_hash {
            builder.check("compiled_wasm_hash", || validate_wasm_hash(hash));
        }
        validate_optional_text(
            &mut builder,
            "error_message",
            &self.error_message,
            MAX_MESSAGE_LENGTH,
        );
        if let Some(ref sig) = self.signature {
            builder.check("signature", || validate_length(sig, 1, 4096));
        }
        builder.build()
    }
}

// ── Auth ─────────────────────────────────────────────────────────────────────

impl Validatable for auth_handlers::VerifyRequest {
    fn sanitize(&mut self) {
        self.address = normalize_stellar_address(&self.address);
        self.public_key = trim(&self.public_key);
        self.signature = trim(&self.signature);
        self.scopes = self
            .scopes
            .iter()
            .map(|s| trim(s))
            .filter(|s| !s.is_empty())
            .collect();
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("address", || validate_stellar_address(&self.address));
        builder.check("public_key", || validate_hex_length(&self.public_key, 64));
        builder.check("signature", || validate_length(&self.signature, 1, 512));
        builder.check("scopes", || {
            validate_collection_size(self.scopes.len(), 0, 20)
        });
        for (i, scope) in self.scopes.iter().enumerate() {
            builder.check(&format!("scopes[{i}]"), || validate_length(scope, 1, 64));
            builder.check(&format!("scopes[{i}]"), || validate_name_format(scope));
        }
        if let Some(exp) = self.expires_in_seconds {
            builder.check("expires_in_seconds", || {
                if exp == 0 || exp > 86_400 {
                    Err("expires_in_seconds must be between 1 and 86400".to_string())
                } else {
                    Ok(())
                }
            });
        }
        builder.build()
    }
}

// ── Handlers (upload source, etc.) ───────────────────────────────────────────

impl Validatable for UploadContractSourceRequest {
    fn sanitize(&mut self) {
        self.source_base64 = trim(&self.source_base64);
        self.source_format = trim(&self.source_format).to_lowercase();
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("source_base64", || {
            validate_base64_size(&self.source_base64, MAX_SOURCE_CODE_BYTES)
        });
        builder.check("source_format", || {
            validate_one_of(&self.source_format, &["rust", "wasm"])
        });
        builder.build()
    }
}

impl Validatable for RevertVersionRequest {
    fn sanitize(&mut self) {
        trim_optional(&mut self.change_notes);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_optional_text(
            &mut builder,
            "change_notes",
            &self.change_notes,
            MAX_DESCRIPTION_LENGTH,
        );
        builder.build()
    }
}

// ── Organizations ────────────────────────────────────────────────────────────

impl Validatable for CreateOrganizationRequest {
    fn sanitize(&mut self) {
        self.name = sanitize_name(&self.name);
        self.slug = trim(&self.slug).to_lowercase();
        sanitize_description_optional(&mut self.description);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "name", &self.name, 1, MAX_NAME_LENGTH);
        builder.check("name", || validate_name_format(&self.name));
        builder.check("slug", || validate_slug(&self.slug));
        validate_optional_text(
            &mut builder,
            "description",
            &self.description,
            MAX_DESCRIPTION_LENGTH,
        );
        builder.build()
    }
}

impl Validatable for UpdateOrganizationRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut name) = self.name {
            *name = sanitize_name(name);
        }
        sanitize_description_optional(&mut self.description);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        if let Some(ref name) = self.name {
            validate_text(&mut builder, "name", name, 1, MAX_NAME_LENGTH);
            builder.check("name", || validate_name_format(name));
        }
        validate_optional_text(
            &mut builder,
            "description",
            &self.description,
            MAX_DESCRIPTION_LENGTH,
        );
        builder.build()
    }
}

impl Validatable for InviteMemberRequest {
    fn sanitize(&mut self) {
        self.email = trim(&self.email).to_lowercase();
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("email", || validate_email(&self.email));
        builder.build()
    }
}

// ── Incidents ────────────────────────────────────────────────────────────────

impl Validatable for ReportIncidentRequest {
    fn sanitize(&mut self) {
        self.title = sanitize_name(&self.title);
        self.description = trim(&self.description);
        self.reporter = trim(&self.reporter);
        trim_optional(&mut self.assigned_to);
        trim_optional(&mut self.cve_id);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "title", &self.title, 1, 255);
        validate_text(
            &mut builder,
            "description",
            &self.description,
            1,
            MAX_DESCRIPTION_LENGTH,
        );
        validate_text(&mut builder, "reporter", &self.reporter, 1, 255);
        builder.check("affected_contract_ids", || {
            validate_collection_size(self.affected_contract_ids.len(), 0, MAX_BATCH_SIZE)
        });
        validate_optional_text(&mut builder, "assigned_to", &self.assigned_to, 255);
        validate_optional_text(&mut builder, "cve_id", &self.cve_id, 32);
        builder.build()
    }
}

impl Validatable for UpdateIncidentStatusRequest {
    fn sanitize(&mut self) {
        self.author = trim(&self.author);
        self.message = trim(&self.message);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "author", &self.author, 1, 255);
        validate_text(
            &mut builder,
            "message",
            &self.message,
            1,
            MAX_MESSAGE_LENGTH,
        );
        builder.build()
    }
}

impl Validatable for AddIncidentUpdateRequest {
    fn sanitize(&mut self) {
        self.author = trim(&self.author);
        self.message = trim(&self.message);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "author", &self.author, 1, 255);
        validate_text(
            &mut builder,
            "message",
            &self.message,
            1,
            MAX_MESSAGE_LENGTH,
        );
        builder.build()
    }
}

impl Validatable for AddAffectedContractRequest {
    fn sanitize(&mut self) {}

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Ok(())
    }
}

impl Validatable for PublishAdvisoryRequest {
    fn sanitize(&mut self) {
        self.title = sanitize_name(&self.title);
        self.summary = trim(&self.summary);
        self.details = trim(&self.details);
        trim_optional(&mut self.affected_versions);
        trim_optional(&mut self.mitigation);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "title", &self.title, 1, 255);
        validate_text(&mut builder, "summary", &self.summary, 1, 2000);
        validate_text(
            &mut builder,
            "details",
            &self.details,
            1,
            MAX_MESSAGE_LENGTH,
        );
        validate_optional_text(
            &mut builder,
            "affected_versions",
            &self.affected_versions,
            500,
        );
        validate_optional_text(
            &mut builder,
            "mitigation",
            &self.mitigation,
            MAX_DESCRIPTION_LENGTH,
        );
        builder.build()
    }
}

impl Validatable for NotifyAffectedUsersRequest {
    fn sanitize(&mut self) {
        trim_optional(&mut self.channel);
        trim_optional(&mut self.message_template);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("channel", || {
            validate_one_of_optional(&self.channel, &["email", "webhook", "in_app"])
        });
        validate_optional_text(
            &mut builder,
            "message_template",
            &self.message_template,
            MAX_MESSAGE_LENGTH,
        );
        builder.build()
    }
}

// ── Subscriptions & webhooks ─────────────────────────────────────────────────

impl Validatable for SubscribeRequest {
    fn sanitize(&mut self) {}

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Ok(())
    }
}

impl Validatable for UpdateSubscriptionRequest {
    fn sanitize(&mut self) {}

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Ok(())
    }
}

impl Validatable for UpdateUserNotificationPreferencesRequest {
    fn sanitize(&mut self) {
        trim_optional(&mut self.webhook_url);
        trim_optional(&mut self.webhook_secret);
        trim_optional(&mut self.quiet_hours_start);
        trim_optional(&mut self.quiet_hours_end);
        trim_optional(&mut self.timezone);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("webhook_url", || validate_url_optional(&self.webhook_url));
        if let Some(ref secret) = self.webhook_secret {
            builder.check("webhook_secret", || validate_length(secret, 8, 256));
        }
        validate_optional_text(&mut builder, "timezone", &self.timezone, 64);
        builder.build()
    }
}

impl Validatable for CreateWebhookRequest {
    fn sanitize(&mut self) {
        self.name = sanitize_name(&self.name);
        self.url = trim(&self.url);
        trim_optional(&mut self.secret);
        if let Some(ref mut headers) = self.custom_headers {
            super::sanitizers::sanitize_json_value(headers);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "name", &self.name, 1, MAX_NAME_LENGTH);
        builder.check("url", || validate_url_optional(&Some(self.url.clone())));
        builder.check("notification_types", || {
            validate_collection_size(self.notification_types.len(), 1, 20)
        });
        if let Some(ref headers) = self.custom_headers {
            builder.check("custom_headers", || {
                validate_json_depth(headers, MAX_JSON_DEPTH)
            });
        }
        builder.build()
    }
}

// ── Contributors ─────────────────────────────────────────────────────────────

impl Validatable for CreateContributorRequest {
    fn sanitize(&mut self) {
        self.stellar_address = normalize_stellar_address(&self.stellar_address);
        if let Some(ref mut name) = self.name {
            *name = sanitize_name(name);
        }
        sanitize_url_optional(&mut self.avatar_url);
        if let Some(ref mut bio) = self.bio {
            *bio = trim(bio);
        }
        if let Some(ref mut links) = self.links {
            super::sanitizers::sanitize_json_value(links);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("stellar_address", || {
            validate_stellar_address(&self.stellar_address)
        });
        if let Some(ref name) = self.name {
            validate_text(&mut builder, "name", name, 1, MAX_NAME_LENGTH);
        }
        builder.check("avatar_url", || validate_url_optional(&self.avatar_url));
        validate_optional_text(&mut builder, "bio", &self.bio, MAX_DESCRIPTION_LENGTH);
        if let Some(ref links) = self.links {
            builder.check("links", || validate_json_depth(links, MAX_JSON_DEPTH));
        }
        builder.build()
    }
}

impl Validatable for UpdateContributorRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut name) = self.name {
            *name = sanitize_name(name);
        }
        sanitize_url_optional(&mut self.avatar_url);
        if let Some(ref mut bio) = self.bio {
            *bio = trim(bio);
        }
        if let Some(ref mut links) = self.links {
            super::sanitizers::sanitize_json_value(links);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        if let Some(ref name) = self.name {
            validate_text(&mut builder, "name", name, 1, MAX_NAME_LENGTH);
        }
        builder.check("avatar_url", || validate_url_optional(&self.avatar_url));
        validate_optional_text(&mut builder, "bio", &self.bio, MAX_DESCRIPTION_LENGTH);
        if let Some(ref links) = self.links {
            builder.check("links", || validate_json_depth(links, MAX_JSON_DEPTH));
        }
        builder.build()
    }
}

// ── Categories ───────────────────────────────────────────────────────────────

impl Validatable for CreateCategoryRequest {
    fn sanitize(&mut self) {
        self.name = sanitize_name(&self.name);
        sanitize_description_optional(&mut self.description);
        trim_optional(&mut self.parent_id);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "name", &self.name, 1, 100);
        builder.check("name", || validate_name_format(&self.name));
        validate_optional_text(
            &mut builder,
            "description",
            &self.description,
            MAX_DESCRIPTION_LENGTH,
        );
        builder.build()
    }
}

impl Validatable for UpdateCategoryRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut name) = self.name {
            *name = sanitize_name(name);
        }
        sanitize_description_optional(&mut self.description);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        if let Some(ref name) = self.name {
            validate_text(&mut builder, "name", name, 1, 100);
            builder.check("name", || validate_name_format(name));
        }
        validate_optional_text(
            &mut builder,
            "description",
            &self.description,
            MAX_DESCRIPTION_LENGTH,
        );
        builder.build()
    }
}

// ── Verification ─────────────────────────────────────────────────────────────

impl Validatable for ContractVerifyRequest {
    fn sanitize(&mut self) {
        self.source_code = super::sanitizers::sanitize_source_code(&self.source_code);
        self.compiler_version = trim(&self.compiler_version);
        super::sanitizers::sanitize_json_value(&mut self.build_params);
        trim_optional(&mut self.notes);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("source_code", || {
            if self.source_code.trim().is_empty() {
                return Err("source_code is required".to_string());
            }
            validate_source_code_size(&self.source_code, MAX_SOURCE_CODE_BYTES)
        });
        builder.check("compiler_version", || {
            validate_semver(&self.compiler_version)
        });
        builder.check("build_params", || {
            validate_json_depth(&self.build_params, MAX_JSON_DEPTH)
        });
        validate_optional_text(&mut builder, "notes", &self.notes, MAX_DESCRIPTION_LENGTH);
        builder.build()
    }
}

// ── Dependencies ─────────────────────────────────────────────────────────────

impl Validatable for DeclareDependenciesRequest {
    fn sanitize(&mut self) {
        for dep in &mut self.dependencies {
            dep.sanitize();
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("dependencies", || {
            validate_collection_size(self.dependencies.len(), 0, 50)
        });
        for (i, dep) in self.dependencies.iter().enumerate() {
            if let Err(errors) = dep.validate() {
                for err in errors {
                    builder.add_error(format!("dependencies[{i}].{}", err.field), err.message);
                }
            }
        }
        builder.build()
    }
}

// ── Migration registry ───────────────────────────────────────────────────────

impl Validatable for RegisterMigrationRequest {
    fn sanitize(&mut self) {
        self.description = trim(&self.description);
        self.filename = trim(&self.filename);
        if let Some(ref mut down) = self.down_sql {
            *down = trim(down);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("version", || {
            if self.version < 0 {
                Err("version must be non-negative".to_string())
            } else {
                Ok(())
            }
        });
        validate_text(&mut builder, "description", &self.description, 1, 500);
        validate_text(&mut builder, "filename", &self.filename, 1, 255);
        validate_text(
            &mut builder,
            "sql_content",
            &self.sql_content,
            1,
            MAX_MESSAGE_LENGTH,
        );
        if let Some(ref down) = self.down_sql {
            builder.check("down_sql", || validate_length(down, 1, MAX_MESSAGE_LENGTH));
        }
        builder.build()
    }
}

// ── Simulation ───────────────────────────────────────────────────────────────

impl Validatable for SimulateDeployRequest {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        self.wasm_binary = trim(&self.wasm_binary);
        self.name = sanitize_name(&self.name);
        sanitize_description_optional(&mut self.description);
        self.publisher_address = normalize_stellar_address(&self.publisher_address);
        self.tags = sanitize_tags(&self.tags);
        for dep in &mut self.dependencies {
            dep.sanitize();
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("contract_id", || validate_contract_id(&self.contract_id));
        builder.check("wasm_binary", || {
            validate_base64_size(&self.wasm_binary, MAX_SOURCE_CODE_BYTES)
        });
        validate_text(&mut builder, "name", &self.name, 1, MAX_NAME_LENGTH);
        builder.check("publisher_address", || {
            validate_stellar_address(&self.publisher_address)
        });
        builder.check("tags", || {
            validate_tags(&self.tags, MAX_TAGS_COUNT, MAX_TAG_LENGTH)
        });
        builder.build()
    }
}

// ── Batch verify ─────────────────────────────────────────────────────────────

impl Validatable for BatchVerifyItem {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        if let Some(ref mut code) = self.source_code {
            *code = super::sanitizers::sanitize_source_code(code);
        }
        if let Some(ref mut version) = self.compiler_version {
            *version = trim(version);
        }
        if let Some(ref mut params) = self.build_params {
            super::sanitizers::sanitize_json_value(params);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("contract_id", || validate_contract_id(&self.contract_id));
        if let Some(ref code) = self.source_code {
            builder.check("source_code", || {
                validate_source_code_size(code, MAX_SOURCE_CODE_BYTES)
            });
        }
        if let Some(ref version) = self.compiler_version {
            builder.check("compiler_version", || validate_semver(version));
        }
        if let Some(ref params) = self.build_params {
            builder.check("build_params", || {
                validate_json_depth(params, MAX_JSON_DEPTH)
            });
        }
        builder.build()
    }
}

impl Validatable for BatchVerifyRequest {
    fn sanitize(&mut self) {
        for item in &mut self.contracts {
            item.sanitize();
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("contracts", || {
            validate_collection_size(self.contracts.len(), 1, MAX_BATCH_SIZE)
        });
        for (i, item) in self.contracts.iter().enumerate() {
            if let Err(errors) = item.validate() {
                for err in errors {
                    builder.add_error(format!("contracts[{i}].{}", err.field), err.message);
                }
            }
        }
        builder.build()
    }
}

// ── Deprecation ──────────────────────────────────────────────────────────────

impl Validatable for DeprecateContractRequest {
    fn sanitize(&mut self) {
        trim_optional(&mut self.replacement_contract_id);
        sanitize_url_optional(&mut self.migration_guide_url);
        trim_optional(&mut self.notes);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        if let Some(ref id) = self.replacement_contract_id {
            builder.check("replacement_contract_id", || validate_contract_id(id));
        }
        builder.check("migration_guide_url", || {
            validate_url_optional(&self.migration_guide_url)
        });
        validate_optional_text(&mut builder, "notes", &self.notes, MAX_DESCRIPTION_LENGTH);
        builder.build()
    }
}

// ── Reviews ──────────────────────────────────────────────────────────────────

impl Validatable for CreateReviewRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut text) = self.review_text {
            *text = trim(text);
        }
        trim_optional(&mut self.version);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("rating", || validate_rating(self.rating, 0.0, 5.0));
        validate_optional_text(
            &mut builder,
            "review_text",
            &self.review_text,
            MAX_DESCRIPTION_LENGTH,
        );
        if let Some(ref version) = self.version {
            builder.check("version", || validate_semver(version));
        }
        builder.build()
    }
}

impl Validatable for ReviewVoteRequest {
    fn sanitize(&mut self) {}
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Ok(())
    }
}

impl Validatable for FlagReviewRequest {
    fn sanitize(&mut self) {
        self.reason = trim(&self.reason);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "reason", &self.reason, 1, 500);
        builder.build()
    }
}

impl Validatable for ModerateReviewRequest {
    fn sanitize(&mut self) {
        self.action = trim(&self.action).to_lowercase();
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("action", || {
            validate_one_of(&self.action, &["approve", "reject", "hide"])
        });
        builder.build()
    }
}

// ── Favorites ────────────────────────────────────────────────────────────────

impl Validatable for UpdateFavoritesRequest {
    fn sanitize(&mut self) {
        self.favorites = self
            .favorites
            .iter()
            .map(|f| trim(f))
            .filter(|f| !f.is_empty())
            .collect();
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("favorites", || {
            validate_collection_size(self.favorites.len(), 0, 500)
        });
        for (i, fav) in self.favorites.iter().enumerate() {
            builder.check(&format!("favorites[{i}]"), || validate_length(fav, 1, 128));
            builder.check(&format!("favorites[{i}]"), || validate_no_xss(fav));
        }
        builder.build()
    }
}

// ── Publisher verification ───────────────────────────────────────────────────

impl Validatable for PublisherVerifyRequest {
    fn sanitize(&mut self) {
        self.email = trim(&self.email).to_lowercase();
        trim_optional(&mut self.verification_token);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("email", || validate_email(&self.email));
        if let Some(ref token) = self.verification_token {
            builder.check("verification_token", || validate_length(token, 8, 256));
        }
        builder.build()
    }
}

// ── Error reporting ──────────────────────────────────────────────────────────

impl Validatable for ErrorReportRequest {
    fn sanitize(&mut self) {
        trim_optional(&mut self.source);
        trim_optional(&mut self.category);
        trim_optional(&mut self.severity);
        self.message = trim(&self.message);
        trim_optional(&mut self.stack_trace);
        trim_optional(&mut self.route);
        trim_optional(&mut self.request_id);
        trim_optional(&mut self.user_agent);
        super::sanitizers::sanitize_json_value(&mut self.metadata);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(
            &mut builder,
            "message",
            &self.message,
            1,
            MAX_MESSAGE_LENGTH,
        );
        builder.check("metadata", || {
            validate_json_depth(&self.metadata, MAX_JSON_DEPTH)
        });
        builder.build()
    }
}

// ── State monitor ────────────────────────────────────────────────────────────

impl Validatable for ResolveAnomalyRequest {
    fn sanitize(&mut self) {
        trim_optional(&mut self.resolution_notes);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_optional_text(
            &mut builder,
            "resolution_notes",
            &self.resolution_notes,
            MAX_DESCRIPTION_LENGTH,
        );
        builder.build()
    }
}

// ── Compatibility ────────────────────────────────────────────────────────────

impl Validatable for AddCompatibilityRequest {
    fn sanitize(&mut self) {
        self.source_version = trim(&self.source_version);
        self.target_version = trim(&self.target_version);
        trim_optional(&mut self.stellar_version);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("source_version", || validate_semver(&self.source_version));
        builder.check("target_version", || validate_semver(&self.target_version));
        validate_optional_text(&mut builder, "stellar_version", &self.stellar_version, 32);
        builder.build()
    }
}

// ── Patch handlers ───────────────────────────────────────────────────────────

impl Validatable for ReconstructRequest {
    fn sanitize(&mut self) {
        self.target_version = trim(&self.target_version);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("target_version", || validate_semver(&self.target_version));
        builder.build()
    }
}

impl Validatable for BulkTarget {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        self.target_version = trim(&self.target_version);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("contract_id", || validate_contract_id(&self.contract_id));
        builder.check("target_version", || validate_semver(&self.target_version));
        builder.build()
    }
}

impl Validatable for BulkApplyRequest {
    fn sanitize(&mut self) {
        for target in &mut self.targets {
            target.sanitize();
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("targets", || {
            validate_collection_size(self.targets.len(), 1, MAX_BATCH_SIZE)
        });
        for (i, target) in self.targets.iter().enumerate() {
            if let Err(errors) = target.validate() {
                for err in errors {
                    builder.add_error(format!("targets[{i}].{}", err.field), err.message);
                }
            }
        }
        builder.build()
    }
}

// ── RecordCustomMetricRequest (shared, used by batch wrapper) ────────────────

impl Validatable for RecordCustomMetricRequest {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        self.metric_name = trim(&self.metric_name);
        trim_optional(&mut self.unit);
        trim_optional(&mut self.transaction_hash);
        if let Some(ref mut meta) = self.metadata {
            super::sanitizers::sanitize_json_value(meta);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("contract_id", || validate_contract_id(&self.contract_id));
        validate_text(&mut builder, "metric_name", &self.metric_name, 1, 128);
        if let Some(ref meta) = self.metadata {
            builder.check("metadata", || validate_json_depth(meta, MAX_JSON_DEPTH));
        }
        builder.build()
    }
}

// ── Similarity ───────────────────────────────────────────────────────────────

impl Validatable for BatchSimilarityAnalysisRequest {
    fn sanitize(&mut self) {
        self.contract_ids = self
            .contract_ids
            .iter()
            .map(|id| normalize_contract_id(id))
            .collect();
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("contract_ids", || {
            validate_collection_size(self.contract_ids.len(), 1, MAX_BATCH_SIZE)
        });
        for (i, id) in self.contract_ids.iter().enumerate() {
            builder.check(&format!("contract_ids[{i}]"), || validate_contract_id(id));
        }
        if let Some(limit) = self.limit_per_contract {
            builder.check("limit_per_contract", || {
                if limit <= 0 || limit > 100 {
                    Err("limit_per_contract must be between 1 and 100".to_string())
                } else {
                    Ok(())
                }
            });
        }
        builder.build()
    }
}

// ── Gas estimation ───────────────────────────────────────────────────────────

impl Validatable for MethodParamHint {
    fn sanitize(&mut self) {
        self.name = trim(&self.name);
        super::sanitizers::sanitize_json_value(&mut self.value);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "name", &self.name, 1, 128);
        builder.check("value", || validate_json_depth(&self.value, MAX_JSON_DEPTH));
        builder.build()
    }
}

impl Validatable for BatchMethodEntry {
    fn sanitize(&mut self) {
        self.method_name = trim(&self.method_name);
        for param in &mut self.params {
            param.sanitize();
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "method_name", &self.method_name, 1, 255);
        builder.check("params", || {
            validate_collection_size(self.params.len(), 0, 50)
        });
        for (i, param) in self.params.iter().enumerate() {
            if let Err(errors) = param.validate() {
                for err in errors {
                    builder.add_error(format!("params[{i}].{}", err.field), err.message);
                }
            }
        }
        builder.build()
    }
}

impl Validatable for BatchGasEstimateRequest {
    fn sanitize(&mut self) {
        for method in &mut self.methods {
            method.sanitize();
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("methods", || {
            validate_collection_size(self.methods.len(), 1, MAX_BATCH_SIZE)
        });
        for (i, method) in self.methods.iter().enumerate() {
            if let Err(errors) = method.validate() {
                for err in errors {
                    builder.add_error(format!("methods[{i}].{}", err.field), err.message);
                }
            }
        }
        builder.build()
    }
}

// ── Backup & disaster recovery ───────────────────────────────────────────────

impl Validatable for CreateBackupRequest {
    fn sanitize(&mut self) {}
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Ok(())
    }
}

impl Validatable for RestoreBackupRequest {
    fn sanitize(&mut self) {
        self.backup_date = trim(&self.backup_date);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "backup_date", &self.backup_date, 1, 64);
        builder.build()
    }
}

impl Validatable for CreateDisasterRecoveryPlanRequest {
    fn sanitize(&mut self) {
        self.recovery_strategy = trim(&self.recovery_strategy);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("rto_minutes", || {
            if self.rto_minutes <= 0 {
                Err("rto_minutes must be positive".to_string())
            } else {
                Ok(())
            }
        });
        builder.check("rpo_minutes", || {
            if self.rpo_minutes <= 0 {
                Err("rpo_minutes must be positive".to_string())
            } else {
                Ok(())
            }
        });
        validate_text(
            &mut builder,
            "recovery_strategy",
            &self.recovery_strategy,
            1,
            255,
        );
        builder.build()
    }
}

impl Validatable for ExecuteRecoveryRequest {
    fn sanitize(&mut self) {
        trim_optional(&mut self.recovery_target);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_optional_text(&mut builder, "recovery_target", &self.recovery_target, 255);
        builder.build()
    }
}

// ── Canary ───────────────────────────────────────────────────────────────────

impl Validatable for CreateCanaryRequest {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        self.to_deployment_id = trim(&self.to_deployment_id);
        trim_optional(&mut self.created_by);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("contract_id", || validate_contract_id(&self.contract_id));
        validate_text(
            &mut builder,
            "to_deployment_id",
            &self.to_deployment_id,
            1,
            128,
        );
        if let Some(threshold) = self.error_rate_threshold {
            builder.check("error_rate_threshold", || validate_percentage(threshold));
        }
        builder.build()
    }
}

impl Validatable for RecordCanaryMetricRequest {
    fn sanitize(&mut self) {
        self.canary_id = trim(&self.canary_id);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "canary_id", &self.canary_id, 1, 128);
        builder.check("requests", || {
            if self.requests < 0 {
                Err("requests must be non-negative".to_string())
            } else {
                Ok(())
            }
        });
        builder.build()
    }
}

// AdvanceCanaryRequest imported from shared above

impl Validatable for AdvanceCanaryRequest {
    fn sanitize(&mut self) {
        self.canary_id = trim(&self.canary_id);
        trim_optional(&mut self.advanced_by);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "canary_id", &self.canary_id, 1, 128);
        if let Some(pct) = self.target_percentage {
            builder.check("target_percentage", || {
                if pct < 0 || pct > 100 {
                    Err("target_percentage must be between 0 and 100".to_string())
                } else {
                    Ok(())
                }
            });
        }
        builder.build()
    }
}

// ── Federation / clone ───────────────────────────────────────────────────────

impl Validatable for CloneContractRequest {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        if let Some(ref mut name) = self.name {
            *name = sanitize_name(name);
        }
        sanitize_description_optional(&mut self.description);
        if let Some(ref mut hash) = self.wasm_hash {
            *hash = trim(hash);
        }
        if let Some(ref mut tags) = self.tags {
            *tags = sanitize_tags(tags);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("contract_id", || validate_contract_id(&self.contract_id));
        if let Some(ref name) = self.name {
            validate_text(&mut builder, "name", name, 1, MAX_NAME_LENGTH);
        }
        if let Some(ref hash) = self.wasm_hash {
            builder.check("wasm_hash", || validate_wasm_hash(hash));
        }
        builder.build()
    }
}

impl Validatable for RegisterFederatedRegistryRequest {
    fn sanitize(&mut self) {
        self.name = sanitize_name(&self.name);
        self.base_url = trim(&self.base_url);
        trim_optional(&mut self.public_key);
        trim_optional(&mut self.federation_protocol_version);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "name", &self.name, 1, MAX_NAME_LENGTH);
        builder.check("base_url", || {
            validate_url_optional(&Some(self.base_url.clone()))
        });
        builder.build()
    }
}

impl Validatable for SyncFederatedRegistryRequest {
    fn sanitize(&mut self) {}

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("batch_size", || {
            if self.batch_size <= 0 || self.batch_size > 1000 {
                Err("batch_size must be between 1 and 1000".to_string())
            } else {
                Ok(())
            }
        });
        builder.build()
    }
}

impl Validatable for FederationOptRequest {
    fn sanitize(&mut self) {}

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        if let Some(ref filters) = self.registry_filters {
            builder.check("registry_filters", || {
                validate_collection_size(filters.len(), 0, MAX_BATCH_SIZE)
            });
        }
        builder.build()
    }
}

// ── Collaborative reviews ──────────────────────────────────────────────────────

impl Validatable for CreateCollaborativeReviewRequest {
    fn sanitize(&mut self) {
        self.version = trim(&self.version);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("version", || validate_semver(&self.version));
        builder.check("reviewer_ids", || {
            validate_collection_size(self.reviewer_ids.len(), 1, 20)
        });
        builder.build()
    }
}

impl Validatable for AddCollaborativeCommentRequest {
    fn sanitize(&mut self) {
        self.content = trim(&self.content);
        trim_optional(&mut self.file_path);
        trim_optional(&mut self.abi_path);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(
            &mut builder,
            "content",
            &self.content,
            1,
            MAX_MESSAGE_LENGTH,
        );
        validate_optional_text(&mut builder, "file_path", &self.file_path, 512);
        builder.build()
    }
}

impl Validatable for UpdateReviewerStatusRequest {
    fn sanitize(&mut self) {}
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Ok(())
    }
}

// ── AB tests ─────────────────────────────────────────────────────────────────

impl Validatable for CreateAbTestRequest {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        self.name = sanitize_name(&self.name);
        sanitize_description_optional(&mut self.description);
        self.variant_a_deployment_id = trim(&self.variant_a_deployment_id);
        self.variant_b_deployment_id = trim(&self.variant_b_deployment_id);
        self.primary_metric = trim(&self.primary_metric);
        trim_optional(&mut self.hypothesis);
        trim_optional(&mut self.created_by);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("contract_id", || validate_contract_id(&self.contract_id));
        validate_text(&mut builder, "name", &self.name, 1, MAX_NAME_LENGTH);
        validate_text(&mut builder, "primary_metric", &self.primary_metric, 1, 128);
        if let Some(split) = self.traffic_split {
            builder.check("traffic_split", || validate_percentage(split));
        }
        builder.build()
    }
}

impl Validatable for RecordAbTestMetricRequest {
    fn sanitize(&mut self) {
        self.test_id = trim(&self.test_id);
        self.metric_name = trim(&self.metric_name);
        trim_optional(&mut self.user_address);
        if let Some(ref mut meta) = self.metadata {
            super::sanitizers::sanitize_json_value(meta);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "test_id", &self.test_id, 1, 128);
        validate_text(&mut builder, "metric_name", &self.metric_name, 1, 128);
        if let Some(ref addr) = self.user_address {
            builder.check("user_address", || validate_stellar_address(addr));
        }
        builder.build()
    }
}

// ── Performance ──────────────────────────────────────────────────────────────

impl Validatable for RecordPerformanceMetricRequest {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        trim_optional(&mut self.function_name);
        if let Some(ref mut meta) = self.metadata {
            super::sanitizers::sanitize_json_value(meta);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("contract_id", || validate_contract_id(&self.contract_id));
        if let Some(ref meta) = self.metadata {
            builder.check("metadata", || validate_json_depth(meta, MAX_JSON_DEPTH));
        }
        builder.build()
    }
}

impl Validatable for RecordPerformanceBenchmarkRequest {
    fn sanitize(&mut self) {
        trim_optional(&mut self.contract_id);
        trim_optional(&mut self.contract_version_id);
        self.benchmark_name = trim(&self.benchmark_name);
        trim_optional(&mut self.source);
        if let Some(ref mut meta) = self.metadata {
            super::sanitizers::sanitize_json_value(meta);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "benchmark_name", &self.benchmark_name, 1, 255);
        if let Some(ref id) = self.contract_id {
            builder.check("contract_id", || validate_contract_id(id));
        }
        builder.build()
    }
}

impl Validatable for CreateAlertConfigRequest {
    fn sanitize(&mut self) {
        self.contract_id = normalize_contract_id(&self.contract_id);
        self.threshold_type = trim(&self.threshold_type);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("contract_id", || validate_contract_id(&self.contract_id));
        builder.check("threshold_type", || {
            validate_one_of(&self.threshold_type, &["above", "below", "equals"])
        });
        builder.build()
    }
}

// ── Security scan ────────────────────────────────────────────────────────────

impl Validatable for TriggerSecurityScanRequest {
    fn sanitize(&mut self) {
        trim_optional(&mut self.version);
        trim_optional(&mut self.scan_type);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        if let Some(ref version) = self.version {
            builder.check("version", || validate_semver(version));
        }
        validate_optional_text(&mut builder, "scan_type", &self.scan_type, 64);
        builder.build()
    }
}

impl Validatable for UpdateSecurityIssueRequest {
    fn sanitize(&mut self) {
        trim_optional(&mut self.notes);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_optional_text(&mut builder, "notes", &self.notes, MAX_DESCRIPTION_LENGTH);
        builder.build()
    }
}

impl Validatable for CreateSecurityScannerRequest {
    fn sanitize(&mut self) {
        self.name = sanitize_name(&self.name);
        sanitize_description_optional(&mut self.description);
        self.scanner_type = trim(&self.scanner_type);
        sanitize_url_optional(&mut self.api_endpoint);
        trim_optional(&mut self.api_key);
        if let Some(ref mut config) = self.configuration {
            super::sanitizers::sanitize_json_value(config);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "name", &self.name, 1, MAX_NAME_LENGTH);
        validate_text(&mut builder, "scanner_type", &self.scanner_type, 1, 64);
        builder.check("api_endpoint", || validate_url_optional(&self.api_endpoint));
        if let Some(ref config) = self.configuration {
            builder.check("configuration", || {
                validate_json_depth(config, MAX_JSON_DEPTH)
            });
        }
        builder.build()
    }
}

// ── Release notes ──────────────────────────────────────────────────────────────

impl Validatable for GenerateReleaseNotesRequest {
    fn sanitize(&mut self) {
        self.version = trim(&self.version);
        trim_optional(&mut self.previous_version);
        trim_optional(&mut self.changelog_content);
        trim_optional(&mut self.contract_address);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("version", || validate_semver(&self.version));
        if let Some(ref prev) = self.previous_version {
            builder.check("previous_version", || validate_semver(prev));
        }
        builder.build()
    }
}

impl Validatable for UpdateReleaseNotesRequest {
    fn sanitize(&mut self) {
        self.notes_text = trim(&self.notes_text);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(
            &mut builder,
            "notes_text",
            &self.notes_text,
            1,
            MAX_MESSAGE_LENGTH,
        );
        builder.build()
    }
}

impl Validatable for PublishReleaseNotesRequest {
    fn sanitize(&mut self) {}
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Ok(())
    }
}

// ── ABI versioning ───────────────────────────────────────────────────────────

impl Validatable for PublishAbiRequest {
    fn sanitize(&mut self) {
        self.version = trim(&self.version);
        super::sanitizers::sanitize_json_value(&mut self.abi);
        trim_optional(&mut self.changelog);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("version", || validate_semver(&self.version));
        builder.check("abi", || validate_json_depth(&self.abi, MAX_JSON_DEPTH));
        validate_optional_text(
            &mut builder,
            "changelog",
            &self.changelog,
            MAX_MESSAGE_LENGTH,
        );
        builder.build()
    }
}

impl Validatable for CheckCompatibilityRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut v) = self.base_version {
            *v = trim(v);
        }
        if let Some(ref mut v) = self.new_version {
            *v = trim(v);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        if let Some(ref v) = self.base_version {
            builder.check("base_version", || validate_semver(v));
        }
        if let Some(ref v) = self.new_version {
            builder.check("new_version", || validate_semver(v));
        }
        builder.build()
    }
}

// ── Formal verification ──────────────────────────────────────────────────────

impl Validatable for TriggerVerificationRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut v) = self.version {
            *v = trim(v);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        if let Some(ref v) = self.version {
            builder.check("version", || validate_semver(v));
        }
        builder.build()
    }
}

// ── Compatibility testing ────────────────────────────────────────────────────

impl Validatable for RunCompatibilityTestRequest {
    fn sanitize(&mut self) {
        self.sdk_version = trim(&self.sdk_version);
        self.wasm_runtime = trim(&self.wasm_runtime);
        self.network = trim(&self.network);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "sdk_version", &self.sdk_version, 1, 64);
        validate_text(&mut builder, "wasm_runtime", &self.wasm_runtime, 1, 64);
        validate_text(&mut builder, "network", &self.network, 1, 32);
        builder.build()
    }
}

// ── Governance ───────────────────────────────────────────────────────────────

impl Validatable for CreateProposalRequest {
    fn sanitize(&mut self) {
        self.title = sanitize_name(&self.title);
        self.description = trim(&self.description);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "title", &self.title, 1, 255);
        validate_text(
            &mut builder,
            "description",
            &self.description,
            1,
            MAX_MESSAGE_LENGTH,
        );
        builder.build()
    }
}

impl Validatable for CastVoteRequest {
    fn sanitize(&mut self) {}
    fn validate(&self) -> Result<(), Vec<FieldError>> {
        Ok(())
    }
}

impl Validatable for UpsertVotingRightsRequest {
    fn sanitize(&mut self) {
        trim_optional(&mut self.source);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("voting_power", || {
            if self.voting_power < 0 {
                Err("voting_power must be non-negative".to_string())
            } else {
                Ok(())
            }
        });
        validate_optional_text(&mut builder, "source", &self.source, 255);
        builder.build()
    }
}

// ── Multisig ─────────────────────────────────────────────────────────────────

impl Validatable for CreateMultisigPolicyRequest {
    fn sanitize(&mut self) {
        self.name = sanitize_name(&self.name);
        self.created_by = normalize_stellar_address(&self.created_by);
        self.signer_addresses = self
            .signer_addresses
            .iter()
            .map(|a| normalize_stellar_address(a))
            .collect();
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "name", &self.name, 1, MAX_NAME_LENGTH);
        builder.check("threshold", || {
            if self.threshold <= 0 {
                Err("threshold must be positive".to_string())
            } else if self.threshold as usize > self.signer_addresses.len() {
                Err("threshold cannot exceed number of signers".to_string())
            } else {
                Ok(())
            }
        });
        builder.check("signer_addresses", || {
            validate_collection_size(self.signer_addresses.len(), 1, 20)
        });
        for (i, addr) in self.signer_addresses.iter().enumerate() {
            builder.check(&format!("signer_addresses[{i}]"), || {
                validate_stellar_address(addr)
            });
        }
        builder.check("created_by", || validate_stellar_address(&self.created_by));
        builder.build()
    }
}

impl Validatable for CreateDeployProposalRequest {
    fn sanitize(&mut self) {
        self.contract_name = sanitize_name(&self.contract_name);
        self.contract_id = normalize_contract_id(&self.contract_id);
        self.wasm_hash = trim(&self.wasm_hash);
        sanitize_description_optional(&mut self.description);
        self.proposer = normalize_stellar_address(&self.proposer);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(
            &mut builder,
            "contract_name",
            &self.contract_name,
            1,
            MAX_NAME_LENGTH,
        );
        builder.check("contract_id", || validate_contract_id(&self.contract_id));
        builder.check("wasm_hash", || validate_wasm_hash(&self.wasm_hash));
        builder.check("proposer", || validate_stellar_address(&self.proposer));
        builder.build()
    }
}

impl Validatable for SignProposalRequest {
    fn sanitize(&mut self) {
        self.signer_address = normalize_stellar_address(&self.signer_address);
        trim_optional(&mut self.signature_data);
        trim_optional(&mut self.comment);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("signer_address", || {
            validate_stellar_address(&self.signer_address)
        });
        validate_optional_text(
            &mut builder,
            "comment",
            &self.comment,
            MAX_DESCRIPTION_LENGTH,
        );
        builder.build()
    }
}

impl Validatable for CreatePublisherKeyRequest {
    fn sanitize(&mut self) {
        self.key_name = sanitize_name(&self.key_name);
        self.public_key = trim(&self.public_key);
        trim_optional(&mut self.algorithm);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "key_name", &self.key_name, 1, 128);
        builder.check("public_key", || validate_length(&self.public_key, 32, 128));
        builder.build()
    }
}

// ── AI handlers ──────────────────────────────────────────────────────────────

impl Validatable for ChatMessage {
    fn sanitize(&mut self) {
        self.role = trim(&self.role).to_lowercase();
        self.content = trim(&self.content);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("role", || {
            validate_one_of(&self.role, &["user", "assistant", "system"])
        });
        validate_text(
            &mut builder,
            "content",
            &self.content,
            1,
            MAX_MESSAGE_LENGTH,
        );
        builder.build()
    }
}

impl Validatable for ChatRequest {
    fn sanitize(&mut self) {
        trim_optional(&mut self.model);
        for msg in &mut self.messages {
            msg.sanitize();
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("messages", || {
            validate_collection_size(self.messages.len(), 1, 50)
        });
        for (i, msg) in self.messages.iter().enumerate() {
            if let Err(errors) = msg.validate() {
                for err in errors {
                    builder.add_error(format!("messages[{i}].{}", err.field), err.message);
                }
            }
        }
        validate_optional_text(&mut builder, "model", &self.model, 64);
        builder.build()
    }
}

impl Validatable for SuggestRequest {
    fn sanitize(&mut self) {
        self.request = trim(&self.request);
        trim_optional(&mut self.context);
        trim_optional(&mut self.model);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(
            &mut builder,
            "request",
            &self.request,
            1,
            MAX_MESSAGE_LENGTH,
        );
        validate_optional_text(&mut builder, "context", &self.context, MAX_MESSAGE_LENGTH);
        builder.build()
    }
}

// ── Analytics & observability ────────────────────────────────────────────────

impl Validatable for WebVitalMetric {
    fn sanitize(&mut self) {
        self.id = trim(&self.id);
        self.name = trim(&self.name);
        trim_optional(&mut self.rating);
        trim_optional(&mut self.navigation_type);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "id", &self.id, 1, 128);
        validate_text(&mut builder, "name", &self.name, 1, 64);
        builder.check("value", || {
            if !self.value.is_finite() {
                Err("value must be a finite number".to_string())
            } else {
                Ok(())
            }
        });
        builder.build()
    }
}

impl Validatable for ClientBreakerReport {
    fn sanitize(&mut self) {
        self.endpoint = trim(&self.endpoint);
        self.state = trim(&self.state).to_lowercase();
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "endpoint", &self.endpoint, 1, 512);
        builder.check("state", || {
            validate_one_of(&self.state, &["open", "closed", "half_open"])
        });
        builder.build()
    }
}

// ── Notifications (disaster recovery models) ─────────────────────────────────

impl Validatable for CreateNotificationTemplateRequest {
    fn sanitize(&mut self) {
        self.name = sanitize_name(&self.name);
        self.subject = trim(&self.subject);
        self.message_template = trim(&self.message_template);
        self.channel = trim(&self.channel);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "name", &self.name, 1, MAX_NAME_LENGTH);
        validate_text(&mut builder, "subject", &self.subject, 1, 255);
        validate_text(
            &mut builder,
            "message_template",
            &self.message_template,
            1,
            MAX_MESSAGE_LENGTH,
        );
        builder.check("channel", || {
            validate_one_of(&self.channel, &["email", "webhook", "in_app"])
        });
        builder.build()
    }
}

impl Validatable for CreateUserNotificationPreferenceRequest {
    fn sanitize(&mut self) {
        for t in &mut self.notification_types {
            *t = trim(t);
        }
        for c in &mut self.channels {
            *c = trim(c);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        builder.check("notification_types", || {
            validate_collection_size(self.notification_types.len(), 1, 20)
        });
        builder.check("channels", || {
            validate_collection_size(self.channels.len(), 1, 10)
        });
        builder.build()
    }
}

impl Validatable for SendNotificationRequest {
    fn sanitize(&mut self) {
        self.notification_type = trim(&self.notification_type);
        trim_optional(&mut self.priority);
        for (_, v) in self.template_variables.iter_mut() {
            *v = trim(v);
        }
        self.recipients = self.recipients.iter().map(|r| trim(r)).collect();
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(
            &mut builder,
            "notification_type",
            &self.notification_type,
            1,
            64,
        );
        builder.check("recipients", || {
            validate_collection_size(self.recipients.len(), 1, MAX_BATCH_SIZE)
        });
        builder.build()
    }
}

// ── Post-incident ────────────────────────────────────────────────────────────

impl Validatable for CreateActionItemRequest {
    fn sanitize(&mut self) {
        self.description = trim(&self.description);
        self.owner = trim(&self.owner);
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(
            &mut builder,
            "description",
            &self.description,
            1,
            MAX_DESCRIPTION_LENGTH,
        );
        validate_text(&mut builder, "owner", &self.owner, 1, 255);
        builder.build()
    }
}

impl Validatable for CreatePostIncidentReportRequest {
    fn sanitize(&mut self) {
        self.title = sanitize_name(&self.title);
        self.description = trim(&self.description);
        self.root_cause = trim(&self.root_cause);
        self.impact_assessment = trim(&self.impact_assessment);
        self.created_by = trim(&self.created_by);
        for step in &mut self.recovery_steps {
            *step = trim(step);
        }
        for lesson in &mut self.lessons_learned {
            *lesson = trim(lesson);
        }
        for item in &mut self.action_items {
            item.sanitize();
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        validate_text(&mut builder, "title", &self.title, 1, 255);
        validate_text(
            &mut builder,
            "description",
            &self.description,
            1,
            MAX_MESSAGE_LENGTH,
        );
        validate_text(
            &mut builder,
            "root_cause",
            &self.root_cause,
            1,
            MAX_MESSAGE_LENGTH,
        );
        validate_text(
            &mut builder,
            "impact_assessment",
            &self.impact_assessment,
            1,
            MAX_MESSAGE_LENGTH,
        );
        validate_text(&mut builder, "created_by", &self.created_by, 1, 255);
        builder.build()
    }
}

// ── Mutation testing ─────────────────────────────────────────────────────────

impl Validatable for RunMutationTestRequest {
    fn sanitize(&mut self) {
        if let Some(ref mut abi) = self.abi {
            super::sanitizers::sanitize_json_value(abi);
        }
    }

    fn validate(&self) -> Result<(), Vec<FieldError>> {
        let mut builder = ValidationBuilder::new();
        if let Some(ref abi) = self.abi {
            builder.check("abi", || validate_json_depth(abi, MAX_JSON_DEPTH));
        }
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn register_validator_request_validates_stellar_address() {
        let req = RegisterValidatorRequest {
            stellar_address: "invalid".to_string(),
            name: "Validator 1".to_string(),
            stake_amount: Decimal::new(100, 0),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn batch_contract_ids_rejects_oversized() {
        let req = BatchContractIdsRequest(vec!["x".repeat(60); 101]);
        assert!(req.validate().is_err());
    }

    #[test]
    fn validation_error_includes_field_errors() {
        let req = RegisterValidatorRequest {
            stellar_address: "bad".to_string(),
            name: "".to_string(),
            stake_amount: Decimal::ZERO,
        };
        let errors = req.validate().unwrap_err();
        assert!(!errors.is_empty());
        assert!(errors
            .iter()
            .any(|e| e.field == "stellar_address" || e.field == "name"));
    }
}
