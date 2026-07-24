use std::collections::HashMap;

use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, Query, State},
    http::header,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    breaking_changes::{diff_abi, has_breaking_changes, resolve_abi, ChangeSeverity},
    error::{ApiError, ApiResult},
    state::AppState,
};
use contract_abi::parser::parse_json_spec;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct ContractVersionRecord {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: String,
    pub wasm_hash: String,
    pub source_url: Option<String>,
    pub commit_hash: Option<String>,
    pub release_notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
struct ContractCompatibilityOverrideRow {
    pub source_version: String,
    pub target_version: String,
    pub stellar_version: Option<String>,
    pub is_compatible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionCompatibilityCell {
    pub target_version: String,
    pub is_compatible: bool,
    pub has_breaking_changes: bool,
    pub breaking_changes: Vec<String>,
    pub breaking_change_count: usize,
    pub stellar_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionCompatibilityRow {
    pub source_version: String,
    pub targets: Vec<VersionCompatibilityCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractVersionCompatibilityResponse {
    pub contract_id: Uuid,
    pub contract_stellar_id: String,
    pub version_order: Vec<String>,
    pub rows: Vec<VersionCompatibilityRow>,
    pub warnings: Vec<String>,
    pub total_pairs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityExportRow {
    pub source_version: String,
    pub target_version: String,
    pub is_compatible: bool,
    pub has_breaking_changes: bool,
    pub breaking_change_count: usize,
    pub breaking_changes: Vec<String>,
    pub stellar_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddCompatibilityRequest {
    pub source_version: String,
    pub target_contract_id: Uuid,
    pub target_version: String,
    pub stellar_version: Option<String>,
    pub is_compatible: bool,
}

#[derive(Debug, Deserialize)]
pub struct ExportFormat {
    pub format: Option<String>,
}

#[derive(Debug, Clone)]
struct CompatibilityComputation {
    source_version: String,
    target_version: String,
    is_compatible: bool,
    has_breaking_changes: bool,
    breaking_changes: Vec<String>,
    stellar_version: Option<String>,
}

async fn resolve_contract_identity(
    state: &AppState,
    contract_id: &str,
) -> ApiResult<(Uuid, String)> {
    if let Ok(uuid) = Uuid::parse_str(contract_id) {
        let row = sqlx::query_as::<_, (Uuid, String)>(
            "SELECT id, contract_id FROM contracts WHERE id = $1",
        )
        .bind(uuid)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

        if let Some(identity) = row {
            return Ok(identity);
        }
    }

    sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, contract_id FROM contracts WHERE contract_id = $1",
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?
    .ok_or_else(|| ApiError::not_found("NotFound", "Contract not found"))
}

async fn load_versions(
    state: &AppState,
    contract_uuid: Uuid,
) -> ApiResult<Vec<ContractVersionRecord>> {
    sqlx::query_as::<_, ContractVersionRecord>(
        "SELECT id, contract_id, version, wasm_hash, source_url, commit_hash, release_notes, created_at \
         FROM contract_versions WHERE contract_id = $1 ORDER BY created_at ASC",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))
}

async fn load_overrides(
    state: &AppState,
    contract_uuid: Uuid,
) -> ApiResult<HashMap<(String, String), ContractCompatibilityOverrideRow>> {
    let rows = sqlx::query_as::<_, ContractCompatibilityOverrideRow>(
        "SELECT source_version, target_version, stellar_version, is_compatible \
         FROM contract_version_compatibility \
         WHERE source_contract_id = $1 AND target_contract_id = $1",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                (row.source_version.clone(), row.target_version.clone()),
                row,
            )
        })
        .collect())
}

async fn compute_upgrade_matrix(
    state: &AppState,
    contract_uuid: Uuid,
    contract_stellar_id: &str,
) -> ApiResult<ContractVersionCompatibilityResponse> {
    let versions = load_versions(state, contract_uuid).await?;
    let version_order: Vec<String> = versions.iter().map(|v| v.version.clone()).collect();
    let overrides = load_overrides(state, contract_uuid).await?;

    let mut rows = Vec::new();
    let mut warnings = Vec::new();
    let mut total_pairs = 0usize;

    for (source_index, source) in versions.iter().enumerate() {
        let mut targets = Vec::new();

        for target in versions.iter().skip(source_index + 1) {
            total_pairs += 1;

            let compatibility = if let Some(override_row) =
                overrides.get(&(source.version.clone(), target.version.clone()))
            {
                CompatibilityComputation {
                    source_version: source.version.clone(),
                    target_version: target.version.clone(),
                    is_compatible: override_row.is_compatible,
                    has_breaking_changes: !override_row.is_compatible,
                    breaking_changes: if override_row.is_compatible {
                        Vec::new()
                    } else {
                        vec![format!(
                            "Manual override marks upgrade from {} to {} as incompatible",
                            source.version, target.version
                        )]
                    },
                    stellar_version: override_row.stellar_version.clone(),
                }
            } else {
                let old_selector = format!("{}@{}", contract_stellar_id, source.version);
                let new_selector = format!("{}@{}", contract_stellar_id, target.version);

                let old_abi = match resolve_abi(state, &old_selector, false).await {
                    Ok(abi) => abi,
                    Err(_) => {
                        let message = format!(
                            "ABI unavailable for upgrade path {} -> {}",
                            source.version, target.version
                        );
                        warnings.push(message.clone());
                        targets.push(VersionCompatibilityCell {
                            target_version: target.version.clone(),
                            is_compatible: false,
                            has_breaking_changes: true,
                            breaking_changes: vec![message],
                            breaking_change_count: 1,
                            stellar_version: None,
                        });
                        continue;
                    }
                };

                let new_abi = match resolve_abi(state, &new_selector, false).await {
                    Ok(abi) => abi,
                    Err(_) => {
                        let message = format!(
                            "ABI unavailable for upgrade path {} -> {}",
                            source.version, target.version
                        );
                        warnings.push(message.clone());
                        targets.push(VersionCompatibilityCell {
                            target_version: target.version.clone(),
                            is_compatible: false,
                            has_breaking_changes: true,
                            breaking_changes: vec![message],
                            breaking_change_count: 1,
                            stellar_version: None,
                        });
                        continue;
                    }
                };

                let old_spec = parse_json_spec(&old_abi, &old_selector).map_err(|e| {
                    ApiError::bad_request_with(
                        "InvalidABI",
                        format!("Failed to parse old ABI: {e}"),
                    )
                })?;
                let new_spec = parse_json_spec(&new_abi, &new_selector).map_err(|e| {
                    ApiError::bad_request_with(
                        "InvalidABI",
                        format!("Failed to parse new ABI: {e}"),
                    )
                })?;

                let changes = diff_abi(&old_spec, &new_spec);
                let breaking_changes = changes
                    .iter()
                    .filter(|change| change.severity == ChangeSeverity::Breaking)
                    .map(|change| change.message.clone())
                    .collect::<Vec<_>>();

                CompatibilityComputation {
                    source_version: source.version.clone(),
                    target_version: target.version.clone(),
                    is_compatible: !has_breaking_changes(&changes),
                    has_breaking_changes: has_breaking_changes(&changes),
                    breaking_changes,
                    stellar_version: None,
                }
            };

            if compatibility.has_breaking_changes {
                warnings.push(format!(
                    "Upgrade from {} to {} has breaking ABI changes",
                    compatibility.source_version, compatibility.target_version
                ));
            }

            targets.push(VersionCompatibilityCell {
                target_version: compatibility.target_version,
                is_compatible: compatibility.is_compatible,
                has_breaking_changes: compatibility.has_breaking_changes,
                breaking_changes: compatibility.breaking_changes.clone(),
                breaking_change_count: compatibility.breaking_changes.len(),
                stellar_version: compatibility.stellar_version,
            });
        }

        rows.push(VersionCompatibilityRow {
            source_version: source.version.clone(),
            targets,
        });
    }

    Ok(ContractVersionCompatibilityResponse {
        contract_id: contract_uuid,
        contract_stellar_id: contract_stellar_id.to_string(),
        version_order,
        rows,
        warnings,
        total_pairs,
    })
}

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/compatibility",
    params(("id" = String, Path, description = "Contract UUID or Stellar contract ID")),
    responses(
        (status = 200, description = "Upgrade compatibility matrix for contract versions"),
        (status = 404, description = "Contract not found")
    ),
    tag = "Versions"
)]
pub async fn get_contract_compatibility(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> ApiResult<Json<ContractVersionCompatibilityResponse>> {
    let (contract_uuid, contract_stellar_id) =
        resolve_contract_identity(&state, &contract_id).await?;
    let response = compute_upgrade_matrix(&state, contract_uuid, &contract_stellar_id).await?;
    Ok(Json(response))
}

pub async fn export_contract_compatibility(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    Query(params): Query<ExportFormat>,
) -> ApiResult<impl IntoResponse> {
    let (contract_uuid, contract_stellar_id) =
        resolve_contract_identity(&state, &contract_id).await?;
    let response = compute_upgrade_matrix(&state, contract_uuid, &contract_stellar_id).await?;

    let export_rows = response
        .rows
        .iter()
        .flat_map(|row| {
            row.targets.iter().map(|target| CompatibilityExportRow {
                source_version: row.source_version.clone(),
                target_version: target.target_version.clone(),
                is_compatible: target.is_compatible,
                has_breaking_changes: target.has_breaking_changes,
                breaking_change_count: target.breaking_change_count,
                breaking_changes: target.breaking_changes.clone(),
                stellar_version: target.stellar_version.clone(),
            })
        })
        .collect::<Vec<_>>();

    match params.format.as_deref().unwrap_or("json") {
        "csv" => {
            let mut csv = String::from(
                "source_version,target_version,is_compatible,has_breaking_changes,breaking_change_count,breaking_changes,stellar_version\n",
            );

            for row in &export_rows {
                csv.push_str(&format!(
                    "{},{},{},{},{},\"{}\",{}\n",
                    row.source_version,
                    row.target_version,
                    row.is_compatible,
                    row.has_breaking_changes,
                    row.breaking_change_count,
                    row.breaking_changes.join(" | ").replace('"', "\"\""),
                    row.stellar_version.as_deref().unwrap_or("")
                ));
            }

            Ok((
                axum::http::StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=\"compatibility.csv\"",
                    ),
                ],
                csv,
            )
                .into_response())
        }
        _ => Ok(Json(export_rows).into_response()),
    }
}

