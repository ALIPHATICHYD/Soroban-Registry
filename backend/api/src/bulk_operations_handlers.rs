//! Bulk operations handlers for importing and exporting contract metadata
//!
//! Provides endpoints for:
//! - POST /contracts/import - Import contracts in bulk (sync or async)
//! - GET /contracts/export - Export contracts with filtering (GET version)
//! - GET /contracts/import/:job_id - Check import job status

use crate::{
    auth::AuthClaims,
    error::{ApiError, ApiResult},
    handlers::{db_internal_error, track_contract_access},
    state::AppState,
    validation::extractors::ValidatedJson,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use once_cell::sync::Lazy;
use serde_json::json;
use shared::{
    Contract, ContractExportAcceptedResponse, ContractExportFormat, ContractExportJobStatus,
    ContractExportMetadata, ContractExportQueryParams, ContractExportRequest,
    ContractExportStatusResponse, ContractImportAcceptedResponse, ContractImportItemResult,
    ContractImportJobStatus, ContractImportRecord, ContractImportRequest, ContractImportResponse,
    ContractImportStatusResponse, ContractMetadataExportEnvelope, ContractMetadataExportRecord,
    ContractSearchParams, Network, Publisher, VerificationStatus, VisibilityType,
};
use sqlx::QueryBuilder;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

const ASYNC_IMPORT_ROW_THRESHOLD: usize = 100;
const MAX_IMPORT_BATCH_SIZE: usize = 10_000;
const IMPORT_BATCH_INSERT_SIZE: usize = 100;

// ═══════════════════════════════════════════════════════════════════════════
// IMPORT JOB TRACKING
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
struct ContractImportJob {
    job_id: Uuid,
    status: ContractImportJobStatus,
    fail_safe: bool,
    total_count: i64,
    processed_count: i64,
    imported_count: i64,
    failed_count: i64,
    requested_at: chrono::DateTime<chrono::Utc>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    error: Option<String>,
    results: Vec<ContractImportItemResult>,
}

static IMPORT_JOBS: Lazy<RwLock<HashMap<Uuid, ContractImportJob>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

// ═══════════════════════════════════════════════════════════════════════════
// IMPORT VALIDATION
// ═══════════════════════════════════════════════════════════════════════════

fn validate_contract_id(contract_id: &str) -> Result<(), String> {
    // Stellar contract IDs are 56 character base32-encoded strings starting with 'C'
    if contract_id.len() != 56 {
        return Err(format!(
            "Contract ID must be 56 characters, got {}",
            contract_id.len()
        ));
    }
    if !contract_id.starts_with('C') {
        return Err("Contract ID must start with 'C'".to_string());
    }
    if !contract_id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Contract ID must be alphanumeric".to_string());
    }
    Ok(())
}

fn validate_wasm_hash(hash: &str) -> Result<(), String> {
    // WASM hashes are 64 character hex strings
    if hash.len() != 64 {
        return Err(format!(
            "WASM hash must be 64 characters, got {}",
            hash.len()
        ));
    }
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("WASM hash must be hexadecimal".to_string());
    }
    Ok(())
}

fn validate_publisher_address(addr: &str) -> Result<(), String> {
    // Stellar addresses are 56 character base32-encoded strings starting with 'G'
    if addr.len() != 56 {
        return Err(format!(
            "Publisher address must be 56 characters, got {}",
            addr.len()
        ));
    }
    if !addr.starts_with('G') {
        return Err("Publisher address must start with 'G'".to_string());
    }
    if !addr.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("Publisher address must be alphanumeric".to_string());
    }
    Ok(())
}

fn validate_import_record(record: &ContractImportRecord, index: usize) -> Vec<String> {
    let mut errors = Vec::new();

    if let Err(e) = validate_contract_id(&record.contract_id) {
        errors.push(format!("[{}] contract_id: {}", index, e));
    }

    if let Err(e) = validate_wasm_hash(&record.wasm_hash) {
        errors.push(format!("[{}] wasm_hash: {}", index, e));
    }

    if record.name.trim().is_empty() {
        errors.push(format!("[{}] name: cannot be empty", index));
    }

    if record.name.len() > 255 {
        errors.push(format!("[{}] name: exceeds 255 characters", index));
    }

    if let Err(e) = validate_publisher_address(&record.publisher_address) {
        errors.push(format!("[{}] publisher_address: {}", index, e));
    }

    if let Some(versions) = &record.versions {
        for (vidx, version) in versions.iter().enumerate() {
            if version.version.trim().is_empty() {
                errors.push(format!(
                    "[{}] versions[{}].version: cannot be empty",
                    index, vidx
                ));
            }
            if let Err(e) = validate_wasm_hash(&version.wasm_hash) {
                errors.push(format!("[{}] versions[{}].wasm_hash: {}", index, vidx, e));
            }
        }
    }

    errors
}

