//! Signature-anchored ownership transfer support (issue #1094).
//!
//! Owns the pieces of the two-phase transfer flow that are not HTTP handling:
//!
//!   * the canonical signing payloads both parties sign,
//!   * the freshness window for a signature,
//!   * signature verification against a publisher's on-chain Stellar account,
//!   * expiry, both lazily (on read/write paths) and via a background sweeper.
//!
//! The handlers live in `handlers.rs` alongside the rest of the #1058 endpoints; this
//! module exists so the crypto and payload construction can be exercised directly by
//! tests and reused by the CLI later.
//!
//! Design notes worth keeping:
//!
//! * **"On-chain-anchored" here means the signing key is the on-chain account key.** A
//!   transfer is authorised by an ed25519 signature that verifies against
//!   `publishers.stellar_address`, which is a Stellar `G...` account. No RPC or Horizon
//!   call is made, and no transaction hash is recorded. Anchoring is to the keypair that
//!   controls the account, not to a specific ledger entry.
//! * **Payloads are colon-joined ASCII, not JSON.** This matches the existing signing
//!   conventions in `signing_handlers.rs` and `handlers/validators.rs`, and sidesteps
//!   canonical-JSON key-ordering entirely. Nonces are restricted to an alphabet without
//!   `:` (see `validation::handler_requests`) so the encoding stays unambiguous.
//! * **Timestamps in payloads are unix seconds.** An RFC3339 round-trip through serde can
//!   change offset or sub-second precision, which would change the signed bytes.
//! * **Every segment that identifies the transfer is inside the payload**: domain,
//!   version, and action, so a phase-1 signature cannot be replayed as a phase-2 one and
//!   an `accept` cannot be replayed as a `reject`.

use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;
use crate::signature_verification::{verify_signature, SigError, SignatureAlgorithm};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Domain separator. Any change here invalidates every previously issued signature, so
/// bump the version segment instead of editing this.
const PAYLOAD_DOMAIN: &str = "soroban-registry:ownership-transfer";
const PAYLOAD_VERSION: &str = "v1";

/// An ed25519 signature is always exactly 64 bytes.
const ED25519_SIGNATURE_LEN: usize = 64;

/// Bounds on the client-supplied nonce. Long enough to be unguessable, short enough to
/// index; the column is `VARCHAR(128)`.
pub const MIN_NONCE_LEN: usize = 16;
pub const MAX_NONCE_LEN: usize = 128;

/// Upper bound on how far in the future a transfer may be set to expire. A transfer is a
/// standing offer to take over a contract, so it should not be able to sit open forever.
pub const MAX_EXPIRY_WINDOW_SECS: i64 = 90 * 24 * 60 * 60;

/// Default lower bound, so a caller cannot create a transfer that is already unusable by
/// the time the recipient sees it.
pub const DEFAULT_MIN_EXPIRY_WINDOW_SECS: i64 = 60;

/// Shortest permitted transfer lifetime, in seconds.
///
/// Configurable via `OWNERSHIP_TRANSFER_MIN_EXPIRY_SECS`. Lowering it is what lets the
/// expiry tests observe a real expiry without sleeping for a minute; the integration suite
/// reads this same function so it stays correct at any setting.
pub fn min_expiry_window_secs() -> i64 {
    std::env::var("OWNERSHIP_TRANSFER_MIN_EXPIRY_SECS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(DEFAULT_MIN_EXPIRY_WINDOW_SECS)
}

/// Rows claimed per sweeper pass.
const EXPIRY_BATCH_SIZE: i64 = 100;

// ── Signing payloads ──────────────────────────────────────────────────────────

/// The phase-2 decision a party is signing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDecision {
    Accept,
    Reject,
}

impl TransferDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Reject => "reject",
        }
    }

    pub fn from_accept_flag(accept: bool) -> Self {
        if accept {
            Self::Accept
        } else {
            Self::Reject
        }
    }
}

/// Canonical payload the outgoing publisher signs to open a transfer.
///
/// The transfer id cannot appear here: the sender has to sign before the row exists. The
/// nonce takes its place as the binding token, and a unique index on `request_nonce`
/// makes the signature single-use.
pub fn initiate_payload(
    contract_id: Uuid,
    from_address: &str,
    to_address: &str,
    expires_at_unix: i64,
    nonce: &str,
    signed_at_unix: i64,
) -> String {
    format!(
        "{}:{}:initiate:{}:{}:{}:{}:{}:{}",
        PAYLOAD_DOMAIN,
        PAYLOAD_VERSION,
        contract_id,
        from_address,
        to_address,
        expires_at_unix,
        nonce,
        signed_at_unix
    )
}

/// Canonical payload a party signs to accept or reject an existing transfer.
///
/// Binds both the transfer id and the originating `request_nonce`, so an acceptance is
/// welded to exactly one initiation and cannot be carried across a re-initiated transfer.
#[allow(clippy::too_many_arguments)]
pub fn decision_payload(
    decision: TransferDecision,
    transfer_id: Uuid,
    contract_id: Uuid,
    from_address: &str,
    to_address: &str,
    request_nonce: &str,
    nonce: &str,
    signed_at_unix: i64,
) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        PAYLOAD_DOMAIN,
        PAYLOAD_VERSION,
        decision.as_str(),
        transfer_id,
        contract_id,
        from_address,
        to_address,
        request_nonce,
        nonce,
        signed_at_unix
    )
}

