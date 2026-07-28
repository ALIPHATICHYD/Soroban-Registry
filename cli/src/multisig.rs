use crate::net::RequestBuilderExt;
// cli/src/multisig.rs
// CLI functions for Multi-Signature Contract Deployment (issue #47)

use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::json;

// ─────────────────────────────────────────────────────────────────────────────
// Create a new multi-sig policy
// ─────────────────────────────────────────────────────────────────────────────

pub async fn create_policy(
    api_url: &str,
    name: &str,
    threshold: u32,
    signers: Vec<String>,
    expiry_secs: Option<u32>,
    created_by: &str,
) -> Result<()> {
    let client = crate::net::client();
    let url = format!("{}/api/multisig/policies", api_url);

    let payload = json!({
        "name": name,
        "threshold": threshold,
        "signer_addresses": signers,
        "expiry_seconds": expiry_secs,
        "created_by": created_by,
    });

    println!("\n{}", "Creating multi-sig policy...".bold().cyan());

    let response = client
        .post(&url)
        .json(&payload)
        .send_with_retry()
        .await
        .context("Failed to reach registry API")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        anyhow::bail!("API error: {}", err);
    }

    let policy: serde_json::Value = response.json().await?;

    println!("{}", "✓ Policy created!".green().bold());
    println!(
        "  {}: {}",
        "ID".bold(),
        policy["id"].as_str().unwrap_or("?")
    );
    println!(
        "  {}: {}",
        "Name".bold(),
        policy["name"].as_str().unwrap_or("?")
    );
    println!(
        "  {}: {}-of-{}",
        "Threshold".bold(),
        policy["threshold"].as_i64().unwrap_or(0),
        policy["signer_addresses"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0),
    );
    println!(
        "  {}: {} seconds",
        "Expiry".bold(),
        policy["expiry_seconds"].as_i64().unwrap_or(86400)
    );

    if let Some(signers) = policy["signer_addresses"].as_array() {
        println!("\n  {} Authorized signers:", "→".bright_black());
        for s in signers {
            println!("    • {}", s.as_str().unwrap_or("?").bright_magenta());
        }
    }
    println!();

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Create a new deployment proposal
// ─────────────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub async fn create_proposal(
    api_url: &str,
    contract_name: &str,
    contract_id: &str,
    wasm_hash: &str,
    network: &str,
    policy_id: &str,
    proposer: &str,
    description: Option<&str>,
) -> Result<()> {
    let client = crate::net::client();
    let url = format!("{}/api/contracts/deploy-proposal", api_url);

    let payload = json!({
        "contract_name": contract_name,
        "contract_id": contract_id,
        "wasm_hash": wasm_hash,
        "network": network,
        "policy_id": policy_id,
        "proposer": proposer,
        "description": description,
    });

    println!("\n{}", "Creating deployment proposal...".bold().cyan());

    let response = client
        .post(&url)
        .json(&payload)
        .send_with_retry()
        .await
        .context("Failed to create deployment proposal")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        anyhow::bail!("API error: {}", err);
    }

    let proposal: serde_json::Value = response.json().await?;

    println!("{}", "✓ Proposal created!".green().bold());
    println!(
        "  {}: {}",
        "Proposal ID".bold(),
        proposal["id"].as_str().unwrap_or("?")
    );
    println!(
        "  {}: {}",
        "Contract".bold(),
        proposal["contract_id"]
            .as_str()
            .unwrap_or("?")
            .bright_black()
    );
    println!(
        "  {}: {}",
        "WASM Hash".bold(),
        proposal["wasm_hash"].as_str().unwrap_or("?").bright_black()
    );
    println!(
        "  {}: {}",
        "Network".bold(),
        proposal["network"].as_str().unwrap_or("?").bright_blue()
    );
    println!(
        "  {}: {}",
        "Status".bold(),
        proposal["status"].as_str().unwrap_or("?").yellow()
    );
    println!(
        "  {}: {}",
        "Expires at".bold(),
        proposal["expires_at"].as_str().unwrap_or("?")
    );
    println!(
        "\n  {} Share the Proposal ID to start collecting signatures.\n",
        "→".bright_black()
    );

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Sign a proposal
// ─────────────────────────────────────────────────────────────────────────────

