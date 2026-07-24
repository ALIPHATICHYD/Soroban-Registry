// patch_handlers.rs
// Differential contract update pipeline (Issue #501).
//
// Detects and stores only the delta (changed fields) between consecutive
// contract versions.  Full versions can be reconstructed by replaying the
// patch chain from the initial baseline.

use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::db_internal_error,
    state::AppState,
};

// ─────────────────────────────────────────────────────────────────────────────
// Data types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ContractPatch {
    pub id: Uuid,
    pub contract_id: Uuid,
    /// Empty string means this is a baseline (first-version) patch.
    pub from_version: String,
    pub to_version: String,
    pub patch: Value,
    pub patch_size_bytes: i32,
    pub full_size_bytes: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PatchListResponse {
    pub contract_id: Uuid,
    pub patches: Vec<ContractPatch>,
    pub total_patch_bytes: i64,
    pub total_full_bytes: i64,
    /// Percentage of storage saved versus keeping full copies
    pub storage_savings_pct: f64,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ReconstructRequest {
    /// Target semver version to reconstruct.
    pub target_version: String,
}

#[derive(Debug, Serialize)]
pub struct ReconstructedVersion {
    pub contract_id: Uuid,
    pub version: String,
    pub wasm_hash: String,
    pub source_url: Option<String>,
    pub commit_hash: Option<String>,
    pub release_notes: Option<String>,
    pub state_schema: Option<Value>,
    pub abi: Option<Value>,
    pub reconstructed_via_patches: bool,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BulkApplyRequest {
    /// List of (contract_id, target_version) pairs to reconstruct in bulk.
    pub targets: Vec<BulkTarget>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct BulkTarget {
    pub contract_id: String,
    pub target_version: String,
}

#[derive(Debug, Serialize)]
pub struct BulkApplyResponse {
    pub results: Vec<BulkApplyResult>,
}

#[derive(Debug, Serialize)]
pub struct BulkApplyResult {
    pub contract_id: String,
    pub target_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<ReconstructedVersion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Public diff/store helpers (called from handlers.rs on version creation)
// ─────────────────────────────────────────────────────────────────────────────

/// Fields tracked in `contract_versions` that we diff between releases.
pub struct VersionSnapshot {
    pub version: String,
    pub wasm_hash: String,
    pub source_url: Option<String>,
    pub commit_hash: Option<String>,
    pub release_notes: Option<String>,
    pub state_schema: Option<Value>,
    pub abi: Value,
}

/// Compute and persist a delta patch between `from` and `to` snapshots.
/// `from` is `None` when `to` is the first version of the contract.
pub async fn store_patch(
    db: &sqlx::PgPool,
    contract_uuid: Uuid,
    from: Option<&VersionSnapshot>,
    to: &VersionSnapshot,
) -> Result<(), sqlx::Error> {
    let patch = build_patch(from, to);
    let patch_bytes = serde_json::to_vec(&patch).unwrap_or_default().len() as i32;
    let full_bytes = serde_json::to_vec(&snapshot_to_json(to))
        .unwrap_or_default()
        .len() as i32;

    // Use "" as sentinel for "no previous version" so the UNIQUE constraint works.
    let from_ver: &str = from.map(|s| s.version.as_str()).unwrap_or("");

    sqlx::query(
        "INSERT INTO contract_patches \
            (contract_id, from_version, to_version, patch, patch_size_bytes, full_size_bytes) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (contract_id, from_version, to_version) DO UPDATE \
            SET patch = EXCLUDED.patch, \
                patch_size_bytes = EXCLUDED.patch_size_bytes, \
                full_size_bytes = EXCLUDED.full_size_bytes",
    )
    .bind(contract_uuid)
    .bind(from_ver)
    .bind(&to.version)
    .bind(&patch)
    .bind(patch_bytes)
    .bind(full_bytes)
    .execute(db)
    .await?;

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Core diff logic
// ─────────────────────────────────────────────────────────────────────────────

/// Build a minimal JSON patch capturing only fields that differ between
/// `from` and `to`.  When `from` is None the patch carries the full `to`
/// snapshot so the baseline can be reconstructed without extra queries.
fn build_patch(from: Option<&VersionSnapshot>, to: &VersionSnapshot) -> Value {
    let mut fields: serde_json::Map<String, Value> = serde_json::Map::new();

    match from {
        None => {
            // First version — store all fields as the initial baseline.
            fields.insert("wasm_hash".into(), json!(to.wasm_hash));
            if let Some(ref v) = to.source_url {
                fields.insert("source_url".into(), json!(v));
            }
            if let Some(ref v) = to.commit_hash {
                fields.insert("commit_hash".into(), json!(v));
            }
            if let Some(ref v) = to.release_notes {
                fields.insert("release_notes".into(), json!(v));
            }
            if let Some(ref v) = to.state_schema {
                fields.insert("state_schema".into(), v.clone());
            }
            let abi_changes = json!({
                "added":    abi_function_names(&to.abi),
                "removed":  [],
                "modified": []
            });
            json!({ "fields": fields, "abi_changes": abi_changes })
        }
        Some(prev) => {
            if prev.wasm_hash != to.wasm_hash {
                fields.insert("wasm_hash".into(), json!(to.wasm_hash));
            }
            if prev.source_url != to.source_url {
                fields.insert("source_url".into(), json!(to.source_url));
            }
            if prev.commit_hash != to.commit_hash {
                fields.insert("commit_hash".into(), json!(to.commit_hash));
            }
            if prev.release_notes != to.release_notes {
                fields.insert("release_notes".into(), json!(to.release_notes));
            }
            if prev.state_schema != to.state_schema {
                fields.insert("state_schema".into(), json!(to.state_schema));
            }
            let abi_changes = diff_abi_functions(&prev.abi, &to.abi);
            json!({ "fields": fields, "abi_changes": abi_changes })
        }
    }
}

/// Return a JSON representation of the full snapshot (used for full_size_bytes).
fn snapshot_to_json(snap: &VersionSnapshot) -> Value {
    json!({
        "version":       snap.version,
        "wasm_hash":     snap.wasm_hash,
        "source_url":    snap.source_url,
        "commit_hash":   snap.commit_hash,
        "release_notes": snap.release_notes,
        "state_schema":  snap.state_schema,
        "abi":           snap.abi,
    })
}

/// Extract function names from an ABI JSON value.
/// Supports the `{"functions": [...]}` envelope as well as a bare array.
fn abi_function_names(abi: &Value) -> Vec<String> {
    let funcs = if let Some(arr) = abi.get("functions").and_then(|v| v.as_array()) {
        arr
    } else if let Some(arr) = abi.as_array() {
        arr
    } else {
        return vec![];
    };
    funcs
        .iter()
        .filter_map(|f| f.get("name").and_then(|n| n.as_str()).map(str::to_owned))
        .collect()
}

/// Compute a simple function-level ABI diff.
fn diff_abi_functions(old_abi: &Value, new_abi: &Value) -> Value {
    use std::collections::HashSet;

    let old_fns = abi_functions_map(old_abi);
    let new_fns = abi_functions_map(new_abi);

    let old_names: HashSet<&String> = old_fns.keys().collect();
    let new_names: HashSet<&String> = new_fns.keys().collect();

    let added: Vec<&String> = new_names.difference(&old_names).copied().collect();
    let removed: Vec<&String> = old_names.difference(&new_names).copied().collect();
    let mut modified: Vec<Value> = Vec::new();

    for name in old_names.intersection(&new_names) {
        if old_fns[*name] != new_fns[*name] {
            modified.push(json!({ "name": name }));
        }
    }

    json!({
        "added":    added,
        "removed":  removed,
        "modified": modified,
    })
}

fn abi_functions_map(abi: &Value) -> std::collections::HashMap<String, Value> {
    let funcs = if let Some(arr) = abi.get("functions").and_then(|v| v.as_array()) {
        arr.clone()
    } else if let Some(arr) = abi.as_array() {
        arr.clone()
    } else {
        return std::collections::HashMap::new();
    };
    funcs
        .into_iter()
        .filter_map(|f| {
            f.get("name")
                .and_then(|n| n.as_str())
                .map(|n| (n.to_owned(), f.clone()))
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Reconstruction
// ─────────────────────────────────────────────────────────────────────────────

/// Replay the patch chain to reconstruct the state at `target_version`.
/// Returns None if the version or its patch chain is not found.
async fn reconstruct_version(
    state: &crate::state::AppState,
    contract_uuid: Uuid,
    target_version: &str,
) -> Result<Option<ReconstructedVersion>, sqlx::Error> {
    // Fetch all patches for this contract, ordered by creation time (proxy for
    // version ordering; semver ordering happens in memory for simplicity).
    let patches: Vec<ContractPatch> = sqlx::query_as(
        "SELECT id, contract_id, from_version, to_version, patch, \
                patch_size_bytes, full_size_bytes, created_at \
         FROM contract_patches \
         WHERE contract_id = $1 \
         ORDER BY created_at ASC",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await?;

    if patches.is_empty() {
        return Ok(None);
    }

    // Walk the chain: find the root patch (from_version == ""), then follow
    // the chain until we reach the target_version.
    let root = match patches.iter().find(|p| p.from_version.is_empty()) {
        Some(p) => p,
        None => return Ok(None),
    };

    // Build current reconstructed state from the root patch.
    let mut current = apply_patch_to_baseline(root);
    let mut current_version = root.to_version.clone();

    if current_version == target_version {
        // Fetch ABI separately — it is stored in contract_abis. Use cache-aware helper.
        let abi = fetch_abi(state, contract_uuid, target_version).await?;
        current.abi = abi;
        current.version = current_version;
        current.reconstructed_via_patches = true;
        return Ok(Some(current));
    }

    // Walk forward through the chain.
    loop {
        let next = patches.iter().find(|p| p.from_version == current_version);
        match next {
            None => return Ok(None),
            Some(p) => {
                apply_patch_to_state(&mut current, p);
                current_version = p.to_version.clone();
                if current_version == target_version {
                    let abi = fetch_abi(state, contract_uuid, target_version).await?;
                    current.abi = abi;
                    current.version = current_version;
                    current.reconstructed_via_patches = true;
                    return Ok(Some(current));
                }
            }
        }
    }
}

/// Build an initial `ReconstructedVersion` from the root (baseline) patch.
fn apply_patch_to_baseline(patch: &ContractPatch) -> ReconstructedVersion {
    let fields = patch.patch.get("fields").cloned().unwrap_or_default();
    ReconstructedVersion {
        contract_id: patch.contract_id,
        version: patch.to_version.clone(),
        wasm_hash: fields
            .get("wasm_hash")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned(),
        source_url: fields
            .get("source_url")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        commit_hash: fields
            .get("commit_hash")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        release_notes: fields
            .get("release_notes")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        state_schema: fields.get("state_schema").cloned(),
        abi: None,
        reconstructed_via_patches: true,
    }
}

/// Overlay changed fields from `patch` onto `state`.
fn apply_patch_to_state(state: &mut ReconstructedVersion, patch: &ContractPatch) {
    let fields = match patch.patch.get("fields") {
        Some(f) => f,
        None => return,
    };
    if let Some(v) = fields.get("wasm_hash").and_then(|v| v.as_str()) {
        state.wasm_hash = v.to_owned();
    }
    if fields.get("source_url").is_some() {
        state.source_url = fields
            .get("source_url")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
    }
    if fields.get("commit_hash").is_some() {
        state.commit_hash = fields
            .get("commit_hash")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
    }
    if fields.get("release_notes").is_some() {
        state.release_notes = fields
            .get("release_notes")
            .and_then(|v| v.as_str())
            .map(str::to_owned);
    }
    if fields.get("state_schema").is_some() {
        state.state_schema = fields.get("state_schema").cloned();
    }
}

async fn fetch_abi(
    state: &crate::state::AppState,
    contract_uuid: Uuid,
    version: &str,
) -> Result<Option<Value>, sqlx::Error> {
    let version_key = format!("{}@{}", contract_uuid.to_string(), version);

    // L1/L2: try cache first
    if let Some(cached) = state.cache.get_abi(&version_key, false).await {
        if let Ok(v) = serde_json::from_str::<Value>(&cached) {
            return Ok(Some(v));
        }
    }

    // DB fallback
    let row: Option<(Value,)> =
        sqlx::query_as("SELECT abi FROM contract_abis WHERE contract_id = $1 AND version = $2")
            .bind(contract_uuid)
            .bind(version)
            .fetch_optional(&state.db)
            .await?;

    if let Some((val,)) = &row {
        if let Ok(abi_str) = serde_json::to_string(val) {
            let _ = state.cache.put_abi(&version_key, abi_str.clone()).await;
            let _ = state
                .cache
                .put_abi(&contract_uuid.to_string(), abi_str)
                .await;
        }
    }

    Ok(row.map(|(v,)| v))
}

// ─────────────────────────────────────────────────────────────────────────────
// HTTP handlers
// ─────────────────────────────────────────────────────────────────────────────

/// GET /api/contracts/:id/patches
///
/// List all stored delta patches for a contract, including storage savings.
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/patches",
    params(("id" = String, Path, description = "Contract UUID or contract_id")),
    responses(
        (status = 200, description = "Patch list with storage savings summary"),
        (status = 404, description = "Contract not found")
    ),
    tag = "Patches"
)]
pub async fn list_contract_patches(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<PatchListResponse>> {
    let contract_uuid = resolve_contract_uuid(&state, &id).await?;

    let patches: Vec<ContractPatch> = sqlx::query_as(
        "SELECT id, contract_id, from_version, to_version, patch, \
                patch_size_bytes, full_size_bytes, created_at \
         FROM contract_patches \
         WHERE contract_id = $1 \
         ORDER BY created_at ASC",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("list contract patches", err))?;

    let total_patch_bytes: i64 = patches.iter().map(|p| p.patch_size_bytes as i64).sum();
    let total_full_bytes: i64 = patches.iter().map(|p| p.full_size_bytes as i64).sum();
    let savings_pct = if total_full_bytes > 0 {
        (1.0 - total_patch_bytes as f64 / total_full_bytes as f64) * 100.0
    } else {
        0.0
    };

    Ok(Json(PatchListResponse {
        contract_id: contract_uuid,
        patches,
        total_patch_bytes,
        total_full_bytes,
        storage_savings_pct: (savings_pct * 10.0).round() / 10.0,
    }))
}

/// GET /api/contracts/:id/patches/:from_version/:to_version
///
/// Fetch the specific delta patch between two consecutive versions.
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/patches/{from_version}/{to_version}",
    params(
        ("id" = String, Path, description = "Contract UUID or contract_id"),
        ("from_version" = String, Path, description = "Source version (use 'base' for the initial patch)"),
        ("to_version" = String, Path, description = "Target version")
    ),
    responses(
        (status = 200, description = "Delta patch"),
        (status = 404, description = "Patch not found")
    ),
    tag = "Patches"
)]
pub async fn get_patch_between_versions(
    State(state): State<AppState>,
    Path((id, from_ver, to_ver)): Path<(String, String, String)>,
) -> ApiResult<Json<ContractPatch>> {
    let contract_uuid = resolve_contract_uuid(&state, &id).await?;
    // "base" is the user-facing alias for the sentinel empty string.
    let from_str: &str = if from_ver == "base" { "" } else { &from_ver };

    let patch: Option<ContractPatch> = sqlx::query_as(
        "SELECT id, contract_id, from_version, to_version, patch, \
                patch_size_bytes, full_size_bytes, created_at \
         FROM contract_patches \
         WHERE contract_id = $1 \
           AND from_version = $2 \
           AND to_version = $3",
    )
    .bind(contract_uuid)
    .bind(from_str)
    .bind(&to_ver)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch patch", err))?;

    patch.map(Json).ok_or_else(|| {
        ApiError::not_found(
            "PatchNotFound",
            format!("No patch found from '{}' to '{}'", from_ver, to_ver),
        )
    })
}

/// POST /api/contracts/:id/patches/reconstruct
///
/// Reconstruct the full state of a specific version by replaying the patch chain.
#[utoipa::path(
    post,
    path = "/api/contracts/{id}/patches/reconstruct",
    params(("id" = String, Path, description = "Contract UUID or contract_id")),
    request_body = ReconstructRequest,
    responses(
        (status = 200, description = "Reconstructed version"),
        (status = 404, description = "Version or patch chain not found")
    ),
    tag = "Patches"
)]
pub async fn reconstruct_contract_version(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<ReconstructRequest>,
) -> ApiResult<Json<ReconstructedVersion>> {
    let contract_uuid = resolve_contract_uuid(&state, &id).await?;

    let result = reconstruct_version(&state, contract_uuid, &req.target_version)
        .await
        .map_err(|err| db_internal_error("reconstruct version", err))?;

    result.map(Json).ok_or_else(|| {
        ApiError::not_found(
            "VersionNotFound",
            format!(
                "Cannot reconstruct version '{}': patch chain not found",
                req.target_version
            ),
        )
    })
}

/// POST /api/contracts/patches/bulk-apply
///
/// Reconstruct multiple contract versions in a single request.
#[utoipa::path(
    post,
    path = "/api/contracts/patches/bulk-apply",
    request_body = BulkApplyRequest,
    responses(
        (status = 200, description = "Bulk reconstruction results"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Patches"
)]
pub async fn bulk_apply_patches(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<BulkApplyRequest>,
) -> ApiResult<Json<BulkApplyResponse>> {
    if req.targets.is_empty() {
        return Err(ApiError::bad_request(
            "EmptyTargets",
            "targets list must not be empty",
        ));
    }
    if req.targets.len() > 100 {
        return Err(ApiError::bad_request(
            "TooManyTargets",
            "bulk-apply supports at most 100 targets per request",
        ));
    }

    let mut results = Vec::with_capacity(req.targets.len());

    for target in &req.targets {
        let result = async {
            let contract_uuid = resolve_contract_uuid(&state, &target.contract_id).await?;
            let version = reconstruct_version(&state, contract_uuid, &target.target_version)
                .await
                .map_err(|err| db_internal_error("reconstruct version (bulk)", err))?;
            Ok::<Option<ReconstructedVersion>, ApiError>(version)
        }
        .await;

        match result {
            Ok(Some(v)) => results.push(BulkApplyResult {
                contract_id: target.contract_id.clone(),
                target_version: target.target_version.clone(),
                version: Some(v),
                error: None,
            }),
            Ok(None) => results.push(BulkApplyResult {
                contract_id: target.contract_id.clone(),
                target_version: target.target_version.clone(),
                version: None,
                error: Some(format!(
                    "Patch chain not found for version '{}'",
                    target.target_version
                )),
            }),
            Err(e) => results.push(BulkApplyResult {
                contract_id: target.contract_id.clone(),
                target_version: target.target_version.clone(),
                version: None,
                error: Some(e.to_string()),
            }),
        }
    }

    Ok(Json(BulkApplyResponse { results }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

async fn resolve_contract_uuid(state: &AppState, id: &str) -> ApiResult<Uuid> {
    // Try parsing directly as UUID first.
    if let Ok(uuid) = Uuid::parse_str(id) {
        return Ok(uuid);
    }
    // Fall back to looking up by contract_id string.
    let row: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM contracts WHERE contract_id = $1 LIMIT 1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("resolve contract uuid", err))?;

    row.map(|(uuid,)| uuid).ok_or_else(|| {
        ApiError::not_found("ContractNotFound", format!("Contract '{}' not found", id))
    })
}
