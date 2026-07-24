// incident_handlers.rs
// Security incident tracking system (Issue #504).
//
// Provides full lifecycle management for security incidents:
//   report → investigate → mitigate → resolve → close
// Linked to affected contracts, with timeline updates, advisory publishing,
// user notifications, and report generation.

use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    state::AppState,
};

// ─────────────────────────────────────────────────────────────────────────────
// Enums
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "incident_severity", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IncidentSeverity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "incident_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum IncidentStatus {
    Reported,
    Investigating,
    Mitigating,
    Resolved,
    Closed,
}

impl std::fmt::Display for IncidentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reported => write!(f, "reported"),
            Self::Investigating => write!(f, "investigating"),
            Self::Mitigating => write!(f, "mitigating"),
            Self::Resolved => write!(f, "resolved"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Database row types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SecurityIncident {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub severity: IncidentSeverity,
    pub status: IncidentStatus,
    pub reporter: String,
    pub assigned_to: Option<String>,
    pub cve_id: Option<String>,
    pub reported_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIncidentDetail {
    #[serde(flatten)]
    pub incident: SecurityIncident,
    pub affected_contracts: Vec<Uuid>,
    pub updates: Vec<IncidentUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct IncidentUpdate {
    pub id: Uuid,
    pub incident_id: Uuid,
    pub author: String,
    pub message: String,
    pub status_change: Option<IncidentStatus>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SecurityAdvisory {
    pub id: Uuid,
    pub incident_id: Option<Uuid>,
    pub title: String,
    pub summary: String,
    pub details: String,
    pub severity: IncidentSeverity,
    pub affected_versions: Option<String>,
    pub mitigation: Option<String>,
    pub published_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Request / response types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ReportIncidentRequest {
    pub title: String,
    pub description: String,
    pub severity: IncidentSeverity,
    pub reporter: String,
    /// UUIDs of contracts known to be affected at report time (may be empty).
    #[serde(default)]
    pub affected_contract_ids: Vec<Uuid>,
    pub assigned_to: Option<String>,
    pub cve_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIncidentStatusRequest {
    pub status: IncidentStatus,
    pub author: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct AddIncidentUpdateRequest {
    pub author: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct AddAffectedContractRequest {
    pub contract_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct PublishAdvisoryRequest {
    pub incident_id: Option<Uuid>,
    pub title: String,
    pub summary: String,
    pub details: String,
    pub severity: IncidentSeverity,
    pub affected_versions: Option<String>,
    pub mitigation: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NotifyAffectedUsersRequest {
    pub channel: Option<String>,
    pub message_template: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NotifyAffectedUsersResponse {
    pub incident_id: Uuid,
    pub notifications_sent: usize,
    pub recipients: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct IncidentReport {
    pub generated_at: DateTime<Utc>,
    pub total_incidents: i64,
    pub by_severity: SeverityBreakdown,
    pub by_status: StatusBreakdown,
    pub open_incidents: Vec<SecurityIncident>,
    pub recent_advisories: Vec<SecurityAdvisory>,
    pub mean_time_to_resolve_hours: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SeverityBreakdown {
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
}

#[derive(Debug, Serialize)]
pub struct StatusBreakdown {
    pub reported: i64,
    pub investigating: i64,
    pub mitigating: i64,
    pub resolved: i64,
    pub closed: i64,
}

#[derive(Debug, Deserialize)]
pub struct ListIncidentsQuery {
    pub severity: Option<IncidentSeverity>,
    pub status: Option<IncidentStatus>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ListIncidentsResponse {
    pub incidents: Vec<SecurityIncident>,
    pub total: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// POST /api/security/incidents
///
/// Report a new security incident and link it to affected contracts.
pub async fn report_incident(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<ReportIncidentRequest>,
) -> ApiResult<(StatusCode, Json<SecurityIncidentDetail>)> {
    if req.title.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidTitle",
            "title must not be empty",
        ));
    }
    if req.description.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidDescription",
            "description must not be empty",
        ));
    }
    if req.reporter.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidReporter",
            "reporter must not be empty",
        ));
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| ApiError::internal(format!("begin transaction: {}", e)))?;

    let incident: SecurityIncident = sqlx::query_as(
        "INSERT INTO security_incidents \
            (title, description, severity, reporter, assigned_to, cve_id) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING *",
    )
    .bind(req.title.trim())
    .bind(req.description.trim())
    .bind(&req.severity)
    .bind(req.reporter.trim())
    .bind(&req.assigned_to)
    .bind(&req.cve_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(format!("insert incident: {}", e)))?;

    // Link affected contracts
    for contract_id in &req.affected_contract_ids {
        sqlx::query(
            "INSERT INTO incident_affected_contracts (incident_id, contract_id) \
             VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(incident.id)
        .bind(contract_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::internal(format!("link contract: {}", e)))?;
    }

    // Record the initial status as a timeline entry
    sqlx::query(
        "INSERT INTO incident_updates (incident_id, author, message, status_change) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(incident.id)
    .bind(req.reporter.trim())
    .bind("Incident reported.")
    .bind(IncidentStatus::Reported)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(format!("insert initial update: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::internal(format!("commit: {}", e)))?;

    let detail = build_incident_detail(&state, &incident).await?;
    Ok((StatusCode::CREATED, Json(detail)))
}

/// GET /api/security/incidents
///
/// List incidents with optional severity/status filters and pagination.
pub async fn list_incidents(
    State(state): State<AppState>,
    Query(params): Query<ListIncidentsQuery>,
) -> ApiResult<Json<ListIncidentsResponse>> {
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);

    // Dynamic filter building
    let (where_clause, severity_bind, status_bind) = match (&params.severity, &params.status) {
        (Some(_), Some(_)) => ("WHERE severity = $1 AND status = $2", true, true),
        (Some(_), None) => ("WHERE severity = $1", true, false),
        (None, Some(_)) => ("WHERE status = $1", false, true),
        (None, None) => ("", false, false),
    };

    // Build and execute count query
    let count_sql = format!("SELECT COUNT(*) FROM security_incidents {}", where_clause);
    let total: i64 = match (&params.severity, &params.status) {
        (Some(sev), Some(st)) => sqlx::query_scalar(&count_sql)
            .bind(sev)
            .bind(st)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("count: {}", e)))?,
        (Some(sev), None) => sqlx::query_scalar(&count_sql)
            .bind(sev)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("count: {}", e)))?,
        (None, Some(st)) => sqlx::query_scalar(&count_sql)
            .bind(st)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("count: {}", e)))?,
        (None, None) => sqlx::query_scalar(&count_sql)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("count: {}", e)))?,
    };

    let param_offset = if severity_bind && status_bind {
        3
    } else if severity_bind || status_bind {
        2
    } else {
        1
    };
    let list_sql = format!(
        "SELECT * FROM security_incidents {} \
         ORDER BY reported_at DESC LIMIT ${} OFFSET ${}",
        where_clause,
        param_offset,
        param_offset + 1,
    );

    let incidents: Vec<SecurityIncident> = match (&params.severity, &params.status) {
        (Some(sev), Some(st)) => sqlx::query_as(&list_sql)
            .bind(sev)
            .bind(st)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("list: {}", e)))?,
        (Some(sev), None) => sqlx::query_as(&list_sql)
            .bind(sev)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("list: {}", e)))?,
        (None, Some(st)) => sqlx::query_as(&list_sql)
            .bind(st)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("list: {}", e)))?,
        (None, None) => sqlx::query_as(&list_sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("list: {}", e)))?,
    };

    Ok(Json(ListIncidentsResponse { incidents, total }))
}

