// PostgreSQL Full-Text Search Service
// Uses the tsvector/tsquery infrastructure from database migrations

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::models::Network;
use shared::pagination::Cursor;
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
    /// Opaque keyset cursor. When set, results are ordered by a stable
    /// `(created_at, id)` key (overriding relevance ordering) so that
    /// concurrent inserts/updates cannot cause skipped or duplicated rows
    /// across pages. Mutually exclusive with `offset` in practice.
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub contracts: Vec<ContractSearchResult>,
    pub total: i64,
    pub took_ms: u64,
    pub facets: Option<SearchFacets>,
    /// Opaque cursor for the next page, present only when more rows remain and
    /// the request used cursor pagination. Pass it back as `cursor` to continue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
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
    pub deprecation_status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement_contract_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub is_deprecated: bool,
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
                c.is_deprecated,
                c.deprecation_status,
                c.replacement_contract_id,
                -- ts_rank returns `real` (FLOAT4); the row struct decodes an f64,
                -- so cast explicitly or decoding fails once rows are returned.
                ts_rank(
                    setweight(c.name_search, 'A') || setweight(c.description_search, 'B'),
                    contracts_build_tsquery($1)
                )::float8 as relevance_score
            FROM contracts c
            WHERE contracts_build_tsquery($1) IS NOT NULL
              -- Actually restrict to documents that MATCH the query. Without this
              -- the WHERE only checked the query was parseable, so every contract
              -- was returned (ordered by relevance, which masked it under offset
              -- pagination) and `total` counted the whole table. The keyset order
              -- introduced for cursor pagination made the stray rows visible.
              AND (setweight(c.name_search, 'A') || setweight(c.description_search, 'B'))
                  @@ contracts_build_tsquery($1)
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

        let limit = query.limit.unwrap_or(20).clamp(1, 100);
        let offset = query.offset.unwrap_or(0).max(0);

        // Count total for the current filters. Computed BEFORE any keyset predicate
        // is appended so `total` stays stable across cursor pages (the cursor
        // narrows the page, not the reported match count). Splitting on the first
        // WHERE keeps any nested one (e.g. the tags EXISTS sub-query) intact.
        let filter_conditions = sql
            .split_once("WHERE")
            .map(|(_, conditions)| conditions.trim().to_string())
            .unwrap_or_else(|| "TRUE".to_string());
        let count_sql = format!("SELECT COUNT(*) FROM contracts c WHERE {filter_conditions}");

        // Cursor mode is requested whenever the `cursor` param is present — even an
        // empty value, which means "first page". A non-empty value is a keyset
        // anchor (already validated by the handler). In cursor mode we order by a
        // stable (created_at DESC, id DESC) key instead of relevance, so rows
        // inserted/updated between page requests can't be skipped or duplicated.
        let cursor_mode = query.cursor.is_some();
        let keyset = query
            .cursor
            .as_deref()
            .filter(|c| !c.is_empty())
            .and_then(|c| Cursor::decode(c).ok());

        if cursor_mode {
            if keyset.is_some() {
                let ts_idx = param_index;
                let id_idx = param_index + 1;
                sql.push_str(&format!(
                    " AND (c.created_at < ${ts_idx} OR (c.created_at = ${ts_idx} AND c.id < ${id_idx}))"
                ));
            }
            sql.push_str(" ORDER BY c.created_at DESC, c.id DESC");
            // Fetch one extra row to detect whether a further page exists.
            sql.push_str(&format!(" LIMIT {}", limit + 1));
        } else {
            sql.push_str(" ORDER BY relevance_score DESC");
            // `args` is a Vec<String>, so every element binds as text. Postgres
            // rejects that for pagination ("argument of OFFSET must be type bigint,
            // not type text"). Both values are clamped integers, so inline them.
            sql.push_str(&format!(" LIMIT {limit} OFFSET {offset}"));
        }

        // Execute search query
        let mut query_builder = sqlx::query_as::<_, ContractSearchRow>(&sql);
        for arg in &args {
            query_builder = query_builder.bind(arg);
        }
        if let Some(ref c) = keyset {
            // Typed binds for the keyset placeholders appended above.
            query_builder = query_builder.bind(c.timestamp).bind(c.id);
        }

        let mut rows = query_builder.fetch_all(&self.db).await?;

        // In cursor mode we fetched limit+1; the extra row signals another page.
        // Trim it and derive the next cursor from the last kept row.
        let mut next_cursor = None;
        if cursor_mode && rows.len() as i64 > limit {
            rows.truncate(limit as usize);
            if let Some(last) = rows.last() {
                next_cursor = Some(Cursor::new(last.created_at, last.id).encode());
            }
        }

        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        // The count query takes the same filter args as the search (keyset binds
        // are not part of it).
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
                deprecation_status: row.deprecation_status,
                replacement_contract_id: row.replacement_contract_id,
                is_deprecated: row.is_deprecated,
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
            next_cursor,
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
    is_deprecated: bool,
    deprecation_status: String,
    replacement_contract_id: Option<uuid::Uuid>,
    // Selected already by the query; needed to build the next-page keyset cursor.
    created_at: DateTime<Utc>,
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

    // Validate a non-empty cursor up front so a malformed one fails clearly with
    // 400 rather than being silently ignored (an empty cursor = "first page").
    if let Some(c) = params.cursor.as_deref() {
        if !c.is_empty() && Cursor::decode(c).is_err() {
            return Err(ApiError::bad_request_with(
                "INVALID_CURSOR",
                "The provided pagination cursor is invalid",
            ));
        }
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
        categories: params.category.as_ref().map(|c| {
            c.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        }),
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
        cursor: params.cursor.clone(),
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
    /// Opaque keyset cursor. Present (even empty) switches this endpoint to
    /// stable cursor pagination; pass back the `next_cursor` from the response.
    pub cursor: Option<String>,
}
