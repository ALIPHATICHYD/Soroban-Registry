use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use futures_util::stream::{self, StreamExt};
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::{json, Value};
use shared::{BatchVerifyItem, Network};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::{db_internal_error, fetch_contract_identity},
    metrics,
    onchain_verification::OnChainVerifier,
    state::AppState,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BULK_VERIFY_SYNC_LIMIT: usize = 50;
const BULK_VERIFY_ASYNC_LIMIT: usize = 500;

#[derive(Debug, Deserialize)]
pub struct BulkVerifyQuery {
    #[serde(default, rename = "async")]
    async_mode: bool,
}

#[derive(Debug, Deserialize)]
pub struct HealthQuery {
    #[serde(default)]
    detailed: bool,
}

#[derive(Debug, Clone)]
struct BulkVerifyJob {
    response: Value,
}

static BULK_VERIFY_JOBS: Lazy<RwLock<HashMap<Uuid, BulkVerifyJob>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, sqlx::FromRow)]
struct ContractMetadataRow {
    id: Uuid,
    contract_id: String,
    wasm_hash: String,
    name: String,
    description: Option<String>,
    publisher_id: Uuid,
    network: Network,
    is_verified: bool,
    verification_status: String,
    category: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    verified_at: Option<DateTime<Utc>>,
    deployed_at: Option<DateTime<Utc>>,
    last_accessed_at: Option<DateTime<Utc>>,
    health_score: i32,
    current_version: Option<String>,
    usage_count: i64,
    owner_address: String,
    owner_username: Option<String>,
    owner_email: Option<String>,
    owner_github_url: Option<String>,
    owner_website: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct TagRow {
    id: Uuid,
    name: String,
    color: String,
}

#[derive(Debug, sqlx::FromRow)]
struct DeploymentRow {
    id: Uuid,
    environment: String,
    status: String,
    wasm_hash: String,
    deployed_at: DateTime<Utc>,
    activated_at: Option<DateTime<Utc>>,
    health_checks_passed: Option<i32>,
    health_checks_failed: Option<i32>,
    last_health_check_at: Option<DateTime<Utc>>,
    error_message: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct RelatedContractRow {
    id: Uuid,
    contract_id: String,
    name: String,
    network: Network,
    category: Option<String>,
    relation_score: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct AuditScoreRow {
    status: Option<String>,
    security_score: Option<i32>,
    total_score: Option<i32>,
    audit_date: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, sqlx::FromRow)]
struct SecurityFindingRow {
    id: String,
    title: String,
    severity: String,
    description: Option<String>,
    source: String,
}

#[derive(Debug, sqlx::FromRow)]
struct DependencyRiskRow {
    package_name: String,
    package_version: Option<String>,
    cve_id: Option<String>,
    severity: Option<String>,
}

/// GET /api/v1/contracts/:id/metadata
pub async fn get_contract_metadata_v1(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let (contract_uuid, contract_address) = fetch_contract_identity(&state, &id).await?;
    let cache_key = format!("v1:{}:{}", contract_uuid, contract_address);

    if let Some(cached) = state.cache.get_contract_meta(&cache_key).await {
        if let Ok(value) = serde_json::from_str::<Value>(&cached) {
            return Ok(Json(value));
        }
    }

    let contract = fetch_metadata_row(&state, contract_uuid).await?;
    let tags = fetch_tags(&state, contract_uuid).await?;
    let last_interaction = fetch_last_interaction(&state, contract_uuid).await?;
    let audit_scores = fetch_audit_scores(&state, contract_uuid).await?;
    let related_contracts = fetch_related_contracts(&state, contract_uuid, &contract).await?;
    let deployments = fetch_deployments(&state, contract_uuid).await?;

    let response = json!({
        "id": contract.id,
        "contract_id": contract.contract_id,
        "wasm_hash": contract.wasm_hash,
        "name": contract.name,
        "description": contract.description,
        "category": contract.category,
        "network": contract.network,
        "owner": {
            "id": contract.publisher_id,
            "stellar_address": contract.owner_address,
            "username": contract.owner_username,
            "email": contract.owner_email,
            "github_url": contract.owner_github_url,
            "website": contract.owner_website
        },
        "timestamps": {
            "created_at": contract.created_at,
            "updated_at": contract.updated_at,
            "verified_at": contract.verified_at,
            "deployed_at": contract.deployed_at,
            "last_interaction": last_interaction,
            "last_accessed_at": contract.last_accessed_at
        },
        "verification": {
            "is_verified": contract.is_verified,
            "status": contract.verification_status,
            "verified_at": contract.verified_at
        },
        "audit_scores": audit_scores,
        "tags": tags,
        "related_contracts": related_contracts,
        "deployment": {
            "current_version": contract.current_version,
            "deployment_count": deployments.len(),
            "usage_count": contract.usage_count,
            "deployments": deployments
        },
        "metadata_only": true,
        "cached_for_seconds": 3600,
        "generated_at": Utc::now()
    });

    if let Ok(serialized) = serde_json::to_string(&response) {
        state.cache.put_contract_meta(&cache_key, serialized).await;
    }

    Ok(Json(response))
}

/// POST /api/v1/contracts/bulk-verify
pub async fn bulk_verify_v1(
    State(state): State<AppState>,
    Query(query): Query<BulkVerifyQuery>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let contracts = match parse_bulk_verify_payload(payload) {
        Ok(items) => items,
        Err(err) => return err.into_response(),
    };

    if contracts.is_empty() {
        return ApiError::bad_request("InvalidRequest", "contract list must not be empty")
            .into_response();
    }

    if query.async_mode {
        if contracts.len() > BULK_VERIFY_ASYNC_LIMIT {
            return ApiError::bad_request(
                "BatchTooLarge",
                format!(
                    "async bulk verification is limited to {BULK_VERIFY_ASYNC_LIMIT} contracts"
                ),
            )
            .into_response();
        }

        let job_id = Uuid::new_v4();
        let now = Utc::now();
        let accepted = json!({
            "job_id": job_id,
            "status": "processing",
            "total": contracts.len(),
            "submitted_at": now,
            "status_url": format!("/api/v1/contracts/bulk-verify/jobs/{job_id}")
        });

        BULK_VERIFY_JOBS.write().await.insert(
            job_id,
            BulkVerifyJob {
                response: accepted.clone(),
            },
        );

        tokio::spawn(run_bulk_verify_job(job_id, contracts, state));
        return (StatusCode::ACCEPTED, Json(accepted)).into_response();
    }

    if contracts.len() > BULK_VERIFY_SYNC_LIMIT {
        return ApiError::bad_request(
            "BatchTooLarge",
            format!("bulk verification is limited to {BULK_VERIFY_SYNC_LIMIT} contracts; use ?async=true for larger batches"),
        )
        .into_response();
    }

    let response = verify_contract_batch(&state, contracts).await;
    (StatusCode::OK, Json(response)).into_response()
}

/// GET /api/v1/contracts/bulk-verify/jobs/:job_id
pub async fn get_bulk_verify_job_v1(Path(job_id): Path<Uuid>) -> impl IntoResponse {
    let jobs = BULK_VERIFY_JOBS.read().await;
    match jobs.get(&job_id) {
        Some(job) => (StatusCode::OK, Json(job.response.clone())).into_response(),
        None => {
            ApiError::not_found("JobNotFound", "bulk verification job not found").into_response()
        }
    }
}

/// GET /api/v1/health
pub async fn health_v1(
    State(state): State<AppState>,
    Query(query): Query<HealthQuery>,
) -> (StatusCode, Json<Value>) {
    let started = std::time::Instant::now();
    let is_shutting_down = state.is_shutting_down.load(Ordering::SeqCst);
    let db_check = tokio::time::timeout(
        std::time::Duration::from_millis(50),
        sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.db),
    )
    .await;

