// PostgreSQL Full-Text Search Service
// Uses the tsvector/tsquery infrastructure from database migrations

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::models::Network;
use sqlx::PgPool;

use crate::error::ApiError;
use crate::state::AppState;

/// Full-text search queries slower than this are counted as slow for alerting.
const SEARCH_SLOW_QUERY_MS: u64 = 500;
use axum::{
    extract::{Query, State},
    Json,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub categories: Option<Vec<String>>,
    pub networks: Option<Vec<Network>>,
    pub verified_only: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub contracts: Vec<ContractSearchResult>,
    pub total: i64,
    pub took_ms: u64,
    pub facets: Option<SearchFacets>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSearchResult {
    pub id: uuid::Uuid,
    pub contract_id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub network: Network,
    pub is_verified: bool,
    pub relevance_score: f64,
    pub matched_terms: Option<Vec<String>>,
    pub highlighted: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacets {
    pub categories: Vec<FacetCount>,
    pub networks: Vec<FacetCount>,
    pub tags: Vec<FacetCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetCount {
    pub value: String,
    pub count: i64,
}

pub struct PostgresSearchService {
    db: PgPool,
}

impl PostgresSearchService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Search contracts using PostgreSQL full-text search
    /// Uses the contracts_build_tsquery function for query sanitization
    pub async fn search(&self, query: SearchQuery) -> Result<SearchResult> {
        let start_time = std::time::Instant::now();

        // Build the SQL query with tsquery
        let mut sql = String::from(
            r#"
            SELECT 
                c.id,
                c.contract_id,
                c.name,
                c.description,
                c.category,
                c.network,
                c.is_verified,
                c.created_at,
                c.updated_at,
                c.slug,
                c.verification_status,
                c.current_version,
                -- ts_rank returns `real` (FLOAT4); the row struct decodes an f64,
                -- so cast explicitly or decoding fails once rows are returned.
                ts_rank(
                    setweight(c.name_search, 'A') || setweight(c.description_search, 'B'),
                    contracts_build_tsquery($1)
                )::float8 as relevance_score
            FROM contracts c
            WHERE contracts_build_tsquery($1) IS NOT NULL
        "#,
        );

        // Add filters
        let mut param_index = 2;
        let mut args: Vec<String> = Vec::new();
        args.push(query.query.clone());

        if let Some(ref cats) = query.categories {
            if !cats.is_empty() {
                let placeholders: Vec<String> = (param_index..param_index + cats.len() as i64)
                    .map(|i| format!("${}", i))
                    .collect();
                sql.push_str(&format!(
                    " AND c.category = ANY(ARRAY[{}])",
                    placeholders.join(", ")
                ));
                for cat in cats {
                    args.push(cat.clone());
                    param_index += 1;
                }
            }
        }

        if let Some(ref nets) = query.networks {
            if !nets.is_empty() {
                let placeholders: Vec<String> = (param_index..param_index + nets.len() as i64)
                    .map(|i| format!("${}", i))
                    .collect();
                sql.push_str(&format!(
                    " AND c.network = ANY(ARRAY[{}]::network_type[])",
                    placeholders.join(", ")
                ));
                for net in nets {
                    args.push(net.to_string());
                    param_index += 1;
                }
            }
        }

        if query.verified_only.unwrap_or(false) {
            sql.push_str(" AND c.is_verified = true");
        }

        if let Some(ref tags) = query.tags {
            if !tags.is_empty() {
                sql.push_str(" AND EXISTS (SELECT 1 FROM contract_tags ct JOIN tags t ON ct.tag_id = t.id WHERE ct.contract_id = c.id AND t.name = ANY(string_to_array($");
                sql.push_str(&param_index.to_string());
                sql.push_str(", ',')))");
                args.push(tags.join(","));
                param_index += 1;
            }
        }

        // Order by relevance and limit
        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        let offset = query.offset.unwrap_or(0).max(0);

        sql.push_str(" ORDER BY relevance_score DESC");
        // `args` is a Vec<String>, so every element binds as text. Postgres rejects
        // that for pagination ("argument of OFFSET must be type bigint, not type
        // text"), which made every fallback search fail. Both values are already
        // clamped integers, so inline them rather than binding them as text.
        sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        // Count total for pagination. Reuse only the WHERE conditions: splitting on
        // "ORDER BY" alone left the whole SELECT in place, producing
        // "... WHERE SELECT ..." and a syntax error. Splitting on the first WHERE
        // keeps any nested one (e.g. the tags EXISTS sub-query) intact.
        let filter_conditions = sql
            .split("ORDER BY")
            .next()
            .unwrap_or("")
            .split_once("WHERE")
            .map(|(_, conditions)| conditions.trim())
            .unwrap_or("TRUE");

        let count_sql = format!(
            "SELECT COUNT(*) FROM contracts c WHERE {}",
            filter_conditions
        );

        // Execute search query
        let mut query_builder = sqlx::query_as::<_, ContractSearchRow>(&sql);
        for arg in &args {
            query_builder = query_builder.bind(arg);
        }

        let rows = query_builder.fetch_all(&self.db).await?;

        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        // Previously skipped the trailing two args (limit/offset); those are no
        // longer bound, so the count query takes the same filter args as the search.
        for arg in &args {
            count_query = count_query.bind(arg);
        }
        let total = count_query.fetch_one(&self.db).await?;

        let took = start_time.elapsed().as_millis() as u64;

        crate::metrics::SEARCH_QUERY_DURATION
            .with_label_values(&["fulltext"])
            .observe(took as f64 / 1000.0);
        if took >= SEARCH_SLOW_QUERY_MS {
            crate::metrics::SEARCH_SLOW_QUERIES
                .with_label_values(&["fulltext"])
                .inc();
        }

        let contracts = rows
            .into_iter()
            .map(|row| ContractSearchResult {
                id: row.id,
                contract_id: row.contract_id,
                name: row.name,
                description: row.description,
                category: row.category,
                network: row.network,
                is_verified: row.is_verified,
                relevance_score: row.relevance_score,
                matched_terms: None,
                highlighted: None,
            })
            .collect();

        Ok(SearchResult {
            contracts,
            total,
            took_ms: took,
            facets: None, // Could add facets via separate query
        })
    }

    /// Get search suggestions (autocomplete)
    pub async fn suggest(&self, query: &str, limit: i32) -> Result<Vec<String>> {
        let suggestions = sqlx::query_scalar::<_, String>(
            r#"
            SELECT name
            FROM contracts
            WHERE to_tsvector('english', name) @@ to_tsquery('english', $1 || ':*')
            ORDER BY ts_rank(to_tsvector('english', name), to_tsquery('english', $1 || ':*')) DESC
            LIMIT $2
            "#,
        )
        .bind(query)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        Ok(suggestions)
    }

    /// Get trending search terms (could be implemented via analytics)
    pub async fn get_trending(&self, timeframe_hours: i32) -> Result<Vec<String>> {
        // Use existing search analytics or compute from recent interactions
        // For now, return top contracts by interactions
        let results = sqlx::query_scalar::<_, String>(
            r#"
            SELECT c.name
            FROM contracts c
            JOIN contract_interactions ci ON c.id = ci.contract_id
            WHERE ci.created_at > NOW() - ($1 || ' hours')::INTERVAL
            GROUP BY c.id, c.name
            ORDER BY COUNT(*) DESC
            LIMIT 10
            "#,
        )
        .bind(timeframe_hours)
        .fetch_all(&self.db)
        .await?;

        Ok(results)
    }
}

#[derive(sqlx::FromRow, Debug)]
struct ContractSearchRow {
    id: uuid::Uuid,
    contract_id: String,
    name: String,
    description: Option<String>,
    category: Option<String>,
    network: Network,
    is_verified: bool,
    relevance_score: f64,
}

// Handler for full-text search endpoint
pub async fn fulltext_search_handler(
    State(state): State<AppState>,
    Query(params): Query<SearchQueryParams>,
) -> Result<Json<SearchResult>, ApiError> {
    let query = params.q.as_deref().unwrap_or("");
    if query.is_empty() {
        return Err(ApiError::bad_request_with(
            "EMPTY_QUERY",
            "Search query cannot be empty",
        ));
    }

    // Build a stable fingerprint for this query to use as cache key
    let fingerprint = {
        use sha2::{Digest, Sha256};
        let canonical = serde_json::to_string(&params).unwrap_or_else(|_| query.to_string());
        let hash = Sha256::digest(canonical.as_bytes());
        hex::encode(hash)
    };

    if let Some(cached) = state.cache.get_search(&fingerprint).await {
        if let Ok(result) = serde_json::from_str::<SearchResult>(&cached) {
            return Ok(Json(result));
        }
    }

    let search_req = SearchQuery {
        query: query.to_string(),
        categories: params.category.as_ref().map(|c| vec![c.clone()]),
        networks: params.network.as_ref().map(|n| {
            n.split(',')
                .filter_map(|s| match s {
                    "mainnet" => Some(Network::Mainnet),
                    "testnet" => Some(Network::Testnet),
                    "futurenet" => Some(Network::Futurenet),
                    _ => None,
                })
                .collect::<Vec<Network>>()
        }),
        verified_only: params.verified_only,
        tags: params
            .tags
            .as_ref()
            .map(|t| t.split(',').map(|s| s.to_string()).collect()),
        limit: params.limit,
        offset: params.offset,
    };

    let result = state
        .pg_search
        .search(search_req)
        .await
        .map_err(|e| ApiError::internal_error("SEARCH_ERROR", e.to_string()))?;

    if let Ok(serialized) = serde_json::to_string(&result) {
        state.cache.put_search(&fingerprint, serialized).await;
    }

    Ok(Json(result))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQueryParams {
    pub q: Option<String>,
    pub category: Option<String>,
    pub network: Option<String>,
    pub verified_only: Option<bool>,
    pub tags: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
