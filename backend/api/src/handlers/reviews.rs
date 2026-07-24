// ═══════════════════════════════════════════════════════════════════════════
// CONTRACT REVIEW SYSTEM HANDLERS
// ═══════════════════════════════════════════════════════════════════════════
//
// This module implements the complete review system for contracts:
// - Submit reviews (POST /contracts/:id/reviews)
// - Fetch reviews with sorting (GET /contracts/:id/reviews)
// - Vote on review helpfulness (POST /contracts/:id/reviews/:review_id/vote)
// - Flag reviews for moderation (POST /contracts/:id/reviews/:review_id/flag)
// - Moderate reviews (admin only) (POST /contracts/:id/reviews/:review_id/moderate)
// - Get rating aggregation (GET /contracts/:id/rating-stats)
//
// Moderation Workflow:
// - New reviews are created with status = 'pending'
// - Only 'approved' reviews are visible in public fetch endpoints
// - Admins can approve or reject pending reviews
// - Users can flag reviews for moderation
//
// Rating Aggregation:
// - Average rating is computed from all approved reviews
// - Rating distribution (1-5 stars) is tracked
// - Aggregation is computed dynamically on each request (can be cached for performance)
// ═══════════════════════════════════════════════════════════════════════════

use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;

use crate::{
    auth::AuthClaims,
    error::{ApiError, ApiResult},
    models::{
        ContractRatingStats, CreateReviewRequest, FlagReviewRequest, GetReviewsQuery,
        ModerateReviewRequest, RatingDistribution, ReviewResponse, ReviewSortBy, ReviewStatus,
        ReviewVoteRequest, ReviewVoteResponse,
    },
    state::AppState,
};

// ═══════════════════════════════════════════════════════════════════════════
// SUBMIT REVIEW
// ═══════════════════════════════════════════════════════════════════════════
// POST /api/contracts/:id/reviews
//
// Creates a new review for a contract. The review starts with 'pending' status
// and must be approved by an admin before becoming visible.
//
// Validation:
// - Rating must be between 1.0 and 5.0 (inclusive)
// - User must be authenticated (JWT required)
// - Contract must exist
// - User cannot submit duplicate reviews (one per contract per user)
// - If verified-user-only is enabled, user must have verified contracts
// ═══════════════════════════════════════════════════════════════════════════

/// Query parameter to enforce verified-user-only rule
#[derive(Debug, Clone, Deserialize)]
pub struct CreateReviewQuery {
    /// If true, only users with verified contracts can submit reviews
    #[serde(default)]
    pub verified_only: bool,
}

pub async fn create_review(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(query_params): Query<CreateReviewQuery>,
    claims: AuthClaims,
    ValidatedJson(payload): ValidatedJson<CreateReviewRequest>,
) -> ApiResult<impl IntoResponse> {
    // Validate rating bounds (1.0 - 5.0)
    if payload.rating < 1.0 || payload.rating > 5.0 {
        return Err(ApiError::bad_request(
            "InvalidRating",
            "Rating must be between 1.0 and 5.0".to_string(),
        ));
    }

    // Verify contract exists
    let contract_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM contracts WHERE id = $1)")
            .bind(contract_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "database error checking contract existence");
                ApiError::internal("Failed to verify contract existence")
            })?;

    if !contract_exists {
        return Err(ApiError::not_found(
            "ContractNotFound",
            format!("Contract with id {} not found", contract_id),
        ));
    }

    // If verified_only is enabled, check if user has verified contracts
    if query_params.verified_only {
        let has_verified = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM contracts 
                WHERE publisher_id = (SELECT id FROM publishers WHERE stellar_address = $1)
                AND is_verified = true
            )
            "#,
        )
        .bind(&claims.sub)
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "database error checking verified status");
            ApiError::internal("Failed to verify user status")
        })?;

        if !has_verified {
            return Err(ApiError::forbidden(
                "Only users with verified contracts can submit reviews",
            ));
        }
    }

    // Get publisher ID from user's stellar address
    let publisher_id = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT id FROM publishers WHERE stellar_address = $1",
    )
    .bind(&claims.sub)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "database error fetching publisher");
        ApiError::internal("Failed to fetch publisher information")
    })?;

    // If user doesn't have a publisher record, they can still review
    // but we'll use a null user_id (anonymous review)
    let user_id = publisher_id;

    // Check for duplicate review (user can only review once per contract)
    // Rejected reviews don't count - user can resubmit
    let duplicate_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM reviews 
            WHERE contract_id = $1 AND user_id = $2 AND status != 'rejected'
        )
        "#,
    )
    .bind(contract_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "database error checking duplicate review");
        ApiError::internal("Failed to check for duplicate reviews")
    })?;

    if duplicate_exists {
        return Err(ApiError::bad_request(
            "DuplicateReview",
            "You have already submitted a review for this contract".to_string(),
        ));
    }

    // Insert the review with 'pending' status
    // New reviews require admin approval before becoming visible
    let review = sqlx::query_as::<_, ReviewResponse>(
        r#"
        INSERT INTO reviews (contract_id, user_id, version, rating, review_text, status, helpful_count, is_flagged)
        VALUES ($1, $2, $3, $4, $5, 'pending', 0, false)
        RETURNING 
            id, 
            contract_id, 
            user_id, 
            version, 
            rating::float8 as "rating!", 
            review_text, 
            helpful_count, 
            is_flagged, 
            status, 
            created_at,
            updated_at
        "#,
    )
    .bind(contract_id)
    .bind(user_id)
    .bind(&payload.version)
    .bind(payload.rating)
    .bind(&payload.review_text)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "database error creating review");
        ApiError::internal("Failed to create review")
    })?;

    tracing::info!(
        contract_id = %contract_id,
        review_id = review.id,
        user_id = %claims.sub,
        "Review submitted successfully (pending approval)"
    );

    Ok((StatusCode::CREATED, Json(review)))
}

