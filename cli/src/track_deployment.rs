//! track_deployment.rs — `soroban-registry track-deployment` (#524)
//!
//! Polls the registry API and the Stellar network to track contract deployment
//! progress and confirm when a deployment is live on-chain.

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::process;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Exit code returned when the wait timeout expires before deployment is confirmed.
pub const EXIT_TIMEOUT: i32 = 2;

const POLL_INTERVAL_SECS: u64 = 5;

struct NetworkDef {
    name: &'static str,
    rpc_endpoint: &'static str,
    horizon_endpoint: &'static str,
}

const NETWORKS: &[NetworkDef] = &[
    NetworkDef {
        name: "mainnet",
        rpc_endpoint: "https://rpc-mainnet.stellar.org",
        horizon_endpoint: "https://horizon.stellar.org",
    },
    NetworkDef {
        name: "testnet",
        rpc_endpoint: "https://soroban-testnet.stellar.org",
        horizon_endpoint: "https://horizon-testnet.stellar.org",
    },
    NetworkDef {
        name: "futurenet",
        rpc_endpoint: "https://rpc-futurenet.stellar.org",
        horizon_endpoint: "https://horizon-futurenet.stellar.org",
    },
];

fn resolve_network(network: &str) -> Option<&'static NetworkDef> {
    NETWORKS
        .iter()
        .find(|n| n.name.eq_ignore_ascii_case(network))
}

// ── Response shapes ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentStatus {
    pub contract_id: String,
    pub network: String,
    pub status: String, // "pending" | "confirmed" | "timeout" | "not_found"
    pub tx_hash: Option<String>,
    pub ledger: Option<u64>,
    pub ledger_close_time: Option<String>,
    pub block_time_secs: Option<u64>,
    pub registered_in_registry: bool,
}

/// Horizon transaction response (only fields we need)
#[derive(Debug, Deserialize)]
struct HorizonTransaction {
    hash: String,
    ledger: u64,
    created_at: String,
    successful: bool,
}

/// Stellar RPC `getTransaction` result
#[derive(Debug, Deserialize)]
struct RpcTransactionResult {
    status: String, // "SUCCESS" | "FAILED" | "NOT_FOUND"
    #[serde(rename = "ledger")]
    ledger: Option<u64>,
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RpcResponse {
    result: Option<RpcTransactionResult>,
    #[allow(dead_code)]
    error: Option<serde_json::Value>,
}

// ── Main entry point ─────────────────────────────────────────────────────────

/// `soroban-registry track-deployment --contract-id <id> --network <network> [options]`
pub async fn run(
    api_url: &str,
    contract_id: &str,
    network: &str,
    tx_hash: Option<&str>,
    wait_timeout: u64,
    json: bool,
) -> Result<()> {
    log::debug!(
        "track-deployment | contract_id={} network={} tx_hash={:?} timeout={}",
        contract_id,
        network,
        tx_hash,
        wait_timeout
    );

    let net_def = resolve_network(network).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown network '{}'. Valid options: mainnet, testnet, futurenet",
            network
        )
    })?;

    if !json {
        print_header();
        println!("  {}   {}", "Contract:".bold(), contract_id.bright_black());
        println!("  {}   {}", "Network:".bold(), network.bright_blue());
        if let Some(hash) = tx_hash {
            println!("  {} {}", "Tx Hash:".bold(), hash.bright_black());
        }
        println!("  {}  {}s", "Timeout:".bold(), wait_timeout);
        println!();
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("Failed to build HTTP client")?;

    let deadline = Instant::now() + Duration::from_secs(wait_timeout);
    let mut attempt: u32 = 0;

    loop {
        attempt += 1;
        let elapsed =
            Instant::now().saturating_duration_since(deadline - Duration::from_secs(wait_timeout));

        log::debug!("Poll attempt {} (elapsed: {}s)", attempt, elapsed.as_secs());

        // ── Check 1: registry API ────────────────────────────────────────────
        let registry_result = poll_registry(&client, api_url, contract_id, network).await;

        // ── Check 2: on-chain via Horizon or RPC ────────────────────────────
        let onchain_result = if let Some(hash) = tx_hash {
            let horizon = poll_horizon_tx(&client, net_def.horizon_endpoint, hash).await;
            if horizon.is_some() {
                horizon
            } else {
                poll_rpc_tx(&client, net_def.rpc_endpoint, hash).await
            }
        } else {
            poll_horizon_contract(&client, net_def.horizon_endpoint, contract_id).await
        };

        let confirmed = registry_result.is_some() || onchain_result.is_some();

        if !json {
            let dots = ".".repeat(((attempt - 1) % 3 + 1) as usize);
            print!("\r  {} Polling{:<3}  attempt {}", "⟳".cyan(), dots, attempt);
            // Flush without newline so the line updates
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }

        if confirmed {
            let (tx, ledger, close_time) =
                merge_results(registry_result.as_ref(), onchain_result.as_ref(), tx_hash);

            let status = DeploymentStatus {
                contract_id: contract_id.to_string(),
                network: network.to_string(),
                status: "confirmed".to_string(),
                tx_hash: tx.clone(),
                ledger,
                ledger_close_time: close_time.clone(),
                block_time_secs: None,
                registered_in_registry: registry_result.is_some(),
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                println!(); // end the polling line
                println!();
                print_success(&status);
            }

            return Ok(());
        }

        if Instant::now() >= deadline {
            let status = DeploymentStatus {
                contract_id: contract_id.to_string(),
                network: network.to_string(),
                status: "timeout".to_string(),
                tx_hash: tx_hash.map(str::to_string),
                ledger: None,
                ledger_close_time: None,
                block_time_secs: None,
                registered_in_registry: false,
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&status)?);
            } else {
                println!(); // end the polling line
                println!();
                print_timeout(contract_id, wait_timeout);
            }

            process::exit(EXIT_TIMEOUT);
        }

        sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}