/// GET /api/security/incidents/:id
pub async fn get_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SecurityIncidentDetail>> {
    let incident = fetch_incident(&state, id).await?;
    let detail = build_incident_detail(&state, &incident).await?;
    Ok(Json(detail))
}

/// PATCH /api/security/incidents/:id/status
///
/// Transition incident status and record a timeline entry.
pub async fn update_incident_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<UpdateIncidentStatusRequest>,
) -> ApiResult<Json<SecurityIncidentDetail>> {
    if req.author.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidAuthor",
            "author must not be empty",
        ));
    }
    if req.message.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidMessage",
            "message must not be empty",
        ));
    }

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| ApiError::internal(format!("begin tx: {}", e)))?;

    let resolved_at: Option<DateTime<Utc>> =
        if req.status == IncidentStatus::Resolved || req.status == IncidentStatus::Closed {
            Some(Utc::now())
        } else {
            None
        };

    let incident: SecurityIncident = sqlx::query_as(
        "UPDATE security_incidents \
         SET status = $1, resolved_at = COALESCE(resolved_at, $2), updated_at = NOW() \
         WHERE id = $3 \
         RETURNING *",
    )
    .bind(&req.status)
    .bind(resolved_at)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(format!("update status: {}", e)))?
    .ok_or_else(|| ApiError::not_found("IncidentNotFound", format!("Incident {} not found", id)))?;

    sqlx::query(
        "INSERT INTO incident_updates (incident_id, author, message, status_change) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(req.author.trim())
    .bind(req.message.trim())
    .bind(&req.status)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(format!("insert update: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::internal(format!("commit: {}", e)))?;

    let detail = build_incident_detail(&state, &incident).await?;
    Ok(Json(detail))
}

/// POST /api/security/incidents/:id/updates
///
/// Append a timeline comment without changing status.
pub async fn add_incident_update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<AddIncidentUpdateRequest>,
) -> ApiResult<Json<IncidentUpdate>> {
    if req.author.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidAuthor",
            "author must not be empty",
        ));
    }
    if req.message.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidMessage",
            "message must not be empty",
        ));
    }
    // Verify incident exists
    fetch_incident(&state, id).await?;

    let update: IncidentUpdate = sqlx::query_as(
        "INSERT INTO incident_updates (incident_id, author, message) \
         VALUES ($1, $2, $3) \
         RETURNING *",
    )
    .bind(id)
    .bind(req.author.trim())
    .bind(req.message.trim())
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("insert update: {}", e)))?;

    Ok(Json(update))
}