// ── Freshness ─────────────────────────────────────────────────────────────────

/// How far `signed_at_unix` may sit from the server clock, in seconds.
///
/// Symmetric, so a client whose clock runs modestly fast is still able to transact rather
/// than being locked out with an error it cannot diagnose.
pub fn freshness_window_secs() -> i64 {
    std::env::var("OWNERSHIP_TRANSFER_SIGNATURE_MAX_AGE_SECS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(300)
}

/// Reject signatures that are stale or dated too far in the future.
///
/// A signature is only single-use because of the nonce indexes; this bound is what keeps
/// a leaked-but-unused signature from being useful indefinitely.
pub fn check_signature_freshness(signed_at_unix: i64, now_unix: i64) -> Result<(), ApiError> {
    let window = freshness_window_secs();
    let skew = now_unix.saturating_sub(signed_at_unix);

    if skew > window {
        return Err(ApiError::unprocessable(
            "SignatureExpired",
            format!(
                "signature timestamp is {}s old; the maximum accepted age is {}s",
                skew, window
            ),
        ));
    }

    if skew < -window {
        return Err(ApiError::unprocessable(
            "SignatureNotYetValid",
            format!(
                "signature timestamp is {}s in the future; the maximum accepted skew is {}s",
                -skew, window
            ),
        ));
    }

    Ok(())
}

// ── Verification ──────────────────────────────────────────────────────────────

/// Verify `signature_b64` over `payload` against the Stellar account `signer_address`.
///
/// `signer_address` is always read from `publishers`, never taken from the request body:
/// the caller proves control of the key belonging to the identity the server resolved.
///
/// Error mapping follows the precedent in `handlers.rs` for version signatures: a payload
/// the server cannot even decode is a 400 (the client sent something malformed), while a
/// well-formed signature that does not verify is a 422 (the request was understood and
/// rejected on its merits).
pub fn verify_transfer_signature(
    signer_address: &str,
    payload: &str,
    signature_b64: &str,
) -> Result<(), ApiError> {
    let public_key = stellar_strkey::ed25519::PublicKey::from_string(signer_address)
        .map_err(|_| {
            // The address came from our own table, so this is bad stored data, not a
            // client error.
            tracing::error!(
                signer_address = signer_address,
                "publisher stellar_address on record is not a valid ed25519 strkey"
            );
            ApiError::internal("The publisher address on record is not a valid Stellar address")
        })?
        .0;

    let signature = BASE64.decode(signature_b64.trim()).map_err(|_| {
        ApiError::bad_request(
            "InvalidSignatureEncoding",
            "signature must be standard base64-encoded",
        )
    })?;

    if signature.len() != ED25519_SIGNATURE_LEN {
        return Err(ApiError::bad_request(
            "InvalidSignatureLength",
            format!(
                "an ed25519 signature must decode to {} bytes, got {}",
                ED25519_SIGNATURE_LEN,
                signature.len()
            ),
        ));
    }

    // `verify_signature` uses `verify_strict` for ed25519, which rejects small-order and
    // otherwise malleable keys.
    verify_signature(
        SignatureAlgorithm::Ed25519,
        &public_key,
        payload.as_bytes(),
        &signature,
    )
    .map_err(|err| match err {
        SigError::InvalidKey => {
            ApiError::internal("The publisher address on record is not a usable ed25519 key")
        }
        SigError::InvalidSignature => ApiError::bad_request(
            "InvalidSignature",
            "signature is not a well-formed ed25519 signature",
        ),
        SigError::VerificationFailed => ApiError::unprocessable(
            "SignatureVerificationFailed",
            "signature does not verify against the signer's Stellar account",
        ),
    })
}

/// Parse and validate the optional `signature_algorithm` field.
///
/// Only ed25519 is accepted: Stellar accounts are ed25519, so a secp256k1 signature could
/// never be checked against `publishers.stellar_address`. Accepting the field at all keeps
/// the wire format forward-compatible without implying support that does not exist.
pub fn resolve_algorithm(requested: Option<&str>) -> Result<&'static str, ApiError> {
    match requested.map(str::trim) {
        None | Some("") => Ok("ed25519"),
        Some(value) if value.eq_ignore_ascii_case("ed25519") => Ok("ed25519"),
        Some(value) => Err(ApiError::bad_request(
            "UnsupportedSignatureAlgorithm",
            format!(
                "signature_algorithm '{}' is not supported; ownership transfers require ed25519",
                value
            ),
        )),
    }
}

// ── Expiry ────────────────────────────────────────────────────────────────────

