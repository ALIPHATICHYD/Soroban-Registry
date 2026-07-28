//! Pricing-plan CRUD per contract.
//!
//! Authorization: only the contract owner (publisher) can create or
//! update plans; reads are public so that browsing the marketplace
//! doesn't require auth.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    auth::AuthenticatedUser,
    error::{ApiError, ApiResult},
    marketplace::models::{CreatePricingPlan, PricingPlan, UpdatePricingPlan},
    state::AppState,
};

const ALLOWED_BILLING_PERIODS: &[&str] = &["monthly", "one_time"];

/// POST /api/contracts/{contract_id}/pricing-plans
pub async fn create_plan(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(contract_id): Path<Uuid>,
    Json(req): Json<CreatePricingPlan>,
) -> ApiResult<(StatusCode, Json<PricingPlan>)> {
    assert_contract_owner(&state, &user, contract_id).await?;

    if req.name.trim().is_empty() {
        return Err(ApiError::bad_request_msg("plan name cannot be empty"));
    }
    if req.price_cents < 0 {
        return Err(ApiError::bad_request_msg("price_cents must be >= 0"));
    }
    if !ALLOWED_BILLING_PERIODS.contains(&req.billing_period.as_str()) {
        return Err(ApiError::bad_request_msg(
            "billing_period must be one of 'monthly' or 'one_time'",
        ));
    }

    let features = if req.features.is_null() {
        serde_json::json!([])
    } else {
        req.features
    };

    let plan = sqlx::query_as::<_, PricingPlan>(
        r#"
        INSERT INTO contract_pricing_plans
            (contract_id, name, description, price_cents, currency, billing_period,
             call_quota, features, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, TRUE)
        RETURNING id, contract_id, name, description, price_cents, currency,
                  billing_period, call_quota, features, is_active,
                  created_at, updated_at
        "#,
    )
    .bind(contract_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(req.price_cents)
    .bind(&req.currency)
    .bind(&req.billing_period)
    .bind(req.call_quota)
    .bind(features)
    .fetch_one(&state.db)
    .await
    .map_err(map_unique_violation)?;

    Ok((StatusCode::CREATED, Json(plan)))
}

/// GET /api/contracts/{contract_id}/pricing-plans
pub async fn list_plans(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<Vec<PricingPlan>>> {
    let plans = sqlx::query_as::<_, PricingPlan>(
        r#"
        SELECT id, contract_id, name, description, price_cents, currency,
               billing_period, call_quota, features, is_active,
               created_at, updated_at
        FROM contract_pricing_plans
        WHERE contract_id = $1 AND is_active
        ORDER BY price_cents ASC, name ASC
        "#,
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(plans))
}

/// PATCH /api/contracts/{contract_id}/pricing-plans/{plan_id}
pub async fn update_plan(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path((contract_id, plan_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdatePricingPlan>,
) -> ApiResult<Json<PricingPlan>> {
    assert_contract_owner(&state, &user, contract_id).await?;

    // Pull current row so we can apply a partial patch in pure SQL with
    // COALESCE — same idiom used elsewhere in this codebase.
    let plan = sqlx::query_as::<_, PricingPlan>(
        r#"
        UPDATE contract_pricing_plans
        SET
            description = COALESCE($3, description),
            price_cents = COALESCE($4, price_cents),
            features    = COALESCE($5, features),
            is_active   = COALESCE($6, is_active)
        WHERE id = $1 AND contract_id = $2
        RETURNING id, contract_id, name, description, price_cents, currency,
                  billing_period, call_quota, features, is_active,
                  created_at, updated_at
        "#,
    )
    .bind(plan_id)
    .bind(contract_id)
    .bind(req.description)
    .bind(req.price_cents)
    .bind(req.features)
    .bind(req.is_active)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("plan_not_found", "pricing plan not found"))?;

    Ok(Json(plan))
}

async fn assert_contract_owner(
    state: &AppState,
    user: &AuthenticatedUser,
    contract_id: Uuid,
) -> ApiResult<()> {
    let owner_id: Option<Uuid> =
        sqlx::query_scalar("SELECT publisher_id FROM contracts WHERE id = $1")
            .bind(contract_id)
            .fetch_optional(&state.db)
            .await?;

    match owner_id {
        Some(pid) if pid == user.publisher_id => Ok(()),
        Some(_) => Err(ApiError::forbidden(
            "only the contract owner can modify its pricing plans",
        )),
        None => Err(ApiError::not_found(
            "contract_not_found",
            "contract not found",
        )),
    }
}

fn map_unique_violation(e: sqlx::Error) -> ApiError {
    if let sqlx::Error::Database(db_err) = &e {
        // Postgres unique_violation = 23505
        if db_err.code().as_deref() == Some("23505") {
            return ApiError::conflict(
                "duplicate_plan",
                "a pricing plan with that name already exists for this contract",
            );
        }
    }
    e.into()
}
