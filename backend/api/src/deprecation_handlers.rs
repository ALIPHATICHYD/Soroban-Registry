use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use shared::{
    DeprecateContractRequest, DeprecationInfo, DeprecationStatus, DeprecationWarning,
};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

// ─── Public helper ────────────────────────────────────────────────────────────

/// Build a `DeprecationWarning` from a raw deprecation record.
///
/// Returns `None` when the contract has no deprecation record.
pub async fn build_deprecation_warning(
    state: &AppState,
    contract_uuid: Uuid,
    replacement_contract_id: Option<String>,
) -> Option<DeprecationWarning> {
    #[allow(clippy::type_complexity)]
    let record: Option<(
        DateTime<Utc>,
        DateTime<Utc>,
        Option<Uuid>,
        Option<String>,
        Option<String>,
        Option<i32>,
    )> = sqlx::query_as(
        "SELECT deprecated_at, retirement_at, replacement_contract_id, \
                migration_guide_url, deprecated_reason, grace_period_days \
         FROM contract_deprecations WHERE contract_id = $1",
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let (deprecated_at, retirement_at, replacement_id, guide_url, reason, grace_period_days) =
        record?;

    let now = Utc::now();
    let days_until_retirement = if retirement_at > now {
        (retirement_at - now).num_days()
    } else {
        0
    };

    let resolved_replacement = replacement_contract_id.or_else(|| {
        replacement_id.map(|id| {
            // Best-effort: resolve UUID → contract_id string. Falls back to UUID string.
            id.to_string()
        })
    });

    let message = reason.clone().unwrap_or_else(|| {
        format!(
            "This contract is deprecated and will be retired on {}.",
            retirement_at.format("%Y-%m-%d")
        )
    });

    Some(DeprecationWarning {
        message,
        deprecated_at,
        retirement_at,
        days_until_retirement,
        replacement_contract_id: resolved_replacement,
        migration_guide_url: guide_url,
        grace_period_days,
    })
}

// ─── GET /api/contracts/:id/deprecation-info ──────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/contracts/{id}/deprecation-info",
    params(
        ("id" = String, Path, description = "Contract identifier (UUID or contract_id)")
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

    let record = sqlx::query_as::<
        _,
        (
            DateTime<Utc>,
            DateTime<Utc>,
            Option<Uuid>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<i32>,
        ),
    >(
        "SELECT deprecated_at, retirement_at, replacement_contract_id, migration_guide_url, \
                notes, deprecated_reason, grace_period_days \
         FROM contract_deprecations WHERE contract_id = $1",
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch deprecation", err))?;

    let dependents_notified: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_deprecation_notifications WHERE deprecated_contract_id = $1",
    )
    .bind(contract_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("count notifications", err))?;

    if let Some((
        deprecated_at,
        retirement_at,
        replacement_id,
        guide_url,
        notes,
        deprecated_reason,
        grace_period_days,
    )) = record
    {
        let now = Utc::now();
        let status = if now >= retirement_at {
            DeprecationStatus::Retired
        } else {
            DeprecationStatus::Deprecated
        };
        let days_remaining = Some(if retirement_at > now {
            (retirement_at - now).num_days()
        } else {
            0
        });

        let replacement_contract_id = replacement_id.map(|id| id.to_string());

        return Ok(Json(DeprecationInfo {
            contract_id,
            status,
            deprecated_at: Some(deprecated_at),
            retirement_at: Some(retirement_at),
            replacement_contract_id,
            migration_guide_url: guide_url,
            notes,
            deprecated_reason,
            grace_period_days,
            days_remaining,
            dependents_notified,
        }));
    }

    Ok(Json(DeprecationInfo {
        contract_id,
        status: DeprecationStatus::Active,
        deprecated_at: None,
        retirement_at: None,
        replacement_contract_id: None,
        migration_guide_url: None,
        notes: None,
        deprecated_reason: None,
        grace_period_days: None,
        days_remaining: None,
        dependents_notified,
    }))
}

// ─── POST /api/contracts/:id/deprecate ────────────────────────────────────────

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

    if req.migration_guide_url.is_none() && req.replacement_contract_id.is_none() {
        return Err(ApiError::bad_request(
            "MissingMigrationPath",
            "Provide replacement_contract_id or migration_guide_url",
        ));
    }

    if req.retirement_at <= Utc::now() {
        return Err(ApiError::bad_request(
            "InvalidRetirementDate",
            "retirement_at must be in the future",
        ));
    }

    if let Some(days) = req.grace_period_days {
        if days <= 0 {
            return Err(ApiError::bad_request(
                "InvalidGracePeriod",
                "grace_period_days must be a positive integer",
            ));
        }
    }

    let replacement_uuid = if let Some(ref selector) = req.replacement_contract_id {
        Some(fetch_contract_uuid(&state, selector).await?)
    } else {
        None
    };

    // Upsert the deprecation record (now includes reason and grace period)
    sqlx::query(
        "INSERT INTO contract_deprecations \
            (contract_id, retirement_at, replacement_contract_id, migration_guide_url, notes, \
             deprecated_reason, grace_period_days) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         ON CONFLICT (contract_id) DO UPDATE SET \
           retirement_at             = EXCLUDED.retirement_at, \
           replacement_contract_id   = EXCLUDED.replacement_contract_id, \
           migration_guide_url       = EXCLUDED.migration_guide_url, \
           notes                     = EXCLUDED.notes, \
           deprecated_reason         = EXCLUDED.deprecated_reason, \
           grace_period_days         = EXCLUDED.grace_period_days, \
           updated_at                = NOW()",
    )
    .bind(contract_uuid)
    .bind(req.retirement_at)
    .bind(replacement_uuid)
    .bind(&req.migration_guide_url)
    .bind(&req.notes)
    .bind(&req.deprecated_reason)
    .bind(req.grace_period_days)
    .execute(&state.db)
    .await
    .map_err(|err| db_internal_error("upsert deprecation", err))?;

    // Set is_deprecated flag on the contracts row for fast filtering
    sqlx::query("UPDATE contracts SET is_deprecated = TRUE WHERE id = $1")
        .bind(contract_uuid)
        .execute(&state.db)
        .await
        .map_err(|err| db_internal_error("set is_deprecated flag", err))?;

    notify_dependents(&state, contract_uuid, &contract_id, req.retirement_at).await?;

    get_deprecation_info(State(state), Path(contract_id)).await
}

