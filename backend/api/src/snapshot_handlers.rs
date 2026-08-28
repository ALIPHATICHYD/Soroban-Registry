//! Signed contract snapshots (Issue #1116).
//!
//! Assembles a contract's lifecycle state into a single JSON document and signs
//! it with the registry's key, so auditors can verify it offline without
//! reaching this API. The document shape, canonical form and verification logic
//! all live in `shared::snapshot` so the CLI signs and verifies the same bytes.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Utc};
use ed25519_dalek::{SigningKey, SECRET_KEY_LENGTH};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::snapshot::{
    key_fingerprint, sign_snapshot, ContractSnapshot, LineageLink, SnapshotContract,
    SnapshotPayload, SnapshotVerification, SNAPSHOT_ALGORITHM, SNAPSHOT_SCHEMA_VERSION,
};
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    handlers::db_internal_error,
    state::AppState,
};

/// Query parameters for GET /api/contracts/:id/snapshot
#[derive(Debug, Default, Deserialize)]
pub struct SnapshotQuery {
    /// Embed the dependency graph and transitive risk report (Issue #1147).
    /// Off by default: it is large, and including it raises the payload to
    /// schema 1.1.
    pub include_dependency_graph: Option<bool>,
}

const ENV_SIGNING_KEY: &str = "REGISTRY_SIGNING_KEY";
const ENV_REGISTRY_URL: &str = "REGISTRY_PUBLIC_URL";

/// Guards against a cycle in `replacement_contract_id` turning lineage walking
/// into an unbounded loop.
const MAX_LINEAGE_DEPTH: usize = 32;

/// The registry's published signing identity, for pinning before offline
/// verification.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RegistrySigningKey {
    pub algorithm: String,
    pub public_key: String,
    pub key_fingerprint: String,
}

/// Load the registry signing key from the environment.
///
/// Kept out of `AppState` so a deployment that never exports snapshots does not
/// need the key configured at all; the endpoints return 503 instead of the
/// process refusing to boot.
fn load_signing_key() -> Result<SigningKey, ApiError> {
    let raw = std::env::var(ENV_SIGNING_KEY).map_err(|_| {
        ApiError::service_unavailable_with(
            "SNAPSHOT_SIGNING_UNAVAILABLE",
            "REGISTRY_SIGNING_KEY is not configured; contract snapshots cannot be signed",
        )
    })?;

    let seed = BASE64
        .decode(raw.trim())
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(raw.trim()))
        .map_err(|_| {
            ApiError::internal_error(
                "SNAPSHOT_SIGNING_KEY_INVALID",
                "REGISTRY_SIGNING_KEY is not valid base64",
            )
        })?;

    let seed: [u8; SECRET_KEY_LENGTH] = seed.as_slice().try_into().map_err(|_| {
        ApiError::internal_error(
            "SNAPSHOT_SIGNING_KEY_INVALID",
            "REGISTRY_SIGNING_KEY must decode to exactly 32 bytes",
        )
    })?;

    Ok(SigningKey::from_bytes(&seed))
}

/// GET /api/registry/signing-key
///
/// Publishes the public half so a verifier can pin the fingerprint out of band.
/// Without pinning, a snapshot only proves internal consistency: anyone can
/// re-sign a modified payload with their own key and embed that public key.
#[utoipa::path(
    get,
    path = "/api/registry/signing-key",
    tag = "snapshots",
    responses(
        (status = 200, description = "Registry snapshot signing key", body = RegistrySigningKey),
        (status = 503, description = "Signing key not configured")
    )
)]
pub async fn get_registry_signing_key() -> ApiResult<Json<RegistrySigningKey>> {
    let key = load_signing_key()?;
    let verifying = key.verifying_key();

    Ok(Json(RegistrySigningKey {
        algorithm: SNAPSHOT_ALGORITHM.to_string(),
        public_key: BASE64.encode(verifying.as_bytes()),
        key_fingerprint: key_fingerprint(verifying.as_bytes()),
    }))
}

type ContractRow = (
    Uuid,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<Uuid>,
    bool,
    DateTime<Utc>,
    DateTime<Utc>,
);