/// POST /api/security/incidents/:id/contracts
///
/// Link an additional contract to an existing incident.
pub async fn add_affected_contract(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<AddAffectedContractRequest>,
) -> ApiResult<StatusCode> {
    fetch_incident(&state, id).await?;

    sqlx::query(
        "INSERT INTO incident_affected_contracts (incident_id, contract_id) \
         VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(req.contract_id)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("link contract: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/contracts/:id/security-incidents
///
/// Return all incidents that affect a given contract.
pub async fn get_contract_incidents(
    State(state): State<AppState>,
    Path(contract_id): Path<String>,
) -> ApiResult<Json<Vec<SecurityIncident>>> {
    let contract_uuid = resolve_contract_uuid(&state, &contract_id).await?;

    let incidents: Vec<SecurityIncident> = sqlx::query_as(
        "SELECT si.* FROM security_incidents si \
         JOIN incident_affected_contracts iac ON iac.incident_id = si.id \
         WHERE iac.contract_id = $1 \
         ORDER BY si.reported_at DESC",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("fetch incidents: {}", e)))?;

    Ok(Json(incidents))
}

/// POST /api/security/advisories
///
/// Publish a security advisory (optionally linked to an incident).
pub async fn publish_advisory(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<PublishAdvisoryRequest>,
) -> ApiResult<(StatusCode, Json<SecurityAdvisory>)> {
    if req.title.trim().is_empty() {
        return Err(ApiError::bad_request(
            "InvalidTitle",
            "title must not be empty",
        ));
    }

    let advisory: SecurityAdvisory = sqlx::query_as(
        "INSERT INTO security_advisories \
            (incident_id, title, summary, details, severity, affected_versions, mitigation) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) \
         RETURNING *",
    )
    .bind(req.incident_id)
    .bind(req.title.trim())
    .bind(req.summary.trim())
    .bind(req.details.trim())
    .bind(&req.severity)
    .bind(&req.affected_versions)
    .bind(&req.mitigation)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("insert advisory: {}", e)))?;

    Ok((StatusCode::CREATED, Json(advisory)))
}

/// GET /api/security/advisories
///
/// List published advisories, most recent first.
pub async fn list_advisories(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<SecurityAdvisory>>> {
    let advisories: Vec<SecurityAdvisory> =
        sqlx::query_as("SELECT * FROM security_advisories ORDER BY published_at DESC LIMIT 100")
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("list advisories: {}", e)))?;

    Ok(Json(advisories))
}

