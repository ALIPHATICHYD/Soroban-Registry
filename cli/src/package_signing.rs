use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use colored::Colorize;
use ed25519_dalek::{Signer, SigningKey, Verifier};
use rand::rngs::OsRng;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

pub async fn sign_package(
    api_url: &str,
    package_path: &str,
    private_key: &str,
    contract_id: &str,
    version: &str,
    expires_at: Option<&str>,
) -> Result<()> {
    println!("\n{}", "Signing contract package...".bold().cyan());

    let package_data = read_package_file(package_path)?;
    let package_hash = compute_hash(&package_data);

    println!("  {}: {}", "Package".bold(), package_path.bright_black());
    println!("  {}: {}", "Hash".bold(), package_hash.bright_black());

    let signing_key = decode_private_key(private_key)?;
    let verifying_key = signing_key.verifying_key();
    let public_key_bytes = verifying_key.to_bytes();
    let public_key_b64 = BASE64.encode(public_key_bytes);

    let message = create_signing_message(&package_hash, contract_id, version);
    let signature = signing_key.sign(&message);
    let signature_b64 = BASE64.encode(signature.to_bytes());

    let signing_address = derive_stellar_address(&public_key_bytes);

    println!(
        "  {}: {}",
        "Signing Address".bold(),
        signing_address.bright_magenta()
    );
    println!("  {}: {}", "Contract ID".bold(), contract_id.bright_black());
    println!("  {}: {}", "Version".bold(), version);

    let client = reqwest::Client::new();
    let url = format!("{}/api/signatures", api_url);

    let expires_dt = expires_at
        .map(|s| chrono::DateTime::parse_from_rfc3339(s))
        .transpose()
        .context("Invalid expires_at format, use RFC3339 (e.g., 2025-12-31T23:59:59Z)")?
        .map(|dt| dt.with_timezone(&Utc));

    let payload = json!({
        "contract_id": contract_id,
        "version": version,
        "wasm_hash": package_hash,
        "signature": signature_b64,
        "signing_address": signing_address,
        "public_key": public_key_b64,
        "algorithm": "ed25519",
        "expires_at": expires_dt,
    });

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        bail!("API error: {}", err);
    }

    let result: serde_json::Value = response.json().await?;

    println!("{}", "\n✓ Package signed successfully!".green().bold());
    println!(
        "  {}: {}",
        "Signature ID".bold(),
        result["id"].as_str().unwrap_or("?")
    );
    println!(
        "  {}: {}",
        "Signed At".bold(),
        result["signed_at"].as_str().unwrap_or("?")
    );
    println!(
        "\n  {} Verify with: soroban-registry verify {} --contract-id {}\n",
        "→".bright_black(),
        package_path,
        contract_id
    );

    Ok(())
}

pub async fn verify_package(
    api_url: &str,
    package_path: &str,
    contract_id: &str,
    version: Option<&str>,
    signature_arg: Option<&str>,
) -> Result<()> {
    println!("\n{}", "Verifying package signature...".bold().cyan());

    let package_data = read_package_file(package_path)?;
    let package_hash = compute_hash(&package_data);

    println!("  {}: {}", "Package".bold(), package_path.bright_black());
    println!("  {}: {}", "Hash".bold(), package_hash.bright_black());

    let client = reqwest::Client::new();

    if let Some(sig_b64) = signature_arg {
        verify_with_signature(
            api_url,
            &client,
            contract_id,
            version,
            &package_hash,
            sig_b64,
        )
        .await
    } else {
        verify_from_registry(api_url, &client, contract_id, version, &package_hash).await
    }
}

