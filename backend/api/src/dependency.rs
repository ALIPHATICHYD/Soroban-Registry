use anyhow::Result;
use shared::{DependencyDeclaration, GraphEdge, GraphNode, GraphResponse, Network};
use sqlx::PgPool;
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

fn strongly_connected_components(
    node_ids: &[Uuid],
    edges: &[(Uuid, Uuid)],
) -> (HashMap<Uuid, usize>, Vec<usize>) {
    let mut graph: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    let mut reverse_graph: HashMap<Uuid, Vec<Uuid>> = HashMap::new();

    for &node_id in node_ids {
        graph.entry(node_id).or_default();
        reverse_graph.entry(node_id).or_default();
    }

    for &(source, target) in edges {
        graph.entry(source).or_default().push(target);
        reverse_graph.entry(target).or_default().push(source);
    }

    let mut visited = HashSet::new();
    let mut finish_order: Vec<Uuid> = Vec::with_capacity(node_ids.len());

    for &start in node_ids {
        if visited.contains(&start) {
            continue;
        }

        let mut stack: Vec<(Uuid, usize)> = vec![(start, 0)];
        visited.insert(start);

        while let Some((node, next_idx)) = stack.pop() {
            let neighbors = graph.get(&node).map(|n| n.as_slice()).unwrap_or(&[]);

            if next_idx < neighbors.len() {
                stack.push((node, next_idx + 1));
                let next = neighbors[next_idx];
                if visited.insert(next) {
                    stack.push((next, 0));
                }
            } else {
                finish_order.push(node);
            }
        }
    }

    let mut component_by_node: HashMap<Uuid, usize> = HashMap::new();
    let mut component_sizes: Vec<usize> = Vec::new();

    for &start in finish_order.iter().rev() {
        if component_by_node.contains_key(&start) {
            continue;
        }

        let component_idx = component_sizes.len();
        let mut stack = vec![start];
        let mut size = 0usize;

        while let Some(node) = stack.pop() {
            if component_by_node.contains_key(&node) {
                continue;
            }

            component_by_node.insert(node, component_idx);
            size += 1;

            if let Some(neighbors) = reverse_graph.get(&node) {
                for &next in neighbors {
                    if !component_by_node.contains_key(&next) {
                        stack.push(next);
                    }
                }
            }
        }

        component_sizes.push(size);
    }

    (component_by_node, component_sizes)
}

/// Calculate transitive closure of dependencies (all recursive dependencies)
pub async fn get_transitive_dependencies(pool: &PgPool, root_id: Uuid) -> Result<Vec<Uuid>> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(root_id);
    visited.insert(root_id);

    let mut result = Vec::new();

    while let Some(current_id) = queue.pop_front() {
        let deps: Vec<Uuid> = sqlx::query_scalar(
            "SELECT dependency_contract_id FROM contract_static_dependencies WHERE contract_id = $1 AND dependency_contract_id IS NOT NULL"
        )
        .bind(current_id)
        .fetch_all(pool)
        .await?;

        for dep_id in deps {
            if !visited.contains(&dep_id) {
                visited.insert(dep_id);
                queue.push_back(dep_id);
                result.push(dep_id);
            }
        }
    }

    Ok(result)
}

/// Calculate transitive closure of dependents (all contracts affected by this one)
pub async fn get_transitive_dependents(pool: &PgPool, root_id: Uuid) -> Result<Vec<Uuid>> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(root_id);
    visited.insert(root_id);

    let mut result = Vec::new();

    while let Some(current_id) = queue.pop_front() {
        let dependents: Vec<Uuid> = sqlx::query_scalar(
            "SELECT contract_id FROM contract_static_dependencies WHERE dependency_contract_id = $1",
        )
        .bind(current_id)
        .fetch_all(pool)
        .await?;

        for dep_id in dependents {
            if !visited.contains(&dep_id) {
                visited.insert(dep_id);
                queue.push_back(dep_id);
                result.push(dep_id);
            }
        }
    }

    Ok(result)
}

