use crate::validation::extractors::ValidatedJson;
use axum::{extract::State, http::StatusCode, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::Row;

use crate::{error::ApiError, state::AppState};

const MAX_MESSAGE_LEN: usize = 2_000;
const MAX_STACK_LEN: usize = 12_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ErrorReportRequest {
    pub source: Option<String>,
    pub category: Option<String>,
    pub severity: Option<String>,
    pub message: String,
    pub stack_trace: Option<String>,
    pub route: Option<String>,
    pub request_id: Option<String>,
    pub user_agent: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Serialize)]
pub struct ErrorReportResponse {
    pub id: String,
    pub stored: bool,
    pub severity: String,
    pub category: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorDashboardResponse {
    pub generated_at: DateTime<Utc>,
    pub last_24h_total: i64,
    pub by_severity: Vec<CountBucket>,
    pub by_category: Vec<CountBucket>,
    pub recent_critical: Vec<RecentError>,
}

#[derive(Debug, Serialize)]
pub struct CountBucket {
    pub key: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct RecentError {
    pub id: String,
    pub source: String,
    pub category: String,
    pub severity: String,
    pub message: String,
    pub route: Option<String>,
    pub request_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub async fn report_error(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<ErrorReportRequest>,
) -> Result<(StatusCode, Json<ErrorReportResponse>), ApiError> {
    let message = truncate(payload.message.trim(), MAX_MESSAGE_LEN);
    if message.is_empty() {
        return Err(ApiError::bad_request(
            "EMPTY_ERROR_MESSAGE",
            "error message cannot be empty",
        ));
    }

    let source = normalize_choice(
        payload.source.as_deref(),
        &["frontend", "backend", "cli"],
        "frontend",
    );
    let category = normalize_choice(
        payload.category.as_deref(),
        &["user", "system", "network", "validation", "unknown"],
        "unknown",
    );
    let severity = normalize_choice(
        payload.severity.as_deref(),
        &["info", "warning", "error", "critical"],
        infer_severity(&category, &message),
    );
    let stack_trace = payload
        .stack_trace
        .map(|stack| truncate(&stack, MAX_STACK_LEN));
    let metadata = sanitize_value(payload.metadata);
    let request_id = payload
        .request_id
        .or_else(crate::request_tracing::current_request_id);

    let row = sqlx::query(
        r#"
        INSERT INTO error_logs (
            source, category, severity, message, stack_trace, route,
            request_id, user_agent, metadata
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id
        "#,
    )
    .bind(&source)
    .bind(&category)
    .bind(&severity)
    .bind(&message)
    .bind(&stack_trace)
    .bind(&payload.route)
    .bind(&request_id)
    .bind(&payload.user_agent)
    .bind(&metadata)
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        tracing::error!(err = %err, "failed_to_store_error_report");
        ApiError::db_error("Failed to store error report")
    })?;

    let id: uuid::Uuid = row.get("id");
    emit_error_signal(&severity, &category, &message, request_id.as_deref());

    Ok((
        StatusCode::CREATED,
        Json(ErrorReportResponse {
            id: id.to_string(),
            stored: true,
            severity,
            category,
        }),
    ))
}

pub async fn error_dashboard(
    State(state): State<AppState>,
) -> Result<Json<ErrorDashboardResponse>, ApiError> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM error_logs WHERE created_at >= NOW() - INTERVAL '24 hours'",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        tracing::error!(err = %err, "failed_to_load_error_total");
        ApiError::db_error("Failed to load error dashboard")
    })?;

    let by_severity = load_buckets(&state, "severity").await?;
    let by_category = load_buckets(&state, "category").await?;
    let recent_critical = load_recent_critical(&state).await?;

    Ok(Json(ErrorDashboardResponse {
        generated_at: Utc::now(),
        last_24h_total: total,
        by_severity,
        by_category,
        recent_critical,
    }))
}

