// Issue #889: runtime checks for the formal-verification integration layer.
//
// Lives in `tests/` so it links the compiled `api` lib without pulling in the
// crate's (pre-existing, unrelated-broken) `#[cfg(test)]` modules.

use api::formal_verification_integration::{requirement_satisfied, VerifierBackend};

#[test]
fn mandatory_policy_requires_completed_run_above_confidence() {
    assert!(requirement_satisfied(
        "mandatory",
        Some("completed"),
        Some(0.95),
        0.8
    ));
    // Below the confidence bar.
    assert!(!requirement_satisfied(
        "mandatory",
        Some("completed"),
        Some(0.5),
        0.8
    ));
    // A timeout never satisfies a mandatory policy.
    assert!(!requirement_satisfied(
        "mandatory",
        Some("timeout"),
        Some(0.99),
        0.8
    ));
    // No run at all.
    assert!(!requirement_satisfied("mandatory", None, None, 0.8));
}

#[test]
fn optional_and_disabled_policies_are_always_satisfied() {
    assert!(requirement_satisfied("optional", None, None, 0.8));
    assert!(requirement_satisfied(
        "disabled",
        Some("failed"),
        Some(0.0),
        0.8
    ));
    // Unknown requirement values default to satisfied (fail-open for non-mandatory).
    assert!(requirement_satisfied("whatever", None, None, 0.8));
}

#[test]
fn backend_selection_follows_env() {
    std::env::remove_var("FORMAL_VERIFICATION_SERVICE_URL");
    assert_eq!(VerifierBackend::from_env().name(), "builtin");

    std::env::set_var(
        "FORMAL_VERIFICATION_SERVICE_URL",
        "https://verifier.example/run",
    );
    assert_eq!(VerifierBackend::from_env().name(), "external");

    // Blank/whitespace falls back to built-in.
    std::env::set_var("FORMAL_VERIFICATION_SERVICE_URL", "   ");
    assert_eq!(VerifierBackend::from_env().name(), "builtin");
    std::env::remove_var("FORMAL_VERIFICATION_SERVICE_URL");
}

#[test]
fn formal_verification_router_builds_without_path_conflicts() {
    // Building the router exercises matchit insertion; a static/param conflict
    // between the new `/run`, `/runs`, `/summary` routes and the existing
    // `/:session_id` route would panic here.
    let _router = api::routes::formal_verification_routes();
}
