// Issue #879: Data partitioning strategy for contract tables.
//
// Manages the lifecycle of declarative PostgreSQL partitions created by the
// migration.  A background task pre-creates the next month's interaction
// partition and the next year's audit-log partition so queries never fall
// into the DEFAULT catch-all.

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Duration as StdDuration;

use crate::error::ApiError;
use crate::state::AppState;

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PartitionInfo {
    pub id: i64,
    pub parent_table: String,
    pub partition_name: String,
    pub partition_key: String,
    pub range_start: Option<DateTime<Utc>>,
    pub range_end: Option<DateTime<Utc>>,
    pub list_value: Option<String>,
    pub row_count: Option<i64>,
    pub size_bytes: Option<i64>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionStatus {
    pub total_partitions: i64,
    pub active_partitions: i64,
    pub archived_partitions: i64,
    pub tables: Vec<TablePartitionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TablePartitionSummary {
    pub parent_table: String,
    pub partition_count: i64,
    pub oldest_partition: Option<DateTime<Utc>>,
    pub newest_partition: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePartitionRequest {
    pub parent_table: String,
    pub partition_name: String,
    pub range_start: Option<DateTime<Utc>>,
    pub range_end: Option<DateTime<Utc>>,
    pub list_value: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/admin/partitions
pub async fn list_partitions(
    State(state): State<AppState>,
) -> Result<Json<Vec<PartitionInfo>>, ApiError> {
    let rows = sqlx::query_as::<_, PartitionInfo>(
        r#"
        SELECT
            id, parent_table, partition_name, partition_key,
            range_start, range_end, list_value,
            row_count, size_bytes, status, created_at, archived_at
        FROM partition_registry
        ORDER BY parent_table, created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("PARTITION_LIST_ERROR", e.to_string()))?;

    Ok(Json(rows))
}

/// GET /api/admin/partitions/status
pub async fn get_partition_status(
    State(state): State<AppState>,
) -> Result<Json<PartitionStatus>, ApiError> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM partition_registry")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    let active: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM partition_registry WHERE status = 'active'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let archived: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM partition_registry WHERE status = 'archived'")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let table_rows = sqlx::query(
        r#"
        SELECT
            parent_table,
            COUNT(*)        AS partition_count,
            MIN(range_start) AS oldest_partition,
            MAX(range_start) AS newest_partition
        FROM partition_registry
        WHERE status = 'active'
        GROUP BY parent_table
        ORDER BY parent_table
        "#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    use sqlx::Row as _;
    let tables = table_rows
        .into_iter()
        .map(|r| TablePartitionSummary {
            parent_table: r.try_get("parent_table").unwrap_or_default(),
            partition_count: r.try_get("partition_count").unwrap_or(0),
            oldest_partition: r.try_get("oldest_partition").ok(),
            newest_partition: r.try_get("newest_partition").ok(),
        })
        .collect();

    Ok(Json(PartitionStatus {
        total_partitions: total,
        active_partitions: active,
        archived_partitions: archived,
        tables,
    }))
}

/// POST /api/admin/partitions/create
/// Creates a new partition for interactions_partitioned or audit_logs_partitioned.
pub async fn create_partition(
    State(state): State<AppState>,
    Json(req): Json<CreatePartitionRequest>,
) -> Result<Json<PartitionInfo>, ApiError> {
    let allowed_parents = ["interactions_partitioned", "audit_logs_partitioned"];
    if !allowed_parents.contains(&req.parent_table.as_str()) {
        return Err(ApiError::bad_request(
            "INVALID_PARENT",
            format!(
                "parent_table must be one of: {}",
                allowed_parents.join(", ")
            ),
        ));
    }

    // Build the CREATE TABLE … PARTITION OF DDL
    let ddl = match (&req.range_start, &req.range_end, &req.list_value) {
        (Some(start), Some(end), _) => format!(
            "CREATE TABLE IF NOT EXISTS {} PARTITION OF {} FOR VALUES FROM ('{}') TO ('{}')",
            req.partition_name,
            req.parent_table,
            start.to_rfc3339(),
            end.to_rfc3339(),
        ),
        (_, _, Some(val)) => format!(
            "CREATE TABLE IF NOT EXISTS {} PARTITION OF {} FOR VALUES IN ('{}')",
            req.partition_name, req.parent_table, val,
        ),
        _ => {
            return Err(ApiError::bad_request(
                "INVALID_PARTITION_SPEC",
                "Provide either range_start+range_end or list_value",
            ))
        }
    };

    sqlx::query(&ddl)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal_error("PARTITION_CREATE_ERROR", e.to_string()))?;

    // Register in the catalogue
    let row = sqlx::query_as::<_, PartitionInfo>(
        r#"
        INSERT INTO partition_registry (
            parent_table, partition_name, partition_key,
            range_start, range_end, list_value, status
        )
        VALUES ($1, $2, 'created_at', $3, $4, $5, 'active')
        RETURNING
            id, parent_table, partition_name, partition_key,
            range_start, range_end, list_value,
            row_count, size_bytes, status, created_at, archived_at
        "#,
    )
    .bind(&req.parent_table)
    .bind(&req.partition_name)
    .bind(req.range_start)
    .bind(req.range_end)
    .bind(&req.list_value)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("PARTITION_REGISTER_ERROR", e.to_string()))?;

    tracing::info!(
        partition = %req.partition_name,
        parent = %req.parent_table,
        "Partition created"
    );

    Ok(Json(row))
}

/// DELETE /api/admin/partitions/:name
/// Marks a partition as archived and detaches it from the parent table.
pub async fn archive_partition(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Detach the partition so the parent no longer routes to it
    let info = sqlx::query_as::<_, PartitionInfo>(
        "SELECT id, parent_table, partition_name, partition_key, range_start, range_end, list_value, row_count, size_bytes, status, created_at, archived_at FROM partition_registry WHERE partition_name = $1",
    )
    .bind(&name)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("PARTITION_FETCH_ERROR", e.to_string()))?
    .ok_or_else(|| ApiError::not_found("PARTITION_NOT_FOUND", format!("Partition '{name}' not found")))?;

    let ddl = format!(
        "ALTER TABLE {} DETACH PARTITION {}",
        info.parent_table, info.partition_name
    );
    sqlx::query(&ddl)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal_error("PARTITION_DETACH_ERROR", e.to_string()))?;

    sqlx::query(
        "UPDATE partition_registry SET status = 'archived', archived_at = NOW() WHERE partition_name = $1",
    )
    .bind(&name)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("PARTITION_ARCHIVE_ERROR", e.to_string()))?;

    tracing::info!(partition = %name, "Partition archived and detached");

    Ok(Json(serde_json::json!({
        "partition": name,
        "status": "archived"
    })))
}