    let database = match db_check {
        Ok(Ok(_)) => json!({"status": "healthy"}),
        Ok(Err(err)) => json!({"status": "unhealthy", "error": err.to_string()}),
        Err(_) => json!({"status": "degraded", "error": "database health check timed out"}),
    };

    let cache_config = state.cache.config();
    let cache = json!({
        "status": "healthy",
        "enabled": cache_config.enabled,
        "redis_enabled": cache_config.redis_enabled
    });

    let external_apis = json!({
        "status": "healthy",
        "search_configured": true,
        "ai_configured": state.ai_service.is_some()
    });

    let status = if is_shutting_down || database["status"] == "unhealthy" {
        "unhealthy"
    } else if database["status"] == "degraded" {
        "degraded"
    } else {
        "healthy"
    };

    let status_code = if status == "unhealthy" {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };

    let commit = option_env!("GIT_COMMIT_SHA")
        .or(option_env!("VERGEN_GIT_SHA"))
        .unwrap_or("unknown");
    let elapsed_ms = started.elapsed().as_millis();

    let mut response = json!({
        "status": status,
        "version": VERSION,
        "commit": commit,
        "timestamp": Utc::now(),
        "uptime_secs": state.started_at.elapsed().as_secs(),
        "dependencies": {
            "database": database,
            "cache": cache,
            "external_apis": external_apis
        },
        "request_metrics": {
            "in_flight": metrics::HTTP_IN_FLIGHT.get(),
            "sampled_from_prometheus": true
        },
        "response_time_ms": elapsed_ms
    });