async fn verify_with_signature(
    api_url: &str,
    client: &reqwest::Client,
    contract_id: &str,
    version: Option<&str>,
    package_hash: &str,
    signature_b64: &str,
) -> Result<()> {
    let url = format!("{}/api/signatures/verify", api_url);

    let payload = json!({
        "contract_id": contract_id,
        "version": version,
        "wasm_hash": package_hash,
        "signature": signature_b64,
    });

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .context("Failed to reach registry API")?;

    let status = response.status();
    let result: serde_json::Value = response.json().await?;

    if !status.is_success() {
        let msg = result["message"].as_str().unwrap_or("Unknown error");
        bail!("Verification failed: {}", msg);
    }

    let valid = result["valid"].as_bool().unwrap_or(false);
    let signature_status = result["status"].as_str().unwrap_or("unknown");
    let signing_address = result["signing_address"].as_str().unwrap_or("?");

    if valid {
        println!("{}", "\n✓ Signature is VALID".green().bold());
        println!(
            "  {}: {}",
            "Signing Address".bold(),
            signing_address.bright_magenta()
        );
        println!("  {}: {}", "Status".bold(), signature_status.green());
        if let Some(signed_at) = result["signed_at"].as_str() {
            println!("  {}: {}", "Signed At".bold(), signed_at);
        }
    } else {
        println!("{}", "\n✗ Signature is INVALID".red().bold());
        println!("  {}: {}", "Status".bold(), signature_status.red());
    }

    println!();
    Ok(())
}

async fn verify_from_registry(
    api_url: &str,
    client: &reqwest::Client,
    contract_id: &str,
    version: Option<&str>,
    package_hash: &str,
) -> Result<()> {
    let mut url = format!(
        "{}/api/signatures/lookup?contract_id={}",
        api_url, contract_id
    );

    if let Some(v) = version {
        url.push_str(&format!("&version={}", v));
    }

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        bail!("Failed to lookup signature: {}", err);
    }

    let result: serde_json::Value = response.json().await?;
    let signatures = result["signatures"]
        .as_array()
        .context("No signatures found in response")?;

    if signatures.is_empty() {
        println!(
            "{}",
            "\n✗ No signatures found for this package".yellow().bold()
        );
        return Ok(());
    }

    let mut found_valid = false;

    for sig in signatures {
        let sig_hash = sig["wasm_hash"].as_str().unwrap_or("");
        let status = sig["status"].as_str().unwrap_or("unknown");

        if sig_hash == package_hash {
            found_valid = true;
            let signing_address = sig["signing_address"].as_str().unwrap_or("?");

            if status == "valid" {
                println!("{}", "\n✓ Found VALID signature".green().bold());
            } else if status == "revoked" {
                println!("{}", "\n✗ Signature has been REVOKED".red().bold());
            } else {
                println!(
                    "{}",
                    format!("\n⚠ Signature status: {}", status).yellow().bold()
                );
            }

            println!(
                "  {}: {}",
                "Signing Address".bold(),
                signing_address.bright_magenta()
            );
            println!("  {}: {}", "Status".bold(), status);
            println!(
                "  {}: {}",
                "Version".bold(),
                sig["version"].as_str().unwrap_or("?")
            );
            println!(
                "  {}: {}",
                "Signed At".bold(),
                sig["signed_at"].as_str().unwrap_or("?")
            );

            if let Some(reason) = sig["revoked_reason"].as_str() {
                println!("  {}: {}", "Revocation Reason".bold(), reason.red());
            }
        }
    }

    if !found_valid {
        println!(
            "{}",
            "\n✗ No matching signature found for this package hash"
                .yellow()
                .bold()
        );
    }

    println!();
    Ok(())
}

pub async fn revoke_signature(
    api_url: &str,
    signature_id: &str,
    revoked_by: &str,
    reason: &str,
) -> Result<()> {
    println!("\n{}", "Revoking signature...".bold().cyan());

    let client = reqwest::Client::new();
    let url = format!("{}/api/signatures/{}/revoke", api_url, signature_id);

    let payload = json!({
        "revoked_by": revoked_by,
        "reason": reason,
    });

    let response = client
        .post(&url)
        .json(&payload)
        .send()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        bail!("Failed to revoke signature: {}", err);
    }

    println!("{}", "✓ Signature revoked successfully!".green().bold());
    println!(
        "  {}: {}",
        "Signature ID".bold(),
        signature_id.bright_black()
    );
    println!("  {}: {}", "Revoked By".bold(), revoked_by.bright_magenta());
    println!("  {}: {}", "Reason".bold(), reason);
    println!();

    Ok(())
}

