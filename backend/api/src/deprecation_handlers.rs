use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use shared::{
    DeprecateContractRequest, DeprecationInfo, DeprecationStatus, DeprecationWarning,
    UndeprecateContractRequest,
};
use std::collections::HashSet;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::ownership_transfer;
use crate::policy::{self, PolicyActor};
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
    .map_err(|err| db_internal_error("fetch deprecation schedule", err))?;

    let dependents_notified: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM contract_deprecation_notifications WHERE deprecated_contract_id = $1",
    )
    .bind(contract_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|err| db_internal_error("count notifications", err))?;

    // Contract columns (Issue #1090) are the source of truth for lifecycle status;
    // the contract_deprecations row (Issue #65/#1061) supplies the retirement
    // schedule, migration guide and grace period.
    let (deprecated_at_col, reason_col, replacement_col, is_deprecated_col) = match contract_row {
        Some((dep_at, reason, repl, is_dep, _status)) => (dep_at, reason, repl, is_dep),
        None => (None, None, None, false),
    };

    let (
        schedule_deprecated_at,
        retirement_at,
        schedule_replacement,
        migration_guide_url,
        notes,
        schedule_reason,
        grace_period_days,
    ) = match schedule {
        Some((dep_at, retirement, repl, guide, notes, reason, grace)) => (
            Some(dep_at),
            Some(retirement),
            repl,
            guide,
            notes,
            reason,
            grace,
        ),
        None => (None, None, None, None, None, None, None),
    };

    let deprecated_at = deprecated_at_col.or(schedule_deprecated_at);
    let replacement_uuid = replacement_col.or(schedule_replacement);
    let deprecated_reason = schedule_reason.or(reason_col).or_else(|| notes.clone());

    let status = if deprecated_at.is_none() && !is_deprecated_col {
        DeprecationStatus::Active
    } else if retirement_at.is_some_and(|retirement| Utc::now() >= retirement) {
        DeprecationStatus::Retired
    } else {
        DeprecationStatus::from_columns(deprecated_at, replacement_uuid)
    };

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
        deprecated_reason.as_deref(),
    );

    Ok(Json(DeprecationInfo {
        contract_id,
        status,
        deprecated_at,
        retirement_at,
        replacement_contract_id,
        migration_guide_url,
        notes,
        deprecated_reason,
        grace_period_days,
        days_remaining,
        dependents_notified,
        replacement_lineage,
        warnings,
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
    actor: PolicyActor,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<DeprecateContractRequest>,
) -> ApiResult<Json<DeprecationInfo>> {
    let (contract_uuid, contract_id) = fetch_contract_identity(&state, &id).await?;
    policy::require_contract_owner(&state, &actor, contract_uuid).await?;
    verify_deprecation_signature(&actor, &contract_id, &req)?;

    let reason = req.deprecated_reason.clone().or_else(|| req.notes.clone());

    if req.migration_guide_url.is_none() && req.replacement_contract_id.is_none() && reason.is_none()
    {
        return Err(ApiError::bad_request(
            "MissingMigrationPath",
            "Provide replacement_contract_id, migration_guide_url, or deprecated_reason",
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

    // Upsert the deprecation record (retirement schedule, reason and grace period)
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
    .bind(&reason)
    .bind(req.grace_period_days)
    .execute(&mut *tx)
    .await
    .map_err(|err| db_internal_error("upsert deprecation schedule", err))?;

    // Denormalize onto contracts so list/search/trending can filter and surface
    // status without joining contract_deprecations (Issue #1090).
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

    // ── Emit contract.deprecated webhook (best-effort) ────────────────────────
    // Fetch the publisher_id for this contract so we can route the webhook to
    // the correct set of subscriptions.
    if let Ok(publisher_id) = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT publisher_id FROM contracts WHERE id = $1",
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map(|opt| opt.unwrap_or(uuid::Uuid::nil()))
    {
        if !publisher_id.is_nil() {
            crate::webhook_events::emit_webhook_event(
                &state.db,
                publisher_id,
                crate::webhook_events::EVENT_CONTRACT_DEPRECATED,
                serde_json::json!({
                    "contract_id": contract_id,
                    "contract_uuid": contract_uuid,
                    "deprecated_reason": reason,
                    "replacement_contract_id": req.replacement_contract_id,
                    "migration_guide_url": req.migration_guide_url,
                    "retirement_at": req.retirement_at,
                }),
            )
            .await;
        }
    }

    // Best-effort ES reindex so search paths stay consistent.
    reindex_contract_search(&state, contract_uuid).await;

    get_deprecation_info(State(state), Path(contract_id)).await
}

// ─── DELETE /api/contracts/:id/deprecate (undeprecate) ───────────────────────

#[utoipa::path(
    delete,
    path = "/api/contracts/{id}/deprecate",
    params(
        ("id" = String, Path, description = "Contract identifier"),
        UndeprecateContractRequest
    ),
    responses(
        (status = 200, description = "Contract undeprecated successfully", body = DeprecationInfo),
        (status = 400, description = "Override flag required to reactivate"),
        (status = 404, description = "Contract not found")
    ),
    tag = "Maintenance"
)]
pub async fn undeprecate_contract(
    State(state): State<AppState>,
    actor: PolicyActor,
    Path(id): Path<String>,
    Query(req): Query<UndeprecateContractRequest>,
) -> ApiResult<Json<DeprecationInfo>> {
    let (contract_uuid, contract_id) = fetch_contract_identity(&state, &id).await?;
    policy::require_contract_owner(&state, &actor, contract_uuid).await?;

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
    actor: PolicyActor,
) -> ApiResult<Json<serde_json::Value>> {
    actor.require_admin()?;

    let (count, deleted_ids) = crate::transaction::with_transaction(&state.db, "purge_expired", |mut tx| async move {
        // Find contracts whose grace period has fully elapsed:
        //   deprecated_at + grace_period_days < NOW()
        let expired: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT c.id, c.contract_id \
             FROM contracts c \
             JOIN contract_deprecations cd ON cd.contract_id = c.id \
             WHERE cd.grace_period_days IS NOT NULL \
               AND (cd.deprecated_at + (cd.grace_period_days || ' days')::INTERVAL) < NOW()",
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|err| db_internal_error("fetch expired deprecations", err))?;

        let count = expired.len();
        let mut deleted_ids: Vec<String> = Vec::with_capacity(count);

        for (uuid, cid) in expired {
            // ON DELETE CASCADE on contract_deprecations and related tables will clean
            // up deprecation records and notifications automatically.
            sqlx::query("DELETE FROM contracts WHERE id = $1")
                .bind(uuid)
                .execute(&mut *tx)
                .await
                .map_err(|err| db_internal_error("hard-delete contract", err))?;

            tracing::info!(
                contract_id = %cid,
                uuid = %uuid,
                "Hard-deleted contract: grace period expired"
            );
            deleted_ids.push(cid);
        }

        Ok(((count, deleted_ids), tx))
    }).await?;

    Ok(Json(serde_json::json!({
        "purged": count,
        "contract_ids": deleted_ids,
        "purged_at": Utc::now(),
    })))
}

// ─── Internal helpers ─────────────────────────────────────────────────────────

fn verify_deprecation_signature(
    actor: &PolicyActor,
    contract_id: &str,
    req: &DeprecateContractRequest,
) -> ApiResult<()> {
    let (payload, signature, signing_address) =
        match (&req.payload, &req.signature, &req.signing_address) {
            (None, None, None) => return Ok(()),
            (Some(payload), Some(signature), Some(signing_address)) => {
                (payload, signature, signing_address)
            }
            _ => {
                return Err(ApiError::bad_request(
                    "IncompleteSignatureEnvelope",
                    "payload, signature, and signing_address must be supplied together",
                ))
            }
        };

    actor.require_signature_identity(signing_address)?;
    if payload.action != "deprecate" || payload.contract_id != contract_id {
        return Err(ApiError::bad_request(
            "SignaturePayloadMismatch",
            "The signed payload does not describe this contract deprecation",
        ));
    }
    if !(16..=128).contains(&payload.nonce.len())
        || !payload
            .nonce
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ApiError::bad_request(
            "InvalidSignatureNonce",
            "The signature nonce must be 16-128 ASCII letters, digits, hyphens, or underscores",
        ));
    }

    let signed_at = DateTime::parse_from_rfc3339(&payload.timestamp)
        .map_err(|_| {
            ApiError::bad_request(
                "InvalidSignatureTimestamp",
                "The signature timestamp must be RFC 3339",
            )
        })?
        .timestamp();
    ownership_transfer::check_signature_freshness(signed_at, Utc::now().timestamp())?;

    let message = format!(
        "{}:{}:{}:{}",
        payload.action, payload.contract_id, payload.timestamp, payload.nonce
    );
    ownership_transfer::verify_transfer_signature(actor.stellar_address(), &message, signature)
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