    if query.detailed {
        response["details"] = json!({
            "shutting_down": is_shutting_down,
            "cache": {
                "max_capacity": cache_config.max_capacity,
                "metadata_ttl_secs": cache_config.metadata_ttl_secs,
                "abi_ttl_secs": cache_config.abi_ttl_secs,
                "stats_ttl_secs": cache_config.stats_ttl_secs
            },
            "database_pool": {
                "size": state.db.size(),
                "idle": state.db.num_idle()
            }
        });
    }

    (status_code, Json(response))
}

/// GET /api/v1/contracts/:id/vulnerability-assessment
pub async fn vulnerability_assessment_v1(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let (contract_uuid, contract_address) = fetch_contract_identity(&state, &id).await?;
    let cache_key = format!("v1:{}:{}", contract_uuid, contract_address);

    if let Some(cached) = state.cache.get_vulnerability_assessment(&cache_key).await {
        if let Ok(value) = serde_json::from_str::<Value>(&cached) {
            return Ok(Json(value));
        }
    }

    let contract = fetch_metadata_row(&state, contract_uuid).await?;
    let findings = fetch_security_findings(&state, contract_uuid).await?;
    let dependency_risks = fetch_dependency_risks(&state, contract_uuid).await?;
    let audit_scores = fetch_audit_scores(&state, contract_uuid).await?;
    let audit_event_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM contract_audit_log WHERE contract_id = $1")
            .bind(contract_uuid)
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

    let age_days = (Utc::now() - contract.created_at).num_days().max(0);
    let days_since_update = (Utc::now() - contract.updated_at).num_days().max(0);
    let maintenance_status = if days_since_update > 365 {
        "stale"
    } else if days_since_update > 180 {
        "needs_review"
    } else {
        "maintained"
    };

    let mut risk_score = 0_i32;
    let mut risk_factors = Vec::new();

    if !contract.is_verified {
        risk_score += 25;
        risk_factors.push(json!({
            "type": "verification",
            "severity": "high",
            "description": "Contract is not verified"
        }));
    }

    if days_since_update > 365 {
        risk_score += 20;
        risk_factors.push(json!({
            "type": "maintenance",
            "severity": "medium",
            "description": "Contract has not been updated in over one year"
        }));
    } else if days_since_update > 180 {
        risk_score += 10;
        risk_factors.push(json!({
            "type": "maintenance",
            "severity": "low",
            "description": "Contract has not been updated in over six months"
        }));
    }

    for finding in &findings {
        risk_score += severity_weight(&finding.severity);
        risk_factors.push(json!({
            "type": "vulnerability",
            "severity": finding.severity,
            "description": finding.title
        }));
    }

    for dep in &dependency_risks {
        if let Some(severity) = &dep.severity {
            risk_score += (severity_weight(severity) / 2).max(3);
            risk_factors.push(json!({
                "type": "dependency",
                "severity": severity,
                "description": format!("Dependency {} has known vulnerability {}", dep.package_name, dep.cve_id.clone().unwrap_or_else(|| "unknown".to_string()))
            }));
        }
    }

    let audit_total_score = audit_scores
        .get("total_score")
        .and_then(Value::as_i64)
        .unwrap_or(i64::from(contract.health_score));
    if audit_event_count == 0 {
        risk_score += 10;
        risk_factors.push(json!({
            "type": "audit_history",
            "severity": "low",
            "description": "No audit history is recorded for this contract"
        }));
    } else if audit_total_score < 50 {
        risk_score += 15;
    }

    let risk_score = risk_score.clamp(0, 100);
    let risk_level = risk_level(risk_score);
    let recommendations = recommendations_for(&risk_factors, risk_level);

    let vulnerabilities: Vec<Value> = findings
        .into_iter()
        .map(|finding| {
            json!({
                "id": finding.id,
                "title": finding.title,
                "severity": finding.severity,
                "description": finding.description,
                "source": finding.source
            })
        })
        .collect();

    let dependency_security: Vec<Value> = dependency_risks
        .into_iter()
        .map(|dep| {
            json!({
                "package_name": dep.package_name,
                "package_version": dep.package_version,
                "cve_id": dep.cve_id,
                "severity": dep.severity,
                "status": if dep.cve_id.is_some() { "vulnerable" } else { "no_known_vulnerability" }
            })
        })
        .collect();

    let response = json!({
        "contract_id": contract.contract_id,
        "risk_score": risk_score,
        "risk_level": risk_level,
        "vulnerabilities": vulnerabilities,
        "risk_factors": risk_factors,
        "known_exploits": vulnerabilities_with_known_exploit_flag(&vulnerabilities),
        "code_age": {
            "created_at": contract.created_at,
            "age_days": age_days,
            "last_updated_at": contract.updated_at,
            "days_since_update": days_since_update,
            "maintenance_status": maintenance_status
        },
        "audit_history": {
            "audit_event_count": audit_event_count,
            "scores": audit_scores
        },
        "dependency_security": dependency_security,
        "recommendations": recommendations,
        "cached_for_seconds": 24 * 3600,
        "generated_at": Utc::now()
    });

    if let Ok(serialized) = serde_json::to_string(&response) {
        state
            .cache
            .put_vulnerability_assessment(&cache_key, serialized)
            .await;
    }

    Ok(Json(response))
}

async fn fetch_metadata_row(
    state: &AppState,
    contract_uuid: Uuid,
) -> ApiResult<ContractMetadataRow> {
    sqlx::query_as::<_, ContractMetadataRow>(
        r#"
        SELECT
            c.id,
            c.contract_id,
            c.wasm_hash,
            c.name,
            c.description,
            c.publisher_id,
            c.network,
            c.is_verified,
            c.verification_status::TEXT AS verification_status,
            c.category,
            c.created_at,
            c.updated_at,
            c.verified_at,
            c.deployed_at,
            c.last_accessed_at,
            c.health_score,
            c.current_version,
            c.usage_count,
            p.stellar_address AS owner_address,
            p.username AS owner_username,
            p.email AS owner_email,
            p.github_url AS owner_github_url,
            p.website AS owner_website
        FROM contracts c
        JOIN publishers p ON p.id = c.publisher_id
        WHERE c.id = $1
        "#,
    )
    .bind(contract_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => ApiError::not_found("ContractNotFound", "Contract not found"),
        _ => db_internal_error("fetch v1 contract metadata", err),
    })
}

