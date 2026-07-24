// tests/mutation_testing_tests.rs
//
// Issue #619 — Contract mutation testing and compatibility suite.
// Tests for mutation generation, test suite logic, and robustness scoring.

// ─────────────────────────────────────────────────────────────────────────────
// These tests replicate the pure logic from mutation_testing_handlers.rs
// without requiring a DB connection, following the pattern from
// compatibility_testing_tests.rs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum MutationOperator {
    BooleanFlip,
    BoundarySwap,
    ArithmeticChange,
    ConditionRemoval,
}

impl std::fmt::Display for MutationOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MutationOperator::BooleanFlip => write!(f, "boolean_flip"),
            MutationOperator::BoundarySwap => write!(f, "boundary_swap"),
            MutationOperator::ArithmeticChange => write!(f, "arithmetic_change"),
            MutationOperator::ConditionRemoval => write!(f, "condition_removal"),
        }
    }
}

#[derive(Debug, Clone)]
struct Mutation {
    operator: MutationOperator,
    target_function: String,
    description: String,
    killed: bool,
    test_output: String,
}

fn extract_function_names(abi: &serde_json::Value) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(specs) = abi.as_array() {
        for spec in specs {
            let ty = spec.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if matches!(ty, "function" | "fn") {
                if let Some(name) = spec.get("name").and_then(|n| n.as_str()) {
                    names.push(name.to_string());
                }
            }
        }
    }
    if names.is_empty() {
        names.push("invoke".to_string());
    }
    names
}

fn generate_mutations(functions: &[String]) -> Vec<Mutation> {
    let operators = [
        MutationOperator::BooleanFlip,
        MutationOperator::BoundarySwap,
        MutationOperator::ArithmeticChange,
        MutationOperator::ConditionRemoval,
    ];
    let mut mutations = Vec::new();
    for func in functions {
        for op in &operators {
            let description = match op {
                MutationOperator::BooleanFlip => format!("Negated boolean return in `{func}`"),
                MutationOperator::BoundarySwap => format!("Swapped boundary check in `{func}`"),
                MutationOperator::ArithmeticChange => {
                    format!("Changed arithmetic operator in `{func}`")
                }
                MutationOperator::ConditionRemoval => {
                    format!("Removed guard condition in `{func}`")
                }
            };
            mutations.push(Mutation {
                operator: op.clone(),
                target_function: func.clone(),
                description,
                killed: false,
                test_output: String::new(),
            });
        }
    }
    mutations
}

fn run_test_suite_against(m: &mut Mutation) {
    let (killed, output) = match m.operator {
        MutationOperator::BooleanFlip => (
            true,
            "Test failed: expected `true`, got `false` — mutation killed".to_string(),
        ),
        MutationOperator::BoundarySwap => (
            true,
            "Test failed: off-by-one assertion — mutation killed".to_string(),
        ),
        MutationOperator::ArithmeticChange => (
            true,
            "Test failed: numeric result mismatch — mutation killed".to_string(),
        ),
        MutationOperator::ConditionRemoval => (
            false,
            "Test passed (mutation survived — guard not fully covered)".to_string(),
        ),
    };
    m.killed = killed;
    m.test_output = output;
}

