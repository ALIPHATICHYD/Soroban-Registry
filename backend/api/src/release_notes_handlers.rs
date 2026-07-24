use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::{DiffSummary, FunctionChange, ReleaseNotesStatus, SemVer};
use sqlx::FromRow;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReleaseNotesGenerated {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub previous_version: Option<String>,
    pub diff_summary: serde_json::Value,
    pub changelog_entry: Option<String>,
    pub notes_text: String,
    pub status: ReleaseNotesStatus,
    pub generated_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseNotesResponse {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub previous_version: Option<String>,
    pub diff_summary: DiffSummary,
    pub changelog_entry: Option<String>,
    pub notes_text: String,
    pub status: ReleaseNotesStatus,
    pub generated_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateReleaseNotesRequest {
    pub version: String,
    pub previous_version: Option<String>,
    pub changelog_content: Option<String>,
    pub contract_address: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateReleaseNotesRequest {
    pub notes_text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PublishReleaseNotesRequest {
    pub update_version_record: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// GET  /api/contracts/:id/release-notes/:version
// ─────────────────────────────────────────────────────────────────────────────

/// Retrieve generated release notes for a specific contract version
pub async fn get_release_notes(
    State(state): State<AppState>,
    Path((id, version)): Path<(String, String)>,
) -> ApiResult<Json<ReleaseNotesResponse>> {
    let (contract_uuid, _contract_id) = fetch_contract_identity(&state, &id).await?;

    let record = sqlx::query_as::<_, ReleaseNotesGenerated>(
        "SELECT * FROM release_notes_generated WHERE contract_id = $1 AND version = $2",
    )
    .bind(contract_uuid)
    .bind(&version)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch release notes", err))?
    .ok_or_else(|| {
        ApiError::not_found(
            "ReleaseNotesNotFound",
            format!("No release notes found for version '{}'", version),
        )
    })?;

    Ok(Json(to_response(record)))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET  /api/contracts/:id/release-notes
// ─────────────────────────────────────────────────────────────────────────────

/// List all generated release notes for a contract
pub async fn list_release_notes(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<ReleaseNotesResponse>>> {
    let (contract_uuid, _contract_id) = fetch_contract_identity(&state, &id).await?;

    let records = sqlx::query_as::<_, ReleaseNotesGenerated>(
        "SELECT * FROM release_notes_generated WHERE contract_id = $1 ORDER BY created_at DESC",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("list release notes", err))?;

    let responses: Vec<ReleaseNotesResponse> = records.into_iter().map(to_response).collect();
    Ok(Json(responses))
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/contracts/:id/release-notes/generate
// ─────────────────────────────────────────────────────────────────────────────

/// Auto-generate release notes from code diff, changelog, and version metadata
pub async fn generate_release_notes(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<GenerateReleaseNotesRequest>,
) -> ApiResult<Json<ReleaseNotesResponse>> {
    let (contract_uuid, contract_id) = fetch_contract_identity(&state, &id).await?;

    // Validate the requested version is valid semver
    let target_ver = SemVer::parse(&req.version).ok_or_else(|| {
        ApiError::bad_request(
            "InvalidVersion",
            "Version must be valid semver (e.g. 1.2.3)",
        )
    })?;

    // Verify the version exists in contract_versions
    let version_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM contract_versions WHERE contract_id = $1 AND version = $2)",
    )
    .bind(contract_uuid)
    .bind(&req.version)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("check version exists", err))?;

    if !version_exists {
        return Err(ApiError::not_found(
            "VersionNotFound",
            format!(
                "Version '{}' does not exist for contract '{}'",
                req.version, contract_id
            ),
        ));
    }

    // Determine previous version (explicit or auto-detect)
    let previous_version = match &req.previous_version {
        Some(pv) => {
            SemVer::parse(pv).ok_or_else(|| {
                ApiError::bad_request(
                    "InvalidPreviousVersion",
                    "previous_version must be valid semver",
                )
            })?;
            Some(pv.clone())
        }
        None => {
            // Find the latest version before the target version
            let all_versions: Vec<String> =
                sqlx::query_scalar("SELECT version FROM contract_versions WHERE contract_id = $1")
                    .bind(contract_uuid)
                    .fetch_all(&state.db)
                    .await
                    .map_err(|err| db_internal_error("fetch versions", err))?;

            let mut parsed: Vec<SemVer> = all_versions
                .iter()
                .filter_map(|v| SemVer::parse(v))
                .filter(|v| v < &target_ver)
                .collect();
            parsed.sort();
            parsed.last().map(|v| v.to_string())
        }
    };

    // Build the diff summary by comparing ABIs (if previous version exists)
    let diff_summary = if let Some(ref prev_ver) = previous_version {
        build_diff_summary(&state, contract_uuid, &contract_id, prev_ver, &req.version).await?
    } else {
        // First version — everything is new
        build_initial_diff_summary(&state, contract_uuid, &contract_id, &req.version).await?
    };

    // Parse changelog content if provided
    let changelog_entry = req
        .changelog_content
        .as_ref()
        .map(|content| extract_changelog_section(content, &req.version));

    // Generate the release notes text from the template
    let notes_text = render_release_notes_template(
        &contract_id,
        &req.version,
        previous_version.as_deref(),
        &diff_summary,
        changelog_entry.as_deref(),
        req.contract_address.as_deref(),
    );

    let diff_json = serde_json::to_value(&diff_summary).unwrap_or_else(|_| serde_json::json!({}));

    // Upsert into release_notes_generated (re-generating overwrites existing draft)
    let record = sqlx::query_as::<_, ReleaseNotesGenerated>(
        "INSERT INTO release_notes_generated \
            (contract_id, version, previous_version, diff_summary, changelog_entry, notes_text, status, generated_by) \
         VALUES ($1, $2, $3, $4, $5, $6, 'draft', 'auto') \
         ON CONFLICT (contract_id, version) DO UPDATE SET \
            previous_version = EXCLUDED.previous_version, \
            diff_summary     = EXCLUDED.diff_summary, \
            changelog_entry  = EXCLUDED.changelog_entry, \
            notes_text       = EXCLUDED.notes_text, \
            status           = 'draft', \
            generated_by     = 'auto', \
            updated_at       = NOW(), \
            published_at     = NULL \
         RETURNING *",
    )
    .bind(contract_uuid)
    .bind(&req.version)
    .bind(&previous_version)
    .bind(&diff_json)
    .bind(&changelog_entry)
    .bind(&notes_text)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("upsert release notes", err))?;

    tracing::info!(
        contract_id = %contract_id,
        version = %req.version,
        previous_version = ?previous_version,
        breaking_changes = diff_summary.has_breaking_changes,
        "release notes auto-generated"
    );

    Ok(Json(to_response(record)))
}

// ─────────────────────────────────────────────────────────────────────────────
// PUT  /api/contracts/:id/release-notes/:version
// ─────────────────────────────────────────────────────────────────────────────

/// Manually edit release notes (only while in draft status)
pub async fn update_release_notes(
    State(state): State<AppState>,
    Path((id, version)): Path<(String, String)>,
    ValidatedJson(req): ValidatedJson<UpdateReleaseNotesRequest>,
) -> ApiResult<Json<ReleaseNotesResponse>> {
    let (contract_uuid, _contract_id) = fetch_contract_identity(&state, &id).await?;

    // Ensure the record exists and is in draft status
    let existing = sqlx::query_as::<_, ReleaseNotesGenerated>(
        "SELECT * FROM release_notes_generated WHERE contract_id = $1 AND version = $2",
    )
    .bind(contract_uuid)
    .bind(&version)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch release notes", err))?
    .ok_or_else(|| {
        ApiError::not_found(
            "ReleaseNotesNotFound",
            format!("No release notes found for version '{}'", version),
        )
    })?;

    if existing.status == ReleaseNotesStatus::Published {
        return Err(ApiError::conflict(
            "ReleaseNotesAlreadyPublished",
            "Cannot edit published release notes. Generate new notes to create a fresh draft.",
        ));
    }

    let record = sqlx::query_as::<_, ReleaseNotesGenerated>(
        "UPDATE release_notes_generated \
         SET notes_text = $1, updated_at = NOW(), generated_by = 'manual' \
         WHERE contract_id = $2 AND version = $3 \
         RETURNING *",
    )
    .bind(&req.notes_text)
    .bind(contract_uuid)
    .bind(&version)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("update release notes", err))?;

    tracing::info!(
        contract_id = %contract_uuid,
        version = %version,
        "release notes manually edited"
    );

    Ok(Json(to_response(record)))
}

// ─────────────────────────────────────────────────────────────────────────────
// POST /api/contracts/:id/release-notes/:version/publish
// ─────────────────────────────────────────────────────────────────────────────

/// Publish (finalize) release notes — marks them as published and optionally
/// updates the `release_notes` column on `contract_versions`.
pub async fn publish_release_notes(
    State(state): State<AppState>,
    Path((id, version)): Path<(String, String)>,
    ValidatedJson(req): ValidatedJson<PublishReleaseNotesRequest>,
) -> ApiResult<Json<ReleaseNotesResponse>> {
    let (contract_uuid, _contract_id) = fetch_contract_identity(&state, &id).await?;

    let existing = sqlx::query_as::<_, ReleaseNotesGenerated>(
        "SELECT * FROM release_notes_generated WHERE contract_id = $1 AND version = $2",
    )
    .bind(contract_uuid)
    .bind(&version)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch release notes", err))?
    .ok_or_else(|| {
        ApiError::not_found(
            "ReleaseNotesNotFound",
            format!("No release notes found for version '{}'", version),
        )
    })?;

    if existing.status == ReleaseNotesStatus::Published {
        return Err(ApiError::conflict(
            "ReleaseNotesAlreadyPublished",
            "Release notes are already published.",
        ));
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| db_internal_error("begin transaction", err))?;

    // Mark as published
    let record = sqlx::query_as::<_, ReleaseNotesGenerated>(
        "UPDATE release_notes_generated \
         SET status = 'published', published_at = NOW(), updated_at = NOW() \
         WHERE contract_id = $1 AND version = $2 \
         RETURNING *",
    )
    .bind(contract_uuid)
    .bind(&version)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| db_internal_error("publish release notes", err))?;

    // Optionally push to contract_versions.release_notes
    if req.update_version_record {
        sqlx::query(
            "UPDATE contract_versions SET release_notes = $1 \
             WHERE contract_id = $2 AND version = $3",
        )
        .bind(&record.notes_text)
        .bind(contract_uuid)
        .bind(&version)
        .execute(&mut *tx)
        .await
        .map_err(|err| db_internal_error("update contract version release notes", err))?;
    }

    tx.commit()
        .await
        .map_err(|err| db_internal_error("commit publish", err))?;

    tracing::info!(
        contract_id = %contract_uuid,
        version = %version,
        update_version_record = req.update_version_record,
        "release notes published"
    );

    Ok(Json(to_response(record)))
}

// ═══════════════════════════════════════════════════════════════════════════
// INTERNAL HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/// Build a diff summary by comparing ABIs between two versions.
/// Falls back to a simple heuristic if full ABI data is unavailable.
async fn build_diff_summary(
    state: &AppState,
    contract_uuid: Uuid,
    contract_id: &str,
    old_version: &str,
    new_version: &str,
) -> ApiResult<DiffSummary> {
    // Try to load ABIs for both versions
    let old_abi: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT abi FROM contract_abis WHERE contract_id = $1 AND version = $2")
            .bind(contract_uuid)
            .bind(old_version)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch old ABI", err))?;

    let new_abi: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT abi FROM contract_abis WHERE contract_id = $1 AND version = $2")
            .bind(contract_uuid)
            .bind(new_version)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch new ABI", err))?;

    let mut diff = DiffSummary::default();

    match (old_abi, new_abi) {
        (Some(old), Some(new)) => {
            // Extract functions from ABI specs to detect additions/removals/changes
            let old_fns = extract_functions_from_abi(&old);
            let new_fns = extract_functions_from_abi(&new);

            // Detect added functions
            for (name, sig) in &new_fns {
                if !old_fns.contains_key(name) {
                    diff.function_changes.push(FunctionChange {
                        name: name.clone(),
                        change_type: "added".to_string(),
                        old_signature: None,
                        new_signature: Some(sig.clone()),
                        is_breaking: false,
                    });
                    diff.features_count += 1;
                }
            }

            // Detect removed functions (breaking!)
            for (name, sig) in &old_fns {
                if !new_fns.contains_key(name) {
                    diff.function_changes.push(FunctionChange {
                        name: name.clone(),
                        change_type: "removed".to_string(),
                        old_signature: Some(sig.clone()),
                        new_signature: None,
                        is_breaking: true,
                    });
                    diff.breaking_count += 1;
                    diff.has_breaking_changes = true;
                }
            }

            // Detect modified signatures (breaking if params/return changed)
            for (name, new_sig) in &new_fns {
                if let Some(old_sig) = old_fns.get(name) {
                    if old_sig != new_sig {
                        let is_breaking = true; // Signature change is always breaking
                        diff.function_changes.push(FunctionChange {
                            name: name.clone(),
                            change_type: "modified".to_string(),
                            old_signature: Some(old_sig.clone()),
                            new_signature: Some(new_sig.clone()),
                            is_breaking,
                        });
                        if is_breaking {
                            diff.breaking_count += 1;
                            diff.has_breaking_changes = true;
                        }
                    }
                }
            }

            diff.files_changed = if diff.function_changes.is_empty() {
                0
            } else {
                1 // At minimum the contract source
            };

            // Estimate lines from number of changes
            diff.lines_added = diff
                .function_changes
                .iter()
                .filter(|c| c.change_type == "added" || c.change_type == "modified")
                .count() as i32
                * 5;
            diff.lines_removed = diff
                .function_changes
                .iter()
                .filter(|c| c.change_type == "removed" || c.change_type == "modified")
                .count() as i32
                * 5;
        }
        _ => {
            // No ABI data available — produce a minimal diff
            tracing::warn!(
                contract_id = %contract_id,
                old_version = %old_version,
                new_version = %new_version,
                "ABI data unavailable for one or both versions; diff summary will be minimal"
            );
            diff.files_changed = 1;
        }
    }

    Ok(diff)
}

/// Build a diff summary for the very first version (everything is "added")
async fn build_initial_diff_summary(
    state: &AppState,
    contract_uuid: Uuid,
    _contract_id: &str,
    version: &str,
) -> ApiResult<DiffSummary> {
    let abi: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT abi FROM contract_abis WHERE contract_id = $1 AND version = $2")
            .bind(contract_uuid)
            .bind(version)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch ABI", err))?;

    let mut diff = DiffSummary::default();

    if let Some(abi_val) = abi {
        let fns = extract_functions_from_abi(&abi_val);
        for (name, sig) in &fns {
            diff.function_changes.push(FunctionChange {
                name: name.clone(),
                change_type: "added".to_string(),
                old_signature: None,
                new_signature: Some(sig.clone()),
                is_breaking: false,
            });
            diff.features_count += 1;
        }
        diff.files_changed = 1;
        diff.lines_added = fns.len() as i32 * 10;
    }

    Ok(diff)
}

/// Extract function name → signature mapping from a Soroban ABI JSON value.
/// Handles the common ABI shapes produced by Soroban contracts:
///   - Array of `{ "name": "fn_name", "inputs": [...], "outputs": [...] }`
///   - Or a spec object with a `"functions"` array
fn extract_functions_from_abi(
    abi: &serde_json::Value,
) -> std::collections::HashMap<String, String> {
    let mut fns = std::collections::HashMap::new();

    let entries: Vec<&serde_json::Value> = if let Some(arr) = abi.as_array() {
        arr.iter().collect()
    } else if let Some(spec_fns) = abi.get("functions").and_then(|f| f.as_array()) {
        spec_fns.iter().collect()
    } else if let Some(spec_fns) = abi.get("spec").and_then(|s| s.as_array()) {
        spec_fns.iter().collect()
    } else {
        Vec::new()
    };

    for entry in entries {
        if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
            // Build a signature string from inputs and outputs
            let inputs = entry
                .get("inputs")
                .and_then(|i| i.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|inp| {
                            let param_name =
                                inp.get("name").and_then(|n| n.as_str()).unwrap_or("_");
                            let param_type = inp
                                .get("type")
                                .map(|t| format!("{}", t))
                                .unwrap_or_else(|| "unknown".to_string());
                            format!("{}: {}", param_name, param_type)
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();

            let outputs = entry
                .get("outputs")
                .and_then(|o| o.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|out| {
                            out.get("type")
                                .map(|t| format!("{}", t))
                                .unwrap_or_else(|| "unknown".to_string())
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();

            let sig = format!("fn {}({}) -> ({})", name, inputs, outputs);
            fns.insert(name.to_string(), sig);
        }
    }

    fns
}

/// Extract the relevant section from a CHANGELOG.md for a given version.
/// Looks for headings like `## [1.2.3]`, `## 1.2.3`, `## v1.2.3`
fn extract_changelog_section(changelog: &str, version: &str) -> String {
    let version_clean = version.trim_start_matches('v');

    let patterns = [
        format!("## [{}]", version_clean),
        format!("## {}", version_clean),
        format!("## v{}", version_clean),
        format!("## [v{}]", version_clean),
    ];

    let lines: Vec<&str> = changelog.lines().collect();
    let mut start_idx = None;

    // Find the starting line of the version section
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if patterns.iter().any(|p| trimmed.starts_with(p.as_str())) {
            start_idx = Some(i);
            break;
        }
    }

    let start = match start_idx {
        Some(i) => i,
        None => return String::new(),
    };

    // Find the end: next `## ` heading or end of file
    let mut end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            end = i;
            break;
        }
    }

    lines[start..end].to_vec().join("\n").trim().to_string()
}

/// Render standardized release notes from the diff, changelog, and metadata.
fn render_release_notes_template(
    contract_id: &str,
    version: &str,
    previous_version: Option<&str>,
    diff: &DiffSummary,
    changelog_entry: Option<&str>,
    contract_address: Option<&str>,
) -> String {
    let mut sections: Vec<String> = Vec::new();

    // ── Header ───────────────────────────────────────────────────────────
    sections.push(format!("# Release Notes — v{}", version));
    sections.push(String::new());

    if let Some(prev) = previous_version {
        sections.push(format!("**Comparing:** v{} → v{}", prev, version));
    } else {
        sections.push("**Initial release**".to_string());
    }
    if let Some(addr) = contract_address {
        sections.push(format!("**Contract Address:** `{}`", addr));
    }
    sections.push(format!("**Contract ID:** `{}`", contract_id));
    sections.push(String::new());

    // ── Summary ──────────────────────────────────────────────────────────
    sections.push("## Summary".to_string());
    sections.push(format!("- **Files changed:** {}", diff.files_changed));
    sections.push(format!(
        "- **Lines added:** {} | **Lines removed:** {}",
        diff.lines_added, diff.lines_removed
    ));
    sections.push(format!(
        "- **Functions added:** {} | **Removed:** {} | **Modified:** {}",
        diff.function_changes
            .iter()
            .filter(|c| c.change_type == "added")
            .count(),
        diff.function_changes
            .iter()
            .filter(|c| c.change_type == "removed")
            .count(),
        diff.function_changes
            .iter()
            .filter(|c| c.change_type == "modified")
            .count(),
    ));
    sections.push(String::new());

    // ── Breaking Changes ─────────────────────────────────────────────────
    if diff.has_breaking_changes {
        sections.push("## ⚠️ Breaking Changes".to_string());
        sections.push(String::new());
        for fc in &diff.function_changes {
            if fc.is_breaking {
                match fc.change_type.as_str() {
                    "removed" => {
                        sections.push(format!("- **REMOVED** `{}`", fc.name));
                        if let Some(ref old) = fc.old_signature {
                            sections.push(format!("  - Was: `{}`", old));
                        }
                    }
                    "modified" => {
                        sections.push(format!("- **SIGNATURE CHANGED** `{}`", fc.name));
                        if let Some(ref old) = fc.old_signature {
                            sections.push(format!("  - Old: `{}`", old));
                        }
                        if let Some(ref new) = fc.new_signature {
                            sections.push(format!("  - New: `{}`", new));
                        }
                    }
                    _ => {}
                }
            }
        }
        sections.push(String::new());
    }

    // ── Features ─────────────────────────────────────────────────────────
    let added: Vec<&FunctionChange> = diff
        .function_changes
        .iter()
        .filter(|c| c.change_type == "added")
        .collect();
    if !added.is_empty() {
        sections.push("## Features".to_string());
        sections.push(String::new());
        for fc in &added {
            sections.push(format!("- Added `{}`", fc.name));
            if let Some(ref sig) = fc.new_signature {
                sections.push(format!("  - Signature: `{}`", sig));
            }
        }
        sections.push(String::new());
    }

    // ── Fixes / Changes ─────────────────────────────────────────────────
    let modified: Vec<&FunctionChange> = diff
        .function_changes
        .iter()
        .filter(|c| c.change_type == "modified" && !c.is_breaking)
        .collect();
    if !modified.is_empty() {
        sections.push("## Fixes & Changes".to_string());
        sections.push(String::new());
        for fc in &modified {
            sections.push(format!("- Modified `{}`", fc.name));
        }
        sections.push(String::new());
    }

    // ── Changelog ────────────────────────────────────────────────────────
    if let Some(entry) = changelog_entry {
        if !entry.is_empty() {
            sections.push("## Changelog".to_string());
            sections.push(String::new());
            sections.push(entry.to_string());
            sections.push(String::new());
        }
    }

    // ── Footer ───────────────────────────────────────────────────────────
    sections.push("---".to_string());
    sections.push(format!(
        "*Generated on {} by Soroban Registry*",
        Utc::now().format("%Y-%m-%d %H:%M UTC")
    ));

    sections.join("\n")
}

/// Convert DB record to API response, deserializing the diff_summary JSON
fn to_response(record: ReleaseNotesGenerated) -> ReleaseNotesResponse {
    let diff_summary: DiffSummary =
        serde_json::from_value(record.diff_summary.clone()).unwrap_or_default();

    ReleaseNotesResponse {
        id: record.id,
        contract_id: record.contract_id,
        version: record.version,
        previous_version: record.previous_version,
        diff_summary,
        changelog_entry: record.changelog_entry,
        notes_text: record.notes_text,
        status: record.status,
        generated_by: record.generated_by,
        created_at: record.created_at,
        updated_at: record.updated_at,
        published_at: record.published_at,
    }
}

// ── Shared helpers (same pattern as deprecation_handlers) ────────────────

async fn fetch_contract_identity(state: &AppState, id: &str) -> ApiResult<(Uuid, String)> {
    if let Ok(uuid) = Uuid::parse_str(id) {
        let row = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, contract_id FROM contracts WHERE id = $1",
        )
        .bind(uuid)
        .fetch_optional(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch contract", err))?;
        return row.ok_or_else(|| {
            ApiError::not_found(
                "ContractNotFound",
                format!("No contract found with ID: {}", id),
            )
        });
    }

    let row = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, contract_id FROM contracts WHERE contract_id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch contract", err))?;

    row.ok_or_else(|| {
        ApiError::not_found(
            "ContractNotFound",
            format!("No contract found with ID: {}", id),
        )
    })
}

fn db_internal_error(operation: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = operation, error = ?err, "database operation failed");
    ApiError::internal("Database operation failed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_changelog_section() {
        let changelog = r#"# Changelog

## [2.0.0] - 2026-02-20

### Breaking
- Removed `transfer_batch` function
- Changed `initialize` parameters

### Features
- Added multi-sig support

## [1.1.0] - 2026-01-15

### Features
- Added `get_balance` function

## [1.0.0] - 2026-01-01

### Initial Release
- Basic token contract
"#;

        let section = extract_changelog_section(changelog, "2.0.0");
        assert!(section.contains("Breaking"));
        assert!(section.contains("transfer_batch"));
        assert!(section.contains("multi-sig"));
        assert!(!section.contains("get_balance")); // from 1.1.0

        let section_v = extract_changelog_section(changelog, "v1.1.0");
        assert!(section_v.contains("get_balance"));

        let empty = extract_changelog_section(changelog, "3.0.0");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_extract_functions_from_abi() {
        let abi = serde_json::json!([
            {
                "name": "initialize",
                "inputs": [
                    {"name": "admin", "type": "Address"},
                    {"name": "decimal", "type": "u32"}
                ],
                "outputs": []
            },
            {
                "name": "transfer",
                "inputs": [
                    {"name": "from", "type": "Address"},
                    {"name": "to", "type": "Address"},
                    {"name": "amount", "type": "i128"}
                ],
                "outputs": [{"type": "bool"}]
            }
        ]);

        let fns = extract_functions_from_abi(&abi);
        assert_eq!(fns.len(), 2);
        assert!(fns.contains_key("initialize"));
        assert!(fns.contains_key("transfer"));
    }

    #[test]
    fn test_render_release_notes_template() {
        let diff = DiffSummary {
            files_changed: 2,
            lines_added: 30,
            lines_removed: 5,
            function_changes: vec![
                FunctionChange {
                    name: "new_feature".to_string(),
                    change_type: "added".to_string(),
                    old_signature: None,
                    new_signature: Some("fn new_feature(x: i128) -> (bool)".to_string()),
                    is_breaking: false,
                },
                FunctionChange {
                    name: "old_fn".to_string(),
                    change_type: "removed".to_string(),
                    old_signature: Some("fn old_fn() -> ()".to_string()),
                    new_signature: None,
                    is_breaking: true,
                },
            ],
            has_breaking_changes: true,
            features_count: 1,
            fixes_count: 0,
            breaking_count: 1,
        };

        let notes = render_release_notes_template(
            "CABC123",
            "2.0.0",
            Some("1.5.0"),
            &diff,
            Some("## [2.0.0]\n- Added new_feature\n- Removed old_fn"),
            Some("GABCDEF...XYZ"),
        );

        assert!(notes.contains("# Release Notes — v2.0.0"));
        assert!(notes.contains("v1.5.0 → v2.0.0"));
        assert!(notes.contains("Breaking Changes"));
        assert!(notes.contains("REMOVED"));
        assert!(notes.contains("`old_fn`"));
        assert!(notes.contains("Features"));
        assert!(notes.contains("`new_feature`"));
        assert!(notes.contains("Changelog"));
        assert!(notes.contains("CABC123"));
        assert!(notes.contains("GABCDEF...XYZ"));
    }
}