async fn fetch_tags(state: &AppState, contract_uuid: Uuid) -> ApiResult<Vec<Value>> {
    let rows = sqlx::query_as::<_, TagRow>(
        "SELECT t.id, t.name, t.color FROM tags t JOIN contract_tags ct ON t.id = ct.tag_id WHERE ct.contract_id = $1 ORDER BY t.name ASC",
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch v1 metadata tags", err))?;

    Ok(rows
        .into_iter()
        .map(|tag| json!({"id": tag.id, "name": tag.name, "color": tag.color}))
        .collect())
}

async fn fetch_last_interaction(
    state: &AppState,
    contract_uuid: Uuid,
) -> ApiResult<Option<DateTime<Utc>>> {
    sqlx::query_scalar("SELECT MAX(created_at) FROM contract_interactions WHERE contract_id = $1")
        .bind(contract_uuid)
        .fetch_one(&state.db)
        .await
        .map_err(|err| db_internal_error("fetch v1 metadata last interaction", err))
}

async fn fetch_audit_scores(state: &AppState, contract_uuid: Uuid) -> ApiResult<Value> {
    let row = sqlx::query_as::<_, AuditScoreRow>(
        r#"
        SELECT
            status,
            security_score,
            total_score,
            audit_date,
            updated_at
        FROM contract_health
        WHERE contract_id = $1
        "#,
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch v1 audit scores", err))?;

    Ok(match row {
        Some(row) => json!({
            "status": row.status,
            "security_score": row.security_score.unwrap_or(0),
            "total_score": row.total_score.unwrap_or(0),
            "audit_date": row.audit_date,
            "updated_at": row.updated_at
        }),
        None => json!({
            "status": "unknown",
            "security_score": null,
            "total_score": null,
            "audit_date": null,
            "updated_at": null
        }),
    })
}

async fn fetch_related_contracts(
    state: &AppState,
    contract_uuid: Uuid,
    contract: &ContractMetadataRow,
) -> ApiResult<Vec<Value>> {
    let rows = sqlx::query_as::<_, RelatedContractRow>(
        r#"
        SELECT
            c.id,
            c.contract_id,
            c.name,
            c.network,
            c.category,
            (
                CASE WHEN c.category IS NOT DISTINCT FROM $2 THEN 2 ELSE 0 END
                + COALESCE(tag_match.matches, 0)
            )::BIGINT AS relation_score
        FROM contracts c
        LEFT JOIN LATERAL (
            SELECT COUNT(*)::BIGINT AS matches
            FROM contract_tags current_ct
            JOIN contract_tags other_ct ON other_ct.tag_id = current_ct.tag_id
            WHERE current_ct.contract_id = $1 AND other_ct.contract_id = c.id
        ) tag_match ON true
        WHERE c.id <> $1
          AND c.network = $3
          AND (c.category IS NOT DISTINCT FROM $2 OR COALESCE(tag_match.matches, 0) > 0)
        ORDER BY relation_score DESC, c.updated_at DESC
        LIMIT 10
        "#,
    )
    .bind(contract_uuid)
    .bind(&contract.category)
    .bind(&contract.network)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch v1 related contracts", err))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.id,
                "contract_id": row.contract_id,
                "name": row.name,
                "network": row.network,
                "category": row.category,
                "relation_score": row.relation_score
            })
        })
        .collect())
}