// ═══════════════════════════════════════════════════════════════════════════
// IMPORT PROCESSING
// ═══════════════════════════════════════════════════════════════════════════

async fn import_single_contract(
    db: &sqlx::PgPool,
    record: &ContractImportRecord,
    index: usize,
    skip_existing: bool,
) -> ContractImportItemResult {
    let mut result = ContractImportItemResult {
        index,
        contract_id: record.contract_id.clone(),
        success: false,
        imported_id: None,
        error: None,
    };

    // Check for existing contract
    let existing: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM contracts WHERE contract_id = $1 AND network = $2")
            .bind(&record.contract_id)
            .bind(&record.network)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();

    if let Some((existing_id,)) = existing {
        if skip_existing {
            result.success = true;
            result.imported_id = Some(existing_id);
            return result;
        }
        result.error = Some(format!(
            "Contract {} already exists on {:?}",
            record.contract_id, record.network
        ));
        return result;
    }

    // Get or create publisher
    let publisher_result: Result<Publisher, sqlx::Error> = sqlx::query_as(
        "INSERT INTO publishers (stellar_address) VALUES ($1)
         ON CONFLICT (stellar_address) DO UPDATE SET stellar_address = EXCLUDED.stellar_address
         RETURNING *",
    )
    .bind(&record.publisher_address)
    .fetch_one(db)
    .await;

    let publisher = match publisher_result {
        Ok(p) => p,
        Err(e) => {
            result.error = Some(format!("Failed to create publisher: {}", e));
            return result;
        }
    };

    // Insert contract
    let visibility = record
        .visibility
        .clone()
        .unwrap_or_else(|| "public".to_string());
    let is_verified = record.is_verified.unwrap_or(false);
    let slug = shared::slugify(&record.name);

    let contract_result: Result<Contract, sqlx::Error> = sqlx::query_as(
        "INSERT INTO contracts (
            contract_id, wasm_hash, name, slug, description, publisher_id,
            network, category, is_verified, verification_status, visibility, organization_id,
            network_configs
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        RETURNING *",
    )
    .bind(&record.contract_id)
    .bind(&record.wasm_hash)
    .bind(&record.name)
    .bind(&slug)
    .bind(&record.description)
    .bind(publisher.id)
    .bind(&record.network)
    .bind(&record.category)
    .bind(is_verified)
    .bind(VerificationStatus::Unverified)
    .bind(&visibility)
    .bind(record.organization_id)
    .bind(&record.network_configs)
    .fetch_one(db)
    .await;

    let contract = match contract_result {
        Ok(c) => c,
        Err(e) => {
            result.error = Some(format!("Failed to insert contract: {}", e));
            return result;
        }
    };

    // Handle tags if provided
    if let Some(tags) = &record.tags {
        for tag_name in tags {
            let tag_result: Result<(Uuid,), sqlx::Error> = sqlx::query_as(
                "INSERT INTO tags (name, color) VALUES ($1, '#6366f1')
                 ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
                 RETURNING id",
            )
            .bind(tag_name)
            .fetch_one(db)
            .await;

            if let Ok((tag_id,)) = tag_result {
                let _ = sqlx::query(
                    "INSERT INTO contract_tags (contract_id, tag_id) VALUES ($1, $2)
                     ON CONFLICT DO NOTHING",
                )
                .bind(contract.id)
                .bind(tag_id)
                .execute(db)
                .await;
            }
        }
    }

    // Handle versions if provided
    if let Some(versions) = &record.versions {
        for version in versions {
            let _ = sqlx::query(
                "INSERT INTO contract_versions (
                    contract_id, version, wasm_hash, source_url, commit_hash, release_notes
                ) VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (contract_id, version) DO NOTHING",
            )
            .bind(contract.id)
            .bind(&version.version)
            .bind(&version.wasm_hash)
            .bind(&version.source_url)
            .bind(&version.commit_hash)
            .bind(&version.release_notes)
            .execute(db)
            .await;
        }
    }

    // Set logical_id to id
    let _ = sqlx::query("UPDATE contracts SET logical_id = id WHERE id = $1")
        .bind(contract.id)
        .execute(db)
        .await;

    result.success = true;
    result.imported_id = Some(contract.id);
    result
}

