pub mod deps;
// Contract verification engine
pub mod engine;
// Compiles source code and compares with on-chain bytecode

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use shared::RegistryError;
use std::process::Stdio;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::instrument;

const DEFAULT_SOROBAN_SDK_VERSION: &str = "21.7.7";
const BUILD_TIMEOUT: Duration = Duration::from_secs(120);

// ── Well-known Soroban network passphrases ────────────────────────────────────
//
// These are the canonical passphrases used by Stellar's public networks.
// They are embedded in every transaction envelope and uniquely identify a
// network at the protocol level.  Storing them alongside verification records
// allows the registry to detect mismatches for custom / private networks that
// happen to share the same enum label.

/// Stellar Mainnet passphrase.
pub const PASSPHRASE_MAINNET: &str = "Public Global Stellar Network ; September 2015";
/// Stellar Testnet passphrase.
pub const PASSPHRASE_TESTNET: &str = "Test SDF Network ; September 2015";
/// Stellar Futurenet passphrase.
pub const PASSPHRASE_FUTURENET: &str = "Test SDF Future Network ; October 2022";

/// Return the canonical passphrase for one of the three well-known networks,
/// or `None` for any custom / private network that has no fixed passphrase.
pub fn known_passphrase_for_network(network: &shared::Network) -> Option<&'static str> {
    match network {
        shared::Network::Mainnet => Some(PASSPHRASE_MAINNET),
        shared::Network::Testnet => Some(PASSPHRASE_TESTNET),
        shared::Network::Futurenet => Some(PASSPHRASE_FUTURENET),
    }
}

/// Precise reason a source-level verification step failed.
///
/// Returned alongside the boolean `verified` flag so callers can present
/// actionable guidance without exposing internal compiler details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VerificationFailureKind {
    /// The compiled wasm hash does not match the deployed wasm hash.
    /// Suggests source code, compiler version, or build flags differ.
    SourceMismatch {
        compiled_hash: String,
        deployed_hash: String,
        hint: String,
    },
    /// The ABI embedded in the registry cannot be parsed against the contract
    /// specification.  The `detail` field contains a sanitised error summary.
    AbiMismatch { detail: String },
    /// The contract does not exist on the specified network.
    NetworkMismatch { network: String },
    /// No on-chain wasm artifact was found to verify against.
    MissingArtifact,
    /// The passphrase supplied (or recorded at publish time) does not match
    /// the passphrase used during this verification attempt.
    ///
    /// This catches the case where two contracts share the same enum label
    /// (mainnet / testnet / futurenet) but belong to different physical
    /// networks (e.g. a private fork that recycles the "testnet" label).
    PassphraseMismatch {
        /// Passphrase that was recorded at publish / first verification time.
        recorded: String,
        /// Passphrase supplied in the current verification request.
        provided: String,
        hint: String,
    },
}