/// GET /api/security/advisories/:id
pub async fn get_advisory(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<SecurityAdvisory>> {
    let advisory: Option<SecurityAdvisory> =
        sqlx::query_as("SELECT * FROM security_advisories WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("fetch advisory: {}", e)))?;

    advisory.map(Json).ok_or_else(|| {
        ApiError::not_found("AdvisoryNotFound", format!("Advisory {} not found", id))
    })
}

/// POST /api/security/incidents/:id/notify
///
/// Notify all users who have interacted with the affected contracts about this incident.
/// Looks up unique user addresses from `contract_interactions` for the affected contracts,
/// then logs a notification entry per recipient.
pub async fn notify_affected_users(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<NotifyAffectedUsersRequest>,
) -> ApiResult<Json<NotifyAffectedUsersResponse>> {
    let incident = fetch_incident(&state, id).await?;

    // Gather affected contract UUIDs
    let affected: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT contract_id FROM incident_affected_contracts WHERE incident_id = $1",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("fetch affected contracts: {}", e)))?;

    if affected.is_empty() {
        return Ok(Json(NotifyAffectedUsersResponse {
            incident_id: id,
            notifications_sent: 0,
            recipients: vec![],
        }));
    }

    let contract_ids: Vec<Uuid> = affected.into_iter().map(|(cid,)| cid).collect();

    // Collect distinct user addresses from interactions with those contracts
    // (also include publisher stellar addresses as they own the contracts)
    let mut recipients: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT ci.user_address \
         FROM contract_interactions ci \
         WHERE ci.contract_id = ANY($1) AND ci.user_address IS NOT NULL",
    )
    .bind(&contract_ids)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("fetch users: {}", e)))?;

    // Also include publishers
    let publishers: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT p.stellar_address \
         FROM publishers p \
         JOIN contracts c ON c.publisher_id = p.id \
         WHERE c.id = ANY($1)",
    )
    .bind(&contract_ids)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("fetch publishers: {}", e)))?;

    for addr in publishers {
        if !recipients.contains(&addr) {
            recipients.push(addr);
        }
    }

    let channel = req.channel.as_deref().unwrap_or("in_app").to_owned();

    let default_template = format!(
        "Security incident reported: [{}] {} — {}",
        incident.severity.label(),
        incident.title,
        incident.description.chars().take(120).collect::<String>(),
    );
    let message = req.message_template.unwrap_or(default_template);

    // Log notification per recipient per affected contract
    let mut sent = 0usize;
    for recipient in &recipients {
        for contract_id in &contract_ids {
            sqlx::query(
                "INSERT INTO incident_notification_log \
                    (incident_id, contract_id, recipient, channel, message) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(id)
            .bind(contract_id)
            .bind(recipient)
            .bind(&channel)
            .bind(&message)
            .execute(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("log notification: {}", e)))?;
        }
        sent += 1;
        tracing::info!(
            incident_id = %id,
            recipient = %recipient,
            channel = %channel,
            "Security incident notification sent",
        );
    }

    Ok(Json(NotifyAffectedUsersResponse {
        incident_id: id,
        notifications_sent: sent,
        recipients,
    }))
}

/// GET /api/security/report
///
/// Generate an aggregate security report: counts by severity/status, open incidents,
/// recent advisories, and mean time to resolve.
pub async fn get_security_report(State(state): State<AppState>) -> ApiResult<Json<IncidentReport>> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM security_incidents")
        .fetch_one(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("count: {}", e)))?;

    let by_severity = count_by_severity(&state).await?;
    let by_status = count_by_status(&state).await?;

    let open_incidents: Vec<SecurityIncident> = sqlx::query_as(
        "SELECT * FROM security_incidents \
         WHERE status NOT IN ('resolved', 'closed') \
         ORDER BY \
           CASE severity \
             WHEN 'critical' THEN 1 \
             WHEN 'high'     THEN 2 \
             WHEN 'medium'   THEN 3 \
             ELSE 4 \
           END, reported_at DESC \
         LIMIT 20",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("open incidents: {}", e)))?;

    let recent_advisories: Vec<SecurityAdvisory> =
        sqlx::query_as("SELECT * FROM security_advisories ORDER BY published_at DESC LIMIT 10")
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("advisories: {}", e)))?;

    // MTTR: average hours from reported_at to resolved_at for resolved/closed incidents
    let mttr: Option<f64> = sqlx::query_scalar(
        "SELECT AVG(EXTRACT(EPOCH FROM (resolved_at - reported_at)) / 3600.0) \
         FROM security_incidents \
         WHERE resolved_at IS NOT NULL",
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("mttr: {}", e)))?;

    Ok(Json(IncidentReport {
        generated_at: Utc::now(),
        total_incidents: total,
        by_severity,
        by_status,
        open_incidents,
        recent_advisories,
        mean_time_to_resolve_hours: mttr,
    }))
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

