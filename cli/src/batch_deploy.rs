use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use std::path::Path;

use crate::deploy;
use crate::io_utils::compute_sha256_streaming;

#[derive(Debug, Serialize)]
pub struct NetworkDeployResult {
    pub network: String,
    pub success: bool,
    pub contract_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchDeploySummary {
    pub wasm_file: String,
    pub wasm_hash: String,
    pub total_networks: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub atomic_rollback: bool,
    pub results: Vec<NetworkDeployResult>,
}

pub fn run_batch_deploy(
    wasm_file: &str,
    networks: &str,
    signer: &str,
    atomic: bool,
    json_out: bool,
) -> Result<()> {
    let wasm_path = Path::new(wasm_file);
    anyhow::ensure!(
        wasm_path.exists() && wasm_path.is_file(),
        "WASM file not found: {}",
        wasm_file
    );
    anyhow::ensure!(
        wasm_path.extension().and_then(|e| e.to_str()) == Some("wasm"),
        "File must be a .wasm file"
    );

    let network_list: Vec<&str> = networks
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    anyhow::ensure!(!network_list.is_empty(), "No networks specified");

    for network in &network_list {
        anyhow::ensure!(
            matches!(
                network.to_lowercase().as_str(),
                "mainnet" | "testnet" | "futurenet"
            ),
            "Invalid network: {}. Must be mainnet, testnet, or futurenet",
            network
        );
    }

    let wasm_hash = compute_sha256_streaming(wasm_path)?;

    if !json_out {
        println!("\n{}", "Batch Deploy".bold().cyan());
        println!("{}", "=".repeat(80).cyan());
        println!("WASM file: {}", wasm_file.bright_black());
        println!("WASM hash: {}", wasm_hash.bright_black());
        println!("Networks: {}", network_list.join(", ").bright_blue());
        println!("Signer: {}", signer.bright_black());
        if atomic {
            println!("Mode: {} (stop on first failure)", "atomic".yellow());
        }
        println!("{}", "-".repeat(80).cyan());
    }

    let mut results = Vec::new();

    for (idx, network) in network_list.iter().enumerate() {
        if !json_out {
            print!(
                "  [{}/{}] Deploying to {} ... ",
                idx + 1,
                network_list.len(),
                network.bold()
            );
        }

        match deploy::deploy_to_network(wasm_file, network, signer) {
            Ok(contract_id) => {
                results.push(NetworkDeployResult {
                    network: network.to_string(),
                    success: true,
                    contract_id: Some(contract_id),
                    error: None,
                });
                if !json_out {
                    println!("{}", "✓".green());
                }
            }
            Err(error) => {
                let error_msg = error.to_string();
                results.push(NetworkDeployResult {
                    network: network.to_string(),
                    success: false,
                    contract_id: None,
                    error: Some(error_msg.clone()),
                });
                if !json_out {
                    println!("{} — {}", "✗".red(), error_msg.red());
                }
                if atomic {
                    let summary = BatchDeploySummary {
                        wasm_file: wasm_file.to_string(),
                        wasm_hash,
                        total_networks: network_list.len(),
                        succeeded: results.iter().filter(|r| r.success).count(),
                        failed: results.iter().filter(|r| !r.success).count(),
                        atomic_rollback: true,
                        results,
                    };
                    emit_summary(&summary, json_out)?;
                    anyhow::bail!("Deployment stopped due to --atomic flag");
                }
            }
        }
    }

    let summary = BatchDeploySummary {
        wasm_file: wasm_file.to_string(),
        wasm_hash,
        total_networks: network_list.len(),
        succeeded: results.iter().filter(|r| r.success).count(),
        failed: results.iter().filter(|r| !r.success).count(),
        atomic_rollback: false,
        results,
    };

    emit_summary(&summary, json_out)?;

    Ok(())
}

fn emit_summary(summary: &BatchDeploySummary, json_out: bool) -> Result<()> {
    if json_out {
        println!("{}", serde_json::to_string_pretty(summary)?);
        return Ok(());
    }

    println!("\n{}", "Deployment Summary".bold().cyan());
    println!(
        "Networks: {}/{} succeeded, {} failed",
        summary.succeeded, summary.total_networks, summary.failed
    );
    if summary.atomic_rollback {
        println!(
            "Status: {} (atomic rollback triggered)",
            "FAILED".red().bold()
        );
    } else if summary.failed == 0 {
        println!("Status: {}", "SUCCESS".green().bold());
    } else {
        println!("Status: {}", "PARTIAL".yellow().bold());
    }
    println!("{}", "-".repeat(80).cyan());

    for result in &summary.results {
        if result.success {
            println!(
                "{} {} → {}",
                "✓".green(),
                result.network.bold(),
                result.contract_id.as_ref().unwrap().bright_black()
            );
        } else {
            println!(
                "{} {} — {}",
                "✗".red(),
                result.network.bold(),
                result.error.as_ref().unwrap().red()
            );
        }
    }

    Ok(())
}
