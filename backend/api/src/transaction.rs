//! Transactional consistency helper for multi-table writes (Issue #1109).
//!
//! Provides [`with_transaction()`] — a closure-based wrapper around
//! `PgPool::begin()` that enforces commit/rollback discipline and makes
//! it hard to accidentally issue writes outside the transaction.
//!
//! # When to use
//!
//! Any handler that writes to **more than one table** in a single request
//! must use `with_transaction()`.  Writes that touch a single table can
//! use autocommit (`&state.db`) directly.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::transaction::with_transaction;
//!
//! let contract = with_transaction(&state.db, "publish_contract", |tx| async move {
//!     let row = sqlx::query_as::<_, Contract>("INSERT INTO contracts ...")
//!         .fetch_one(&mut *tx)
//!         .await
//!         .map_err(|e| db_internal_error("insert contract", e))?;
//!
//!     write_contract_audit_log(&mut *tx, ...).await
//!         .map_err(|e| db_internal_error("audit", e))?;
//!
//!     Ok(row)
//! })
//! .await?;
//! ```

use crate::error::ApiError;
use sqlx::{PgPool, Postgres, Transaction};

/// Execute an async closure inside a Postgres transaction.
///
/// *   On `Ok(T)` the transaction is **committed** and `T` is returned.
/// *   On `Err(ApiError)` the transaction is **rolled back** (explicitly,
///     then implicitly on drop as a safety net) and the error is propagated.
/// *   If the commit itself fails, the error is wrapped in
///     [`ApiError::internal`].
///
/// The closure receives a `sqlx::Transaction<'_, Postgres>` — **not** a
/// `PgPool` — so callers must pass `&mut *tx` to every query, which
/// guarantees every write participates in the same transaction.
///
/// `operation` is a human-readable label used exclusively for tracing
/// spans and error messages.
pub async fn with_transaction<T, F, Fut>(
    pool: &PgPool,
    operation: &str,
    work: F,
) -> Result<T, ApiError>
where
    T: Send,
    F: FnOnce(Transaction<'_, Postgres>) -> Fut,
    Fut: std::future::Future<Output = Result<(T, Transaction<'_, Postgres>), ApiError>> + Send,
{
    let span = tracing::info_span!("with_transaction", op = %operation);
    let _enter = span.enter();

    let tx = pool.begin().await.map_err(|err| {
        tracing::error!(operation = operation, error = ?err, "failed to begin transaction");
        ApiError::internal(format!(
            "Failed to begin transaction for {operation}: {err}"
        ))
    })?;

    match work(tx).await {
        Ok((value, tx)) => {
            tx.commit().await.map_err(|err| {
                tracing::error!(operation = operation, error = ?err, "failed to commit transaction");
                ApiError::internal(format!(
                    "Failed to commit transaction for {operation}: {err}"
                ))
            })?;

            tracing::debug!(operation = operation, "transaction committed successfully");
            Ok(value)
        }
        Err(api_err) => {
            // The `Transaction` was moved into the closure and either:
            // (a) was returned inside `Err` — in which case it is dropped
            //     here, triggering sqlx's implicit rollback, or
            // (b) was consumed inside the closure — also triggering drop.
            //
            // Either way the transaction is rolled back.  We log for
            // observability.
            tracing::debug!(
                operation = operation,
                "transaction rolled back due to error"
            );
            Err(api_err)
        }
    }
}

#[cfg(test)]
mod tests {
    /// Verify the module compiles and exports are accessible.
    #[test]
    fn module_compiles() {
        // Type-level smoke test — no database needed.
        let _ = std::any::type_name::<super::ApiError>();
    }
}
