//! Bounded, tenancy-safe dependency traversal (Issue #1147).
//!
//! One `WITH RECURSIVE` CTE replaces three separate N+1 application-level walks.
//! The one in `dependency_handlers` was `#[async_recursion]` and removed each
//! node from its `visited` set on unwind, so a diamond was re-expanded once per
//! incoming path -- exponential on wide graphs, and non-deterministic in which
//! path it reported for a shared node.
//!
//! ## How the traversal is bounded
//!
//! Three independent bounds, because each fails differently:
//!
//! - **Cycle guard.** The CTE carries the root-to-here path as a `uuid[]` and
//!   only descends when `NOT (next_target = ANY(path))`. Without it a cyclic
//!   graph makes the CTE non-terminating; the depth bound alone would mask that
//!   as silent truncation rather than reporting the cycle.
//! - **Depth bound.** `depth < $limit` in the recursive term.
//! - **Node budget and time budget.** `LIMIT` on the outer select, plus
//!   `SET LOCAL statement_timeout` inside a transaction. The timeout is not
//!   redundant: dropping an axum future does **not** cancel an in-flight sqlx
//!   query. The Postgres backend keeps executing and the pooled connection stays
//!   checked out, so a client that walks away could otherwise hold a connection
//!   for the full runtime of a pathological query.
//!
//! ## Tenancy
//!
//! Every traversal query carries the visibility predicate. A node the caller may
//! not see is still *counted* -- so `total_dependencies` stays honest -- but is
//! returned as a redacted placeholder with no name, address, or publisher. The
//! predicate is applied inside the recursive term, not as a post-filter, so a
//! private contract cannot even be used as a stepping stone to enumerate the
//! graph behind it.
//!
//! ## Query style
//!
//! Runtime `sqlx::query_as` + `.bind()` + `#[derive(FromRow)]`, matching the
//! rest of the tree. There are zero compile-time query macros in this repo and
//! introducing one would add a build-time database requirement.

use crate::error::{ApiError, ApiResult};
use crate::handlers::db_internal_error;
use shared::dependency_graph::{
    clamp_depth, clamp_max_nodes, cycle_segment, sort_diagnostics, Diagnostic, DiagnosticKind,
    EdgeSource, EdgeState, TruncationReason, TRAVERSAL_STATEMENT_TIMEOUT_MS,
};
use shared::Network;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// Which way to walk the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// What this contract depends on.
    Dependencies,
    /// What depends on this contract.
    Dependents,
}

/// Inputs to one traversal.
#[derive(Debug, Clone)]
pub struct TraversalRequest {
    pub root: Uuid,
    pub network: Network,
    pub direction: Direction,
    /// `false` walks only direct edges (depth 1).
    pub transitive: bool,
    pub max_depth: u32,
    pub max_nodes: usize,
    /// Replay the declared graph as it stood at this instant. `None` is "now".
    pub as_of: Option<chrono::DateTime<chrono::Utc>>,
    /// Organization of the caller, or `None` for an anonymous/public caller.
    pub caller_org: Option<Uuid>,
    /// Include telemetry-derived edges alongside declared ones.
    pub include_telemetry: bool,
}

impl TraversalRequest {
    /// Effective depth: 1 for a direct-only walk, the clamped value otherwise.
    pub fn effective_depth(&self) -> u32 {
        if self.transitive {
            clamp_depth(Some(self.max_depth))
        } else {
            1
        }
    }

    pub fn effective_max_nodes(&self) -> usize {
        clamp_max_nodes(Some(self.max_nodes))
    }
}