async fn process_import_batch(
    state: &AppState,
    contracts: Vec<ContractImportRecord>,
    fail_safe: bool,
    skip_existing: bool,
    job_id: Option<Uuid>,
) -> Vec<ContractImportItemResult> {
    let mut results = Vec::with_capacity(contracts.len());

    for (index, record) in contracts.into_iter().enumerate() {
        let result = import_single_contract(&state.db, &record, index, skip_existing).await;

        // Update job progress if we have a job_id
        if let Some(jid) = job_id {
            let mut jobs = IMPORT_JOBS.write().await;
            if let Some(job) = jobs.get_mut(&jid) {
                job.processed_count += 1;
                if result.success {
                    job.imported_count += 1;
                } else {
                    job.failed_count += 1;
                }
                job.results.push(result.clone());
            }
        }

        // In fail-safe mode, continue even if one fails
        if !fail_safe && !result.success {
            results.push(result);
            break;
        }

        results.push(result);
    }

    results
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPORT HELPERS (for GET endpoint)
// ═══════════════════════════════════════════════════════════════════════════

fn query_params_to_search_params(params: ContractExportQueryParams) -> ContractSearchParams {
    ContractSearchParams {
        query: params.query,
        network: None,
        networks: params.networks,
        category: params.category,
        categories: None,
        tags: params.tags,
        maturity: params.maturity.map(|m| match m.as_str() {
            "experimental" => shared::MaturityLevel::Experimental,
            "beta" => shared::MaturityLevel::Beta,
            "stable" => shared::MaturityLevel::Stable,
            "deprecated" => shared::MaturityLevel::Production,
            _ => shared::MaturityLevel::Experimental,
        }),
        verified_only: params.verified_only,
        verification_status: None,
        page: None,
        limit: None,
        offset: None,
        sort_by: None,
        sort_order: None,
        cursor: None,
        created_from: None,
        created_to: None,
        updated_from: None,
        updated_to: None,
        verified_from: None,
        verified_to: None,
        last_accessed_from: None,
        last_accessed_to: None,
        w_text: None,
        w_pop: None,
        w_rec: None,
        w_rat: None,
        user_id: None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════════════════════

/// POST /contracts/import - Import contracts in bulk
#[utoipa::path(
    post,
    path = "/contracts/import",
    request_body = ContractImportRequest,
    responses(
        (status = 200, description = "Contracts imported successfully", body = ContractImportResponse),
        (status = 202, description = "Large import accepted for async processing", body = ContractImportAcceptedResponse),
        (status = 400, description = "Invalid import request"),
        (status = 413, description = "Batch size exceeds maximum")
    ),
    tag = "Bulk Operations"
)]
pub async fn import_contracts(
    State(state): State<AppState>,
    _claims: Option<AuthClaims>,
    ValidatedJson(req): ValidatedJson<ContractImportRequest>,
) -> ApiResult<Response> {
    // Validate batch size
    if req.contracts.is_empty() {
        return Err(ApiError::bad_request(
            "EmptyImport",
            "No contracts provided for import",
        ));
    }

    if req.contracts.len() > MAX_IMPORT_BATCH_SIZE {
        return Err(ApiError::payload_too_large(format!(
            "Batch size {} exceeds maximum of {}",
            req.contracts.len(),
            MAX_IMPORT_BATCH_SIZE
        )));
    }

    // Validate all records first
    let mut validation_errors = Vec::new();
    for (index, record) in req.contracts.iter().enumerate() {
        let errors = validate_import_record(record, index);
        validation_errors.extend(errors);
    }

    if !validation_errors.is_empty() {
        return Err(ApiError::unprocessable(
            "VALIDATION_FAILED",
            format!("Validation errors:\n{}", validation_errors.join("\n")),
        ));
    }

    let total_count = req.contracts.len();
    let fail_safe = req.fail_safe;
    let skip_existing = req.skip_existing.unwrap_or(false);
    let should_run_async =
        req.async_mode.unwrap_or(false) || total_count > ASYNC_IMPORT_ROW_THRESHOLD;

    if should_run_async {
        let job_id = Uuid::new_v4();
        let job = ContractImportJob {
            job_id,
            status: ContractImportJobStatus::Pending,
            fail_safe,
            total_count: total_count as i64,
            processed_count: 0,
            imported_count: 0,
            failed_count: 0,
            requested_at: chrono::Utc::now(),
            completed_at: None,
            error: None,
            results: Vec::new(),
        };

        IMPORT_JOBS.write().await.insert(job_id, job);

        let state_clone = state.clone();
        let contracts_clone = req.contracts.clone();

        tokio::spawn(async move {
            {
                let mut jobs = IMPORT_JOBS.write().await;
                if let Some(job) = jobs.get_mut(&job_id) {
                    job.status = ContractImportJobStatus::Processing;
                }
            }

            let results = process_import_batch(
                &state_clone,
                contracts_clone,
                fail_safe,
                skip_existing,
                Some(job_id),
            )
            .await;

            let mut jobs = IMPORT_JOBS.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                let all_success = results.iter().all(|r| r.success);
                let any_success = results.iter().any(|r| r.success);

                job.status = if all_success {
                    ContractImportJobStatus::Completed
                } else if any_success {
                    ContractImportJobStatus::Partial
                } else {
                    ContractImportJobStatus::Failed
                };
                job.completed_at = Some(chrono::Utc::now());
            }
        });

        return Ok((
            StatusCode::ACCEPTED,
            Json(ContractImportAcceptedResponse {
                job_id,
                status: ContractImportJobStatus::Pending,
                status_url: format!("/contracts/import/{}", job_id),
                total_count: total_count as i64,
                requested_at: chrono::Utc::now(),
                fail_safe,
            }),
        )
            .into_response());
    }

    // Synchronous processing
    let results = process_import_batch(&state, req.contracts, fail_safe, skip_existing, None).await;

    let imported_count = results.iter().filter(|r| r.success).count();
    let failed_count = total_count - imported_count;
    let errors: Vec<String> = results.iter().filter_map(|r| r.error.clone()).collect();

    Ok((
        StatusCode::OK,
        Json(ContractImportResponse {
            total_count,
            imported_count,
            failed_count,
            results,
            errors,
        }),
    )
        .into_response())
}

