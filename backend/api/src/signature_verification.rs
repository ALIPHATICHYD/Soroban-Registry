// Issue #888: Contract signature verification system.
//
// Cryptographic authentication of contracts via deployer signatures. Provides:
//   • Multi-algorithm verification: Ed25519 and secp256k1 (ECDSA/SHA-256).
//   • Deployer-key registration with fingerprints, validity windows, metadata.
//   • Certificate-chain validation up to a trusted root.
//   • A revocation list for keys and individual signatures.
//   • Signature timestamp validation (validity windows + expiry).
//   • Key rotation that preserves the validity of previously-issued signatures.
//   • An in-memory verification-result cache for performance.
//
// The pure crypto core (`verify_signature`, `SignatureAlgorithm`, `fingerprint`)
// has no DB dependency and is exercised by integration tests.

use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::time::Instant;

use axum::{
    extract::{Path, State},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::ApiError;
use crate::state::AppState;

const MAX_CHAIN_DEPTH: usize = 10;
const CACHE_CAPACITY: usize = 10_000;

fn cache_ttl_secs() -> u64 {
    std::env::var("SIGNATURE_CACHE_TTL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(300)
}

// ── Algorithms ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SignatureAlgorithm {
    Ed25519,
    Secp256k1,
}

impl SignatureAlgorithm {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ed25519" => Some(Self::Ed25519),
            "secp256k1" | "ecdsa" | "ecdsa-secp256k1" | "es256k" => Some(Self::Secp256k1),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ed25519 => "ed25519",
            Self::Secp256k1 => "secp256k1",
        }
    }
}

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SigError {
    InvalidKey,
    InvalidSignature,
    VerificationFailed,
}

impl SigError {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidKey => "invalid_public_key",
            Self::InvalidSignature => "invalid_signature_encoding",
            Self::VerificationFailed => "verification_failed",
        }
    }
}

// ── Crypto core (pure, DB-free) ───────────────────────────────────────────────

/// Verify `signature` over `message` for `public_key` under `alg`.
/// `message` is the raw bytes that were signed (the caller decides any hashing
/// of an artifact into this message; secp256k1 additionally SHA-256-prehashes).
pub fn verify_signature(
    alg: SignatureAlgorithm,
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), SigError> {
    match alg {
        SignatureAlgorithm::Ed25519 => verify_ed25519(public_key, message, signature),
        SignatureAlgorithm::Secp256k1 => verify_secp256k1(public_key, message, signature),
    }
}

fn verify_ed25519(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), SigError> {
    use ed25519_dalek::{Signature, VerifyingKey};

    let pk: [u8; 32] = public_key.try_into().map_err(|_| SigError::InvalidKey)?;
    let vk = VerifyingKey::from_bytes(&pk).map_err(|_| SigError::InvalidKey)?;
    let sig = Signature::from_slice(signature).map_err(|_| SigError::InvalidSignature)?;
    // verify_strict rejects small-order / malleable keys.
    vk.verify_strict(message, &sig)
        .map_err(|_| SigError::VerificationFailed)
}

fn verify_secp256k1(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), SigError> {
    use k256::ecdsa::signature::Verifier;
    use k256::ecdsa::{Signature, VerifyingKey};

    // Accept SEC1 compressed (33B) or uncompressed (65B) public keys.
    let vk = VerifyingKey::from_sec1_bytes(public_key).map_err(|_| SigError::InvalidKey)?;
    // Accept fixed 64-byte (r||s) or DER-encoded signatures.
    let sig = Signature::from_slice(signature)
        .or_else(|_| Signature::from_der(signature))
        .map_err(|_| SigError::InvalidSignature)?;
    // `verify` SHA-256-prehashes the message (standard secp256k1 ECDSA).
    vk.verify(message, &sig)
        .map_err(|_| SigError::VerificationFailed)
}

/// Deterministic key fingerprint: hex(sha256(algorithm || ':' || public key)).
pub fn fingerprint(alg: SignatureAlgorithm, public_key: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(alg.as_str().as_bytes());
    h.update(b":");
    h.update(public_key);
    hex::encode(h.finalize())
}