/// Detect if adding a dependency would create a cycle
pub async fn detect_cycle(pool: &PgPool, start_node: Uuid, potential_dep: Uuid) -> Result<bool> {
    if start_node == potential_dep {
        return Ok(true);
    }

    // If potential_dep already depends on start_node (directly or indirectly), adding start_node -> potential_dep creates a cycle
    let transitive_deps = get_transitive_dependencies(pool, potential_dep).await?;
    Ok(transitive_deps.contains(&start_node))
}

/// Build D3-compatible graph representation
pub async fn build_dependency_graph(
    pool: &PgPool,
    network: Option<shared::Network>,
) -> Result<GraphResponse> {
    let contracts: Vec<GraphNode> = sqlx::query_as(
        "SELECT id, contract_id, name, network, is_verified, category, tags 
         FROM contracts
         WHERE ($1::network_type IS NULL OR network = $1)",
    )
    .bind(network.as_ref())
    .fetch_all(pool)
    .await?;

    let node_ids: Vec<Uuid> = contracts.iter().map(|node| node.id).collect();
    let edge_rows: Vec<(Uuid, Uuid)> = if node_ids.is_empty() {
        Vec::new()
    } else {
        sqlx::query_as(
            "SELECT contract_id as source, dependency_contract_id as target
             FROM contract_static_dependencies
             WHERE dependency_contract_id IS NOT NULL
               AND contract_id = ANY($1)
               AND dependency_contract_id = ANY($1)",
        )
        .bind(&node_ids)
        .fetch_all(pool)
        .await?
    };

    let exact_edge_counts: HashMap<(Uuid, Uuid), i64> = if node_ids.is_empty() {
        HashMap::new()
    } else {
        let rows: Vec<(Uuid, Uuid, i64)> = sqlx::query_as(
            "SELECT source_contract_id, target_contract_id, COALESCE(SUM(call_count), 0)::bigint AS total
             FROM contract_call_edge_daily_aggregates
             WHERE source_contract_id = ANY($1)
               AND target_contract_id = ANY($1)
               AND ($2::network_type IS NULL OR network = $2)
             GROUP BY source_contract_id, target_contract_id",
        )
        .bind(&node_ids)
        .bind(network.as_ref())
        .fetch_all(pool)
        .await?;
        rows.into_iter()
            .map(|(source, target, total)| ((source, target), total))
            .collect()
    };

    let source_interaction_counts: HashMap<Uuid, i64> = if node_ids.is_empty() {
        HashMap::new()
    } else {
        let rows: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT contract_id, COALESCE(SUM(count), 0)::bigint AS total
             FROM contract_interaction_daily_aggregates
             WHERE contract_id = ANY($1)
               AND interaction_type = 'invoke'
               AND ($2::network_type IS NULL OR network = $2)
             GROUP BY contract_id",
        )
        .bind(&node_ids)
        .bind(network.as_ref())
        .fetch_all(pool)
        .await?;
        rows.into_iter().collect()
    };

    let mut out_degree: HashMap<Uuid, i64> = HashMap::new();
    for (source, _) in &edge_rows {
        *out_degree.entry(*source).or_insert(0) += 1;
    }

    let (component_by_node, component_sizes) = strongly_connected_components(&node_ids, &edge_rows);

    let edges: Vec<GraphEdge> = edge_rows
        .into_iter()
        .map(|(source, target)| {
            let exact_frequency = exact_edge_counts.get(&(source, target)).copied();
            let source_total = source_interaction_counts.get(&source).copied();
            let degree = out_degree.get(&source).copied().unwrap_or(0);

            let inferred_frequency = if degree > 0 {
                source_total
                    .filter(|total| *total > 0)
                    .map(|total| (total / degree).max(1))
            } else {
                None
            };

            let is_estimated = exact_frequency.is_none() && inferred_frequency.is_some();
            let call_frequency = exact_frequency.or(inferred_frequency);

            let component_source = component_by_node.get(&source).copied();
            let component_target = component_by_node.get(&target).copied();
            let is_circular = match (component_source, component_target) {
                (Some(cs), Some(ct)) if cs == ct => {
                    component_sizes.get(cs).copied().unwrap_or(0) > 1 || source == target
                }
                _ => false,
            };

            GraphEdge {
                source,
                target,
                dependency_type: "calls".to_string(),
                call_frequency,
                call_volume: call_frequency,
                is_estimated,
                is_circular,
            }
        })
        .collect();

    Ok(GraphResponse {
        nodes: contracts,
        edges,
    })
}