// ═══════════════════════════════════════════════════════════════════════════
// FETCH REVIEWS
// ═══════════════════════════════════════════════════════════════════════════
// GET /api/contracts/:id/reviews
//
// Fetches approved reviews for a contract with sorting support.
// Only reviews with status = 'approved' are returned.
//
// Sorting options:
// - most_helpful: Order by helpful_count DESC
// - most_recent: Order by created_at DESC (default)
// - highest_rated: Order by rating DESC
// - lowest_rated: Order by rating ASC
// ═══════════════════════════════════════════════════════════════════════════

pub async fn get_reviews(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(query): Query<GetReviewsQuery>,
) -> ApiResult<Json<Vec<ReviewResponse>>> {
    // Validate limit (max 100 to prevent abuse)
    let limit = query.limit.clamp(1, 100);

    // Build ORDER BY clause based on sort_by parameter
    let order_clause = match query.sort_by {
        ReviewSortBy::MostHelpful => "ORDER BY r.helpful_count DESC, r.created_at DESC",
        ReviewSortBy::MostRecent => "ORDER BY r.created_at DESC",
        ReviewSortBy::HighestRated => "ORDER BY r.rating DESC, r.created_at DESC",
        ReviewSortBy::LowestRated => "ORDER BY r.rating ASC, r.created_at DESC",
    };

    // Fetch only approved reviews (moderation workflow)
    let query_str = format!(
        r#"
        SELECT 
            r.id, 
            r.contract_id, 
            r.user_id, 
            r.version, 
            r.rating::float8 as "rating!", 
            r.review_text, 
            r.helpful_count, 
            r.is_flagged, 
            r.status, 
            r.created_at,
            r.updated_at
        FROM reviews r
        WHERE r.contract_id = $1 
          AND r.status = 'approved'
        {}
        LIMIT $2 OFFSET $3
        "#,
        order_clause
    );

    let reviews = sqlx::query_as::<_, ReviewResponse>(&query_str)
        .bind(contract_id)
        .bind(limit)
        .bind(query.offset)
        .fetch_all(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, contract_id = %contract_id, "database error fetching reviews");
            ApiError::internal("Failed to fetch reviews")
        })?;

    Ok(Json(reviews))
}

// ═══════════════════════════════════════════════════════════════════════════
// VOTE ON REVIEW HELPFULNESS
// ═══════════════════════════════════════════════════════════════════════════
// POST /api/contracts/:id/reviews/:review_id/vote
//
// Allows users to vote on whether a review was helpful.
// Prevents duplicate votes from the same user.
// Updates the helpful_count on the review.
// ═══════════════════════════════════════════════════════════════════════════