impl std::fmt::Display for VerificationFailureKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceMismatch { hint, .. } => write!(f, "source_mismatch: {hint}"),
            Self::AbiMismatch { detail } => write!(f, "abi_mismatch: {detail}"),
            Self::NetworkMismatch { network } => {
                write!(f, "network_mismatch: contract not found on {network}")
            }
            Self::MissingArtifact => write!(f, "missing_artifact: no on-chain artifact found"),
            Self::PassphraseMismatch {
                recorded, provided, ..
            } => {
                write!(
                    f,
                    "passphrase_mismatch: recorded passphrase '{recorded}' does not match \
                     provided passphrase '{provided}'"
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub verified: bool,
    pub compiled_wasm_hash: String,
    pub deployed_wasm_hash: String,
    pub message: Option<String>,
    /// Structured failure reason; `None` when `verified` is `true`.
    pub failure_kind: Option<VerificationFailureKind>,
}

/// Outcome of a passphrase compatibility check.
#[derive(Debug, Clone, PartialEq)]
pub enum PassphraseCheckOutcome {
    /// Passphrases match (or one / both were absent – treated as a soft pass
    /// for backward compatibility).
    Pass,
    /// The provided passphrase conflicts with the passphrase that was recorded
    /// for this contract at publish / first-verification time.
    Mismatch {
        recorded: String,
        provided: String,
    },
}

/// Compare a `provided` passphrase against the `recorded` one stored in the
/// registry.
///
/// # Rules
/// 1. If `recorded` is `None` the contract predates passphrase tracking; we
///    cannot reject it, so the check passes with a soft warning (the caller is
///    responsible for logging the warning).
/// 2. If `provided` is `None` the submitter did not supply a passphrase.  This
///    is also treated as a soft pass so existing tooling keeps working.
/// 3. Only when *both* sides are `Some` and they disagree do we return
///    `Mismatch`.
///
/// Comparisons are case-sensitive because Stellar passphrases are case-sensitive.
pub fn check_passphrase(
    recorded: Option<&str>,
    provided: Option<&str>,
) -> PassphraseCheckOutcome {
    match (recorded, provided) {
        (Some(rec), Some(prov)) if rec != prov => PassphraseCheckOutcome::Mismatch {
            recorded: rec.to_owned(),
            provided: prov.to_owned(),
        },
        _ => PassphraseCheckOutcome::Pass,
    }
}

#[instrument(skip(source_code, build_params), fields(component = "verifier", deployed_wasm_hash = %deployed_wasm_hash))]
pub async fn verify_contract(
    source_code: &str,
    deployed_wasm_hash: &str,
    compiler_version: Option<&str>,
    build_params: Option<&Value>,
) -> Result<VerificationResult, RegistryError> {
    if source_code.trim().is_empty() {
        return Err(RegistryError::invalid_input(
            "source_code cannot be empty".to_string(),
        ));
    }

    let deployed_normalized = normalize_hash(deployed_wasm_hash).ok_or_else(|| {
        RegistryError::invalid_input("deployed_wasm_hash must be a 64-char hex hash".to_string())
    })?;

    tracing::info!(
        deployed_wasm_hash = %deployed_normalized,
        "Starting contract verification"
    );

    let compiled_wasm = compile_contract(source_code, compiler_version, build_params).await?;
    let compiled_hash = hash_wasm(&compiled_wasm);

    if compiled_hash == deployed_normalized {
        return Ok(VerificationResult {
            verified: true,
            compiled_wasm_hash: compiled_hash,
            deployed_wasm_hash: deployed_normalized,
            message: None,
            failure_kind: None,
        });
    }

    let hint = "Check that the compiler version, build flags, and source code match the \
                deployed artifact exactly."
        .to_string();

    Ok(VerificationResult {
        verified: false,
        message: Some(format!(
            "Bytecode mismatch: compiled hash {} does not match deployed hash {}. {hint}",
            compiled_hash, deployed_normalized
        )),
        failure_kind: Some(VerificationFailureKind::SourceMismatch {
            compiled_hash: compiled_hash.clone(),
            deployed_hash: deployed_normalized.clone(),
            hint,
        }),
        compiled_wasm_hash: compiled_hash,
        deployed_wasm_hash: deployed_normalized,
    })
}

/// Passphrase-aware entry-point for contract verification.
///
/// Performs the same bytecode check as [`verify_contract`] **and** validates
/// the network passphrase before compiling anything.  A passphrase mismatch
/// is returned as a hard failure (`verified = false`) with a structured
/// [`VerificationFailureKind::PassphraseMismatch`] reason.
///
/// # Arguments
/// * `source_code`          – Rust source or `wasm_base64:…` payload.
/// * `deployed_wasm_hash`   – 64-char hex SHA-256 of the deployed WASM.
/// * `compiler_version`     – Optional Soroban SDK version string.
/// * `build_params`         – Optional extra cargo build flags.
/// * `recorded_passphrase`  – Passphrase stored in the registry for this
///                            contract (may be `None` for pre-1117 records).
/// * `provided_passphrase`  – Passphrase supplied by the verification caller
///                            (may be `None` for tooling that hasn't been
///                            updated yet).
#[instrument(
    skip(source_code, build_params),
    fields(component = "verifier", deployed_wasm_hash = %deployed_wasm_hash)
)]
pub async fn verify_contract_with_passphrase(
    source_code: &str,
    deployed_wasm_hash: &str,
    compiler_version: Option<&str>,
    build_params: Option<&Value>,
    recorded_passphrase: Option<&str>,
    provided_passphrase: Option<&str>,
) -> Result<VerificationResult, RegistryError> {
    // ── Passphrase guard ──────────────────────────────────────────────────────
    match check_passphrase(recorded_passphrase, provided_passphrase) {
        PassphraseCheckOutcome::Mismatch { recorded, provided } => {
            let hint = format!(
                "The network passphrase recorded at publish time ('{}') does not match \
                 the passphrase supplied in this verification request ('{}').  \
                 Ensure you are verifying against the correct network.",
                recorded, provided
            );
            return Ok(VerificationResult {
                verified: false,
                compiled_wasm_hash: String::new(),
                deployed_wasm_hash: deployed_wasm_hash.to_owned(),
                message: Some(hint.clone()),
                failure_kind: Some(VerificationFailureKind::PassphraseMismatch {
                    recorded: recorded.clone(),
                    provided: provided.clone(),
                    hint,
                }),
            });
        }
        PassphraseCheckOutcome::Pass => {}
    }

    // ── Bytecode check (delegates to existing logic) ──────────────────────────
    verify_contract(source_code, deployed_wasm_hash, compiler_version, build_params).await
}