/// Expire a single transfer if it is live and past `expires_at`.
///
/// Returns `true` when this call was the one that expired it. The state change and its
/// history row go in one transaction, so history can never disagree with the row.
pub async fn expire_transfer_if_due(db: &PgPool, transfer_id: Uuid) -> Result<bool, sqlx::Error> {
    let mut tx = db.begin().await?;

    let expired: Option<(Uuid,)> = sqlx::query_as(
        "UPDATE ownership_transfers
         SET status = 'expired', completed_at = NOW()
         WHERE id = $1
           AND status IN ('pending', 'confirmed')
           AND expires_at <= NOW()
         RETURNING id",
    )
    .bind(transfer_id)
    .fetch_optional(&mut *tx)
    .await?;

    if expired.is_none() {
        tx.rollback().await?;
        return Ok(false);
    }

    insert_system_expiry_log(&mut tx, &[transfer_id]).await?;
    tx.commit().await?;

    tracing::info!(
        target: "ownership_transfer",
        transfer_id = %transfer_id,
        "ownership transfer expired"
    );

    Ok(true)
}

/// Expire every live, past-due transfer for one contract. Used by the list endpoint so a
/// read never reports a transfer as actionable when it is not.
pub async fn expire_due_transfers_for_contract(
    db: &PgPool,
    contract_id: Uuid,
) -> Result<u64, sqlx::Error> {
    let mut tx = db.begin().await?;

    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "UPDATE ownership_transfers
         SET status = 'expired', completed_at = NOW()
         WHERE contract_id = $1
           AND status IN ('pending', 'confirmed')
           AND expires_at <= NOW()
         RETURNING id",
    )
    .bind(contract_id)
    .fetch_all(&mut *tx)
    .await?;

    if rows.is_empty() {
        tx.rollback().await?;
        return Ok(0);
    }

    let ids: Vec<Uuid> = rows.into_iter().map(|(id,)| id).collect();
    insert_system_expiry_log(&mut tx, &ids).await?;
    tx.commit().await?;

    tracing::info!(
        target: "ownership_transfer",
        contract_id = %contract_id,
        expired = ids.len(),
        "expired past-due ownership transfers for contract"
    );

    Ok(ids.len() as u64)
}

/// Sweep a batch of past-due transfers across all contracts.
///
/// `FOR UPDATE SKIP LOCKED` means the sweeper never blocks, and is never blocked by, a
/// confirmation that is mid-transaction on the same row.
pub async fn expire_due_transfers(db: &PgPool) -> Result<u64, sqlx::Error> {
    let mut total = 0u64;

    loop {
        let mut tx = db.begin().await?;

        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "WITH claimed AS (
                 SELECT id
                 FROM ownership_transfers
                 WHERE status IN ('pending', 'confirmed')
                   AND expires_at <= NOW()
                 ORDER BY expires_at ASC
                 LIMIT $1
                 FOR UPDATE SKIP LOCKED
             )
             UPDATE ownership_transfers t
             SET status = 'expired', completed_at = NOW()
             FROM claimed c
             WHERE t.id = c.id
             RETURNING t.id",
        )
        .bind(EXPIRY_BATCH_SIZE)
        .fetch_all(&mut *tx)
        .await?;

        if rows.is_empty() {
            tx.rollback().await?;
            break;
        }

        let ids: Vec<Uuid> = rows.into_iter().map(|(id,)| id).collect();
        let claimed = ids.len();
        insert_system_expiry_log(&mut tx, &ids).await?;
        tx.commit().await?;

        total += claimed as u64;

        // A short batch means the queue is drained; anything else risks spinning.
        if (claimed as i64) < EXPIRY_BATCH_SIZE {
            break;
        }
    }

    if total > 0 {
        tracing::info!(
            target: "ownership_transfer",
            expired = total,
            "expired past-due ownership transfers"
        );
    }

    Ok(total)
}

/// Append one `transfer_expired` history row per id, authored by the system.
async fn insert_system_expiry_log(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    transfer_ids: &[Uuid],
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ownership_transfer_logs (transfer_id, actor_id, actor_type, action, details)
         SELECT id, NULL, 'system', 'transfer_expired', $2::jsonb
         FROM UNNEST($1::uuid[]) AS t(id)",
    )
    .bind(transfer_ids)
    .bind(serde_json::json!({
        "reason": "Transfer request passed its expiry without a verified acceptance",
    }))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

// ── Background task ───────────────────────────────────────────────────────────

/// Spawn the periodic expiry sweeper.
///
/// Expiry is also applied lazily on every read and write path, so this task is a
/// backstop: it keeps `expires_at` meaningful for transfers nobody ever looks at again,
/// which is what makes the history table an accurate record rather than a set of rows
/// frozen in whatever state they were last observed in.
///
/// Interval is configurable via `OWNERSHIP_TRANSFER_EXPIRY_INTERVAL_SECS` (default 300).
pub fn spawn_ownership_transfer_expiry_task(pool: PgPool) {
    let interval_secs = std::env::var("OWNERSHIP_TRANSFER_EXPIRY_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(300);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        // Skip the immediate first tick so startup is not competing with a sweep.
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(err) = expire_due_transfers(&pool).await {
                tracing::error!(
                    target: "ownership_transfer",
                    error = ?err,
                    "periodic ownership transfer expiry sweep failed"
                );
            }
        }
    });
}
