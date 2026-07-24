//! Contract dependency resolution with topological sort and cycle detection.
//!
//! Given a map of contract IDs to their direct dependencies, [`resolve`] returns
//! a build order (topological sort) in which every dependency appears before the
//! contract that requires it.
//!
//! If the graph contains a cycle, [`DependencyError::CircularDependency`] is
//! returned with the names of the contracts involved.
//!
//! Missing dependencies (a contract references an ID not present in the graph)
//! are reported as [`DependencyError::MissingDependency`] rather than panicking.

use std::collections::{HashMap, HashSet, VecDeque};

/// Errors produced by the dependency resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyError {
    /// A contract depends on an ID that is not present in the graph.
    MissingDependency { contract: String, missing: String },
    /// The dependency graph contains a cycle.
    CircularDependency {
        /// Contracts involved in the cycle (in detection order).
        cycle: Vec<String>,
    },
}

impl std::fmt::Display for DependencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDependency { contract, missing } => {
                write!(
                    f,
                    "contract '{contract}' depends on '{missing}' which is not in the graph"
                )
            }
            Self::CircularDependency { cycle } => {
                write!(f, "circular dependency detected: {}", cycle.join(" -> "))
            }
        }
    }
}

/// Resolve a dependency graph into a valid build order.
///
/// # Parameters
/// - `graph`: map of `contract_id -> [dependency_id, ...]`
///
/// # Returns
/// A `Vec<String>` of contract IDs in topological order (dependencies first),
/// or a [`DependencyError`] if the graph is invalid.
///
/// # Algorithm
/// Kahn's algorithm (BFS-based topological sort):
/// 1. Compute in-degree for every node.
/// 2. Enqueue all nodes with in-degree 0.
/// 3. Repeatedly dequeue a node, add it to the result, and decrement the
///    in-degree of its dependents.
/// 4. If the result length < graph length, a cycle exists.
pub fn resolve(graph: &HashMap<String, Vec<String>>) -> Result<Vec<String>, DependencyError> {
    // Validate: every referenced dependency must exist in the graph.
    for (contract, deps) in graph {
        for dep in deps {
            if !graph.contains_key(dep) {
                return Err(DependencyError::MissingDependency {
                    contract: contract.clone(),
                    missing: dep.clone(),
                });
            }
        }
    }

    // Build in-degree map.
    let mut in_degree: HashMap<&str, usize> = graph.keys().map(|k| (k.as_str(), 0)).collect();
    for deps in graph.values() {
        for dep in deps {
            *in_degree.entry(dep.as_str()).or_insert(0) += 1;
        }
    }

    // Enqueue zero-in-degree nodes (sorted for deterministic output).
    let mut queue: VecDeque<&str> = {
        let mut v: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(&k, _)| k)
            .collect();
        v.sort_unstable();
        VecDeque::from(v)
    };

    let mut order: Vec<String> = Vec::with_capacity(graph.len());

    while let Some(node) = queue.pop_front() {
        order.push(node.to_string());
        // Find all contracts that list `node` as a dependency.
        let mut dependents: Vec<&str> = graph
            .iter()
            .filter(|(_, deps)| deps.iter().any(|d| d == node))
            .map(|(k, _)| k.as_str())
            .collect();
        dependents.sort_unstable();
        for dependent in dependents {
            let deg = in_degree.entry(dependent).or_insert(0);
            *deg = deg.saturating_sub(1);
            if *deg == 0 {
                queue.push_back(dependent);
            }
        }
    }

    if order.len() != graph.len() {
        // Cycle: collect nodes not yet emitted.
        let emitted: HashSet<&str> = order.iter().map(|s| s.as_str()).collect();
        let mut cycle: Vec<String> = graph
            .keys()
            .filter(|k| !emitted.contains(k.as_str()))
            .cloned()
            .collect();
        cycle.sort();
        return Err(DependencyError::CircularDependency { cycle });
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(k, vs)| (k.to_string(), vs.iter().map(|v| v.to_string()).collect()))
            .collect()
    }

    #[test]
    fn empty_graph_returns_empty_order() {
        let result = resolve(&HashMap::new()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn single_node_no_deps() {
        let g = graph(&[("a", &[])]);
        assert_eq!(resolve(&g).unwrap(), vec!["a"]);
    }

    #[test]
    fn linear_chain_resolves_in_order() {
        // c -> b -> a  (a has no deps, b depends on a, c depends on b)
        let g = graph(&[("a", &[]), ("b", &["a"]), ("c", &["b"])]);
        let order = resolve(&g).unwrap();
        assert!(order.iter().position(|x| x == "a") < order.iter().position(|x| x == "b"));
        assert!(order.iter().position(|x| x == "b") < order.iter().position(|x| x == "c"));
    }

    #[test]
    fn diamond_dependency_resolves() {
        // d depends on b and c; b and c both depend on a
        let g = graph(&[("a", &[]), ("b", &["a"]), ("c", &["a"]), ("d", &["b", "c"])]);
        let order = resolve(&g).unwrap();
        let pos = |x: &str| order.iter().position(|s| s == x).unwrap();
        assert!(pos("a") < pos("b"));
        assert!(pos("a") < pos("c"));
        assert!(pos("b") < pos("d"));
        assert!(pos("c") < pos("d"));
    }

    #[test]
    fn circular_dependency_detected() {
        // a -> b -> c -> a
        let g = graph(&[("a", &["b"]), ("b", &["c"]), ("c", &["a"])]);
        match resolve(&g) {
            Err(DependencyError::CircularDependency { cycle }) => {
                assert!(!cycle.is_empty());
                // All cycle members should be a, b, or c
                for node in &cycle {
                    assert!(["a", "b", "c"].contains(&node.as_str()));
                }
            }
            other => panic!("expected CircularDependency, got {:?}", other),
        }
    }

    #[test]
    fn self_loop_detected_as_cycle() {
        let g = graph(&[("a", &["a"])]);
        assert!(matches!(
            resolve(&g),
            Err(DependencyError::CircularDependency { .. })
        ));
    }

    #[test]
    fn missing_dependency_reported() {
        let g = graph(&[("a", &["nonexistent"])]);
        match resolve(&g) {
            Err(DependencyError::MissingDependency { contract, missing }) => {
                assert_eq!(contract, "a");
                assert_eq!(missing, "nonexistent");
            }
            other => panic!("expected MissingDependency, got {:?}", other),
        }
    }

    #[test]
    fn multiple_roots_all_resolved() {
        // Two independent trees
        let g = graph(&[("a", &[]), ("b", &[]), ("c", &["a"]), ("d", &["b"])]);
        let order = resolve(&g).unwrap();
        assert_eq!(order.len(), 4);
        let pos = |x: &str| order.iter().position(|s| s == x).unwrap();
        assert!(pos("a") < pos("c"));
        assert!(pos("b") < pos("d"));
    }

    #[test]
    fn partial_cycle_with_valid_nodes() {
        // e and f are valid; a->b->a is a cycle
        let g = graph(&[("a", &["b"]), ("b", &["a"]), ("e", &[]), ("f", &["e"])]);
        match resolve(&g) {
            Err(DependencyError::CircularDependency { cycle }) => {
                assert!(cycle.contains(&"a".to_string()) || cycle.contains(&"b".to_string()));
            }
            other => panic!("expected CircularDependency, got {:?}", other),
        }
    }
}
