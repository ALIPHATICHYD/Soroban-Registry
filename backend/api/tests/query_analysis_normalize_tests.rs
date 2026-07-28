// Issue #887: runtime verification of query normalization.
//
// Lives in `tests/` so it links the compiled `api` lib and does NOT pull in the
// crate's (pre-existing, unrelated-broken) `#[cfg(test)]` modules. Focused on the
// public `normalize_sql`, which is the security-critical surface: it must collapse
// every literal/parameter so no bound values (secrets) can ever be persisted.

use api::query_analysis::normalize_sql;

#[test]
fn literals_and_params_are_collapsed() {
    let a = normalize_sql("SELECT * FROM contracts WHERE id = '123' AND n = 5");
    let b = normalize_sql("SELECT * FROM contracts WHERE id = '999' AND n = 42");
    assert_eq!(a, b, "different literal values must normalize identically");
    assert!(a.contains("id = ?"));
    assert!(!a.contains("123") && !a.contains("999"));
}

#[test]
fn secret_string_literals_are_never_retained() {
    let sql = "SELECT * FROM users WHERE api_key = 'sk_live_supersecret_value'";
    let n = normalize_sql(sql);
    assert!(!n.contains("supersecret"), "secret literal leaked: {n}");
    assert!(n.contains("api_key = ?"));
}

#[test]
fn dollar_params_and_in_lists_collapse() {
    let n = normalize_sql("SELECT * FROM t WHERE id = $1 AND x IN (1, 2, 3, 4)");
    assert_eq!(n, "SELECT * FROM t WHERE id = ? AND x IN (?)");
}

#[test]
fn whitespace_is_normalized() {
    let a = normalize_sql("SELECT   *\n  FROM    t\tWHERE  id = $1");
    let b = normalize_sql("SELECT * FROM t WHERE id = $2");
    assert_eq!(a, b);
}

#[test]
fn distinct_shapes_stay_distinct() {
    let a = normalize_sql("SELECT a FROM t WHERE id = $1");
    let b = normalize_sql("SELECT a, b FROM t WHERE id = $1");
    assert_ne!(a, b);
}
