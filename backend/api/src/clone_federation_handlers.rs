//! Contract Clone and Federation Handlers
//!
//! #487: Contract Mirror/Clone Endpoint
//! #499: Federated Contract Registry Protocol

use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, Query, State},
    http::{header::HeaderMap, StatusCode},
    Json,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::{db_internal_error, extract_ip_address, write_contract_audit_log},
    state::AppState,
};
use shared::{
    AuditActionType, CloneContractRequest, CloneContractResponse, Contract, FederatedRegistry,
    FederatedRegistryListResponse, FederatedRegistryResponse, FederatedRegistrySummary,
    FederationAttribution, FederationDiscoveryResponse, FederationOptRequest,
    FederationProtocolConfig, FederationSyncHistoryResponse, FederationSyncJob,
    FederationSyncResponse, RegisterFederatedRegistryRequest, SyncFederatedRegistryRequest,
};

// ═══════════════════════════════════════════════════════════════════════════
// #487: Contract Clone/Mirror Handlers
// ═══════════════════════════════════════════════════════════════════════════

/// Clone an existing contract as a new registry entry
#[utoipa::path(
    post,
    path = "/api/contracts/{id}/clone",
    params(
        ("id" = String, Path, description = "Contract ID to clone")
    ),
    request_body = CloneContractRequest,
    responses(
        (status = 201, description = "Contract cloned successfully", body = CloneContractResponse),
        (status = 400, description = "Invalid request or contract not found"),
        (status = 409, description = "Contract ID already exists")
    ),
    tag = "Contracts"
)]
pub async fn clone_contract(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<CloneContractRequest>,
) -> ApiResult<Json<CloneContractResponse>> {
    // Resolve the source contract ID
    let source_uuid = resolve_contract_id(&state.db, &id).await?;

    // Fetch the original contract
    let original: Contract = sqlx::query_as("SELECT * FROM contracts WHERE id = $1")
        .bind(source_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| {
            if matches!(err, sqlx::Error::RowNotFound) {
                ApiError::not_found("ContractNotFound", "Original contract not found")
            } else {
                db_internal_error("fetch original contract", err)
            }
        })?;

    // Check if the new contract_id already exists on the target network
    let target_network = req
        .network
        .clone()
        .unwrap_or_else(|| original.network.clone());
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM contracts WHERE contract_id = $1 AND network = $2",
    )
    .bind(&req.contract_id)
    .bind(&target_network)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("check existing contract", err))?;

    if existing > 0 {
        return Err(ApiError::conflict(
            "ContractAlreadyExists",
            format!(
                "Contract {} already exists on {}",
                req.contract_id, target_network
            ),
        ));
    }

    // Determine publisher (use request or default to original)
    let publisher_id = req.publisher_id.unwrap_or(original.publisher_id);

    // Build new name
    let new_name = req
        .name
        .unwrap_or_else(|| format!("{} Clone", original.name));

    // Start transaction
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| db_internal_error("begin transaction", err))?;

    // Resolve tags: map Vec<Tag> -> Vec<String> for sqlx binding
    let tags_strings: Vec<String> = req
        .tags
        .as_ref()
        .map(|tags| tags.iter().map(|t| t.clone()).collect())
        .unwrap_or_else(|| original.tags.iter().map(|t| t.name.clone()).collect());

    // Insert the cloned contract
    let clone: Contract = sqlx::query_as(
        r#"
        INSERT INTO contracts (
            contract_id, wasm_hash, name, description, publisher_id, network,
            category, tags, cloned_from_id, clone_count, logical_id, network_configs,
            is_verified, health_score, is_maintenance, visibility
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0, $10, $11, false, 0, false, 'public')
        RETURNING *
        "#,
    )
    .bind(&req.contract_id)
    .bind(req.wasm_hash.as_deref().unwrap_or(&original.wasm_hash))
    .bind(&new_name)
    .bind(
        req.description
            .as_deref()
            .or(original.description.as_deref()),
    )
    .bind(publisher_id)
    .bind(&target_network)
    .bind(req.category.as_deref().or(original.category.as_deref()))
    .bind(&tags_strings as &[String])
    .bind(original.id)
    .bind(original.logical_id)
    .bind(&original.network_configs)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| db_internal_error("insert cloned contract", err))?;

    // Copy ABI from original contract
    let abi_copied = copy_contract_abi(&mut tx, original.id, clone.id).await?;

    // Copy contract versions
    copy_contract_versions(&mut tx, original.id, clone.id).await?;

    // Record clone history
    let metadata_overrides = json!({
        "name": new_name,
        "description": req.description,
        "network": target_network,
        "category": req.category,
        "tags": req.tags
    });

    sqlx::query(
        r#"
        INSERT INTO contract_clone_history (
            parent_contract_id, cloned_contract_id, metadata_overrides, network
        ) VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(original.id)
    .bind(clone.id)
    .bind(metadata_overrides)
    .bind(&target_network)
    .execute(&mut *tx)
    .await
    .map_err(|err| db_internal_error("record clone history", err))?;

    // Increment clone count on original
    sqlx::query("UPDATE contracts SET clone_count = clone_count + 1 WHERE id = $1")
        .bind(original.id)
        .execute(&mut *tx)
        .await
        .map_err(|err| db_internal_error("increment clone count", err))?;

    tx.commit()
        .await
        .map_err(|err| db_internal_error("commit transaction", err))?;

    // Write audit log
    let changes = json!({
        "cloned_from": {
            "contract_id": original.contract_id,
            "name": original.name,
            "id": original.id
        },
        "new_contract_id": clone.contract_id,
        "name": clone.name
    });

    write_contract_audit_log(
        &state.db,
        // FIX: use the correct variant name from shared::AuditActionType
        AuditActionType::ContractPublished,
        clone.id,
        publisher_id,
        changes,
        &extract_ip_address(&headers),
    )
    .await
    .map_err(|err| db_internal_error("write clone audit log", err))?;

    let response = CloneContractResponse {
        id: clone.id,
        contract_id: clone.contract_id.clone(),
        name: clone.name.clone(),
        original_contract_id: original.id,
        original_contract_name: original.name.clone(),
        clone_link: format!("/api/contracts/{}", clone.id),
        network: clone.network.clone(),
        inherited_abi: abi_copied,
        created_at: clone.created_at,
    };

    Ok(Json(response))
}

