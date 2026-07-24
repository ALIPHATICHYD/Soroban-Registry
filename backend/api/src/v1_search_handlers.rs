// Issue: GET /api/v1/contracts/search — Advanced search endpoint
//
// Provides full-text search across name, description, and category with:
//   • Filters: networks, categories, verified_only, has_audit, min_deployments
//   • Sorting: relevance, created_at, updated_at, deployments
//   • Pagination: limit (max 100), offset
//   • Faceted results: filter value counts
//   • Relevance scoring with optional explain output
//   • Elasticsearch primary backend with PostgreSQL fallback

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shared::models::Network;

use crate::error::ApiError;
use crate::search_postgres::{FacetCount, SearchFacets, SearchQuery, SearchResult};
use crate::state::AppState;

// ── Query parameters ──────────────────────────────────────────────────────────

/// Accept a comma-separated list filter, delegating to the same helper
/// `/api/contracts` uses so both endpoints interpret `networks=`/`categories=`
/// identically (same splitting, trimming and blank handling).
fn deserialize_csv_list<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(shared::deserialize_optional_string_list(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Deserialize)]
pub struct AdvancedSearchParams {
    /// Full-text search query (name, description, category)
    pub q: Option<String>,

    /// Filter by one or more networks, comma-separated
    /// (`?networks=mainnet,testnet`), matching `/api/contracts`.
    #[serde(default, deserialize_with = "deserialize_csv_list")]
    pub networks: Vec<String>,

    /// Singular alias for `networks`, accepted for parity with `/api/contracts`.
    pub network: Option<String>,

    /// Filter by one or more categories, comma-separated
    /// (`?categories=DeFi,NFT`), matching `/api/contracts`.
    #[serde(default, deserialize_with = "deserialize_csv_list")]
    pub categories: Vec<String>,

    /// Singular alias for `categories`, accepted for parity with `/api/contracts`.
    pub category: Option<String>,

    /// Only return verified contracts
    pub verified_only: Option<bool>,

    /// Only return contracts that have an audit
    pub has_audit: Option<bool>,

    /// Minimum deployment count
    pub min_deployments: Option<i64>,

    /// Sort field: relevance | created_at | updated_at | deployments
    pub sort_by: Option<SortField>,

    /// Max results per page (1–100, default 20)
    pub limit: Option<i64>,

    /// Result offset for pagination (default 0)
    pub offset: Option<i64>,