/// One reachable node, with the path that reached it.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TraversedEdgeRow {
    pub depth: i32,
    /// NULL for an unresolved or cross-network edge.
    pub target_contract_id: Option<Uuid>,
    pub target_ref: String,
    pub network: Network,
    pub edge_source: String,
    pub edge_state: String,
    pub version_constraint: Option<String>,
    pub expected_interface_id: Option<String>,
    /// The target's interface id *now*, JOINed rather than denormalized.
    pub current_interface_id: Option<String>,
    pub source_contract_id: Uuid,
    /// Root-to-node path. Ends at the *source* for an edge that contributed no
    /// node (unresolved, or cycle-closing).
    pub path: Vec<Uuid>,
    /// Set when this edge closes a loop: the already-visited contract it points
    /// back to.
    pub cycle_with: Option<Uuid>,
    /// True when the walk did not descend past this row.
    pub terminal: bool,
    /// False when tenancy hides this node's details from the caller.
    pub visible: bool,
    pub target_address: Option<String>,
    pub target_name: Option<String>,
}

/// Result of one traversal: the reachable set plus what had to be reported
/// about the shape of the walk.
#[derive(Debug, Clone)]
pub struct TraversalResult {
    pub rows: Vec<TraversedEdgeRow>,
    pub diagnostics: Vec<Diagnostic>,
    pub truncated: bool,
    pub truncation_reason: Option<TruncationReason>,
}

