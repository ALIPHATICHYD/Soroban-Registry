use anyhow::Result;
use chrono::{DateTime, Utc};
use shared::{Contract, ContractHealth, ContractStats, HealthStatus};
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time;
use tracing::{error, info, warn};

use crate::state::AppState;

const MAX_TOTAL_HEALTH_SCORE: i32 = 100;
const MIN_TOTAL_HEALTH_SCORE: i32 = 0;
const HEALTHY_STATUS_MIN_SCORE: i32 = 80;
const WARNING_STATUS_MIN_SCORE: i32 = 50;
const INACTIVITY_WARNING_DAYS: i64 = 30;
const INACTIVITY_CRITICAL_DAYS: i64 = 90;

/// Security score is intentionally reported on a 0..50 scale.
///
/// We divide total health by 2 (ratio 0.5) so the security sub-score remains comparable
/// but lower-range than the 0..100 total score used for health-status thresholds.
const SECURITY_SCORE_DIVISOR: i32 = 2;
const SECURITY_TO_HEALTH_RATIO: f64 = 1.0 / SECURITY_SCORE_DIVISOR as f64;
const MIN_SECURITY_SCORE: i32 = 0;
const MAX_SECURITY_SCORE: i32 = (MAX_TOTAL_HEALTH_SCORE as f64 * SECURITY_TO_HEALTH_RATIO) as i32;

/// Tracks the runtime status of the health monitor background task.
#[derive(Clone)]
pub struct HealthMonitorStatus {
    pub last_run: Arc<RwLock<Option<DateTime<Utc>>>>,
    pub last_success: Arc<RwLock<Option<DateTime<Utc>>>>,
    pub last_error: Arc<RwLock<Option<String>>>,
    pub total_runs: Arc<AtomicU64>,
    pub total_failures: Arc<AtomicU64>,
    pub contracts_checked: Arc<AtomicU64>,
    pub contracts_failed: Arc<AtomicU64>,
}

impl Default for HealthMonitorStatus {
    fn default() -> Self {
        Self {
            last_run: Arc::new(RwLock::new(None)),
            last_success: Arc::new(RwLock::new(None)),
            last_error: Arc::new(RwLock::new(None)),
            total_runs: Arc::new(AtomicU64::new(0)),
            total_failures: Arc::new(AtomicU64::new(0)),
            contracts_checked: Arc::new(AtomicU64::new(0)),
            contracts_failed: Arc::new(AtomicU64::new(0)),
        }
    }
}

/// Serializable snapshot of the health monitor status for the API response.
#[derive(Debug, serde::Serialize)]
pub struct HealthMonitorStatusResponse {
    pub running: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub last_success: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub total_runs: u64,
    pub total_failures: u64,
    pub contracts_checked: u64,
    pub contracts_failed: u64,
}

impl HealthMonitorStatus {
    pub async fn snapshot(&self) -> HealthMonitorStatusResponse {
        let last_run = *self.last_run.read().await;
        let last_success = *self.last_success.read().await;
        let last_error = self.last_error.read().await.clone();
        HealthMonitorStatusResponse {
            running: last_run.is_some(),
            last_run,
            last_success,
            last_error,
            total_runs: self.total_runs.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            contracts_checked: self.contracts_checked.load(Ordering::Relaxed),
            contracts_failed: self.contracts_failed.load(Ordering::Relaxed),
        }
    }
}

/// Main loop for the health monitor background task
pub async fn run_health_monitor(state: AppState, status: HealthMonitorStatus) {
    info!("Starting health monitor background task");

    // Run every hour
    let mut interval = time::interval(time::Duration::from_secs(3600));

    loop {
        interval.tick().await;
        info!("Running health checks...");

        status.total_runs.fetch_add(1, Ordering::Relaxed);
        *status.last_run.write().await = Some(Utc::now());

        match perform_health_checks(&state.db, &status).await {
            Ok(()) => {
                *status.last_success.write().await = Some(Utc::now());
                *status.last_error.write().await = None;
                info!("Health checks completed successfully");
            }
            Err(e) => {
                let err_msg = format!("{}", e);
                error!("Error performing health checks: {}", err_msg);
                status.total_failures.fetch_add(1, Ordering::Relaxed);
                *status.last_error.write().await = Some(err_msg);
            }
        }
    }
}

