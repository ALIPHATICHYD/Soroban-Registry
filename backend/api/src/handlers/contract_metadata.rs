use crate::{
    error::{ApiError, ApiResult},
    handlers::db_internal_error,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use shared::models::{Contract, ContractMetadataVersion, MetadataDiff, MetadataHistoryResponse};
use uuid::Uuid;

/// Get metadata history for a contract (#729)
pub async fn get_metadata_versions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<MetadataHistoryResponse>> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;

    let versions = sqlx::query_as::<_, ContractMetadataVersion>(
        "SELECT * FROM contract_metadata_versions 
         WHERE contract_id = $1 
         ORDER BY created_at DESC 
         LIMIT 50",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch metadata versions", err))?;

    Ok(Json(MetadataHistoryResponse {
        contract_id: contract_uuid,
        versions,
    }))
}

/// Get a specific metadata version (#729)
pub async fn get_metadata_version(
    State(state): State<AppState>,
    Path((_id, version_id)): Path<(String, Uuid)>,
) -> ApiResult<Json<ContractMetadataVersion>> {
    let version = sqlx::query_as::<_, ContractMetadataVersion>(
        "SELECT * FROM contract_metadata_versions WHERE id = $1",
    )
    .bind(version_id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => {
            ApiError::not_found("VersionNotFound", "Metadata version not found")
        }
        _ => db_internal_error("fetch specific metadata version", err),
    })?;

    Ok(Json(version))
}

/// Rollback contract metadata to a previous version (#729)
pub async fn rollback_metadata(
    State(state): State<AppState>,
    Path((id, version_id)): Path<(String, Uuid)>,
) -> ApiResult<Json<Contract>> {
    let contract_uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::bad_request(
            "InvalidContractId",
            format!("Invalid contract ID format: {}", id),
        )
    })?;

    // 1. Fetch the target version
    let version = sqlx::query_as::<_, ContractMetadataVersion>(
        "SELECT * FROM contract_metadata_versions WHERE id = $1 AND contract_id = $2",
    )
    .bind(version_id)
    .bind(contract_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => ApiError::not_found(
            "VersionNotFound",
            "Metadata version not found for this contract",
        ),
        _ => db_internal_error("fetch version for rollback", err),
    })?;

    // 2. Start transaction
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| db_internal_error("begin rollback tx", err))?;

    // 3. Update contract metadata
    let updated_contract: Contract = sqlx::query_as(
        "UPDATE contracts 
         SET name = $2, 
             description = $3, 
             category = $4, 
             updated_at = NOW() 
         WHERE id = $1 
         RETURNING *",
    )
    .bind(contract_uuid)
    .bind(&version.name)
    .bind(&version.description)
    .bind(&version.category)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| db_internal_error("rollback contract update", err))?;

    // 4. Update tags
    sqlx::query("DELETE FROM contract_tags WHERE contract_id = $1")
        .bind(contract_uuid)
        .execute(&mut *tx)
        .await
        .map_err(|err| db_internal_error("delete tags for rollback", err))?;

    for tag_name in &version.tags {
        // Ensure tag exists
        let tag_id: Uuid = sqlx::query_scalar(
            "INSERT INTO tags (name) VALUES ($1) ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name RETURNING id"
        )
        .bind(tag_name)
        .fetch_one(&mut *tx)
        .await
        .map_err(|err| db_internal_error("ensure tag exists for rollback", err))?;

        sqlx::query("INSERT INTO contract_tags (contract_id, tag_id) VALUES ($1, $2)")
            .bind(contract_uuid)
            .bind(tag_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| db_internal_error("insert tag for rollback", err))?;
    }

    // 5. Record this rollback as a new version
    sqlx::query(
        "INSERT INTO contract_metadata_versions (contract_id, name, description, category, tags, change_summary) 
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(contract_uuid)
    .bind(&version.name)
    .bind(&version.description)
    .bind(&version.category)
    .bind(&version.tags)
    .bind(format!("Rollback to version from {}", version.created_at))
    .execute(&mut *tx)
    .await
    .map_err(|err| db_internal_error("record rollback version", err))?;

    tx.commit()
        .await
        .map_err(|err| db_internal_error("commit rollback tx", err))?;

    Ok(Json(updated_contract))
}

/// Calculate diff between two metadata versions (#729)
pub async fn compare_metadata_versions(
    State(state): State<AppState>,
    Path((_id, v1_id, v2_id)): Path<(String, Uuid, Uuid)>,
) -> ApiResult<Json<Vec<MetadataDiff>>> {
    let v1 = sqlx::query_as::<_, ContractMetadataVersion>(
        "SELECT * FROM contract_metadata_versions WHERE id = $1",
    )
    .bind(v1_id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch version 1", err))?;

    let v2 = sqlx::query_as::<_, ContractMetadataVersion>(
        "SELECT * FROM contract_metadata_versions WHERE id = $1",
    )
    .bind(v2_id)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch version 2", err))?;

    let mut diffs = Vec::new();

    if v1.name != v2.name {
        diffs.push(MetadataDiff {
            field: "name".to_string(),
            old_value: Some(serde_json::Value::String(v1.name)),
            new_value: Some(serde_json::Value::String(v2.name)),
        });
    }

    if v1.description != v2.description {
        diffs.push(MetadataDiff {
            field: "description".to_string(),
            old_value: v1.description.map(serde_json::Value::String),
            new_value: v2.description.map(serde_json::Value::String),
        });
    }

    if v1.category != v2.category {
        diffs.push(MetadataDiff {
            field: "category".to_string(),
            old_value: v1.category.map(serde_json::Value::String),
            new_value: v2.category.map(serde_json::Value::String),
        });
    }

    if v1.tags != v2.tags {
        diffs.push(MetadataDiff {
            field: "tags".to_string(),
            old_value: Some(serde_json::to_value(v1.tags).unwrap()),
            new_value: Some(serde_json::to_value(v2.tags).unwrap()),
        });
    }

    Ok(Json(diffs))
}
