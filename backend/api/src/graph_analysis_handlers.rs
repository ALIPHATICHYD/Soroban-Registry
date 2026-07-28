// Graph analysis API handlers.
//
// Endpoints:
//   GET  /api/contracts/graph/analysis          – full report (clusters + criticality + cycles)
//   GET  /api/contracts/graph/clusters          – sub-network / community list
//   GET  /api/contracts/graph/critical          – critical contract ranking
//   GET  /api/contracts/:id/vulnerability-propagation  – propagation from a specific contract

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use sqlx;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    dependency,
    error::{ApiError, ApiResult},
    graph_analysis,
    state::AppState,
};
use shared::{
    CriticalContractScore, GraphAnalysisReport, IssueSeverity, VulnerabilityPropagationResult,
};

// ─── Query params ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct GraphAnalysisQuery {
    /// Filter by network (optional).
    pub network: Option<shared::Network>,
    /// If true, skip cache and recompute.
    pub force: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CriticalContractsQuery {
    pub network: Option<shared::Network>,
    /// Maximum number of results (default 20, max 100).
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct PropagationQuery {
    /// Treat the contract as having this severity vulnerability (default: high).
    pub severity: Option<IssueSeverity>,
    /// Maximum propagation depth (default 5, max 8).
    pub max_depth: Option<usize>,
}

// ─── Cache helpers ────────────────────────────────────────────────────────────

const ANALYSIS_CACHE_TTL_SECS: u64 = 600; // 10 minutes

fn analysis_cache_key(network: Option<&shared::Network>) -> String {
    format!(
        "graph_analysis:{}",
        network
            .map(|n| n.to_string())
            .unwrap_or_else(|| "all".to_string())
    )
}

/// Fetch the base graph from cache or build it fresh, then run the analysis algorithms.
async fn build_report(
    state: &AppState,
    network: Option<shared::Network>,
    force: bool,
) -> ApiResult<GraphAnalysisReport> {
    let cache_key = analysis_cache_key(network.as_ref());

    if !force {
        if let (Some(cached), true) = state.cache.get("system", &cache_key).await {
            if let Ok(report) = serde_json::from_str::<GraphAnalysisReport>(&cached) {
                return Ok(report);
            }
        }
    }

    let graph = dependency::build_dependency_graph(&state.db, network)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to build dependency graph: {}", e)))?;

    let report = graph_analysis::run_full_analysis(&graph.nodes, &graph.edges);

    if let Ok(serialized) = serde_json::to_string(&report) {
        state
            .cache
            .put(
                "system",
                &cache_key,
                serialized,
                Some(Duration::from_secs(ANALYSIS_CACHE_TTL_SECS)),
            )
            .await;
    }

    Ok(report)
}

// ─── GET /api/contracts/graph/analysis ───────────────────────────────────────

pub async fn get_graph_analysis(
    State(state): State<AppState>,
    Query(query): Query<GraphAnalysisQuery>,
) -> ApiResult<Json<GraphAnalysisReport>> {
    let report = build_report(&state, query.network, query.force.unwrap_or(false)).await?;
    Ok(Json(report))
}

// ─── GET /api/contracts/graph/clusters ───────────────────────────────────────

pub async fn get_graph_clusters(
    State(state): State<AppState>,
    Query(query): Query<GraphAnalysisQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let report = build_report(&state, query.network, query.force.unwrap_or(false)).await?;

    Ok(Json(serde_json::json!({
        "total_clusters": report.clusters.len(),
        "total_nodes": report.total_nodes,
        "clusters": report.clusters,
    })))
}

// ─── GET /api/contracts/graph/critical ───────────────────────────────────────

pub async fn get_critical_contracts(
    State(state): State<AppState>,
    Query(query): Query<CriticalContractsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let network = query.network.clone();
    let report = build_report(&state, network.clone(), false).await?;

    let top: Vec<&CriticalContractScore> = report.critical_contracts.iter().take(limit).collect();

    Ok(Json(serde_json::json!({
        "total_ranked": report.critical_contracts.len(),
        "returned": top.len(),
        "critical_contracts": top,
        "cyclic_contracts": report.cyclic_contracts,
    })))
}

// ─── GET /api/contracts/:id/vulnerability-propagation ────────────────────────

pub async fn get_vulnerability_propagation(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(query): Query<PropagationQuery>,
) -> ApiResult<Json<VulnerabilityPropagationResult>> {
    // Verify contract exists.
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM contracts WHERE id = $1)")
        .bind(contract_id)
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

    if !exists {
        return Err(ApiError::not_found("contract", "Contract not found"));
    }

    // Derive a source severity from the query param or from open security issues.
    let severity_score = match query.severity {
        Some(IssueSeverity::Critical) => 1.0,
        Some(IssueSeverity::High) | None => 0.75,
        Some(IssueSeverity::Medium) => 0.50,
        Some(IssueSeverity::Low) => 0.25,
    };

    // If no explicit severity was supplied, check whether this contract has
    // open security issues and use the highest severity found.
    let effective_severity = if query.severity.is_none() {
        let max_sev: Option<String> = sqlx::query_scalar(
            r#"
            SELECT severity FROM security_issues
            WHERE contract_id = $1 AND status = 'open'
            ORDER BY CASE severity
                WHEN 'critical' THEN 1 WHEN 'high' THEN 2
                WHEN 'medium' THEN 3 ELSE 4
            END ASC
            LIMIT 1
            "#,
        )
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

        match max_sev.as_deref() {
            Some("critical") => 1.0,
            Some("high") => 0.75,
            Some("medium") => 0.50,
            _ => severity_score,
        }
    } else {
        severity_score
    };

    // Build the full graph (cached from prior analysis calls if available).
    let graph = dependency::build_dependency_graph(&state.db, None)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to build graph: {}", e)))?;

    let g = graph_analysis::AnalysisGraph::build(&graph.nodes, &graph.edges);

    // Find the index of the source contract.
    let source_idx = g.index.get(&contract_id).copied().ok_or_else(|| {
        ApiError::not_found(
            "contract_in_graph",
            "Contract has no dependency relationships and is not part of the interaction graph",
        )
    })?;

    let result = graph_analysis::propagate_vulnerability(&g, &[(source_idx, effective_severity)]);

    Ok(Json(result))
}

