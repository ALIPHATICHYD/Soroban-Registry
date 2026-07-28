use crate::validation::extractors::ValidatedJson;
use crate::{
    auth::AuthClaims,
    error::{ApiError, ApiResult},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use shared::{
    CreateOrganizationRequest, InviteMemberRequest, Organization, OrganizationMember,
    OrganizationRole, UpdateOrganizationRequest,
};
use sqlx::postgres::PgRow;
use sqlx::PgPool;
use uuid::Uuid;

fn db_internal_error(operation: &str, err: sqlx::Error) -> ApiError {
    tracing::error!(operation = operation, error = ?err, "database operation failed");
    ApiError::internal("An unexpected database error occurred")
}

pub async fn create_organization(
    State(state): State<AppState>,
    claims: AuthClaims,
    ValidatedJson(payload): ValidatedJson<CreateOrganizationRequest>,
) -> ApiResult<(StatusCode, Json<Organization>)> {
    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| db_internal_error("begin_transaction", e))?;

    let publisher_id: Uuid =
        sqlx::query_scalar("SELECT id FROM publishers WHERE stellar_address = $1")
            .bind(&claims.sub)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| db_internal_error("get_publisher_id", e))?
            .ok_or_else(|| ApiError::unauthorized("Publisher not found for authenticated user"))?;

    // 2. Create organization
    let org: Organization = sqlx::query_as(
        r#"
        INSERT INTO organizations (name, slug, description, is_private)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(&payload.name)
    .bind(&payload.slug)
    .bind(&payload.description)
    .bind(payload.is_private.unwrap_or(true))
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| db_internal_error("create_organization", e))?;

    // 3. Add creator as Admin
    sqlx::query(
        r#"
        INSERT INTO organization_members (organization_id, publisher_id, role)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(org.id)
    .bind(publisher_id)
    .bind(OrganizationRole::Admin)
    .execute(&mut *tx)
    .await
    .map_err(|e| db_internal_error("add_org_admin", e))?;

    tx.commit()
        .await
        .map_err::<crate::error::ApiError, _>(|e| db_internal_error("commit_transaction", e))?;

    Ok((StatusCode::CREATED, Json(org)))
}

pub async fn get_organization(
    State(state): State<AppState>,
    claims: Option<AuthClaims>,
    Path(slug_or_id): Path<String>,
) -> ApiResult<Json<Organization>> {
    let org_id = Uuid::parse_str(&slug_or_id).ok();

    let org = if let Some(id) = org_id {
        sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
    } else {
        sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE slug = $1")
            .bind(slug_or_id)
            .fetch_optional(&state.db)
            .await
    }
    .map_err(|e| db_internal_error("get_organization", e))?
    .ok_or_else(|| ApiError::not_found("OrganizationNotFound", "Organization not found"))?;

    if org.is_private {
        let is_member = if let Some(ref claims) = claims {
            check_org_role(&state.db, org.id, &claims.sub, OrganizationRole::Viewer)
                .await
                .is_ok()
        } else {
            false
        };

        if !is_member {
            return Err(ApiError::forbidden("Access denied to private organization"));
        }
    }

    Ok(Json(org))
}

pub async fn update_organization(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<UpdateOrganizationRequest>,
) -> ApiResult<Json<Organization>> {
    // Check if user is Admin of the org
    check_org_role(&state.db, id, &claims.sub, OrganizationRole::Admin).await?;

    let org: Organization = sqlx::query_as(
        r#"
        UPDATE organizations
        SET
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            is_private = COALESCE($3, is_private),
            updated_at = NOW()
        WHERE id = $4
        RETURNING *
        "#,
    )
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(payload.is_private)
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_internal_error("update_organization", e))?;

    Ok(Json(org))
}