/// Compile Rust source code to WASM.
/// Supports two source modes:
/// - raw Rust contract source (compiled with cargo)
/// - `wasm_base64:<...>` for precompiled test payloads
#[instrument(skip(source_code, build_params), fields(component = "verifier"))]
pub async fn compile_contract(
    source_code: &str,
    compiler_version: Option<&str>,
    build_params: Option<&Value>,
) -> Result<Vec<u8>, RegistryError> {
    if let Some(encoded) = source_code.trim().strip_prefix("wasm_base64:") {
        return BASE64.decode(encoded.trim()).map_err(|e| {
            RegistryError::invalid_input(format!("Invalid wasm_base64 payload: {}", e))
        });
    }

    let temp_dir = TempDir::new()
        .map_err(|e| RegistryError::internal(format!("Failed to create temp dir: {}", e)))?;
    bootstrap_project(temp_dir.path(), source_code, compiler_version).await?;

    let mut command = Command::new("cargo");
    command
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .current_dir(temp_dir.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(params) = build_params {
        apply_build_params(&mut command, params);
    }

    let output = timeout(BUILD_TIMEOUT, command.output())
        .await
        .map_err(|_| RegistryError::verification_failed("Compilation timed out".to_string()))?
        .map_err(|e| RegistryError::internal(format!("Failed to execute cargo build: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let details = format!(
            "Compilation failed. stdout: {} stderr: {}",
            truncate_for_error(&stdout),
            truncate_for_error(&stderr)
        );
        return Err(RegistryError::verification_failed(details));
    }

    let wasm_path = temp_dir
        .path()
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("verify_contract.wasm");

    if !wasm_path.exists() {
        // Try to find any wasm file if the standard name doesn't exist
        let release_dir = temp_dir
            .path()
            .join("target")
            .join("wasm32-unknown-unknown")
            .join("release");
        if let Ok(mut entries) = fs::read_dir(release_dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                    return Ok(fs::read(path).await?);
                }
            }
        }
        return Err(RegistryError::internal(
            "No WASM file found after build".to_string(),
        ));
    }

    Ok(fs::read(&wasm_path).await?)
}

async fn bootstrap_project(
    root: &std::path::Path,
    source_code: &str,
    compiler_version: Option<&str>,
) -> Result<(), RegistryError> {
    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir).await?;

    let sdk_version = compiler_version
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(DEFAULT_SOROBAN_SDK_VERSION);
    let cargo_toml = format!(
        "[package]\nname = \"verify_contract\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\nsoroban-sdk = \"{}\"\n",
        sdk_version
    );

    let cargo_path = root.join("Cargo.toml");
    fs::write(&cargo_path, cargo_toml).await?;

    let lib_path = src_dir.join("lib.rs");
    fs::write(&lib_path, source_code).await?;

    Ok(())
}

fn apply_build_params(command: &mut Command, build_params: &Value) {
    if let Some(profile) = build_params.get("profile").and_then(Value::as_str) {
        command.arg("--profile").arg(profile);
    }
    if let Some(features) = build_params.get("features").and_then(Value::as_array) {
        let joined = features
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>()
            .join(",");
        if !joined.is_empty() {
            command.arg("--features").arg(joined);
        }
    }
}

pub fn hash_wasm(wasm_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(wasm_bytes);
    hex::encode(hasher.finalize())
}

pub fn normalize_hash(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let stripped = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if stripped.len() != 64 || !stripped.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(stripped.to_ascii_lowercase())
}

