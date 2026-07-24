use chrono::Timelike;
use sqlx::PgPool;
use std::time::Duration;

/// Spawn the background aggregation task.
///
/// Runs every hour:
///   1. Aggregate raw events into daily summaries (yesterday + today).
///   2. Delete raw events older than 90 days.
pub fn spawn_aggregation_task(pool: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));

        loop {
            interval.tick().await;
            tracing::info!("aggregation: starting hourly run");

            if let Err(err) = run_aggregation(&pool).await {
                tracing::error!(error = ?err, "aggregation: run failed");
            }

            if let Err(err) = cleanup_old_events(&pool).await {
                tracing::error!(error = ?err, "aggregation: retention cleanup failed");
            }

            if let Err(err) = run_custom_metrics_aggregation(&pool).await {
                tracing::error!(error = ?err, "aggregation: custom metrics aggregation failed");
            }

            if let Err(err) = run_contract_stats_aggregation(&pool).await {
                tracing::error!(error = ?err, "aggregation: contract stats aggregation failed");
            }

            // Daily contract health score update (runs at 2 AM UTC)
            if chrono::Utc::now().hour() == 2 {
                if let Err(err) = crate::health::update_all_health_scores(&pool).await {
                    tracing::error!(error = ?err, "aggregation: health score update failed");
                }
            }
        }
    });
}