async fn fetch_deployments(state: &AppState, contract_uuid: Uuid) -> ApiResult<Vec<Value>> {
    let rows = sqlx::query_as::<_, DeploymentRow>(
        r#"
        SELECT
            id,
            environment::TEXT AS environment,
            status::TEXT AS status,
            wasm_hash,
            deployed_at,
            activated_at,
            health_checks_passed,
            health_checks_failed,
            last_health_check_at,
            error_message
        FROM contract_deployments
        WHERE contract_id = $1
        ORDER BY deployed_at DESC
        "#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch v1 deployments", err))?;

    Ok(rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.id,
                "environment": row.environment,
                "status": row.status,
                "wasm_hash": row.wasm_hash,
                "deployed_at": row.deployed_at,
                "activated_at": row.activated_at,
                "health_checks_passed": row.health_checks_passed.unwrap_or(0),
                "health_checks_failed": row.health_checks_failed.unwrap_or(0),
                "last_health_check_at": row.last_health_check_at,
                "error_message": row.error_message
            })
        })
        .collect())
}

fn parse_bulk_verify_payload(payload: Value) -> Result<Vec<BatchVerifyItem>, ApiError> {
    let value = if let Some(contracts) = payload.get("contracts") {
        contracts.clone()
    } else {
        payload
    };

    let array = value.as_array().ok_or_else(|| {
        ApiError::bad_request(
            "InvalidRequest",
            "request body must be a JSON array of contract addresses or objects",
        )
    })?;

    array
        .iter()
        .map(|entry| {
            if let Some(contract_id) = entry.as_str() {
                return Ok(BatchVerifyItem {
                    contract_id: contract_id.to_string(),
                    source_code: None,
                    build_params: None,
                    compiler_version: None,
                    level: None,
                });
            }

            let obj = entry.as_object().ok_or_else(|| {
                ApiError::bad_request(
                    "InvalidContractEntry",
                    "each contract must be a string address or object",
                )
            })?;
            let contract_id = obj
                .get("contract_id")
                .or_else(|| obj.get("address"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ApiError::bad_request(
                        "InvalidContractEntry",
                        "contract object must include contract_id or address",
                    )
                })?;

            Ok(BatchVerifyItem {
                contract_id: contract_id.to_string(),
                source_code: obj
                    .get("source_code")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                build_params: obj.get("build_params").cloned(),
                compiler_version: obj
                    .get("compiler_version")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                level: obj
                    .get("level")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
            })
        })
        .collect()
}