/// Get clone history for a contract
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/clones",
    params(
        ("id" = String, Path, description = "Contract ID")
    ),
    responses(
        (status = 200, description = "List of clones", body = [Contract]),
        (status = 404, description = "Contract not found")
    ),
    tag = "Contracts"
)]
pub async fn get_contract_clones(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<Contract>>> {
    let contract_uuid = resolve_contract_id(&state.db, &id).await?;

    let clones: Vec<Contract> = sqlx::query_as(
        "SELECT * FROM contracts WHERE cloned_from_id = $1 ORDER BY created_at DESC",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch clones", err))?;

    Ok(Json(clones))
}

// ═══════════════════════════════════════════════════════════════════════════
// #499: Federated Registry Protocol Handlers
// ═══════════════════════════════════════════════════════════════════════════

/// List all federated registries
#[utoipa::path(
    get,
    path = "/api/federation/registries",
    responses(
        (status = 200, description = "List of federated registries", body = FederatedRegistryListResponse),
    ),
    tag = "Federation"
)]
pub async fn list_federated_registries(
    State(state): State<AppState>,
) -> ApiResult<Json<FederatedRegistryListResponse>> {
    let registries: Vec<FederatedRegistry> =
        sqlx::query_as("SELECT * FROM federated_registries ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await
            .map_err(|err| db_internal_error("list federated registries", err))?;

    let total = registries.len() as i64;
    Ok(Json(FederatedRegistryListResponse {
        registries,
        total_count: total,
    }))
}

/// Register a new federated registry
#[utoipa::path(
    post,
    path = "/api/federation/registries",
    request_body = RegisterFederatedRegistryRequest,
    responses(
        (status = 201, description = "Registry registered", body = FederatedRegistryResponse),
        (status = 409, description = "Registry URL already exists")
    ),
    tag = "Federation"
)]
pub async fn register_federated_registry(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<RegisterFederatedRegistryRequest>,
) -> ApiResult<Json<FederatedRegistryResponse>> {
    let protocol_version = req
        .federation_protocol_version
        .unwrap_or_else(|| "1.0".to_string());

    let registry: FederatedRegistry = sqlx::query_as(
        r#"
        INSERT INTO federated_registries (
            name, base_url, public_key, federation_protocol_version
        ) VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(&req.name)
    .bind(&req.base_url)
    .bind(req.public_key.as_deref())
    .bind(&protocol_version)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if let sqlx::Error::Database(ref e) = err {
            if e.constraint() == Some("federated_registries_base_url_key") {
                return ApiError::conflict(
                    "RegistryAlreadyExists",
                    "A registry with this URL is already registered",
                );
            }
        }
        db_internal_error("register federated registry", err)
    })?;

    let response = FederatedRegistryResponse {
        id: registry.id,
        name: registry.name.clone(),
        base_url: registry.base_url.clone(),
        is_active: registry.is_active,
        federation_protocol_version: registry.federation_protocol_version.clone(),
        registration_link: format!("/api/federation/registries/{}", registry.id),
        created_at: registry.created_at,
    };

    Ok(Json(response))
}

