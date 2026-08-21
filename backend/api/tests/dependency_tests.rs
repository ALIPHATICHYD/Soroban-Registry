// tests/dependency_tests.rs
//
// Issue #610 — Contract dependency tracking and resolution.
// Issue #1147 — rewritten to test the shipped code.
//
// This file used to assert against helper functions defined in the file itself:
// an in-memory `has_cycle_in_memory` that reimplemented cycle detection, a
// `get_transitive_dependencies_in_memory` whose entire body was `vec![]`, and
// checks like "a version constraint string is not empty". None of it exercised
// any code that ships, so it passed no matter what the real implementation did
// -- the worst kind of test, because it reads as coverage.
//
// The behaviour is now tested where it actually lives:
//
// - Cycle, ordering, dedup, severity and version-conflict rules are pure and
//   live in `shared::dependency_graph`; `cargo test -p shared` covers them with
//   no database.
// - The traversal itself is a recursive CTE, so it is only meaningfully tested
//   against Postgres: see `api/tests/dependency_graph_tests.rs`.
//
// What remains here is the part that belongs at this level: the invariants
// Issue #610 cared about, asserted against the real shared functions rather
// than against a copy of them.

use shared::dependency_graph::{cycle_segment, EdgeState};
use uuid::Uuid;

fn node(n: u8) -> Uuid {
    Uuid::from_bytes([n; 16])
}

#[test]
fn a_direct_cycle_is_detected() {
    // A -> B, and B points back at A.
    let path = [node(1), node(2)];
    let segment = cycle_segment(&path, node(1)).expect("A -> B -> A is a cycle");
    assert_eq!(segment, vec![node(1), node(2), node(1)]);
}

#[test]
fn a_transitive_cycle_is_detected() {
    // A -> B -> C, and C points back at A.
    let path = [node(1), node(2), node(3)];
    let segment = cycle_segment(&path, node(1)).expect("A -> B -> C -> A is a cycle");
    assert_eq!(segment, vec![node(1), node(2), node(3), node(1)]);
}

#[test]
fn a_self_dependency_is_a_cycle() {
    let segment = cycle_segment(&[node(1)], node(1)).expect("A -> A is a cycle");
    assert_eq!(segment, vec![node(1), node(1)]);
}

#[test]
fn a_diamond_is_not_a_cycle() {
    // A -> B -> D and A -> C -> D share D but close no loop. Reporting a
    // diamond as circular was the practical failure mode of the old
    // visited-set walk, which removed nodes on unwind.
    assert_eq!(cycle_segment(&[node(1), node(2)], node(4)), None);
    assert_eq!(cycle_segment(&[node(1), node(3)], node(4)), None);
}

#[test]
fn the_cycle_segment_excludes_the_lead_in() {
    // Entering a loop is not the same as being in it: a diagnostic that named
    // the whole path would point at contracts that are not part of the cycle.
    let path = [node(9), node(1), node(2)];
    let segment = cycle_segment(&path, node(1)).expect("cycle");
    assert!(
        !segment.contains(&node(9)),
        "the lead-in node is not part of the loop: {segment:?}"
    );
}

#[test]
fn only_resolved_edges_are_followed() {
    // Issue #1147 D3: a dependency reference that names nothing must be kept
    // and reported, but never treated as a dependency for traversal or risk.
    assert!(EdgeState::Resolved.is_traversable());
    assert!(!EdgeState::Unresolved.is_traversable());
    assert!(!EdgeState::NetworkMismatch.is_traversable());
}
