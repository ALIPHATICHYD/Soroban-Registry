use crate::package_signing::decode_private_key;
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use colored::Colorize;
use ed25519_dalek::{Signer, Verifier};
use serde_json::json;
use sha2::{Digest, Sha256};
use shared::models::{Contract, ContractVersion, SignedSnapshot, SnapshotPayload};
use std::fs;

fn decode_public_key(key: &str) -> Result<ed25519_dalek::VerifyingKey> {
    let bytes = BASE64
        .decode(key)
        .context("Invalid public key format (expected base64)")?;

    let bytes: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Public key must be 32 bytes"))?;

    ed25519_dalek::VerifyingKey::from_bytes(&bytes)
        .map_err(|_| anyhow::anyhow!("Invalid public key"))
}

pub async fn export(api_url: &str, output: &str) -> Result<()> {
    println!("\n{}", "Exporting registry snapshot...".bold().cyan());

    let client = crate::net::client();
    let url = format!("{}/api/contracts?limit=1000", api_url);

    // Fetch contracts
    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        bail!("Failed to fetch contracts: {}", response.status());
    }

    let result: serde_json::Value = response.json().await?;
    let items = result
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut contracts = Vec::new();
    let mut versions = Vec::new();

    for item in items {
        if let Ok(contract) = serde_json::from_value::<Contract>(item.clone()) {
            contracts.push(contract.clone());

            // Fetch versions for this contract
            let versions_url = format!("{}/api/contracts/{}/versions", api_url, contract.id);
            if let Ok(v_resp) = client.get(&versions_url).send().await {
                if v_resp.status().is_success() {
                    if let Ok(v_data) = v_resp.json::<serde_json::Value>().await {
                        let v_items = v_data
                            .get("items")
                            .and_then(|v| v.as_array())
                            .cloned()
                            .unwrap_or_default();
                        for v_item in v_items {
                            if let Ok(version) = serde_json::from_value::<ContractVersion>(v_item) {
                                versions.push(version);
                            }
                        }
                    }
                }
            }
        }
    }

    let payload = SnapshotPayload {
        version: "1".to_string(),
        registry_identity: api_url.to_string(),
        network: "unknown".to_string(),
        timestamp: Utc::now(),
        contracts,
        versions,
        artifact_hashes: vec![],        // Populate if necessary
        interface_fingerprints: vec![], // Populate if necessary
        provenance_metadata: json!({}),
        deprecation_state: json!({}),
        vulnerability_state: json!({}),
    };

    let snapshot = SignedSnapshot {
        payload,
        signature: None,
        public_key: None,
    };

    let json = serde_json::to_string_pretty(&snapshot)?;
    fs::write(output, json)?;

    println!("{} Snapshot exported to {}", "[OK]".green(), output);
    Ok(())
}

fn canonical_hash(payload: &SnapshotPayload) -> Result<Vec<u8>> {
    // Basic canonicalization: serialize to JSON string and hash it.
    // In a real strict implementation, we would sort keys and remove whitespace.
    let json_bytes = serde_json::to_vec(&payload)?;
    let mut hasher = Sha256::new();
    hasher.update(&json_bytes);
    Ok(hasher.finalize().to_vec())
}

pub async fn sign(snapshot_file: &str, key_path: &str) -> Result<()> {
    println!("\n{}", "Signing registry snapshot...".bold().cyan());

    let snapshot_content = fs::read_to_string(snapshot_file)?;
    let mut snapshot: SignedSnapshot =
        serde_json::from_str(&snapshot_content).context("Failed to parse snapshot file")?;

    let hash = canonical_hash(&snapshot.payload)?;

    // Read and decode private key
    let key_content = fs::read_to_string(key_path)?;
    let signing_key = decode_private_key(key_content.trim())?;

    let signature = signing_key.sign(&hash);
    let signature_b64 = BASE64.encode(signature.to_bytes());

    let verifying_key = signing_key.verifying_key();
    let public_key_b64 = BASE64.encode(verifying_key.to_bytes());

    snapshot.signature = Some(signature_b64.clone());
    snapshot.public_key = Some(public_key_b64);

    let json = serde_json::to_string_pretty(&snapshot)?;
    fs::write(snapshot_file, json)?;

    println!("{} Snapshot signed successfully", "[OK]".green());
    Ok(())
}

pub async fn verify(snapshot_file: &str, trust_key_path: &str) -> Result<()> {
    println!("\n{}", "Verifying registry snapshot...".bold().cyan());

    let snapshot_content = fs::read_to_string(snapshot_file)?;
    let snapshot: SignedSnapshot =
        serde_json::from_str(&snapshot_content).context("Failed to parse snapshot file")?;

    let signature_b64 = snapshot
        .signature
        .as_ref()
        .context("Snapshot has no signature")?;

    // Check against trust key
    let trust_key_content = fs::read_to_string(trust_key_path)?;
    let verifying_key = decode_public_key(trust_key_content.trim())?;

    // If snapshot has a public key, optionally verify it matches the trust key
    if let Some(snapshot_pub_b64) = &snapshot.public_key {
        let expected_b64 = BASE64.encode(verifying_key.to_bytes());
        if *snapshot_pub_b64 != expected_b64 {
            bail!("Snapshot public key does not match the trusted key");
        }
    }

    let signature_bytes = BASE64
        .decode(signature_b64)
        .context("Invalid base64 signature")?;

    let sig = ed25519_dalek::Signature::from_slice(&signature_bytes)
        .context("Invalid Ed25519 signature format")?;

    let hash = canonical_hash(&snapshot.payload)?;

    if verifying_key.verify(&hash, &sig).is_ok() {
        println!("{} Signature is VALID", "[OK]".green());
        Ok(())
    } else {
        println!("{} Signature is INVALID", "[ERR]".red());
        bail!("Signature verification failed");
    }
}

pub async fn inspect(snapshot_file: &str) -> Result<()> {
    let snapshot_content = fs::read_to_string(snapshot_file)?;
    let snapshot: SignedSnapshot =
        serde_json::from_str(&snapshot_content).context("Failed to parse snapshot file")?;

    println!("\n{}", "Snapshot Metadata".bold().cyan());
    println!("  {}: {}", "Version".bold(), snapshot.payload.version);
    println!("  {}: {}", "Network".bold(), snapshot.payload.network);
    println!(
        "  {}: {}",
        "Identity".bold(),
        snapshot.payload.registry_identity
    );
    println!("  {}: {}", "Timestamp".bold(), snapshot.payload.timestamp);
    println!(
        "  {}: {}",
        "Contracts".bold(),
        snapshot.payload.contracts.len()
    );
    println!(
        "  {}: {}",
        "Versions".bold(),
        snapshot.payload.versions.len()
    );

    if let Some(sig) = &snapshot.signature {
        println!("  {}: {}", "Signed".bold(), "Yes".green());
    } else {
        println!("  {}: {}", "Signed".bold(), "No".yellow());
    }

    Ok(())
}