/// The recursive CTE.
///
/// Bind order:
///   $1 root uuid, $2 network, $3 as_of (nullable), $4 depth limit,
///   $5 caller org (nullable), $6 include telemetry, $7 node budget (+1 probe)
///
/// The `+1` on the outer LIMIT is a truncation probe: fetching one row beyond
/// the budget is how we distinguish "the graph is exactly this big" from "we
/// stopped". Reporting `truncated: false` on a graph that was in fact clipped
/// would be the worst possible failure mode for a tool people gate deploys on.
const FORWARD_TRAVERSAL_SQL: &str = r#"
-- Bind order: $1 root, $2 network, $3 as_of, $4 depth limit,
--             $5 caller org, $6 include telemetry, $7 node budget (+1 probe)
WITH RECURSIVE edges AS (
    -- Declared edges, resolved to their target's identity and visibility once
    -- here rather than in the outer select, so the recursive term can test
    -- visibility without re-joining and without leaking a private node's
    -- onward dependencies.
    SELECT
        e.source_contract_id,
        e.target_contract_id,
        e.target_ref,
        e.network,
        e.edge_source::text AS edge_source,
        e.edge_state::text  AS edge_state,
        e.version_constraint,
        e.expected_interface_id,
        -- COALESCE is load-bearing: `c.organization_id = $5` yields NULL, not
        -- false, when either side is NULL, and `false OR NULL` is NULL in SQL's
        -- three-valued logic. Without it a private contract read by a caller
        -- with no organization produced a NULL `visible`, which neither decodes
        -- into a bool nor means "hidden".
        (c.id IS NULL
         OR COALESCE(c.visibility = 'public', false)
         OR COALESCE(c.organization_id = $5::uuid, false)) AS target_visible,
        c.contract_id  AS target_address,
        c.name         AS target_name,
        c.interface_id AS target_interface_id
    FROM contract_dependency_edges e
    LEFT JOIN contracts c ON c.id = e.target_contract_id
    WHERE e.network = $2
      AND (
            -- `as_of` replays declared history: recorded by then, and not yet
            -- superseded at that instant.
            ($3::timestamptz IS NULL AND e.superseded_at IS NULL)
         OR ($3::timestamptz IS NOT NULL
             AND e.recorded_at <= $3
             AND (e.superseded_at IS NULL OR e.superseded_at > $3))
          )
      AND ($6::boolean OR e.edge_source = 'declared')

    UNION ALL

    -- Telemetry edges are derived live rather than materialized. The aggregate
    -- table is upserted on every contract invocation, so copying it into the
    -- bitemporal table would add a supersede+insert to the hottest write path in
    -- the system for zero information gain -- it already *is* the historical
    -- record, which is also why `day <= as_of` gives exact historical state free.
    SELECT DISTINCT
        a.source_contract_id,
        a.target_contract_id,
        t.contract_id AS target_ref,
        a.network,
        'telemetry'::text,
        'resolved'::text,
        NULL::text,
        NULL::text,
        (COALESCE(t.visibility = 'public', false)
         OR COALESCE(t.organization_id = $5::uuid, false)) AS target_visible,
        t.contract_id,
        t.name,
        t.interface_id
    FROM contract_call_edge_daily_aggregates a
    JOIN contracts t ON t.id = a.target_contract_id
    WHERE $6::boolean
      AND a.network = $2
      AND ($3::timestamptz IS NULL OR a.day <= $3::date)
),
walk AS (
    SELECT
        1 AS depth,
        e.source_contract_id,
        e.target_contract_id,
        e.target_ref,
        e.network,
        e.edge_source,
        e.edge_state,
        e.version_constraint,
        e.expected_interface_id,
        e.target_visible,
        e.target_address,
        e.target_name,
        e.target_interface_id,
        -- An unresolved edge contributes no node, so its path ends at the source.
        CASE WHEN e.target_contract_id IS NULL
             THEN ARRAY[e.source_contract_id]
             ELSE ARRAY[e.source_contract_id, e.target_contract_id]
        END AS path,
        NULL::uuid AS cycle_with,
        -- `terminal` marks a row the walk must not descend from. Carrying it as
        -- a column, instead of filtering the edge out of the recursive term, is
        -- what lets the row still be emitted: an unresolved reference, a
        -- cycle-closing edge, and a redacted node are all things the caller
        -- needs reported. Filtering them would make them invisible, not terminal.
        (e.edge_state <> 'resolved'
         OR e.target_contract_id IS NULL
         -- A node the caller cannot see is not a stepping stone. Stopping here
         -- means a private contract's own dependencies are never enumerated
         -- through it; the redacted placeholder still says the graph continues.
         OR NOT e.target_visible) AS terminal
    FROM edges e
    WHERE e.source_contract_id = $1

    UNION ALL

    SELECT
        w.depth + 1,
        e.source_contract_id,
        e.target_contract_id,
        e.target_ref,
        e.network,
        e.edge_source,
        e.edge_state,
        e.version_constraint,
        e.expected_interface_id,
        e.target_visible,
        e.target_address,
        e.target_name,
        e.target_interface_id,
        CASE WHEN e.target_contract_id IS NULL OR e.target_contract_id = ANY(w.path)
             THEN w.path
             ELSE w.path || e.target_contract_id
        END,
        -- The node that closes the loop, so the diagnostic can name the cycle
        -- rather than the whole path leading into it.
        CASE WHEN e.target_contract_id = ANY(w.path) THEN e.target_contract_id END,
        (e.edge_state <> 'resolved'
         OR e.target_contract_id IS NULL
         OR NOT e.target_visible
         -- Cycle guard. Emitted once, then terminal, so the CTE terminates on a
         -- cyclic graph AND the cycle is reportable instead of looking like
         -- silent depth truncation.
         OR e.target_contract_id = ANY(w.path)) AS terminal
    FROM walk w
    JOIN edges e ON e.source_contract_id = w.target_contract_id
    WHERE w.depth < $4
      AND NOT w.terminal
)
SELECT
    w.depth,
    w.target_contract_id,
    w.target_ref,
    w.network,
    w.edge_source,
    w.edge_state,
    w.version_constraint,
    w.expected_interface_id,
    w.source_contract_id,
    w.path,
    w.cycle_with,
    w.terminal,
    w.target_visible AS visible,
    CASE WHEN w.target_visible THEN w.target_address END AS target_address,
    CASE WHEN w.target_visible THEN w.target_name END AS target_name,
    -- Also gated: an interface fingerprint is a stable identifier for a private
    -- contract's ABI and would confirm a guess about what it exposes.
    CASE WHEN w.target_visible THEN w.target_interface_id END AS current_interface_id
FROM walk w
-- Deterministic total order. No floats in any sort key, and the trailing columns
-- make the order total even when depth and target coincide.
ORDER BY w.depth ASC, w.network ASC, w.target_contract_id ASC NULLS LAST,
         w.target_ref ASC, w.edge_source ASC
LIMIT $7
"#;

