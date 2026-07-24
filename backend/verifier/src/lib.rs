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
}
