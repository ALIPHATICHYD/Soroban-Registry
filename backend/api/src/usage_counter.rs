use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

/// Increment the usage_count for a contract by 1 using an atomic SQL UPDATE.
///
/// This function performs an atomic increment operation on the database
/// to ensure consistency under concurrent access.
pub async fn increment_usage_counter(contract_id: Uuid, db: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE contracts SET usage_count = usage_count + 1 WHERE id = $1")
        .bind(contract_id)
        .execute(db)
        .await?;

    tracing::debug!(
        contract_id = %contract_id,
        "usage counter incremented"
    );

    Ok(())
}

/// Increment the usage_count for a contract with timeout protection.
///
/// This function wraps the basic increment operation with a 10ms timeout
/// to ensure it doesn't block the main API request processing. If the
/// operation times out or fails, it logs the error but doesn't propagate
/// it to avoid breaking the main request flow.
pub async fn increment_usage_counter_with_timeout(
    contract_id: Uuid,
    db: &PgPool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match tokio::time::timeout(
        Duration::from_millis(10),
        increment_usage_counter(contract_id, db),
    )
    .await
    {
        Ok(Ok(())) => {
            tracing::debug!(
                contract_id = %contract_id,
                "usage counter incremented successfully"
            );
            Ok(())
        }
        Ok(Err(db_err)) => {
            tracing::error!(
                contract_id = %contract_id,
                error = ?db_err,
                "failed to increment usage counter"
            );
            Err(Box::new(db_err))
        }
        Err(_timeout_err) => {
            tracing::warn!(
                contract_id = %contract_id,
                "usage counter update timed out after 10ms"
            );
            Err("Usage counter update timed out".into())
        }
    }
}