// ─── GET /api/contracts/graph/subnetwork/:cluster_id ─────────────────────────

pub async fn get_subnetwork(
    State(state): State<AppState>,
    Path(cluster_id): Path<usize>,
    Query(query): Query<GraphAnalysisQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let network = query.network.clone();
    let report = build_report(&state, network.clone(), false).await?;

    let cluster = report
        .clusters
        .iter()
        .find(|c| c.cluster_id == cluster_id)
        .ok_or_else(|| ApiError::not_found("cluster", "Sub-network not found"))?;

    // Fetch contract details for the cluster members.
    let members: Vec<Uuid> = cluster.members.clone();
    let contracts: Vec<shared::Contract> =
        sqlx::query_as("SELECT * FROM contracts WHERE id = ANY($1) ORDER BY name ASC")
            .bind(&members)
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

    // Cross-network edges within this cluster.
    let member_set: std::collections::HashSet<Uuid> = members.iter().copied().collect();

    let graph = dependency::build_dependency_graph(&state.db, network)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to build graph: {}", e)))?;

    let internal_edges: Vec<&shared::GraphEdge> = graph
        .edges
        .iter()
        .filter(|e| member_set.contains(&e.source) && member_set.contains(&e.target))
        .collect();

    Ok(Json(serde_json::json!({
        "cluster_id": cluster.cluster_id,
        "hub_contract_id": cluster.hub_contract_id,
        "member_count": cluster.members.len(),
        "cohesion": cluster.cohesion,
        "external_edges": cluster.external_edges,
        "contracts": contracts,
        "internal_edges": internal_edges,
    })))
}