async fn run_bulk_verify_job(job_id: Uuid, contracts: Vec<BatchVerifyItem>, state: AppState) {
    let mut response = verify_contract_batch(&state, contracts).await;
    response["job_id"] = json!(job_id);
    response["status"] = json!("completed");
    response["completed_at"] = json!(Utc::now());

    let mut jobs = BULK_VERIFY_JOBS.write().await;
    if let Some(job) = jobs.get_mut(&job_id) {
        job.response = response;
    }
}

async fn verify_contract_batch(state: &AppState, contracts: Vec<BatchVerifyItem>) -> Value {
    let verifier = OnChainVerifier::new();
    let verified_at = Utc::now();
    let results = stream::iter(contracts.into_iter().map(|item| {
        let state = state.clone();
        let verifier = verifier.clone();
        async move { verify_contract_item_v1(&state, &verifier, item, verified_at).await }
    }))
    .buffer_unordered(8)
    .collect::<Vec<Value>>()
    .await;

    let verified = results
        .iter()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some("verified"))
        .count();
    let failed = results
        .iter()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some("failed"))
        .count();
    let not_found = results
        .iter()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some("not-found"))
        .count();

    json!({
        "total": results.len(),
        "verified": verified,
        "failed": failed,
        "not_found": not_found,
        "verified_at": verified_at,
        "results": results
    })
}