async fn perform_health_checks(pool: &PgPool, status: &HealthMonitorStatus) -> Result<()> {
    // 1. Fetch all contracts
    let contracts: Vec<Contract> = sqlx::query_as("SELECT * FROM contracts")
        .fetch_all(pool)
        .await?;

    info!("Found {} contracts to check", contracts.len());

    let mut checked: u64 = 0;
    let mut failed: u64 = 0;

    for contract in contracts {
        // 2. Fetch stats (last activity)
        let stats: Option<ContractStats> =
            sqlx::query_as("SELECT * FROM contract_stats WHERE contract_id = $1")
                .bind(contract.id)
                .fetch_optional(pool)
                .await?;

        // 3. Calculate verification level
        let verification_level = if contract.is_verified {
            VerificationLevel::Verified
        } else {
            VerificationLevel::Unverified
        };

        let health = calculate_health(&contract, stats.as_ref(), verification_level);

        // 4. Update database — per-contract error handling so one failure doesn't stop the loop
        match upsert_contract_health(pool, &health).await {
            Ok(()) => {
                checked += 1;
            }
            Err(e) => {
                failed += 1;
                error!(
                    contract_id = %contract.id,
                    error = %e,
                    "Failed to upsert health for contract"
                );
            }
        }
    }

    status
        .contracts_checked
        .fetch_add(checked, Ordering::Relaxed);
    status.contracts_failed.fetch_add(failed, Ordering::Relaxed);

    info!(
        checked = checked,
        failed = failed,
        "Health checks completed"
    );

    if failed > 0 {
        warn!(
            "{} contract health upserts failed out of {}",
            failed,
            checked + failed
        );
    }

    Ok(())
}

/// Represents the graduated verification level of a smart contract.
/// Each level carries a varying degree of trust, which directly impacts the contract's health score.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum VerificationLevel {
    /// Contract is completely unverified. No source code has been matched.
    Unverified,
    /// Contract verification is currently in progress or awaiting review.
    Pending,
    /// Contract source code matches the deployed bytecode perfectly.
    Verified,
    /// Contract is verified AND has been externally audited by a trusted security firm.
    Audited,
}

impl VerificationLevel {
    /// Returns the health score weight modifier for the verification level.
    pub fn score_weight(&self) -> i32 {
        match self {
            // Unverified contracts suffer a severe health penalty (-40 base) due to lack of transparency
            VerificationLevel::Unverified => -40,
            // Pending contracts get an intermediate penalty since they are unverified but attempting reform
            VerificationLevel::Pending => -20,
            // Verified contracts are standard expectation; no penalty or bonus (baseline)
            VerificationLevel::Verified => 0,
            // Audited contracts receive a substantial health bonus (+20 base) reflecting premium trust
            VerificationLevel::Audited => 20,
        }
    }
}

fn derive_security_score(total_score: i32) -> i32 {
    let bounded_total = total_score.clamp(MIN_TOTAL_HEALTH_SCORE, MAX_TOTAL_HEALTH_SCORE);

    // Keep explicit validation before integer conversion to prevent underflow/overflow
    // if score bounds or ratio are changed in the future.
    let scaled = f64::from(bounded_total) * SECURITY_TO_HEALTH_RATIO;
    if !scaled.is_finite() {
        return MIN_SECURITY_SCORE;
    }

    let floored = scaled.floor();
    if floored < f64::from(MIN_SECURITY_SCORE) {
        return MIN_SECURITY_SCORE;
    }
    if floored > f64::from(MAX_SECURITY_SCORE) {
        return MAX_SECURITY_SCORE;
    }

    floored as i32
}