/// Look up a contract by an opaque identifier: a registry UUID or a Stellar
/// contract address.
///
/// Deliberately does **not** resolve by `contracts.name` (Issue #1147).
/// `contracts.name` carries no UNIQUE constraint, so a name lookup silently
/// picks an arbitrary row among duplicates. It also does not have the old
/// unchecked `Uuid::parse_str` fast path, which returned an id for a UUID that
/// matched no row at all.
///
/// This is a general read-side helper for path parameters and telemetry
/// targets. It is **not** how a dependency edge is bound — see
/// [`resolve_dependency_target`], which additionally requires a network.
pub async fn lookup_contract_by_identifier(
    pool: &PgPool,
    identifier: &str,
) -> Result<Option<Uuid>> {
    let identifier = identifier.trim();

    if let Ok(uuid) = Uuid::parse_str(identifier) {
        let id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM contracts WHERE id = $1")
            .bind(uuid)
            .fetch_optional(pool)
            .await?;
        return Ok(id);
    }

    let id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM contracts WHERE contract_id = $1")
        .bind(identifier)
        .fetch_optional(pool)
        .await?;

    Ok(id)
}

/// Outcome of binding a declared dependency reference to a registry row.
///
/// An unresolved or cross-network reference is **retained, not discarded**: the
/// operator declared something real, and dropping it would silently shrink the
/// graph. It is simply never treated as a dependency for risk purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyResolution {
    /// Bound to a contract registered on the requested network.
    Resolved(Uuid),
    /// A syntactically valid contract address that is registered on some other
    /// network. An explicit diagnostic, never a silent cross-network binding.
    NetworkMismatch {
        contract_uuid: Uuid,
        found_on: Network,
    },
    /// A syntactically valid contract address that is not registered at all.
    UnknownAddress,
    /// Not a Stellar contract address. Free-form names are informational only.
    NotAnAddress,
}

/// Resolve a declared dependency reference to a registry contract, scoped to a
/// network.
///
/// Two rules, both required by Issue #1147:
///
/// - **Addresses only.** The reference must pass
///   [`crate::validation::validate_contract_id`]. Resolving arbitrary strings
///   against `contracts.name` bound dependencies to whatever row happened to
///   share a name; names are not unique and are chosen by the publisher.
/// - **Network scoped.** `contracts` is `UNIQUE(contract_id, network)`, so an
///   address alone is ambiguous. Without the predicate a mainnet contract could
///   be recorded as the dependency of a testnet one.
pub async fn resolve_dependency_target(
    pool: &PgPool,
    target_ref: &str,
    network: Network,
) -> Result<DependencyResolution> {
    let target_ref = target_ref.trim();

    if crate::validation::validate_contract_id(target_ref).is_err() {
        return Ok(DependencyResolution::NotAnAddress);
    }

    if let Some(id) = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM contracts WHERE contract_id = $1 AND network = $2",
    )
    .bind(target_ref)
    .bind(network)
    .fetch_optional(pool)
    .await?
    {
        return Ok(DependencyResolution::Resolved(id));
    }

    // Registered, but somewhere else. Reported rather than bound.
    if let Some((id, found_on)) = sqlx::query_as::<_, (Uuid, Network)>(
        "SELECT id, network FROM contracts WHERE contract_id = $1 ORDER BY network LIMIT 1",
    )
    .bind(target_ref)
    .fetch_optional(pool)
    .await?
    {
        return Ok(DependencyResolution::NetworkMismatch {
            contract_uuid: id,
            found_on,
        });
    }

    Ok(DependencyResolution::UnknownAddress)
}