// ─── DELETE /api/contracts/:id/deprecate (undeprecate) ───────────────────────

#[utoipa::path(
    delete,
    path = "/api/contracts/{id}/deprecate",
    params(
        ("id" = String, Path, description = "Contract identifier")
    ),
    responses(
        (status = 200, description = "Contract undeprecated successfully", body = DeprecationInfo),
        (status = 404, description = "Contract not found")
    ),
    tag = "Maintenance"
)]
pub async fn undeprecate_contract(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<DeprecationInfo>> {
    let (contract_uuid, contract_id) = fetch_contract_identity(&state, &id).await?;

    // Remove the deprecation record
    sqlx::query("DELETE FROM contract_deprecations WHERE contract_id = $1")
        .bind(contract_uuid)
        .execute(&state.db)
        .await
        .map_err(|err| db_internal_error("delete deprecation", err))?;

    // Clear is_deprecated flag
    sqlx::query("UPDATE contracts SET is_deprecated = FALSE WHERE id = $1")
        .bind(contract_uuid)
        .execute(&state.db)
        .await
        .map_err(|err| db_internal_error("clear is_deprecated flag", err))?;

    get_deprecation_info(State(state), Path(contract_id)).await
}

// ─── POST /api/admin/deprecation/purge-expired ────────────────────────────────

/// Hard-delete contracts whose grace period has elapsed.
///
/// This endpoint is intended to be called by a scheduled job (cron / k8s CronJob).
/// It returns the list of contract IDs that were permanently deleted.
#[utoipa::path(
    post,
    path = "/api/admin/deprecation/purge-expired",
    responses(
        (status = 200, description = "Expired contracts purged", body = Object),
        (status = 500, description = "Internal server error")
    ),
    tag = "Admin"
)]
pub async fn purge_expired_deprecated_contracts(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    // Find contracts whose grace period has fully elapsed:
    //   deprecated_at + grace_period_days < NOW()
    let expired: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT c.id, c.contract_id \
         FROM contracts c \
         JOIN contract_deprecations cd ON cd.contract_id = c.id \
         WHERE cd.grace_period_days IS NOT NULL \
           AND (cd.deprecated_at + (cd.grace_period_days || ' days')::INTERVAL) < NOW()",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch expired deprecations", err))?;

    let count = expired.len();
    let mut deleted_ids: Vec<String> = Vec::with_capacity(count);

    for (uuid, cid) in expired {
        // ON DELETE CASCADE on contract_deprecations and related tables will clean
        // up deprecation records and notifications automatically.
        sqlx::query("DELETE FROM contracts WHERE id = $1")
            .bind(uuid)
            .execute(&state.db)
            .await
            .map_err(|err| db_internal_error("hard-delete contract", err))?;

        tracing::info!(
            contract_id = %cid,
            uuid = %uuid,
            "Hard-deleted contract: grace period expired"
        );
        deleted_ids.push(cid);
    }

    Ok(Json(serde_json::json!({
        "purged": count,
        "contract_ids": deleted_ids,
        "purged_at": Utc::now(),
    })))
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

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
            "SELECT DISTINCT contract_id FROM contract_static_dependencies \
             WHERE dependency_contract_id = $1",
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
            "INSERT INTO contract_deprecation_notifications \
                (contract_id, deprecated_contract_id, message) \
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

pub(crate) async fn fetch_contract_identity(
    state: &AppState,
    id: &str,
) -> ApiResult<(Uuid, String)> {
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
        return Ok(uuid);
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

fn db_internal_error(operation: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = operation, error = ?err, "database operation failed");
    ApiError::internal("Database operation failed")
}

async fn column_exists(state: &AppState, table: &str, column: &str) -> ApiResult<bool> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM information_schema.columns \
          WHERE table_name = $1 AND column_name = $2)",
    )
    .bind(table)
    .bind(column)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("check column", err))?;

    Ok(exists)
}