fn calculate_health(
    contract: &Contract,
    stats: Option<&ContractStats>,
    verification_level: VerificationLevel,
) -> ContractHealth {
    let mut score = MAX_TOTAL_HEALTH_SCORE;

    // Apply graduated verification score
    score += verification_level.score_weight();

    // Penalize for inactivity (older than 30 days)
    let last_activity = stats
        .and_then(|s| s.last_interaction)
        .unwrap_or(contract.created_at);

    let days_since_activity = (Utc::now() - last_activity).num_days();

    if days_since_activity > INACTIVITY_WARNING_DAYS {
        score -= 20;
    }

    if days_since_activity > INACTIVITY_CRITICAL_DAYS {
        score -= 20;
    }

    // Ensure score is within 0-100
    score = score.clamp(MIN_TOTAL_HEALTH_SCORE, MAX_TOTAL_HEALTH_SCORE);

    let mut recommendations = Vec::new();

    // Status thresholds remain tied to the total health score range (0..100),
    // while security_score is a derived 0..50 reporting sub-score.
    let status = match score {
        HEALTHY_STATUS_MIN_SCORE..=MAX_TOTAL_HEALTH_SCORE => HealthStatus::Healthy,
        WARNING_STATUS_MIN_SCORE..=79 => HealthStatus::Warning,
        _ => {
            tracing::warn!(contract_id = %contract.id, score, "Contract health is critical");
            HealthStatus::Critical
        }
    };

    match verification_level {
        VerificationLevel::Unverified => {
            recommendations.push(
                "Verify the contract source code to improve trust and health score.".to_string(),
            );
        }
        VerificationLevel::Pending => {
            recommendations.push("Contract verification is pending. Health score will improve once verification is complete.".to_string());
        }
        VerificationLevel::Verified => {
            // Optionally recommend an audit
            recommendations.push(
                "Consider obtaining an external audit to achieve maximum trust and health score."
                    .to_string(),
            );
        }
        VerificationLevel::Audited => {
            // Maximum verification achieved
        }
    }

    if days_since_activity > INACTIVITY_CRITICAL_DAYS {
        recommendations.push("Contract has been inactive for over 90 days. Consider engaging users or updating the contract.".to_string());
    } else if days_since_activity > INACTIVITY_WARNING_DAYS {
        recommendations.push("Contract has been inactive for over 30 days.".to_string());
    }

    if recommendations.is_empty() {
        recommendations.push("Contract is healthy and active. Keep it up!".to_string());
    }

    ContractHealth {
        contract_id: contract.id,
        status: status.to_string(),
        last_activity,
        security_score: derive_security_score(score),
        audit_date: None,
        total_score: score,
        recommendations,
        updated_at: Utc::now(),
    }
}

async fn upsert_contract_health(pool: &PgPool, health: &ContractHealth) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO contract_health (contract_id, status, last_activity, security_score, audit_date, total_score, recommendations, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (contract_id)
        DO UPDATE SET
            status = EXCLUDED.status,
            last_activity = EXCLUDED.last_activity,
            security_score = EXCLUDED.security_score,
            audit_date = EXCLUDED.audit_date,
            total_score = EXCLUDED.total_score,
            recommendations = EXCLUDED.recommendations,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(health.contract_id)
    .bind(&health.status)          // now a String — compatible with TEXT column
    .bind(health.last_activity)
    .bind(health.security_score)
    .bind(health.audit_date)
    .bind(health.total_score)
    .bind(&health.recommendations) // Vec<String> — compatible with TEXT[] column
    .bind(health.updated_at)
    .execute(pool)
    .await?;

    Ok(())
}

