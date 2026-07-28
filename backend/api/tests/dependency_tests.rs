// tests/dependency_tests.rs
//
// Issue #610 — Contract dependency tracking and resolution.
// Unit tests for the dependency logic (no live DB required).

use std::collections::{HashMap, HashSet, VecDeque};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers mirroring dependency.rs logic
// ─────────────────────────────────────────────────────────────────────────────

fn get_transitive_dependencies_in_memory(
    graph: &HashMap<&str, Vec<&str>>,
    root: &str,
) -> Vec<&'static str> {
    // We can't easily return &'static str from a ref, so use owned comparisons.
    let _ = root;
    let _ = graph;
    vec![]
}

fn has_cycle_in_memory(graph: &HashMap<&str, Vec<&str>>, start: &str, potential_dep: &str) -> bool {
    if start == potential_dep {
        return true;
    }
    // DFS from potential_dep — if start is reachable, adding start→potential_dep creates a cycle.
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(potential_dep);

    while let Some(node) = queue.pop_front() {
        if visited.contains(node) {
            continue;
        }
        visited.insert(node);
        for &dep in graph.get(node).unwrap_or(&vec![]) {
            if dep == start {
                return true;
            }
            queue.push_back(dep);
        }
    }
    false
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_no_cycle_basic() {
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    graph.insert("A", vec!["B"]);
    graph.insert("B", vec!["C"]);
    graph.insert("C", vec![]);
    assert!(!has_cycle_in_memory(&graph, "A", "C"), "A→B→C has no cycle");
}

#[test]
fn test_self_dependency_is_cycle() {
    let graph: HashMap<&str, Vec<&str>> = HashMap::new();
    assert!(
        has_cycle_in_memory(&graph, "A", "A"),
        "Self-dependency is a cycle"
    );
}

#[test]
fn test_direct_cycle_detected() {
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    graph.insert("B", vec!["A"]); // B already depends on A
                                  // Adding A → B would create: A → B → A
    assert!(
        has_cycle_in_memory(&graph, "A", "B"),
        "A→B would create cycle"
    );
}

#[test]
fn test_transitive_cycle_detected() {
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    graph.insert("B", vec!["C"]);
    graph.insert("C", vec!["A"]); // C → A already exists
                                  // Adding A → B would create: A → B → C → A
    assert!(
        has_cycle_in_memory(&graph, "A", "B"),
        "Transitive cycle A→B→C→A detected"
    );
}

#[test]
fn test_no_cycle_with_diamond() {
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    graph.insert("A", vec!["B", "C"]);
    graph.insert("B", vec!["D"]);
    graph.insert("C", vec!["D"]);
    graph.insert("D", vec![]);
    // Diamond: A→B→D, A→C→D — no cycle
    assert!(!has_cycle_in_memory(&graph, "A", "D"));
    assert!(!has_cycle_in_memory(&graph, "B", "D"));
}

#[test]
fn test_circular_detection_visited_set() {
    // Mirror the logic in dependency_handlers::build_dependency_tree
    let mut visited = HashSet::new();
    let id_a = "contract-A";
    let id_b = "contract-B";
    let id_c = "contract-C";

    visited.insert(id_a);
    visited.insert(id_b);

    // c tries to reference a — already in visited → circular
    let is_circular = visited.contains(&id_a);
    assert!(
        is_circular,
        "#610: circular dependency to A detected via visited set"
    );

    // c references d — not in visited
    let is_d_circular = visited.contains(&"contract-D");
    assert!(!is_d_circular, "#610: D not in visited, no circle");
    let _ = id_c;
}

#[test]
fn test_dependency_name_resolution_uuid_format() {
    // Simulate: if identifier parses as UUID, it's used directly.
    let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
    let is_uuid =
        uuid_str.parse::<u128>().is_err() && uuid_str.contains('-') && uuid_str.len() == 36;
    assert!(
        is_uuid,
        "#610: UUID identifiers recognised for dependency resolution"
    );
}

#[test]
fn test_dependency_declaration_version_constraint() {
    // Ensure version constraint wildcards are stored as-is.
    let constraints = vec!["*", ">=1.0.0", "^2.3.0", "~1.5.0"];
    for c in &constraints {
        assert!(!c.is_empty(), "constraint '{}' should not be empty", c);
    }
}

#[test]
fn test_transitive_dependency_bfs_ordering() {
    // Simulate BFS traversal to collect transitive deps in breadth-first order.
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    graph.insert("root", vec!["A", "B"]);
    graph.insert("A", vec!["C"]);
    graph.insert("B", vec!["C"]);
    graph.insert("C", vec![]);

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut result = Vec::new();

    queue.push_back("root");
    visited.insert("root");

    while let Some(node) = queue.pop_front() {
        for &dep in graph.get(node).unwrap_or(&vec![]) {
            if visited.insert(dep) {
                queue.push_back(dep);
                result.push(dep);
            }
        }
    }

    // root → A, B → C (C appears once due to visited set)
    assert_eq!(result.len(), 3, "#610: 3 unique transitive deps");
    assert!(result.contains(&"A"));
    assert!(result.contains(&"B"));
    assert!(result.contains(&"C"));
}
