//! Retention policy for proposal history tables.
//!
//! Terminal-state proposals (`executed`, `expired`, `rejected`, `cancelled`) are
//! never read back in normal operation but keep growing the live tables and the
//! indexes on their status and timestamp columns. This job moves them into the
//! `*_archive` tables past a configurable retention window.
//!
//! Records in a non-terminal state are never touched regardless of age: a
//! `pending` proposal is still collecting signatures, an `approved` deploy
//! proposal is still awaiting execution, and a `passed` governance proposal is
//! still inside its execution delay.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use std::env;
use uuid::Uuid;

const DEFAULT_RETENTION_DAYS: u32 = 90;
const DEFAULT_INTERVAL_SECONDS: u64 = 86_400;
const DEFAULT_BATCH_SIZE: i64 = 1_000;

/// Statuses a deploy proposal can no longer transition out of.
///
/// `pending` and `approved` are excluded: an approved proposal has collected its
/// signatures but has not been executed yet.
pub const DEPLOY_PROPOSAL_TERMINAL_STATES: [&str; 3] = ["executed", "expired", "rejected"];

/// Statuses a governance proposal can no longer transition out of.
///
/// `pending`, `active`, and `passed` are excluded: a passed proposal is waiting
/// out `execution_delay_hours` before it can be executed.
pub const GOVERNANCE_PROPOSAL_TERMINAL_STATES: [&str; 3] = ["executed", "rejected", "cancelled"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionConfig {
    pub enabled: bool,
    pub retention_days: u32,
    pub interval_seconds: u64,
    pub batch_size: i64,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            retention_days: DEFAULT_RETENTION_DAYS,
            interval_seconds: DEFAULT_INTERVAL_SECONDS,
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }
}

impl RetentionConfig {
    pub fn from_env() -> Self {
        let config = Self {
            enabled: env_bool("RETENTION_ENABLED", true),
            retention_days: env_u32("RETENTION_DAYS", DEFAULT_RETENTION_DAYS),
            interval_seconds: env_u64("RETENTION_INTERVAL_SECONDS", DEFAULT_INTERVAL_SECONDS),
            batch_size: env_u32("RETENTION_BATCH_SIZE", DEFAULT_BATCH_SIZE as u32) as i64,
        };

        tracing::info!(
            enabled = config.enabled,
            retention_days = config.retention_days,
            interval_seconds = config.interval_seconds,
            batch_size = config.batch_size,
            "retention: config loaded"
        );

        config
    }

    /// The instant a record must predate to be eligible for archival.
    pub fn cutoff(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now - Duration::days(i64::from(self.retention_days))
    }
}

/// Number of rows moved into archive tables by a single run.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RetentionOutcome {
    pub deploy_proposals: u64,
    pub proposal_signatures: u64,
    pub governance_proposals: u64,
    pub governance_votes: u64,
}

/// Reference definition of the eligibility rule the purge queries implement.
///
/// A record sitting exactly on the retention threshold is still inside the
/// window and is retained; only records strictly older than the cutoff are
/// archived. The filter itself runs in SQL, so this is not called on the
/// production path — it pins the boundary semantics the queries must match.
#[allow(dead_code)]
pub fn is_purgeable(
    status: &str,
    terminal_states: &[&str],
    terminal_at: DateTime<Utc>,
    cutoff: DateTime<Utc>,
) -> bool {
    terminal_states.contains(&status) && terminal_at < cutoff
}

/// Spawn the background retention task.
pub fn spawn_retention_task(pool: PgPool, config: RetentionConfig) {
    if !config.enabled {
        tracing::info!("retention: disabled, background task not started");
        return;
    }

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(config.interval_seconds));

        loop {
            interval.tick().await;
            tracing::info!("retention: starting run");

            match run_retention(&pool, &config, Utc::now()).await {
                Ok(outcome) => tracing::info!(
                    deploy_proposals = outcome.deploy_proposals,
                    proposal_signatures = outcome.proposal_signatures,
                    governance_proposals = outcome.governance_proposals,
                    governance_votes = outcome.governance_votes,
                    "retention: run complete"
                ),
                Err(err) => tracing::error!(error = ?err, "retention: run failed"),
            }
        }
    });
}

