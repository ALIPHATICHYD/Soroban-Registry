use crate::validation::extractors::ValidatedJson;
use crate::{
    auth::AuthenticatedUser,
    disaster_recovery_models::SendNotificationRequest,
    error::{ApiError, ApiResult},
    notification_handlers,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use shared::models::{
    AddCollaborativeCommentRequest, CollaborativeComment, CollaborativeReview,
    CollaborativeReviewDetails, CollaborativeReviewStatus, CollaborativeReviewer,
    CreateCollaborativeReviewRequest, UpdateReviewerStatusRequest,
};
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;

/// POST /api/reviews/collaborative
/// Starts a new collaborative review session for a contract version.
pub async fn create_collaborative_review(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    ValidatedJson(payload): ValidatedJson<CreateCollaborativeReviewRequest>,
) -> ApiResult<Json<CollaborativeReview>> {
    let mut tx = state.db.begin().await.map_err(ApiError::from)?;

    let review = sqlx::query_as::<_, CollaborativeReview>(
        r#"
        INSERT INTO collaborative_reviews (contract_id, version, status)
        VALUES ($1, $2, 'pending')
        RETURNING *
        "#,
    )
    .bind(payload.contract_id)
    .bind(&payload.version)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::from)?;

    for user_id in payload.reviewer_ids {
        sqlx::query(
            r#"
            INSERT INTO collaborative_reviewers (review_id, user_id, status)
            VALUES ($1, $2, 'pending')
            "#,
        )
        .bind(review.id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::from)?;

        // Trigger notification (Email log simulation)
        let _ = notification_handlers::send_notification(
            State(state.clone()),
            crate::validation::ValidatedJson(SendNotificationRequest {
                contract_id: payload.contract_id,
                notification_type: "review_assigned".to_string(),
                recipients: vec![user_id.to_string()],
                template_variables: {
                    let mut map = HashMap::new();
                    map.insert("contract_name".to_string(), "Selected Contract".to_string());
                    map.insert("version".to_string(), payload.version.clone());
                    map
                },
                priority: Some("normal".to_string()),
            }),
        )
        .await;
    }

    tx.commit().await.map_err(ApiError::from)?;

    Ok(Json(review))
}

/// POST /api/reviews/collaborative/:id/comment
/// Adds a comment to a review, potentially inline (code/ABI).
pub async fn add_collaborative_comment(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(review_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<AddCollaborativeCommentRequest>,
) -> ApiResult<Json<CollaborativeComment>> {
    let user_id = user.id;

    let comment = sqlx::query_as::<_, CollaborativeComment>(
        r#"
        INSERT INTO collaborative_review_comments 
        (review_id, user_id, content, line_number, file_path, abi_path, parent_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(review_id)
    .bind(user_id)
    .bind(&payload.content)
    .bind(payload.line_number)
    .bind(&payload.file_path)
    .bind(&payload.abi_path)
    .bind(payload.parent_id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::from)?;

    Ok(Json(comment))
}

/// PATCH /api/reviews/collaborative/:id/status
/// Updates the status of an individual reviewer.
pub async fn update_reviewer_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(review_id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<UpdateReviewerStatusRequest>,
) -> ApiResult<StatusCode> {
    let user_id = user.id;

    sqlx::query(
        r#"
        UPDATE collaborative_reviewers 
        SET status = $1, updated_at = NOW()
        WHERE review_id = $2 AND user_id = $3
        "#,
    )
    .bind(&payload.status)
    .bind(review_id)
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(ApiError::from)?;

    // Check if review can be finalized (all approved)
    let pending_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM collaborative_reviewers WHERE review_id = $1 AND status != 'approved'"
    )
    .bind(review_id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::from)?;

    if pending_count == 0 {
        sqlx::query("UPDATE collaborative_reviews SET status = 'approved' WHERE id = $1")
            .bind(review_id)
            .execute(&state.db)
            .await
            .map_err(ApiError::from)?;
    } else if payload.status == CollaborativeReviewStatus::ChangesRequested {
        sqlx::query("UPDATE collaborative_reviews SET status = 'changes_requested' WHERE id = $1")
            .bind(review_id)
            .execute(&state.db)
            .await
            .map_err(ApiError::from)?;
    }

    Ok(StatusCode::OK)
}

/// GET /api/reviews/collaborative/:id
/// Fetches all details for a collaborative review session.
pub async fn get_collaborative_review(
    State(state): State<AppState>,
    Path(review_id): Path<Uuid>,
) -> ApiResult<Json<CollaborativeReviewDetails>> {
    let review = sqlx::query_as::<_, CollaborativeReview>(
        "SELECT * FROM collaborative_reviews WHERE id = $1",
    )
    .bind(review_id)
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::from)?;

    let reviewers = sqlx::query_as::<_, CollaborativeReviewer>(
        "SELECT * FROM collaborative_reviewers WHERE review_id = $1",
    )
    .bind(review_id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::from)?;

    let comments = sqlx::query_as::<_, CollaborativeComment>(
        "SELECT * FROM collaborative_review_comments WHERE review_id = $1 ORDER BY created_at ASC",
    )
    .bind(review_id)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::from)?;

    Ok(Json(CollaborativeReviewDetails {
        review,
        reviewers,
        comments,
    }))
}