pub async fn get_chain_of_custody(api_url: &str, contract_id: &str) -> Result<()> {
    println!("\n{}", "Chain of Custody".bold().cyan());
    println!("{}", "=".repeat(70).cyan());

    let client = reqwest::Client::new();
    let url = format!("{}/api/signatures/custody/{}", api_url, contract_id);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        bail!("Failed to get chain of custody: {}", err);
    }

    let result: serde_json::Value = response.json().await?;
    let entries = result["entries"].as_array().cloned().unwrap_or_default();

    if entries.is_empty() {
        println!("{}", "\n  No custody records found.\n".yellow());
        return Ok(());
    }

    println!(
        "\n  {}: {}\n",
        "Contract ID".bold(),
        contract_id.bright_black()
    );

    for entry in &entries {
        let action = entry["action"].as_str().unwrap_or("?");
        let actor = entry["actor"].as_str().unwrap_or("?");
        let timestamp = entry["timestamp"].as_str().unwrap_or("?");

        let action_colored = match action {
            "package_signed" => action.green(),
            "signature_verified" => action.bright_blue(),
            "signature_revoked" => action.red(),
            _ => action.bright_black(),
        };

        println!(
            "  {} [{}] {} by {}",
            "•".bright_black(),
            timestamp,
            action_colored.bold(),
            actor.bright_magenta()
        );
    }

    println!("\n{}\n", "=".repeat(70).cyan());

    Ok(())
}

pub async fn get_transparency_log(
    api_url: &str,
    contract_id: Option<&str>,
    entry_type: Option<&str>,
    limit: usize,
) -> Result<()> {
    println!("\n{}", "Transparency Log".bold().cyan());
    println!("{}", "=".repeat(70).cyan());

    let client = reqwest::Client::new();
    let mut url = format!("{}/api/signatures/transparency?limit={}", api_url, limit);

    if let Some(cid) = contract_id {
        url.push_str(&format!("&contract_id={}", cid));
    }
    if let Some(et) = entry_type {
        url.push_str(&format!("&entry_type={}", et));
    }

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        bail!("Failed to get transparency log: {}", err);
    }

    let result: serde_json::Value = response.json().await?;
    let entries = result["items"].as_array().cloned().unwrap_or_default();

    if entries.is_empty() {
        println!("{}", "\n  No log entries found.\n".yellow());
        return Ok(());
    }

    println!();

    for entry in &entries {
        let entry_type = entry["entry_type"].as_str().unwrap_or("?");
        let actor = entry["actor_address"].as_str().unwrap_or("?");
        let timestamp = entry["timestamp"].as_str().unwrap_or("?");
        let hash = entry["entry_hash"].as_str().unwrap_or("?");

        let type_colored = match entry_type {
            "package_signed" => entry_type.green(),
            "signature_verified" => entry_type.bright_blue(),
            "signature_revoked" => entry_type.red(),
            "key_rotated" => entry_type.yellow(),
            _ => entry_type.bright_black(),
        };

        println!(
            "  {} [{}] {} by {}",
            "•".bright_black(),
            timestamp,
            type_colored.bold(),
            actor.bright_magenta()
        );
        println!("    Hash: {}", hash.bright_black());
        println!();
    }

    let total = result["total"].as_i64().unwrap_or(entries.len() as i64);
    println!(
        "{}\nShowing {} of {} entries\n",
        "=".repeat(70).cyan(),
        entries.len(),
        total
    );

    Ok(())
}

pub fn generate_keypair() -> Result<()> {
    println!("\n{}", "Generating Ed25519 keypair...".bold().cyan());

    let mut csprng = OsRng;
    let signing_key: SigningKey = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    let secret_bytes = signing_key.to_bytes();
    let public_bytes = verifying_key.to_bytes();

    let secret_b64 = BASE64.encode(secret_bytes);
    let public_b64 = BASE64.encode(public_bytes);
    let stellar_address = derive_stellar_address(&public_bytes);

    println!("{}", "\n✓ Keypair generated successfully!".green().bold());
    println!();
    println!("  {} (keep this secret!):", "Private Key".bold().red());
    println!("  {}", secret_b64.bright_red());
    println!();
    println!("  {}:", "Public Key".bold().green());
    println!("  {}", public_b64.bright_green());
    println!();
    println!("  {}:", "Stellar Address".bold().cyan());
    println!("  {}", stellar_address.bright_cyan());
    println!();
    println!(
        "  {} Use the private key to sign packages:",
        "→".bright_black()
    );
    println!(
        "  soroban-registry sign package.tar.gz --private-key {} --contract-id <ID> --version <VERSION>\n",
        secret_b64.bright_red()
    );

    Ok(())
}