/// Get details of a specific federated registry
#[utoipa::path(
    get,
    path = "/api/federation/registries/{id}",
    params(
        ("id" = String, Path, description = "Registry ID")
    ),
    responses(
        (status = 200, description = "Registry details", body = FederatedRegistry),
        (status = 404, description = "Registry not found")
    ),
    tag = "Federation"
)]
pub async fn get_federated_registry(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<FederatedRegistry>> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| ApiError::bad_request_with("InvalidId", "Invalid registry ID format"))?;

    let registry: FederatedRegistry =
        sqlx::query_as("SELECT * FROM federated_registries WHERE id = $1")
            .bind(uuid)
            .fetch_one(&state.db)
            .await
            .map_err(|err| {
                if matches!(err, sqlx::Error::RowNotFound) {
                    ApiError::not_found("RegistryNotFound", "Federated registry not found")
                } else {
                    db_internal_error("fetch federated registry", err)
                }
            })?;

    Ok(Json(registry))
}

/// Sync contracts from a federated registry
#[utoipa::path(
    post,
    path = "/api/federation/sync",
    request_body = SyncFederatedRegistryRequest,
    responses(
        (status = 202, description = "Sync job started", body = FederationSyncResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Registry not found")
    ),
    tag = "Federation"
)]
pub async fn sync_from_federated_registry(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<SyncFederatedRegistryRequest>,
) -> ApiResult<Json<FederationSyncResponse>> {
    // Verify registry exists and is active
    let registry: FederatedRegistry =
        sqlx::query_as("SELECT * FROM federated_registries WHERE id = $1 AND is_active = true")
            .bind(req.registry_id)
            .fetch_one(&state.db)
            .await
            .map_err(|err| {
                if matches!(err, sqlx::Error::RowNotFound) {
                    ApiError::not_found("RegistryNotFound", "Registry not found or not active")
                } else {
                    db_internal_error("fetch registry", err)
                }
            })?;

    // Create sync job
    let job: FederationSyncJob = sqlx::query_as(
        r#"
        INSERT INTO federation_sync_jobs (registry_id, status)
        VALUES ($1, 'pending')
        RETURNING *
        "#,
    )
    .bind(req.registry_id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("create sync job", err))?;

    // Update registry status
    sqlx::query("UPDATE federated_registries SET sync_status = 'syncing' WHERE id = $1")
        .bind(req.registry_id)
        .execute(&state.db)
        .await
        .map_err(|err| db_internal_error("update registry sync status", err))?;

    // Spawn background task to perform the actual sync
    tokio::spawn(async move {
        perform_federation_sync(state.db.clone(), job.id, req.registry_id, req.batch_size).await;
    });

    let response = FederationSyncResponse {
        job_id: job.id,
        registry_id: req.registry_id,
        registry_name: registry.name,
        status: job.status,
        contracts_synced: 0,
        contracts_failed: 0,
        duplicates_detected: 0,
        sync_link: format!("/api/federation/sync/{}", job.id),
        started_at: None,
    };

    Ok(Json(response))
}

