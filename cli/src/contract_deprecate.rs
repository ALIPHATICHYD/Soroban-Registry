//! contract_deprecate.rs — `soroban-registry contract deprecate` (#1091)
//!
//! Deprecate a contract with Ed25519 signature-authenticated authorization.
//! The publisher's keypair signs a deprecation payload (`{contract_id, action,
//! timestamp, nonce}`) so the backend can verify the request was authorized by
//! the on-chain publisher identity — not just an API token.
//!
//! Usage:
//!   soroban-registry contract deprecate <ADDRESS> --reason <REASON> --private-key <KEY>
//!       [--replacement <ID>] [--migration-guide <URL>] [--grace-period-days N] [-y] [--json]

use crate::net::RequestBuilderExt;
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use colored::Colorize;
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Maximum age of a signed deprecation timestamp before the backend should
/// reject it as stale (5 minutes).
const MAX_SIGNATURE_AGE_SECS: i64 = 300;

// ── Payload types ────────────────────────────────────────────────────────────

/// The canonical message that gets signed by the publisher's private key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecationPayload {
    pub contract_id: String,
    pub action: String,
    pub timestamp: String,
    pub nonce: String,
}

/// The full request body sent to the deprecation API endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedDeprecationRequest {
    /// Deprecation reason visible to consumers.
    pub reason: String,
    /// Replacement contract ID for migration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement_contract_id: Option<String>,
    /// Migration guide URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_guide_url: Option<String>,
    /// Grace period in days before hard removal.
    pub grace_period_days: i32,
    /// The canonical payload that was signed.
    pub payload: DeprecationPayload,
    /// Base64-encoded Ed25519 signature of the payload.
    pub signature: String,
    /// Base64-encoded Ed25519 public key of the signer.
    pub public_key: String,
    /// Stellar address derived from the public key.
    pub signing_address: String,
}

// ── Entry point ──────────────────────────────────────────────────────────────

