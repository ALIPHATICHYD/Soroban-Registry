use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use colored::Colorize;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub async fn run(
    api_url: &str,
    id: Option<String>,
    submit: bool,
    check: bool,
    history: bool,
    level: String,
    json_output: bool,
    path: &str,
    notes: Option<String>,
) -> Result<()> {
    let client = crate::net::client();

    if submit {
        submit_verification(api_url, &client, id, &level, path, notes, json_output).await?;
    } else if check {
        let contract_id = id.ok_or_else(|| {
            anyhow::anyhow!(
                "Contract ID is required for status check. Provide it as a positional argument."
            )
        })?;
        check_status(api_url, &client, &contract_id, json_output).await?;
    } else if history {
        let contract_id = id.ok_or_else(|| {
            anyhow::anyhow!(
                "Contract ID is required for history. Provide it as a positional argument."
            )
        })?;
        show_history(api_url, &client, &contract_id, &level, json_output).await?;
    } else {
        // Default to status check if no flag provided but ID is present
        if let Some(contract_id) = id {
            check_status(api_url, &client, &contract_id, json_output).await?;
        } else {
            anyhow::bail!("Specify an action: --submit, --check, or --history");
        }
    }

    Ok(())
}

async fn submit_verification(
    api_url: &str,
    client: &reqwest::Client,
    id: Option<String>,
    level: &str,
    path: &str,
    notes: Option<String>,
    json_output: bool,
) -> Result<()> {
    if !json_output {
        println!(
            "\n{} Preparing verification submission...",
            "•".cyan().bold()
        );
    }

    let project_path = Path::new(path);
    let mut contract_id = id;

    // Try to detect contract ID from project if not provided
    if contract_id.is_none() {
        // In a real scenario, we might look for a .soroban/id or Cargo.toml metadata
        // For now, we'll ask the user to provide it if we can't find it.
    }

    let contract_id = contract_id.ok_or_else(|| {
        anyhow::anyhow!("Contract ID is required for submission. Provide it as a positional argument or use --id if added.")
    })?;

    // Read source code. For Soroban, we'll try to find the main lib.rs or bundle the src.
    // Simplifying: read src/lib.rs if it exists.
    let src_path = project_path.join("src").join("lib.rs");
    let source_code = if src_path.exists() {
        fs::read_to_string(&src_path)?
    } else {
        anyhow::bail!(
            "Could not find contract source at {}. Verification requires a valid Soroban project.",
            src_path.display()
        );
    };

    // Prepare request
    let payload = json!({
        "source_code": source_code,
        "build_params": {
            "level": level,
            "optimizer": "z"
        },
        "compiler_version": "latest",
        "notes": notes
    });

    let url = format!(
        "{}/api/contracts/{}/verify",
        api_url.trim_end_matches('/'),
        contract_id
    );
    let response = client.post(&url).json(&payload).send_with_retry().await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        anyhow::bail!("Submission failed ({}): {}", status, error_text);
    }

    let result: Value = response.json().await?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!(
            "{} {}",
            "✓".green().bold(),
            "Verification submitted successfully".bold()
        );
        println!(
            "{:<20} {}",
            "Verification ID:".bold(),
            result["verification_id"].as_str().unwrap_or("N/A")
        );
        println!(
            "{:<20} {}",
            "Status:".bold(),
            result["status"].as_str().unwrap_or("pending").yellow()
        );
        println!(
            "{:<20} {}",
            "Message:".bold(),
            result["message"].as_str().unwrap_or("Queued")
        );
        println!(
            "\nUse 'soroban-registry verify {} --check' to track progress.",
            contract_id
        );
    }

    Ok(())
}

async fn check_status(
    api_url: &str,
    client: &reqwest::Client,
    contract_id: &str,
    json_output: bool,
) -> Result<()> {
    let url = format!(
        "{}/api/contracts/{}/verification-status",
        api_url.trim_end_matches('/'),
        contract_id
    );
    let response = client.get(&url).send_with_retry().await?;

    if !response.status().is_success() {
        if response.status() == 404 {
            anyhow::bail!(
                "Verification status not found for contract: {}. Has it been submitted?",
                contract_id
            );
        }
        anyhow::bail!(
            "Failed to fetch status ({}): {}",
            response.status(),
            response.text().await.unwrap_or_default()
        );
    }

    let status: Value = response.json().await?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("\n{}", "Verification Status".bold().cyan());
        println!("{}", "=".repeat(60).cyan());

        let vs = status["verification_status"].as_str().unwrap_or("unknown");
        let status_colored = match vs.to_lowercase().as_str() {
            "verified" => vs.green().bold(),
            "failed" => vs.red().bold(),
            "pending" | "processing" => vs.yellow().bold(),
            _ => vs.normal(),
        };

        println!("{:<20} {}", "Contract ID:".bold(), contract_id);
        println!("{:<20} {}", "Status:".bold(), status_colored);
        println!(
            "{:<20} {}",
            "Is Verified:".bold(),
            status["is_verified"].as_bool().unwrap_or(false)
        );

        if let Some(at) = status["verified_at"].as_str() {
            println!("{:<20} {}", "Verified At:".bold(), at);
        }

        if let Some(notes) = status["verification_notes"].as_str() {
            println!("{:<20} {}", "Notes:".bold(), notes);
        }

        if let Some(auditor) = status["auditor"].as_str() {
            println!("{:<20} {}", "Auditor/Verifier:".bold(), auditor);
        }

        println!();
    }

    Ok(())
}

async fn show_history(
    api_url: &str,
    client: &reqwest::Client,
    contract_id: &str,
    level: &str,
    json_output: bool,
) -> Result<()> {
    let url = format!(
        "{}/api/contracts/{}/verification-history?level={}",
        api_url.trim_end_matches('/'),
        contract_id,
        level
    );
    let response = client.get(&url).send_with_retry().await?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch history ({}): {}",
            response.status(),
            response.text().await.unwrap_or_default()
        );
    }

    let data: Value = response.json().await?;
    let history = data["history"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid history response"))?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&data)?);
    } else {
        println!(
            "\n{}",
            format!("Verification History for {}", contract_id)
                .bold()
                .cyan()
        );
        println!("{}", "═".repeat(80).cyan());

        if history.is_empty() {
            println!("  No verification history found.");
        } else {
            for entry in history {
                let date = entry["created_at"].as_str().unwrap_or("Unknown Date");
                let from = entry["from_status"].as_str().unwrap_or("?");
                let to = entry["to_status"].as_str().unwrap_or("?");
                let notes = entry["notes"].as_str().unwrap_or("");

                println!(
                    "[{}] {} -> {}",
                    date.dimmed(),
                    from.yellow(),
                    to.bold().green()
                );
                if !notes.is_empty() {
                    println!("    Notes: {}", notes.bright_black());
                }
                println!("{}", "-".repeat(40).bright_black());
            }
        }
        println!();
    }

    Ok(())
}