pub async fn add_contract_compatibility(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
    ValidatedJson(body): ValidatedJson<AddCompatibilityRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let (source_contract_uuid, _) = resolve_contract_identity(&state, &contract_id).await?;

    let target_exists: bool =
        sqlx::query_scalar("SELECT COUNT(*) > 0 FROM contracts WHERE id = $1")
            .bind(body.target_contract_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    if !target_exists {
        return Err(ApiError::not_found("NotFound", "Target contract not found"));
    }

    sqlx::query(
        "INSERT INTO contract_version_compatibility \
         (source_contract_id, source_version, target_contract_id, target_version, stellar_version, is_compatible) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (source_contract_id, source_version, target_contract_id, target_version) \
         DO UPDATE SET stellar_version = EXCLUDED.stellar_version, is_compatible = EXCLUDED.is_compatible, updated_at = NOW()",
    )
    .bind(source_contract_uuid)
    .bind(&body.source_version)
    .bind(body.target_contract_id)
    .bind(&body.target_version)
    .bind(&body.stellar_version)
    .bind(body.is_compatible)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {e}")))?;

    Ok(Json(serde_json::json!({
        "message": "Compatibility entry saved",
        "source_contract_id": source_contract_uuid,
        "source_version": body.source_version,
        "target_contract_id": body.target_contract_id,
        "target_version": body.target_version,
        "is_compatible": body.is_compatible,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cell(
        target_version: &str,
        is_compatible: bool,
        breaking_changes: Vec<&str>,
    ) -> VersionCompatibilityCell {
        VersionCompatibilityCell {
            target_version: target_version.to_string(),
            is_compatible,
            has_breaking_changes: !breaking_changes.is_empty(),
            breaking_change_count: breaking_changes.len(),
            breaking_changes: breaking_changes
                .into_iter()
                .map(|item| item.to_string())
                .collect(),
            stellar_version: None,
        }
    }

    #[test]
    fn breaking_cells_reflect_breaking_change_count() {
        let cell = make_cell("2.0.0", false, vec!["Function 'transfer' was removed"]);
        assert!(!cell.is_compatible);
        assert!(cell.has_breaking_changes);
        assert_eq!(cell.breaking_change_count, 1);
    }

    #[test]
    fn export_rows_can_be_flattened_from_matrix_rows() {
        let rows = vec![VersionCompatibilityRow {
            source_version: "1.0.0".to_string(),
            targets: vec![
                make_cell("1.1.0", true, vec![]),
                make_cell(
                    "2.0.0",
                    false,
                    vec!["Function 'balance' return type changed"],
                ),
            ],
        }];

        let flattened = rows
            .iter()
            .flat_map(|row| {
                row.targets.iter().map(|target| CompatibilityExportRow {
                    source_version: row.source_version.clone(),
                    target_version: target.target_version.clone(),
                    is_compatible: target.is_compatible,
                    has_breaking_changes: target.has_breaking_changes,
                    breaking_change_count: target.breaking_change_count,
                    breaking_changes: target.breaking_changes.clone(),
                    stellar_version: target.stellar_version.clone(),
                })
            })
            .collect::<Vec<_>>();

        assert_eq!(flattened.len(), 2);
        assert!(flattened[0].is_compatible);
        assert_eq!(flattened[1].breaking_change_count, 1);
    }
}
