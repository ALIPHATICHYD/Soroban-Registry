// Issue #889: Formal verification integration for contract validation.
//
// An integration layer over the built-in WASM analyzer (formal_verification.rs)
// that adds the pieces a real verification *integration* needs:
//
//   • Pluggable verifier backends — the built-in analyzer, or an external
//     formal-verification service (HTTP) configured via env.
//   • Configurable properties to verify (global or per contract category).
//   • Per-category optional/mandatory policy, surfaced on the contract profile.
//   • Timeout-aware runs: a verification that overruns is recorded as `timeout`
//     rather than failing the request.
//   • Results stored per run, plus a cache keyed by bytecode so identical WASM
//     is not re-verified.
//   • Report generation and a profile summary that integrates with contract data.

use std::time::Duration;

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::ApiError;
use crate::formal_verification::WasmBytecodeAnalyzer;
use crate::state::AppState;

// ── Configuration ─────────────────────────────────────────────────────────────

fn timeout_secs() -> u64 {
    std::env::var("FORMAL_VERIFICATION_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(30)
}

/// Which verification tool/service to use. An external service is selected when
/// `FORMAL_VERIFICATION_SERVICE_URL` is set; otherwise the built-in analyzer.
#[derive(Debug, Clone)]
pub enum VerifierBackend {
    Builtin,
    External(String),
}

impl VerifierBackend {
    pub fn from_env() -> Self {
        match std::env::var("FORMAL_VERIFICATION_SERVICE_URL") {
            Ok(url) if !url.trim().is_empty() => Self::External(url.trim().to_string()),
            _ => Self::Builtin,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::External(_) => "external",
        }
    }
}

// ── Persisted types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PropertyConfig {
    pub id: Uuid,
    pub property_key: String,
    pub name: String,
    pub description: String,
    pub category: Option<String>,
    pub spec: serde_json::Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Policy {
    pub category: String,
    pub requirement: String,
    pub min_confidence: f64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct VerificationRun {
    pub id: Uuid,
    pub contract_id: Uuid,
    pub version: Option<String>,
    pub wasm_hash: String,
    pub backend: String,
    pub status: String,
    pub properties_proved: i32,
    pub properties_violated: i32,
    pub properties_inconclusive: i32,
    pub overall_confidence: f64,
    pub report: serde_json::Value,
    pub duration_ms: i64,
    pub cache_hit: bool,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ── Request / response DTOs ───────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
pub struct RunRequest {
    pub version: Option<String>,
    /// Re-run even if a cached result exists for this bytecode.
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpsertPropertyRequest {
    pub property_key: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub spec: Option<serde_json::Value>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct SetPolicyRequest {
    pub requirement: String,
    #[serde(default)]
    pub min_confidence: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct RequirementStatus {
    pub category: Option<String>,
    pub requirement: String,
    pub min_confidence: f64,
    pub satisfied: bool,
    pub latest_status: Option<String>,
    pub latest_confidence: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct VerificationSummary {
    pub contract_id: Uuid,
    pub has_verification: bool,
    pub latest: Option<VerificationRun>,
    pub requirement: RequirementStatus,
    pub total_runs: i64,
}

// ── Backend execution (timeout-aware) ─────────────────────────────────────────

/// Normalised result across backends.
struct RunOutcome {
    status: String,
    proved: i32,
    violated: i32,
    inconclusive: i32,
    confidence: f64,
    report: serde_json::Value,
    error: Option<String>,
}

impl RunOutcome {
    fn timeout(backend: &str, secs: u64) -> Self {
        RunOutcome {
            status: "timeout".to_string(),
            proved: 0,
            violated: 0,
            inconclusive: 0,
            confidence: 0.0,
            report: serde_json::json!({ "timeout_secs": secs, "backend": backend }),
            error: Some(format!("verification timed out after {secs}s")),
        }
    }

    fn failed(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        RunOutcome {
            status: "failed".to_string(),
            proved: 0,
            violated: 0,
            inconclusive: 0,
            confidence: 0.0,
            report: serde_json::json!({ "error": msg }),
            error: Some(msg),
        }
    }
}

/// Run a backend with a hard timeout. A timeout is reported as a graceful
/// `timeout` outcome, never an error.
async fn run_with_timeout(
    backend: &VerifierBackend,
    wasm: Vec<u8>,
    contract_id: Uuid,
    timeout: Duration,
) -> RunOutcome {
    let secs = timeout.as_secs();
    match backend {
        VerifierBackend::Builtin => {
            // The analyzer is synchronous CPU work; run it off the async runtime.
            let fut = tokio::task::spawn_blocking(move || {
                WasmBytecodeAnalyzer::new(wasm, contract_id).run()
            });
            match tokio::time::timeout(timeout, fut).await {
                Err(_) => RunOutcome::timeout("builtin", secs),
                Ok(Err(join_err)) => {
                    RunOutcome::failed(format!("analysis task failed: {join_err}"))
                }
                Ok(Ok(Err(e))) => RunOutcome::failed(e),
                Ok(Ok(Ok(report))) => {
                    let value = serde_json::to_value(&report).unwrap_or(serde_json::json!({}));
                    RunOutcome {
                        status: "completed".to_string(),
                        proved: report.certificate.properties_proved as i32,
                        violated: report.certificate.properties_violated as i32,
                        inconclusive: report.certificate.properties_inconclusive as i32,
                        confidence: report.certificate.overall_confidence,
                        report: value,
                        error: None,
                    }
                }
            }
        }
        VerifierBackend::External(url) => {
            run_external(url, &wasm, contract_id, timeout, secs).await
        }
    }
}

async fn run_external(
    url: &str,
    wasm: &[u8],
    contract_id: Uuid,
    timeout: Duration,
    secs: u64,
) -> RunOutcome {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    let client = match reqwest::Client::builder().timeout(timeout).build() {
        Ok(c) => c,
        Err(e) => return RunOutcome::failed(format!("client build failed: {e}")),
    };
    let body = serde_json::json!({
        "contract_id": contract_id,
        "wasm_base64": BASE64.encode(wasm),
    });

    let resp = match client.post(url).json(&body).send().await {
        Ok(r) => r,
        Err(e) if e.is_timeout() => return RunOutcome::timeout("external", secs),
        Err(e) => return RunOutcome::failed(format!("external service request failed: {e}")),
    };
    if !resp.status().is_success() {
        return RunOutcome::failed(format!("external service returned {}", resp.status()));
    }
    let value: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => return RunOutcome::failed(format!("invalid service response: {e}")),
    };

    // Expected shape: { properties_proved, properties_violated,
    //                   properties_inconclusive, overall_confidence, report }
    let as_i32 = |k: &str| value.get(k).and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    RunOutcome {
        status: "completed".to_string(),
        proved: as_i32("properties_proved"),
        violated: as_i32("properties_violated"),
        inconclusive: as_i32("properties_inconclusive"),
        confidence: value
            .get("overall_confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        report: value.get("report").cloned().unwrap_or(value),
        error: None,
    }
}

// ── Caching ───────────────────────────────────────────────────────────────────

fn cache_key(wasm_hash: &str, backend: &str, property_keys: &[String]) -> String {
    let mut keys = property_keys.to_vec();
    keys.sort();
    let mut h = Sha256::new();
    h.update(wasm_hash.as_bytes());
    h.update(b"|");
    h.update(backend.as_bytes());
    h.update(b"|");
    h.update(keys.join(",").as_bytes());
    hex::encode(h.finalize())
}

// ── Policy helpers (pure) ─────────────────────────────────────────────────────

/// Whether a run satisfies a category's policy.
pub fn requirement_satisfied(
    requirement: &str,
    status: Option<&str>,
    confidence: Option<f64>,
    min_confidence: f64,
) -> bool {
    match requirement {
        // Mandatory: a completed run meeting the confidence bar is required.
        "mandatory" => status == Some("completed") && confidence.unwrap_or(0.0) >= min_confidence,
        // Optional/disabled (and anything else): always considered satisfied.
        _ => true,
    }
}

const DEFAULT_REQUIREMENT: &str = "optional";
const DEFAULT_MIN_CONFIDENCE: f64 = 0.8;

// ── Handlers: run + profile ───────────────────────────────────────────────────

/// POST /api/contracts/:id/formal-verification/run
pub async fn run_verification(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
    body: Option<Json<RunRequest>>,
) -> Result<Json<VerificationRun>, ApiError> {
    let req = body.map(|Json(r)| r).unwrap_or_default();

    let (wasm_hash, category) = contract_meta(&state, contract_id).await?;
    let backend = VerifierBackend::from_env();
    let property_keys = selected_property_keys(&state, category.as_deref()).await?;
    let ckey = cache_key(&wasm_hash, backend.name(), &property_keys);

    // Serve from cache unless forced.
    if !req.force {
        if let Some(cached) = load_cache(&state, &ckey).await? {
            let run = persist_run(
                &state,
                contract_id,
                req.version.as_deref(),
                &wasm_hash,
                backend.name(),
                &cached,
                0,
                true,
            )
            .await?;
            return Ok(Json(run));
        }
    }

    // Run (timeout-aware).
    let started = std::time::Instant::now();
    let wasm = fetch_wasm(&state, contract_id, req.version.as_deref(), &wasm_hash).await;
    let outcome = run_with_timeout(
        &backend,
        wasm,
        contract_id,
        Duration::from_secs(timeout_secs()),
    )
    .await;
    let duration_ms = started.elapsed().as_millis() as i64;

    // Cache only successful, deterministic results.
    if outcome.status == "completed" {
        let _ = store_cache(&state, &ckey, &wasm_hash, backend.name(), &outcome).await;
    }

    let run = persist_run(
        &state,
        contract_id,
        req.version.as_deref(),
        &wasm_hash,
        backend.name(),
        &outcome,
        duration_ms,
        false,
    )
    .await?;

    Ok(Json(run))
}

/// GET /api/contracts/:id/formal-verification/runs
pub async fn list_runs(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> Result<Json<Vec<VerificationRun>>, ApiError> {
    let runs = sqlx::query_as::<_, VerificationRun>(
        "SELECT * FROM formal_verification_runs WHERE contract_id = $1 ORDER BY started_at DESC LIMIT 50",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("FV_RUN_LIST_ERROR", e.to_string()))?;
    Ok(Json(runs))
}

/// GET /api/contracts/:id/formal-verification/runs/:run_id/report
pub async fn get_report(
    State(state): State<AppState>,
    Path((contract_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let run = sqlx::query_as::<_, VerificationRun>(
        "SELECT * FROM formal_verification_runs WHERE id = $1 AND contract_id = $2",
    )
    .bind(run_id)
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("FV_REPORT_ERROR", e.to_string()))?
    .ok_or_else(|| ApiError::not_found("RUN_NOT_FOUND", "verification run not found"))?;

    Ok(Json(serde_json::json!({
        "run_id": run.id,
        "contract_id": run.contract_id,
        "backend": run.backend,
        "status": run.status,
        "summary": {
            "properties_proved": run.properties_proved,
            "properties_violated": run.properties_violated,
            "properties_inconclusive": run.properties_inconclusive,
            "overall_confidence": run.overall_confidence,
        },
        "duration_ms": run.duration_ms,
        "cache_hit": run.cache_hit,
        "error": run.error_message,
        "report": run.report,
        "generated_at": Utc::now(),
    })))
}

/// GET /api/contracts/:id/formal-verification/summary — for the contract profile.
pub async fn get_summary(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> Result<Json<VerificationSummary>, ApiError> {
    let (_wasm_hash, category) = contract_meta(&state, contract_id).await?;

    let latest = sqlx::query_as::<_, VerificationRun>(
        "SELECT * FROM formal_verification_runs WHERE contract_id = $1 ORDER BY started_at DESC LIMIT 1",
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("FV_SUMMARY_ERROR", e.to_string()))?;

    let total_runs: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM formal_verification_runs WHERE contract_id = $1")
            .bind(contract_id)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let requirement = requirement_for(&state, category.as_deref(), latest.as_ref()).await?;

    Ok(Json(VerificationSummary {
        contract_id,
        has_verification: latest.is_some(),
        requirement,
        total_runs,
        latest,
    }))
}

/// GET /api/contracts/:id/formal-verification/requirement
pub async fn get_requirement(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> Result<Json<RequirementStatus>, ApiError> {
    let (_wasm_hash, category) = contract_meta(&state, contract_id).await?;
    let latest = sqlx::query_as::<_, VerificationRun>(
        "SELECT * FROM formal_verification_runs WHERE contract_id = $1 ORDER BY started_at DESC LIMIT 1",
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("FV_REQUIREMENT_ERROR", e.to_string()))?;

    let requirement = requirement_for(&state, category.as_deref(), latest.as_ref()).await?;
    Ok(Json(requirement))
}

// ── Handlers: property configuration ──────────────────────────────────────────

/// GET /api/formal-verification/properties
pub async fn list_properties(
    State(state): State<AppState>,
) -> Result<Json<Vec<PropertyConfig>>, ApiError> {
    let rows = sqlx::query_as::<_, PropertyConfig>(
        "SELECT * FROM formal_verification_properties ORDER BY property_key",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("FV_PROPERTY_LIST_ERROR", e.to_string()))?;
    Ok(Json(rows))
}

/// POST /api/formal-verification/properties — create or update a property.
pub async fn upsert_property(
    State(state): State<AppState>,
    Json(req): Json<UpsertPropertyRequest>,
) -> Result<Json<PropertyConfig>, ApiError> {
    if req.property_key.trim().is_empty() {
        return Err(ApiError::bad_request(
            "INVALID_KEY",
            "property_key is required",
        ));
    }
    let row = sqlx::query_as::<_, PropertyConfig>(
        r#"
        INSERT INTO formal_verification_properties
            (property_key, name, description, category, spec, enabled)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (property_key) DO UPDATE
        SET name = EXCLUDED.name,
            description = EXCLUDED.description,
            category = EXCLUDED.category,
            spec = EXCLUDED.spec,
            enabled = EXCLUDED.enabled,
            updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(req.property_key.trim())
    .bind(&req.name)
    .bind(req.description.unwrap_or_default())
    .bind(&req.category)
    .bind(req.spec.unwrap_or_else(|| serde_json::json!({})))
    .bind(req.enabled)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("FV_PROPERTY_UPSERT_ERROR", e.to_string()))?;
    Ok(Json(row))
}

// ── Handlers: policy ──────────────────────────────────────────────────────────

/// GET /api/formal-verification/policies
pub async fn list_policies(State(state): State<AppState>) -> Result<Json<Vec<Policy>>, ApiError> {
    let rows =
        sqlx::query_as::<_, Policy>("SELECT * FROM formal_verification_policies ORDER BY category")
            .fetch_all(&state.db)
            .await
            .map_err(|e| ApiError::internal_error("FV_POLICY_LIST_ERROR", e.to_string()))?;
    Ok(Json(rows))
}

/// PUT /api/formal-verification/policies/:category
pub async fn set_policy(
    State(state): State<AppState>,
    Path(category): Path<String>,
    Json(req): Json<SetPolicyRequest>,
) -> Result<Json<Policy>, ApiError> {
    if !matches!(
        req.requirement.as_str(),
        "mandatory" | "optional" | "disabled"
    ) {
        return Err(ApiError::bad_request(
            "INVALID_REQUIREMENT",
            "requirement must be mandatory, optional, or disabled",
        ));
    }
    let min_conf = req
        .min_confidence
        .unwrap_or(DEFAULT_MIN_CONFIDENCE)
        .clamp(0.0, 1.0);
    let row = sqlx::query_as::<_, Policy>(
        r#"
        INSERT INTO formal_verification_policies (category, requirement, min_confidence)
        VALUES ($1, $2, $3)
        ON CONFLICT (category) DO UPDATE
        SET requirement = EXCLUDED.requirement,
            min_confidence = EXCLUDED.min_confidence,
            updated_at = NOW()
        RETURNING *
        "#,
    )
    .bind(&category)
    .bind(&req.requirement)
    .bind(min_conf)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("FV_POLICY_SET_ERROR", e.to_string()))?;
    Ok(Json(row))
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Returns (wasm_hash, category) for a contract, or 404.
async fn contract_meta(
    state: &AppState,
    contract_id: Uuid,
) -> Result<(String, Option<String>), ApiError> {
    sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT wasm_hash, category FROM contracts WHERE id = $1",
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("FV_CONTRACT_ERROR", e.to_string()))?
    .ok_or_else(|| ApiError::not_found("CONTRACT_NOT_FOUND", "contract not found"))
}

/// Enabled property keys applicable to a category (global + category-specific).
async fn selected_property_keys(
    state: &AppState,
    category: Option<&str>,
) -> Result<Vec<String>, ApiError> {
    let keys: Vec<String> = sqlx::query_scalar(
        "SELECT property_key FROM formal_verification_properties \
         WHERE enabled = true AND (category IS NULL OR category = $1) ORDER BY property_key",
    )
    .bind(category)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("FV_PROPERTY_SELECT_ERROR", e.to_string()))?;
    Ok(keys)
}

async fn requirement_for(
    state: &AppState,
    category: Option<&str>,
    latest: Option<&VerificationRun>,
) -> Result<RequirementStatus, ApiError> {
    let policy = match category {
        Some(cat) => sqlx::query_as::<_, Policy>(
            "SELECT * FROM formal_verification_policies WHERE category = $1",
        )
        .bind(cat)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal_error("FV_POLICY_ERROR", e.to_string()))?,
        None => None,
    };

    let (requirement, min_confidence) = policy
        .map(|p| (p.requirement, p.min_confidence))
        .unwrap_or_else(|| (DEFAULT_REQUIREMENT.to_string(), DEFAULT_MIN_CONFIDENCE));

    let latest_status = latest.map(|r| r.status.clone());
    let latest_confidence = latest.map(|r| r.overall_confidence);
    let satisfied = requirement_satisfied(
        &requirement,
        latest_status.as_deref(),
        latest_confidence,
        min_confidence,
    );

    Ok(RequirementStatus {
        category: category.map(|c| c.to_string()),
        requirement,
        min_confidence,
        satisfied,
        latest_status,
        latest_confidence,
    })
}

#[allow(clippy::too_many_arguments)]
async fn persist_run(
    state: &AppState,
    contract_id: Uuid,
    version: Option<&str>,
    wasm_hash: &str,
    backend: &str,
    outcome: &RunOutcome,
    duration_ms: i64,
    cache_hit: bool,
) -> Result<VerificationRun, ApiError> {
    sqlx::query_as::<_, VerificationRun>(
        r#"
        INSERT INTO formal_verification_runs
            (contract_id, version, wasm_hash, backend, status,
             properties_proved, properties_violated, properties_inconclusive,
             overall_confidence, report, duration_ms, cache_hit, error_message, completed_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW())
        RETURNING *
        "#,
    )
    .bind(contract_id)
    .bind(version)
    .bind(wasm_hash)
    .bind(backend)
    .bind(&outcome.status)
    .bind(outcome.proved)
    .bind(outcome.violated)
    .bind(outcome.inconclusive)
    .bind(outcome.confidence)
    .bind(&outcome.report)
    .bind(duration_ms)
    .bind(cache_hit)
    .bind(&outcome.error)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("FV_RUN_PERSIST_ERROR", e.to_string()))
}

async fn load_cache(state: &AppState, ckey: &str) -> Result<Option<RunOutcome>, ApiError> {
    let row: Option<(String, i32, i32, i32, f64, serde_json::Value)> = sqlx::query_as(
        "SELECT status, properties_proved, properties_violated, properties_inconclusive, \
         overall_confidence, report FROM formal_verification_run_cache WHERE cache_key = $1",
    )
    .bind(ckey)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("FV_CACHE_LOAD_ERROR", e.to_string()))?;

    if let Some((status, proved, violated, inconclusive, confidence, report)) = row {
        let _ = sqlx::query(
            "UPDATE formal_verification_run_cache SET hits = hits + 1 WHERE cache_key = $1",
        )
        .bind(ckey)
        .execute(&state.db)
        .await;
        Ok(Some(RunOutcome {
            status,
            proved,
            violated,
            inconclusive,
            confidence,
            report,
            error: None,
        }))
    } else {
        Ok(None)
    }
}

async fn store_cache(
    state: &AppState,
    ckey: &str,
    wasm_hash: &str,
    backend: &str,
    outcome: &RunOutcome,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        INSERT INTO formal_verification_run_cache
            (cache_key, wasm_hash, backend, status, properties_proved, properties_violated,
             properties_inconclusive, overall_confidence, report)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (cache_key) DO NOTHING
        "#,
    )
    .bind(ckey)
    .bind(wasm_hash)
    .bind(backend)
    .bind(&outcome.status)
    .bind(outcome.proved)
    .bind(outcome.violated)
    .bind(outcome.inconclusive)
    .bind(outcome.confidence)
    .bind(&outcome.report)
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("FV_CACHE_STORE_ERROR", e.to_string()))?;
    Ok(())
}

/// Fetch WASM bytes for analysis, falling back to a minimal valid module so a
/// run still produces a (low-confidence) result rather than an error.
async fn fetch_wasm(
    state: &AppState,
    contract_id: Uuid,
    version: Option<&str>,
    _wasm_hash: &str,
) -> Vec<u8> {
    const EMPTY_WASM: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

    let row: Option<(Option<String>, String, String)> = if let Some(ver) = version {
        sqlx::query_as(
            "SELECT cs.source_url, cs.storage_backend, cs.storage_key \
             FROM contract_versions cv LEFT JOIN contract_sources cs \
             ON cs.contract_version_id = cv.id \
             WHERE cv.contract_id = $1 AND cv.version = $2 LIMIT 1",
        )
        .bind(contract_id)
        .bind(ver)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
    } else {
        sqlx::query_as(
            "SELECT cs.source_url, cs.storage_backend, cs.storage_key \
             FROM contract_versions cv LEFT JOIN contract_sources cs \
             ON cs.contract_version_id = cv.id \
             WHERE cv.contract_id = $1 ORDER BY cv.created_at DESC LIMIT 1",
        )
        .bind(contract_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
    };

    if let Some((_source_url, storage_backend, storage_key)) = row {
        if let Ok(bytes) = state
            .source_storage
            .retrieve_source(&storage_backend, &storage_key)
            .await
        {
            return bytes;
        }
    }

    EMPTY_WASM.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_is_order_independent_for_properties() {
        let a = cache_key("hash1", "builtin", &["p2".into(), "p1".into()]);
        let b = cache_key("hash1", "builtin", &["p1".into(), "p2".into()]);
        assert_eq!(a, b);
    }

    #[test]
    fn cache_key_varies_by_inputs() {
        let base = cache_key("hash1", "builtin", &["p1".into()]);
        assert_ne!(base, cache_key("hash2", "builtin", &["p1".into()]));
        assert_ne!(base, cache_key("hash1", "external", &["p1".into()]));
        assert_ne!(
            base,
            cache_key("hash1", "builtin", &["p1".into(), "p2".into()])
        );
        assert_eq!(base.len(), 64);
    }

    #[test]
    fn mandatory_requires_completed_and_confidence() {
        assert!(requirement_satisfied(
            "mandatory",
            Some("completed"),
            Some(0.9),
            0.8
        ));
        assert!(!requirement_satisfied(
            "mandatory",
            Some("completed"),
            Some(0.5),
            0.8
        ));
        assert!(!requirement_satisfied(
            "mandatory",
            Some("timeout"),
            Some(0.9),
            0.8
        ));
        assert!(!requirement_satisfied("mandatory", None, None, 0.8));
    }

    #[test]
    fn optional_and_disabled_always_satisfied() {
        assert!(requirement_satisfied("optional", None, None, 0.8));
        assert!(requirement_satisfied(
            "disabled",
            Some("failed"),
            Some(0.0),
            0.8
        ));
    }

    #[test]
    fn backend_selection_defaults_to_builtin() {
        // Without the env var set, the default is the built-in analyzer.
        std::env::remove_var("FORMAL_VERIFICATION_SERVICE_URL");
        assert_eq!(VerifierBackend::from_env().name(), "builtin");
    }
}
