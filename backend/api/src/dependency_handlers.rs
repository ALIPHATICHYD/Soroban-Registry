//! Contract dependency endpoints.
//!
//! The read path was rebuilt on the bounded recursive traversal in
//! [`crate::dependency_graph`] (Issue #1147). What it replaced had three
//! defects that are worth naming, because each is easy to reintroduce:
//!
//! - It queried `contract_dependencies` under a *call* shape
//!   (`caller_id` / `callee_contract_id` / `call_volume`) that no migration ever
//!   created, so the endpoint could only ever fail.
//! - Its `LEFT JOIN contracts c ON c.contract_id = cd.callee_contract_id` had no
//!   network qualifier. Since `contracts` is `UNIQUE(contract_id, network)`, an
//!   address deployed on two networks matched **two** rows: a duplicate edge plus
//!   silent cross-network contamination.
//! - It was `#[async_recursion]` and called `visited.remove()` on unwind, so a
//!   diamond was re-expanded once per incoming path -- exponential on wide
//!   graphs, and non-deterministic about which path it reported.

use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use shared::dependency_graph::{DiagnosticKind, EdgeState};
use shared::{ContractDependency, DependencyDeclaration, DependencyNode, DependencyResponse};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::{
    contract_ref, dependency,
    dependency_graph::{self, Direction, TraversalRequest, TraversedEdgeRow},
    error::{ApiError, ApiResult},
    handlers::db_internal_error,
    state::AppState,
};

/// Query parameters shared by the dependency read endpoints.
#[derive(Debug, Default, serde::Deserialize)]
pub struct DependencyQuery {
    /// Required to disambiguate a bare contract address, which is only unique
    /// per `(contract_id, network)`.
    pub network: Option<shared::Network>,
    /// Walk the whole closure rather than only direct edges. Defaults to true so
    /// the endpoint keeps its historical recursive behaviour.
    pub transitive: Option<bool>,
    pub depth: Option<u32>,
    pub max_nodes: Option<usize>,
    /// Replay the declared graph as it stood at this instant.
    pub as_of: Option<chrono::DateTime<chrono::Utc>>,
    /// Include on-chain call edges alongside declared ones.
    pub include_telemetry: Option<bool>,
    /// 1-based page for the flat node list on `/dependency-graph`.
    pub page: Option<i64>,
    /// Nodes per page. Capped by [`MAX_PER_PAGE`].
    pub per_page: Option<i64>,
}

/// Upper bound on `per_page`.
///
/// Offset pagination is only defensible here because the traversal already
/// hard-caps its result set (`max_nodes`, itself capped at 10000). This keeps a
/// single page from simply re-requesting that whole cap.
const MAX_PER_PAGE: i64 = 200;
const DEFAULT_PER_PAGE: i64 = 50;

/// GET /api/contracts/:id/dependencies
///
/// `:id` accepts a registry UUID or a Stellar contract address. A bare address
/// registered on more than one network is a 409 with the candidates, not a
/// silent pick.
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/dependencies",
    params(
        ("id" = String, Path, description = "Contract UUID or Stellar contract address"),
        ("network" = Option<String>, Query, description = "mainnet | testnet | futurenet. Required to disambiguate an address registered on more than one network"),
        ("transitive" = Option<bool>, Query, description = "Walk the whole closure (default true) rather than direct edges only"),
        ("depth" = Option<u32>, Query, description = "Maximum traversal depth, capped server-side"),
        ("max_nodes" = Option<u32>, Query, description = "Maximum nodes returned, capped server-side"),
        ("as_of" = Option<String>, Query, description = "RFC3339 instant; replays the declared graph as it stood then"),
        ("include_telemetry" = Option<bool>, Query, description = "Include on-chain call edges alongside declared ones")
    ),
    responses(
        (status = 200, description = "Dependency tree", body = DependencyResponse),
        (status = 400, description = "The id is neither a UUID nor a contract address"),
        (status = 404, description = "Contract not found"),
        (status = 409, description = "Ambiguous contract address; retry with ?network="),
        (status = 503, description = "Traversal exceeded its time budget")
    ),
    tag = "Graphs"
)]
pub async fn get_contract_dependencies(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DependencyQuery>,
) -> ApiResult<Json<DependencyResponse>> {
    let contract = contract_ref::resolve(&state, &id, query.network).await?;
    let response = dependency_tree(&state, contract, &query, Direction::Dependencies).await?;
    Ok(Json(response))
}