/// Replace a contract's declared dependencies with `decls`.
///
/// The whole set is replaced because the declaration is the operator's complete
/// statement of what this contract depends on: a dependency they removed must
/// disappear. That is only safe because, after Issue #1147, nothing *infers*
/// declarations — the ABI scraper that used to feed this function on every
/// version publish is gone, so the only caller is an explicit operator action.
///
/// The delete and the inserts run in one transaction. Previously they did not,
/// so a mid-loop failure left the contract with a truncated dependency set and
/// no way to tell.
///
/// Returns the resolution outcome per declaration, in input order, so callers
/// can report unresolved and cross-network references instead of silently
/// recording NULLs.
pub async fn save_dependencies(
    pool: &PgPool,
    contract_id: Uuid,
    network: Network,
    decls: &[DependencyDeclaration],
) -> Result<Vec<DependencyResolution>> {
    let mut resolutions = Vec::with_capacity(decls.len());
    for decl in decls {
        resolutions.push(resolve_dependency_target(pool, &decl.name, network).await?);
    }

    for (decl, resolution) in decls.iter().zip(&resolutions) {
        if let DependencyResolution::Resolved(dep_id) = resolution {
            if detect_cycle(pool, contract_id, *dep_id)
                .await
                .unwrap_or(false)
            {
                tracing::warn!(
                    contract_id = %contract_id,
                    dependency_id = %dep_id,
                    "circular dependency declared"
                );
            }
        } else {
            tracing::info!(
                contract_id = %contract_id,
                target_ref = %decl.name,
                resolution = ?resolution,
                "dependency declaration retained but not bound to a registry contract"
            );
        }
    }

    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM contract_static_dependencies WHERE contract_id = $1")
        .bind(contract_id)
        .execute(&mut *tx)
        .await?;

    for (decl, resolution) in decls.iter().zip(&resolutions) {
        let dep_contract_id = match resolution {
            DependencyResolution::Resolved(id) => Some(*id),
            // A cross-network match is deliberately NOT bound: recording the
            // UUID here would make it indistinguishable from a real dependency.
            _ => None,
        };

        sqlx::query(
            "INSERT INTO contract_static_dependencies (contract_id, dependency_name, dependency_contract_id, version_constraint)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (contract_id, dependency_name) DO UPDATE SET
                dependency_contract_id = EXCLUDED.dependency_contract_id,
                version_constraint = EXCLUDED.version_constraint"
        )
        .bind(contract_id)
        .bind(&decl.name)
        .bind(dep_contract_id)
        .bind(&decl.version_constraint)
        .execute(&mut *tx)
        .await?;
    }

    write_canonical_edges(&mut tx, contract_id, network, decls, &resolutions).await?;

    tx.commit().await?;

    Ok(resolutions)
}

/// Row shape for comparing a declaration against the edge currently on record.
#[derive(sqlx::FromRow)]
struct CurrentEdge {
    target_ref: String,
    target_contract_id: Option<Uuid>,
    version_constraint: Option<String>,
    expected_interface_id: Option<String>,
    edge_state: String,
}

