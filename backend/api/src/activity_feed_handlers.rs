use crate::error::ApiError;
use crate::error::ApiResult;
use crate::state::AppState;
use axum::extract::{Query, State};
use axum::Json;
use shared::models::{ActivityFeedParams, AnalyticsEvent, CursorPaginatedResponse};
use sqlx::QueryBuilder;

/// GET /api/activity-feed
/// Returns a cursor-paginated list of analytics events across the registry.
pub async fn get_activity_feed(
    State(state): State<AppState>,
    Query(params): Query<ActivityFeedParams>,
) -> ApiResult<Json<CursorPaginatedResponse<AnalyticsEvent>>> {
    let limit = params.limit.clamp(1, 100);

    // 1. Build the main query
    let mut query_builder: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        r#"
        SELECT id, event_type, contract_id, user_id, metadata, created_at
        FROM analytics_events
        WHERE 1=1
        "#,
    );

    if let Some(cursor) = params.cursor {
        query_builder.push(" AND created_at < ");
        query_builder.push_bind(cursor);
    }

    if let Some(ref event_type) = params.event_type {
        query_builder.push(" AND event_type = ");
        query_builder.push_bind(event_type);
    }

    if let Some(contract_id) = params.contract_id {
        query_builder.push(" AND contract_id = ");
        query_builder.push_bind(contract_id);
    }

    query_builder.push(" ORDER BY created_at DESC LIMIT ");
    query_builder.push_bind(limit);

    let events: Vec<AnalyticsEvent> = query_builder
        .build_query_as()
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("fetch activity feed failed: {}", e)))?;

    // 2. Count total matches for this filter
    let mut count_builder: QueryBuilder<sqlx::Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM analytics_events WHERE 1=1");

    if let Some(ref event_type) = params.event_type {
        count_builder.push(" AND event_type = ");
        count_builder.push_bind(event_type);
    }

    if let Some(contract_id) = params.contract_id {
        count_builder.push(" AND contract_id = ");
        count_builder.push_bind(contract_id);
    }

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("count activity feed failed: {}", e)))?;

    let next_cursor = events.last().map(|e| e.created_at);

    Ok(Json(CursorPaginatedResponse::new(
        events,
        total,
        limit,
        next_cursor,
    )))
}