pub async fn check_org_role(
    pool: &PgPool,
    org_id: Uuid,
    user_address: &str,
    min_role: OrganizationRole,
) -> ApiResult<OrganizationRole> {
    let member_row: Option<(OrganizationRole,)> = sqlx::query_as(
        r#"
        SELECT om.role
        FROM organization_members om
        JOIN publishers p ON om.publisher_id = p.id
        WHERE om.organization_id = $1 AND p.stellar_address = $2
        LIMIT 1
        "#,
    )
    .bind(org_id)
    .bind(user_address)
    .fetch_optional(pool)
    .await
    .map_err(|e| db_internal_error("check_org_role", e))?;

    let role = member_row.ok_or(StatusCode::FORBIDDEN)?.0;

    let has_access = match (min_role, &role) {
        (OrganizationRole::Admin, OrganizationRole::Admin) => true,
        (OrganizationRole::Member, OrganizationRole::Admin | OrganizationRole::Member) => true,
        (OrganizationRole::Viewer, _) => true,
        _ => false,
    };

    if has_access {
        Ok(role)
    } else {
        Err(ApiError::forbidden("Insufficient organization role"))
    }
}

pub async fn list_org_members(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<OrganizationMember>>> {
    // Check if user is a member of the org
    check_org_role(&state.db, id, &claims.sub, OrganizationRole::Viewer).await?;

    let members = sqlx::query_as::<_, OrganizationMember>(
        r#"
        SELECT organization_id, publisher_id, role, joined_at
        FROM organization_members
        WHERE organization_id = $1
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_internal_error("list_org_members", e))?;

    Ok(Json(members))
}

pub async fn invite_member(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<InviteMemberRequest>,
) -> ApiResult<StatusCode> {
    // Check if user is an Admin of the org
    check_org_role(&state.db, id, &claims.sub, OrganizationRole::Admin).await?;

    let inviter_id: Uuid =
        sqlx::query_scalar("SELECT id FROM publishers WHERE stellar_address = $1")
            .bind(&claims.sub)
            .fetch_one(&state.db)
            .await
            .map_err(|e| db_internal_error("get_inviter_id", e))?;

    let token = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + chrono::Duration::days(7);

    sqlx::query(
        r#"
        INSERT INTO organization_invitations (organization_id, email, role, token, inviter_id, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(id)
    .bind(&payload.email)
    .bind(payload.role)
    .bind(&token)
    .bind(inviter_id)
    .bind(expires_at)
    .execute(&state.db)
    .await
    .map_err::<crate::error::ApiError, _>(|e| db_internal_error("create_invitation", e))?;

    Ok(StatusCode::ACCEPTED)
}

pub async fn accept_invitation(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(token): Path<String>,
) -> ApiResult<StatusCode> {
    #[derive(sqlx::FromRow)]
    struct InviteRow {
        id: Uuid,
        organization_id: Uuid,
        role: OrganizationRole,
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| db_internal_error("begin_transaction", e))?;

    // 1. Validate invitation
    let invite = sqlx::query_as::<_, shared::OrganizationInvitation>(
        r#"
        SELECT id, organization_id, role
        FROM organization_invitations
        WHERE token = $1 AND accepted_at IS NULL AND expires_at > NOW()
        "#,
    )
    .bind(&token)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| db_internal_error("get_invitation", e))?
    .ok_or_else(|| ApiError::not_found("InvitationNotFound", "Invitation is invalid or expired"))?;

    let publisher_id: Uuid =
        sqlx::query_scalar("SELECT id FROM publishers WHERE stellar_address = $1")
            .bind(&claims.sub)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| db_internal_error("get_publisher_id", e))?;

    // 3. Add member
    sqlx::query(
        r#"
        INSERT INTO organization_members (organization_id, publisher_id, role)
        VALUES ($1, $2, $3)
        ON CONFLICT (organization_id, publisher_id) DO UPDATE SET role = EXCLUDED.role
        "#,
    )
    .bind(invite.organization_id)
    .bind(publisher_id)
    .bind(invite.role)
    .execute(&mut *tx)
    .await
    .map_err(|e| db_internal_error("add_member", e))?;

    // 4. Mark invitation as accepted
    sqlx::query("UPDATE organization_invitations SET accepted_at = NOW() WHERE id = $1")
        .bind(invite.id)
        .execute(&mut *tx)
        .await
        .map_err(|e| db_internal_error("mark_invite_accepted", e))?;

    tx.commit()
        .await
        .map_err::<crate::error::ApiError, _>(|e| db_internal_error("commit_transaction", e))?;

    Ok(StatusCode::OK)
}
