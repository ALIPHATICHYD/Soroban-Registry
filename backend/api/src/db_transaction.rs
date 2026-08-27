//! Enforced transaction boundaries for multi-table write handlers (issue #1164).
//!
//! Handlers like publish, ownership transfer, and deprecation mutate several
//! tables that must move together — the contract record, its audit log, and its
//! dependency edges. When those writes run as separate autocommit statements on
//! the pool connection, a mid-request crash or a pool-level error can leave an
//! audit entry with no matching state change (or vice versa), and every
//! downstream consumer of audit/state consistency — signed snapshots, policy
//! decisions, dependency risk — silently trusts a lie.
//!
//! [`with_transaction`] makes the atomic unit the *closure*: it begins a
//! transaction, hands the live [`Transaction`] to the closure, commits on
//! `Ok`, and rolls back on `Err`. A handler that mutates state through the
//! closure cannot partially apply a multi-table change.
//!
//! # Structural enforcement
//!
//! The helper only compiles when the closure performs its writes through the
//! transaction it receives. Handlers that need more than one write must get the
//! transaction from [`with_transaction`] rather than reaching for the pool, and
//! `scripts/check-transaction-boundaries.py` enforces that in CI: a handler
//! performing more than one write outside the helper fails the check.
//!
//! # Usage
//!
//! The closure receives `&mut Transaction` and returns the value the handler
//! needs after the commit (or an error, which rolls everything back):
//!
//! ```ignore
//! let (contract, dependency_resolutions) = with_transaction(&state.db, |tx| {
//!     Box::pin(async move {
//!         let contract = sqlx::query_as::<_, Contract>(
//!             "INSERT INTO contracts (...) VALUES (...) RETURNING *",
//!         )
//!         .bind(...)
//!         .fetch_one(&mut **tx)
//!         .await?;
//!
//!         write_contract_audit_log(&mut **tx, ...).await?;
//!
//!         Ok((contract, resolutions))
//!     })
//! })
//! .await?;
//! ```
//!
//! The `Box::pin` is required because the future borrows the transaction, so it
//! cannot be a `'static` async block; the bound below ties the future's lifetime
//! to the borrow of the transaction it is handed.

use sqlx::{PgPool, Postgres, Transaction};
use std::future::Future;
use std::pin::Pin;

/// Run `f` inside a single database transaction.
///
/// * Begins a transaction on `pool`.
/// * Calls `f` with the live transaction; every state mutation the handler
///   performs must go through it.
/// * On `Ok`, commits the transaction and returns the closure's value.
/// * On `Err`, rolls the transaction back (best-effort — dropping the
///   transaction would also roll back) and returns the error unchanged.
///
/// `begin`/`commit` failures are converted to `E` via `From<sqlx::Error>`, so
/// handlers keep their existing error type (`ApiError` implements it).
pub async fn with_transaction<T, E, F>(
    pool: &PgPool,
    f: F,
) -> Result<T, E>
where
    E: From<sqlx::Error> + Send + 'static,
    F: for<'c> FnOnce(
        &'c mut Transaction<'static, Postgres>,
    ) -> Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'c>>,
{
    let mut tx = pool.begin().await.map_err(E::from)?;
    match f(&mut tx).await {
        Ok(value) => {
            tx.commit().await.map_err(E::from)?;
            Ok(value)
        }
        Err(err) => {
            // Explicit rollback: frees the connection promptly and documents the
            // intent, even though dropping the transaction would also roll back.
            let _ = tx.rollback().await;
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // End-to-end commit/rollback semantics need a live database; the test below
    // is `#[ignore]`d (matching the convention in `usage_counter.rs`) and is
    // exercised when `DATABASE_URL` points at a scratch database.
    #[tokio::test]
    #[ignore = "requires DATABASE_URL pointing at a scratch database"]
    async fn commits_on_ok_and_rolls_back_on_err() {
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set for the transaction integration test");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect(&database_url)
            .await
            .expect("connect to test database");

        let _ = sqlx::query("DROP TABLE IF EXISTS tx_boundary_test").execute(&pool).await;
        sqlx::query("CREATE TABLE tx_boundary_test (id SERIAL PRIMARY KEY, value TEXT NOT NULL)")
            .execute(&pool)
            .await
            .expect("create scratch table");

        // Commit path: both writes land.
        with_transaction(&pool, |tx| {
            Box::pin(async move {
                sqlx::query("INSERT INTO tx_boundary_test (value) VALUES ('a')")
                    .execute(&mut **tx)
                    .await?;
                sqlx::query("INSERT INTO tx_boundary_test (value) VALUES ('b')")
                    .execute(&mut **tx)
                    .await?;
                Ok::<(), sqlx::Error>(())
            })
        })
        .await
        .expect("commit path succeeds");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tx_boundary_test")
            .fetch_one(&pool)
            .await
            .expect("count rows");
        assert_eq!(count, 2, "both writes must be committed together");

        // Rollback path: the first write is undone when the closure errors.
        let err = with_transaction(&pool, |tx| {
            Box::pin(async move {
                sqlx::query("INSERT INTO tx_boundary_test (value) VALUES ('c')")
                    .execute(&mut **tx)
                    .await?;
                sqlx::query("INSERT INTO tx_boundary_test (value) VALUES ('d')")
                    .execute(&mut **tx)
                    .await?;
                Err::<(), sqlx::Error>(sqlx::Error::RowNotFound)
            })
        })
        .await;

        assert!(err.is_err(), "closure error must surface from with_transaction");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tx_boundary_test")
            .fetch_one(&pool)
            .await
            .expect("count rows");
        assert_eq!(count, 2, "no rows from the failed transaction may survive");

        let _ = sqlx::query("DROP TABLE IF EXISTS tx_boundary_test")
            .execute(&pool)
            .await;
        pool.close().await;
    }
}
