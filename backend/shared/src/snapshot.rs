//! Portable, signed contract snapshots (Issue #1116).
//!
//! A snapshot is a single JSON document capturing a contract's lifecycle state
//! at a point in time, signed by the registry so it can be audited offline —
//! for compliance, archival, or air-gapped review — without reaching the API.
//!
//! Both the registry and the CLI build and verify snapshots through this
//! module, so the bytes covered by the signature are produced by one
//! implementation rather than two that must be kept in step.
//!
//! # Canonical form
//!
//! The signature covers a canonical serialization of the payload: object keys
//! sorted lexicographically, no insignificant whitespace. Sorting is applied
//! explicitly rather than relying on `serde_json`'s map type, whose ordering
//! depends on the `preserve_order` feature that any crate in the dependency
//! graph could enable.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

/// Bumped when the payload shape changes in a way that alters canonical bytes.
pub const SNAPSHOT_SCHEMA_VERSION: &str = "1.0";

pub const SNAPSHOT_ALGORITHM: &str = "ed25519";

/// A signed snapshot: the state captured, plus the registry's signature over it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSnapshot {
    pub payload: SnapshotPayload,
    pub signature: SnapshotSignature,
}

/// Everything the signature covers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotPayload {
    pub schema_version: String,
    /// When the registry assembled this snapshot.
    pub exported_at: DateTime<Utc>,
    /// Registry that produced it, for provenance when several are in play.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_url: Option<String>,
    pub contract: SnapshotContract,
    pub verification: SnapshotVerification,
    /// Dependency vulnerability report, absent when the contract has never
    /// been scanned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependency_scan: Option<Value>,
    /// Deprecation state, absent when the contract is active and has never
    /// been deprecated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<Value>,
    /// Successor chain, oldest first, built by following
    /// `replacement_contract_id` transitively. Empty when nothing supersedes
    /// this contract.
    pub lineage: Vec<LineageLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotContract {
    pub id: String,
    pub contract_id: String,
    pub name: String,
    pub network: String,
    pub wasm_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotVerification {
    pub is_verified: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_at: Option<DateTime<Utc>>,
}

/// One hop in the successor chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageLink {
    pub contract_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSignature {
    pub algorithm: String,
    /// Base64 ed25519 public key, so the bundle carries what is needed to
    /// check it. See `verify_snapshot` on why that alone is not authenticity.
    pub public_key: String,
    /// Hex sha256 over the raw public key bytes. Pin this out of band.
    pub key_fingerprint: String,
    /// Base64 signature over the canonical payload bytes.
    pub signature: String,
    pub signed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    UnsupportedSchema(String),
    UnsupportedAlgorithm(String),
    MalformedKey(String),
    MalformedSignature(String),
    /// The payload does not match the signature: tampered or re-serialized by
    /// a different implementation.
    SignatureMismatch,
    /// Signature is valid but the key is not the one the caller expected.
    UntrustedKey {
        expected: String,
        actual: String,
    },
    /// Snapshot is older than the caller's freshness tolerance.
    Stale {
        exported_at: DateTime<Utc>,
        max_age_days: i64,
    },
    Serialization(String),
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema(v) => {
                write!(f, "unsupported snapshot schema version: {v}")
            }
            Self::UnsupportedAlgorithm(a) => write!(f, "unsupported signature algorithm: {a}"),
            Self::MalformedKey(e) => write!(f, "malformed public key: {e}"),
            Self::MalformedSignature(e) => write!(f, "malformed signature: {e}"),
            Self::SignatureMismatch => write!(
                f,
                "signature does not match payload: the snapshot has been modified"
            ),
            Self::UntrustedKey { expected, actual } => write!(
                f,
                "snapshot is signed by an untrusted key: expected fingerprint {expected}, found {actual}"
            ),
            Self::Stale {
                exported_at,
                max_age_days,
            } => write!(
                f,
                "snapshot exported at {exported_at} is older than the {max_age_days} day freshness limit"
            ),
            Self::Serialization(e) => write!(f, "failed to serialize snapshot payload: {e}"),
        }
    }
}

impl std::error::Error for SnapshotError {}

