//! Point-in-time and diff queries over the contract state change log.
//!
//! Derives "state at ledger N" / "state at time T" from the existing
//! `contract_state_history` change log without requiring a separate
//! full-snapshot table. For each `state_key` we pick the most recent
//! row with `ledger_index <= N` (or `created_at <= T`) — that row's
//! `new_value` is the value as of that point.
//!
//! **Important caveat for any caller relying on these endpoints in
//! production:** at the time this module was added, NO writer in the
//! codebase populates `contract_state_history`. The table is read by
//! the state monitor's event listener and anomaly detector, but no
//! Rust code does `INSERT INTO contract_state_history`. These queries
//! will return empty results until an upstream writer (indexer hook,
//! admin ingest endpoint, or external ETL) lands. See issue tracker
//! for follow-up.
//!
//! Deliberately *not* using `sqlx::query!` macros here — those
//! require a live DATABASE_URL at compile time and break the IDE for
//! contributors without one. Runtime-checked `query_as` is the choice
//! for marketplace and indexer code in the same codebase.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// One key/value pair in a derived state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StateValue {
    pub state_key: String,
    pub value: Option<String>,
    pub value_type: Option<String>,
    pub ledger_index: Option<i64>,
    pub transaction_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// Derived snapshot of a contract's state at a specific point.
#[derive(Debug, Serialize)]
pub struct StateSnapshot {
    pub contract_id: Uuid,
    /// Ledger the snapshot was computed at. May be `None` when the
    /// caller queried by timestamp and no `ledger_index` was set on
    /// the latest matching rows.
    pub as_of_ledger: Option<i64>,
    pub as_of_time: Option<DateTime<Utc>>,
    pub entries: Vec<StateValue>,
    pub total: usize,
}

/// A diff between two derived snapshots (`from_*` → `to_*`).
#[derive(Debug, Serialize)]
pub struct StateDiff {
    pub contract_id: Uuid,
    pub from_ledger: Option<i64>,
    pub to_ledger: Option<i64>,
    pub from_time: Option<DateTime<Utc>>,
    pub to_time: Option<DateTime<Utc>>,
    pub added: Vec<DiffEntry>,
    pub removed: Vec<DiffEntry>,
    pub changed: Vec<ChangedEntry>,
}