pub async fn sign_proposal(
    api_url: &str,
    proposal_id: &str,
    signer_address: &str,
    signature_data: Option<&str>,
) -> Result<()> {
    let client = crate::net::client();
    let url = format!("{}/api/contracts/{}/sign", api_url, proposal_id);

    let payload = json!({
        "signer_address": signer_address,
        "signature_data": signature_data,
    });

    println!("\n{}", "Signing proposal...".bold().cyan());
    println!("  Proposal: {}", proposal_id.bright_black());
    println!("  Signer:   {}", signer_address.bright_magenta());

    let response = client
        .post(&url)
        .json(&payload)
        .send_with_retry()
        .await
        .context("Failed to sign proposal")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await?;

    if !status.is_success() {
        let err = body["message"].as_str().unwrap_or("unknown error");
        anyhow::bail!("API error ({}): {}", status, err);
    }

    println!("{}", "✓ Signature recorded!".green().bold());

    let collected = body["signatures_collected"].as_i64().unwrap_or(0);
    let needed = body["signatures_needed"].as_i64().unwrap_or(0);
    let threshold_met = body["threshold_met"].as_bool().unwrap_or(false);
    let proposal_status = body["proposal_status"].as_str().unwrap_or("pending");

    if threshold_met {
        println!(
            "  {} Threshold reached! Proposal status: {}",
            "🎉".bold(),
            proposal_status.green().bold()
        );
    } else {
        println!(
            "  Signatures: {}/{} collected — {} more needed",
            collected,
            collected + needed,
            needed.to_string().yellow().bold()
        );
    }
    println!("  Status: {}", proposal_status.yellow());
    println!();

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Execute a proposal
// ─────────────────────────────────────────────────────────────────────────────

pub async fn execute_proposal(api_url: &str, proposal_id: &str) -> Result<()> {
    let client = crate::net::client();
    let url = format!("{}/api/contracts/{}/execute", api_url, proposal_id);

    println!("\n{}", "Executing deployment proposal...".bold().cyan());
    println!("  Proposal: {}", proposal_id.bright_black());

    let response = client
        .post(&url)
        .send_with_retry()
        .await
        .context("Failed to execute proposal")?;

    let status = response.status();
    let body: serde_json::Value = response.json().await?;

    if !status.is_success() {
        let err = body["message"].as_str().unwrap_or("unknown error");
        anyhow::bail!("API error ({}): {}", status, err);
    }

    println!("{}", "✓ Deployment executed successfully!".green().bold());
    println!(
        "  {}: {}",
        "Contract".bold(),
        body["contract_id"].as_str().unwrap_or("?").bright_black()
    );
    println!(
        "  {}: {}",
        "WASM Hash".bold(),
        body["wasm_hash"].as_str().unwrap_or("?").bright_black()
    );
    println!(
        "  {}: {}",
        "Executed at".bold(),
        body["executed_at"].as_str().unwrap_or("?")
    );
    println!();

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Get proposal info
// ─────────────────────────────────────────────────────────────────────────────

pub async fn proposal_info(api_url: &str, proposal_id: &str) -> Result<()> {
    let client = crate::net::client();
    let url = format!("{}/api/contracts/{}/proposal", api_url, proposal_id);

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to fetch proposal info")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        anyhow::bail!("API error: {}", err);
    }

    let data: serde_json::Value = response.json().await?;
    let proposal = &data["proposal"];
    let policy = &data["policy"];
    let signatures = data["signatures"].as_array().cloned().unwrap_or_default();
    let needed = data["signatures_needed"].as_i64().unwrap_or(0);

    println!("\n{}", "Proposal Information:".bold().cyan());
    println!("{}", "=".repeat(70).cyan());

    let status = proposal["status"].as_str().unwrap_or("?");
    let status_colored = match status {
        "approved" => status.green().bold(),
        "executed" => status.bright_green().bold(),
        "expired" | "rejected" => status.red().bold(),
        _ => status.yellow().bold(),
    };

    println!(
        "\n  {}: {}",
        "ID".bold(),
        proposal["id"].as_str().unwrap_or("?")
    );
    println!(
        "  {}: {}",
        "Contract".bold(),
        proposal["contract_id"]
            .as_str()
            .unwrap_or("?")
            .bright_black()
    );
    println!(
        "  {}: {}",
        "Contract Name".bold(),
        proposal["contract_name"].as_str().unwrap_or("?")
    );
    println!(
        "  {}: {}",
        "WASM Hash".bold(),
        proposal["wasm_hash"].as_str().unwrap_or("?").bright_black()
    );
    println!(
        "  {}: {}",
        "Network".bold(),
        proposal["network"].as_str().unwrap_or("?").bright_blue()
    );
    println!("  {}: {}", "Status".bold(), status_colored);
    println!(
        "  {}: {}",
        "Proposer".bold(),
        proposal["proposer"]
            .as_str()
            .unwrap_or("?")
            .bright_magenta()
    );
    println!(
        "  {}: {}",
        "Expires at".bold(),
        proposal["expires_at"].as_str().unwrap_or("?")
    );

    if let Some(desc) = proposal["description"].as_str() {
        if !desc.is_empty() {
            println!("  {}: {}", "Description".bold(), desc);
        }
    }

    println!(
        "\n  {} Policy: {} (threshold: {}-of-{})",
        "→".bright_black(),
        policy["name"].as_str().unwrap_or("?").bold(),
        policy["threshold"].as_i64().unwrap_or(0),
        policy["signer_addresses"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0),
    );

    println!(
        "\n  {} Signatures: {}/{} collected{}",
        "→".bright_black(),
        signatures.len(),
        policy["threshold"].as_i64().unwrap_or(0),
        if needed > 0 {
            format!(" ({} more needed)", needed.to_string().yellow())
        } else {
            String::new()
        }
    );

    for sig in &signatures {
        println!(
            "    ✓ {} at {}",
            sig["signer_address"]
                .as_str()
                .unwrap_or("?")
                .bright_magenta(),
            sig["signed_at"].as_str().unwrap_or("?")
        );
    }

    println!("\n{}", "=".repeat(70).cyan());
    println!();

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// List proposals
// ─────────────────────────────────────────────────────────────────────────────

pub async fn list_proposals(
    api_url: &str,
    status_filter: Option<&str>,
    limit: usize,
) -> Result<()> {
    let client = crate::net::client();
    let mut url = format!("{}/api/multisig/proposals?limit={}", api_url, limit);
    if let Some(s) = status_filter {
        url.push_str(&format!("&status={}", s));
    }

    let response = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to list proposals")?;

    if !response.status().is_success() {
        let err = response.text().await?;
        anyhow::bail!("API error: {}", err);
    }

    let data: serde_json::Value = response.json().await?;
    let items = data["items"].as_array().cloned().unwrap_or_default();

    println!("\n{}", "Deployment Proposals:".bold().cyan());
    println!("{}", "=".repeat(70).cyan());

    if items.is_empty() {
        println!("{}", "\n  No proposals found.\n".yellow());
        return Ok(());
    }

    for (i, p) in items.iter().enumerate() {
        let status = p["status"].as_str().unwrap_or("?");
        let status_colored = match status {
            "approved" => status.green(),
            "executed" => status.bright_green(),
            "expired" | "rejected" => status.red(),
            _ => status.yellow(),
        };

        println!(
            "\n  {}. {} [{}]",
            i + 1,
            p["contract_name"].as_str().unwrap_or("Unknown").bold(),
            status_colored
        );
        println!(
            "     ID: {} | Network: {}",
            p["id"].as_str().unwrap_or("?").bright_black(),
            p["network"].as_str().unwrap_or("?").bright_blue()
        );
        println!(
            "     Contract: {} | Expires: {}",
            p["contract_id"].as_str().unwrap_or("?").bright_black(),
            p["expires_at"].as_str().unwrap_or("?")
        );
    }

    let total = data["total"].as_i64().unwrap_or(items.len() as i64);
    println!(
        "\n{}\nShowing {} of {} proposal(s)\n",
        "=".repeat(70).cyan(),
        items.len(),
        total
    );

    Ok(())
}