/// GET /api/contracts/:id/snapshot
///
/// Returns a signed, self-contained record of the contract's lifecycle state as
/// of now.
#[utoipa::path(
    get,
    path = "/api/contracts/{id}/snapshot",
    tag = "snapshots",
    params(
        ("id" = Uuid, Path, description = "Contract UUID"),
        ("include_dependency_graph" = Option<bool>, Query, description = "Embed the dependency graph and transitive risk report. Raises the payload to schema 1.1, which older verifiers reject as UnsupportedSchema rather than mis-reporting a signature failure.")
    ),
    responses(
        (status = 200, description = "Signed contract snapshot"),
        (status = 404, description = "Contract not found"),
        (status = 503, description = "Signing key not configured")
    )
)]
pub async fn get_contract_snapshot(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<SnapshotQuery>,
) -> ApiResult<Json<ContractSnapshot>> {
    let signing_key = load_signing_key()?;

    let row: Option<ContractRow> = sqlx::query_as(
        r#"
        SELECT id, contract_id, name, network::text, wasm_hash,
               description, category, publisher_id, is_verified,
               created_at, updated_at
        FROM contracts
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| db_internal_error("fetch contract for snapshot", e))?;

    let Some((
        contract_uuid,
        contract_id,
        name,
        network,
        wasm_hash,
        description,
        category,
        publisher_id,
        is_verified,
        created_at,
        updated_at,
    )) = row
    else {
        return Err(ApiError::not_found(
            "ContractNotFound",
            "Contract not found",
        ));
    };

    let verification = fetch_verification(&state, contract_uuid).await?;
    let dependency_scan = fetch_dependency_scan(&state, contract_uuid).await?;
    let deprecation = fetch_deprecation(&state, contract_uuid).await?;
    let lineage = fetch_lineage(&state, contract_uuid).await?;
    let policy_evaluation = fetch_policy_evaluation(&state, contract_uuid).await?;

    // Off by default. The graph is large, and its presence changes the schema
    // version, so it is never added to a snapshot that did not ask for it.
    let dependency_graph = if query.include_dependency_graph.unwrap_or(false) {
        // `Network` has no FromStr; the row was selected as text, so map it
        // back explicitly rather than round-tripping through serde.
        let parsed_network = match network.as_str() {
            "mainnet" => shared::Network::Mainnet,
            "testnet" => shared::Network::Testnet,
            "futurenet" => shared::Network::Futurenet,
            other => {
                return Err(ApiError::internal(format!(
                    "contract has an unrecognized network: {other}"
                )))
            }
        };
        let report = crate::dependency_risk::assess(
            &state,
            contract_uuid,
            parsed_network,
            shared::dependency_graph::DEFAULT_MAX_DEPTH,
            None,
        )
        .await?;
        Some(serde_json::to_value(&report).map_err(|e| {
            ApiError::internal(format!("failed to serialize dependency graph: {e}"))
        })?)
    } else {
        None
    };

    let mut payload = SnapshotPayload {
        schema_version: SNAPSHOT_SCHEMA_VERSION.to_string(),
        exported_at: Utc::now(),
        registry_url: std::env::var(ENV_REGISTRY_URL).ok(),
        contract: SnapshotContract {
            id: contract_uuid.to_string(),
            contract_id,
            name,
            network,
            wasm_hash,
            description,
            category,
            publisher_id: publisher_id.map(|p| p.to_string()),
            created_at,
            updated_at,
        },
        verification: SnapshotVerification {
            is_verified,
            status: verification.as_ref().map(|(s, _)| s.clone()),
            verified_at: verification.and_then(|(_, at)| at),
        },
        dependency_scan,
        deprecation,
        policy_evaluation,
        lineage,
        dependency_graph,
    };

    // Derived from the content, never from the request, so a graph-bearing
    // payload cannot claim to be 1.0 and mislead an older verifier into
    // reporting a signature failure.
    payload.schema_version = payload.required_schema_version().to_string();

    let snapshot = sign_snapshot(payload, &signing_key).map_err(|e| {
        ApiError::internal_error(
            "SNAPSHOT_SIGN_FAILED",
            format!("failed to sign snapshot: {e}"),
        )
    })?;

    Ok(Json(snapshot))
}

/// Latest verification attempt, if the contract has ever been verified.
async fn fetch_verification(
    state: &AppState,
    contract_uuid: Uuid,
) -> ApiResult<Option<(String, Option<DateTime<Utc>>)>> {
    sqlx::query_as::<_, (String, Option<DateTime<Utc>>)>(
        r#"
        SELECT status::text, verified_at
        FROM verifications
        WHERE contract_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| db_internal_error("fetch verification for snapshot", e))
}