/// Get sync job status
#[utoipa::path(
    get,
    path = "/api/federation/sync/{job_id}",
    params(
        ("job_id" = String, Path, description = "Sync job ID")
    ),
    responses(
        (status = 200, description = "Sync job status", body = FederationSyncJob),
        (status = 404, description = "Job not found")
    ),
    tag = "Federation"
)]
pub async fn get_sync_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> ApiResult<Json<FederationSyncJob>> {
    let uuid = Uuid::parse_str(&job_id)
        .map_err(|_| ApiError::bad_request_with("InvalidId", "Invalid job ID format"))?;

    let job: FederationSyncJob = sqlx::query_as("SELECT * FROM federation_sync_jobs WHERE id = $1")
        .bind(uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| {
            if matches!(err, sqlx::Error::RowNotFound) {
                ApiError::not_found("SyncJobNotFound", "Sync job not found")
            } else {
                db_internal_error("fetch sync job", err)
            }
        })?;

    Ok(Json(job))
}

/// Get federation sync history
#[utoipa::path(
    get,
    path = "/api/federation/sync-history",
    params(
        ("limit" = Option<i64>, Query, description = "Limit results"),
        ("offset" = Option<i64>, Query, description = "Offset for pagination")
    ),
    responses(
        (status = 200, description = "Sync history", body = FederationSyncHistoryResponse),
    ),
    tag = "Federation"
)]
pub async fn get_federation_sync_history(
    State(state): State<AppState>,
    Query(params): Query<SyncHistoryQuery>,
) -> ApiResult<Json<FederationSyncHistoryResponse>> {
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);

    let jobs: Vec<FederationSyncJob> = sqlx::query_as(
        "SELECT * FROM federation_sync_jobs ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch sync history", err))?;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM federation_sync_jobs")
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("count sync jobs", err))?;

    Ok(Json(FederationSyncHistoryResponse {
        jobs,
        total_count: total,
    }))
}

#[derive(Debug, Clone, serde::Deserialize, utoipa::IntoParams)]
pub struct SyncHistoryQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Discover federated registries
#[utoipa::path(
    get,
    path = "/api/federation/discover",
    responses(
        (status = 200, description = "Discovered registries", body = FederationDiscoveryResponse),
    ),
    tag = "Federation"
)]
pub async fn discover_federated_registries(
    State(state): State<AppState>,
) -> ApiResult<Json<FederationDiscoveryResponse>> {
    let registries: Vec<FederatedRegistry> =
        sqlx::query_as("SELECT * FROM federated_registries WHERE is_active = true ORDER BY name")
            .fetch_all(&state.db)
            .await
            .map_err(|err| db_internal_error("discover registries", err))?;

    let summaries: Vec<FederatedRegistrySummary> = registries
        .into_iter()
        .map(|r| FederatedRegistrySummary {
            id: r.id,
            name: r.name,
            base_url: r.base_url,
            contracts_count: r.contracts_count,
            protocol_version: r.federation_protocol_version,
            is_active: r.is_active,
        })
        .collect();

    // FIX: capture total_count before moving summaries into the struct
    let total_count = summaries.len() as i64;
    Ok(Json(FederationDiscoveryResponse {
        registries: summaries,
        total_count,
        discovered_at: chrono::Utc::now(),
    }))
}