async fn fetch_incident(state: &AppState, id: Uuid) -> ApiResult<SecurityIncident> {
    sqlx::query_as("SELECT * FROM security_incidents WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("fetch incident: {}", e)))?
        .ok_or_else(|| {
            ApiError::not_found("IncidentNotFound", format!("Incident {} not found", id))
        })
}

async fn build_incident_detail(
    state: &AppState,
    incident: &SecurityIncident,
) -> ApiResult<SecurityIncidentDetail> {
    let affected: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT contract_id FROM incident_affected_contracts WHERE incident_id = $1",
    )
    .bind(incident.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("fetch affected: {}", e)))?;

    let updates: Vec<IncidentUpdate> = sqlx::query_as(
        "SELECT * FROM incident_updates WHERE incident_id = $1 ORDER BY created_at ASC",
    )
    .bind(incident.id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("fetch updates: {}", e)))?;

    Ok(SecurityIncidentDetail {
        incident: incident.clone(),
        affected_contracts: affected.into_iter().map(|(id,)| id).collect(),
        updates,
    })
}

async fn count_by_severity(state: &AppState) -> ApiResult<SeverityBreakdown> {
    #[derive(FromRow)]
    struct Row {
        severity: IncidentSeverity,
        count: i64,
    }
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT severity, COUNT(*) AS count FROM security_incidents GROUP BY severity",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("severity counts: {}", e)))?;

    let mut b = SeverityBreakdown {
        critical: 0,
        high: 0,
        medium: 0,
        low: 0,
    };
    for row in rows {
        match row.severity {
            IncidentSeverity::Critical => b.critical = row.count,
            IncidentSeverity::High => b.high = row.count,
            IncidentSeverity::Medium => b.medium = row.count,
            IncidentSeverity::Low => b.low = row.count,
        }
    }
    Ok(b)
}

async fn count_by_status(state: &AppState) -> ApiResult<StatusBreakdown> {
    #[derive(FromRow)]
    struct Row {
        status: IncidentStatus,
        count: i64,
    }
    let rows: Vec<Row> =
        sqlx::query_as("SELECT status, COUNT(*) AS count FROM security_incidents GROUP BY status")
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("status counts: {}", e)))?;

    let mut b = StatusBreakdown {
        reported: 0,
        investigating: 0,
        mitigating: 0,
        resolved: 0,
        closed: 0,
    };
    for row in rows {
        match row.status {
            IncidentStatus::Reported => b.reported = row.count,
            IncidentStatus::Investigating => b.investigating = row.count,
            IncidentStatus::Mitigating => b.mitigating = row.count,
            IncidentStatus::Resolved => b.resolved = row.count,
            IncidentStatus::Closed => b.closed = row.count,
        }
    }
    Ok(b)
}

async fn resolve_contract_uuid(state: &AppState, id: &str) -> ApiResult<Uuid> {
    if let Ok(uuid) = Uuid::parse_str(id) {
        return Ok(uuid);
    }
    let row: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM contracts WHERE contract_id = $1 LIMIT 1")
            .bind(id)
            .fetch_optional(&state.db)
            .await
            .map_err(|e| ApiError::internal(format!("resolve uuid: {}", e)))?;

    row.map(|(uuid,)| uuid).ok_or_else(|| {
        ApiError::not_found("ContractNotFound", format!("Contract '{}' not found", id))
    })
}

impl IncidentSeverity {
    fn label(&self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
        }
    }
}
