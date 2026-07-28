// Favorites Handlers — GET/PATCH /api/me/preferences (favorites field)
// Allows authenticated publishers to read and update their favorites list.

use crate::validation::extractors::ValidatedJson;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    auth,
    error::{ApiError, ApiResult},
    state::AppState,
};

/// Response for GET /api/me/preferences
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserFavoritesPreferences {
    pub favorites: Vec<String>,
}

/// Request body for PATCH /api/me/preferences
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateFavoritesRequest {
    pub favorites: Vec<String>,
}

/// Get the authenticated user's favorites list.
///
/// GET /api/me/preferences
pub async fn get_favorites(
    State(state): State<AppState>,
    auth_user: auth::AuthenticatedUser,
) -> ApiResult<Json<UserFavoritesPreferences>> {
    let publisher_id = auth_user.publisher_id;

    let row: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT favorites FROM user_preferences WHERE publisher_id = $1")
            .bind(publisher_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let favorites = match row {
        Some((val,)) => parse_favorites_json(val),
        None => vec![],
    };

    Ok(Json(UserFavoritesPreferences { favorites }))
}

/// Update the authenticated user's favorites list.
///
/// PATCH /api/me/preferences
pub async fn update_favorites(
    State(state): State<AppState>,
    auth_user: auth::AuthenticatedUser,
    ValidatedJson(req): ValidatedJson<UpdateFavoritesRequest>,
) -> ApiResult<Json<UserFavoritesPreferences>> {
    let publisher_id = auth_user.publisher_id;

    // Deduplicate and cap at 500 entries (mirrors frontend validation)
    let mut seen = std::collections::HashSet::new();
    let deduped: Vec<String> = req
        .favorites
        .into_iter()
        .filter(|id| seen.insert(id.clone()))
        .take(500)
        .collect();

    let favorites_json = serde_json::to_value(&deduped)
        .map_err(|e| ApiError::internal(format!("Serialization error: {}", e)))?;

    // Upsert: insert a new row or update the favorites column if one already exists
    sqlx::query(
        r#"
        INSERT INTO user_preferences (publisher_id, favorites, theme, language, default_network, extensible_settings)
        VALUES ($1, $2, 'dark', 'en', 'testnet', '{}')
        ON CONFLICT (publisher_id)
        DO UPDATE SET favorites = $2, updated_at = NOW()
        "#,
    )
    .bind(publisher_id)
    .bind(&favorites_json)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(UserFavoritesPreferences { favorites: deduped }))
}

fn parse_favorites_json(val: serde_json::Value) -> Vec<String> {
    match val {
        serde_json::Value::Array(arr) => arr
            .into_iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec![],
    }
}