/// Get federation attribution for a contract
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/federation",
    params(
        ("id" = String, Path, description = "Contract ID")
    ),
    responses(
        (status = 200, description = "Federation attribution info", body = FederationAttribution),
        (status = 404, description = "Contract not found or not federated")
    ),
    tag = "Federation"
)]
pub async fn get_contract_federation_attribution(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<FederationAttribution>> {
    let contract_uuid = resolve_contract_id(&state.db, &id).await?;

    let result: (Uuid, String, String, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
        r#"
        SELECT f.id, f.name, c.original_registry_contract_id, c.created_at
        FROM contracts c
        JOIN federated_registries f ON c.federated_from_id = f.id
        WHERE c.id = $1
        "#,
    )
    .bind(contract_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        if matches!(err, sqlx::Error::RowNotFound) {
            ApiError::not_found("NotFederated", "Contract is not from a federated registry")
        } else {
            db_internal_error("fetch federation attribution", err)
        }
    })?;

    Ok(Json(FederationAttribution {
        source_registry_id: result.0,
        source_registry_name: result.1,
        original_contract_id: result.2,
        synced_at: result.3,
        attribution_link: format!("/api/federation/registries/{}", result.0),
    }))
}

/// Update contract federation settings
#[utoipa::path(
    patch,
    path = "/api/contracts/{id}/federation",
    params(
        ("id" = String, Path, description = "Contract ID")
    ),
    request_body = FederationOptRequest,
    responses(
        (status = 200, description = "Settings updated"),
        (status = 404, description = "Contract not found")
    ),
    tag = "Federation"
)]
pub async fn update_contract_federation_settings(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<FederationOptRequest>,
) -> ApiResult<StatusCode> {
    let contract_uuid = resolve_contract_id(&state.db, &id).await?;

    let metadata = json!({
        "allow_federation": req.allow_federation,
        "registry_filters": req.registry_filters
    });

    // Update or insert federation metadata
    sqlx::query(
        r#"
        UPDATE contracts 
        SET federation_metadata = COALESCE(federation_metadata, '{}'::jsonb) || $1
        WHERE id = $2
        "#,
    )
    .bind(metadata)
    .bind(contract_uuid)
    .execute(&state.db)
    .await
    .map_err(|err| db_internal_error("update federation settings", err))?;

    Ok(StatusCode::OK)
}

