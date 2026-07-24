//! GET /api/v1/contracts/deprecated — list deprecated contracts (issue #872)

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

const DEPRECATED_CACHE_NS: &str = "deprecated_contracts";
const DEPRECATED_CACHE_TTL_SECS: u64 = 3_600; // 1 hour

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct DeprecatedQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}
fn default_page_size() -> i64 {
    20
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeprecatedContractItem {
    pub contract_id: String,
    pub name: String,
    pub reason: Option<String>,
    pub replacement_contract_id: Option<String>,
    pub deprecated_at: Option<DateTime<Utc>>,
    pub removal_date: Option<DateTime<Utc>>,
    pub migration_guide: Option<String>,
    pub dependent_count: i64,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeprecatedContractsResponse {
    pub items: Vec<DeprecatedContractItem>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
    pub cached: bool,
    pub generated_at: DateTime<Utc>,
}

// ── Handler ───────────────────────────────────────────────────────────────────

#[utoipa::path(
    get,
    path = "/api/v1/contracts/deprecated",
    params(DeprecatedQuery),
    responses(
        (status = 200, description = "Paginated list of deprecated contracts", body = DeprecatedContractsResponse),
    ),
    tag = "Contracts"
)]
pub async fn list_deprecated_contracts(
    State(state): State<AppState>,
    Query(params): Query<DeprecatedQuery>,
) -> ApiResult<impl IntoResponse> {
    let page = params.page.max(1);
    let page_size = params.page_size.clamp(1, 100);
    let offset = (page - 1) * page_size;

    let cache_key = format!("p{}:s{}", page, page_size);
    let (cached_val, cache_hit) = state.cache.get(DEPRECATED_CACHE_NS, &cache_key).await;

    if let (Some(json_str), true) = (cached_val, cache_hit) {
        if let Ok(mut resp) = serde_json::from_str::<DeprecatedContractsResponse>(&json_str) {
            resp.cached = true;
            return Ok(Json(resp));
        }
    }

    // Total count
    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM contracts c
        INNER JOIN contract_deprecations cd ON cd.contract_id = c.id
        "#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("db error: {e}")))?;

    // Fetch items
    let items: Vec<DeprecatedContractItem> = sqlx::query_as(
        r#"
        SELECT
            c.contract_id,
            c.name,
            cd.notes          AS reason,
            (
                SELECT rc.contract_id
                FROM contracts rc
                WHERE rc.id = cd.replacement_contract_id
            )                 AS replacement_contract_id,
            cd.deprecated_at,
            cd.retirement_at  AS removal_date,
            cd.migration_guide_url AS migration_guide,
            COALESCE(
                (SELECT COUNT(*)
                 FROM contract_deprecation_notifications dn
                 WHERE dn.deprecated_contract_id = c.id),
                0
            )                 AS dependent_count
        FROM contracts c
        INNER JOIN contract_deprecations cd ON cd.contract_id = c.id
        ORDER BY cd.deprecated_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("db error fetching deprecated: {e}")))?;

    let response = DeprecatedContractsResponse {
        items,
        page,
        page_size,
        total,
        cached: false,
        generated_at: Utc::now(),
    };

    if let Ok(serialized) = serde_json::to_string(&response) {
        state
            .cache
            .put(
                DEPRECATED_CACHE_NS,
                &cache_key,
                serialized,
                Some(Duration::from_secs(DEPRECATED_CACHE_TTL_SECS)),
            )
            .await;
    }

    Ok(Json(response))
}