/// GET /api/contracts/:id/dependents
///
/// The reverse edges: what would be affected if this contract changed.
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/dependents",
    params(
        ("id" = String, Path, description = "Contract UUID or Stellar contract address"),
        ("network" = Option<String>, Query, description = "mainnet | testnet | futurenet. Required to disambiguate an address registered on more than one network"),
        ("transitive" = Option<bool>, Query, description = "Walk the whole closure (default true) rather than direct edges only"),
        ("depth" = Option<u32>, Query, description = "Maximum traversal depth, capped server-side"),
        ("max_nodes" = Option<u32>, Query, description = "Maximum nodes returned, capped server-side"),
        ("as_of" = Option<String>, Query, description = "RFC3339 instant; replays the declared graph as it stood then"),
        ("include_telemetry" = Option<bool>, Query, description = "Include on-chain call edges alongside declared ones")
    ),
    responses(
        (status = 200, description = "Dependent tree", body = DependencyResponse),
        (status = 400, description = "The id is neither a UUID nor a contract address"),
        (status = 404, description = "Contract not found"),
        (status = 409, description = "Ambiguous contract address; retry with ?network="),
        (status = 503, description = "Traversal exceeded its time budget")
    ),
    tag = "Graphs"
)]
pub async fn get_contract_dependents(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DependencyQuery>,
) -> ApiResult<Json<DependencyResponse>> {
    let contract = contract_ref::resolve(&state, &id, query.network).await?;
    let response = dependency_tree(&state, contract, &query, Direction::Dependents).await?;
    Ok(Json(response))
}