fn decode_message(message: &str, encoding: Option<&str>) -> Result<Vec<u8>, ApiError> {
    match encoding.unwrap_or("hex") {
        "hex" => hex::decode(message.trim())
            .map_err(|_| ApiError::bad_request("INVALID_MESSAGE", "message is not valid hex")),
        "base64" => BASE64
            .decode(message.trim())
            .map_err(|_| ApiError::bad_request("INVALID_MESSAGE", "message is not valid base64")),
        "utf8" => Ok(message.as_bytes().to_vec()),
        _ => Err(ApiError::bad_request(
            "INVALID_ENCODING",
            "encoding must be hex, base64, or utf8",
        )),
    }
}

// ── Verification result cache ─────────────────────────────────────────────────

static VERIFY_CACHE: Lazy<Mutex<lru::LruCache<String, (bool, Instant)>>> = Lazy::new(|| {
    Mutex::new(lru::LruCache::new(
        NonZeroUsize::new(CACHE_CAPACITY).unwrap(),
    ))
});

fn cache_key(alg: SignatureAlgorithm, pk: &[u8], msg: &[u8], sig: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(alg.as_str().as_bytes());
    h.update(pk);
    h.update(msg);
    h.update(sig);
    hex::encode(h.finalize())
}

/// Verify with an in-memory cache keyed by the content of all inputs. Returns
/// `(valid, cache_hit)`.
fn verify_cached(alg: SignatureAlgorithm, pk: &[u8], msg: &[u8], sig: &[u8]) -> (bool, bool) {
    let key = cache_key(alg, pk, msg, sig);
    let ttl = cache_ttl_secs();

    if let Ok(mut cache) = VERIFY_CACHE.lock() {
        if let Some((valid, at)) = cache.get(&key) {
            if at.elapsed().as_secs() < ttl {
                return (*valid, true);
            }
        }
    }

    let valid = verify_signature(alg, pk, msg, sig).is_ok();

    if let Ok(mut cache) = VERIFY_CACHE.lock() {
        cache.put(key, (valid, Instant::now()));
    }
    (valid, false)
}

/// Clear the verification cache (used after revocation/rotation changes).
pub fn clear_cache() {
    if let Ok(mut cache) = VERIFY_CACHE.lock() {
        cache.clear();
    }
}