/// Build daily aggregates from raw `analytics_events`.
///
/// Uses `ON CONFLICT … DO UPDATE` so re-running is idempotent.
async fn run_aggregation(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Aggregate events from the last 2 days (yesterday + partial today)
    // to ensure we always capture the freshest data.
    let rows_affected = sqlx::query(
        r#"
        INSERT INTO analytics_daily_aggregates (
            contract_id, date,
            deployment_count, unique_deployers,
            verification_count, publish_count, version_count, update_count,
            total_events, unique_users,
            network_breakdown, top_users
        )
        SELECT
            e.contract_id,
            DATE(e.created_at) AS agg_date,

            -- deployment counts
            COUNT(*) FILTER (WHERE e.event_type = 'contract_deployed') AS deployment_count,
            COUNT(DISTINCT e.user_address) FILTER (WHERE e.event_type = 'contract_deployed') AS unique_deployers,

            -- other event counts
            COUNT(*) FILTER (WHERE e.event_type = 'contract_verified') AS verification_count,
            COUNT(*) FILTER (WHERE e.event_type = 'contract_published') AS publish_count,
            COUNT(*) FILTER (WHERE e.event_type = 'version_created') AS version_count,
            COUNT(*) FILTER (WHERE e.event_type = 'contract_updated') AS update_count,

            -- totals
            COUNT(*) AS total_events,
            COUNT(DISTINCT e.user_address) AS unique_users,

            -- network breakdown as JSON object
            COALESCE(
                jsonb_object_agg(
                    COALESCE(e.network::text, 'unknown'),
                    sub.net_count
                ) FILTER (WHERE sub.net_count IS NOT NULL),
                '{}'::jsonb
            ) AS network_breakdown,

            -- top users as JSON array (top 10)
            COALESCE(
                (
                    SELECT jsonb_agg(
                        jsonb_build_object('address', tu.user_address, 'count', tu.cnt)
                        ORDER BY tu.cnt DESC
                    )
                    FROM (
                        SELECT e2.user_address, COUNT(*) AS cnt
                        FROM analytics_events e2
                        WHERE e2.contract_id = e.contract_id
                          AND DATE(e2.created_at) = DATE(e.created_at)
                          AND e2.user_address IS NOT NULL
                        GROUP BY e2.user_address
                        ORDER BY cnt DESC
                        LIMIT 10
                    ) tu
                ),
                '[]'::jsonb
            ) AS top_users

        FROM analytics_events e
        LEFT JOIN LATERAL (
            SELECT e.network, COUNT(*) AS net_count
            FROM analytics_events e3
            WHERE e3.contract_id = e.contract_id
              AND DATE(e3.created_at) = DATE(e.created_at)
              AND e3.network IS NOT NULL
            GROUP BY e3.network
        ) sub ON true
        WHERE e.created_at >= CURRENT_DATE - INTERVAL '1 day'
        GROUP BY e.contract_id, DATE(e.created_at)

        ON CONFLICT (contract_id, date) DO UPDATE SET
            deployment_count    = EXCLUDED.deployment_count,
            unique_deployers    = EXCLUDED.unique_deployers,
            verification_count  = EXCLUDED.verification_count,
            publish_count       = EXCLUDED.publish_count,
            version_count       = EXCLUDED.version_count,
            update_count        = EXCLUDED.update_count,
            total_events        = EXCLUDED.total_events,
            unique_users        = EXCLUDED.unique_users,
            network_breakdown   = EXCLUDED.network_breakdown,
            top_users           = EXCLUDED.top_users
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();

    tracing::info!(
        rows = rows_affected,
        "aggregation: daily summaries upserted"
    );

    let interaction_rows: i64 =
        sqlx::query_scalar("SELECT refresh_contract_interaction_daily_aggregates(2)")
            .fetch_one(pool)
            .await?;
    tracing::info!(
        interaction_rows,
        "aggregation: contract interaction daily aggregates refreshed"
    );

    Ok(())
}

/// Delete raw analytics events older than 90 days.
async fn cleanup_old_events(pool: &PgPool) -> Result<(), sqlx::Error> {
    let deleted =
        sqlx::query("DELETE FROM analytics_events WHERE created_at < NOW() - INTERVAL '90 days'")
            .execute(pool)
            .await?
            .rows_affected();

    if deleted > 0 {
        tracing::info!(deleted, "aggregation: cleaned up old raw events");
    }

    let archived_interactions: i64 =
        sqlx::query_scalar("SELECT archive_old_contract_interactions(90)")
            .fetch_one(pool)
            .await?;
    if archived_interactions > 0 {
        tracing::info!(
            archived_interactions,
            "aggregation: archived old contract interactions"
        );
    }

    Ok(())
}

/// Aggregate custom contract metrics into hourly and daily rollups.
async fn run_custom_metrics_aggregation(pool: &PgPool) -> Result<(), sqlx::Error> {
    let hourly_rows = sqlx::query(
        r#"
        INSERT INTO contract_custom_metrics_hourly (
            contract_id, metric_name, metric_type,
            bucket_start, bucket_end,
            sample_count,
            sum_value, avg_value, min_value, max_value,
            p50_value, p95_value, p99_value
        )
        SELECT
            contract_id,
            metric_name,
            metric_type,
            date_trunc('hour', timestamp) AS bucket_start,
            date_trunc('hour', timestamp) + INTERVAL '1 hour' AS bucket_end,
            COUNT(*) AS sample_count,
            SUM(value) AS sum_value,
            AVG(value) AS avg_value,
            MIN(value) AS min_value,
            MAX(value) AS max_value,
            percentile_cont(0.50) WITHIN GROUP (ORDER BY value) AS p50_value,
            percentile_cont(0.95) WITHIN GROUP (ORDER BY value) AS p95_value,
            percentile_cont(0.99) WITHIN GROUP (ORDER BY value) AS p99_value
        FROM contract_custom_metrics
        WHERE timestamp >= NOW() - INTERVAL '2 hours'
        GROUP BY contract_id, metric_name, metric_type, date_trunc('hour', timestamp)
        ON CONFLICT (contract_id, metric_name, metric_type, bucket_start) DO UPDATE SET
            bucket_end   = EXCLUDED.bucket_end,
            sample_count = EXCLUDED.sample_count,
            sum_value    = EXCLUDED.sum_value,
            avg_value    = EXCLUDED.avg_value,
            min_value    = EXCLUDED.min_value,
            max_value    = EXCLUDED.max_value,
            p50_value    = EXCLUDED.p50_value,
            p95_value    = EXCLUDED.p95_value,
            p99_value    = EXCLUDED.p99_value,
            updated_at   = NOW()
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();

    let daily_rows = sqlx::query(
        r#"
        INSERT INTO contract_custom_metrics_daily (
            contract_id, metric_name, metric_type,
            bucket_start, bucket_end,
            sample_count,
            sum_value, avg_value, min_value, max_value,
            p50_value, p95_value, p99_value
        )
        SELECT
            contract_id,
            metric_name,
            metric_type,
            date_trunc('day', timestamp) AS bucket_start,
            date_trunc('day', timestamp) + INTERVAL '1 day' AS bucket_end,
            COUNT(*) AS sample_count,
            SUM(value) AS sum_value,
            AVG(value) AS avg_value,
            MIN(value) AS min_value,
            MAX(value) AS max_value,
            percentile_cont(0.50) WITHIN GROUP (ORDER BY value) AS p50_value,
            percentile_cont(0.95) WITHIN GROUP (ORDER BY value) AS p95_value,
            percentile_cont(0.99) WITHIN GROUP (ORDER BY value) AS p99_value
        FROM contract_custom_metrics
        WHERE timestamp >= NOW() - INTERVAL '2 days'
        GROUP BY contract_id, metric_name, metric_type, date_trunc('day', timestamp)
        ON CONFLICT (contract_id, metric_name, metric_type, bucket_start) DO UPDATE SET
            bucket_end   = EXCLUDED.bucket_end,
            sample_count = EXCLUDED.sample_count,
            sum_value    = EXCLUDED.sum_value,
            avg_value    = EXCLUDED.avg_value,
            min_value    = EXCLUDED.min_value,
            max_value    = EXCLUDED.max_value,
            p50_value    = EXCLUDED.p50_value,
            p95_value    = EXCLUDED.p95_value,
            p99_value    = EXCLUDED.p99_value,
            updated_at   = NOW()
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();

    tracing::info!(
        hourly_rows = hourly_rows,
        daily_rows = daily_rows,
        "aggregation: custom metrics rollups updated"
    );

    Ok(())
}

/// Aggregate contract usage stats into the `contract_usage_stats` table and
/// refresh the `trending_contracts_mv` materialized view.
async fn run_contract_stats_aggregation(pool: &PgPool) -> Result<(), sqlx::Error> {
    use chrono::Utc;
    use uuid::Uuid;

    let now = Utc::now();

    // Refresh stats for all contracts across the three standard windows
    for days in &[7i64, 30, 90] {
        let period_start = now - chrono::Duration::days(*days);

        // Upsert stats for every contract
        sqlx::query(
            r#"
            INSERT INTO contract_usage_stats (
                contract_id, period_start, period_end,
                deployment_count, call_count, error_count,
                unique_callers, unique_deployers, total_interactions,
                avg_calls_per_day, error_rate
            )
            SELECT
                c.id,
                $1 AS period_start,
                $2 AS period_end,
                COALESCE(SUM(agg.count) FILTER (WHERE agg.interaction_type = 'deploy'), 0),
                COALESCE(SUM(agg.count) FILTER (WHERE agg.interaction_type IN ('invoke', 'transfer', 'query')), 0),
                COALESCE(SUM(agg.count) FILTER (WHERE agg.interaction_type = 'publish_failed'), 0),
                COUNT(DISTINCT ci.user_address) FILTER (WHERE ci.user_address IS NOT NULL AND ci.interaction_type IN ('invoke', 'transfer', 'query')),
                COUNT(DISTINCT ci.user_address) FILTER (WHERE ci.user_address IS NOT NULL AND ci.interaction_type = 'deploy'),
                COALESCE(SUM(agg.count), 0),
                CASE
                    WHEN COALESCE(SUM(agg.count) FILTER (WHERE agg.interaction_type IN ('invoke', 'transfer', 'query')), 0) > 0
                    THEN COALESCE(SUM(agg.count) FILTER (WHERE agg.interaction_type IN ('invoke', 'transfer', 'query')), 0)::NUMERIC / GREATEST(EXTRACT(EPOCH FROM ($2 - $1)) / 86400.0, 1.0)
                    ELSE 0
                END,
                CASE
                    WHEN COALESCE(SUM(agg.count), 0) > 0
                    THEN COALESCE(SUM(agg.count) FILTER (WHERE agg.interaction_type = 'publish_failed'), 0)::NUMERIC / SUM(agg.count)::NUMERIC
                    ELSE 0
                END
            FROM contracts c
            LEFT JOIN contract_interaction_daily_aggregates agg
                ON agg.contract_id = c.id AND agg.day >= $1::DATE AND agg.day <= $2::DATE
            LEFT JOIN contract_interactions ci
                ON ci.contract_id = c.id
                AND DATE(ci.interaction_timestamp) >= $1::DATE
                AND DATE(ci.interaction_timestamp) <= $2::DATE
                AND ci.interaction_type IN ('invoke', 'transfer', 'query', 'deploy', 'publish_failed')
            GROUP BY c.id
            ON CONFLICT (contract_id, period_start, period_end) DO UPDATE SET
                deployment_count = EXCLUDED.deployment_count,
                call_count = EXCLUDED.call_count,
                error_count = EXCLUDED.error_count,
                unique_callers = EXCLUDED.unique_callers,
                unique_deployers = EXCLUDED.unique_deployers,
                total_interactions = EXCLUDED.total_interactions,
                avg_calls_per_day = EXCLUDED.avg_calls_per_day,
                error_rate = EXCLUDED.error_rate,
                updated_at = NOW()
            "#,
        )
        .bind(period_start)
        .bind(now)
        .execute(pool)
        .await?;

        tracing::debug!(days = days, "stats aggregation: {}d window upserted", days);
    }

    // Refresh trending materialized view
    sqlx::query("SELECT refresh_trending_contracts()")
        .execute(pool)
        .await?;

    tracing::info!("aggregation: contract usage stats and trending view refreshed");

    Ok(())
}