/// Recursively sort object keys so the same payload always yields the same
/// bytes regardless of struct field order or map implementation.
fn canonicalize(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted = Map::new();
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            for (k, v) in entries {
                sorted.insert(k, canonicalize(v));
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(canonicalize).collect()),
        other => other,
    }
}

/// The exact bytes the signature covers.
pub fn canonical_payload_bytes(payload: &SnapshotPayload) -> Result<Vec<u8>, SnapshotError> {
    let value =
        serde_json::to_value(payload).map_err(|e| SnapshotError::Serialization(e.to_string()))?;
    serde_json::to_vec(&canonicalize(value))
        .map_err(|e| SnapshotError::Serialization(e.to_string()))
}

/// Hex sha256 over raw public key bytes.
pub fn key_fingerprint(public_key: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(public_key);
    hex::encode(hasher.finalize())
}

/// Sign a payload with the registry's key, producing a complete snapshot.
pub fn sign_snapshot(
    payload: SnapshotPayload,
    signing_key: &SigningKey,
) -> Result<ContractSnapshot, SnapshotError> {
    let bytes = canonical_payload_bytes(&payload)?;
    let signature = signing_key.sign(&bytes);
    let verifying = signing_key.verifying_key();

    Ok(ContractSnapshot {
        payload,
        signature: SnapshotSignature {
            algorithm: SNAPSHOT_ALGORITHM.to_string(),
            public_key: BASE64.encode(verifying.as_bytes()),
            key_fingerprint: key_fingerprint(verifying.as_bytes()),
            signature: BASE64.encode(signature.to_bytes()),
            signed_at: Utc::now(),
        },
    })
}

