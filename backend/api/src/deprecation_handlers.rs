use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use shared::{
    DeprecateContractRequest, DeprecationInfo, DeprecationStatus, UndeprecateContractRequest,
};
use std::collections::HashSet;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/deprecation",
    params(
        ("id" = String, Path, description = "Contract identifier")
    ),
    responses(
        (status = 200, description = "Deprecation status and info", body = DeprecationInfo),
        (status = 404, description = "Contract not found")
    ),
    tag = "Maintenance"
)]
pub async fn get_deprecation_info(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<DeprecationInfo>> {
    let (contract_uuid, contract_id) = fetch_contract_identity(&state, &id).await?;

    let contract_row = sqlx::query_as::<
        _,
        (
            Option<DateTime<Utc>>,
            Option<String>,
            Option<Uuid>,
            bool,
            DeprecationStatus,
        ),
    >(
        "SELECT deprecated_at, deprecation_reason, replacement_contract_id, is_deprecated, deprecation_status \
         FROM contracts WHERE id = $1",
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch contract deprecation columns", err))?;

    let schedule = sqlx::query_as::<
        _,
        (
            DateTime<Utc>,
            DateTime<Utc>,
            Option<Uuid>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT deprecated_at, retirement_at, replacement_contract_id, migration_guide_url, notes \
         FROM contract_deprecations WHERE contract_id = $1",
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch deprecation schedule", err))?;

    let dependents_notified: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_deprecation_notifications WHERE deprecated_contract_id = $1",
    )
    .bind(contract_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("count notifications", err))?;

    let (
        deprecated_at,
        deprecation_reason,
        replacement_uuid,
        _is_deprecated,
        mut status,
        retirement_at,
        migration_guide_url,
        notes,
    ) = match (contract_row, schedule) {
        (Some((dep_at, reason, repl, is_dep, st)), Some((s_dep_at, retirement, s_repl, guide, notes))) => {
            let deprecated_at = dep_at.or(Some(s_dep_at));
            let replacement = repl.or(s_repl);
            let reason = reason.or(notes.clone());
            let status = if Utc::now() >= retirement {
                DeprecationStatus::Retired
            } else if is_dep || deprecated_at.is_some() {
                DeprecationStatus::from_columns(deprecated_at, replacement)
            } else {
                st
            };
            (
                deprecated_at,
                reason,
                replacement,
                is_dep,
                status,
                Some(retirement),
                guide,
                notes,
            )
        }
        (Some((dep_at, reason, repl, is_dep, st)), None) => {
            let status = if is_dep || dep_at.is_some() {
                DeprecationStatus::from_columns(dep_at, repl)
            } else {
                st
            };
            (dep_at, reason, repl, is_dep, status, None, None, None)
        }
        (None, Some((s_dep_at, retirement, s_repl, guide, notes))) => {
            let status = if Utc::now() >= retirement {
                DeprecationStatus::Retired
            } else {
                DeprecationStatus::from_columns(Some(s_dep_at), s_repl)
            };
            (
                Some(s_dep_at),
                notes.clone(),
                s_repl,
                true,
                status,
                Some(retirement),
                guide,
                notes,
            )
        }
        (None, None) => (
            None,
            None,
            None,
            false,
            DeprecationStatus::Active,
            None,
            None,
            None,
        ),
    };

    // Schedule retirement can still override to Retired for Issue #65 consumers.
    if let Some(retirement) = retirement_at {
        if Utc::now() >= retirement && deprecated_at.is_some() {
            status = DeprecationStatus::Retired;
        }
    }

    let days_remaining = retirement_at.map(|retirement| {
        let now = Utc::now();
        if retirement > now {
            (retirement - now).num_days()
        } else {
            0
        }
    });

    let replacement_contract_id = match replacement_uuid {
        Some(id) => Some(resolve_contract_selector(&state, id).await?),
        None => None,
    };

    let replacement_lineage =
        build_replacement_lineage(&state, replacement_uuid, &contract_id).await?;
    let warnings = build_lineage_warnings(
        &status,
        &contract_id,
        replacement_contract_id.as_deref(),
        &replacement_lineage,
        deprecation_reason.as_deref(),
    );

    Ok(Json(DeprecationInfo {
        contract_id,
        status,
        deprecated_at,
        retirement_at,
        replacement_contract_id,
        migration_guide_url,
        notes,
        deprecation_reason,
        days_remaining,
        dependents_notified,
        replacement_lineage,
        warnings,
    }))
}

#[utoipa::path(
    post,
    path = "/api/contracts/{id}/deprecate",
    params(
        ("id" = String, Path, description = "Contract identifier")
    ),
    request_body = DeprecateContractRequest,
    responses(
        (status = 200, description = "Contract deprecated successfully", body = DeprecationInfo),
        (status = 404, description = "Contract not found"),
        (status = 400, description = "Invalid input or missing migration path")
    ),
    tag = "Maintenance"
)]
pub async fn deprecate_contract(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<DeprecateContractRequest>,
) -> ApiResult<Json<DeprecationInfo>> {
    let (contract_uuid, contract_id) = fetch_contract_identity(&state, &id).await?;

    let reason = req
        .deprecation_reason
        .clone()
        .or_else(|| req.notes.clone());

    if req.migration_guide_url.is_none() && req.replacement_contract_id.is_none() && reason.is_none()
    {
        return Err(ApiError::bad_request(
            "MissingMigrationPath",
            "Provide replacement_contract_id, migration_guide_url, or deprecation_reason",
        ));
    }

    if req.retirement_at <= Utc::now() {
        return Err(ApiError::bad_request(
            "InvalidRetirementDate",
            "retirement_at must be in the future",
        ));
    }

    let replacement_uuid = if let Some(ref selector) = req.replacement_contract_id {
        let uuid = fetch_contract_uuid(&state, selector).await?;
        if uuid == contract_uuid {
            return Err(ApiError::bad_request(
                "InvalidReplacement",
                "replacement_contract_id cannot reference the same contract",
            ));
        }
        Some(uuid)
    } else {
        None
    };

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| db_internal_error("begin deprecate tx", err))?;

    sqlx::query(
        "INSERT INTO contract_deprecations (contract_id, retirement_at, replacement_contract_id, migration_guide_url, notes) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (contract_id) DO UPDATE SET \
           retirement_at = EXCLUDED.retirement_at, \
           replacement_contract_id = EXCLUDED.replacement_contract_id, \
           migration_guide_url = EXCLUDED.migration_guide_url, \
           notes = EXCLUDED.notes, \
           updated_at = NOW()",
    )
    .bind(contract_uuid)
    .bind(req.retirement_at)
    .bind(replacement_uuid)
    .bind(&req.migration_guide_url)
    .bind(&reason)
    .execute(&mut *tx)
    .await
    .map_err(|err| db_internal_error("upsert deprecation schedule", err))?;

    // Denormalize onto contracts for list/search (Issue #1090).
    sqlx::query(
        "UPDATE contracts SET \
            deprecated_at = COALESCE(deprecated_at, NOW()), \
            deprecation_reason = $2, \
            replacement_contract_id = $3, \
            is_deprecated = TRUE, \
            updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(contract_uuid)
    .bind(&reason)
    .bind(replacement_uuid)
    .execute(&mut *tx)
    .await
    .map_err(|err| db_internal_error("update contract deprecation columns", err))?;

    tx.commit()
        .await
        .map_err(|err| db_internal_error("commit deprecate tx", err))?;

    notify_dependents(&state, contract_uuid, &contract_id, req.retirement_at).await?;

    // Best-effort ES reindex so search paths stay consistent.
    reindex_contract_search(&state, contract_uuid).await;

    get_deprecation_info(State(state), Path(contract_id)).await
}

