//! contract_snapshot.rs — `soroban-registry contract snapshot` / `verify-snapshot` (#1116)
//!
//! Produces a portable, signed record of a contract's lifecycle state — metadata,
//! verification status, dependency scan findings, deprecation state and successor
//! lineage — as a single JSON file, and verifies such a file offline.
//!
//! Verification performs no network calls, so an exported snapshot can be audited
//! in an air-gapped environment. The document shape, canonical form and signature
//! checks come from `shared::snapshot`, so the CLI validates exactly the bytes the
//! registry signed rather than a reimplementation of them.

use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde::Deserialize;
use shared::snapshot::{verify_snapshot, ContractSnapshot, SnapshotError};
use std::path::Path;

use crate::net::RequestBuilderExt;

/// Registry signing identity, as published by `GET /api/registry/signing-key`.
#[derive(Debug, Deserialize)]
struct RegistrySigningKey {
    key_fingerprint: String,
}

// ── export ───────────────────────────────────────────────────────────────────

/// `soroban-registry contract snapshot <ID> --output <FILE>`
pub async fn run_export(
    api_url: &str,
    contract_id: &str,
    output: &str,
    json_output: bool,
) -> Result<()> {
    let url = format!(
        "{}/api/contracts/{}/snapshot",
        api_url.trim_end_matches('/'),
        contract_id
    );

    let response = crate::net::client()
        .get(&url)
        .send_with_retry()
        .await
        .with_context(|| format!("failed to reach the registry at {url}"))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .context("failed to read the registry response body")?;

    if !status.is_success() {
        if status.as_u16() == 503 {
            bail!(
                "the registry cannot sign snapshots: REGISTRY_SIGNING_KEY is not configured \
                 on the server ({status})"
            );
        }
        bail!("registry returned {status}: {body}");
    }

    // Parse before writing so a malformed response is not persisted as if it
    // were a valid snapshot.
    let snapshot: ContractSnapshot = serde_json::from_str(&body)
        .context("registry returned a response that is not a valid contract snapshot")?;

    // Confirm the registry signed what it sent us. Catches a misconfigured or
    // truncated response at export time rather than at audit time.
    verify_snapshot(&snapshot, None, None).map_err(|e| {
        anyhow::anyhow!("the registry returned a snapshot that fails its own signature check: {e}")
    })?;

    let pretty = serde_json::to_string_pretty(&snapshot)
        .context("failed to serialize the snapshot for writing")?;
    std::fs::write(output, pretty.as_bytes())
        .with_context(|| format!("failed to write snapshot to {output}"))?;

    if json_output {
        println!(
            "{}",
            serde_json::json!({
                "output": output,
                "contract_id": snapshot.payload.contract.contract_id,
                "exported_at": snapshot.payload.exported_at,
                "key_fingerprint": snapshot.signature.key_fingerprint,
                "lineage_depth": snapshot.payload.lineage.len(),
            })
        );
    } else {
        println!("{} {}", "Snapshot written to".green(), output.bold());
        println!("  contract:    {}", snapshot.payload.contract.contract_id);
        println!("  name:        {}", snapshot.payload.contract.name);
        println!("  network:     {}", snapshot.payload.contract.network);
        println!("  exported at: {}", snapshot.payload.exported_at);
        println!("  signed by:   {}", snapshot.signature.key_fingerprint);
        println!();
        println!(
            "{}",
            "Pin this fingerprint out of band. Verifying without --expect-key only proves the"
                .dimmed()
        );
        println!(
            "{}",
            "bundle is self-consistent, not that this registry produced it.".dimmed()
        );
    }

    Ok(())
}

// ── verify ───────────────────────────────────────────────────────────────────

/// `soroban-registry contract verify-snapshot <FILE>`
///
/// Offline by default. `--fetch-key` is the one path that touches the network,
/// and is opt-in precisely because the point of the command is to work without it.
pub async fn run_verify(
    api_url: &str,
    file: &str,
    expect_key: Option<&str>,
    max_age_days: Option<i64>,
    fetch_key: bool,
    json_output: bool,
) -> Result<()> {
    let path = Path::new(file);
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read snapshot file {file}"))?;

    let snapshot: ContractSnapshot = serde_json::from_str(&raw)
        .with_context(|| format!("{file} is not a valid contract snapshot document"))?;

    // Resolve the fingerprint to pin against, in order of trust.
    let expected: Option<String> = match (expect_key, fetch_key) {
        (Some(fp), _) => Some(fp.trim().to_string()),
        (None, true) => Some(fetch_registry_fingerprint(api_url).await?),
        (None, false) => None,
    };

    let result = verify_snapshot(&snapshot, expected.as_deref(), max_age_days);

    match result {
        Ok(()) => {
            if json_output {
                println!(
                    "{}",
                    serde_json::json!({
                        "valid": true,
                        "pinned": expected.is_some(),
                        "contract_id": snapshot.payload.contract.contract_id,
                        "exported_at": snapshot.payload.exported_at,
                        "key_fingerprint": snapshot.signature.key_fingerprint,
                    })
                );
            } else {
                println!("{}", "Snapshot signature is valid.".green().bold());
                println!("  contract:    {}", snapshot.payload.contract.contract_id);
                println!("  name:        {}", snapshot.payload.contract.name);
                println!("  network:     {}", snapshot.payload.contract.network);
                println!(
                    "  verified:    {}",
                    snapshot.payload.verification.is_verified
                );
                println!("  exported at: {}", snapshot.payload.exported_at);
                println!("  signed by:   {}", snapshot.signature.key_fingerprint);

                if expected.is_some() {
                    println!("  key:         {}", "pinned and matched".green());
                } else {
                    println!(
                        "  key:         {}",
                        "NOT pinned - authenticity unproven".yellow()
                    );
                    println!();
                    println!(
                        "{}",
                        "Re-run with --expect-key <fingerprint> to confirm this snapshot came from"
                            .dimmed()
                    );
                    println!("{}", "the registry you trust.".dimmed());
                }
            }
            Ok(())
        }
        Err(err) => {
            if json_output {
                println!(
                    "{}",
                    serde_json::json!({
                        "valid": false,
                        "reason": err.to_string(),
                        "kind": error_kind(&err),
                    })
                );
            } else {
                println!("{} {}", "Snapshot verification FAILED:".red().bold(), err);
            }
            // Non-zero exit so this is usable as a CI/compliance gate.
            std::process::exit(1);
        }
    }
}