fn read_package_file(path: &str) -> Result<Vec<u8>> {
    let path = Path::new(path);
    if !path.exists() {
        bail!("Package file not found: {}", path.display());
    }

    let mut file = fs::File::open(path).context("Failed to open package file")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .context("Failed to read package file")?;

    Ok(data)
}

fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn decode_private_key(key: &str) -> Result<SigningKey> {
    let bytes = BASE64
        .decode(key)
        .context("Invalid private key format (expected base64)")?;

    let bytes: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Private key must be 32 bytes"))?;

    Ok(SigningKey::from_bytes(&bytes))
}

fn create_signing_message(hash: &str, contract_id: &str, version: &str) -> Vec<u8> {
    format!("{}:{}:{}", contract_id, version, hash).into_bytes()
}

fn derive_stellar_address(public_key_bytes: &[u8; 32]) -> String {
    use ripemd::Ripemd160;
    use sha2::{Digest as _, Sha256};

    let sha256_hash = Sha256::digest(public_key_bytes);
    let ripemd_hash = Ripemd160::digest(&sha256_hash);

    let mut versioned = vec![0x00];
    versioned.extend_from_slice(&ripemd_hash);

    let checksum = Sha256::digest(&Sha256::digest(&versioned));
    versioned.extend_from_slice(&checksum[..4]);

    bs58::encode(&versioned).into_string()
}

/// Verify a contract binary locally against an Ed25519 signature and public key.
/// This does not contact the registry API and is suitable for offline verification.
pub fn verify_contract_local(
    wasm_path: &str,
    contract_id: &str,
    version: &str,
    signature_b64: &str,
    public_key_b64: &str,
) -> Result<()> {
    println!("\n{}", "Verifying contract binary signature...".bold().cyan());

    let wasm_bytes = read_package_file(wasm_path)?;
    let wasm_hash = compute_hash(&wasm_bytes);

    println!("  {}: {}", "Contract Path".bold(), wasm_path.bright_black());
    println!("  {}: {}", "Hash".bold(), wasm_hash.bright_black());
    println!("  {}: {}", "Contract ID".bold(), contract_id);
    println!("  {}: {}", "Version".bold(), version);

    // Decode public key
    let pk_bytes = BASE64
        .decode(public_key_b64.trim())
        .context("Invalid public key (expected base64-encoded Ed25519 key)")?;
    let pk_array: [u8; 32] = pk_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Public key must decode to 32 bytes"))?;
    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&pk_array)
        .map_err(|_| anyhow::anyhow!("Public key is not a valid Ed25519 key"))?;

    // Decode signature
    let sig_bytes = BASE64
        .decode(signature_b64.trim())
        .context("Invalid signature (expected base64-encoded Ed25519 signature)")?;
    let sig_array: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Signature must decode to 64 bytes"))?;
    let signature = ed25519_dalek::Signature::from_bytes(&sig_array);

    let message = create_signing_message(&wasm_hash, contract_id, version);

    let start = std::time::Instant::now();
    let ok = verifying_key.verify(&message, &signature).is_ok();
    let elapsed = start.elapsed();

    if ok {
        println!("{}", "\n✓ Signature is VALID".green().bold());
        println!(
            "  {}: {:.3} ms",
            "Verification time".bold(),
            elapsed.as_secs_f64() * 1000.0
        );
    } else {
        println!("{}", "\n✗ Signature is INVALID".red().bold());
        anyhow::bail!("Ed25519 verification failed for this contract binary");
    }

    println!();
    Ok(())
}