pub async fn vote_review(
    State(state): State<AppState>,
    Path((contract_id, review_id)): Path<(Uuid, i32)>,
    claims: AuthClaims,
    ValidatedJson(payload): ValidatedJson<ReviewVoteRequest>,
) -> ApiResult<Json<ReviewVoteResponse>> {
    // Verify the review exists and belongs to the contract
    let review_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM reviews WHERE id = $1 AND contract_id = $2)",
    )
    .bind(review_id)
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "database error checking review existence");
        ApiError::internal("Failed to verify review existence")
    })?;

    if !review_exists {
        return Err(ApiError::not_found(
            "ReviewNotFound",
            "Review not found or does not belong to this contract".to_string(),
        ));
    }

    // Get user's publisher ID
    let user_id = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT id FROM publishers WHERE stellar_address = $1",
    )
    .bind(&claims.sub)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "database error fetching publisher");
        ApiError::internal("Failed to fetch publisher information")
    })?;

    // Use user_id or create a placeholder for anonymous voting
    // For now, we require authentication, so user_id should exist
    if user_id.is_none() {
        return Err(ApiError::bad_request(
            "UserNotFound",
            "User must have a publisher account to vote".to_string(),
        ));
    }

    let user_id = user_id.unwrap();

    // Start a transaction to ensure and update vote
    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!(error = ?e, "failed to start transaction");
        ApiError::internal("Failed to start database transaction")
    })?;

    // Check if user already voted on this review
    let existing_vote = sqlx::query_scalar::<_, Option<bool>>(
        "SELECT vote FROM review_votes WHERE review_id = $1 AND user_id = $2",
    )
    .bind(review_id)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "database error checking existing vote");
        ApiError::internal("Failed to check existing vote")
    })?;

    let vote_recorded = if let Some(prev_vote) = existing_vote {
        // Update existing vote if it's different
        if prev_vote != payload.helpful {
            sqlx::query("UPDATE review_votes SET vote = $1 WHERE review_id = $2 AND user_id = $3")
                .bind(payload.helpful)
                .bind(review_id)
                .bind(user_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, "database error updating vote");
                    ApiError::internal("Failed to update vote")
                })?;

            // Adjust helpful_count: +1 if changing to helpful, -1 if changing to unhelpful
            let delta = if payload.helpful { 1 } else { -1 };
            sqlx::query("UPDATE reviews SET helpful_count = helpful_count + $1 WHERE id = $2")
                .bind(delta)
                .bind(review_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, "database error updating helpful count");
                    ApiError::internal("Failed to update helpful count")
                })?;

            true
        } else {
            // Vote is the same, no change needed
            false
        }
    } else {
        // Insert new vote
        sqlx::query("INSERT INTO review_votes (review_id, user_id, vote) VALUES ($1, $2, $3)")
            .bind(review_id)
            .bind(user_id)
            .bind(payload.helpful)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "database error inserting vote");
                ApiError::internal("Failed to insert vote")
            })?;

        // Update helpful_count: +1 for helpful, 0 for unhelpful (unhelpful doesn't decrease)
        if payload.helpful {
            sqlx::query("UPDATE reviews SET helpful_count = helpful_count + 1 WHERE id = $1")
                .bind(review_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, "database error updating helpful count");
                    ApiError::internal("Failed to update helpful count")
                })?;
        }

        true
    };

    tx.commit().await.map_err(|e| {
        tracing::error!(error = ?e, "failed to commit transaction");
        ApiError::internal("Failed to commit vote transaction")
    })?;

    // Fetch updated helpful_count
    let helpful_count =
        sqlx::query_scalar::<_, i32>("SELECT helpful_count FROM reviews WHERE id = $1")
            .bind(review_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "database error fetching helpful count");
                ApiError::internal("Failed to fetch updated helpful count")
            })?;

    Ok(Json(ReviewVoteResponse {
        review_id,
        helpful_count,
        vote_recorded,
    }))
}

// ═══════════════════════════════════════════════════════════════════════════
// FLAG REVIEW FOR MODERATION
// ═══════════════════════════════════════════════════════════════════════════
// POST /api/contracts/:id/reviews/:review_id/flag
//
// Allows users to flag a review for moderation.
// Flagged reviews are reviewed by admins.
// Users cannot flag the same review multiple times.
// ═══════════════════════════════════════════════════════════════════════════