// ── Polling helpers ───────────────────────────────────────────────────────────

/// Check the registry API for the contract. Returns raw JSON value on success.
async fn poll_registry(
    client: &reqwest::Client,
    api_url: &str,
    contract_id: &str,
    network: &str,
) -> Option<serde_json::Value> {
    let url = format!(
        "{}/api/contracts?contract_id={}&network={}",
        api_url, contract_id, network
    );
    log::debug!("GET {}", url);

    let res = client.get(&url).send_with_retry().await.ok()?;
    if !res.status().is_success() {
        return None;
    }

    let body: serde_json::Value = res.json().await.ok()?;

    // Accept either a paginated list with items or a direct object
    if let Some(items) = body["items"].as_array() {
        items
            .iter()
            .find(|c| {
                c["contract_id"].as_str() == Some(contract_id)
                    || c["id"].as_str() == Some(contract_id)
            })
            .cloned()
    } else if body.is_object() && (body["contract_id"].is_string() || body["id"].is_string()) {
        Some(body)
    } else {
        None
    }
}

/// Look up a transaction on Horizon by hash. Returns the transaction JSON on success.
async fn poll_horizon_tx(
    client: &reqwest::Client,
    horizon_url: &str,
    tx_hash: &str,
) -> Option<HorizonTransaction> {
    let url = format!("{}/transactions/{}", horizon_url, tx_hash);
    log::debug!("GET {}", url);

    let res = client.get(&url).send_with_retry().await.ok()?;
    if !res.status().is_success() {
        return None;
    }

    let tx: HorizonTransaction = res.json().await.ok()?;
    if tx.successful {
        Some(tx)
    } else {
        None
    }
}

/// Check Horizon for the contract account (Soroban contracts show up as contract accounts).
async fn poll_horizon_contract(
    client: &reqwest::Client,
    horizon_url: &str,
    contract_id: &str,
) -> Option<HorizonTransaction> {
    // Stellar contracts appear in the operations endpoint; a lightweight check
    // is to see if any successful `create_contract` or `invoke_host_function`
    // operation with this contract ID exists. For a simpler approach, we look
    // for the contract as an account-like entity.
    let url = format!(
        "{}/operations?contract_id={}&limit=1&order=asc",
        horizon_url, contract_id
    );
    log::debug!("GET {}", url);

    let res = client.get(&url).send_with_retry().await.ok()?;
    if !res.status().is_success() {
        return None;
    }

    let body: serde_json::Value = res.json().await.ok()?;
    let records = body["_embedded"]["records"].as_array()?;

    if records.is_empty() {
        return None;
    }

    // We confirmed the contract exists on-chain, synthesize a minimal result
    let op = &records[0];
    let tx_hash = op["transaction_hash"].as_str()?;
    let ledger = op["paging_token"]
        .as_str()
        .and_then(|t| t.split('-').next())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let created_at = op["created_at"].as_str().unwrap_or("").to_string();

    Some(HorizonTransaction {
        hash: tx_hash.to_string(),
        ledger,
        created_at,
        successful: true,
    })
}

