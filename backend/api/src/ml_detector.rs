use std::path::{Path, PathBuf};
use std::process::Command;

use axum::http::StatusCode;
use serde_json::Value;
use uuid::Uuid;

use crate::error::ApiError;
use shared::{IssueSeverity, SecurityScan};

#[derive(Debug, Clone)]
pub struct MlDetectorOutput {
    pub score: i32,
    pub grade: String,
    pub model: String,
    pub summary: String,
    pub findings: Vec<MlDetectorFinding>,
    pub signals: Value,
    pub probabilities: Value,
}

#[derive(Debug, Clone)]
pub struct MlDetectorFinding {
    pub title: String,
    pub severity: String,
    pub category: String,
    pub line: i32,
    pub evidence: String,
    pub explanation: String,
    pub recommendation: String,
    pub confidence: f64,
    pub weight: f64,
}

fn detector_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/ml-vuln-detector")
}

pub fn detector_available() -> bool {
    detector_root().join("bin/scan.js").exists() && detector_root().join("bin/train.js").exists()
}

pub async fn source_for_contract(
    state: &crate::state::AppState,
    contract_id: Uuid,
    _version: Option<&str>,
) -> Result<(String, Uuid), ApiError> {
    let row: Option<(Uuid, Option<String>)> = sqlx::query_as(
        "SELECT id, source_code FROM verifications WHERE contract_id = $1 AND status = 'verified' ORDER BY created_at DESC LIMIT 1",
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("DB error: {}", e)))?;

    let (verification_id, source_code) = row
        .ok_or_else(|| ApiError::not_found("verification", "No verified source code available"))?;

    let source = source_code
        .ok_or_else(|| ApiError::not_found("verification", "Verified source code is empty"))?;

    Ok((source, verification_id))
}