pub async fn flag_review(
    State(state): State<AppState>,
    Path((contract_id, review_id)): Path<(Uuid, i32)>,
    claims: AuthClaims,
    ValidatedJson(payload): ValidatedJson<FlagReviewRequest>,
) -> ApiResult<impl IntoResponse> {
    // Verify the review exists and belongs to the contract
    let review_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM reviews WHERE id = $1 AND contract_id = $2)",
    )
    .bind(review_id)
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "database error checking review existence");
        ApiError::internal("Failed to verify review existence")
    })?;

    if !review_exists {
        return Err(ApiError::not_found(
            "ReviewNotFound",
            "Review not found or does not belong to this contract".to_string(),
        ));
    }

    // Get user's publisher ID
    let user_id = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT id FROM publishers WHERE stellar_address = $1",
    )
    .bind(&claims.sub)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "database error fetching publisher");
        ApiError::internal("Failed to fetch publisher information")
    })?;

    if user_id.is_none() {
        return Err(ApiError::bad_request(
            "UserNotFound",
            "User must have a publisher account to flag reviews".to_string(),
        ));
    }

    let user_id = user_id.unwrap();

    // Check if user already flagged this review
    let already_flagged = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM review_flags WHERE review_id = $1 AND user_id = $2 AND resolved = false)",
    )
    .bind(review_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "database error checking existing flag");
        ApiError::internal("Failed to check existing flag")
    })?;

    if already_flagged {
        return Err(ApiError::bad_request(
            "AlreadyFlagged",
            "You have already flagged this review".to_string(),
        ));
    }

    // Insert the flag
    sqlx::query(
        "INSERT INTO review_flags (review_id, user_id, reason, resolved) VALUES ($1, $2, $3, false)",
    )
    .bind(review_id)
    .bind(user_id)
    .bind(&payload.reason)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "database error inserting flag");
        ApiError::internal("Failed to flag review")
    })?;

    // Mark review as flagged
    sqlx::query("UPDATE reviews SET is_flagged = true WHERE id = $1")
        .bind(review_id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "database error updating review flag status");
            ApiError::internal("Failed to update review flag status")
        })?;

    tracing::info!(
        review_id = review_id,
        user_id = %claims.sub,
        reason = %payload.reason,
        "Review flagged for moderation"
    );

    Ok(StatusCode::NO_CONTENT)
}

// ═══════════════════════════════════════════════════════════════════════════
// MODERATE REVIEW (ADMIN ONLY)
// ═══════════════════════════════════════════════════════════════════════════
// POST /api/contracts/:id/reviews/:review_id/moderate
//
// Admin endpoint to approve or reject pending reviews.
// Only approved reviews are visible to users.
// Action must be "approve" or "reject".
// ═══════════════════════════════════════════════════════════════════════════

pub async fn moderate_review(
    State(state): State<AppState>,
    Path((contract_id, review_id)): Path<(Uuid, i32)>,
    claims: AuthClaims,
    ValidatedJson(payload): ValidatedJson<ModerateReviewRequest>,
) -> ApiResult<Json<ReviewResponse>> {
    // Verify admin status
    if !claims.admin && claims.role.as_deref() != Some("admin") {
        return Err(ApiError::forbidden(
            "Only administrators can moderate reviews",
        ));
    }

    // Validate action
    let new_status = match payload.action.to_lowercase().as_str() {
        "approve" => ReviewStatus::Approved,
        "reject" => ReviewStatus::Rejected,
        _ => {
            return Err(ApiError::bad_request(
                "InvalidAction",
                "Action must be 'approve' or 'reject'".to_string(),
            ))
        }
    };

    // Verify the review exists and belongs to the contract
    let review_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM reviews WHERE id = $1 AND contract_id = $2)",
    )
    .bind(review_id)
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "database error checking review existence");
        ApiError::internal("Failed to verify review existence")
    })?;

    if !review_exists {
        return Err(ApiError::not_found(
            "ReviewNotFound",
            "Review not found or does not belong to this contract".to_string(),
        ));
    }

    // Update review status
    let review = sqlx::query_as::<_, ReviewResponse>(
        r#"
        UPDATE reviews 
        SET status = $1
        WHERE id = $2
        RETURNING 
            id, 
            contract_id, 
            user_id, 
            version, 
            rating::float8 as "rating!", 
            review_text, 
            helpful_count, 
            is_flagged, 
            status, 
            created_at,
            updated_at
        "#,
    )
    .bind(&new_status)
    .bind(review_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, "database error updating review status");
        ApiError::internal("Failed to update review status")
    })?;

    // If approved, resolve any flags
    if new_status == ReviewStatus::Approved {
        sqlx::query("UPDATE review_flags SET resolved = true WHERE review_id = $1")
            .bind(review_id)
            .execute(&state.db)
            .await
            .ok(); // Don't fail if this fails
    }

    tracing::info!(
        review_id = review_id,
        action = %payload.action,
        admin = %claims.sub,
        "Review moderated successfully"
    );

    Ok(Json(review))
}