#[utoipa::path(
    post,
    path = "/api/contracts/{id}/undeprecate",
    params(
        ("id" = String, Path, description = "Contract identifier")
    ),
    request_body = UndeprecateContractRequest,
    responses(
        (status = 200, description = "Contract reactivated successfully", body = DeprecationInfo),
        (status = 400, description = "Override flag required to reactivate"),
        (status = 404, description = "Contract not found")
    ),
    tag = "Maintenance"
)]
pub async fn undeprecate_contract(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<UndeprecateContractRequest>,
) -> ApiResult<Json<DeprecationInfo>> {
    let (contract_uuid, contract_id) = fetch_contract_identity(&state, &id).await?;

    let is_deprecated: bool = sqlx::query_scalar(
        "SELECT COALESCE(is_deprecated, FALSE) FROM contracts WHERE id = $1",
    )
    .bind(contract_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch is_deprecated", err))?;

    if !is_deprecated {
        return get_deprecation_info(State(state), Path(contract_id)).await;
    }

    if !req.has_override() {
        return Err(ApiError::bad_request(
            "OverrideRequired",
            "Reactivating a deprecated contract requires override=true (or force=true)",
        ));
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|err| db_internal_error("begin undeprecate tx", err))?;

    sqlx::query(
        "UPDATE contracts SET \
            deprecated_at = NULL, \
            deprecation_reason = NULL, \
            replacement_contract_id = NULL, \
            is_deprecated = FALSE, \
            updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(contract_uuid)
    .execute(&mut *tx)
    .await
    .map_err(|err| db_internal_error("clear contract deprecation columns", err))?;

    sqlx::query("DELETE FROM contract_deprecations WHERE contract_id = $1")
        .bind(contract_uuid)
        .execute(&mut *tx)
        .await
        .map_err(|err| db_internal_error("delete deprecation schedule", err))?;

    tx.commit()
        .await
        .map_err(|err| db_internal_error("commit undeprecate tx", err))?;

    reindex_contract_search(&state, contract_uuid).await;

    get_deprecation_info(State(state), Path(contract_id)).await
}

async fn reindex_contract_search(state: &AppState, contract_uuid: Uuid) {
    if let Ok(Some(contract)) =
        sqlx::query_as::<_, shared::Contract>("SELECT * FROM contracts WHERE id = $1")
            .bind(contract_uuid)
            .fetch_optional(&state.db)
            .await
    {
        let _ = state.search.index_contract(&contract, None).await;
    }
}

async fn build_replacement_lineage(
    state: &AppState,
    mut next: Option<Uuid>,
    origin_contract_id: &str,
) -> ApiResult<Vec<String>> {
    let mut lineage = Vec::new();
    let mut seen = HashSet::new();
    seen.insert(origin_contract_id.to_string());

    // Cap depth to avoid pathological graphs.
    for _ in 0..16 {
        let Some(id) = next else {
            break;
        };
        let row = sqlx::query_as::<_, (String, Option<Uuid>)>(
            "SELECT contract_id, replacement_contract_id FROM contracts WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch replacement lineage", err))?;

        let Some((selector, replacement)) = row else {
            break;
        };
        if !seen.insert(selector.clone()) {
            lineage.push(format!("{selector} (cycle detected)"));
            break;
        }
        lineage.push(selector);
        next = replacement;
    }

    Ok(lineage)
}

fn build_lineage_warnings(
    status: &DeprecationStatus,
    contract_id: &str,
    replacement: Option<&str>,
    lineage: &[String],
    reason: Option<&str>,
) -> Vec<String> {
    let mut warnings = Vec::new();
    match status {
        DeprecationStatus::Active => {}
        DeprecationStatus::Deprecated => {
            warnings.push(format!(
                "Contract {contract_id} is deprecated and has no replacement successor"
            ));
        }
        DeprecationStatus::Superseded => {
            if let Some(repl) = replacement {
                warnings.push(format!(
                    "Contract {contract_id} is superseded; resolve to {repl} instead"
                ));
            } else {
                warnings.push(format!("Contract {contract_id} is superseded"));
            }
        }
        DeprecationStatus::Retired => {
            warnings.push(format!(
                "Contract {contract_id} is retired and should not be used for new deployments"
            ));
        }
    }
    if let Some(reason) = reason {
        if !reason.is_empty() {
            warnings.push(format!("Deprecation reason: {reason}"));
        }
    }
    if lineage.len() > 1 {
        warnings.push(format!(
            "Replacement lineage: {} → {}",
            contract_id,
            lineage.join(" → ")
        ));
    }
    warnings
}

async fn notify_dependents(
    state: &AppState,
    deprecated_id: Uuid,
    contract_id: &str,
    retirement_at: DateTime<Utc>,
) -> ApiResult<()> {
    let has_dep_contract_id = column_exists(
        state,
        "contract_static_dependencies",
        "dependency_contract_id",
    )
    .await?;
    let has_dep_name =
        column_exists(state, "contract_static_dependencies", "dependency_name").await?;
    let has_package_name =
        column_exists(state, "contract_static_dependencies", "package_name").await?;

    let dependents: Vec<Uuid> = if has_dep_contract_id {
        sqlx::query_scalar(
            "SELECT DISTINCT contract_id FROM contract_static_dependencies WHERE dependency_contract_id = $1",
        )
        .bind(deprecated_id)
        .fetch_all(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch dependents", err))?
    } else if has_dep_name || has_package_name {
        let name_column = if has_dep_name {
            "dependency_name"
        } else {
            "package_name"
        };
        let sql = format!(
            "SELECT DISTINCT cd.contract_id \
             FROM contract_static_dependencies cd \
             JOIN contracts c ON c.name = cd.{name_column} \
             WHERE c.contract_id = $1",
        );
        sqlx::query_scalar(&sql)
            .bind(contract_id)
            .fetch_all(&state.db)
            .await
            .map_err(|err| db_internal_error("fetch dependents", err))?
    } else {
        Vec::new()
    };

    if dependents.is_empty() {
        return Ok(());
    }

    for dependent in dependents {
        let message = format!(
            "Contract {} has been deprecated and will retire on {}",
            contract_id,
            retirement_at.to_rfc3339()
        );

        let _ = sqlx::query(
            "INSERT INTO contract_deprecation_notifications (contract_id, deprecated_contract_id, message) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (contract_id, deprecated_contract_id) DO NOTHING",
        )
        .bind(dependent)
        .bind(deprecated_id)
        .bind(&message)
        .execute(&state.db)
        .await
        .map_err(|err| db_internal_error("insert notification", err))?;
    }

    Ok(())
}

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

async fn fetch_contract_uuid(state: &AppState, contract_id: &str) -> ApiResult<Uuid> {
    if let Ok(uuid) = Uuid::parse_str(contract_id) {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM contracts WHERE id = $1)")
                .bind(uuid)
                .fetch_one(&state.db)
                .await
                .map_err(|err| db_internal_error("fetch contract", err))?;
        if exists {
            return Ok(uuid);
        }
        return Err(ApiError::not_found(
            "ContractNotFound",
            format!("Contract '{}' not found", contract_id),
        ));
    }

    let uuid = sqlx::query_scalar::<_, Uuid>("SELECT id FROM contracts WHERE contract_id = $1")
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch contract", err))?
        .ok_or_else(|| {
            ApiError::not_found(
                "ContractNotFound",
                format!("Contract '{}' not found", contract_id),
            )
        })?;

    Ok(uuid)
}

async fn resolve_contract_selector(state: &AppState, id: Uuid) -> ApiResult<String> {
    let selector = sqlx::query_scalar::<_, String>("SELECT contract_id FROM contracts WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|err| db_internal_error("resolve replacement selector", err))?
        .unwrap_or_else(|| id.to_string());
    Ok(selector)
}

fn db_internal_error(operation: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = operation, error = ?err, "database operation failed");
    ApiError::internal("Database operation failed")
}

async fn column_exists(state: &AppState, table: &str, column: &str) -> ApiResult<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = $1 AND column_name = $2)",
    )
    .bind(table)
    .bind(column)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("check column", err))?;

    Ok(exists)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_from_columns_covers_transitions() {
        assert_eq!(
            DeprecationStatus::from_columns(None, None),
            DeprecationStatus::Active
        );
        assert_eq!(
            DeprecationStatus::from_columns(Some(Utc::now()), None),
            DeprecationStatus::Deprecated
        );
        assert_eq!(
            DeprecationStatus::from_columns(Some(Utc::now()), Some(Uuid::nil())),
            DeprecationStatus::Superseded
        );
    }

    #[test]
    fn undeprecate_requires_override_flag() {
        assert!(!UndeprecateContractRequest {
            r#override: false,
            force: false
        }
        .has_override());
        assert!(UndeprecateContractRequest {
            r#override: true,
            force: false
        }
        .has_override());
        assert!(UndeprecateContractRequest {
            r#override: false,
            force: true
        }
        .has_override());
    }

    #[test]
    fn lineage_warnings_include_successor_chain() {
        let warnings = build_lineage_warnings(
            &DeprecationStatus::Superseded,
            "C_OLD",
            Some("C_NEW"),
            &["C_NEW".into(), "C_NEWER".into()],
            Some("security advisory"),
        );
        assert!(warnings.iter().any(|w| w.contains("superseded")));
        assert!(warnings.iter().any(|w| w.contains("security advisory")));
        assert!(warnings.iter().any(|w| w.contains("C_NEW → C_NEWER")));
    }
}