async fn verify_contract_item_v1(
    state: &AppState,
    onchain_verifier: &OnChainVerifier,
    item: BatchVerifyItem,
    verified_at: DateTime<Utc>,
) -> Value {
    let contract = match fetch_metadata_by_address(state, &item.contract_id).await {
        Ok(Some(contract)) => contract,
        Ok(None) => {
            return json!({
                "contract_id": item.contract_id,
                "status": "not-found",
                "verified": false,
                "verification_timestamp": verified_at,
                "signature_validity": {
                    "valid": false,
                    "status": "not_found"
                },
                "anomalies": ["contract_not_found"]
            });
        }
        Err(err) => {
            return json!({
                "contract_id": item.contract_id,
                "status": "failed",
                "verified": false,
                "verification_timestamp": verified_at,
                "signature_validity": {
                    "valid": false,
                    "status": "unknown"
                },
                "anomalies": [err.to_string()]
            });
        }
    };

    let mut anomalies = Vec::new();
    let abi_json = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT abi FROM contract_abis WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(contract.id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
    .map(|value| value.to_string());

    let on_chain = match onchain_verifier
        .verify_contract(
            &state.cache,
            &metadata_as_contract(&contract),
            abi_json.as_deref(),
        )
        .await
    {
        Ok(result) => result,
        Err(err) => {
            anomalies.push(err.to_string());
            return json!({
                "contract_id": contract.contract_id,
                "status": "failed",
                "verified": false,
                "network": contract.network,
                "verification_timestamp": verified_at,
                "signature_validity": signature_validity(state, contract.id).await,
                "anomalies": anomalies
            });
        }
    };

    if !on_chain.contract_exists_on_chain {
        anomalies.push("contract_not_found_on_chain".to_string());
    }
    if !on_chain.wasm_hash_matches {
        anomalies.push("wasm_hash_mismatch".to_string());
    }
    if !on_chain.abi_valid {
        anomalies.push("abi_invalid_or_missing".to_string());
    }
    for reason in &on_chain.failure_reasons {
        anomalies.push(reason.to_string());
    }

    let signature = signature_validity(state, contract.id).await;
    if signature.get("valid").and_then(Value::as_bool) != Some(true) {
        anomalies.push("signature_not_valid".to_string());
    }

    let verified = on_chain.contract_exists_on_chain
        && on_chain.wasm_hash_matches
        && on_chain.abi_valid
        && signature.get("valid").and_then(Value::as_bool) == Some(true);

    json!({
        "contract_id": contract.contract_id,
        "status": if verified { "verified" } else { "failed" },
        "verified": verified,
        "network": contract.network,
        "verification_timestamp": verified_at,
        "signature_validity": signature,
        "on_chain": {
            "contract_exists": on_chain.contract_exists_on_chain,
            "wasm_hash_matches": on_chain.wasm_hash_matches,
            "abi_valid": on_chain.abi_valid,
            "cached": on_chain.cached
        },
        "anomalies": anomalies
    })
}

async fn fetch_metadata_by_address(
    state: &AppState,
    contract_id: &str,
) -> ApiResult<Option<ContractMetadataRow>> {
    sqlx::query_as::<_, ContractMetadataRow>(
        r#"
        SELECT
            c.id,
            c.contract_id,
            c.wasm_hash,
            c.name,
            c.description,
            c.publisher_id,
            c.network,
            c.is_verified,
            c.verification_status::TEXT AS verification_status,
            c.category,
            c.created_at,
            c.updated_at,
            c.verified_at,
            c.deployed_at,
            c.last_accessed_at,
            c.health_score,
            c.current_version,
            c.usage_count,
            p.stellar_address AS owner_address,
            p.username AS owner_username,
            p.email AS owner_email,
            p.github_url AS owner_github_url,
            p.website AS owner_website
        FROM contracts c
        JOIN publishers p ON p.id = c.publisher_id
        WHERE c.contract_id = $1
        ORDER BY c.created_at DESC
        LIMIT 1
        "#,
    )
    .bind(contract_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch v1 contract by address", err))
}

fn metadata_as_contract(row: &ContractMetadataRow) -> shared::Contract {
    shared::Contract {
        id: row.id,
        contract_id: row.contract_id.clone(),
        wasm_hash: row.wasm_hash.clone(),
        name: row.name.clone(),
        slug: row.contract_id.clone(),
        description: row.description.clone(),
        publisher_id: row.publisher_id,
        network: row.network.clone(),
        is_verified: row.is_verified,
        verification_status: match row.verification_status.as_str() {
            "verified" => shared::VerificationStatus::Verified,
            "pending" => shared::VerificationStatus::Pending,
            "failed" => shared::VerificationStatus::Failed,
            _ => shared::VerificationStatus::Unverified,
        },
        category: row.category.clone(),
        tags: Vec::new(),
        created_at: row.created_at,
        updated_at: row.updated_at,
        verified_at: row.verified_at,
        deployed_at: row.deployed_at,
        verified_by: None,
        verification_notes: None,
        last_accessed_at: row.last_accessed_at,
        health_score: row.health_score,
        is_maintenance: false,
        logical_id: None,
        network_configs: None,
        relevance_score: None,
        organization_id: None,
        visibility: shared::VisibilityType::Public,
        current_version: row.current_version.clone(),
        usage_count: row.usage_count,
    }
}

async fn signature_validity(state: &AppState, contract_uuid: Uuid) -> Value {
    let row = sqlx::query_as::<_, (i64, i64)>(
        r#"
        SELECT
            COUNT(*)::BIGINT AS total,
            COUNT(*) FILTER (
                WHERE cs.verified = true
                  AND (cs.not_before IS NULL OR cs.not_before <= NOW())
                  AND (cs.expires_at IS NULL OR cs.expires_at > NOW())
                  AND sk.status = 'active'
                  AND sr_sig.id IS NULL
                  AND sr_key.id IS NULL
            )::BIGINT AS valid
        FROM contract_signatures cs
        LEFT JOIN signing_keys sk ON sk.key_id = cs.key_id
        LEFT JOIN signature_revocations sr_sig ON sr_sig.signature_id = cs.id
        LEFT JOIN signature_revocations sr_key ON sr_key.key_id = cs.key_id
        WHERE cs.contract_id = $1
        "#,
    )
    .bind(contract_uuid)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok((total, valid)) => json!({
            "valid": valid > 0,
            "status": if valid > 0 { "valid" } else if total > 0 { "invalid" } else { "missing" },
            "signatures_checked": total,
            "valid_signatures": valid
        }),
        Err(_) => json!({
            "valid": false,
            "status": "unavailable",
            "signatures_checked": 0,
            "valid_signatures": 0
        }),
    }
}

async fn fetch_security_findings(
    state: &AppState,
    contract_uuid: Uuid,
) -> ApiResult<Vec<SecurityFindingRow>> {
    let mut findings = sqlx::query_as::<_, SecurityFindingRow>(
        r#"
        SELECT
            cv.cve_id AS id,
            cv.title,
            cv.severity,
            cv.description,
            'dependency_scan' AS source
        FROM contract_scan_results csr
        JOIN cve_vulnerabilities cv ON cv.cve_id = csr.cve_id
        WHERE csr.contract_id = $1
          AND csr.is_false_positive = false
        "#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch v1 dependency findings", err))?;

    let scan_findings = sqlx::query_as::<_, SecurityFindingRow>(
        r#"
        SELECT
            si.id::TEXT AS id,
            si.title,
            si.severity::TEXT AS severity,
            si.description,
            'security_scan' AS source
        FROM security_issues si
        JOIN security_scans ss ON ss.id = si.scan_id
        WHERE ss.contract_id = $1
          AND si.status::TEXT NOT IN ('resolved', 'false_positive')
        "#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    findings.extend(scan_findings);
    Ok(findings)
}

async fn fetch_dependency_risks(
    state: &AppState,
    contract_uuid: Uuid,
) -> ApiResult<Vec<DependencyRiskRow>> {
    sqlx::query_as::<_, DependencyRiskRow>(
        r#"
        SELECT
            cpd.package_name,
            cpd.version AS package_version,
            cv.cve_id,
            cv.severity
        FROM contract_package_dependencies cpd
        LEFT JOIN cve_vulnerabilities cv
            ON cv.package_name = cpd.package_name
        WHERE cpd.contract_id = $1
        ORDER BY cv.severity DESC NULLS LAST, cpd.package_name ASC
        LIMIT 50
        "#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|err| db_internal_error("fetch v1 dependency risks", err))
}

fn severity_weight(severity: &str) -> i32 {
    match severity.to_ascii_lowercase().as_str() {
        "critical" => 35,
        "high" => 25,
        "medium" => 15,
        "low" => 7,
        _ => 3,
    }
}

fn risk_level(score: i32) -> &'static str {
    match score {
        0..=24 => "low",
        25..=49 => "medium",
        50..=74 => "high",
        _ => "critical",
    }
}

fn recommendations_for(risk_factors: &[Value], risk_level: &str) -> Vec<Value> {
    let mut recommendations = Vec::new();

    if risk_factors.iter().any(|f| f["type"] == "verification") {
        recommendations.push(json!({
            "priority": "high",
            "action": "Complete source and on-chain verification before integrating this contract"
        }));
    }

    if risk_factors.iter().any(|f| f["type"] == "vulnerability") {
        recommendations.push(json!({
            "priority": "high",
            "action": "Review open security findings and require remediation or compensating controls"
        }));
    }

    if risk_factors.iter().any(|f| f["type"] == "dependency") {
        recommendations.push(json!({
            "priority": "medium",
            "action": "Upgrade vulnerable dependencies and re-run dependency scanning"
        }));
    }

    if risk_factors.iter().any(|f| f["type"] == "maintenance") {
        recommendations.push(json!({
            "priority": "medium",
            "action": "Confirm maintainer activity and review recent deployment history"
        }));
    }

    if recommendations.is_empty() {
        recommendations.push(json!({
            "priority": "low",
            "action": "Continue routine monitoring and re-assess after new deployments or audits"
        }));
    }

    if risk_level == "critical" {
        recommendations.insert(
            0,
            json!({
                "priority": "critical",
                "action": "Do not integrate until critical risks are reviewed and explicitly accepted"
            }),
        );
    }

    recommendations
}

fn vulnerabilities_with_known_exploit_flag(vulnerabilities: &[Value]) -> Vec<Value> {
    vulnerabilities
        .iter()
        .filter(|v| {
            let severity = v
                .get("severity")
                .and_then(Value::as_str)
                .unwrap_or_default();
            matches!(severity.to_ascii_lowercase().as_str(), "critical" | "high")
        })
        .map(|v| {
            json!({
                "id": v.get("id").cloned().unwrap_or(Value::Null),
                "title": v.get("title").cloned().unwrap_or(Value::Null),
                "severity": v.get("severity").cloned().unwrap_or(Value::Null),
                "known_exploit": true
            })
        })
        .collect()
}