/// Reverse traversal for `/dependents`. Structurally identical to
/// [`FORWARD_TRAVERSAL_SQL`] with source and target exchanged; kept as a second
/// literal rather than string-built so both remain greppable and reviewable.
const REVERSE_TRAVERSAL_SQL: &str = r#"
-- Same bind order as FORWARD_TRAVERSAL_SQL.
WITH RECURSIVE edges AS (
    -- Reverse direction, so it is the *source* whose identity and visibility
    -- matter: it is the dependent being reported.
    SELECT
        e.source_contract_id,
        e.target_contract_id,
        e.target_ref,
        e.network,
        e.edge_source::text AS edge_source,
        e.edge_state::text  AS edge_state,
        e.version_constraint,
        e.expected_interface_id,
        (COALESCE(c.visibility = 'public', false)
         OR COALESCE(c.organization_id = $5::uuid, false)) AS source_visible,
        c.contract_id  AS source_address,
        c.name         AS source_name,
        c.interface_id AS source_interface_id
    FROM contract_dependency_edges e
    JOIN contracts c ON c.id = e.source_contract_id
    WHERE e.network = $2
      -- Only a resolved edge names a contract that could be a dependent.
      AND e.target_contract_id IS NOT NULL
      AND (
            ($3::timestamptz IS NULL AND e.superseded_at IS NULL)
         OR ($3::timestamptz IS NOT NULL
             AND e.recorded_at <= $3
             AND (e.superseded_at IS NULL OR e.superseded_at > $3))
          )
      AND ($6::boolean OR e.edge_source = 'declared')

    UNION ALL

    SELECT DISTINCT
        a.source_contract_id,
        a.target_contract_id,
        s.contract_id AS target_ref,
        a.network,
        'telemetry'::text,
        'resolved'::text,
        NULL::text,
        NULL::text,
        (COALESCE(s.visibility = 'public', false)
         OR COALESCE(s.organization_id = $5::uuid, false)) AS source_visible,
        s.contract_id,
        s.name,
        s.interface_id
    FROM contract_call_edge_daily_aggregates a
    JOIN contracts s ON s.id = a.source_contract_id
    WHERE $6::boolean
      AND a.network = $2
      AND ($3::timestamptz IS NULL OR a.day <= $3::date)
),
walk AS (
    -- Column names match the forward query exactly -- `target_contract_id` here
    -- is the dependent -- so one row struct, one diagnostic derivation, and one
    -- ordering serve both directions.
    SELECT
        1 AS depth,
        e.target_contract_id AS source_contract_id,
        e.source_contract_id AS target_contract_id,
        e.target_ref,
        e.network,
        e.edge_source,
        e.edge_state,
        e.version_constraint,
        e.expected_interface_id,
        e.source_visible   AS target_visible,
        e.source_address   AS target_address,
        e.source_name      AS target_name,
        e.source_interface_id AS target_interface_id,
        ARRAY[e.target_contract_id, e.source_contract_id] AS path,
        NULL::uuid AS cycle_with,
        (NOT e.source_visible) AS terminal
    FROM edges e
    WHERE e.target_contract_id = $1

    UNION ALL

    SELECT
        w.depth + 1,
        e.target_contract_id,
        e.source_contract_id,
        e.target_ref,
        e.network,
        e.edge_source,
        e.edge_state,
        e.version_constraint,
        e.expected_interface_id,
        e.source_visible,
        e.source_address,
        e.source_name,
        e.source_interface_id,
        CASE WHEN e.source_contract_id = ANY(w.path)
             THEN w.path
             ELSE w.path || e.source_contract_id
        END,
        CASE WHEN e.source_contract_id = ANY(w.path) THEN e.source_contract_id END,
        (NOT e.source_visible OR e.source_contract_id = ANY(w.path)) AS terminal
    FROM walk w
    JOIN edges e ON e.target_contract_id = w.target_contract_id
    WHERE w.depth < $4
      AND NOT w.terminal
)
SELECT
    w.depth,
    w.target_contract_id,
    w.target_ref,
    w.network,
    w.edge_source,
    w.edge_state,
    w.version_constraint,
    w.expected_interface_id,
    w.source_contract_id,
    w.path,
    w.cycle_with,
    w.terminal,
    w.target_visible AS visible,
    CASE WHEN w.target_visible THEN w.target_address END AS target_address,
    CASE WHEN w.target_visible THEN w.target_name END AS target_name,
    CASE WHEN w.target_visible THEN w.target_interface_id END AS current_interface_id
