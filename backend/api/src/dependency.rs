use anyhow::Result;
use shared::{DependencyDeclaration, GraphEdge, GraphNode, GraphResponse};
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

/// Detect dependencies from a contract ABI JSON
pub fn detect_dependencies_from_abi(abi_json: &serde_json::Value) -> Vec<DependencyDeclaration> {
    let mut dependencies = Vec::new();
    let mut seen = HashSet::new();

    // In Soroban, external contract calls often use a 'Client' or are defined in the spec.
    // We look for 'custom' types that might represent other contracts,
    // or specific annotations if they exist.
    // For now, we'll scan for common patterns.

    if let Some(specs) = abi_json.as_array() {
        for spec in specs {
            // Look for interface/contract client definitions
            if let Some(spec_type) = spec.get("type").and_then(|t| t.as_str()) {
                if spec_type == "contract_spec_interface" || spec_type == "interface" {
                    if let Some(name) = spec.get("name").and_then(|n| n.as_str()) {
                        if seen.insert(name.to_string()) {
                            dependencies.push(DependencyDeclaration {
                                name: name.to_string(),
                                version_constraint: "*".to_string(), // Default constraint
                            });
                        }
                    }
                }
            }

            // Scan function inputs/outputs for custom types that might be contract IDs?
            // Actually, usually they are addressed by Symbol or Address.
            // But sometimes the 'type' field in the spec itself points to another contract.
        }
    }

    dependencies
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

/// Resolve a dependency name/id to a contract UUID if it exists in the registry
pub async fn resolve_contract_id(pool: &PgPool, identifier: &str) -> Result<Option<Uuid>> {
    // Try UUID first
    if let Ok(id) = Uuid::parse_str(identifier) {
        return Ok(Some(id));
    }

    // Try contract_id (public key)
    let id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM contracts WHERE contract_id = $1")
        .bind(identifier)
        .fetch_optional(pool)
        .await?;

    if id.is_some() {
        return Ok(id);
    }

    // Try name
    let id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM contracts WHERE name = $1")
        .bind(identifier)
        .fetch_optional(pool)
        .await?;

    Ok(id)
}

/// Save dependencies for a contract, resolving them if possible
pub async fn save_dependencies(
    pool: &PgPool,
    contract_id: Uuid,
    decls: &[DependencyDeclaration],
) -> Result<()> {
    // Clear existing dependencies (optional, depends on if we want to merge or replace)
    sqlx::query("DELETE FROM contract_static_dependencies WHERE contract_id = $1")
        .bind(contract_id)
        .execute(pool)
        .await?;

    for decl in decls {
        let dep_contract_id = resolve_contract_id(pool, &decl.name).await?;

        if let Some(dep_id) = dep_contract_id {
            if detect_cycle(pool, contract_id, dep_id)
                .await
                .unwrap_or(false)
            {
                tracing::warn!(
                    "Circular dependency detected: contract {} -> {}",
                    contract_id,
                    dep_id
                );
            }
        }

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
        .execute(pool)
        .await?;
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_dependencies() {
        let abi = json!([
            {
                "type": "interface",
                "name": "TokenInterface"
            },
            {
                "type": "function",
                "name": "hello"
            }
        ]);

        let deps = detect_dependencies_from_abi(&abi);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "TokenInterface");
    }

    #[test]
    fn test_detect_dependencies_duplicate() {
        let abi = json!([
            { "type": "interface", "name": "Auth" },
            { "type": "interface", "name": "Auth" }
        ]);

        let deps = detect_dependencies_from_abi(&abi);
        assert_eq!(deps.len(), 1);
    }
}