/// Mirror the declared dependency set into `contract_dependency_edges`
/// (Issue #1147), superseding rather than deleting.
///
/// **Only genuine changes are superseded.** The natural implementation --
/// supersede everything, insert everything, mirroring the DELETE-then-INSERT
/// above -- would append a full history generation on every save even when
/// nothing changed. `as_of` would then replay a wall of no-op churn and stop
/// meaning anything. So a declaration whose `(target, version_constraint,
/// expected_interface_id, state)` tuple is unchanged is left alone, keeping its
/// original `recorded_at`.
///
/// Telemetry edges are never written here: they are read live from
/// `contract_call_edge_daily_aggregates`, which is already the historical
/// record and is upserted on every contract invocation.
async fn write_canonical_edges(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    contract_id: Uuid,
    network: Network,
    decls: &[DependencyDeclaration],
    resolutions: &[DependencyResolution],
) -> Result<()> {
    let current: Vec<CurrentEdge> = sqlx::query_as(
        "SELECT target_ref, target_contract_id, version_constraint, expected_interface_id,
                edge_state::text AS edge_state
         FROM contract_dependency_edges
         WHERE source_contract_id = $1
           AND network = $2
           AND edge_source = 'declared'
           AND superseded_at IS NULL",
    )
    .bind(contract_id)
    .bind(network)
    .fetch_all(&mut **tx)
    .await?;

    let current_by_ref: HashMap<&str, &CurrentEdge> = current
        .iter()
        .map(|edge| (edge.target_ref.as_str(), edge))
        .collect();

    let mut unchanged: Vec<&str> = Vec::new();
    let mut to_write: Vec<(&DependencyDeclaration, EdgeFacts)> = Vec::new();

    for (decl, resolution) in decls.iter().zip(resolutions) {
        let facts = edge_facts(tx, resolution).await?;
        match current_by_ref.get(decl.name.as_str()) {
            Some(existing)
                if existing.target_contract_id == facts.target_contract_id
                    && existing.version_constraint.as_deref()
                        == Some(decl.version_constraint.as_str())
                    && existing.expected_interface_id == facts.expected_interface_id
                    && existing.edge_state == facts.edge_state =>
            {
                unchanged.push(decl.name.as_str());
            }
            _ => to_write.push((decl, facts)),
        }
    }

    // Supersede every current edge that is not being carried forward unchanged:
    // both the ones being rewritten and the ones the operator dropped.
    sqlx::query(
        "UPDATE contract_dependency_edges
         SET superseded_at = NOW()
         WHERE source_contract_id = $1
           AND network = $2
           AND edge_source = 'declared'
           AND superseded_at IS NULL
           AND target_ref <> ALL($3)",
    )
    .bind(contract_id)
    .bind(network)
    .bind(&unchanged)
    .execute(&mut **tx)
    .await?;

    for (decl, facts) in &to_write {
        sqlx::query(
            "INSERT INTO contract_dependency_edges
                (source_contract_id, target_contract_id, target_ref, network, edge_source,
                 edge_state, version_constraint, expected_interface_id, expected_wasm_hash)
             VALUES ($1, $2, $3, $4, 'declared', $5::dependency_edge_state, $6, $7, $8)",
        )
        .bind(contract_id)
        .bind(facts.target_contract_id)
        .bind(&decl.name)
        .bind(network)
        .bind(&facts.edge_state)
        .bind(&decl.version_constraint)
        .bind(&facts.expected_interface_id)
        .bind(&facts.expected_wasm_hash)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

/// The target-derived facts recorded on an edge at declaration time.
struct EdgeFacts {
    target_contract_id: Option<Uuid>,
    edge_state: String,
    expected_interface_id: Option<String>,
    expected_wasm_hash: Option<String>,
}

/// Snapshot the target's interface id and wasm hash as they are *now*.
///
/// These are the comparands for interface incompatibility later: the edge says
/// what the target looked like when the dependency was declared, and drift from
/// the target's current values is the diagnostic. Current values are always
/// JOINed at read time; they are never denormalized onto the edge.
async fn edge_facts(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    resolution: &DependencyResolution,
) -> Result<EdgeFacts> {
    let (target_contract_id, edge_state) = match resolution {
        DependencyResolution::Resolved(id) => (Some(*id), "resolved"),
        DependencyResolution::NetworkMismatch { .. } => (None, "network_mismatch"),
        DependencyResolution::UnknownAddress | DependencyResolution::NotAnAddress => {
            (None, "unresolved")
        }
    };

    let (expected_interface_id, expected_wasm_hash) = match target_contract_id {
        Some(id) => sqlx::query_as::<_, (Option<String>, String)>(
            "SELECT interface_id, wasm_hash FROM contracts WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .map(|(interface_id, wasm_hash)| (interface_id, Some(wasm_hash)))
        .unwrap_or((None, None)),
        None => (None, None),
    };

    Ok(EdgeFacts {
        target_contract_id,
        edge_state: edge_state.to_string(),
        expected_interface_id,
        expected_wasm_hash,
    })
}

/// Build a localized graph around a specific contract
pub async fn build_local_graph(pool: &PgPool, root_id: Uuid, depth: u32) -> Result<GraphResponse> {
    let mut neighborhood = HashSet::new();
    neighborhood.insert(root_id);

    let mut current_layer = vec![root_id];
    for _ in 0..depth {
        if current_layer.is_empty() {
            break;
        }

        let next_nodes: Vec<Uuid> = sqlx::query_scalar(
            r#"
            SELECT target FROM (
                SELECT dependency_contract_id as target FROM contract_dependencies WHERE contract_id = ANY($1) AND dependency_contract_id IS NOT NULL
                UNION
                SELECT contract_id as target FROM contract_dependencies WHERE dependency_contract_id = ANY($1)
            ) t
            "#,
        )
        .bind(&current_layer)
        .fetch_all(pool)
        .await?;

        current_layer.clear();
        for node_id in next_nodes {
            if neighborhood.insert(node_id) {
                current_layer.push(node_id);
            }
        }
    }

    let node_ids: Vec<Uuid> = neighborhood.into_iter().collect();
    if node_ids.is_empty() {
        return Ok(GraphResponse {
            nodes: vec![],
            edges: vec![],
        });
    }

    let contracts: Vec<GraphNode> = sqlx::query_as(
        "SELECT id, contract_id, name, network, is_verified, category, tags 
         FROM contracts
         WHERE id = ANY($1)",
    )
    .bind(&node_ids)
    .fetch_all(pool)
    .await?;

    let edge_rows: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT contract_id as source, dependency_contract_id as target
         FROM contract_dependencies
         WHERE dependency_contract_id IS NOT NULL
           AND contract_id = ANY($1)
           AND dependency_contract_id = ANY($1)",
    )
    .bind(&node_ids)
    .fetch_all(pool)
    .await?;

    let exact_edge_counts: HashMap<(Uuid, Uuid), i64> = {
        let rows: Vec<(Uuid, Uuid, i64)> = sqlx::query_as(
            "SELECT source_contract_id, target_contract_id, COALESCE(SUM(call_count), 0)::bigint AS total
             FROM contract_call_edge_daily_aggregates
             WHERE source_contract_id = ANY($1)
               AND target_contract_id = ANY($1)
             GROUP BY source_contract_id, target_contract_id",
        )
        .bind(&node_ids)
        .fetch_all(pool)
        .await?;
        rows.into_iter()
            .map(|(source, target, total)| ((source, target), total))
            .collect()
    };

    let source_interaction_counts: HashMap<Uuid, i64> = {
        let rows: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT contract_id, COALESCE(SUM(count), 0)::bigint AS total
             FROM contract_interaction_daily_aggregates
             WHERE contract_id = ANY($1)
               AND interaction_type = 'invoke'
             GROUP BY contract_id",
        )
        .bind(&node_ids)
        .fetch_all(pool)
        .await?;
        rows.into_iter().collect()
    };

    let mut out_degree: HashMap<Uuid, i64> = HashMap::new();
    for (source, _) in &edge_rows {
        *out_degree.entry(*source).or_insert(0) += 1;
    }

    let (component_by_node, component_sizes) = strongly_connected_components(&node_ids, &edge_rows);

    let edges: Vec<GraphEdge> = edge_rows
        .into_iter()
        .map(|(source, target)| {
            let exact_frequency = exact_edge_counts.get(&(source, target)).copied();
            let source_total = source_interaction_counts.get(&source).copied();
            let degree = out_degree.get(&source).copied().unwrap_or(0);

            let inferred_frequency = if degree > 0 {
                source_total
                    .filter(|total| *total > 0)
                    .map(|total| (total / degree).max(1))
            } else {
                None
            };

            let is_estimated = exact_frequency.is_none() && inferred_frequency.is_some();
            let call_frequency = exact_frequency.or(inferred_frequency);

            let component_source = component_by_node.get(&source).copied();
            let component_target = component_by_node.get(&target).copied();
            let is_circular = match (component_source, component_target) {
                (Some(cs), Some(ct)) if cs == ct => {
                    component_sizes.get(cs).copied().unwrap_or(0) > 1 || source == target
                }
                _ => false,
            };

            GraphEdge {
                source,
                target,
                dependency_type: "calls".to_string(),
                call_frequency,
                call_volume: call_frequency,
                is_estimated,
                is_circular,
            }
        })
        .collect();

    Ok(GraphResponse {
        nodes: contracts,
        edges,
    })
}