/// Archive terminal-state records older than the retention window.
///
/// Each table is archived in its own transaction, so a partial failure never
/// leaves a proposal deleted without its archive copy. The run stops at the
/// first table that errors; already-committed work stands and the next tick
/// retries the remainder.
pub async fn run_retention(
    pool: &PgPool,
    config: &RetentionConfig,
    now: DateTime<Utc>,
) -> Result<RetentionOutcome, sqlx::Error> {
    let cutoff = config.cutoff(now);
    let mut outcome = RetentionOutcome::default();

    let (proposals, signatures) = archive_deploy_proposals(pool, cutoff, config.batch_size).await?;
    outcome.deploy_proposals = proposals;
    outcome.proposal_signatures = signatures;

    let (proposals, votes) = archive_governance_proposals(pool, cutoff, config.batch_size).await?;
    outcome.governance_proposals = proposals;
    outcome.governance_votes = votes;

    Ok(outcome)
}

/// Move terminal deploy proposals and their signatures into archive tables.
///
/// The retention anchor is `updated_at`, maintained by the table's trigger, so it
/// marks when the record entered its terminal state rather than when it was
/// created.
async fn archive_deploy_proposals(
    pool: &PgPool,
    cutoff: DateTime<Utc>,
    batch_size: i64,
) -> Result<(u64, u64), sqlx::Error> {
    let terminal_states: Vec<String> = DEPLOY_PROPOSAL_TERMINAL_STATES
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut tx = pool.begin().await?;

    let ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM deploy_proposals
        WHERE status::text = ANY($1)
          AND updated_at < $2
        ORDER BY updated_at
        LIMIT $3
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(&terminal_states)
    .bind(cutoff)
    .bind(batch_size)
    .fetch_all(&mut *tx)
    .await?;

    if ids.is_empty() {
        tx.rollback().await?;
        return Ok((0, 0));
    }

    let signatures = sqlx::query(
        r#"
        INSERT INTO proposal_signatures_archive (
            id, proposal_id, signer_address, signature_data, signed_at
        )
        SELECT id, proposal_id, signer_address, signature_data, signed_at
        FROM proposal_signatures
        WHERE proposal_id = ANY($1)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(&ids)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    let proposals = sqlx::query(
        r#"
        INSERT INTO deploy_proposals_archive (
            id, contract_name, contract_id, wasm_hash, network, description,
            policy_id, status, expires_at, executed_at, proposer, created_at, updated_at
        )
        SELECT id, contract_name, contract_id, wasm_hash, network::text, description,
               policy_id, status::text, expires_at, executed_at, proposer, created_at, updated_at
        FROM deploy_proposals
        WHERE id = ANY($1)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(&ids)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    // proposal_signatures cascade-delete with the parent; they are archived above.
    sqlx::query("DELETE FROM deploy_proposals WHERE id = ANY($1)")
        .bind(&ids)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok((proposals, signatures))
}

/// Move terminal governance proposals and their votes into archive tables.
///
/// `governance_proposals` has no `updated_at`, so the retention anchor is
/// `executed_at` when present and `created_at` otherwise. The expression is
/// backed by a matching index.
async fn archive_governance_proposals(
    pool: &PgPool,
    cutoff: DateTime<Utc>,
    batch_size: i64,
) -> Result<(u64, u64), sqlx::Error> {
    let terminal_states: Vec<String> = GOVERNANCE_PROPOSAL_TERMINAL_STATES
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut tx = pool.begin().await?;

    let ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM governance_proposals
        WHERE status::text = ANY($1)
          AND COALESCE(executed_at, created_at) < $2
        ORDER BY COALESCE(executed_at, created_at)
        LIMIT $3
        FOR UPDATE SKIP LOCKED
        "#,
    )
    .bind(&terminal_states)
    .bind(cutoff)
    .bind(batch_size)
    .fetch_all(&mut *tx)
    .await?;

    if ids.is_empty() {
        tx.rollback().await?;
        return Ok((0, 0));
    }

    let votes = sqlx::query(
        r#"
        INSERT INTO governance_votes_archive (
            id, proposal_id, voter, vote_choice, voting_power, delegated_from, created_at
        )
        SELECT id, proposal_id, voter, vote_choice::text, voting_power, delegated_from, created_at
        FROM governance_votes
        WHERE proposal_id = ANY($1)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(&ids)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    let proposals = sqlx::query(
        r#"
        INSERT INTO governance_proposals_archive (
            id, contract_id, title, description, governance_model, proposer, status,
            voting_starts_at, voting_ends_at, execution_delay_hours, quorum_required,
            approval_threshold, created_at, executed_at
        )
        SELECT id, contract_id, title, description, governance_model::text, proposer, status::text,
               voting_starts_at, voting_ends_at, execution_delay_hours, quorum_required,
               approval_threshold, created_at, executed_at
        FROM governance_proposals
        WHERE id = ANY($1)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(&ids)
    .execute(&mut *tx)
    .await?
    .rows_affected();

    // governance_votes cascade-delete with the parent; they are archived above.
    sqlx::query("DELETE FROM governance_proposals WHERE id = ANY($1)")
        .bind(&ids)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok((proposals, votes))
}

fn env_bool(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(raw) => match raw.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" => true,
            "false" | "0" | "no" => false,
            _ => {
                tracing::warn!("Invalid value for {key} (`{raw}`), using default {default}");
                default
            }
        },
        Err(_) => default,
    }
}

fn env_u32(key: &str, default: u32) -> u32 {
    match env::var(key) {
        Ok(raw) => match raw.parse::<u32>() {
            Ok(value) if value > 0 => value,
            _ => {
                tracing::warn!("Invalid value for {key} (`{raw}`), using default {default}");
                default
            }
        },
        Err(_) => default,
    }
}

fn env_u64(key: &str, default: u64) -> u64 {
    match env::var(key) {
        Ok(raw) => match raw.parse::<u64>() {
            Ok(value) if value > 0 => value,
            _ => {
                tracing::warn!("Invalid value for {key} (`{raw}`), using default {default}");
                default
            }
        },
        Err(_) => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(retention_days: u32) -> RetentionConfig {
        RetentionConfig {
            retention_days,
            ..RetentionConfig::default()
        }
    }

    fn at(now: DateTime<Utc>, days_ago: i64) -> DateTime<Utc> {
        now - Duration::days(days_ago)
    }

    #[test]
    fn cutoff_is_retention_days_before_now() {
        let now = Utc::now();
        assert_eq!(config(90).cutoff(now), now - Duration::days(90));
    }

    #[test]
    fn record_exactly_at_threshold_is_retained() {
        let now = Utc::now();
        let cutoff = config(90).cutoff(now);

        assert!(!is_purgeable(
            "expired",
            &DEPLOY_PROPOSAL_TERMINAL_STATES,
            at(now, 90),
            cutoff
        ));
    }

    #[test]
    fn record_just_under_threshold_is_retained() {
        let now = Utc::now();
        let cutoff = config(90).cutoff(now);
        let terminal_at = at(now, 90) + Duration::seconds(1);

        assert!(!is_purgeable(
            "expired",
            &DEPLOY_PROPOSAL_TERMINAL_STATES,
            terminal_at,
            cutoff
        ));
    }

    #[test]
    fn record_just_over_threshold_is_purged() {
        let now = Utc::now();
        let cutoff = config(90).cutoff(now);
        let terminal_at = at(now, 90) - Duration::seconds(1);

        assert!(is_purgeable(
            "expired",
            &DEPLOY_PROPOSAL_TERMINAL_STATES,
            terminal_at,
            cutoff
        ));
    }

    #[test]
    fn active_deploy_statuses_are_never_purged_regardless_of_age() {
        let now = Utc::now();
        let cutoff = config(90).cutoff(now);
        let ancient = at(now, 3650);

        for status in ["pending", "approved"] {
            assert!(
                !is_purgeable(status, &DEPLOY_PROPOSAL_TERMINAL_STATES, ancient, cutoff),
                "{status} must never be purged"
            );
        }
    }

    #[test]
    fn active_governance_statuses_are_never_purged_regardless_of_age() {
        let now = Utc::now();
        let cutoff = config(90).cutoff(now);
        let ancient = at(now, 3650);

        for status in ["pending", "active", "passed"] {
            assert!(
                !is_purgeable(
                    status,
                    &GOVERNANCE_PROPOSAL_TERMINAL_STATES,
                    ancient,
                    cutoff
                ),
                "{status} must never be purged"
            );
        }
    }

    #[test]
    fn terminal_deploy_statuses_past_window_are_purged() {
        let now = Utc::now();
        let cutoff = config(90).cutoff(now);
        let old = at(now, 91);

        for status in DEPLOY_PROPOSAL_TERMINAL_STATES {
            assert!(
                is_purgeable(status, &DEPLOY_PROPOSAL_TERMINAL_STATES, old, cutoff),
                "{status} should be purged"
            );
        }
    }

    #[test]
    fn terminal_governance_statuses_past_window_are_purged() {
        let now = Utc::now();
        let cutoff = config(90).cutoff(now);
        let old = at(now, 91);

        for status in GOVERNANCE_PROPOSAL_TERMINAL_STATES {
            assert!(
                is_purgeable(status, &GOVERNANCE_PROPOSAL_TERMINAL_STATES, old, cutoff),
                "{status} should be purged"
            );
        }
    }

    #[test]
    fn custom_retention_window_shifts_the_boundary() {
        let now = Utc::now();
        let cutoff = config(30).cutoff(now);

        assert!(!is_purgeable(
            "rejected",
            &DEPLOY_PROPOSAL_TERMINAL_STATES,
            at(now, 30),
            cutoff
        ));
        assert!(is_purgeable(
            "rejected",
            &DEPLOY_PROPOSAL_TERMINAL_STATES,
            at(now, 31),
            cutoff
        ));
    }

    #[test]
    fn passed_is_not_terminal_for_governance() {
        assert!(!GOVERNANCE_PROPOSAL_TERMINAL_STATES.contains(&"passed"));
    }

    #[test]
    fn approved_is_not_terminal_for_deploy() {
        assert!(!DEPLOY_PROPOSAL_TERMINAL_STATES.contains(&"approved"));
    }

    #[test]
    fn default_config_matches_documented_defaults() {
        let config = RetentionConfig::default();

        assert!(config.enabled);
        assert_eq!(config.retention_days, 90);
        assert_eq!(config.interval_seconds, 86_400);
        assert_eq!(config.batch_size, 1_000);
    }
}

/// Boundary tests that exercise the purge SQL against a real database.
///
/// These are `#[ignore]`d because they need a migrated Postgres instance:
///
/// ```text
/// RETENTION_TEST_DATABASE_URL=postgresql://postgres:postgres@localhost/soroban_registry_test \
///     cargo test --bin api retention::db_tests -- --ignored --test-threads=1
/// ```
#[cfg(test)]
mod db_tests {
    use super::*;

    async fn test_pool() -> PgPool {
        let url = env::var("RETENTION_TEST_DATABASE_URL")
            .expect("RETENTION_TEST_DATABASE_URL must be set to run DB-backed retention tests");

        PgPool::connect(&url)
            .await
            .expect("connect to retention test database")
    }

    async fn seed_policy(pool: &PgPool) -> Uuid {
        sqlx::query_scalar(
            r#"
            INSERT INTO multisig_policies (name, threshold, signer_addresses, created_by)
            VALUES ('retention-test', 1, ARRAY['GRETENTIONTEST'], 'GRETENTIONTEST')
            RETURNING id
            "#,
        )
        .fetch_one(pool)
        .await
        .expect("seed multisig policy")
    }

    async fn seed_deploy_proposal(
        pool: &PgPool,
        policy_id: Uuid,
        status: &str,
        updated_at: DateTime<Utc>,
    ) -> Uuid {
        sqlx::query_scalar(
            r#"
            INSERT INTO deploy_proposals (
                contract_name, contract_id, wasm_hash, network, policy_id,
                status, expires_at, proposer, created_at, updated_at
            )
            VALUES (
                'retention-test', 'CRETENTIONTEST', 'deadbeef', 'testnet'::network_type, $1,
                $2::proposal_status, $3, 'GRETENTIONTEST', $3, $3
            )
            RETURNING id
            "#,
        )
        .bind(policy_id)
        .bind(status)
        .bind(updated_at)
        .fetch_one(pool)
        .await
        .expect("seed deploy proposal")
    }

    async fn live_exists(pool: &PgPool, id: Uuid) -> bool {
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM deploy_proposals WHERE id = $1)")
            .bind(id)
            .fetch_one(pool)
            .await
            .expect("check live row")
    }

    async fn archived_exists(pool: &PgPool, id: Uuid) -> bool {
        sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM deploy_proposals_archive WHERE id = $1)",
        )
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("check archived row")
    }

    async fn cleanup(pool: &PgPool, policy_id: Uuid, ids: &[Uuid]) {
        sqlx::query("DELETE FROM proposal_signatures_archive WHERE proposal_id = ANY($1)")
            .bind(ids)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM deploy_proposals_archive WHERE id = ANY($1)")
            .bind(ids)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM deploy_proposals WHERE id = ANY($1)")
            .bind(ids)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM multisig_policies WHERE id = $1")
            .bind(policy_id)
            .execute(pool)
            .await
            .ok();
    }

    /// Records exactly at, just inside, and just past the retention threshold.
    #[tokio::test]
    #[ignore]
    async fn purge_respects_retention_boundary() {
        let pool = test_pool().await;
        let config = RetentionConfig::default();
        let now = Utc::now();
        let threshold = config.cutoff(now);

        let policy_id = seed_policy(&pool).await;
        let exactly_at = seed_deploy_proposal(&pool, policy_id, "expired", threshold).await;
        let just_under =
            seed_deploy_proposal(&pool, policy_id, "expired", threshold + Duration::seconds(1))
                .await;
        let just_over =
            seed_deploy_proposal(&pool, policy_id, "expired", threshold - Duration::seconds(1))
                .await;

        run_retention(&pool, &config, now).await.expect("retention run");

        assert!(
            live_exists(&pool, exactly_at).await,
            "record exactly at the threshold is still within the window"
        );
        assert!(
            live_exists(&pool, just_under).await,
            "record just inside the window is retained"
        );
        assert!(
            !live_exists(&pool, just_over).await,
            "record just past the window is removed from the live table"
        );
        assert!(
            archived_exists(&pool, just_over).await,
            "purged record is archived, not hard-deleted"
        );

        cleanup(&pool, policy_id, &[exactly_at, just_under, just_over]).await;
    }

    /// Non-terminal records are retained no matter how old they are.
    #[tokio::test]
    #[ignore]
    async fn active_records_are_never_purged() {
        let pool = test_pool().await;
        let config = RetentionConfig::default();
        let now = Utc::now();
        let ancient = now - Duration::days(3650);

        let policy_id = seed_policy(&pool).await;
        let pending = seed_deploy_proposal(&pool, policy_id, "pending", ancient).await;
        let approved = seed_deploy_proposal(&pool, policy_id, "approved", ancient).await;

        run_retention(&pool, &config, now).await.expect("retention run");

        assert!(live_exists(&pool, pending).await, "pending must survive");
        assert!(live_exists(&pool, approved).await, "approved must survive");

        cleanup(&pool, policy_id, &[pending, approved]).await;
    }

    /// Cascading child rows are archived rather than destroyed with the parent.
    #[tokio::test]
    #[ignore]
    async fn cascading_signatures_are_archived_with_the_parent() {
        let pool = test_pool().await;
        let config = RetentionConfig::default();
        let now = Utc::now();

        let policy_id = seed_policy(&pool).await;
        let proposal_id =
            seed_deploy_proposal(&pool, policy_id, "executed", now - Duration::days(120)).await;

        sqlx::query(
            r#"
            INSERT INTO proposal_signatures (proposal_id, signer_address, signature_data)
            VALUES ($1, 'GRETENTIONTEST', 'sig')
            "#,
        )
        .bind(proposal_id)
        .execute(&pool)
        .await
        .expect("seed signature");

        run_retention(&pool, &config, now).await.expect("retention run");

        let archived_signatures: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM proposal_signatures_archive WHERE proposal_id = $1",
        )
        .bind(proposal_id)
        .fetch_one(&pool)
        .await
        .expect("count archived signatures");

        assert_eq!(
            archived_signatures, 1,
            "signature trail must be preserved when the parent proposal is purged"
        );

        cleanup(&pool, policy_id, &[proposal_id]).await;
    }
}