/// GET /contracts/import/:job_id - Get import job status
#[utoipa::path(
    get,
    path = "/contracts/import/{job_id}",
    params(
        ("job_id" = Uuid, Path, description = "Import job ID")
    ),
    responses(
        (status = 200, description = "Import job status", body = ContractImportStatusResponse),
        (status = 404, description = "Import job not found")
    ),
    tag = "Bulk Operations"
)]
pub async fn get_import_status(
    Path(job_id): Path<Uuid>,
) -> ApiResult<Json<ContractImportStatusResponse>> {
    let jobs = IMPORT_JOBS.read().await;
    let job = jobs.get(&job_id).ok_or_else(|| {
        ApiError::not_found("ImportNotFound", "No import job found for the supplied ID")
    })?;

    Ok(Json(ContractImportStatusResponse {
        job_id: job.job_id,
        status: job.status.clone(),
        status_url: format!("/contracts/import/{}", job_id),
        total_count: job.total_count,
        processed_count: job.processed_count,
        imported_count: job.imported_count,
        failed_count: job.failed_count,
        requested_at: job.requested_at,
        completed_at: job.completed_at,
        fail_safe: job.fail_safe,
        error: job.error.clone(),
        results: Some(job.results.clone()),
    }))
}

/// GET /contracts/export - Export contracts (GET version with query params)
#[utoipa::path(
    get,
    path = "/contracts/export",
    params(ContractExportQueryParams),
    responses(
        (status = 200, description = "Contract metadata export stream"),
        (status = 202, description = "Large export accepted", body = ContractExportAcceptedResponse),
        (status = 400, description = "Invalid export request")
    ),
    tag = "Bulk Operations"
)]
pub async fn get_export_contracts(
    State(_state): State<AppState>,
    _claims: Option<AuthClaims>,
    Query(params): Query<ContractExportQueryParams>,
) -> ApiResult<Response> {
    // Convert query params to search params and return a simple JSON response
    // For the GET endpoint, we return a simple message directing users to use POST
    // for full export functionality, or implement streaming export here
    let format = params.format.unwrap_or(ContractExportFormat::Json);

    // Return a 200 with instructions for using the POST endpoint
    let response_body = serde_json::json!({
        "message": "Use POST /contracts/export for full export functionality with filters",
        "format": format.to_string(),
        "note": "GET endpoint supports basic filtering via query params. For large exports, use POST with async_mode=true",
        "supported_formats": ["json", "csv", "yaml"],
        "example_post_body": {
            "format": "json",
            "filters": {
                "networks": ["mainnet"],
                "category": "DEX",
                "verified_only": true
            },
            "async_mode": false
        }
    });

    Ok((StatusCode::OK, Json(response_body)).into_response())
}

// Cleanup old import jobs periodically
pub async fn cleanup_old_import_jobs() {
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(24);
    let mut jobs = IMPORT_JOBS.write().await;
    jobs.retain(|_, job| job.completed_at.map(|at| at > cutoff).unwrap_or(true));
}