/// Verify a snapshot's signature without contacting the registry.
///
/// A valid result means the payload has not been altered since it was signed by
/// the holder of the embedded key. It does **not** by itself prove the registry
/// produced it: anyone can re-sign a modified payload with their own key and
/// embed that public key. Pass `expected_fingerprint` — obtained out of band
/// from the registry's published key — to make this an authenticity check.
pub fn verify_snapshot(
    snapshot: &ContractSnapshot,
    expected_fingerprint: Option<&str>,
    max_age_days: Option<i64>,
) -> Result<(), SnapshotError> {
    if snapshot.payload.schema_version != SNAPSHOT_SCHEMA_VERSION {
        return Err(SnapshotError::UnsupportedSchema(
            snapshot.payload.schema_version.clone(),
        ));
    }

    if snapshot.signature.algorithm != SNAPSHOT_ALGORITHM {
        return Err(SnapshotError::UnsupportedAlgorithm(
            snapshot.signature.algorithm.clone(),
        ));
    }

    let key_bytes = BASE64
        .decode(&snapshot.signature.public_key)
        .map_err(|e| SnapshotError::MalformedKey(e.to_string()))?;
    let key_array: [u8; 32] = key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| SnapshotError::MalformedKey(format!("expected 32 bytes, got {}", key_bytes.len())))?;
    let verifying = VerifyingKey::from_bytes(&key_array)
        .map_err(|e| SnapshotError::MalformedKey(e.to_string()))?;

    // Check the key before the signature: "signed by the wrong key" is a more
    // useful diagnosis than "signature valid" for a bundle from an unknown source.
    let actual_fingerprint = key_fingerprint(verifying.as_bytes());
    if let Some(expected) = expected_fingerprint {
        if !expected.eq_ignore_ascii_case(&actual_fingerprint) {
            return Err(SnapshotError::UntrustedKey {
                expected: expected.to_string(),
                actual: actual_fingerprint,
            });
        }
    }

    let sig_bytes = BASE64
        .decode(&snapshot.signature.signature)
        .map_err(|e| SnapshotError::MalformedSignature(e.to_string()))?;
    let sig_array: [u8; 64] = sig_bytes.as_slice().try_into().map_err(|_| {
        SnapshotError::MalformedSignature(format!("expected 64 bytes, got {}", sig_bytes.len()))
    })?;
    let signature = Signature::from_bytes(&sig_array);

    let bytes = canonical_payload_bytes(&snapshot.payload)?;
    verifying
        .verify(&bytes, &signature)
        .map_err(|_| SnapshotError::SignatureMismatch)?;

    if let Some(max_age) = max_age_days {
        let age = Utc::now() - snapshot.payload.exported_at;
        if age.num_days() > max_age {
            return Err(SnapshotError::Stale {
                exported_at: snapshot.payload.exported_at,
                max_age_days: max_age,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn test_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn sample_payload() -> SnapshotPayload {
        SnapshotPayload {
            schema_version: SNAPSHOT_SCHEMA_VERSION.to_string(),
            exported_at: Utc::now(),
            registry_url: Some("https://registry.example".into()),
            contract: SnapshotContract {
                id: "11111111-1111-1111-1111-111111111111".into(),
                contract_id: "CAAAA".into(),
                name: "token".into(),
                network: "testnet".into(),
                wasm_hash: "deadbeef".into(),
                description: Some("a token".into()),
                category: None,
                publisher_id: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            verification: SnapshotVerification {
                is_verified: true,
                status: Some("verified".into()),
                verified_at: Some(Utc::now()),
            },
            dependency_scan: Some(serde_json::json!({"findings": [], "scanned": true})),
            deprecation: None,
            lineage: vec![LineageLink {
                contract_id: "CBBBB".into(),
                name: Some("token-v2".into()),
                status: "active".into(),
                deprecated_at: None,
            }],
        }
    }

    #[test]
    fn round_trip_export_then_verify() {
        let key = test_key(1);
        let snapshot = sign_snapshot(sample_payload(), &key).unwrap();

        // Through serialized form, as a file on disk would be.
        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: ContractSnapshot = serde_json::from_str(&json).unwrap();

        assert!(verify_snapshot(&parsed, None, None).is_ok());
    }

    #[test]
    fn round_trip_survives_key_reordering() {
        // A verifier that re-serializes with different key order must still
        // reach the same canonical bytes.
        let key = test_key(1);
        let snapshot = sign_snapshot(sample_payload(), &key).unwrap();

        let value: Value = serde_json::to_value(&snapshot).unwrap();
        let shuffled = serde_json::to_string(&value).unwrap();
        let parsed: ContractSnapshot = serde_json::from_str(&shuffled).unwrap();

        assert!(verify_snapshot(&parsed, None, None).is_ok());
    }

    #[test]
    fn tampered_field_is_detected() {
        let key = test_key(1);
        let mut snapshot = sign_snapshot(sample_payload(), &key).unwrap();

        snapshot.payload.verification.is_verified = false;

        assert_eq!(
            verify_snapshot(&snapshot, None, None).unwrap_err(),
            SnapshotError::SignatureMismatch
        );
    }

    #[test]
    fn tampered_nested_field_is_detected() {
        let key = test_key(1);
        let mut snapshot = sign_snapshot(sample_payload(), &key).unwrap();

        snapshot.payload.contract.wasm_hash = "cafebabe".into();

        assert_eq!(
            verify_snapshot(&snapshot, None, None).unwrap_err(),
            SnapshotError::SignatureMismatch
        );
    }

    #[test]
    fn tampered_lineage_is_detected() {
        let key = test_key(1);
        let mut snapshot = sign_snapshot(sample_payload(), &key).unwrap();

        snapshot.payload.lineage.clear();

        assert_eq!(
            verify_snapshot(&snapshot, None, None).unwrap_err(),
            SnapshotError::SignatureMismatch
        );
    }

    #[test]
    fn tampered_json_text_is_detected() {
        // Editing the file directly, the way an auditor's adversary would.
        let key = test_key(1);
        let snapshot = sign_snapshot(sample_payload(), &key).unwrap();
        let json = serde_json::to_string(&snapshot).unwrap();

        let edited = json.replace("\"is_verified\":true", "\"is_verified\":false");
        assert_ne!(edited, json, "test fixture must actually change");

        let parsed: ContractSnapshot = serde_json::from_str(&edited).unwrap();
        assert_eq!(
            verify_snapshot(&parsed, None, None).unwrap_err(),
            SnapshotError::SignatureMismatch
        );
    }

    #[test]
    fn resigning_with_another_key_is_caught_by_fingerprint_pinning() {
        // The attack the embedded public key alone does not stop: modify the
        // payload, re-sign with your own key, ship the matching public key.
        let registry = test_key(1);
        let attacker = test_key(9);

        let genuine = sign_snapshot(sample_payload(), &registry).unwrap();
        let expected = genuine.signature.key_fingerprint.clone();

        let mut forged_payload = sample_payload();
        forged_payload.verification.is_verified = false;
        let forged = sign_snapshot(forged_payload, &attacker).unwrap();

        // Internally consistent, so an unpinned check passes.
        assert!(verify_snapshot(&forged, None, None).is_ok());

        // Pinning the registry's fingerprint rejects it.
        match verify_snapshot(&forged, Some(&expected), None) {
            Err(SnapshotError::UntrustedKey { .. }) => {}
            other => panic!("expected UntrustedKey, got {other:?}"),
        }
    }

    #[test]
    fn correct_fingerprint_is_accepted() {
        let key = test_key(1);
        let snapshot = sign_snapshot(sample_payload(), &key).unwrap();
        let fp = snapshot.signature.key_fingerprint.clone();

        assert!(verify_snapshot(&snapshot, Some(&fp), None).is_ok());
        // Case-insensitive, since users paste fingerprints by hand.
        assert!(verify_snapshot(&snapshot, Some(&fp.to_uppercase()), None).is_ok());
    }

    #[test]
    fn stale_snapshot_is_rejected_within_tolerance() {
        let key = test_key(1);
        let mut payload = sample_payload();
        payload.exported_at = Utc::now() - chrono::Duration::days(40);
        let snapshot = sign_snapshot(payload, &key).unwrap();

        assert!(verify_snapshot(&snapshot, None, Some(90)).is_ok());
        match verify_snapshot(&snapshot, None, Some(30)) {
            Err(SnapshotError::Stale { .. }) => {}
            other => panic!("expected Stale, got {other:?}"),
        }
    }

    #[test]
    fn freshness_boundary_is_inclusive() {
        // Exactly at the limit is still fresh; only strictly older is stale.
        let key = test_key(1);
        let mut payload = sample_payload();
        payload.exported_at = Utc::now() - chrono::Duration::days(30);
        let snapshot = sign_snapshot(payload, &key).unwrap();

        assert!(verify_snapshot(&snapshot, None, Some(30)).is_ok());
    }

    #[test]
    fn schema_version_mismatch_is_rejected() {
        let key = test_key(1);
        let mut snapshot = sign_snapshot(sample_payload(), &key).unwrap();
        snapshot.payload.schema_version = "999.0".into();

        match verify_snapshot(&snapshot, None, None) {
            Err(SnapshotError::UnsupportedSchema(v)) => assert_eq!(v, "999.0"),
            other => panic!("expected UnsupportedSchema, got {other:?}"),
        }
    }

    #[test]
    fn malformed_signature_is_rejected_not_panicking() {
        let key = test_key(1);
        let mut snapshot = sign_snapshot(sample_payload(), &key).unwrap();
        snapshot.signature.signature = "not-base64!!".into();

        match verify_snapshot(&snapshot, None, None) {
            Err(SnapshotError::MalformedSignature(_)) => {}
            other => panic!("expected MalformedSignature, got {other:?}"),
        }

        snapshot.signature.signature = BASE64.encode([0u8; 10]);
        match verify_snapshot(&snapshot, None, None) {
            Err(SnapshotError::MalformedSignature(_)) => {}
            other => panic!("expected MalformedSignature, got {other:?}"),
        }
    }

    #[test]
    fn malformed_public_key_is_rejected_not_panicking() {
        let key = test_key(1);
        let mut snapshot = sign_snapshot(sample_payload(), &key).unwrap();
        snapshot.signature.public_key = BASE64.encode([0u8; 5]);

        match verify_snapshot(&snapshot, None, None) {
            Err(SnapshotError::MalformedKey(_)) => {}
            other => panic!("expected MalformedKey, got {other:?}"),
        }
    }

    #[test]
    fn canonical_bytes_have_sorted_keys() {
        let bytes = canonical_payload_bytes(&sample_payload()).unwrap();
        let text = String::from_utf8(bytes).unwrap();

        let contract = text.find("\"contract\"").unwrap();
        let exported = text.find("\"exported_at\"").unwrap();
        let schema = text.find("\"schema_version\"").unwrap();
        assert!(contract < exported && exported < schema, "keys must be sorted: {text}");
    }
}
