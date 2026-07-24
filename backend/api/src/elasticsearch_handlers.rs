// Issue #880: Full-text search integration with Elasticsearch.
//
// Provides handlers that first attempt Elasticsearch and transparently fall
// back to the PostgreSQL full-text search service when ES is unavailable.
// Search events are recorded to search_analytics for popularity tracking.

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::models::Network;

use crate::error::ApiError;
use crate::search_postgres::{SearchQuery, SearchResult};
use crate::state::AppState;

// ── Request / response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ElasticsearchSearchParams {
    pub q: Option<String>,
    pub category: Option<String>,
    pub network: Option<String>,
    pub verified_only: Option<bool>,
    pub tags: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SearchAnalyticsRecord {
    pub id: i64,
    pub query_text: String,
    pub backend: String,
    pub result_count: i32,
    pub took_ms: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopularTerm {
    pub query_text: String,
    pub search_count: i64,
    pub avg_result_count: f64,
    pub avg_took_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SearchSynonym {
    pub id: i64,
    pub term: String,
    pub synonyms: Vec<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpsertSynonymRequest {
    pub term: String,
    pub synonyms: Vec<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct TrendingParams {
    pub hours: Option<i64>,
    pub limit: Option<i64>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_networks(network_str: &str) -> Vec<Network> {
    network_str
        .split(',')
        .filter_map(|s| match s.trim() {
            "mainnet" => Some(Network::Mainnet),
            "testnet" => Some(Network::Testnet),
            "futurenet" => Some(Network::Futurenet),
            _ => None,
        })
        .collect()
}

async fn record_search_event(
    pool: &sqlx::PgPool,
    query_text: &str,
    backend: &str,
    result_count: i64,
    took_ms: u64,
) {
    let _ = sqlx::query(
        "INSERT INTO search_analytics (query_text, backend, result_count, took_ms) VALUES ($1, $2, $3, $4)",
    )
    .bind(query_text)
    .bind(backend)
    .bind(result_count as i32)
    .bind(took_ms as i32)
    .execute(pool)
    .await;
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/search/elasticsearch
/// Runs a search against Elasticsearch with facet aggregations.
/// Falls back to PostgreSQL full-text search if ES is unavailable.
pub async fn elasticsearch_search(
    State(state): State<AppState>,
    Query(params): Query<ElasticsearchSearchParams>,
) -> Result<Json<Value>, ApiError> {
    let query = params.q.as_deref().unwrap_or("").trim().to_string();
    if query.is_empty() {
        return Err(ApiError::bad_request_with(
            "EMPTY_QUERY",
            "Search query cannot be empty",
        ));
    }

    let categories = params.category.as_ref().map(|c| vec![c.clone()]);
    let networks = params
        .network
        .as_deref()
        .map(parse_networks)
        .filter(|v| !v.is_empty());

    // Try Elasticsearch first
    let es_result = state
        .search
        .search_contracts(&query, categories.clone(), networks.clone())
        .await;

    match es_result {
        Ok(es_response) => {
            // Extract result count from ES hits for analytics
            let result_count = es_response["hits"]["total"]["value"].as_i64().unwrap_or(0);

            record_search_event(&state.db, &query, "elasticsearch", result_count, 0).await;

            Ok(Json(json!({
                "backend": "elasticsearch",
                "query":   query,
                "results": es_response
            })))
        }
        Err(es_err) => {
            // ES unavailable — fall back to PostgreSQL
            tracing::warn!(
                error = %es_err,
                query = %query,
                "Elasticsearch unavailable, falling back to PostgreSQL search"
            );

            let search_req = SearchQuery {
                query: query.clone(),
                categories,
                networks,
                verified_only: params.verified_only,
                tags: params
                    .tags
                    .as_deref()
                    .map(|t| t.split(',').map(|s| s.to_string()).collect()),
                limit: params.limit,
                offset: params.offset,
            };

            let pg_result =
                state.pg_search.search(search_req).await.map_err(|e| {
                    ApiError::internal_error("SEARCH_FALLBACK_ERROR", e.to_string())
                })?;

            record_search_event(
                &state.db,
                &query,
                "postgres_fallback",
                pg_result.total,
                pg_result.took_ms,
            )
            .await;

            Ok(Json(json!({
                "backend":  "postgres_fallback",
                "query":    query,
                "results":  pg_result
            })))
        }
    }
}

/// GET /api/search/analytics
/// Returns recent search events from search_analytics.
pub async fn get_search_analytics(
    State(state): State<AppState>,
    Query(params): Query<TrendingParams>,
) -> Result<Json<Vec<SearchAnalyticsRecord>>, ApiError> {
    let hours = params.hours.unwrap_or(24).clamp(1, 168);
    let limit = params.limit.unwrap_or(100).clamp(1, 500);

    let rows = sqlx::query_as::<_, SearchAnalyticsRecord>(
        r#"
        SELECT id, query_text, backend, result_count, took_ms, created_at
        FROM search_analytics
        WHERE created_at >= NOW() - ($1 * INTERVAL '1 hour')
        ORDER BY created_at DESC
        LIMIT $2
        "#,
    )
    .bind(hours as f64)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("ANALYTICS_FETCH_ERROR", e.to_string()))?;

    Ok(Json(rows))
}

/// GET /api/search/trending
/// Returns the most-searched terms in the requested time window.
pub async fn get_trending_searches(
    State(state): State<AppState>,
    Query(params): Query<TrendingParams>,
) -> Result<Json<Vec<PopularTerm>>, ApiError> {
    let hours = params.hours.unwrap_or(24).clamp(1, 168);
    let limit = params.limit.unwrap_or(20).clamp(1, 100);

    let rows = sqlx::query(
        r#"
        SELECT
            query_text,
            COUNT(*)            AS search_count,
            AVG(result_count)   AS avg_result_count,
            AVG(took_ms)        AS avg_took_ms
        FROM search_analytics
        WHERE created_at >= NOW() - ($1 * INTERVAL '1 hour')
        GROUP BY query_text
        ORDER BY search_count DESC
        LIMIT $2
        "#,
    )
    .bind(hours as f64)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("TRENDING_FETCH_ERROR", e.to_string()))?;

    use sqlx::Row as _;
    let terms = rows
        .into_iter()
        .map(|r| PopularTerm {
            query_text: r.try_get("query_text").unwrap_or_default(),
            search_count: r.try_get("search_count").unwrap_or(0),
            avg_result_count: r.try_get("avg_result_count").unwrap_or(0.0),
            avg_took_ms: r.try_get("avg_took_ms").unwrap_or(0.0),
        })
        .collect();

    Ok(Json(terms))
}

/// POST /api/admin/search/reindex
/// Triggers a full re-index of all contracts into Elasticsearch.
pub async fn reindex_contracts(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    // Fetch all contracts and index them into ES
    let contracts = sqlx::query_as::<_, shared::models::Contract>(
        "SELECT * FROM contracts ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("REINDEX_FETCH_ERROR", e.to_string()))?;

    // Ensure the ES index exists before writing
    if let Err(e) = state.search.ensure_index().await {
        tracing::warn!(error = %e, "Failed to ensure ES index during reindex");
    }

    let total = contracts.len();
    let mut indexed = 0usize;
    let mut failed = 0usize;

    for contract in &contracts {
        match state.search.index_contract(contract, None).await {
            Ok(_) => indexed += 1,
            Err(e) => {
                failed += 1;
                tracing::warn!(
                    contract_id = %contract.contract_id,
                    error = %e,
                    "Failed to index contract into Elasticsearch"
                );
            }
        }
    }

    tracing::info!(total, indexed, failed, "Elasticsearch reindex complete");

    Ok(Json(json!({
        "total":   total,
        "indexed": indexed,
        "failed":  failed,
        "completed_at": Utc::now()
    })))
}

/// GET /api/admin/search/synonyms
/// Returns all synonym entries.
pub async fn get_synonyms(
    State(state): State<AppState>,
) -> Result<Json<Vec<SearchSynonym>>, ApiError> {
    let rows = sqlx::query_as::<_, SearchSynonym>(
        "SELECT id, term, synonyms, is_active, created_at, updated_at FROM search_synonyms ORDER BY term",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("SYNONYM_FETCH_ERROR", e.to_string()))?;

    Ok(Json(rows))
}

/// PUT /api/admin/search/synonyms
/// Creates or updates a synonym entry.
pub async fn upsert_synonym(
    State(state): State<AppState>,
    Json(req): Json<UpsertSynonymRequest>,
) -> Result<Json<SearchSynonym>, ApiError> {
    if req.term.trim().is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_TERM",
            "term cannot be empty",
        ));
    }

    let row = sqlx::query_as::<_, SearchSynonym>(
        r#"
        INSERT INTO search_synonyms (term, synonyms, is_active)
        VALUES ($1, $2, $3)
        ON CONFLICT (term) DO UPDATE
            SET synonyms   = EXCLUDED.synonyms,
                is_active  = COALESCE($3, search_synonyms.is_active),
                updated_at = NOW()
        RETURNING id, term, synonyms, is_active, created_at, updated_at
        "#,
    )
    .bind(req.term.trim().to_lowercase())
    .bind(&req.synonyms)
    .bind(req.is_active.unwrap_or(true))
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("SYNONYM_UPSERT_ERROR", e.to_string()))?;

    Ok(Json(row))
}