FROM walk w
ORDER BY w.depth ASC, w.network ASC, w.target_contract_id ASC NULLS LAST,
         w.target_ref ASC, w.edge_source ASC
LIMIT $7
"#;

/// Walk the dependency graph from `request.root`.
pub async fn traverse(pool: &PgPool, request: &TraversalRequest) -> ApiResult<TraversalResult> {
    let depth = request.effective_depth();
    let budget = request.effective_max_nodes();

    let mut tx = pool
        .begin()
        .await
        .map_err(|err| db_internal_error("begin dependency traversal", err))?;

    apply_statement_timeout(&mut tx).await?;

    let sql = match request.direction {
        Direction::Dependencies => FORWARD_TRAVERSAL_SQL,
        Direction::Dependents => REVERSE_TRAVERSAL_SQL,
    };

    let rows: Vec<TraversedEdgeRow> = sqlx::query_as(sql)
        .bind(request.root)
        .bind(request.network)
        .bind(request.as_of)
        .bind(depth as i32)
        .bind(request.caller_org)
        .bind(request.include_telemetry)
        // +1 row is the truncation probe; see FORWARD_TRAVERSAL_SQL.
        .bind(budget as i64 + 1)
        .fetch_all(&mut *tx)
        .await
        .map_err(traversal_error)?;

    // The transaction is read-only; committing releases the connection and the
    // SET LOCAL timeout in one step.
    tx.commit()
        .await
        .map_err(|err| db_internal_error("commit dependency traversal", err))?;

    Ok(assemble(rows, depth, budget))
}

/// Bound the query in wall-clock time as well as in rows.
///
/// `SET LOCAL` scopes the timeout to this transaction, so a pooled connection
/// is never handed to the next request carrying a modified timeout.
async fn apply_statement_timeout(tx: &mut Transaction<'_, Postgres>) -> ApiResult<()> {
    // Interpolated, not bound: SET does not accept bind parameters. The value is
    // a compile-time constant, never caller input.
    let sql = format!("SET LOCAL statement_timeout = {TRAVERSAL_STATEMENT_TIMEOUT_MS}");
    sqlx::query(&sql)
        .execute(&mut **tx)
        .await
        .map_err(|err| db_internal_error("set traversal statement timeout", err))?;
    Ok(())
}

/// Map a traversal failure, distinguishing the time budget from a real error.
///
/// A statement timeout is `57014 query_canceled`. Reporting it as a 500 would
/// tell the operator the registry is broken when in fact their graph is too
/// large for the budget -- a different problem with a different fix.
fn traversal_error(err: sqlx::Error) -> ApiError {
    if let sqlx::Error::Database(ref db_err) = err {
        if db_err.code().as_deref() == Some("57014") {
            return ApiError::service_unavailable_with(
                "DependencyTraversalTimeout",
                format!(
                    "The dependency traversal exceeded its {TRAVERSAL_STATEMENT_TIMEOUT_MS}ms budget. Retry with a smaller depth or max_nodes."
                ),
            );
        }
    }
    db_internal_error("dependency traversal", err)
}