/// Main entry point for `soroban-registry contract deprecate`.
pub async fn run(
    api_url: &str,
    address: &str,
    reason: &str,
    replacement: Option<&str>,
    private_key: &str,
    migration_guide: Option<&str>,
    grace_period_days: i32,
    yes: bool,
    json_output: bool,
) -> Result<()> {
    // 1. Decode the private key and derive public key / address
    let signing_key = decode_private_key(private_key)?;
    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = verifying_key.to_bytes();
    let public_key_b64 = BASE64.encode(public_key_bytes);
    let signing_address = derive_stellar_address(&public_key_bytes);

    // 2. Fetch current contract state so we can show a diff
    let contract_info = fetch_contract_info(api_url, address).await?;

    // 3. Show state diff and confirm
    if !yes {
        render_state_diff(address, reason, replacement, &contract_info, &signing_address);
        if !prompt_confirmation()? {
            println!("{}", "Deprecation cancelled.".yellow());
            return Ok(());
        }
    }

    // 4. Build and sign the deprecation payload
    let nonce = uuid::Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();

    let canonical_contract_id = contract_info
        .get("contract_id")
        .and_then(Value::as_str)
        .unwrap_or(address)
        .to_string();

    let payload = DeprecationPayload {
        contract_id: canonical_contract_id,
        action: "deprecate".to_string(),
        timestamp,
        nonce,
    };

    let message = build_signing_message(&payload);
    let signature = signing_key.sign(message.as_bytes());
    let signature_b64 = BASE64.encode(signature.to_bytes());

    let request = SignedDeprecationRequest {
        reason: reason.to_string(),
        replacement_contract_id: replacement.map(String::from),
        migration_guide_url: migration_guide.map(String::from),
        grace_period_days,
        payload,
        signature: signature_b64,
        public_key: public_key_b64,
        signing_address: signing_address.clone(),
    };

    // 5. Submit to registry
    let client = crate::net::client();
    let url = format!(
        "{}/api/contracts/{}/deprecate",
        api_url.trim_end_matches('/'),
        address
    );
    log::debug!("POST {}", url);

    let resp = client
        .post(&url)
        .json(&request)
        .send_with_retry()
        .await
        .context("Failed to reach the registry API. Is the registry running?")?;

    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(Value::Null);

    if !status.is_success() {
        let error_type = body
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let error_msg = body
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Unknown error");

        match error_type {
            "unauthorized" | "signature_invalid" => {
                bail!(
                    "Signature authorization failed: {}. \
                     Ensure you are using the publisher's private key for this contract.",
                    error_msg
                );
            }
            "signature_expired" => {
                bail!(
                    "Signature expired: {}. \
                     The signed timestamp was too old. Try again.",
                    error_msg
                );
            }
            "key_mismatch" => {
                bail!(
                    "Key mismatch: {}. \
                     The signing key does not match the registered publisher for this contract.",
                    error_msg
                );
            }
            _ => {
                bail!(
                    "Deprecation failed (HTTP {}): {} — {}",
                    status.as_u16(),
                    error_type,
                    error_msg
                );
            }
        }
    }

    // 6. Render success
    if json_output {
        let result = json!({
            "status": "deprecated",
            "contract_id": address,
            "reason": reason,
            "replacement_contract_id": replacement,
            "signing_address": signing_address,
            "grace_period_days": grace_period_days,
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("\n{}", "✓ Contract deprecated successfully!".green().bold());
        println!("  {}: {}", "Contract".bold(), address.cyan());
        println!("  {}: {}", "Reason".bold(), reason);
        if let Some(repl) = replacement {
            println!("  {}: {}", "Replacement".bold(), repl.cyan());
        }
        if let Some(guide) = migration_guide {
            println!("  {}: {}", "Migration Guide".bold(), guide);
        }
        println!(
            "  {}: {} days",
            "Grace Period".bold(),
            grace_period_days
        );
        println!(
            "  {}: {}",
            "Signed By".bold(),
            signing_address.bright_magenta()
        );
        println!();
    }

    Ok(())
}

// ── Signing ──────────────────────────────────────────────────────────────────

/// Build the canonical signing message from a deprecation payload.
///
/// Format: `"deprecate:{contract_id}:{timestamp}:{nonce}"`
///
/// This deterministic format ensures both CLI and backend compute the same
/// message for signature verification.
pub fn build_signing_message(payload: &DeprecationPayload) -> String {
    format!(
        "{}:{}:{}:{}",
        payload.action, payload.contract_id, payload.timestamp, payload.nonce
    )
}

/// Sign a deprecation payload and return the base64-encoded signature.
pub fn sign_deprecation(payload: &DeprecationPayload, signing_key: &SigningKey) -> String {
    let message = build_signing_message(payload);
    let signature = signing_key.sign(message.as_bytes());
    BASE64.encode(signature.to_bytes())
}

/// Verify a signature against a deprecation payload and public key.
pub fn verify_deprecation_signature(
    payload: &DeprecationPayload,
    signature_b64: &str,
    verifying_key: &VerifyingKey,
) -> Result<bool> {
    let message = build_signing_message(payload);
    let sig_bytes = BASE64
        .decode(signature_b64)
        .context("Invalid signature encoding (expected base64)")?;
    let sig_array: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Signature must be 64 bytes"))?;
    let signature = ed25519_dalek::Signature::from_bytes(&sig_array);
    Ok(verifying_key.verify(message.as_bytes(), &signature).is_ok())
}

/// Check whether a signed timestamp is within the acceptable age window.
pub fn is_timestamp_valid(timestamp: &str) -> Result<bool> {
    let signed_at = chrono::DateTime::parse_from_rfc3339(timestamp)
        .context("Invalid timestamp format in payload")?;
    let age = Utc::now().signed_duration_since(signed_at.with_timezone(&Utc));
    Ok(age.num_seconds().abs() <= MAX_SIGNATURE_AGE_SECS)
}

// ── Key handling (reused patterns from package_signing.rs) ───────────────────

/// Decode a base64-encoded Ed25519 private key into a `SigningKey`.
fn decode_private_key(key: &str) -> Result<SigningKey> {
    let bytes = BASE64
        .decode(key.trim())
        .context("Invalid private key format (expected base64-encoded Ed25519 key)")?;
    let bytes: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Private key must be 32 bytes"))?;
    Ok(SigningKey::from_bytes(&bytes))
}

/// Derive a Stellar-style address from a 32-byte Ed25519 public key.
fn derive_stellar_address(public_key_bytes: &[u8; 32]) -> String {
    use ripemd::Ripemd160;
    use sha2::{Digest, Sha256};

    let sha256_hash = Sha256::digest(public_key_bytes);
    let ripemd_hash = Ripemd160::digest(sha256_hash);
    let mut versioned = vec![0x00];
    versioned.extend_from_slice(&ripemd_hash);
    let checksum = Sha256::digest(&Sha256::digest(&versioned));
    versioned.extend_from_slice(&checksum[..4]);
    bs58::encode(&versioned).into_string()
}

// ── Contract info fetch ──────────────────────────────────────────────────────

/// Fetch current contract metadata from the registry.
async fn fetch_contract_info(api_url: &str, address: &str) -> Result<Value> {
    let client = crate::net::client();
    let url = format!(
        "{}/api/contracts/{}",
        api_url.trim_end_matches('/'),
        address
    );
    log::debug!("GET {}", url);

    let resp = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch contract info from registry")?;

    let status = resp.status();
    if status.as_u16() == 404 {
        bail!("Contract {} not found in the registry", address);
    }
    if !status.is_success() {
        bail!("Failed to fetch contract info (HTTP {})", status.as_u16());
    }

    resp.json()
        .await
        .context("Failed to parse contract info response")
}

// ── UI helpers ───────────────────────────────────────────────────────────────

/// Print a state diff showing what will change when the contract is deprecated.
fn render_state_diff(
    address: &str,
    reason: &str,
    replacement: Option<&str>,
    contract_info: &Value,
    signing_address: &str,
) {
    let name = contract_info
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("(unknown)");
    let current_status = contract_info
        .get("status")
        .or_else(|| contract_info.get("deprecation_status"))
        .and_then(Value::as_str)
        .unwrap_or("active");
    let network = contract_info
        .get("network")
        .and_then(Value::as_str)
        .unwrap_or("(unknown)");

    println!("\n{}", "Contract Deprecation Preview".bold().cyan());
    println!("{}", "═".repeat(50).cyan());
    println!("  {}: {} ({})", "Contract".bold(), address.cyan(), name.dimmed());
    println!("  {}: {}", "Network".bold(), network);
    println!("  {}: {}", "Signed By".bold(), signing_address.bright_magenta());

    println!("\n  {}", "State Change:".bold().yellow());
    println!(
        "    {} {} → {}",
        "Status:".bold(),
        current_status.green(),
        "deprecated".red()
    );
    println!(
        "    {} (none) → {}",
        "Reason:".bold(),
        reason
    );
    if let Some(repl) = replacement {
        println!(
            "    {} (none) → {}",
            "Replacement:".bold(),
            repl.cyan()
        );
    }
    println!("{}\n", "═".repeat(50).cyan());
}

/// Prompt the user for yes/no confirmation.
fn prompt_confirmation() -> Result<bool> {
    use std::io::{self, Write};
    print!(
        "  {} ",
        "Proceed with deprecation? [y/N]".bold()
    );
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim().to_lowercase();
    Ok(trimmed == "y" || trimmed == "yes")
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn make_payload(contract_id: &str, nonce: &str) -> DeprecationPayload {
        DeprecationPayload {
            contract_id: contract_id.to_string(),
            action: "deprecate".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            nonce: nonce.to_string(),
        }
    }

    fn generate_test_keypair() -> (SigningKey, VerifyingKey) {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        (signing_key, verifying_key)
    }

    // ── Signing message tests ────────────────────────────────────────────

    #[test]
    fn signing_message_is_deterministic() {
        let payload = DeprecationPayload {
            contract_id: "CABC123".to_string(),
            action: "deprecate".to_string(),
            timestamp: "2025-07-01T00:00:00Z".to_string(),
            nonce: "test-nonce-1".to_string(),
        };
        let msg1 = build_signing_message(&payload);
        let msg2 = build_signing_message(&payload);
        assert_eq!(msg1, msg2);
        assert_eq!(msg1, "deprecate:CABC123:2025-07-01T00:00:00Z:test-nonce-1");
    }

    #[test]
    fn signing_message_varies_with_nonce() {
        let p1 = make_payload("CABC123", "nonce-aaa");
        let p2 = make_payload("CABC123", "nonce-bbb");
        assert_ne!(build_signing_message(&p1), build_signing_message(&p2));
    }

    #[test]
    fn signing_message_varies_with_contract_id() {
        let p1 = make_payload("CONTRACT_A", "same-nonce");
        let p2 = make_payload("CONTRACT_B", "same-nonce");
        assert_ne!(build_signing_message(&p1), build_signing_message(&p2));
    }

    #[test]
    fn signing_message_includes_action() {
        let payload = make_payload("CTEST", "n1");
        let msg = build_signing_message(&payload);
        assert!(msg.starts_with("deprecate:"));
    }

    // ── Signature roundtrip tests ────────────────────────────────────────

    #[test]
    fn valid_signature_roundtrip() {
        let (signing_key, verifying_key) = generate_test_keypair();
        let payload = make_payload("CABC123", "roundtrip-nonce");
        let sig_b64 = sign_deprecation(&payload, &signing_key);

        let valid = verify_deprecation_signature(&payload, &sig_b64, &verifying_key)
            .expect("verification should not error");
        assert!(valid, "Signature should verify with correct key");
    }

    #[test]
    fn wrong_key_signature_fails_verification() {
        let (signing_key_a, _) = generate_test_keypair();
        let (_, verifying_key_b) = generate_test_keypair();

        let payload = make_payload("CABC123", "wrong-key-nonce");
        let sig_b64 = sign_deprecation(&payload, &signing_key_a);

        let valid = verify_deprecation_signature(&payload, &sig_b64, &verifying_key_b)
            .expect("verification should not error");
        assert!(!valid, "Signature from key A should NOT verify with key B");
    }

    #[test]
    fn tampered_payload_fails_verification() {
        let (signing_key, verifying_key) = generate_test_keypair();
        let payload = make_payload("CABC123", "original-nonce");
        let sig_b64 = sign_deprecation(&payload, &signing_key);

        // Tamper with the contract_id
        let tampered = DeprecationPayload {
            contract_id: "CTAMPERED".to_string(),
            ..payload
        };

        let valid = verify_deprecation_signature(&tampered, &sig_b64, &verifying_key)
            .expect("verification should not error");
        assert!(!valid, "Tampered payload should fail verification");
    }

    #[test]
    fn replay_with_different_nonce_produces_different_signature() {
        let (signing_key, _) = generate_test_keypair();

        let p1 = DeprecationPayload {
            contract_id: "CABC123".to_string(),
            action: "deprecate".to_string(),
            timestamp: "2025-07-01T00:00:00Z".to_string(),
            nonce: "nonce-1".to_string(),
        };
        let p2 = DeprecationPayload {
            nonce: "nonce-2".to_string(),
            ..p1.clone()
        };

        let sig1 = sign_deprecation(&p1, &signing_key);
        let sig2 = sign_deprecation(&p2, &signing_key);
        assert_ne!(sig1, sig2, "Different nonces must produce different signatures");
    }

    // ── Timestamp validation tests ───────────────────────────────────────

    #[test]
    fn fresh_timestamp_is_valid() {
        let ts = Utc::now().to_rfc3339();
        assert!(is_timestamp_valid(&ts).unwrap());
    }

    #[test]
    fn old_timestamp_is_expired() {
        let old = Utc::now() - chrono::Duration::seconds(MAX_SIGNATURE_AGE_SECS + 60);
        let ts = old.to_rfc3339();
        assert!(
            !is_timestamp_valid(&ts).unwrap(),
            "Timestamp older than {} seconds should be rejected",
            MAX_SIGNATURE_AGE_SECS
        );
    }

    #[test]
    fn invalid_timestamp_format_errors() {
        assert!(is_timestamp_valid("not-a-date").is_err());
    }

    // ── Key handling tests ───────────────────────────────────────────────

    #[test]
    fn decode_valid_private_key() {
        let (signing_key, _) = generate_test_keypair();
        let encoded = BASE64.encode(signing_key.to_bytes());
        let decoded = decode_private_key(&encoded);
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap().to_bytes(), signing_key.to_bytes());
    }

    #[test]
    fn decode_invalid_base64_private_key_errors() {
        let result = decode_private_key("not-valid-base64!!!");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("base64"), "Error should mention base64: {}", err);
    }

    #[test]
    fn decode_wrong_length_private_key_errors() {
        let too_short = BASE64.encode(b"tooshort");
        let result = decode_private_key(&too_short);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("32 bytes"), "Error should mention key length: {}", err);
    }

    // ── Stellar address derivation ───────────────────────────────────────

    #[test]
    fn stellar_address_is_deterministic() {
        let (_, verifying_key) = generate_test_keypair();
        let bytes = verifying_key.to_bytes();
        let addr1 = derive_stellar_address(&bytes);
        let addr2 = derive_stellar_address(&bytes);
        assert_eq!(addr1, addr2);
        assert!(!addr1.is_empty());
    }

    #[test]
    fn different_keys_produce_different_addresses() {
        let (_, vk1) = generate_test_keypair();
        let (_, vk2) = generate_test_keypair();
        let addr1 = derive_stellar_address(&vk1.to_bytes());
        let addr2 = derive_stellar_address(&vk2.to_bytes());
        assert_ne!(addr1, addr2);
    }

    // ── Serialization tests ──────────────────────────────────────────────

    #[test]
    fn deprecation_payload_json_roundtrip() {
        let payload = make_payload("CTEST123", "test-nonce");
        let json = serde_json::to_string(&payload).unwrap();
        let parsed: DeprecationPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.contract_id, "CTEST123");
        assert_eq!(parsed.action, "deprecate");
        assert_eq!(parsed.nonce, "test-nonce");
    }

    #[test]
    fn signed_request_serialization_includes_all_fields() {
        let (signing_key, verifying_key) = generate_test_keypair();
        let payload = make_payload("CTEST", "ser-nonce");
        let sig = sign_deprecation(&payload, &signing_key);
        let pub_key = BASE64.encode(verifying_key.to_bytes());
        let address = derive_stellar_address(&verifying_key.to_bytes());

        let request = SignedDeprecationRequest {
            reason: "end of life".to_string(),
            replacement_contract_id: Some("CNEW".to_string()),
            migration_guide_url: Some("https://example.com/migrate".to_string()),
            grace_period_days: 30,
            payload,
            signature: sig,
            public_key: pub_key,
            signing_address: address,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["reason"], "end of life");
        assert_eq!(json["replacement_contract_id"], "CNEW");
        assert_eq!(json["grace_period_days"], 30);
        assert!(json["signature"].is_string());
        assert!(json["public_key"].is_string());
        assert!(json["signing_address"].is_string());
        assert!(json["payload"]["contract_id"].is_string());
    }
}