#[derive(Debug, Serialize)]
pub struct DiffEntry {
    pub state_key: String,
    pub value: Option<String>,
    pub value_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChangedEntry {
    pub state_key: String,
    pub from_value: Option<String>,
    pub to_value: Option<String>,
    pub value_type: Option<String>,
}

/// Anchor for a point-in-time query. Exactly one variant; the caller
/// validates "exactly one of ?ledger / ?timestamp" before constructing.
#[derive(Debug, Clone, Copy)]
pub enum Anchor {
    Ledger(i64),
    Timestamp(DateTime<Utc>),
}

/// Build a derived state snapshot for `contract_id` at `anchor`.
///
/// For each `state_key` we pick the row with the largest
/// `ledger_index` (or `created_at`) ≤ the anchor, using DISTINCT ON.
/// Keys whose latest value is NULL are filtered out — they represent
/// deletions and shouldn't appear in a "current state" listing.
pub async fn snapshot_at(
    db: &PgPool,
    contract_id: Uuid,
    anchor: Anchor,
) -> Result<StateSnapshot, sqlx::Error> {
    let (entries, as_of_ledger, as_of_time) = match anchor {
        Anchor::Ledger(n) => {
            let rows: Vec<StateValue> = sqlx::query_as::<_, StateValue>(
                r#"
                SELECT DISTINCT ON (state_key)
                    state_key,
                    new_value AS value,
                    value_type,
                    ledger_index,
                    transaction_hash,
                    created_at AS updated_at
                FROM contract_state_history
                WHERE contract_id = $1
                  AND ledger_index IS NOT NULL
                  AND ledger_index <= $2
                ORDER BY state_key, ledger_index DESC, created_at DESC
                "#,
            )
            .bind(contract_id)
            .bind(n)
            .fetch_all(db)
            .await?;
            (rows, Some(n), None)
        }
        Anchor::Timestamp(t) => {
            let rows: Vec<StateValue> = sqlx::query_as::<_, StateValue>(
                r#"
                SELECT DISTINCT ON (state_key)
                    state_key,
                    new_value AS value,
                    value_type,
                    ledger_index,
                    transaction_hash,
                    created_at AS updated_at
                FROM contract_state_history
                WHERE contract_id = $1
                  AND created_at <= $2
                ORDER BY state_key, created_at DESC, ledger_index DESC
                "#,
            )
            .bind(contract_id)
            .bind(t)
            .fetch_all(db)
            .await?;
            (rows, None, Some(t))
        }
    };

    // Filter out keys whose most-recent value is NULL (deletions).
    let live: Vec<StateValue> = entries.into_iter().filter(|e| e.value.is_some()).collect();
    let total = live.len();

    Ok(StateSnapshot {
        contract_id,
        as_of_ledger,
        as_of_time,
        entries: live,
        total,
    })
}

/// Diff two derived snapshots. Caller chooses anchor type per side —
/// mixing ledger and timestamp is allowed but discouraged.
pub async fn diff(
    db: &PgPool,
    contract_id: Uuid,
    from: Anchor,
    to: Anchor,
) -> Result<StateDiff, sqlx::Error> {
    let from_snap = snapshot_at(db, contract_id, from).await?;
    let to_snap = snapshot_at(db, contract_id, to).await?;

    use std::collections::HashMap;
    let from_map: HashMap<&str, &StateValue> = from_snap
        .entries
        .iter()
        .map(|e| (e.state_key.as_str(), e))
        .collect();
    let to_map: HashMap<&str, &StateValue> = to_snap
        .entries
        .iter()
        .map(|e| (e.state_key.as_str(), e))
        .collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for (k, to_v) in &to_map {
        match from_map.get(k) {
            None => added.push(DiffEntry {
                state_key: (*k).to_string(),
                value: to_v.value.clone(),
                value_type: to_v.value_type.clone(),
            }),
            Some(from_v) if from_v.value != to_v.value => changed.push(ChangedEntry {
                state_key: (*k).to_string(),
                from_value: from_v.value.clone(),
                to_value: to_v.value.clone(),
                value_type: to_v.value_type.clone().or(from_v.value_type.clone()),
            }),
            Some(_) => { /* unchanged */ }
        }
    }
    for (k, from_v) in &from_map {
        if !to_map.contains_key(k) {
            removed.push(DiffEntry {
                state_key: (*k).to_string(),
                value: from_v.value.clone(),
                value_type: from_v.value_type.clone(),
            });
        }
    }

    // Stable ordering so clients see deterministic output.
    added.sort_by(|a, b| a.state_key.cmp(&b.state_key));
    removed.sort_by(|a, b| a.state_key.cmp(&b.state_key));
    changed.sort_by(|a, b| a.state_key.cmp(&b.state_key));

    Ok(StateDiff {
        contract_id,
        from_ledger: from_snap.as_of_ledger,
        to_ledger: to_snap.as_of_ledger,
        from_time: from_snap.as_of_time,
        to_time: to_snap.as_of_time,
        added,
        removed,
        changed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pure-logic tests for the diff combinator — no DB needed. We
    // bypass `snapshot_at` and feed synthetic snapshots into a helper
    // that mirrors the merge logic.

    fn sv(k: &str, v: Option<&str>) -> StateValue {
        StateValue {
            state_key: k.to_string(),
            value: v.map(String::from),
            value_type: None,
            ledger_index: None,
            transaction_hash: None,
            updated_at: chrono::Utc::now(),
        }
    }

    fn diff_pure(
        from: Vec<StateValue>,
        to: Vec<StateValue>,
    ) -> (Vec<String>, Vec<String>, Vec<String>) {
        use std::collections::HashMap;
        let fm: HashMap<&str, &StateValue> =
            from.iter().map(|e| (e.state_key.as_str(), e)).collect();
        let tm: HashMap<&str, &StateValue> = to.iter().map(|e| (e.state_key.as_str(), e)).collect();

        let mut added: Vec<String> = tm
            .iter()
            .filter(|(k, _)| !fm.contains_key(*k))
            .map(|(k, _)| (*k).to_string())
            .collect();
        let mut removed: Vec<String> = fm
            .iter()
            .filter(|(k, _)| !tm.contains_key(*k))
            .map(|(k, _)| (*k).to_string())
            .collect();
        let mut changed: Vec<String> = tm
            .iter()
            .filter_map(|(k, tv)| {
                fm.get(k).and_then(|fv| {
                    if fv.value != tv.value {
                        Some((*k).to_string())
                    } else {
                        None
                    }
                })
            })
            .collect();
        added.sort();
        removed.sort();
        changed.sort();
        (added, removed, changed)
    }

    #[test]
    fn diff_added_removed_changed() {
        let (a, r, c) = diff_pure(
            vec![
                sv("a", Some("1")),
                sv("b", Some("x")),
                sv("c", Some("keep")),
            ],
            vec![
                sv("a", Some("2")),
                sv("c", Some("keep")),
                sv("d", Some("new")),
            ],
        );
        assert_eq!(a, vec!["d"]);
        assert_eq!(r, vec!["b"]);
        assert_eq!(c, vec!["a"]);
    }

    #[test]
    fn diff_empty_when_identical() {
        let (a, r, c) = diff_pure(vec![sv("a", Some("1"))], vec![sv("a", Some("1"))]);
        assert!(a.is_empty() && r.is_empty() && c.is_empty());
    }

    #[test]
    fn diff_null_value_treated_as_changed() {
        // `new_value = NULL` is how the schema represents a deletion.
        // After filtering in snapshot_at(), such rows are dropped, so
        // a key that gets nulled-out shows as "removed" between the
        // two snapshots. This test documents that contract.
        let (a, r, c) = diff_pure(
            vec![sv("a", Some("1"))],
            vec![], // deletion filtered out upstream
        );
        assert!(a.is_empty() && c.is_empty());
        assert_eq!(r, vec!["a"]);
    }
}