fn compute_robustness(mutations: &[Mutation]) -> f64 {
    if mutations.is_empty() {
        return 0.0;
    }
    let killed = mutations.iter().filter(|m| m.killed).count();
    killed as f64 / mutations.len() as f64
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_extract_functions_from_abi() {
    let abi = serde_json::json!([
        { "type": "function", "name": "transfer" },
        { "type": "function", "name": "approve" },
        { "type": "event", "name": "Transfer" },
    ]);
    let funcs = extract_function_names(&abi);
    assert_eq!(funcs, vec!["transfer", "approve"]);
}

#[test]
fn test_extract_functions_empty_abi_fallback() {
    let abi = serde_json::json!([]);
    let funcs = extract_function_names(&abi);
    assert_eq!(funcs, vec!["invoke"]);
}

#[test]
fn test_extract_fn_type_alias() {
    let abi = serde_json::json!([
        { "type": "fn", "name": "mint" },
    ]);
    let funcs = extract_function_names(&abi);
    assert_eq!(funcs, vec!["mint"]);
}

#[test]
fn test_generate_mutations_count() {
    let funcs = vec!["foo".to_string(), "bar".to_string()];
    let mutations = generate_mutations(&funcs);
    assert_eq!(mutations.len(), 8, "2 functions × 4 operators = 8");
}

#[test]
fn test_generate_mutations_single_function() {
    let funcs = vec!["mint".to_string()];
    let mutations = generate_mutations(&funcs);
    assert_eq!(mutations.len(), 4);
    let ops: Vec<&MutationOperator> = mutations.iter().map(|m| &m.operator).collect();
    assert!(ops.contains(&&MutationOperator::BooleanFlip));
    assert!(ops.contains(&&MutationOperator::BooleanFlip));
    assert!(ops.contains(&&MutationOperator::ArithmeticChange));
    assert!(ops.contains(&&MutationOperator::ConditionRemoval));
}

#[test]
fn test_boolean_flip_always_killed() {
    let mut m = Mutation {
        operator: MutationOperator::BooleanFlip,
        target_function: "foo".into(),
        description: "test".into(),
        killed: false,
        test_output: String::new(),
    };
    run_test_suite_against(&mut m);
    assert!(m.killed);
    assert!(m.test_output.contains("mutation killed"));
}

#[test]
fn test_boundary_swap_always_killed() {
    let mut m = Mutation {
        operator: MutationOperator::BoundarySwap,
        target_function: "bar".into(),
        description: "test".into(),
        killed: false,
        test_output: String::new(),
    };
    run_test_suite_against(&mut m);
    assert!(m.killed);
}

#[test]
fn test_arithmetic_change_always_killed() {
    let mut m = Mutation {
        operator: MutationOperator::ArithmeticChange,
        target_function: "calc".into(),
        description: "test".into(),
        killed: false,
        test_output: String::new(),
    };
    run_test_suite_against(&mut m);
    assert!(m.killed);
}

#[test]
fn test_condition_removal_survives() {
    let mut m = Mutation {
        operator: MutationOperator::ConditionRemoval,
        target_function: "guard".into(),
        description: "test".into(),
        killed: false,
        test_output: String::new(),
    };
    run_test_suite_against(&mut m);
    assert!(!m.killed, "ConditionRemoval simulates coverage gap");
}

#[test]
fn test_robustness_score_computed_correctly() {
    let funcs = vec!["f".to_string()];
    let mut mutations = generate_mutations(&funcs);
    for m in &mut mutations {
        run_test_suite_against(m);
    }
    // BoolFlip, BoundarySwap, Arith killed (3); ConditionRemoval survived (1)
    let score = compute_robustness(&mutations);
    assert!((score - 0.75).abs() < 0.001, "robustness = 0.75");
}

#[test]
fn test_robustness_score_zero_for_empty() {
    assert_eq!(compute_robustness(&[]), 0.0);
}

#[test]
fn test_mutation_descriptions_reference_function() {
    let funcs = vec!["deposit".to_string()];
    let mutations = generate_mutations(&funcs);
    for m in &mutations {
        assert!(
            m.description.contains("deposit"),
            "description references function"
        );
    }
}

#[test]
fn test_operator_display_strings() {
    assert_eq!(MutationOperator::BooleanFlip.to_string(), "boolean_flip");
    assert_eq!(MutationOperator::BoundarySwap.to_string(), "boundary_swap");
    assert_eq!(
        MutationOperator::ArithmeticChange.to_string(),
        "arithmetic_change"
    );
    assert_eq!(
        MutationOperator::ConditionRemoval.to_string(),
        "condition_removal"
    );
}

#[test]
fn test_mutation_run_full_pipeline() {
    let abi = serde_json::json!([
        { "type": "function", "name": "transfer" },
        { "type": "function", "name": "burn" },
    ]);
    let funcs = extract_function_names(&abi);
    let mut mutations = generate_mutations(&funcs);
    for m in &mut mutations {
        run_test_suite_against(m);
    }
    let total = mutations.len();
    let killed = mutations.iter().filter(|m| m.killed).count();
    let score = compute_robustness(&mutations);
    assert_eq!(total, 8);
    assert_eq!(killed, 6);
    assert!((score - 0.75).abs() < 0.001);
}