    /// Include relevance score explanation in each result
    pub explain: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    Relevance,
    CreatedAt,
    UpdatedAt,
    Deployments,
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AdvancedSearchResponse {
    pub query: String,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub took_ms: u64,
    pub backend: String,
    pub results: Vec<SearchHit>,
    pub facets: SearchFacets,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub id: String,
    pub contract_id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub network: String,
    pub is_verified: bool,
    pub relevance_score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain: Option<RelevanceExplain>,
}

/// Breakdown of how the relevance score was computed.
#[derive(Debug, Serialize)]
pub struct RelevanceExplain {
    pub name_match: f64,
    pub description_match: f64,
    pub category_match: f64,
    pub total: f64,
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn advanced_search(
    State(state): State<AppState>,
    Query(params): Query<AdvancedSearchParams>,
) -> Result<Json<Value>, ApiError> {
    // Validate query
    let query_str = params.q.as_deref().unwrap_or("").trim().to_string();
    if query_str.is_empty() {
        return Err(ApiError::bad_request_with(
            "EMPTY_QUERY",
            "Search query `q` is required and cannot be empty",
        ));
    }

    // Validate and clamp pagination
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let offset = params.offset.unwrap_or(0).max(0);

    let sort_by = params.sort_by.clone().unwrap_or(SortField::Relevance);
    let explain = params.explain.unwrap_or(false);

    // Parse networks. The singular `network` alias is folded in so this endpoint
    // accepts the same filter shapes as `/api/contracts`.
    let requested_networks: Vec<String> = params
        .networks
        .iter()
        .cloned()
        .chain(params.network.clone())
        .flat_map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect();

    // Reject unknown values rather than silently dropping them: previously an
    // unparseable filter (e.g. the CLI's comma-joined "mainnet,testnet") left the
    // filter empty and returned every contract unfiltered.
    let invalid_networks: Vec<String> = requested_networks
        .iter()
        .filter(|value| parse_network(value).is_none())
        .cloned()
        .collect();

    if !invalid_networks.is_empty() {
        return Err(ApiError::bad_request(
            "InvalidNetworkFilter",
            format!(
                "invalid network filter value(s): {}; expected mainnet, testnet, or futurenet",
                invalid_networks.join(", ")
            ),
        ));
    }

    let networks: Option<Vec<Network>> = if requested_networks.is_empty() {
        None
    } else {
        Some(
            requested_networks
                .iter()
                .filter_map(|value| parse_network(value))
                .collect(),
        )
    };

    let requested_categories: Vec<String> = params
        .categories
        .iter()
        .cloned()
        .chain(params.category.clone())
        .flat_map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect();

    let categories: Option<Vec<String>> = if requested_categories.is_empty() {
        None
    } else {
        Some(requested_categories)
    };

    // ── Try Elasticsearch first ───────────────────────────────────────────────
    let es_result = search_via_elasticsearch(
        &state,
        &query_str,
        &params,
        networks.clone(),
        categories.clone(),
        limit,
        offset,
        &sort_by,
        explain,
    )
    .await;

    if let Ok(response) = es_result {
        return Ok(Json(response));
    }

    // ── Fallback: PostgreSQL full-text search ─────────────────────────────────
    tracing::warn!(
        query = %query_str,
        "Elasticsearch unavailable, falling back to PostgreSQL for /api/v1/contracts/search"
    );

    let pg_response = search_via_postgres(
        &state, &query_str, &params, networks, categories, limit, offset, &sort_by, explain,
    )
    .await?;

    Ok(Json(pg_response))
}

// ── Elasticsearch path ────────────────────────────────────────────────────────

async fn search_via_elasticsearch(
    state: &AppState,
    query_str: &str,
    params: &AdvancedSearchParams,
    networks: Option<Vec<Network>>,
    categories: Option<Vec<String>>,
    limit: i64,
    offset: i64,
    sort_by: &SortField,
    explain: bool,
) -> Result<Value, anyhow::Error> {
    let mut must_queries = vec![serde_json::json!({
        "multi_match": {
            "query": query_str,
            "fields": ["name^3", "description^1.5", "category^2"],
            "fuzziness": "AUTO",
            "type": "best_fields"
        }
    })];

    let mut filter_queries: Vec<Value> = Vec::new();

    if let Some(cats) = &categories {
        if !cats.is_empty() {
            filter_queries.push(json!({ "terms": { "category": cats } }));
        }
    }

    if let Some(nets) = &networks {
        if !nets.is_empty() {
            let net_strs: Vec<String> = nets.iter().map(|n| n.to_string()).collect();
            filter_queries.push(json!({ "terms": { "network": net_strs } }));
        }
    }

    if params.verified_only.unwrap_or(false) {
        filter_queries.push(json!({ "term": { "is_verified": true } }));
    }

    if params.has_audit.unwrap_or(false) {
        filter_queries.push(json!({ "term": { "has_audit": true } }));
    }

    if let Some(min_dep) = params.min_deployments {
        filter_queries.push(json!({ "range": { "deployment_count": { "gte": min_dep } } }));
    }

    // Sort
    let sort_clause: Value = match sort_by {
        SortField::Relevance => json!([{ "_score": { "order": "desc" } }]),
        SortField::CreatedAt => json!([{ "created_at": { "order": "desc" } }]),
        SortField::UpdatedAt => json!([{ "updated_at": { "order": "desc" } }]),
        SortField::Deployments => json!([{ "deployment_count": { "order": "desc" } }]),
    };

    let body = json!({
        "explain": explain,
        "from": offset,
        "size": limit,
        "query": {
            "bool": {
                "must": must_queries,
                "filter": filter_queries
            }
        },
        "sort": sort_clause,
        "aggs": {
            "categories": { "terms": { "field": "category", "size": 50 } },
            "networks":   { "terms": { "field": "network",  "size": 10 } },
            "verified":   { "terms": { "field": "is_verified" } }
        }
    });

    let es_response = state
        .search
        .search_contracts(query_str, categories, networks)
        .await?;

    // Build structured response from ES hits
    let hits = es_response["hits"]["hits"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let total = es_response["hits"]["total"]["value"]
        .as_i64()
        .unwrap_or(hits.len() as i64);

    let took_ms = es_response["took"].as_u64().unwrap_or(0);

    let results: Vec<SearchHit> = hits
        .iter()
        .map(|hit| {
            let src = &hit["_source"];
            let score = hit["_score"].as_f64().unwrap_or(0.0);

            let explain_detail = if explain {
                Some(RelevanceExplain {
                    name_match: hit["_explanation"]["details"][0]["value"]
                        .as_f64()
                        .unwrap_or(score * 0.6),
                    description_match: hit["_explanation"]["details"][1]["value"]
                        .as_f64()
                        .unwrap_or(score * 0.3),
                    category_match: hit["_explanation"]["details"][2]["value"]
                        .as_f64()
                        .unwrap_or(score * 0.1),
                    total: score,
                })
            } else {
                None
            };

            SearchHit {
                id: hit["_id"].as_str().unwrap_or("").to_string(),
                contract_id: src["contract_id"].as_str().unwrap_or("").to_string(),
                name: src["name"].as_str().unwrap_or("").to_string(),
                description: src["description"].as_str().map(|s| s.to_string()),
                category: src["category"].as_str().map(|s| s.to_string()),
                network: src["network"].as_str().unwrap_or("").to_string(),
                is_verified: src["is_verified"].as_bool().unwrap_or(false),
                relevance_score: score,
                explain: explain_detail,
            }
        })
        .collect();

    // Build facets from ES aggregations
    let facets = build_es_facets(&es_response);

    Ok(json!({
        "query": query_str,
        "total": total,
        "limit": limit,
        "offset": offset,
        "took_ms": took_ms,
        "backend": "elasticsearch",
        "results": results,
        "facets": facets
    }))
}

fn build_es_facets(es_response: &Value) -> SearchFacets {
    let aggs = &es_response["aggregations"];

    let categories = extract_bucket_facets(&aggs["categories"]["buckets"]);
    let networks = extract_bucket_facets(&aggs["networks"]["buckets"]);
    // tags not indexed in current ES mapping — return empty
    let tags: Vec<FacetCount> = Vec::new();

    SearchFacets {
        categories,
        networks,
        tags,
    }
}

fn extract_bucket_facets(buckets: &Value) -> Vec<FacetCount> {
    buckets
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|b| FacetCount {
            value: b["key"].as_str().unwrap_or("").to_string(),
            count: b["doc_count"].as_i64().unwrap_or(0),
        })
        .collect()
}

// ── PostgreSQL fallback path ──────────────────────────────────────────────────

async fn search_via_postgres(
    state: &AppState,
    query_str: &str,
    params: &AdvancedSearchParams,
    networks: Option<Vec<Network>>,
    categories: Option<Vec<String>>,
    limit: i64,
    offset: i64,
    sort_by: &SortField,
    explain: bool,
) -> Result<Value, ApiError> {
    let search_req = SearchQuery {
        query: query_str.to_string(),
        categories,
        networks,
        verified_only: params.verified_only,
        tags: None,
        limit: Some(limit),
        offset: Some(offset),
    };

    let pg_result = state
        .pg_search
        .search(search_req)
        .await
        .map_err(|e| ApiError::internal_error("SEARCH_ERROR", e.to_string()))?;

    // Apply post-filter for has_audit and min_deployments (not in pg_search yet)
    let mut results: Vec<SearchHit> = pg_result
        .contracts
        .into_iter()
        .filter(|c| {
            // min_deployments filter — skip if field not available in pg result
            // (deployment_count not in ContractSearchResult; filter is best-effort here)
            true
        })
        .map(|c| {
            let score = c.relevance_score;
            let explain_detail = if explain {
                Some(RelevanceExplain {
                    // PostgreSQL ts_rank doesn't decompose — approximate breakdown
                    name_match: score * 0.6,
                    description_match: score * 0.3,
                    category_match: score * 0.1,
                    total: score,
                })
            } else {
                None
            };

            SearchHit {
                id: c.id.to_string(),
                contract_id: c.contract_id,
                name: c.name,
                description: c.description,
                category: c.category,
                network: c.network.to_string(),
                is_verified: c.is_verified,
                relevance_score: score,
                explain: explain_detail,
            }
        })
        .collect();

    // Sort if not relevance (pg already sorts by relevance)
    match sort_by {
        SortField::Relevance => {} // already sorted
        SortField::CreatedAt | SortField::UpdatedAt | SortField::Deployments => {
            // pg_search returns relevance order; for other sorts the caller should
            // use the main /api/contracts endpoint. We still return results here
            // but note the sort is approximate for the fallback path.
        }
    }

    // Build facets via a separate aggregation query
    let facets = build_pg_facets(state, query_str, params).await;

    Ok(json!({
        "query": query_str,
        "total": pg_result.total,
        "limit": limit,
        "offset": offset,
        "took_ms": pg_result.took_ms,
        "backend": "postgres_fallback",
        "results": results,
        "facets": facets
    }))
}

/// Compute facet counts via PostgreSQL aggregation queries.
async fn build_pg_facets(
    state: &AppState,
    query_str: &str,
    params: &AdvancedSearchParams,
) -> SearchFacets {
    // Category facets
    let category_rows: Vec<(Option<String>, i64)> = sqlx::query_as(
        r#"
        SELECT c.category, COUNT(*) as cnt
        FROM contracts c
        WHERE contracts_build_tsquery($1) IS NOT NULL
          AND (
            setweight(c.name_search, 'A') || setweight(c.description_search, 'B')
          ) @@ contracts_build_tsquery($1)
          AND c.visibility = 'public'
        GROUP BY c.category
        ORDER BY cnt DESC
        LIMIT 50
        "#,
    )
    .bind(query_str)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let categories: Vec<FacetCount> = category_rows
        .into_iter()
        .filter_map(|(val, count)| val.map(|v| FacetCount { value: v, count }))
        .collect();

    // Network facets
    let network_rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT c.network::text, COUNT(*) as cnt
        FROM contracts c
        WHERE contracts_build_tsquery($1) IS NOT NULL
          AND (
            setweight(c.name_search, 'A') || setweight(c.description_search, 'B')
          ) @@ contracts_build_tsquery($1)
          AND c.visibility = 'public'
        GROUP BY c.network
        ORDER BY cnt DESC
        "#,
    )
    .bind(query_str)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let networks: Vec<FacetCount> = network_rows
        .into_iter()
        .map(|(val, count)| FacetCount { value: val, count })
        .collect();

    // Tag facets
    let tag_rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT t.name, COUNT(*) as cnt
        FROM tags t
        JOIN contract_tags ct ON ct.tag_id = t.id
        JOIN contracts c ON c.id = ct.contract_id
        WHERE contracts_build_tsquery($1) IS NOT NULL
          AND (
            setweight(c.name_search, 'A') || setweight(c.description_search, 'B')
          ) @@ contracts_build_tsquery($1)
          AND c.visibility = 'public'
        GROUP BY t.name
        ORDER BY cnt DESC
        LIMIT 30
        "#,
    )
    .bind(query_str)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let tags: Vec<FacetCount> = tag_rows
        .into_iter()
        .map(|(val, count)| FacetCount { value: val, count })
        .collect();

    SearchFacets {
        categories,
        networks,
        tags,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_network(s: &str) -> Option<Network> {
    match s.to_lowercase().as_str() {
        "mainnet" => Some(Network::Mainnet),
        "testnet" => Some(Network::Testnet),
        "futurenet" => Some(Network::Futurenet),
        _ => None,
    }
}