// ── Persisted types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SigningKey {
    pub id: Uuid,
    pub key_id: String,
    pub owner: String,
    pub algorithm: String,
    pub public_key: String,
    pub parent_key_id: Option<String>,
    pub cert_signature: Option<String>,
    pub is_root: bool,
    pub not_before: DateTime<Utc>,
    pub not_after: Option<DateTime<Utc>>,
    pub status: String,
    pub rotated_to: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ContractSignature {
    pub id: Uuid,
    pub contract_id: Option<Uuid>,
    pub contract_ref: String,
    pub subject_hash: String,
    pub algorithm: String,
    pub signature: String,
    pub key_id: String,
    pub signed_at: DateTime<Utc>,
    pub not_before: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub verified: bool,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Revocation {
    pub id: i64,
    pub key_id: Option<String>,
    pub signature_id: Option<Uuid>,
    pub reason: String,
    pub revoked_by: Option<String>,
    pub revoked_at: DateTime<Utc>,
}

// ── Request / response DTOs ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterKeyRequest {
    pub owner: String,
    pub algorithm: String,
    pub public_key: String,
    #[serde(default)]
    pub parent_key_id: Option<String>,
    #[serde(default)]
    pub cert_signature: Option<String>,
    #[serde(default)]
    pub is_root: bool,
    #[serde(default)]
    pub not_before: Option<DateTime<Utc>>,
    #[serde(default)]
    pub not_after: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RotateKeyRequest {
    pub algorithm: String,
    pub public_key: String,
    #[serde(default)]
    pub cert_signature: Option<String>,
    #[serde(default)]
    pub not_after: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RevokeRequest {
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub revoked_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StoreSignatureRequest {
    #[serde(default)]
    pub contract_id: Option<Uuid>,
    pub contract_ref: String,
    pub subject_hash: String,
    pub algorithm: String,
    pub signature: String,
    pub key_id: String,
    #[serde(default)]
    pub signed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub not_before: Option<DateTime<Utc>>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    /// The signed message (defaults to hex encoding — wasm hashes are hex).
    pub message: String,
    #[serde(default)]
    pub encoding: Option<String>,
    pub signature: String,
    /// Either reference a registered key…
    #[serde(default)]
    pub key_id: Option<String>,
    /// …or supply the key inline.
    #[serde(default)]
    pub algorithm: Option<String>,
    #[serde(default)]
    pub public_key: Option<String>,
    /// Claimed signing time, validated against the key's validity window.
    #[serde(default)]
    pub signed_at: Option<DateTime<Utc>>,
    /// Also validate the signer key's certificate chain to a trusted root.
    #[serde(default)]
    pub check_chain: bool,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub valid: bool,
    pub algorithm: String,
    pub key_id: Option<String>,
    pub revoked: bool,
    pub timestamp_valid: bool,
    pub chain_validated: Option<bool>,
    pub cache_hit: bool,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct ChainResponse {
    pub valid: bool,
    pub path: Vec<String>,
    pub reason: String,
}

// ── Handlers: key management ──────────────────────────────────────────────────

/// POST /api/signatures/keys — register a deployer or certificate-authority key.
pub async fn register_key(
    State(state): State<AppState>,
    Json(req): Json<RegisterKeyRequest>,
) -> Result<Json<SigningKey>, ApiError> {
    let alg = SignatureAlgorithm::parse(&req.algorithm).ok_or_else(|| {
        ApiError::bad_request(
            "UNSUPPORTED_ALGORITHM",
            "algorithm must be ed25519 or secp256k1",
        )
    })?;
    let pk_bytes = BASE64.decode(req.public_key.trim()).map_err(|_| {
        ApiError::bad_request("INVALID_PUBLIC_KEY", "public_key is not valid base64")
    })?;
    // Reject structurally invalid keys up front.
    validate_public_key(alg, &pk_bytes)?;

    let key_id = fingerprint(alg, &pk_bytes);
    let not_before = req.not_before.unwrap_or_else(Utc::now);
    let metadata = req.metadata.unwrap_or_else(|| serde_json::json!({}));

    let key = sqlx::query_as::<_, SigningKey>(
        r#"
        INSERT INTO signing_keys
            (key_id, owner, algorithm, public_key, parent_key_id, cert_signature,
             is_root, not_before, not_after, status, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active', $10)
        ON CONFLICT (key_id) DO UPDATE
        SET owner = EXCLUDED.owner,
            parent_key_id = EXCLUDED.parent_key_id,
            cert_signature = EXCLUDED.cert_signature,
            is_root = EXCLUDED.is_root,
            not_after = EXCLUDED.not_after,
            metadata = EXCLUDED.metadata
        RETURNING *
        "#,
    )
    .bind(&key_id)
    .bind(&req.owner)
    .bind(alg.as_str())
    .bind(&req.public_key)
    .bind(&req.parent_key_id)
    .bind(&req.cert_signature)
    .bind(req.is_root)
    .bind(not_before)
    .bind(req.not_after)
    .bind(&metadata)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("KEY_REGISTER_ERROR", e.to_string()))?;

    Ok(Json(key))
}

/// GET /api/signatures/keys/:key_id
pub async fn get_key(
    State(state): State<AppState>,
    Path(key_id): Path<String>,
) -> Result<Json<SigningKey>, ApiError> {
    let key = load_key(&state, &key_id)
        .await?
        .ok_or_else(|| ApiError::not_found("KEY_NOT_FOUND", "signing key not found"))?;
    Ok(Json(key))
}

/// POST /api/signatures/keys/:key_id/rotate — supersede a key with a new one.
/// The old key is marked `rotated` (not revoked), so signatures it already made
/// remain verifiable; only new signing should use the replacement.
pub async fn rotate_key(
    State(state): State<AppState>,
    Path(old_key_id): Path<String>,
    Json(req): Json<RotateKeyRequest>,
) -> Result<Json<SigningKey>, ApiError> {
    let old = load_key(&state, &old_key_id)
        .await?
        .ok_or_else(|| ApiError::not_found("KEY_NOT_FOUND", "signing key not found"))?;

    let alg = SignatureAlgorithm::parse(&req.algorithm).ok_or_else(|| {
        ApiError::bad_request(
            "UNSUPPORTED_ALGORITHM",
            "algorithm must be ed25519 or secp256k1",
        )
    })?;
    let pk_bytes = BASE64.decode(req.public_key.trim()).map_err(|_| {
        ApiError::bad_request("INVALID_PUBLIC_KEY", "public_key is not valid base64")
    })?;
    validate_public_key(alg, &pk_bytes)?;

    let new_key_id = fingerprint(alg, &pk_bytes);
    let metadata = req.metadata.unwrap_or_else(|| serde_json::json!({}));

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(|e| ApiError::internal_error("ROTATE_TX_ERROR", e.to_string()))?;

    let new_key = sqlx::query_as::<_, SigningKey>(
        r#"
        INSERT INTO signing_keys
            (key_id, owner, algorithm, public_key, parent_key_id, cert_signature,
             is_root, not_before, not_after, status, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), $8, 'active', $9)
        ON CONFLICT (key_id) DO UPDATE SET status = 'active'
        RETURNING *
        "#,
    )
    .bind(&new_key_id)
    .bind(&old.owner)
    .bind(alg.as_str())
    .bind(&req.public_key)
    .bind(&old.parent_key_id)
    .bind(&req.cert_signature)
    .bind(old.is_root)
    .bind(req.not_after)
    .bind(&metadata)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ApiError::internal_error("ROTATE_INSERT_ERROR", e.to_string()))?;

    sqlx::query("UPDATE signing_keys SET status = 'rotated', rotated_to = $1 WHERE key_id = $2")
        .bind(&new_key_id)
        .bind(&old_key_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| ApiError::internal_error("ROTATE_UPDATE_ERROR", e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| ApiError::internal_error("ROTATE_COMMIT_ERROR", e.to_string()))?;

    Ok(Json(new_key))
}

/// POST /api/signatures/keys/:key_id/revoke — add a key to the revocation list.
pub async fn revoke_key(
    State(state): State<AppState>,
    Path(key_id): Path<String>,
    body: Option<Json<RevokeRequest>>,
) -> Result<Json<Revocation>, ApiError> {
    let req = body.map(|Json(r)| r).unwrap_or(RevokeRequest {
        reason: None,
        revoked_by: None,
    });

    let exists = load_key(&state, &key_id).await?.is_some();
    if !exists {
        return Err(ApiError::not_found(
            "KEY_NOT_FOUND",
            "signing key not found",
        ));
    }

    let revocation = sqlx::query_as::<_, Revocation>(
        r#"
        INSERT INTO signature_revocations (key_id, reason, revoked_by)
        VALUES ($1, $2, $3)
        ON CONFLICT (key_id) WHERE key_id IS NOT NULL
        DO UPDATE SET reason = EXCLUDED.reason, revoked_by = EXCLUDED.revoked_by, revoked_at = NOW()
        RETURNING *
        "#,
    )
    .bind(&key_id)
    .bind(req.reason.unwrap_or_default())
    .bind(&req.revoked_by)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("REVOKE_ERROR", e.to_string()))?;

    sqlx::query("UPDATE signing_keys SET status = 'revoked' WHERE key_id = $1")
        .bind(&key_id)
        .execute(&state.db)
        .await
        .map_err(|e| ApiError::internal_error("REVOKE_UPDATE_ERROR", e.to_string()))?;

    // Revocation changes verification outcomes; drop cached results.
    clear_cache();

    Ok(Json(revocation))
}

/// GET /api/signatures/revocations — the revocation list.
pub async fn list_revocations(
    State(state): State<AppState>,
) -> Result<Json<Vec<Revocation>>, ApiError> {
    let rows = sqlx::query_as::<_, Revocation>(
        "SELECT * FROM signature_revocations ORDER BY revoked_at DESC LIMIT 500",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("REVOCATION_LIST_ERROR", e.to_string()))?;
    Ok(Json(rows))
}

// ── Handlers: signatures ──────────────────────────────────────────────────────

/// POST /api/signatures — store a contract signature with its metadata.
pub async fn store_signature(
    State(state): State<AppState>,
    Json(req): Json<StoreSignatureRequest>,
) -> Result<Json<ContractSignature>, ApiError> {
    SignatureAlgorithm::parse(&req.algorithm).ok_or_else(|| {
        ApiError::bad_request(
            "UNSUPPORTED_ALGORITHM",
            "algorithm must be ed25519 or secp256k1",
        )
    })?;

    let signed_at = req.signed_at.unwrap_or_else(Utc::now);
    let metadata = req.metadata.unwrap_or_else(|| serde_json::json!({}));

    let sig = sqlx::query_as::<_, ContractSignature>(
        r#"
        INSERT INTO contract_signatures
            (contract_id, contract_ref, subject_hash, algorithm, signature, key_id,
             signed_at, not_before, expires_at, metadata)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(req.contract_id)
    .bind(&req.contract_ref)
    .bind(&req.subject_hash)
    .bind(&req.algorithm)
    .bind(&req.signature)
    .bind(&req.key_id)
    .bind(signed_at)
    .bind(req.not_before)
    .bind(req.expires_at)
    .bind(&metadata)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("SIGNATURE_STORE_ERROR", e.to_string()))?;

    Ok(Json(sig))
}

/// GET /api/contracts/:id/signatures — signatures recorded for a contract.
pub async fn list_contract_signatures(
    State(state): State<AppState>,
    Path(contract_id): Path<Uuid>,
) -> Result<Json<Vec<ContractSignature>>, ApiError> {
    let rows = sqlx::query_as::<_, ContractSignature>(
        "SELECT * FROM contract_signatures WHERE contract_id = $1 ORDER BY signed_at DESC",
    )
    .bind(contract_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal_error("SIGNATURE_LIST_ERROR", e.to_string()))?;
    Ok(Json(rows))
}

/// POST /api/signatures/verify — the full verification pipeline:
/// crypto check (cached) + timestamp window + revocation + optional cert chain.
pub async fn verify(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, ApiError> {
    let message = decode_message(&req.message, req.encoding.as_deref())?;
    let sig_bytes = BASE64
        .decode(req.signature.trim())
        .map_err(|_| ApiError::bad_request("INVALID_SIGNATURE", "signature is not valid base64"))?;

    // Resolve the key + algorithm, either from a registered key or inline.
    let (alg, pk_bytes, resolved_key, key_id) = if let Some(kid) = &req.key_id {
        let key = load_key(&state, kid)
            .await?
            .ok_or_else(|| ApiError::not_found("KEY_NOT_FOUND", "signing key not found"))?;
        let alg = SignatureAlgorithm::parse(&key.algorithm).ok_or_else(|| {
            ApiError::internal_error("BAD_STORED_ALGORITHM", "stored key has invalid algorithm")
        })?;
        let pk = BASE64.decode(key.public_key.trim()).map_err(|_| {
            ApiError::internal_error("BAD_STORED_KEY", "stored public key is not base64")
        })?;
        (alg, pk, Some(key), Some(kid.clone()))
    } else {
        let alg = req
            .algorithm
            .as_deref()
            .and_then(SignatureAlgorithm::parse)
            .ok_or_else(|| {
                ApiError::bad_request(
                    "MISSING_ALGORITHM",
                    "provide key_id, or algorithm + public_key",
                )
            })?;
        let pk_b64 = req.public_key.as_deref().ok_or_else(|| {
            ApiError::bad_request(
                "MISSING_PUBLIC_KEY",
                "provide key_id, or algorithm + public_key",
            )
        })?;
        let pk = BASE64.decode(pk_b64.trim()).map_err(|_| {
            ApiError::bad_request("INVALID_PUBLIC_KEY", "public_key is not valid base64")
        })?;
        let derived = fingerprint(alg, &pk);
        (alg, pk, None, Some(derived))
    };

    // Revocation check.
    let revoked = match &key_id {
        Some(kid) => is_revoked(&state, kid).await?,
        None => false,
    };

    // Timestamp validity against the key's window (if the key is registered).
    let now = Utc::now();
    let check_time = req.signed_at.unwrap_or(now);
    let timestamp_valid = match &resolved_key {
        Some(key) => {
            check_time >= key.not_before && key.not_after.map(|na| check_time <= na).unwrap_or(true)
        }
        None => true,
    };

    // Cryptographic check (cached).
    let (crypto_valid, cache_hit) = verify_cached(alg, &pk_bytes, &message, &sig_bytes);

    // Optional certificate-chain validation.
    let chain_validated = if req.check_chain {
        match &key_id {
            Some(kid) => Some(validate_chain_inner(&state, kid).await?.0),
            None => Some(false),
        }
    } else {
        None
    };

    let valid = crypto_valid && !revoked && timestamp_valid && chain_validated.unwrap_or(true);

    let reason = if !crypto_valid {
        "cryptographic verification failed".to_string()
    } else if revoked {
        "signing key is revoked".to_string()
    } else if !timestamp_valid {
        "signature timestamp outside key validity window".to_string()
    } else if chain_validated == Some(false) {
        "certificate chain validation failed".to_string()
    } else {
        "signature is valid".to_string()
    };

    Ok(Json(VerifyResponse {
        valid,
        algorithm: alg.as_str().to_string(),
        key_id,
        revoked,
        timestamp_valid,
        chain_validated,
        cache_hit,
        reason,
    }))
}

/// POST /api/signatures/keys/:key_id/verify-chain — validate the cert chain.
pub async fn verify_chain(
    State(state): State<AppState>,
    Path(key_id): Path<String>,
) -> Result<Json<ChainResponse>, ApiError> {
    let (valid, path, reason) = validate_chain_inner(&state, &key_id).await?;
    Ok(Json(ChainResponse {
        valid,
        path,
        reason,
    }))
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn validate_public_key(alg: SignatureAlgorithm, pk: &[u8]) -> Result<(), ApiError> {
    let ok = match alg {
        SignatureAlgorithm::Ed25519 => pk.len() == 32,
        // SEC1 compressed (33) or uncompressed (65).
        SignatureAlgorithm::Secp256k1 => pk.len() == 33 || pk.len() == 65,
    };
    if ok {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "INVALID_PUBLIC_KEY",
            "public key length is invalid for the algorithm",
        ))
    }
}

async fn load_key(state: &AppState, key_id: &str) -> Result<Option<SigningKey>, ApiError> {
    sqlx::query_as::<_, SigningKey>("SELECT * FROM signing_keys WHERE key_id = $1")
        .bind(key_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ApiError::internal_error("KEY_LOAD_ERROR", e.to_string()))
}

async fn is_revoked(state: &AppState, key_id: &str) -> Result<bool, ApiError> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM signature_revocations WHERE key_id = $1")
            .bind(key_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| ApiError::internal_error("REVOCATION_CHECK_ERROR", e.to_string()))?;
    Ok(count > 0)
}

/// Walk the certificate chain from `start_key_id` up to a trusted root, checking
/// at each hop: not revoked, within its validity window, and (for non-roots)
/// that the parent's key verifies this key's `cert_signature` over its public
/// key bytes. Returns `(valid, path, reason)`.
async fn validate_chain_inner(
    state: &AppState,
    start_key_id: &str,
) -> Result<(bool, Vec<String>, String), ApiError> {
    let now = Utc::now();
    let mut path = Vec::new();
    let mut current = match load_key(state, start_key_id).await? {
        Some(k) => k,
        None => return Ok((false, path, "start key not found".to_string())),
    };

    for _ in 0..MAX_CHAIN_DEPTH {
        path.push(current.key_id.clone());

        if is_revoked(state, &current.key_id).await? || current.status == "revoked" {
            return Ok((false, path, format!("key {} is revoked", current.key_id)));
        }
        if now < current.not_before || current.not_after.map(|na| now > na).unwrap_or(false) {
            return Ok((
                false,
                path,
                format!("key {} is outside its validity window", current.key_id),
            ));
        }
        if current.is_root {
            return Ok((true, path, "chain anchored to a trusted root".to_string()));
        }

        let Some(parent_id) = current.parent_key_id.clone() else {
            return Ok((
                false,
                path,
                "chain ends without reaching a trusted root".to_string(),
            ));
        };
        let Some(cert_sig_b64) = current.cert_signature.clone() else {
            return Ok((
                false,
                path,
                format!("key {} has no certificate signature", current.key_id),
            ));
        };
        let parent = match load_key(state, &parent_id).await? {
            Some(p) => p,
            None => return Ok((false, path, format!("parent key {parent_id} not found"))),
        };

        // Verify the parent signed this key's raw public-key bytes.
        let parent_alg = SignatureAlgorithm::parse(&parent.algorithm).ok_or_else(|| {
            ApiError::internal_error("BAD_STORED_ALGORITHM", "parent key has invalid algorithm")
        })?;
        let parent_pk = BASE64.decode(parent.public_key.trim()).map_err(|_| {
            ApiError::internal_error("BAD_STORED_KEY", "parent public key not base64")
        })?;
        let child_pk = BASE64.decode(current.public_key.trim()).map_err(|_| {
            ApiError::internal_error("BAD_STORED_KEY", "child public key not base64")
        })?;
        let cert_sig = BASE64
            .decode(cert_sig_b64.trim())
            .map_err(|_| ApiError::internal_error("BAD_CERT_SIG", "cert signature not base64"))?;

        if verify_signature(parent_alg, &parent_pk, &child_pk, &cert_sig).is_err() {
            return Ok((
                false,
                path,
                format!("invalid certificate signature for key {}", current.key_id),
            ));
        }

        current = parent;
    }

    Ok((
        false,
        path,
        "certificate chain exceeds maximum depth".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn algorithm_parsing() {
        assert_eq!(
            SignatureAlgorithm::parse("Ed25519"),
            Some(SignatureAlgorithm::Ed25519)
        );
        assert_eq!(
            SignatureAlgorithm::parse("secp256k1"),
            Some(SignatureAlgorithm::Secp256k1)
        );
        assert_eq!(
            SignatureAlgorithm::parse("ecdsa"),
            Some(SignatureAlgorithm::Secp256k1)
        );
        assert_eq!(SignatureAlgorithm::parse("rsa"), None);
    }

    #[test]
    fn fingerprint_is_stable_and_algorithm_scoped() {
        let pk = [7u8; 32];
        let a = fingerprint(SignatureAlgorithm::Ed25519, &pk);
        assert_eq!(a, fingerprint(SignatureAlgorithm::Ed25519, &pk));
        assert_ne!(a, fingerprint(SignatureAlgorithm::Secp256k1, &pk));
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn ed25519_roundtrip_and_tamper() {
        use ed25519_dalek::{Signer, SigningKey};
        use rand::rngs::OsRng;

        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();
        let msg = b"deploy:contract-abc:wasm-hash";
        let sig = sk.sign(msg);

        assert!(verify_signature(
            SignatureAlgorithm::Ed25519,
            vk.as_bytes(),
            msg,
            &sig.to_bytes()
        )
        .is_ok());
        // Tampered message fails.
        assert_eq!(
            verify_signature(
                SignatureAlgorithm::Ed25519,
                vk.as_bytes(),
                b"other",
                &sig.to_bytes()
            ),
            Err(SigError::VerificationFailed)
        );
    }

    #[test]
    fn secp256k1_roundtrip_and_tamper() {
        use k256::ecdsa::{signature::Signer, Signature, SigningKey};
        use rand::rngs::OsRng;

        let sk = SigningKey::random(&mut OsRng);
        let vk = sk.verifying_key();
        let pk = vk.to_sec1_bytes(); // compressed SEC1
        let msg = b"deploy:contract-xyz:wasm-hash";
        let sig: Signature = sk.sign(msg);

        assert!(verify_signature(SignatureAlgorithm::Secp256k1, &pk, msg, &sig.to_bytes()).is_ok());
        assert!(verify_signature(
            SignatureAlgorithm::Secp256k1,
            &pk,
            b"tampered",
            &sig.to_bytes()
        )
        .is_err());
    }

    #[test]
    fn wrong_key_length_is_rejected() {
        assert_eq!(
            verify_signature(SignatureAlgorithm::Ed25519, &[0u8; 10], b"m", &[0u8; 64]),
            Err(SigError::InvalidKey)
        );
    }
}
