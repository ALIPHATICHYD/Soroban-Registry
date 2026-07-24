use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use shared::models::{
    Contract, Contributor, ContributorWithStats, CreateContributorRequest, UpdateContributorRequest,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::db_internal_error,
    state::AppState,
};

/// Count contracts belonging to a contributor (via matching stellar_address -> publisher)
async fn contributor_contract_count(
    db: &sqlx::PgPool,
    stellar_address: &str,
) -> Result<i64, ApiError> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(c.id) FROM contracts c
         JOIN publishers p ON c.publisher_id = p.id
         WHERE p.stellar_address = $1",
    )
    .bind(stellar_address)
    .fetch_one(db)
    .await
    .map_err(|e| db_internal_error("count contributor contracts", e))?;

    Ok(count)
}

/// GET /api/contributors — list all contributor profiles with stats
pub async fn list_contributors(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ContributorWithStats>>> {
    let contributors: Vec<Contributor> =
        sqlx::query_as("SELECT * FROM contributors ORDER BY created_at DESC")
            .fetch_all(&state.db)
            .await
            .map_err(|e| db_internal_error("list contributors", e))?;

    let mut result = Vec::with_capacity(contributors.len());
    for c in contributors {
        let count = contributor_contract_count(&state.db, &c.stellar_address).await?;
        result.push(ContributorWithStats::from_contributor(c, count));
    }

    Ok(Json(result))
}

/// POST /api/contributors — create a contributor profile
pub async fn create_contributor(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateContributorRequest>,
) -> impl IntoResponse {
    if req.stellar_address.is_empty() {
        return ApiError::bad_request_with("InvalidRequest", "stellar_address is required")
            .into_response();
    }

    let links = req.links.unwrap_or_else(|| serde_json::json!({}));

    let contributor: Contributor = match sqlx::query_as(
        "INSERT INTO contributors (stellar_address, name, avatar_url, bio, links)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING *",
    )
    .bind(&req.stellar_address)
    .bind(&req.name)
    .bind(&req.avatar_url)
    .bind(&req.bio)
    .bind(&links)
    .fetch_one(&state.db)
    .await
    {
        Ok(c) => c,
        Err(sqlx::Error::Database(e))
            if e.constraint() == Some("contributors_stellar_address_key") =>
        {
            return ApiError::bad_request(
                "DuplicateAddress",
                "A contributor profile already exists for this stellar address",
            )
            .into_response();
        }
        Err(e) => return db_internal_error("create contributor", e).into_response(),
    };

    let count = match contributor_contract_count(&state.db, &contributor.stellar_address).await {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };

    (
        StatusCode::CREATED,
        Json(ContributorWithStats::from_contributor(contributor, count)),
    )
        .into_response()
}

/// GET /api/contributors/:id — get a contributor profile with stats
pub async fn get_contributor(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ContributorWithStats>> {
    let contributor: Contributor = sqlx::query_as("SELECT * FROM contributors WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => ApiError::not_found(
                "ContributorNotFound",
                format!("No contributor found with ID: {}", id),
            ),
            _ => db_internal_error("get contributor", e),
        })?;

    let count = contributor_contract_count(&state.db, &contributor.stellar_address).await?;

    Ok(Json(ContributorWithStats::from_contributor(
        contributor,
        count,
    )))
}

/// PUT /api/contributors/:id — update a contributor profile
pub async fn update_contributor(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<UpdateContributorRequest>,
) -> ApiResult<Json<ContributorWithStats>> {
    let contributor: Contributor = sqlx::query_as(
        "UPDATE contributors
         SET name       = COALESCE($1, name),
             avatar_url = COALESCE($2, avatar_url),
             bio        = COALESCE($3, bio),
             links      = COALESCE($4, links),
             updated_at = NOW()
         WHERE id = $5
         RETURNING *",
    )
    .bind(&req.name)
    .bind(&req.avatar_url)
    .bind(&req.bio)
    .bind(&req.links)
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => ApiError::not_found(
            "ContributorNotFound",
            format!("No contributor found with ID: {}", id),
        ),
        _ => db_internal_error("update contributor", e),
    })?;

    let count = contributor_contract_count(&state.db, &contributor.stellar_address).await?;

    Ok(Json(ContributorWithStats::from_contributor(
        contributor,
        count,
    )))
}

/// GET /api/contributors/:id/contracts — get contracts published by a contributor
pub async fn get_contributor_contracts(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<Contract>>> {
    // Verify contributor exists
    let contributor: Option<Contributor> =
        sqlx::query_as("SELECT * FROM contributors WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| db_internal_error("get contributor", e))?;

    let contributor = contributor.ok_or_else(|| {
        ApiError::not_found(
            "ContributorNotFound",
            format!("No contributor found with ID: {}", id),
        )
    })?;

    let contracts: Vec<Contract> = sqlx::query_as(
        "SELECT c.* FROM contracts c
         JOIN publishers p ON c.publisher_id = p.id
         WHERE p.stellar_address = $1
         ORDER BY c.created_at DESC",
    )
    .bind(&contributor.stellar_address)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_internal_error("get contributor contracts", e))?;

    Ok(Json(contracts))
}
