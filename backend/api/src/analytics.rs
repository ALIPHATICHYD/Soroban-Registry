use shared::{AnalyticsEventType, Network};
use sqlx::PgPool;
use uuid::Uuid;

/// Increment the view_count for a contract by 1 in a background task.
///
/// This is intentionally fire-and-forget so that fetching a contract page
/// is never blocked waiting for a write.  Any database error is logged at
/// the WARN level but never propagated to the caller.
#[allow(dead_code)]
pub fn increment_view_count_async(pool: PgPool, contract_id: Uuid) {
    tokio::spawn(async move {
        let result = sqlx::query("UPDATE contracts SET view_count = view_count + 1 WHERE id = $1")
            .bind(contract_id)
            .execute(&pool)
            .await;

        if let Err(err) = result {
            tracing::warn!(
                contract_id = %contract_id,
                error = ?err,
                "failed to increment view_count"
            );
        }
    });
}

/// Record an analytics event.
///
/// This is intentionally fire-and-forget: callers should log errors but
/// never let a failed analytics insert break the main request flow.
pub async fn record_event(
    pool: &PgPool,
    event_type: AnalyticsEventType,
    contract_id: Option<Uuid>,
    publisher_id: Option<Uuid>,
    user_address: Option<&str>,
    network: Option<&Network>,
    metadata: Option<serde_json::Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO analytics_events (event_type, contract_id, publisher_id, user_address, network, metadata)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(&event_type)
    .bind(contract_id)
    .bind(publisher_id)
    .bind(user_address)
    .bind(network)
    .bind(metadata.unwrap_or(serde_json::json!({})))
    .execute(pool)
    .await?;

    tracing::debug!(
        event = %event_type,
        contract = ?contract_id,
        publisher = ?publisher_id,
        "analytics event recorded"
    );

    Ok(())
}