// ── Background lifecycle task ─────────────────────────────────────────────────

/// Spawns a task that pre-creates the next monthly interaction partition and
/// the next yearly audit-log partition if they do not already exist.
pub fn spawn_partition_manager_task(pool: PgPool) {
    tokio::spawn(async move {
        // Check once per hour
        let mut interval = tokio::time::interval(StdDuration::from_secs(3600));
        loop {
            interval.tick().await;
            ensure_upcoming_interaction_partition(&pool).await;
            ensure_upcoming_audit_log_partition(&pool).await;
        }
    });
}

async fn ensure_upcoming_interaction_partition(pool: &PgPool) {
    let next_month_start = {
        let now = Utc::now();
        let y = if now.month() == 12 {
            now.year() + 1
        } else {
            now.year()
        };
        let m = if now.month() == 12 {
            1
        } else {
            now.month() + 1
        };
        DateTime::<Utc>::from_naive_utc_and_offset(
            chrono::NaiveDate::from_ymd_opt(y, m, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            Utc,
        )
    };
    let next_month_end = next_month_start + Duration::days(32);
    let next_month_end = DateTime::<Utc>::from_naive_utc_and_offset(
        chrono::NaiveDate::from_ymd_opt(next_month_end.year(), next_month_end.month(), 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
        Utc,
    );
    let partition_name = format!(
        "interactions_p_{:04}_{:02}",
        next_month_start.year(),
        next_month_start.month()
    );

    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_class WHERE relname = $1)")
            .bind(&partition_name)
            .fetch_one(pool)
            .await
            .unwrap_or(false);

    if exists {
        return;
    }

    let ddl = format!(
        "CREATE TABLE IF NOT EXISTS {} PARTITION OF interactions_partitioned FOR VALUES FROM ('{}') TO ('{}')",
        partition_name,
        next_month_start.to_rfc3339(),
        next_month_end.to_rfc3339(),
    );

    if let Err(e) = sqlx::query(&ddl).execute(pool).await {
        tracing::warn!(error = %e, partition = %partition_name, "Failed to auto-create interaction partition");
        return;
    }

    let _ = sqlx::query(
        r#"
        INSERT INTO partition_registry (parent_table, partition_name, partition_key, range_start, range_end, status)
        VALUES ('interactions_partitioned', $1, 'created_at', $2, $3, 'active')
        ON CONFLICT (partition_name) DO NOTHING
        "#,
    )
    .bind(&partition_name)
    .bind(next_month_start)
    .bind(next_month_end)
    .execute(pool)
    .await;

    tracing::info!(partition = %partition_name, "Auto-created upcoming interaction partition");
}

async fn ensure_upcoming_audit_log_partition(pool: &PgPool) {
    let next_year = Utc::now().year() + 1;
    let partition_name = format!("audit_logs_p_{next_year:04}");

    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_class WHERE relname = $1)")
            .bind(&partition_name)
            .fetch_one(pool)
            .await
            .unwrap_or(false);

    if exists {
        return;
    }

    let year_start = DateTime::<Utc>::from_naive_utc_and_offset(
        chrono::NaiveDate::from_ymd_opt(next_year, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
        Utc,
    );
    let year_end = DateTime::<Utc>::from_naive_utc_and_offset(
        chrono::NaiveDate::from_ymd_opt(next_year + 1, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap(),
        Utc,
    );

    let ddl = format!(
        "CREATE TABLE IF NOT EXISTS {} PARTITION OF audit_logs_partitioned FOR VALUES FROM ('{}') TO ('{}')",
        partition_name,
        year_start.to_rfc3339(),
        year_end.to_rfc3339(),
    );

    if let Err(e) = sqlx::query(&ddl).execute(pool).await {
        tracing::warn!(error = %e, partition = %partition_name, "Failed to auto-create audit-log partition");
        return;
    }

    let _ = sqlx::query(
        r#"
        INSERT INTO partition_registry (parent_table, partition_name, partition_key, range_start, range_end, status)
        VALUES ('audit_logs_partitioned', $1, 'created_at', $2, $3, 'active')
        ON CONFLICT (partition_name) DO NOTHING
        "#,
    )
    .bind(&partition_name)
    .bind(year_start)
    .bind(year_end)
    .execute(pool)
    .await;

    tracing::info!(partition = %partition_name, "Auto-created upcoming audit-log partition");
}