// ═══════════════════════════════════════════════════════════════════════════
// GET RATING AGGREGATION
// ═══════════════════════════════════════════════════════════════════════════
// GET /api/contracts/:id/rating-stats
//
// Returns aggregated rating statistics for a contract:
// - Average rating (computed from approved reviews only)
// - Total number of reviews
// - Distribution of ratings (1-5 stars)
//
// Performance:
// - Computed dynamically on each request
// - Can be cached at application layer for high-traffic contracts
// - Uses database indexes on (contract_id, status) for efficient filtering
// ═══════════════════════════════════════════════════════════════════════════

pub async fn get_rating_stats(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> ApiResult<Json<ContractRatingStats>> {
    // Fetch aggregated stats in a single query
    let stats = sqlx::query(
        r#"
        SELECT 
            AVG(rating)::float8 as avg_rating,
            COUNT(*) as total_reviews,
            COUNT(*) FILTER (WHERE rating >= 1.0 AND rating < 2.0) as stars_1,
            COUNT(*) FILTER (WHERE rating >= 2.0 AND rating < 3.0) as stars_2,
            COUNT(*) FILTER (WHERE rating >= 3.0 AND rating < 4.0) as stars_3,
            COUNT(*) FILTER (WHERE rating >= 4.0 AND rating < 5.0) as stars_4,
            COUNT(*) FILTER (WHERE rating >= 5.0) as stars_5
        FROM reviews
        WHERE contract_id = $1 AND status = 'approved'
        "#,
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = ?e, contract_id = %contract_id, "database error fetching rating stats");
        ApiError::internal("Failed to fetch rating statistics")
    })?;

    let avg_rating: Option<f64> = stats.try_get("avg_rating").ok().flatten();
    let total_reviews: Option<i64> = stats.try_get("total_reviews").ok().flatten();
    let stars_1: Option<i64> = stats.try_get("stars_1").ok().flatten();
    let stars_2: Option<i64> = stats.try_get("stars_2").ok().flatten();
    let stars_3: Option<i64> = stats.try_get("stars_3").ok().flatten();
    let stars_4: Option<i64> = stats.try_get("stars_4").ok().flatten();
    let stars_5: Option<i64> = stats.try_get("stars_5").ok().flatten();

    let rating_distribution = if total_reviews.unwrap_or(0) > 0 {
        Some(RatingDistribution {
            stars_1: stars_1.unwrap_or(0),
            stars_2: stars_2.unwrap_or(0),
            stars_3: stars_3.unwrap_or(0),
            stars_4: stars_4.unwrap_or(0),
            stars_5: stars_5.unwrap_or(0),
        })
    } else {
        None
    };

    Ok(Json(ContractRatingStats {
        average_rating: avg_rating.unwrap_or(0.0),
        total_reviews: total_reviews.unwrap_or(0),
        rating_distribution,
    }))
}

// ═══════════════════════════════════════════════════════════════════════════
// HELPER: Get pending reviews count (for admin dashboard)
// ═══════════════════════════════════════════════════════════════════════════

#[allow(dead_code)]
pub async fn get_pending_reviews_count(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM reviews WHERE status = 'pending'")
            .fetch_one(&state.db)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "database error fetching pending reviews count");
                ApiError::internal("Failed to fetch pending reviews count")
            })?;

    Ok(Json(serde_json::json!({ "pending_reviews": count })))
}
