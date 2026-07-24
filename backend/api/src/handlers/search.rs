use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use shared::models::Network;
use crate::{
    error::ApiResult,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub categories: Option<String>, // comma-separated
    pub networks: Option<String>,   // comma-separated
}

/// Full-text search for contracts using Elasticsearch (#730)
pub async fn search_contracts(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let categories = params.categories.map(|s| s.split(',').map(|v| v.trim().to_string()).collect());
    let networks = params.networks.map(|s| {
        s.split(',')
            .map(|v| v.trim().parse::<Network>())
            .filter_map(Result::ok)
            .collect()
    });

    let results = state.search.search_contracts(&params.q, categories, networks).await
        .map_err(|e| crate::error::ApiError::internal(format!("Search failed: {}", e)))?;

    Ok(Json(results))
}

/// Autocomplete suggestions for contract names (#730)
pub async fn autocomplete(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> ApiResult<Json<Vec<String>>> {
    let suggestions = state.search.autocomplete(&params.q).await
        .map_err(|e| crate::error::ApiError::internal(format!("Autocomplete failed: {}", e)))?;

    Ok(Json(suggestions))
}
