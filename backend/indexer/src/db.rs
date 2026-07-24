use crate::rpc::ContractDeployment;
/// Database writer module
/// Handles writing detected contracts to the database
use shared::{Contract, Network};
use sqlx::{PgPool, QueryBuilder, Row};
use thiserror::Error;
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    SqlError(String),
    #[error("Contract already exists: {0}")]
    DuplicateContract(String),
}

/// Database writer for storing discovered contracts
pub struct DatabaseWriter {
    pool: PgPool,
}

impl DatabaseWriter {
    /// Create new database writer
    pub fn new(pool: PgPool) -> Self {
        DatabaseWriter { pool }
    }

    /// Write discovered contract to database
    /// Returns true if new contract was inserted, false if already existed
    pub async fn write_contract(
        &self,
        deployment: &ContractDeployment,
        network: &Network,
    ) -> Result<bool, DatabaseError> {
        debug!(
            "Writing contract to database: contract_id={}, network={:?}",
            deployment.contract_id, network
        );

        let network_str = network_to_str(network);

        // Check if contract already exists
        let existing = sqlx::query(
            r#"
            SELECT id FROM contracts
            WHERE contract_id = $1 AND network = $2::network_type
            LIMIT 1
            "#,
        )
        .bind(&deployment.contract_id)
        .bind(network_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to check for existing contract: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        if existing.is_some() {
            debug!(
                "Contract already exists in database: {}",
                deployment.contract_id
            );
            return Ok(false);
        }

        // Create a publisher record for the deployer if it doesn't exist
        let publisher_id = self.get_or_create_publisher(&deployment.deployer).await?;

        // Insert new contract with is_verified = false
        let contract_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let slug = shared::slugify(&deployment.contract_id);

        sqlx::query(
            r#"
            INSERT INTO contracts (
                id,
                contract_id,
                wasm_hash,
                name,
                slug,
                publisher_id,
                network,
                is_verified,
                created_at,
                updated_at,
                deployed_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7::network_type, $8, $9, $10, $11)
        "#,
        )
        .bind(contract_id)
        .bind(&deployment.contract_id)
        .bind(format!("{}_{}", deployment.contract_id, deployment.op_id))
        .bind(&deployment.contract_id)
        .bind(slug)
        .bind(publisher_id)
        .bind(network_str)
        .bind(false)
        .bind(now)
        .bind(now)
        .bind(
            chrono::DateTime::parse_from_rfc3339(&deployment.deployed_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or(now),
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!(
                "Failed to insert contract record: {} ({})",
                deployment.contract_id, e
            );
            DatabaseError::SqlError(e.to_string())
        })?;

        // Record the initial deploy interaction
        sqlx::query(
            r#"
            INSERT INTO contract_interactions
              (
                contract_id, user_address, interaction_type, transaction_hash,
                method, parameters, return_value, interaction_timestamp,
                interaction_count, network, created_at
              )
            VALUES ($1, $2, 'deploy', NULL, NULL, NULL, NULL, $3, 1, $4::network_type, $3)
            "#,
        )
        .bind(contract_id)
        .bind(Some(deployment.deployer.as_str()))
        .bind(now)
        .bind(network_str)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!(
                "Failed to insert deploy interaction for contract {}: {}",
                deployment.contract_id, e
            );
            DatabaseError::SqlError(e.to_string())
        })?;