pub fn write_temp_source(source: &str) -> Result<PathBuf, ApiError> {
    let temp_dir = std::env::temp_dir().join(format!("soroban-ml-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| ApiError::internal(format!("Failed to create temp dir: {}", e)))?;
    let path = temp_dir.join("source.rs");
    std::fs::write(&path, source)
        .map_err(|e| ApiError::internal(format!("Failed to write temp source: {}", e)))?;
    Ok(path)
}

pub fn run_ml_detector(source: &str) -> Result<MlDetectorOutput, ApiError> {
    let source_path = write_temp_source(source)?;
    let detector = detector_root();
    let output = Command::new("node")
        .arg(detector.join("bin/scan.js"))
        .arg(detector.join("model.json"))
        .arg(&source_path)
        .output()
        .map_err(|e| ApiError::internal(format!("Failed to launch ML detector: {}", e)))?;

    if !output.status.success() {
        return Err(ApiError::internal(format!(
            "ML detector failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let value: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| ApiError::internal(format!("Invalid ML detector output: {}", e)))?;

    let findings = value
        .get("findings")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .map(|item| MlDetectorFinding {
                    title: item
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("ML finding")
                        .to_string(),
                    severity: item
                        .get("severity")
                        .and_then(|v| v.as_str())
                        .unwrap_or("low")
                        .to_string(),
                    category: item
                        .get("category")
                        .and_then(|v| v.as_str())
                        .unwrap_or("optimization")
                        .to_string(),
                    line: item.get("line").and_then(|v| v.as_i64()).unwrap_or(1) as i32,
                    evidence: item
                        .get("evidence")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    explanation: item
                        .get("explanation")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    recommendation: item
                        .get("recommendation")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    confidence: item
                        .get("confidence")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0),
                    weight: item.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.0),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(MlDetectorOutput {
        score: value.get("score").and_then(|v| v.as_i64()).unwrap_or(100) as i32,
        grade: value
            .get("grade")
            .and_then(|v| v.as_str())
            .unwrap_or("A")
            .to_string(),
        model: value
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("ml-vuln-detector-v1")
            .to_string(),
        summary: value
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        findings,
        signals: value
            .get("signals")
            .cloned()
            .unwrap_or(Value::Array(vec![])),
        probabilities: value
            .get("probabilities")
            .cloned()
            .unwrap_or(Value::Object(Default::default())),
    })
}

fn severity_from_text(value: &str) -> IssueSeverity {
    match value.to_lowercase().as_str() {
        "critical" => IssueSeverity::Critical,
        "high" => IssueSeverity::High,
        "medium" => IssueSeverity::Medium,
        _ => IssueSeverity::Low,
    }
}

pub async fn persist_ml_scan(
    state: &crate::state::AppState,
    contract_id: Uuid,
    contract_version_id: Uuid,
    source: String,
) -> Result<SecurityScan, ApiError> {
    let output = run_ml_detector(&source)?;
    let scan = sqlx::query_as::<_, SecurityScan>(
        r#"
        INSERT INTO security_scans
            (contract_id, contract_version_id, scanner_id, status, scan_type, triggered_by_event,
             total_issues, critical_issues, high_issues, medium_issues, low_issues,
             scan_duration_ms, scanner_version, scan_parameters, scan_result_raw, started_at, completed_at,
             created_at, updated_at)
        VALUES
            ($1, $2, NULL, 'completed', 'ml', 'upload',
             $3, $4, $5, $6, $7,
             0, $8, $9, $10, NOW(), NOW(), NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(Some(contract_version_id))
    .bind(output.findings.len() as i32)
    .bind(output.findings.iter().filter(|finding| finding.severity == "critical").count() as i32)
    .bind(output.findings.iter().filter(|finding| finding.severity == "high").count() as i32)
    .bind(output.findings.iter().filter(|finding| finding.severity == "medium").count() as i32)
    .bind(output.findings.iter().filter(|finding| finding.severity == "low").count() as i32)
    .bind(&output.model)
    .bind(serde_json::json!({
        "grade": output.grade,
        "score": output.score,
        "summary": output.summary,
        "signals": output.signals,
        "probabilities": output.probabilities,
    }))
    .bind(serde_json::json!({
        "detector": "ml-vuln-detector-v1",
        "model": output.model,
    }))
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("Failed to create ML scan: {}", e)))?;

    for finding in &output.findings {
        let _ = sqlx::query(
            r#"
            INSERT INTO security_issues
                (id, scan_id, contract_id, contract_version_id, title, description, severity, status, category,
                 source_file, source_line_start, source_line_end, code_snippet, remediation, references,
                 is_false_positive, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'open', $8, $9, $10, $11, $12, $13, $14, false, NOW(), NOW())
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(scan.id)
        .bind(contract_id)
        .bind(Some(contract_version_id))
        .bind(&finding.title)
        .bind(&finding.explanation)
        .bind(severity_from_text(&finding.severity))
        .bind(&finding.category)
        .bind(Some("source.rs".to_string()))
        .bind(Some(finding.line))
        .bind(Some(finding.line))
        .bind(Some(finding.evidence.clone()))
        .bind(Some(finding.recommendation.clone()))
        .bind(Some(vec![format!("confidence={:.4}", finding.confidence)]))
        .execute(&state.db)
        .await;
    }

    let _ = sqlx::query(
        r#"
        INSERT INTO security_score_history
            (id, contract_id, contract_version_id, overall_score, score_breakdown, critical_count,
             high_count, medium_count, low_count, scan_id, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(contract_id)
    .bind(contract_version_id)
    .bind(output.score)
    .bind(serde_json::json!({
        "model": output.model,
        "grade": output.grade,
        "summary": output.summary,
        "probabilities": output.probabilities,
    }))
    .bind(
        output
            .findings
            .iter()
            .filter(|finding| finding.severity == "critical")
            .count() as i32,
    )
    .bind(
        output
            .findings
            .iter()
            .filter(|finding| finding.severity == "high")
            .count() as i32,
    )
    .bind(
        output
            .findings
            .iter()
            .filter(|finding| finding.severity == "medium")
            .count() as i32,
    )
    .bind(
        output
            .findings
            .iter()
            .filter(|finding| finding.severity == "low")
            .count() as i32,
    )
    .bind(scan.id)
    .execute(&state.db)
    .await;

    Ok(scan)
}