/// Most recent dependency scan, as recorded findings rather than a fresh scan:
/// a snapshot records state as of export, and must not mutate anything.
async fn fetch_dependency_scan(state: &AppState, contract_uuid: Uuid) -> ApiResult<Option<Value>> {
    let findings: Vec<(String, String, String, Option<String>, bool)> = sqlx::query_as(
        r#"
        SELECT r.cve_id, r.package_name, r.current_version,
               r.recommended_version, r.is_false_positive
        FROM contract_scan_results r
        WHERE r.contract_id = $1
        ORDER BY r.cve_id
        "#,
    )
    .bind(contract_uuid)
    .fetch_all(&state.db)
    .await
    .map_err(|e| db_internal_error("fetch scan results for snapshot", e))?;

    let last_scanned_at: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT MAX(scanned_at) FROM contract_dependency_scan_runs WHERE contract_id = $1",
    )
    .bind(contract_uuid)
    .fetch_one(&state.db)
    .await
    .map_err(|e| db_internal_error("fetch scan runs for snapshot", e))?;

    if findings.is_empty() && last_scanned_at.is_none() {
        return Ok(None);
    }

    let findings: Vec<Value> = findings
        .into_iter()
        .map(
            |(cve_id, package_name, current_version, recommended_version, is_false_positive)| {
                serde_json::json!({
                    "cve_id": cve_id,
                    "package_name": package_name,
                    "current_version": current_version,
                    "recommended_version": recommended_version,
                    "is_false_positive": is_false_positive,
                })
            },
        )
        .collect();

    Ok(Some(serde_json::json!({
        "last_scanned_at": last_scanned_at,
        "finding_count": findings.len(),
        "findings": findings,
    })))
}

/// Deprecation state, omitted entirely for a contract that has never been
/// deprecated so the payload stays honest about what is known.
async fn fetch_deprecation(state: &AppState, contract_uuid: Uuid) -> ApiResult<Option<Value>> {
    let row: Option<(Option<DateTime<Utc>>, Option<String>, Option<Uuid>, String)> =
        sqlx::query_as(
            r#"
            SELECT deprecated_at, deprecation_reason, replacement_contract_id,
                   COALESCE(deprecation_status::text, 'active')
            FROM contracts
            WHERE id = $1
            "#,
        )
        .bind(contract_uuid)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| db_internal_error("fetch deprecation for snapshot", e))?;

    let Some((deprecated_at, reason, replacement, status)) = row else {
        return Ok(None);
    };

    if deprecated_at.is_none() && replacement.is_none() && status == "active" {
        return Ok(None);
    }

    Ok(Some(serde_json::json!({
        "status": status,
        "deprecated_at": deprecated_at,
        "reason": reason,
        "replacement_contract_id": replacement.map(|r| r.to_string()),
    })))
}

/// Walk `replacement_contract_id` forward to build the successor chain.
///
/// Stops at `MAX_LINEAGE_DEPTH` and on revisiting a contract, so a cycle in the
/// data cannot hang the request.
async fn fetch_lineage(state: &AppState, contract_uuid: Uuid) -> ApiResult<Vec<LineageLink>> {
    let mut chain = Vec::new();
    let mut seen = vec![contract_uuid];
    let mut current = contract_uuid;

    for _ in 0..MAX_LINEAGE_DEPTH {
        let next: Option<Uuid> =
            sqlx::query_scalar("SELECT replacement_contract_id FROM contracts WHERE id = $1")
                .bind(current)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| db_internal_error("fetch lineage pointer for snapshot", e))?
                .flatten();

        let Some(next_id) = next else { break };
        if seen.contains(&next_id) {
            break;
        }

        let row: Option<(String, Option<String>, Option<DateTime<Utc>>, String)> = sqlx::query_as(
            r#"
            SELECT contract_id, name, deprecated_at,
                   COALESCE(deprecation_status::text, 'active')
            FROM contracts
            WHERE id = $1
            "#,
        )
        .bind(next_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| db_internal_error("fetch lineage entry for snapshot", e))?;

        let Some((contract_id, name, deprecated_at, status)) = row else {
            break;
        };

        chain.push(LineageLink {
            contract_id,
            name,
            status,
            deprecated_at,
        });

        seen.push(next_id);
        current = next_id;
    }

    Ok(chain)
}

/// Fetch the most recent policy evaluation result from the audit log for this
/// contract. Policy evaluations are stored as part of the `ContractPublished`
/// action's `new_value` field.
async fn fetch_policy_evaluation(
    state: &AppState,
    contract_uuid: Uuid,
) -> ApiResult<Option<Value>> {
    let row: Option<(Value,)> = sqlx::query_as(
        r#"
        SELECT new_value
        FROM contract_audit_log
        WHERE contract_id = $1
          AND action_type = 'contract_published'
        ORDER BY timestamp DESC
        LIMIT 1
        "#,
    )
    .bind(contract_uuid)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| db_internal_error("fetch policy evaluation for snapshot", e))?;

    Ok(row.and_then(|(new_val,)| {
        new_val
            .get("policy_evaluation")
            .filter(|v| !v.is_null())
            .cloned()
    }))
}