        // Update daily aggregates for the deploy interaction
        sqlx::query(
            r#"
            INSERT INTO contract_interaction_daily_aggregates
              (contract_id, interaction_type, network, day, count, updated_at)
            VALUES ($1, 'deploy', $2::network_type, $3, 1, NOW())
            ON CONFLICT (contract_id, interaction_type, network, day)
            DO UPDATE SET
              count = contract_interaction_daily_aggregates.count + 1,
              updated_at = NOW()
            "#,
        )
        .bind(contract_id)
        .bind(network_str)
        .bind(now.date_naive())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!(
                "Failed to update deploy daily aggregate for contract {}: {}",
                deployment.contract_id, e
            );
            DatabaseError::SqlError(e.to_string())
        })?;

        info!(
            "Contract record created: contract_id={}, network={}, publisher={}",
            deployment.contract_id, network_str, deployment.deployer
        );

        Ok(true)
    }

    /// Write multiple contracts in a single transaction.
    ///
    /// Deployments are processed in chunks of at most `BATCH_CHUNK_SIZE` rows
    /// per INSERT to bound the size of each `QueryBuilder` SQL string. Duplicates
    /// (by `contract_id + network`) are silently skipped via `ON CONFLICT DO NOTHING`.
    pub async fn write_contracts_batch(
        &self,
        deployments: &[ContractDeployment],
        network: &Network,
    ) -> Result<(usize, usize), DatabaseError> {
        if deployments.is_empty() {
            return Ok((0, 0));
        }

        // Cap each INSERT to prevent the QueryBuilder SQL string from growing
        // unboundedly when a ledger contains a large burst of deployments.
        const BATCH_CHUNK_SIZE: usize = 50;

        let mut tx = self.pool.begin().await.map_err(|e| {
            error!("Failed to begin transaction: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        let network_str = network_to_str(network);
        let now = chrono::Utc::now();

        // 1. Resolve all publishers first (deduplicated by address)
        let deployers: std::collections::HashSet<&str> =
            deployments.iter().map(|d| d.deployer.as_str()).collect();

        let mut publisher_map = std::collections::HashMap::new();

        for deployer in deployers {
            let publisher_id = self.get_or_create_publisher_tx(&mut tx, deployer).await?;
            publisher_map.insert(deployer, publisher_id);
        }

        let mut total_new = 0usize;

        // 2. Batch insert contracts in bounded chunks to cap QueryBuilder memory usage.
        for chunk in deployments.chunks(BATCH_CHUNK_SIZE) {
            let mut query_builder: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
                "INSERT INTO contracts \
                 (id, contract_id, wasm_hash, name, slug, publisher_id, network, \
                  is_verified, created_at, updated_at, deployed_at) ",
            );

            query_builder.push_values(chunk.iter(), |mut b, deployment| {
                let publisher_id = publisher_map.get(deployment.deployer.as_str()).unwrap();
                let wasm_hash = format!("{}_{}", deployment.contract_id, deployment.op_id);
                let slug = shared::slugify(&deployment.contract_id);
                b.push_bind(Uuid::new_v4())
                    .push_bind(&deployment.contract_id)
                    .push_bind(wasm_hash)
                    .push_bind(&deployment.contract_id)
                    .push_bind(slug)
                    .push_bind(publisher_id)
                    .push_bind(network_str)
                    .push_bind(false)
                    .push_bind(now)
                    .push_bind(now)
                    .push_bind(
                        chrono::DateTime::parse_from_rfc3339(&deployment.deployed_at)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                            .unwrap_or(now),
                    );
            });

            query_builder
                .push(" ON CONFLICT (contract_id, network) DO NOTHING RETURNING contract_id");

            // Count returned rows directly — avoids collecting all IDs into a HashSet.
            let chunk_new = query_builder
                .build()
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| {
                    error!("Failed to execute batch contract insert: {}", e);
                    DatabaseError::SqlError(e.to_string())
                })?
                .len();

            total_new += chunk_new;
        }

        // publisher_map is no longer needed; drop it before the commit so its
        // heap allocation is released while the transaction is still open.
        drop(publisher_map);

        let duplicate_count = deployments.len() - total_new;

        tx.commit().await.map_err(|e| {
            error!("Failed to commit transaction: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        info!(
            "Batch write complete: new={}, duplicates={}",
            total_new, duplicate_count
        );

        Ok((total_new, duplicate_count))
    }

    /// Get or create a publisher record — transaction-scoped variant.
    ///
    /// Uses `DO NOTHING` + follow-up `SELECT` (same safe pattern as the
    /// non-tx version) to avoid the UUID-overwrite race described in
    /// issue #316.
    async fn get_or_create_publisher_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        address: &str,
    ) -> Result<Uuid, DatabaseError> {
        let now = chrono::Utc::now();
        let candidate_id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO publishers (id, stellar_address, created_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (stellar_address) DO NOTHING
            "#,
        )
        .bind(candidate_id)
        .bind(address)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            error!("Failed to upsert publisher: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        let row = sqlx::query(
            r#"
            SELECT id FROM publishers
            WHERE stellar_address = $1
            LIMIT 1
            "#,
        )
        .bind(address)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| {
            error!("Failed to fetch publisher after upsert: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        let id: Uuid = row.try_get("id").map_err(|e| {
            DatabaseError::SqlError(format!("Failed to decode publisher uuid: {}", e))
        })?;

        Ok(id)
    }

    /// Get or create a publisher record for a deployer address.
    ///
    /// ## Bug fix (issue #316)
    ///
    /// The original implementation had two problems:
    ///
    /// 1. **UUID overwrite / race condition** — the `ON CONFLICT` clause used
    ///    `DO UPDATE SET id = EXCLUDED.id`, which replaced the existing
    ///    publisher's primary key with a freshly generated UUID. Under
    ///    concurrent load (two calls racing for the same `stellar_address`)
    ///    this orphaned every `contracts` row that referenced the old `id`.
    ///
    ///    Fix: use `DO NOTHING` so the existing row is never touched, then
    ///    always `SELECT` afterwards to retrieve the canonical `id` — whether
    ///    the row was just inserted or already existed.
    ///
    /// 2. **Fragile UUID decoding** — the existing code read `id` as raw
    ///    `Vec<u8>` and called `Uuid::from_slice`, which fails when PostgreSQL
    ///    returns the UUID in its text representation (e.g. when the column is
    ///    `uuid` type and the driver returns a string).
    ///
    ///    Fix: use sqlx's native `Uuid` type via `row.try_get::<Uuid, _>("id")`
    ///    which handles both binary and text wire formats correctly.
    async fn get_or_create_publisher(&self, address: &str) -> Result<Uuid, DatabaseError> {
        debug!("Getting or creating publisher for address: {}", address);

        let now = chrono::Utc::now();
        let candidate_id = Uuid::new_v4();

        // Insert a new row only if no row with this stellar_address exists yet.
        // DO NOTHING ensures we never overwrite the existing publisher's id,
        // which would orphan all contracts referencing the old id.
        sqlx::query(
            r#"
            INSERT INTO publishers (id, stellar_address, created_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (stellar_address) DO NOTHING
            "#,
        )
        .bind(candidate_id)
        .bind(address)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to upsert publisher: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        // Always SELECT the canonical id — whether we just inserted or the row
        // already existed. Using sqlx's native Uuid type avoids the fragile
        // Vec<u8> -> Uuid::from_slice conversion that broke on text wire format.
        let row = sqlx::query(
            r#"
            SELECT id FROM publishers
            WHERE stellar_address = $1
            LIMIT 1
            "#,
        )
        .bind(address)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch publisher after upsert: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        let id: Uuid = row.try_get("id").map_err(|e| {
            DatabaseError::SqlError(format!("Failed to decode publisher uuid: {}", e))
        })?;

        debug!("Resolved publisher: {} -> {}", address, id);
        Ok(id)
    }

    /// Get recently indexed contracts (for verification)
    pub async fn get_recent_contracts(
        &self,
        network: &Network,
        limit: i32,
    ) -> Result<Vec<Contract>, DatabaseError> {
        let network_str = network_to_str(network);

        let rows = sqlx::query_as::<_, Contract>(
            r#"
            SELECT
                id, contract_id, wasm_hash, name, description,
                publisher_id, network, is_verified, verification_status, verified_by, verification_notes, verified_at, deployed_at, category, tags,
                created_at, updated_at
            FROM contracts
            WHERE network = $1::network_type AND is_verified = false
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(network_str)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch recent contracts: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        debug!("Fetched {} recent unverified contracts", rows.len());

        Ok(rows)
    }

    /// Check if a contract exists
    pub async fn contract_exists(
        &self,
        contract_id: &str,
        network: &Network,
    ) -> Result<bool, DatabaseError> {
        let network_str = network_to_str(network);

        let result = sqlx::query(
            r#"
            SELECT id FROM contracts
            WHERE contract_id = $1 AND network = $2::network_type
            LIMIT 1
            "#,
        )
        .bind(contract_id)
        .bind(network_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to check contract existence: {}", e);
            DatabaseError::SqlError(e.to_string())
        })?;

        Ok(result.is_some())
    }
}

/// Convert Network enum to string for database queries
fn network_to_str(network: &Network) -> &str {
    match network {
        Network::Mainnet => "mainnet",
        Network::Testnet => "testnet",
        Network::Futurenet => "futurenet",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_to_str() {
        assert_eq!(network_to_str(&Network::Mainnet), "mainnet");
        assert_eq!(network_to_str(&Network::Testnet), "testnet");
        assert_eq!(network_to_str(&Network::Futurenet), "futurenet");
    }
}