/// GraphQL entry point (`Contract.dependencies`).
///
/// Takes a UUID because GraphQL has already resolved the contract; the network
/// is read from the row rather than guessed.
pub(crate) async fn get_contract_dependencies_internal(
    state: &AppState,
    id: Uuid,
) -> ApiResult<DependencyResponse> {
    let network: shared::Network =
        sqlx::query_scalar("SELECT network FROM contracts WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|err| db_internal_error("get root contract for dependencies", err))?
            .ok_or_else(|| ApiError::not_found("ContractNotFound", "Contract not found"))?;

    dependency_tree(
        state,
        contract_ref::ResolvedContract { uuid: id, network },
        &DependencyQuery::default(),
        Direction::Dependencies,
    )
    .await
}

/// Run the traversal and fold its flat rows back into the nested
/// [`DependencyResponse`] shape.
///
/// The response type is kept **field-stable**: `graphql/types.rs` resolves
/// `Contract.dependencies` through it, so fields may be added with serde
/// defaults but never renamed or removed.
async fn dependency_tree(
    state: &AppState,
    contract: contract_ref::ResolvedContract,
    query: &DependencyQuery,
    direction: Direction,
) -> ApiResult<DependencyResponse> {
    let root = sqlx::query_as::<_, (String, String, String)>(
        "SELECT contract_id, name, verification_status::text FROM contracts WHERE id = $1",
    )
    .bind(contract.uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("get root contract for dependencies", err))?
    .ok_or_else(|| ApiError::not_found("ContractNotFound", "Contract not found"))?;

    let request = TraversalRequest {
        root: contract.uuid,
        network: contract.network,
        direction,
        transitive: query.transitive.unwrap_or(true),
        max_depth: query
            .depth
            .unwrap_or(shared::dependency_graph::DEFAULT_MAX_DEPTH),
        max_nodes: query
            .max_nodes
            .unwrap_or(shared::dependency_graph::DEFAULT_MAX_NODES),
        as_of: query.as_of,
        // Tenancy is enforced inside the traversal; this endpoint has no
        // authenticated actor, so it sees the public graph only.
        caller_org: None,
        include_telemetry: query.include_telemetry.unwrap_or(false),
    };

    let result = dependency_graph::traverse(&state.db, &request).await?;

    let has_circular = result
        .diagnostics
        .iter()
        .any(|d| d.kind == DiagnosticKind::Cycle);
    let max_depth = result
        .rows
        .iter()
        .map(|r| r.depth as usize)
        .max()
        .unwrap_or(0);
    let total_dependencies = result.rows.len();

    let (root_contract_id, root_name, root_status) = root;

    Ok(DependencyResponse {
        root: DependencyNode {
            contract_id: root_contract_id,
            resolved_id: Some(contract.uuid),
            name: Some(root_name),
            call_volume: 0,
            status: root_status,
            is_circular: false,
            dependencies: nest(&result.rows, contract.uuid),
            visualization_hints: serde_json::json!({
                "node_type": "root",
                "depth": 0,
                "truncated": result.truncated,
                "truncation_reason": result.truncation_reason,
                "diagnostics": result.diagnostics,
            }),
        },
        total_dependencies,
        max_depth,
        has_circular,
    })
}

/// Rebuild the nested tree from the flat, path-carrying traversal rows.
///
/// Grouping is keyed on the **parent path**, not on the parent contract id.
/// Keying on the id alone conflates every occurrence of a contract that is
/// reachable by more than one route: in a diamond, the shared node's children
/// would be attached once per incoming path and then rendered under every
/// occurrence, multiplying the subtree.
///
/// Each row already carries its full root-to-here path, so this is a single
/// grouping pass -- no further queries and no recursion over the database.
fn nest(rows: &[TraversedEdgeRow], root: Uuid) -> Vec<DependencyNode> {
    let mut by_parent: BTreeMap<Vec<Uuid>, Vec<&TraversedEdgeRow>> = BTreeMap::new();
    for row in rows {
        by_parent
            .entry(parent_path(row).to_vec())
            .or_default()
            .push(row);
    }
    children_of(&by_parent, &[root], 1)
}

/// The path of the node this row hangs under.
///
/// A row that contributed a node (resolved, non-cycle) has `path` ending at
/// that node, so its parent is one element shorter. A row that contributed no
/// node -- unresolved, or cycle-closing -- already ends at its source, which is
/// its parent.
fn parent_path(row: &TraversedEdgeRow) -> &[Uuid] {
    if row.target_contract_id.is_some() && row.cycle_with.is_none() {
        &row.path[..row.path.len().saturating_sub(1)]
    } else {
        &row.path
    }
}

fn children_of(
    by_parent: &BTreeMap<Vec<Uuid>, Vec<&TraversedEdgeRow>>,
    parent: &[Uuid],
    depth: usize,
) -> Vec<DependencyNode> {
    let Some(rows) = by_parent.get(parent) else {
        return Vec::new();
    };

    rows.iter()
        .map(|row| {
            let is_circular = row.cycle_with.is_some();
            // A cycle-closing row must not be expanded again here, or the tree
            // would be infinite even though the traversal itself terminated.
            let dependencies = if is_circular || row.target_contract_id.is_none() {
                Vec::new()
            } else {
                children_of(by_parent, &row.path, depth + 1)
            };

            DependencyNode {
                contract_id: display_ref(row),
                resolved_id: row.target_contract_id.filter(|_| row.visible),
                name: row.target_name.clone(),
                // Call volume is not carried by the edge model; the aggregate
                // table remains the source for it and is reported by the graph
                // endpoints rather than guessed at here.
                call_volume: 0,
                status: node_status(row),
                is_circular,
                dependencies,
                visualization_hints: serde_json::json!({
                    "depth": depth,
                    "node_type": if is_circular { "circular" } else { "standard" },
                    "edge_source": row.edge_source,
                    "edge_state": row.edge_state,
                    "redacted": !row.visible,
                }),
            }
        })
        .collect()
}

/// What to call a node in the response.
///
/// A redacted node is named by neither address nor name -- only the fact that it
/// exists. Echoing `target_ref` would defeat the redaction, since the reference
/// *is* the private contract's address.
fn display_ref(row: &TraversedEdgeRow) -> String {
    if row.visible {
        row.target_address
            .clone()
            .unwrap_or_else(|| row.target_ref.clone())
    } else {
        "[redacted]".to_string()
    }
}

fn node_status(row: &TraversedEdgeRow) -> String {
    match dependency_graph::parse_edge_state(&row.edge_state) {
        EdgeState::Resolved if !row.visible => "redacted".to_string(),
        EdgeState::Resolved => "resolved".to_string(),
        EdgeState::Unresolved => "unresolved".to_string(),
        EdgeState::NetworkMismatch => "network_mismatch".to_string(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Issue #610 — Write endpoint: declare contract dependencies
// ─────────────────────────────────────────────────────────────────────────────

/// Request body for POST /api/contracts/:id/dependencies
#[derive(Debug, serde::Deserialize)]
pub struct DeclareDependenciesRequest {
    pub dependencies: Vec<DependencyDeclaration>,
}

/// A declaration that was stored but could not be bound to a registry contract
/// (Issue #1147). Retained rather than dropped, because the operator declared
/// something real; reported so it is not mistaken for a resolved dependency.
#[derive(Debug, serde::Serialize)]
pub struct UnresolvedDependency {
    pub target_ref: String,
    pub reason: String,
}

/// Response for POST /api/contracts/:id/dependencies
#[derive(Debug, serde::Serialize)]
pub struct DeclareDependenciesResponse {
    pub contract_id: Uuid,
    pub saved: Vec<ContractDependency>,
    pub has_circular: bool,
    /// Empty when every declaration bound to a registered contract.
    #[serde(default)]
    pub unresolved: Vec<UnresolvedDependency>,
}

/// POST /api/contracts/:id/dependencies
///
/// Declare (or replace) the dependency list for a contract.
/// Circular dependencies are detected and flagged; they are stored but a warning
/// is included in the response.  Returns 201 Created on success.
///
/// Issue #610: dependencies stored and retrieved correctly, circular deps detected.
pub async fn declare_contract_dependencies(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(body): ValidatedJson<DeclareDependenciesRequest>,
) -> ApiResult<(StatusCode, Json<DeclareDependenciesResponse>)> {
    // Fetch the network alongside the existence check: a dependency reference
    // is only meaningful within one network (Issue #1147), so the declaring
    // contract's own network is what its targets are resolved against.
    let network: shared::Network =
        sqlx::query_scalar("SELECT network FROM contracts WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| db_internal_error("check contract exists", e))?
            .ok_or_else(|| ApiError::not_found("ContractNotFound", "Contract not found"))?;

    // Pre-check for self-referential declarations.
    let self_dep = body.dependencies.iter().any(|d| d.name == id.to_string());
    if self_dep {
        return Err(ApiError::bad_request(
            "SelfDependency",
            "A contract cannot declare itself as a dependency",
        ));
    }

    // Save declarations (will detect cycles against existing graph in the DB).
    let resolutions = dependency::save_dependencies(&state.db, id, network, &body.dependencies)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to save dependencies: {e}")))?;

    // Surface references that were stored but could not be bound, rather than
    // letting them appear as a silent NULL in `dependency_contract_id`.
    let unresolved: Vec<UnresolvedDependency> = body
        .dependencies
        .iter()
        .zip(&resolutions)
        .filter_map(|(decl, resolution)| {
            let reason = match resolution {
                dependency::DependencyResolution::Resolved(_) => return None,
                dependency::DependencyResolution::NetworkMismatch { found_on, .. } => {
                    format!("registered on {found_on}, not {network}")
                }
                dependency::DependencyResolution::UnknownAddress => {
                    "no contract with this address is registered".to_string()
                }
                dependency::DependencyResolution::NotAnAddress => {
                    "not a Stellar contract address".to_string()
                }
            };
            Some(UnresolvedDependency {
                target_ref: decl.name.clone(),
                reason,
            })
        })
        .collect();

    // Fetch stored rows for response.
    let saved: Vec<ContractDependency> = sqlx::query_as(
        r#"
        SELECT id, contract_id, dependency_name, dependency_contract_id,
               version_constraint, created_at
        FROM contract_dependencies
        WHERE contract_id = $1
        ORDER BY dependency_name
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_internal_error("fetch saved dependencies", e))?;

    // Detect whether any stored dep forms a cycle.
    let mut has_circular = false;
    for dep in &saved {
        if let Some(dep_id) = dep.dependency_contract_id {
            if dependency::detect_cycle(&state.db, id, dep_id)
                .await
                .unwrap_or(false)
            {
                has_circular = true;
                break;
            }
        }
    }

    Ok((
        StatusCode::CREATED,
        Json(DeclareDependenciesResponse {
            contract_id: id,
            saved,
            has_circular,
            unresolved,
        }),
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Issue #1147 — Dependency graph summary and transitive risk
// ─────────────────────────────────────────────────────────────────────────────

/// Response for GET /api/contracts/:id/dependency-graph
///
/// A flat summary rather than the nested tree: this endpoint exists to answer
/// "how big and how healthy is the closure", which a caller should be able to
/// read without walking a recursive structure.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct DependencyGraphSummary {
    pub contract_id: Uuid,
    pub network: String,
    pub total_dependencies: usize,
    /// Reachable contracts that resolved to a registry row.
    pub resolved: usize,
    /// References retained but not bound to any contract.
    pub unresolved: usize,
    /// Nodes hidden by tenancy: counted so the total stays honest, never named.
    pub redacted: usize,
    pub max_depth: usize,
    pub has_circular: bool,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation_reason: Option<shared::dependency_graph::TruncationReason>,
    /// Graph facts with no severity: cycles, unresolved edges, truncation.
    pub diagnostics: Vec<shared::dependency_graph::Diagnostic>,
    /// The reachable set as a flat, paginated list.
    ///
    /// Flat rather than nested because a tree cannot be coherently paginated:
    /// page 2 of a tree is not a tree. `/dependencies` returns the nested shape
    /// whole (bounded by `max_nodes`); this is the pageable view of the same
    /// traversal.
    pub nodes: shared::PaginatedResponse<DependencyGraphNode>,
}

/// One reachable contract, flattened out of the traversal.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct DependencyGraphNode {
    /// The target's address, or `[redacted]` when tenancy hides it.
    pub contract_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub depth: i32,
    pub edge_source: String,
    pub edge_state: String,
    /// Root-to-node path.
    pub path: Vec<Uuid>,
    pub redacted: bool,
}

/// GET /api/contracts/:id/dependency-graph
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/dependency-graph",
    params(
        ("id" = String, Path, description = "Contract UUID or Stellar contract address"),
        ("network" = Option<String>, Query, description = "Required to disambiguate an address registered on more than one network"),
        ("depth" = Option<u32>, Query, description = "Maximum traversal depth, capped server-side"),
        ("max_nodes" = Option<u32>, Query, description = "Maximum nodes returned, capped server-side"),
        ("as_of" = Option<String>, Query, description = "RFC3339 instant; replays the declared graph as it stood then"),
        ("include_telemetry" = Option<bool>, Query, description = "Include on-chain call edges alongside declared ones"),
        ("page" = Option<i64>, Query, description = "1-based page of the flat node list (default 1)"),
        ("per_page" = Option<i64>, Query, description = "Nodes per page, default 50, capped at 200")
    ),
    responses(
        (status = 200, description = "Dependency graph summary", body = DependencyGraphSummary),
        (status = 404, description = "Contract not found"),
        (status = 409, description = "Ambiguous contract address; retry with ?network=")
    ),
    tag = "Graphs"
)]
pub async fn get_dependency_graph(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DependencyQuery>,
) -> ApiResult<Json<DependencyGraphSummary>> {
    let contract = contract_ref::resolve(&state, &id, query.network).await?;

    let request = TraversalRequest {
        root: contract.uuid,
        network: contract.network,
        direction: Direction::Dependencies,
        transitive: query.transitive.unwrap_or(true),
        max_depth: query
            .depth
            .unwrap_or(shared::dependency_graph::DEFAULT_MAX_DEPTH),
        max_nodes: query
            .max_nodes
            .unwrap_or(shared::dependency_graph::DEFAULT_MAX_NODES),
        as_of: query.as_of,
        caller_org: None,
        include_telemetry: query.include_telemetry.unwrap_or(false),
    };

    let result = dependency_graph::traverse(&state.db, &request).await?;

    let resolved = result
        .rows
        .iter()
        .filter(|r| r.target_contract_id.is_some() && r.visible)
        .count();
    let redacted = result.rows.iter().filter(|r| !r.visible).count();
    let unresolved = result
        .rows
        .iter()
        .filter(|r| r.target_contract_id.is_none())
        .count();

    // Offset pagination over the flat node list. The rows arrive in the
    // traversal's total order, so a page boundary is stable for a given graph
    // rather than depending on row arrival.
    let per_page = query
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);
    let page = query.page.unwrap_or(1).max(1);
    let total = result.rows.len() as i64;
    let offset = ((page - 1) * per_page).min(total) as usize;
    let end = (offset + per_page as usize).min(result.rows.len());

    let items: Vec<DependencyGraphNode> = result.rows[offset..end]
        .iter()
        .map(|row| DependencyGraphNode {
            contract_id: display_ref(row),
            resolved_id: row.target_contract_id.filter(|_| row.visible),
            name: row.target_name.clone(),
            depth: row.depth,
            edge_source: row.edge_source.clone(),
            edge_state: row.edge_state.clone(),
            path: row.path.clone(),
            redacted: !row.visible,
        })
        .collect();

    Ok(Json(DependencyGraphSummary {
        contract_id: contract.uuid,
        network: contract.network.to_string(),
        nodes: shared::PaginatedResponse::new(items, total, page, per_page),
        total_dependencies: result.rows.len(),
        resolved,
        unresolved,
        redacted,
        max_depth: result
            .rows
            .iter()
            .map(|r| r.depth as usize)
            .max()
            .unwrap_or(0),
        has_circular: result
            .diagnostics
            .iter()
            .any(|d| d.kind == DiagnosticKind::Cycle),
        truncated: result.truncated,
        truncation_reason: result.truncation_reason,
        diagnostics: result.diagnostics,
    }))
}

/// GET /api/contracts/:id/dependency-risk
///
/// Direct and inherited findings, each with the shortest path that reaches it,
/// plus severity-free diagnostics.
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/dependency-risk",
    params(
        ("id" = String, Path, description = "Contract UUID or Stellar contract address"),
        ("network" = Option<String>, Query, description = "Required to disambiguate an address registered on more than one network"),
        ("depth" = Option<u32>, Query, description = "Maximum traversal depth, capped server-side")
    ),
    responses(
        (status = 200, description = "Transitive risk report", body = Object),
        (status = 404, description = "Contract not found"),
        (status = 409, description = "Ambiguous contract address; retry with ?network=")
    ),
    tag = "Graphs"
)]
pub async fn get_dependency_risk(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DependencyQuery>,
) -> ApiResult<Json<crate::dependency_risk::DependencyRiskReport>> {
    let contract = contract_ref::resolve(&state, &id, query.network).await?;

    let report = crate::dependency_risk::assess(
        &state,
        contract.uuid,
        contract.network,
        query
            .depth
            .unwrap_or(shared::dependency_graph::DEFAULT_MAX_DEPTH),
        None,
    )
    .await?;

    Ok(Json(report))
}