async fn load_buckets(state: &AppState, column: &str) -> Result<Vec<CountBucket>, ApiError> {
    let sql = format!(
        "SELECT {column} AS key, COUNT(*) AS count \
         FROM error_logs \
         WHERE created_at >= NOW() - INTERVAL '24 hours' \
         GROUP BY {column} \
         ORDER BY count DESC"
    );

    let rows = sqlx::query(&sql)
        .fetch_all(&state.db)
        .await
        .map_err(|err| {
            tracing::error!(err = %err, column, "failed_to_load_error_buckets");
            ApiError::db_error("Failed to load error dashboard")
        })?;

    Ok(rows
        .into_iter()
        .map(|row| CountBucket {
            key: row.get::<String, _>("key"),
            count: row.get::<i64, _>("count"),
        })
        .collect())
}

async fn load_recent_critical(state: &AppState) -> Result<Vec<RecentError>, ApiError> {
    let rows = sqlx::query(
        r#"
        SELECT id, source, category, severity, message, route, request_id, created_at
        FROM error_logs
        WHERE severity = 'critical'
        ORDER BY created_at DESC
        LIMIT 10
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        tracing::error!(err = %err, "failed_to_load_recent_critical_errors");
        ApiError::db_error("Failed to load error dashboard")
    })?;

    Ok(rows
        .into_iter()
        .map(|row| RecentError {
            id: row.get::<uuid::Uuid, _>("id").to_string(),
            source: row.get("source"),
            category: row.get("category"),
            severity: row.get("severity"),
            message: row.get("message"),
            route: row.get("route"),
            request_id: row.get("request_id"),
            created_at: row.get("created_at"),
        })
        .collect())
}

fn normalize_choice(raw: Option<&str>, allowed: &[&str], default: &str) -> String {
    let normalized = raw
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| default.to_string());

    if allowed.iter().any(|allowed| *allowed == normalized) {
        normalized
    } else {
        default.to_string()
    }
}

fn infer_severity(category: &str, message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    if category == "system"
        || lower.contains("panic")
        || lower.contains("invariant")
        || lower.contains("data loss")
    {
        "critical"
    } else if category == "validation" || category == "user" {
        "warning"
    } else {
        "error"
    }
}

fn truncate(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        value.to_string()
    } else {
        let boundary = value
            .char_indices()
            .map(|(idx, _)| idx)
            .take_while(|idx| *idx <= max_len)
            .last()
            .unwrap_or(0);
        format!("{}...[truncated]", &value[..boundary])
    }
}

fn sanitize_value(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(sanitize_object(object)),
        Value::Array(values) => Value::Array(values.into_iter().map(sanitize_value).collect()),
        other => other,
    }
}

fn sanitize_object(object: Map<String, Value>) -> Map<String, Value> {
    object
        .into_iter()
        .map(|(key, value)| {
            let sanitized = if is_sensitive_key(&key) {
                Value::String("[REDACTED]".to_string())
            } else {
                sanitize_value(value)
            };
            (key, sanitized)
        })
        .collect()
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "password",
        "secret",
        "token",
        "api_key",
        "authorization",
        "cookie",
    ]
    .iter()
    .any(|needle| key.contains(needle))
}

fn emit_error_signal(severity: &str, category: &str, message: &str, request_id: Option<&str>) {
    let backtrace = ::std::backtrace::Backtrace::capture();
    let backtrace_str = if backtrace.status() == ::std::backtrace::BacktraceStatus::Captured {
        Some(format!("{:#}", backtrace))
    } else {
        None
    };

    match severity {
        "fatal" => {
            tracing::error!(
                severity,
                category,
                request_id,
                message,
                backtrace = %backtrace_str.as_deref().unwrap_or("none"),
                alert = true,
                fatal = true,
                "fatal_error_reported"
            );
            std::process::abort();
        }
        "critical" => tracing::error!(
            severity,
            category,
            request_id,
            message,
            backtrace = %backtrace_str.as_deref().unwrap_or("none"),
            alert = true,
            "critical_error_reported"
        ),
        "error" => tracing::error!(
            severity,
            category,
            request_id,
            message,
            backtrace = %backtrace_str.as_deref().unwrap_or("none"),
            "error_reported"
        ),
        "warning" => tracing::warn!(severity, category, request_id, message, "error_reported"),
        _ => tracing::info!(severity, category, request_id, message, "error_reported"),
    }
}