/// Try to get a transaction via the Stellar JSON-RPC endpoint.
async fn poll_rpc_tx(
    client: &reqwest::Client,
    rpc_url: &str,
    tx_hash: &str,
) -> Option<HorizonTransaction> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": { "hash": tx_hash }
    });

    log::debug!("POST {} getTransaction hash={}", rpc_url, tx_hash);

    let res = client
        .post(rpc_url)
        .json(&body)
        .send_with_retry()
        .await
        .ok()?;
    if !res.status().is_success() {
        return None;
    }

    let rpc: RpcResponse = res.json().await.ok()?;
    let result = rpc.result?;

    if result.status != "SUCCESS" {
        return None;
    }

    Some(HorizonTransaction {
        hash: tx_hash.to_string(),
        ledger: result.ledger.unwrap_or(0),
        created_at: result.created_at.unwrap_or_default(),
        successful: true,
    })
}

// ── Result merging ────────────────────────────────────────────────────────────

/// Merge registry + on-chain results into (tx_hash, ledger, close_time).
fn merge_results(
    registry: Option<&serde_json::Value>,
    onchain: Option<&HorizonTransaction>,
    cli_tx_hash: Option<&str>,
) -> (Option<String>, Option<u64>, Option<String>) {
    let tx = onchain
        .map(|t| t.hash.clone())
        .or_else(|| cli_tx_hash.map(str::to_string))
        .or_else(|| registry.and_then(|r| r["tx_hash"].as_str().map(str::to_string)));

    let ledger = onchain
        .map(|t| t.ledger)
        .or_else(|| registry.and_then(|r| r["ledger_sequence"].as_u64()));

    let close_time = onchain.map(|t| t.created_at.clone()).or_else(|| {
        registry.and_then(|r| {
            r["deployed_at"]
                .as_str()
                .or(r["created_at"].as_str())
                .map(str::to_string)
        })
    });

    (tx, ledger, close_time)
}

// ── Display helpers ───────────────────────────────────────────────────────────

fn print_header() {
    println!();
    println!("{}", "Deployment Tracker".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
}

fn print_success(status: &DeploymentStatus) {
    println!("{}", "═".repeat(60).cyan());
    println!(
        "  {} {}",
        "✔".green().bold(),
        "Deployment confirmed!".green().bold()
    );
    println!();
    println!(
        "  {}  {}",
        "Contract:".bold(),
        status.contract_id.bright_black()
    );
    println!("  {}  {}", "Network:".bold(), status.network.bright_blue());

    if let Some(tx) = &status.tx_hash {
        println!("  {}  {}", "Tx Hash:".bold(), tx.bright_black());
    }

    if let Some(ledger) = status.ledger {
        println!("  {}   {}", "Ledger:".bold(), ledger.to_string().bold());
    }

    if let Some(t) = &status.ledger_close_time {
        println!("  {}    {}", "Time:".bold(), t.dimmed());
    }

    let reg_label = if status.registered_in_registry {
        "Yes".green().bold()
    } else {
        "Not yet indexed".yellow()
    };
    println!("  {} {}", "In registry:".bold(), reg_label);

    println!("{}", "═".repeat(60).cyan());
    println!();
}

fn print_timeout(contract_id: &str, timeout_secs: u64) {
    println!("{}", "═".repeat(60).cyan());
    println!(
        "  {} {}",
        "✘".red().bold(),
        "Deployment not confirmed within timeout.".red().bold()
    );
    println!();
    println!("  {} {}", "Contract:".bold(), contract_id.bright_black());
    println!("  {} {}s elapsed", "Timeout:".bold(), timeout_secs);
    println!();
    println!(
        "  {}: The transaction may still be pending. Re-run with a longer",
        "Hint".bold()
    );
    println!("  --wait-timeout to keep polling, or check the network manually.");
    println!("{}", "═".repeat(60).cyan());
    println!();
}