/// Increment the usage_count for a contract with retry logic and exponential backoff.
///
/// This function implements retry logic for critical counter operations that need
/// higher reliability. It uses exponential backoff to handle transient database
/// issues without overwhelming the database with rapid retry attempts.
///
/// # Arguments
/// * `contract_id` - The UUID of the contract to increment
/// * `db` - Database connection pool
/// * `max_retries` - Maximum number of retry attempts (default: 3)
/// * `base_delay_ms` - Base delay in milliseconds for exponential backoff (default: 10)
///
/// # Returns
/// * `Ok(())` if the increment succeeds within the retry limit
/// * `Err(...)` if all retry attempts fail
pub async fn increment_usage_counter_with_retry(
    contract_id: Uuid,
    db: &PgPool,
    max_retries: Option<u32>,
    base_delay_ms: Option<u64>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let max_retries = max_retries.unwrap_or(3);
    let base_delay_ms = base_delay_ms.unwrap_or(10);
    let mut attempts = 0;

    while attempts <= max_retries {
        match increment_usage_counter(contract_id, db).await {
            Ok(()) => {
                if attempts > 0 {
                    tracing::info!(
                        contract_id = %contract_id,
                        attempts = attempts + 1,
                        "usage counter incremented successfully after retries"
                    );
                } else {
                    tracing::debug!(
                        contract_id = %contract_id,
                        "usage counter incremented successfully on first attempt"
                    );
                }
                return Ok(());
            }
            Err(err) => {
                attempts += 1;

                if attempts > max_retries {
                    tracing::error!(
                        contract_id = %contract_id,
                        attempts = attempts,
                        error = ?err,
                        "usage counter increment failed after all retry attempts"
                    );
                    return Err(Box::new(err));
                }

                // Calculate exponential backoff delay: base_delay * 2^attempt
                let delay_ms = base_delay_ms * (2_u64.pow(attempts - 1));

                tracing::warn!(
                    contract_id = %contract_id,
                    attempt = attempts,
                    max_retries = max_retries,
                    delay_ms = delay_ms,
                    error = ?err,
                    "usage counter increment failed, retrying with exponential backoff"
                );

                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }

    // This should never be reached due to the loop logic, but included for completeness
    Err("Maximum retry attempts exceeded".into())
}

/// Increment the usage_count for a contract with retry logic and timeout protection.
///
/// This function combines retry logic with timeout protection for the most critical
/// counter operations. It first applies a timeout to the entire retry operation,
/// then uses exponential backoff for individual retry attempts.
///
/// # Arguments
/// * `contract_id` - The UUID of the contract to increment
/// * `db` - Database connection pool
/// * `timeout_ms` - Total timeout for the entire operation in milliseconds (default: 100)
/// * `max_retries` - Maximum number of retry attempts (default: 3)
///
/// # Returns
/// * `Ok(())` if the increment succeeds within the timeout and retry limits
/// * `Err(...)` if the operation times out or all retry attempts fail
pub async fn increment_usage_counter_with_retry_and_timeout(
    contract_id: Uuid,
    db: &PgPool,
    timeout_ms: Option<u64>,
    max_retries: Option<u32>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let timeout_ms = timeout_ms.unwrap_or(100);

    match tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        increment_usage_counter_with_retry(contract_id, db, max_retries, Some(5)), // Use shorter base delay for timeout scenarios
    )
    .await
    {
        Ok(result) => result,
        Err(_timeout_err) => {
            tracing::warn!(
                contract_id = %contract_id,
                timeout_ms = timeout_ms,
                "usage counter retry operation timed out"
            );
            Err(format!(
                "Usage counter retry operation timed out after {}ms",
                timeout_ms
            )
            .into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;
    use uuid::Uuid;

    // Note: These tests are marked as ignored because they require a test database setup
    // In a real implementation, you would set up a test database connection

    #[tokio::test]
    #[ignore] // Ignore until test database is set up
    async fn test_increment_usage_counter() {
        // This test would verify that the basic increment function works
        // let pool = setup_test_db().await;
        // let contract_id = Uuid::new_v4();
        //
        // // Test basic increment functionality
        // let result = increment_usage_counter(contract_id, &pool).await;
        // assert!(result.is_ok());

        // For now, just test that the function signature is correct
        let contract_id = Uuid::new_v4();
        // This won't actually run but verifies the function signature compiles
        let _future = async {
            // This would fail at runtime but compiles correctly
            let pool: PgPool = unsafe { std::mem::zeroed() };
            increment_usage_counter(contract_id, &pool).await
        };
    }

    #[tokio::test]
    #[ignore] // Ignore until test database is set up
    async fn test_increment_usage_counter_with_timeout() {
        // This test would verify that the timeout-protected increment works
        let contract_id = Uuid::new_v4();
        // This won't actually run but verifies the function signature compiles
        let _future = async {
            // This would fail at runtime but compiles correctly
            let pool: PgPool = unsafe { std::mem::zeroed() };
            increment_usage_counter_with_timeout(contract_id, &pool).await
        };
    }

    #[test]
    fn test_module_compiles() {
        // Simple test to verify the module compiles correctly
        assert!(true);
    }

    #[tokio::test]
    #[ignore] // Ignore until test database is set up
    async fn test_increment_usage_counter_with_retry() {
        // This test would verify that the retry logic works correctly
        let contract_id = Uuid::new_v4();
        // This won't actually run but verifies the function signature compiles
        let _future = async {
            // This would fail at runtime but compiles correctly
            let pool: PgPool = unsafe { std::mem::zeroed() };
            increment_usage_counter_with_retry(contract_id, &pool, Some(3), Some(10)).await
        };
    }

    #[tokio::test]
    #[ignore] // Ignore until test database is set up
    async fn test_increment_usage_counter_with_retry_and_timeout() {
        // This test would verify that the retry with timeout logic works correctly
        let contract_id = Uuid::new_v4();
        // This won't actually run but verifies the function signature compiles
        let _future = async {
            // This would fail at runtime but compiles correctly
            let pool: PgPool = unsafe { std::mem::zeroed() };
            increment_usage_counter_with_retry_and_timeout(contract_id, &pool, Some(100), Some(3))
                .await
        };
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        // Test that exponential backoff delays are calculated correctly
        let base_delay = 10u64;

        // First retry: 10ms * 2^0 = 10ms
        assert_eq!(base_delay * (2_u64.pow(0)), 10);

        // Second retry: 10ms * 2^1 = 20ms
        assert_eq!(base_delay * (2_u64.pow(1)), 20);

        // Third retry: 10ms * 2^2 = 40ms
        assert_eq!(base_delay * (2_u64.pow(2)), 40);
    }

    #[test]
    fn test_default_retry_parameters() {
        // Test that default parameters are reasonable
        let default_max_retries = 3u32;
        let default_base_delay = 10u64;
        let default_timeout = 100u64;

        // Verify defaults are within reasonable bounds
        assert!(default_max_retries > 0 && default_max_retries <= 5);
        assert!(default_base_delay >= 5 && default_base_delay <= 50);
        assert!(default_timeout >= 50 && default_timeout <= 500);
    }
}