fn truncate_for_error(value: &str) -> String {
    const MAX_ERROR_LEN: usize = 1_000;
    if value.len() <= MAX_ERROR_LEN {
        return value.to_string();
    }
    let mut out = value[..MAX_ERROR_LEN].to_string();
    out.push_str("...[truncated]");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_verify_contract_invalid_hash() {
        let result = verify_contract("fn main() {}", "invalid_hash", None, None).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn verify_contract_matches_known_good_wasm_pair() {
        let wasm = b"known-good-wasm";
        let expected_hash = hash_wasm(wasm);
        let source = format!("wasm_base64:{}", BASE64.encode(wasm));

        let result = verify_contract(&source, &expected_hash, None, None)
            .await
            .expect("verification should succeed");

        assert!(result.verified);
        assert_eq!(result.compiled_wasm_hash, expected_hash);
        assert!(result.message.is_none());
        assert!(result.failure_kind.is_none());
    }

    #[tokio::test]
    async fn verify_contract_detects_mismatch_for_known_bad_pair() {
        let source = format!("wasm_base64:{}", BASE64.encode(b"known-bad-wasm"));
        let wrong_hash = hash_wasm(b"different-wasm");

        let result = verify_contract(&source, &wrong_hash, None, None)
            .await
            .expect("verification should complete");

        assert!(!result.verified);
        assert!(result
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("Bytecode mismatch"));
    }

    #[tokio::test]
    async fn mismatch_produces_source_mismatch_failure_kind() {
        let compiled = b"contract-a";
        let deployed = b"contract-b";
        let source = format!("wasm_base64:{}", BASE64.encode(compiled));
        let deployed_hash = hash_wasm(deployed);

        let result = verify_contract(&source, &deployed_hash, None, None)
            .await
            .expect("verification should complete without error");

        assert!(!result.verified);
        match result.failure_kind {
            Some(VerificationFailureKind::SourceMismatch {
                compiled_hash,
                deployed_hash: dh,
                ..
            }) => {
                assert_eq!(compiled_hash, hash_wasm(compiled));
                assert_eq!(dh, deployed_hash);
            }
            other => panic!("expected SourceMismatch, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn empty_source_returns_error_not_mismatch() {
        let result = verify_contract("", "a".repeat(64).as_str(), None, None).await;
        assert!(result.is_err(), "empty source should be rejected");
    }

    #[tokio::test]
    async fn invalid_deployed_hash_returns_error() {
        let source = format!("wasm_base64:{}", BASE64.encode(b"wasm"));
        let result = verify_contract(&source, "not-a-hex-hash", None, None).await;
        assert!(result.is_err(), "invalid hash should be rejected");
    }

    // ── Passphrase check unit tests ───────────────────────────────────────────

    #[test]
    fn check_passphrase_both_match_returns_pass() {
        let outcome = check_passphrase(Some(PASSPHRASE_MAINNET), Some(PASSPHRASE_MAINNET));
        assert_eq!(outcome, PassphraseCheckOutcome::Pass);
    }

    #[test]
    fn check_passphrase_mismatch_returns_mismatch() {
        let outcome = check_passphrase(Some(PASSPHRASE_MAINNET), Some(PASSPHRASE_TESTNET));
        assert!(
            matches!(outcome, PassphraseCheckOutcome::Mismatch { .. }),
            "expected Mismatch, got {outcome:?}"
        );
    }

    #[test]
    fn check_passphrase_recorded_none_returns_pass() {
        // No recorded passphrase means the contract predates passphrase tracking –
        // treat as soft pass so existing records remain verifiable.
        let outcome = check_passphrase(None, Some(PASSPHRASE_TESTNET));
        assert_eq!(outcome, PassphraseCheckOutcome::Pass);
    }

    #[test]
    fn check_passphrase_provided_none_returns_pass() {
        // Caller didn't supply a passphrase – backward-compatible soft pass.
        let outcome = check_passphrase(Some(PASSPHRASE_MAINNET), None);
        assert_eq!(outcome, PassphraseCheckOutcome::Pass);
    }

    #[test]
    fn check_passphrase_both_none_returns_pass() {
        let outcome = check_passphrase(None, None);
        assert_eq!(outcome, PassphraseCheckOutcome::Pass);
    }

    #[test]
    fn known_passphrases_differ_across_networks() {
        assert_ne!(PASSPHRASE_MAINNET, PASSPHRASE_TESTNET);
        assert_ne!(PASSPHRASE_MAINNET, PASSPHRASE_FUTURENET);
        assert_ne!(PASSPHRASE_TESTNET, PASSPHRASE_FUTURENET);
    }

    #[test]
    fn known_passphrase_for_network_returns_correct_values() {
        assert_eq!(
            known_passphrase_for_network(&shared::Network::Mainnet),
            Some(PASSPHRASE_MAINNET)
        );
        assert_eq!(
            known_passphrase_for_network(&shared::Network::Testnet),
            Some(PASSPHRASE_TESTNET)
        );
        assert_eq!(
            known_passphrase_for_network(&shared::Network::Futurenet),
            Some(PASSPHRASE_FUTURENET)
        );
    }

    // ── Passphrase-aware verify_contract_with_passphrase tests ───────────────

    #[tokio::test]
    async fn passphrase_aware_verify_passes_when_passphrases_match() {
        let wasm = b"passphrase-good-wasm";
        let hash = hash_wasm(wasm);
        let source = format!("wasm_base64:{}", BASE64.encode(wasm));

        let result = verify_contract_with_passphrase(
            &source,
            &hash,
            None,
            None,
            Some(PASSPHRASE_TESTNET),
            Some(PASSPHRASE_TESTNET),
        )
        .await
        .expect("verify_contract_with_passphrase should not return Err");

        assert!(result.verified, "matching passphrases + matching wasm should pass");
        assert!(result.failure_kind.is_none());
    }

    #[tokio::test]
    async fn passphrase_aware_verify_fails_on_mismatch() {
        let wasm = b"passphrase-mismatch-wasm";
        let hash = hash_wasm(wasm);
        let source = format!("wasm_base64:{}", BASE64.encode(wasm));

        let result = verify_contract_with_passphrase(
            &source,
            &hash,
            None,
            None,
            Some(PASSPHRASE_MAINNET),  // recorded: mainnet
            Some(PASSPHRASE_TESTNET),  // provided: testnet  ← mismatch
        )
        .await
        .expect("verify_contract_with_passphrase should not return Err");

        assert!(!result.verified, "passphrase mismatch must reject verification");
        match &result.failure_kind {
            Some(VerificationFailureKind::PassphraseMismatch { recorded, provided, .. }) => {
                assert_eq!(recorded, PASSPHRASE_MAINNET);
                assert_eq!(provided, PASSPHRASE_TESTNET);
            }
            other => panic!("expected PassphraseMismatch, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn passphrase_aware_verify_passes_when_recorded_is_none() {
        // Pre-existing contracts that lack a recorded passphrase must still
        // be verifiable (backward compatibility requirement).
        let wasm = b"legacy-wasm-no-passphrase";
        let hash = hash_wasm(wasm);
        let source = format!("wasm_base64:{}", BASE64.encode(wasm));

        let result = verify_contract_with_passphrase(
            &source,
            &hash,
            None,
            None,
            None,                      // recorded: none (legacy record)
            Some(PASSPHRASE_TESTNET),  // provided: testnet
        )
        .await
        .expect("verify_contract_with_passphrase should not return Err");

        assert!(
            result.verified,
            "missing recorded passphrase should not block verification"
        );
    }

    #[tokio::test]
    async fn passphrase_aware_verify_passes_when_provided_is_none() {
        // Legacy tooling that doesn't supply a passphrase must keep working.
        let wasm = b"legacy-client-wasm";
        let hash = hash_wasm(wasm);
        let source = format!("wasm_base64:{}", BASE64.encode(wasm));

        let result = verify_contract_with_passphrase(
            &source,
            &hash,
            None,
            None,
            Some(PASSPHRASE_MAINNET), // recorded: mainnet
            None,                     // provided: none (old client)
        )
        .await
        .expect("verify_contract_with_passphrase should not return Err");

        assert!(
            result.verified,
            "missing provided passphrase should not block verification (backward compat)"
        );
    }

    #[tokio::test]
    async fn passphrase_mismatch_takes_priority_over_bytecode_mismatch() {
        // Even if the bytecode would also mismatch, passphrase guard fires first.
        let source = format!("wasm_base64:{}", BASE64.encode(b"some-wasm"));
        let wrong_hash = hash_wasm(b"different-wasm");

        let result = verify_contract_with_passphrase(
            &source,
            &wrong_hash,
            None,
            None,
            Some(PASSPHRASE_MAINNET),
            Some(PASSPHRASE_FUTURENET),
        )
        .await
        .expect("should not error");

        assert!(!result.verified);
        assert!(
            matches!(result.failure_kind, Some(VerificationFailureKind::PassphraseMismatch { .. })),
            "PassphraseMismatch must take priority over SourceMismatch"
        );
    }
}