/// Turn raw rows into a result: trim the probe row, and derive the diagnostics
/// that the SQL deliberately does not compute.
fn assemble(mut rows: Vec<TraversedEdgeRow>, depth: u32, budget: usize) -> TraversalResult {
    let mut truncated = false;
    let mut truncation_reason = None;

    if rows.len() > budget {
        rows.truncate(budget);
        truncated = true;
        truncation_reason = Some(TruncationReason::NodeLimit);
    } else if rows
        .iter()
        .any(|row| row.depth as u32 >= depth && !row.terminal)
    {
        // Truncated only if a row sitting *at* the depth limit could still have
        // been descended from. A chain that happens to be exactly `depth` long
        // and ends in terminal rows is a complete answer, and reporting it as
        // truncated would make every exact-fit graph look clipped.
        truncated = true;
        truncation_reason = Some(TruncationReason::DepthLimit);
    }

    let mut diagnostics = derive_diagnostics(&rows);

    if let Some(reason) = truncation_reason {
        let limit = match reason {
            TruncationReason::NodeLimit => budget as u64,
            TruncationReason::DepthLimit => depth as u64,
            TruncationReason::TimeLimit => u64::from(TRAVERSAL_STATEMENT_TIMEOUT_MS),
        };
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::Truncated,
            path: Vec::new(),
            detail: reason.detail(limit),
        });
    }

    sort_diagnostics(&mut diagnostics);

    TraversalResult {
        rows,
        diagnostics,
        truncated,
        truncation_reason,
    }
}

/// Diagnostics derivable from the returned rows alone.
///
/// Cycles are found here rather than in SQL because the CTE's guard *prevents*
/// the revisit -- it never emits the row that would prove the loop. The evidence
/// is instead an edge whose target already appears on its own path prefix.
fn derive_diagnostics(rows: &[TraversedEdgeRow]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for row in rows {
        match row.edge_state.as_str() {
            "unresolved" => diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnresolvedEdge,
                path: row.path.clone(),
                detail: unresolved_detail(&row.target_ref),
            }),
            "network_mismatch" => diagnostics.push(Diagnostic {
                kind: DiagnosticKind::NetworkMismatch,
                path: row.path.clone(),
                detail: format!(
                    "'{}' is registered on a different network than {}",
                    row.target_ref, row.network
                ),
            }),
            _ => {}
        }

        if !row.visible {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::RedactedNode,
                path: row.path.clone(),
                detail: format!("a node at depth {} is not visible to you", row.depth),
            });
        }

        // `cycle_with` is set by the CTE on the one row that closes a loop. It
        // has to come from the query rather than be inferred here, because the
        // guard stops the walk *before* the repeat would show up in the path.
        if let Some(repeated) = row.cycle_with {
            if let Some(segment) = cycle_segment(&row.path, repeated) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::Cycle,
                    path: segment,
                    detail: "dependency cycle; traversal stopped at the repeated contract"
                        .to_string(),
                });
            }
        }
    }

    // Several paths can produce the same diagnostic; sorting first makes dedup
    // total rather than only collapsing adjacent duplicates.
    sort_diagnostics(&mut diagnostics);
    diagnostics.dedup();
    diagnostics
}

/// Unresolved edges are split by shape (Issue #1147).
///
/// After dependencies stopped being inferred from arbitrary strings, most
/// unresolved references are free-form library names, and treating every one as
/// a gap would bury the real signal. A syntactically valid contract address that
/// is not registered is a genuine hole in the graph; anything else is an
/// undeclared library and merely informational.
fn unresolved_detail(target_ref: &str) -> String {
    if crate::validation::validate_contract_id(target_ref).is_ok() {
        format!("'{target_ref}' is a valid contract address but is not registered")
    } else {
        format!("'{target_ref}' is not a contract address; treated as an undeclared library")
    }
}

/// Parse the textual `edge_source`/`edge_state` columns back into the shared
/// enums. Unknown values degrade rather than fail: a future enum value added by
/// a newer migration must not 500 an older reader.
pub fn parse_edge_source(value: &str) -> EdgeSource {
    match value {
        "telemetry" => EdgeSource::Telemetry,
        _ => EdgeSource::Declared,
    }
}

pub fn parse_edge_state(value: &str) -> EdgeState {
    match value {
        "resolved" => EdgeState::Resolved,
        "network_mismatch" => EdgeState::NetworkMismatch,
        _ => EdgeState::Unresolved,
    }
}
