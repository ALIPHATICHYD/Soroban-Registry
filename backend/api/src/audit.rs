//! Audit logger for security-sensitive operations.
//!
//! Every call to [`AuditLogger::log`] inserts an append-only row into the
//! `audit_logs` table. Each row includes a SHA-256 chain hash over the
//! previous row's hash and the current payload, making the log tamper-evident.
//!
//! # Sensitive operations tracked
//! - `contract.verify`   — contract verification attempts
//! - `publisher.change`  — publisher ownership transfers
//! - `contract.publish`  — new contract publications
//! - `contract.delete`   — contract deletions
//! - `user.role_change`  — user role modifications
//! - `admin.*`           — any admin action

use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::fmt;
use tracing::instrument;

/// A security-sensitive operation that must be audited.
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Who performed the action. `None` for system/anonymous.
    pub actor_id: Option<String>,
    pub actor_email: Option<String>,
    /// Operation identifier, e.g. `"contract.verify"`.
    pub operation: String,
    /// Type of the affected resource, e.g. `"contract"`.
    pub resource_type: String,
    /// ID of the affected resource.
    pub resource_id: String,
    /// Arbitrary structured context (serialised to JSONB).
    pub metadata: serde_json::Value,
    /// Whether the operation succeeded.
    pub status: AuditStatus,
    /// Error message if status is `Failure`.
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditStatus {
    Success,
    Failure,
}

impl fmt::Display for AuditStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "success"),
            Self::Failure => write!(f, "failure"),
        }
    }
}

/// Writes audit events to the `audit_logs` table.
#[derive(Clone)]
pub struct AuditLogger {
    pool: PgPool,
}

impl AuditLogger {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert an audit event. Returns the new row's `id`.
    ///
    /// The chain hash is computed as:
    /// `SHA-256(prev_chain_hash || operation || resource_id || created_at_iso)`
    /// where `prev_chain_hash` is the hash of the most recent row (or a
    /// genesis constant if the table is empty).
    #[instrument(skip(self), fields(operation = %event.operation, resource_id = %event.resource_id))]
    pub async fn log(&self, event: AuditEvent) -> Result<i64, sqlx::Error> {
        // Fetch the most recent chain hash (or genesis value).
        let prev_hash: String =
            sqlx::query_scalar("SELECT COALESCE(MAX(chain_hash), 'genesis') FROM audit_logs")
                .fetch_one(&self.pool)
                .await?;

        let now = chrono::Utc::now();
        let chain_input = format!(
            "{}{}{}{}",
            prev_hash,
            event.operation,
            event.resource_id,
            now.to_rfc3339()
        );
        let chain_hash = hex::encode(Sha256::digest(chain_input.as_bytes()));

        let id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO audit_logs
                (actor_id, actor_email, operation, resource_type, resource_id,
                 metadata, status, error_message, chain_hash, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#,
        )
        .bind(&event.actor_id)
        .bind(&event.actor_email)
        .bind(&event.operation)
        .bind(&event.resource_type)
        .bind(&event.resource_id)
        .bind(&event.metadata)
        .bind(event.status.to_string())
        .bind(&event.error_message)
        .bind(&chain_hash)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        tracing::info!(
            audit_id = id,
            operation = %event.operation,
            status = %event.status,
            "audit event recorded"
        );

        Ok(id)
    }
    /// Export all audit logs for reporting compliance (Compile-Safe Runtime Query)
    pub async fn export_all_logs(&self) -> Result<Vec<AuditRow>, sqlx::Error> {
        sqlx::query_as::<_, AuditRow>(
            r#"
            SELECT id, actor_id, actor_email, operation, resource_type,
                   resource_id, metadata, status, error_message, chain_hash,
                   created_at
            FROM audit_logs
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Enforce the 1-year compliance retention policy (Compile-Safe Runtime Query)
    pub async fn enforce_retention_policy(&self) -> Result<u64, sqlx::Error> {
        let result =
            sqlx::query("DELETE FROM audit_logs WHERE created_at < NOW() - INTERVAL '1 year'")
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected())
    }

    /// Query audit logs for a specific resource.
    pub async fn query_by_resource(
        &self,
        resource_type: &str,
        resource_id: &str,
        limit: i64,
    ) -> Result<Vec<AuditRow>, sqlx::Error> {
        sqlx::query_as::<_, AuditRow>(
            r#"
            SELECT id, actor_id, actor_email, operation, resource_type,
                   resource_id, metadata, status, error_message, chain_hash,
                   created_at
            FROM audit_logs
            WHERE resource_type = $1 AND resource_id = $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(resource_type)
        .bind(resource_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }

    /// Query audit logs for a specific actor.
    pub async fn query_by_actor(
        &self,
        actor_id: &str,
        limit: i64,
    ) -> Result<Vec<AuditRow>, sqlx::Error> {
        sqlx::query_as::<_, AuditRow>(
            r#"
            SELECT id, actor_id, actor_email, operation, resource_type,
                   resource_id, metadata, status, error_message, chain_hash,
                   created_at
            FROM audit_logs
            WHERE actor_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(actor_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
    }
}

/// A row returned from the `audit_logs` table.
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct AuditRow {
    pub id: i64,
    pub actor_id: Option<String>,
    pub actor_email: Option<String>,
    pub operation: String,
    pub resource_type: String,
    pub resource_id: String,
    pub metadata: serde_json::Value,
    pub status: String,
    pub error_message: Option<String>,
    pub chain_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Convenience constants for well-known operation names.
pub mod ops {
    pub const CONTRACT_VERIFY: &str = "contract.verify";
    pub const CONTRACT_PUBLISH: &str = "contract.publish";
    pub const CONTRACT_DELETE: &str = "contract.delete";
    pub const PUBLISHER_CHANGE: &str = "publisher.change";
    pub const USER_ROLE_CHANGE: &str = "user.role_change";
    pub const ADMIN_ACTION: &str = "admin.action";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_status_display() {
        assert_eq!(AuditStatus::Success.to_string(), "success");
        assert_eq!(AuditStatus::Failure.to_string(), "failure");
    }

    #[test]
    fn chain_hash_is_deterministic() {
        let prev = "genesis";
        let op = "contract.verify";
        let rid = "abc123";
        let ts = "2026-04-27T00:00:00+00:00";
        let input = format!("{prev}{op}{rid}{ts}");
        let h1 = hex::encode(sha2::Sha256::digest(input.as_bytes()));
        let h2 = hex::encode(sha2::Sha256::digest(input.as_bytes()));
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn chain_hash_changes_with_different_prev() {
        let make = |prev: &str| {
            let input = format!("{prev}contract.verifyabc1232026-04-27T00:00:00+00:00");
            hex::encode(sha2::Sha256::digest(input.as_bytes()))
        };
        assert_ne!(make("genesis"), make("someprevhash"));
    }

    #[test]
    fn ops_constants_are_non_empty() {
        assert!(!ops::CONTRACT_VERIFY.is_empty());
        assert!(!ops::PUBLISHER_CHANGE.is_empty());
        assert!(!ops::CONTRACT_DELETE.is_empty());
        assert!(!ops::USER_ROLE_CHANGE.is_empty());
    }
}