/// Stable machine-readable discriminant for `--json` consumers.
fn error_kind(err: &SnapshotError) -> &'static str {
    match err {
        SnapshotError::UnsupportedSchema(_) => "unsupported_schema",
        SnapshotError::UnsupportedAlgorithm(_) => "unsupported_algorithm",
        SnapshotError::MalformedKey(_) => "malformed_key",
        SnapshotError::MalformedSignature(_) => "malformed_signature",
        SnapshotError::SignatureMismatch => "signature_mismatch",
        SnapshotError::UntrustedKey { .. } => "untrusted_key",
        SnapshotError::Stale { .. } => "stale",
        SnapshotError::Serialization(_) => "serialization",
    }
}

async fn fetch_registry_fingerprint(api_url: &str) -> Result<String> {
    let url = format!("{}/api/registry/signing-key", api_url.trim_end_matches('/'));

    let response = crate::net::client()
        .get(&url)
        .send_with_retry()
        .await
        .with_context(|| format!("failed to fetch the registry signing key from {url}"))?;

    if !response.status().is_success() {
        bail!(
            "registry returned {} when asked for its signing key",
            response.status()
        );
    }

    let key: RegistrySigningKey = response
        .json()
        .await
        .context("registry signing-key response was not in the expected form")?;

    Ok(key.key_fingerprint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::snapshot::{
        sign_snapshot, LineageLink, SnapshotContract, SnapshotPayload, SnapshotVerification,
        SNAPSHOT_SCHEMA_VERSION,
    };

    use chrono::Utc;
    use ed25519_dalek::SigningKey;

    fn payload() -> SnapshotPayload {
        SnapshotPayload {
            schema_version: SNAPSHOT_SCHEMA_VERSION.to_string(),
            exported_at: Utc::now(),
            registry_url: None,
            contract: SnapshotContract {
                id: "11111111-1111-1111-1111-111111111111".into(),
                contract_id: "CTEST".into(),
                name: "token".into(),
                network: "testnet".into(),
                wasm_hash: "deadbeef".into(),
                description: None,
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
            dependency_scan: None,
            deprecation: None,
            dependency_graph: None,
            lineage: vec![LineageLink {
                contract_id: "CNEXT".into(),
                name: None,
                status: "active".into(),
                deprecated_at: None,
            }],
        }
    }

    /// Round-trip through an actual file, the way the two commands are used.
    #[test]
    fn export_file_round_trips_through_verify() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let snapshot = sign_snapshot(payload(), &key).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snapshot.json");
        std::fs::write(&path, serde_json::to_string_pretty(&snapshot).unwrap()).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        let parsed: ContractSnapshot = serde_json::from_str(&raw).unwrap();

        assert!(verify_snapshot(&parsed, None, None).is_ok());
        assert!(verify_snapshot(&parsed, Some(&parsed.signature.key_fingerprint), None).is_ok());
    }

    /// Editing the file on disk must be caught.
    #[test]
    fn tampered_file_fails_verification() {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let snapshot = sign_snapshot(payload(), &key).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("snapshot.json");
        std::fs::write(&path, serde_json::to_string_pretty(&snapshot).unwrap()).unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        let tampered = raw.replace("\"name\": \"token\"", "\"name\": \"not-token\"");
        assert_ne!(tampered, raw, "fixture must actually change the document");
        std::fs::write(&path, &tampered).unwrap();

        let reread = std::fs::read_to_string(&path).unwrap();
        let parsed: ContractSnapshot = serde_json::from_str(&reread).unwrap();

        assert_eq!(
            verify_snapshot(&parsed, None, None).unwrap_err(),
            SnapshotError::SignatureMismatch
        );
    }

    #[test]
    fn error_kinds_are_stable() {
        assert_eq!(
            error_kind(&SnapshotError::SignatureMismatch),
            "signature_mismatch"
        );
        assert_eq!(
            error_kind(&SnapshotError::UntrustedKey {
                expected: "a".into(),
                actual: "b".into()
            }),
            "untrusted_key"
        );
        assert_eq!(
            error_kind(&SnapshotError::Stale {
                exported_at: Utc::now(),
                max_age_days: 30
            }),
            "stale"
        );
    }
}