/// Handler: GET /api/health-monitor/status
///
/// Returns the current status of the health monitor background task.
pub async fn get_health_monitor_status(
    axum::extract::State(state): axum::extract::State<crate::state::AppState>,
) -> axum::Json<HealthMonitorStatusResponse> {
    axum::Json(state.health_monitor_status.snapshot().await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use shared::{Contract, ContractStats, Network};
    use uuid::Uuid;

    fn build_dummy_contract() -> Contract {
        Contract {
            id: Uuid::new_v4(),
            contract_id: "CDUMMY".to_string(),
            wasm_hash: "hash".to_string(),
            name: "Dummy".to_string(),
            slug: "dummy".to_string(),
            description: None,
            publisher_id: Uuid::new_v4(),
            network: Network::Testnet,
            is_verified: true,
            verification_status: shared::VerificationStatus::Verified,
            category: None,
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            verified_at: None,
            deployed_at: None,
            verified_by: None,
            verification_notes: None,
            health_score: 0,
            is_maintenance: false,
            last_accessed_at: None,
            logical_id: None,
            network_configs: None,
            relevance_score: None,
            organization_id: None,
            visibility: shared::VisibilityType::Public,
            current_version: None,
            usage_count: 0,
        }
    }

    #[test]
    fn test_security_score_edge_zero() {
        assert_eq!(derive_security_score(0), 0);
    }

    #[test]
    fn test_security_score_edge_max() {
        assert_eq!(
            derive_security_score(MAX_TOTAL_HEALTH_SCORE),
            MAX_SECURITY_SCORE
        );
    }

    #[test]
    fn test_security_score_clamps_out_of_range_inputs() {
        assert_eq!(derive_security_score(i32::MIN), MIN_SECURITY_SCORE);
        assert_eq!(derive_security_score(i32::MAX), MAX_SECURITY_SCORE);
    }

    #[test]
    fn test_security_score_alignment_with_status_thresholds() {
        assert_eq!(derive_security_score(HEALTHY_STATUS_MIN_SCORE), 40);
        assert_eq!(derive_security_score(WARNING_STATUS_MIN_SCORE), 25);
    }

    #[test]
    fn test_health_score_unverified() {
        let contract = build_dummy_contract();
        // Unverified penalty: -40. Base 100 -> 60
        let health = calculate_health(&contract, None, VerificationLevel::Unverified);
        assert_eq!(health.total_score, 60);

        assert_eq!(health.security_score, 30);
        assert!(health.recommendations.contains(
            &"Verify the contract source code to improve trust and health score.".to_string()
        ));
    }

    #[test]
    fn test_health_score_pending() {
        let contract = build_dummy_contract();
        // Pending penalty: -20. Base 100 -> 80
        let health = calculate_health(&contract, None, VerificationLevel::Pending);
        assert_eq!(health.total_score, 80);
        assert_eq!(health.security_score, 40);
        assert!(health.recommendations.contains(&"Contract verification is pending. Health score will improve once verification is complete.".to_string()));
    }

    #[test]
    fn test_health_score_verified() {
        let contract = build_dummy_contract();
        // Verified: +0. Base 100 -> 100
        let health = calculate_health(&contract, None, VerificationLevel::Verified);
        assert_eq!(health.total_score, 100);
        assert!(health.recommendations.contains(
            &"Consider obtaining an external audit to achieve maximum trust and health score."
                .to_string()
        ));
    }

    #[test]
    fn test_health_score_audited() {
        let contract = build_dummy_contract();
        // Audited: +20. Base 100 -> 100 (capped at 100)
        let health = calculate_health(&contract, None, VerificationLevel::Audited);
        assert_eq!(health.total_score, 100);
        assert_eq!(health.security_score, 50);
    }

    #[test]
    fn test_health_score_audited_with_inactivity() {
        let contract = build_dummy_contract();
        let stats = ContractStats {
            contract_id: contract.id,
            total_deployments: 1,
            total_interactions: 1,
            unique_users: 1,
            last_interaction: Some(Utc::now() - chrono::Duration::days(40)), // > 30 days inactive -> -20 penalty
        };
        // Base 100 + 20 (Audited) - 20 (Inactive > 30 days) = 100
        let health = calculate_health(&contract, Some(&stats), VerificationLevel::Audited);
        assert_eq!(health.total_score, 100);
        assert_eq!(health.security_score, 50);
    }

    // ─── Additional tests for the health scoring pipeline ───────────────

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "healthy");
        assert_eq!(HealthStatus::Warning.to_string(), "warning");
        assert_eq!(HealthStatus::Critical.to_string(), "critical");
    }

    #[test]
    fn test_health_status_from_str() {
        assert_eq!(
            "healthy".parse::<HealthStatus>().unwrap(),
            HealthStatus::Healthy
        );
        assert_eq!(
            "warning".parse::<HealthStatus>().unwrap(),
            HealthStatus::Warning
        );
        assert_eq!(
            "critical".parse::<HealthStatus>().unwrap(),
            HealthStatus::Critical
        );
        assert!("unknown".parse::<HealthStatus>().is_err());
    }

    #[test]
    fn test_contract_health_status_field_is_string() {
        let contract = build_dummy_contract();
        let health = calculate_health(&contract, None, VerificationLevel::Verified);
        // status should be a valid health status string
        assert_eq!(health.status, "healthy");
        assert_eq!(health.health_status().unwrap(), HealthStatus::Healthy);
    }

    #[test]
    fn test_health_status_critical_with_unverified_and_inactivity() {
        let mut contract = build_dummy_contract();
        contract.is_verified = false;
        let stats = ContractStats {
            contract_id: contract.id,
            total_deployments: 0,
            total_interactions: 0,
            unique_users: 0,
            last_interaction: Some(Utc::now() - chrono::Duration::days(100)), // > 90 days
        };
        // Base 100 - 40 (Unverified) - 20 (>30d) - 20 (>90d) = 20 => Critical
        let health = calculate_health(&contract, Some(&stats), VerificationLevel::Unverified);
        assert_eq!(health.total_score, 20);
        assert_eq!(health.status, "critical");
        assert!(health.recommendations.iter().any(|r| r.contains("90 days")));
        assert!(health.recommendations.iter().any(|r| r.contains("Verify")));
    }

    #[test]
    fn test_health_recommendations_not_empty() {
        let contract = build_dummy_contract();
        // Even for the best case, recommendations should not be empty
        let health = calculate_health(&contract, None, VerificationLevel::Audited);
        assert!(
            !health.recommendations.is_empty(),
            "Recommendations should never be empty"
        );
        assert!(health
            .recommendations
            .contains(&"Contract is healthy and active. Keep it up!".to_string()));
    }

    #[test]
    fn test_verification_level_score_weights() {
        assert_eq!(VerificationLevel::Unverified.score_weight(), -40);
        assert_eq!(VerificationLevel::Pending.score_weight(), -20);
        assert_eq!(VerificationLevel::Verified.score_weight(), 0);
        assert_eq!(VerificationLevel::Audited.score_weight(), 20);
    }

    #[test]
    fn test_derive_security_score_midpoint() {
        // score 50 -> security 25
        assert_eq!(derive_security_score(50), 25);
    }

    #[test]
    fn test_derive_security_score_negative_input() {
        assert_eq!(derive_security_score(-10), 0);
    }

    #[test]
    fn test_derive_security_score_above_max() {
        assert_eq!(derive_security_score(200), MAX_SECURITY_SCORE);
    }

    #[test]
    fn test_health_warning_status_range() {
        let mut contract = build_dummy_contract();
        contract.is_verified = false;
        // Unverified: -40. Base 100 -> 60 => Warning (50..79)
        let health = calculate_health(&contract, None, VerificationLevel::Unverified);
        assert_eq!(health.status, "warning");
    }

    #[tokio::test]
    async fn test_health_monitor_status_default() {
        let status = HealthMonitorStatus::default();
        let snapshot = status.snapshot().await;
        assert!(!snapshot.running);
        assert!(snapshot.last_run.is_none());
        assert!(snapshot.last_success.is_none());
        assert!(snapshot.last_error.is_none());
        assert_eq!(snapshot.total_runs, 0);
        assert_eq!(snapshot.total_failures, 0);
        assert_eq!(snapshot.contracts_checked, 0);
        assert_eq!(snapshot.contracts_failed, 0);
    }

    #[tokio::test]
    async fn test_health_monitor_status_tracks_runs() {
        let status = HealthMonitorStatus::default();
        status.total_runs.fetch_add(1, Ordering::Relaxed);
        *status.last_run.write().await = Some(Utc::now());
        *status.last_success.write().await = Some(Utc::now());

        let snapshot = status.snapshot().await;
        assert!(snapshot.running);
        assert!(snapshot.last_run.is_some());
        assert!(snapshot.last_success.is_some());
        assert_eq!(snapshot.total_runs, 1);
    }

    #[tokio::test]
    async fn test_health_monitor_status_tracks_errors() {
        let status = HealthMonitorStatus::default();
        status.total_failures.fetch_add(1, Ordering::Relaxed);
        *status.last_error.write().await = Some("test error".to_string());
        *status.last_run.write().await = Some(Utc::now());

        let snapshot = status.snapshot().await;
        assert_eq!(snapshot.total_failures, 1);
        assert_eq!(snapshot.last_error, Some("test error".to_string()));
    }
}
