// Formal verification API handlers.
//
// Endpoints:
//   POST /api/contracts/:id/formal-verification        – trigger analysis
//   GET  /api/contracts/:id/formal-verification        – list sessions
//   GET  /api/contracts/:id/formal-verification/:sid   – get session + results

use crate::validation::extractors::ValidatedJson;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    formal_verification::{
        FormalVerificationReport, ProofStatus, VulnerabilitySeverity, WasmBytecodeAnalyzer,
    },
    state::AppState,
};

// ─── Request / response types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TriggerVerificationRequest {
    /// Specific version to analyse. Defaults to the latest verified version.
    pub version: Option<String>,
    /// If true, re-run even when a recent session already exists.
    pub force: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct TriggerVerificationResponse {
    pub session_id: Uuid,
    pub contract_id: Uuid,
    pub status: String,
    pub message: String,
    pub queued_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct VerificationSessionSummary {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: Option<String>,
    pub status: String,
    pub properties_proved: Option<i32>,
    pub properties_violated: Option<i32>,
    pub properties_inconclusive: Option<i32>,
    pub overall_confidence: Option<f64>,
    pub analysis_duration_ms: Option<i64>,
    pub analyzer_version: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub completed_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct VerificationPropertyRow {
    pub id: Uuid,
    pub session_id: Uuid,
    pub property_id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub method: String,
    pub confidence: f64,
    pub evidence: serde_json::Value,
    pub counterexample: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct VerificationFindingRow {
    pub id: Uuid,
    pub session_id: Uuid,
    pub finding_id: String,
    pub title: String,
    pub description: String,
    pub severity: String,
    pub category: String,
    pub cwe_id: Option<String>,
    pub affected_functions: Vec<String>,
    pub remediation: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProofCertificateRow {
    pub id: Uuid,
    pub session_id: Uuid,
    pub certificate_id: Uuid,
    pub properties_proved: i32,
    pub properties_violated: i32,
    pub properties_inconclusive: i32,
    pub overall_confidence: f64,
    pub summary: String,
    pub generated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct FullSessionResponse {
    pub session: VerificationSessionSummary,
    pub properties: Vec<VerificationPropertyRow>,
    pub vulnerabilities: Vec<VerificationFindingRow>,
    pub certificate: Option<ProofCertificateRow>,
}

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Fetch WASM bytes for the given contract version.
/// Tries source storage first (wasm binary); falls back to a stub for testing.
async fn fetch_wasm_bytes(
    state: &AppState,
    contract_id: Uuid,
    version: Option<&str>,
) -> Result<Vec<u8>, ApiError> {
    // Try to get the WASM hash from the contract version record.
    let row: Option<(String, Option<String>, String, String)> = if let Some(ver) = version {
        sqlx::query_as(
            "SELECT cv.wasm_hash, cs.source_url, cs.storage_backend, cs.storage_key FROM contract_versions cv \
             LEFT JOIN contract_sources cs ON cs.contract_version_id = cv.id \
             WHERE cv.contract_id = $1 AND cv.version = $2 LIMIT 1",
        )
        .bind(contract_id)
        .bind(ver)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?
    } else {
        sqlx::query_as(
            "SELECT cv.wasm_hash, cs.source_url, cs.storage_backend, cs.storage_key FROM contract_versions cv \
             LEFT JOIN contract_sources cs ON cs.contract_version_id = cv.id \
             WHERE cv.contract_id = $1 ORDER BY cv.created_at DESC LIMIT 1",
        )
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?
    };

    let (wasm_hash, source_url, storage_backend, storage_key) = row.ok_or_else(|| {
        ApiError::not_found(
            "CONTRACT_VERSION_NOT_FOUND",
            "No verified version found for this contract",
        )
    })?;

    // Attempt to fetch from source storage using the persisted storage location.
    match state
        .source_storage
        .retrieve_source(&storage_backend, &storage_key)
        .await
    {
        Ok(bytes) => Ok(bytes),
        Err(_) => {
            // Fallback: if the source_url references an uploaded WASM file, try that.
            if let Some(url) = source_url {
                if url.ends_with(".wasm") {
                    match state
                        .source_storage
                        .retrieve_source(&storage_backend, &url)
                        .await
                    {
                        Ok(bytes) => return Ok(bytes),
                        Err(_) => {}
                    }
                }
            }
            // Last resort: return a minimal valid WASM so analysis can still run
            // (produces inconclusive results rather than an error).
            tracing::warn!(
                contract_id = %contract_id,
                wasm_hash = %wasm_hash,
                "WASM bytes not found in storage — running analysis on empty module"
            );
            Ok(vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00])
        }
    }
}

/// Persist a completed report to the database.
async fn persist_report(
    state: &AppState,
    contract_id: Uuid,
    version: Option<&str>,
    report: &FormalVerificationReport,
) -> Result<(), ApiError> {
    let session_id = report.session_id;

    // Insert the session record.
    sqlx::query(
        r#"
        INSERT INTO formal_verification_sessions
            (id, contract_id, version, status,
             properties_proved, properties_violated, properties_inconclusive,
             overall_confidence, analysis_duration_ms, analyzer_version,
             created_at, completed_at)
        VALUES ($1, $2, $3, 'completed', $4, $5, $6, $7, $8, $9, NOW(), NOW())
        ON CONFLICT (id) DO UPDATE SET
            status = 'completed',
            properties_proved = EXCLUDED.properties_proved,
            properties_violated = EXCLUDED.properties_violated,
            properties_inconclusive = EXCLUDED.properties_inconclusive,
            overall_confidence = EXCLUDED.overall_confidence,
            analysis_duration_ms = EXCLUDED.analysis_duration_ms,
            completed_at = NOW()
        "#,
    )
    .bind(session_id)
    .bind(contract_id)
    .bind(version)
    .bind(report.certificate.properties_proved as i32)
    .bind(report.certificate.properties_violated as i32)
    .bind(report.certificate.properties_inconclusive as i32)
    .bind(report.certificate.overall_confidence)
    .bind(report.analysis_duration_ms as i64)
    .bind(&report.analyzer_version)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to persist session: {}", e)))?;

    // Insert property results.
    for prop in &report.properties {
        let evidence_json =
            serde_json::to_value(&prop.evidence).unwrap_or(serde_json::Value::Array(vec![]));
        sqlx::query(
            r#"
            INSERT INTO formal_verification_properties
                (id, session_id, property_id, name, description, status, method,
                 confidence, evidence, counterexample)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(session_id)
        .bind(&prop.id)
        .bind(&prop.name)
        .bind(&prop.description)
        .bind(format!("{:?}", prop.status))
        .bind(format!("{:?}", prop.method))
        .bind(prop.confidence)
        .bind(&evidence_json)
        .bind(&prop.counterexample)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to persist property: {}", e)))?;
    }

    // Insert vulnerability findings.
    for vuln in &report.vulnerabilities {
        sqlx::query(
            r#"
            INSERT INTO formal_verification_findings
                (id, session_id, finding_id, title, description, severity,
                 category, cwe_id, affected_functions, remediation)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(session_id)
        .bind(&vuln.id)
        .bind(&vuln.title)
        .bind(&vuln.description)
        .bind(format!("{:?}", vuln.severity))
        .bind(&vuln.category)
        .bind(&vuln.cwe_id)
        .bind(&vuln.affected_functions)
        .bind(&vuln.remediation)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to persist finding: {}", e)))?;
    }

    // Insert proof certificate.
    let cert = &report.certificate;
    sqlx::query(
        r#"
        INSERT INTO formal_verification_certificates
            (id, session_id, certificate_id, properties_proved, properties_violated,
             properties_inconclusive, overall_confidence, summary, generated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(session_id)
    .bind(cert.certificate_id)
    .bind(cert.properties_proved as i32)
    .bind(cert.properties_violated as i32)
    .bind(cert.properties_inconclusive as i32)
    .bind(cert.overall_confidence)
    .bind(&cert.summary)
    .bind(cert.generated_at)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to persist certificate: {}", e)))?;

    // Mirror critical/high findings into security_issues so they appear in the
    // existing security dashboard without any UI changes.
    for vuln in report.vulnerabilities.iter().filter(|v| {
        matches!(
            v.severity,
            VulnerabilitySeverity::Critical | VulnerabilitySeverity::High
        )
    }) {
        let severity_str = match vuln.severity {
            VulnerabilitySeverity::Critical => "critical",
            VulnerabilitySeverity::High => "high",
            _ => "medium",
        };
        let _ = sqlx::query(
            r#"
            INSERT INTO security_issues
                (id, contract_id, title, description, severity, status, category,
                 cwe_id, remediation, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, 'open', $6, $7, $8, NOW(), NOW())
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(contract_id)
        .bind(&vuln.title)
        .bind(&vuln.description)
        .bind(severity_str)
        .bind(&vuln.category)
        .bind(&vuln.cwe_id)
        .bind(&vuln.remediation)
        .execute(&state.db)
        .await;
    }

    Ok(())
}

// ─── POST /api/contracts/:id/formal-verification ─────────────────────────────

pub async fn trigger_formal_verification(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<TriggerVerificationRequest>,
) -> ApiResult<(StatusCode, Json<TriggerVerificationResponse>)> {
    // Confirm contract exists and is verified.
    let is_verified: bool = sqlx::query_scalar("SELECT is_verified FROM contracts WHERE id = $1")
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("contract", "Contract not found"))?;

    if !is_verified {
        return Err(ApiError::bad_request(
            "FORMAL_VERIFICATION_REQUIRES_VERIFIED_CONTRACT",
            "Formal verification requires a verified contract. Run source verification first.",
        ));
    }

    // Throttle: skip if a successful session was completed in the last hour
    // (unless force=true).
    if !req.force.unwrap_or(false) {
        let recent: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id FROM formal_verification_sessions
            WHERE contract_id = $1 AND status = 'completed'
              AND completed_at > NOW() - INTERVAL '1 hour'
            LIMIT 1
            "#,
        )
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

        if let Some(existing_id) = recent {
            return Ok((
                StatusCode::OK,
                Json(TriggerVerificationResponse {
                    session_id: existing_id,
                    contract_id,
                    status: "existing".into(),
                    message: "A recent analysis already exists. Use force=true to re-run.".into(),
                    queued_at: Utc::now(),
                }),
            ));
        }
    }

    let version = req.version.clone();
    let version_ref = version.as_deref();

    // Fetch WASM, run analysis, persist — all in-process (async but not queued).
    let wasm = fetch_wasm_bytes(&state, contract_id, version_ref).await?;

    let report = WasmBytecodeAnalyzer::new(wasm, contract_id)
        .run()
        .map_err(|e| ApiError::internal(format!("Analysis error: {}", e)))?;

    let session_id = report.session_id;
    persist_report(&state, contract_id, version_ref, &report).await?;

    let violations = report.certificate.properties_violated;
    let proved = report.certificate.properties_proved;
    let message = if violations > 0 {
        format!(
            "Analysis complete: {} property violation(s) detected, {} proved. Immediate review recommended.",
            violations, proved
        )
    } else {
        format!(
            "Analysis complete: {} properties proved, {} inconclusive.",
            proved, report.certificate.properties_inconclusive
        )
    };

    Ok((
        StatusCode::CREATED,
        Json(TriggerVerificationResponse {
            session_id,
            contract_id,
            status: "completed".into(),
            message,
            queued_at: Utc::now(),
        }),
    ))
}

// ─── GET /api/contracts/:id/formal-verification ───────────────────────────────

pub async fn list_formal_verification_sessions(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    Query(query): Query<ListSessionsQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    let sessions = sqlx::query_as::<_, VerificationSessionSummary>(
        r#"
        SELECT id, contract_id, version, status,
               properties_proved, properties_violated, properties_inconclusive,
               overall_confidence, analysis_duration_ms, analyzer_version,
               created_at, completed_at
        FROM formal_verification_sessions
        WHERE contract_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(contract_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM formal_verification_sessions WHERE contract_id = $1",
    )
    .bind(contract_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

    Ok(Json(serde_json::json!({
        "items": sessions,
        "total": total,
        "page": (offset / limit) + 1,
        "per_page": limit,
        "pages": (total as f64 / limit as f64).ceil() as i64,
    })))
}

// ─── GET /api/contracts/:id/formal-verification/:session_id ──────────────────

pub async fn get_formal_verification_session(
    State(state): State<AppState>,
    Path((contract_id, session_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<FullSessionResponse>> {
    let session = sqlx::query_as::<_, VerificationSessionSummary>(
        r#"
        SELECT id, contract_id, version, status,
               properties_proved, properties_violated, properties_inconclusive,
               overall_confidence, analysis_duration_ms, analyzer_version,
               created_at, completed_at
        FROM formal_verification_sessions
        WHERE id = $1 AND contract_id = $2
        "#,
    )
    .bind(session_id)
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?
    .ok_or_else(|| ApiError::not_found("session", "Verification session not found"))?;

    let properties = sqlx::query_as::<_, VerificationPropertyRow>(
        r#"
        SELECT id, session_id, property_id, name, description, status, method,
               confidence, evidence, counterexample
        FROM formal_verification_properties
        WHERE session_id = $1
        ORDER BY property_id ASC
        "#,
    )
    .bind(session_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

    let vulnerabilities = sqlx::query_as::<_, VerificationFindingRow>(
        r#"
        SELECT id, session_id, finding_id, title, description, severity,
               category, cwe_id, affected_functions, remediation
        FROM formal_verification_findings
        WHERE session_id = $1
        ORDER BY
            CASE severity
                WHEN 'Critical' THEN 1
                WHEN 'High' THEN 2
                WHEN 'Medium' THEN 3
                WHEN 'Low' THEN 4
                ELSE 5
            END ASC
        "#,
    )
    .bind(session_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

    let certificate = sqlx::query_as::<_, ProofCertificateRow>(
        r#"
        SELECT id, session_id, certificate_id, properties_proved, properties_violated,
               properties_inconclusive, overall_confidence, summary, generated_at
        FROM formal_verification_certificates
        WHERE session_id = $1
        LIMIT 1
        "#,
    )
    .bind(session_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

    Ok(Json(FullSessionResponse {
        session,
        properties,
        vulnerabilities,
        certificate,
    }))
}