/// Get federation protocol configuration
#[utoipa::path(
    get,
    path = "/api/federation/config",
    responses(
        (status = 200, description = "Federation configuration", body = [FederationProtocolConfig]),
    ),
    tag = "Federation"
)]
pub async fn get_federation_config(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<FederationProtocolConfig>>> {
    let config: Vec<FederationProtocolConfig> =
        sqlx::query_as("SELECT * FROM federation_protocol_config ORDER BY config_key")
            .fetch_all(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch federation config", err))?;

    Ok(Json(config))
}

// ═══════════════════════════════════════════════════════════════════════════
// Helper Functions
// ═══════════════════════════════════════════════════════════════════════════

async fn resolve_contract_id(db: &PgPool, identifier: &str) -> ApiResult<Uuid> {
    // Try parsing as UUID first
    if let Ok(uuid) = Uuid::parse_str(identifier) {
        return Ok(uuid);
    }

    // Try as contract_id (address)
    let uuid = sqlx::query_scalar::<_, Uuid>("SELECT id FROM contracts WHERE contract_id = $1")
        .bind(identifier)
        .fetch_optional(db)
        .await
        .map_err(|err| db_internal_error("resolve contract id", err))?;

    uuid.ok_or_else(|| {
        ApiError::not_found(
            "ContractNotFound",
            format!("Contract '{}' not found", identifier),
        )
    })
}

async fn copy_contract_abi(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    from: Uuid,
    to: Uuid,
) -> ApiResult<bool> {
    let result = sqlx::query(
        r#"
        INSERT INTO contract_abis (contract_id, abi_json, schema_json, created_at)
        SELECT $1, abi_json, schema_json, NOW()
        FROM contract_abis
        WHERE contract_id = $2
        RETURNING id
        "#,
    )
    .bind(to)
    .bind(from)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|err| db_internal_error("copy ABI", err))?;

    Ok(result.is_some())
}

async fn copy_contract_versions(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    from: Uuid,
    to: Uuid,
) -> ApiResult<()> {
    sqlx::query(
        r#"
        INSERT INTO contract_versions (
            contract_id, version, wasm_hash, source_url, commit_hash,
            release_notes, state_schema, signature, publisher_key,
            signature_algorithm, change_notes, is_revert, reverted_from
        )
        SELECT $1, version, wasm_hash, source_url, commit_hash,
               release_notes, state_schema, signature, publisher_key,
               signature_algorithm, change_notes, false, NULL
        FROM contract_versions
        WHERE contract_id = $2
        "#,
    )
    .bind(to)
    .bind(from)
    .execute(&mut **tx)
    .await
    .map_err(|err| db_internal_error("copy versions", err))?;

    Ok(())
}

/// Background task to perform federation sync
async fn perform_federation_sync(db: PgPool, job_id: Uuid, registry_id: Uuid, batch_size: i32) {
    use reqwest::Client;

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap_or_else(|_| Client::new());

    // Update job status to running
    let _ = sqlx::query(
        "UPDATE federation_sync_jobs SET status = 'running', started_at = NOW() WHERE id = $1",
    )
    .bind(job_id)
    .execute(&db)
    .await;

    // Fetch registry details
    let registry_url: Option<String> =
        sqlx::query_scalar("SELECT base_url FROM federated_registries WHERE id = $1")
            .bind(registry_id)
            .fetch_optional(&db)
            .await
            .ok()
            .flatten();

    let Some(base_url) = registry_url else {
        let _ = sqlx::query(
            "UPDATE federation_sync_jobs SET status = 'failed', error_message = $1 WHERE id = $2",
        )
        .bind("Registry URL not found")
        .bind(job_id)
        .execute(&db)
        .await;
        return;
    };

    // Fetch contracts from remote registry
    let sync_url = format!(
        "{}/api/contracts?limit={}",
        base_url.trim_end_matches('/'),
        batch_size
    );

    // FIX: use Result<_, String> with explicit .map_err(|e| e.to_string()) so
    // reqwest::Error converts to String (the only From impl available here).
    let result: Result<serde_json::Value, String> = async {
        let mut headers = reqwest::header::HeaderMap::new();
        crate::request_tracing::inject_current_trace_context(&mut headers);
        let response = client
            .get(&sync_url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("Failed to fetch contracts: {}", response.status()));
        }
        let contracts: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        Ok(contracts)
    }
    .await;

    match result {
        Ok(contracts_json) => {
            // Process contracts (simplified - in production would handle each contract)
            let contracts_synced =
                if let Some(arr) = contracts_json.get("contracts").and_then(|v| v.as_array()) {
                    arr.len() as i32
                } else {
                    0
                };

            let _ = sqlx::query(
                "UPDATE federation_sync_jobs SET status = 'completed', contracts_synced = $1, completed_at = NOW() WHERE id = $2",
            )
            .bind(contracts_synced)
            .bind(job_id)
            .execute(&db)
            .await;

            let _ = sqlx::query(
                "UPDATE federated_registries SET sync_status = 'synced', last_synced_at = NOW(), contracts_count = $1 WHERE id = $2",
            )
            .bind(contracts_synced)
            .bind(registry_id)
            .execute(&db)
            .await;
        }
        Err(err) => {
            let _ = sqlx::query(
                "UPDATE federation_sync_jobs SET status = 'failed', error_message = $1 WHERE id = $2",
            )
            .bind(&err)
            .bind(job_id)
            .execute(&db)
            .await;

            let _ = sqlx::query(
                "UPDATE federated_registries SET sync_status = 'error', sync_error = $1 WHERE id = $2",
            )
            .bind(&err)
            .bind(registry_id)
            .execute(&db)
            .await;
        }
    }
}
