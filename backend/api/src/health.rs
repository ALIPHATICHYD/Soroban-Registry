use crate::error::ApiResult;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Calculates the health score for a single contract.
/// Health score: 0-100 calculated from:
/// - Verification (40%): +40 points if verified
/// - Deployment count (20%): +1 point per deployment (max 20)
/// - Update frequency (20%): +20 (<30d), +10 (<90d), +5 (<180d)
/// - Security issues (10%): Starts at 10, -10 for Critical, -5 for Warning
/// - Abandonment (10%): Starts at 10, -10 if no update in 1+ year
pub async fn calculate_health_score(pool: &PgPool, contract_id: Uuid) -> ApiResult<i32> {
    let mut score = 0;

    // 1. Verification (40 points)
    let is_verified: bool = sqlx::query_scalar("SELECT is_verified FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

    if is_verified {
        score += 40;
    }

    // 2. Deployment count (20 points)
    let deployments: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(deployment_count), 0) FROM analytics_daily_aggregates WHERE contract_id = $1"
    )
    .bind(contract_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    score += (deployments.min(20)) as i32;

    // 3. Update frequency (20 points) & Abandonment (10 points)
    let last_update: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT MAX(created_at) FROM contract_versions WHERE contract_id = $1")
            .bind(contract_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

    let mut activity_score = 10; // Not abandoned by default
    if let Some(last_date) = last_update {
        let age = Utc::now() - last_date;

        if age < Duration::days(30) {
            score += 20;
        } else if age < Duration::days(90) {
            score += 10;
        } else if age < Duration::days(180) {
            score += 5;
        }

        if age > Duration::days(365) {
            activity_score = 0; // Abandoned
        }
    } else {
        // No versions? Default to abandoned if contract exists
        activity_score = 0;
    }
    score += activity_score;

    // 4. Security issues (10 points)
    // Checking both contract_scan_results and security_patches applied/pending
    let security_deduction: i32 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(MAX(
            CASE 
                WHEN v.severity = 'critical' OR v.severity = 'high' THEN 10
                WHEN v.severity = 'medium' OR v.severity = 'low' THEN 5
                ELSE 0
            END
        ), 0)
        FROM contract_scan_results sr
        JOIN cve_vulnerabilities v ON sr.cve_id = v.cve_id
        WHERE sr.contract_id = $1 AND sr.is_false_positive = false
        "#,
    )
    .bind(contract_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let security_base = 10;
    score += (security_base - security_deduction).max(0) as i32;

    Ok(score.clamp(0, 100))
}

/// Updates health scores for all contracts in the registry.
pub async fn update_all_health_scores(pool: &PgPool) -> ApiResult<()> {
    let contract_ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM contracts")
        .fetch_all(pool)
        .await
        .map_err(|e| {
            crate::error::ApiError::internal(format!("Failed to fetch contracts: {}", e))
        })?;

    tracing::info!(
        "Starting health score update for {} contracts",
        contract_ids.len()
    );

    for id in contract_ids {
        match calculate_health_score(pool, id).await {
            Ok(score) => {
                sqlx::query(
                    "UPDATE contracts SET health_score = $1, updated_at = NOW() WHERE id = $2",
                )
                .bind(score)
                .bind(id)
                .execute(pool)
                .await
                .map_err(|e| {
                    crate::error::ApiError::internal(format!(
                        "Failed to update health score for {}: {}",
                        id, e
                    ))
                })?;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to calculate health score for contract {}: {}",
                    id,
                    e
                );
            }
        }
    }

    tracing::info!("Completed health score update");
    Ok(())
}
// TODO: implement readiness health checks
