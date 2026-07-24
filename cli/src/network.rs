#![allow(dead_code)]

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const CACHE_FILE: &str = "network-cache.json";
const REQUEST_TIMEOUT_SECS: u64 = 10;
/// Ledger is considered stale if closed more than this many seconds ago.
const STALE_LEDGER_SECS: i64 = 300;

struct NetworkDef {
    name: &'static str,
    network_type: &'static str,
    rpc_endpoint: &'static str,
    horizon_endpoint: &'static str,
}

const NETWORKS: &[NetworkDef] = &[
    NetworkDef {
        name: "mainnet",
        network_type: "mainnet",
        rpc_endpoint: "https://rpc-mainnet.stellar.org",
        horizon_endpoint: "https://horizon.stellar.org",
    },
    NetworkDef {
        name: "testnet",
        network_type: "testnet",
        rpc_endpoint: "https://soroban-testnet.stellar.org",
        horizon_endpoint: "https://horizon-testnet.stellar.org",
    },
    NetworkDef {
        name: "futurenet",
        network_type: "futurenet",
        rpc_endpoint: "https://rpc-futurenet.stellar.org",
        horizon_endpoint: "https://horizon-futurenet.stellar.org",
    },
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkStatus {
    Up,
    Down,
    Degraded,
}

impl std::fmt::Display for NetworkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkStatus::Up => write!(f, "up"),
            NetworkStatus::Down => write!(f, "down"),
            NetworkStatus::Degraded => write!(f, "degraded"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub name: String,
    pub network_type: String,
    pub endpoint: String,
    pub status: NetworkStatus,
    pub last_ledger: Option<u64>,
    pub last_ledger_close_time: Option<DateTime<Utc>>,
    pub checked_at: DateTime<Utc>,
    pub from_cache: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct NetworkCache {
    networks: Vec<NetworkInfo>,
    cached_at: DateTime<Utc>,
}

// Stellar RPC JSON-RPC types
#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,
    id: u32,
    method: &'a str,
}

#[derive(Deserialize)]
struct RpcResponse {
    result: Option<LatestLedgerResult>,
}

#[derive(Deserialize)]
struct LatestLedgerResult {
    sequence: Option<u64>,
}

// Horizon root endpoint type
#[derive(Deserialize)]
struct HorizonRoot {
    history_latest_ledger: Option<u64>,
    #[serde(default)]
    history_latest_ledger_close_time: Option<serde_json::Value>,
}

fn cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".soroban-registry").join(CACHE_FILE))
}

fn load_cache() -> Option<NetworkCache> {
    let path = cache_path()?;
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_cache(networks: &[NetworkInfo]) {
    let Some(path) = cache_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let cache = NetworkCache {
        networks: networks.to_vec(),
        cached_at: Utc::now(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&cache) {
        let _ = fs::write(&path, json);
    }
}

async fn check_network(def: &NetworkDef, client: &reqwest::Client) -> NetworkInfo {
    let rpc_req = RpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method: "getLatestLedger",
    };

    let now = Utc::now();

    // Query the RPC endpoint for latest ledger sequence
    let rpc_result = client
        .post(def.rpc_endpoint)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .json(&rpc_req)
        .send_with_retry()
        .await;

    let (sequence, close_time) = match rpc_result {
        Ok(resp) if resp.status().is_success() => {
            let rpc: RpcResponse = resp.json().await.unwrap_or(RpcResponse { result: None });
            let seq = rpc.result.and_then(|r| r.sequence);

            // If we got a sequence, try to fetch the close time from Horizon
            let ct = if seq.is_some() {
                fetch_close_time(def, client).await
            } else {
                None
            };
            (seq, ct)
        }
        _ => (None, None),
    };

    let status = match (sequence, close_time) {
        (None, _) => NetworkStatus::Down,
        (Some(_), Some(ct)) => {
            let age_secs = (now - ct).num_seconds();
            if age_secs > STALE_LEDGER_SECS {
                NetworkStatus::Degraded
            } else {
                NetworkStatus::Up
            }
        }
        (Some(_), None) => NetworkStatus::Up,
    };

    NetworkInfo {
        name: def.name.to_string(),
        network_type: def.network_type.to_string(),
        endpoint: def.rpc_endpoint.to_string(),
        status,
        last_ledger: sequence,
        last_ledger_close_time: close_time,
        checked_at: now,
        from_cache: false,
    }
}

async fn fetch_close_time(def: &NetworkDef, client: &reqwest::Client) -> Option<DateTime<Utc>> {
    let resp = client
        .get(def.horizon_endpoint)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .send_with_retry()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let root: HorizonRoot = resp.json().await.ok()?;
    let ts_val = root.history_latest_ledger_close_time?;

    // The value may be a unix timestamp (number or string)
    if let Some(ts) = ts_val.as_i64() {
        return Utc.timestamp_opt(ts, 0).single();
    }
    if let Some(s) = ts_val.as_str() {
        // Try RFC3339 first, then unix timestamp string
        if let Ok(dt) = s.parse::<DateTime<Utc>>() {
            return Some(dt);
        }
        if let Ok(n) = s.parse::<i64>() {
            return Utc.timestamp_opt(n, 0).single();
        }
    }
    None
}

pub async fn status(json: bool) -> Result<()> {
    let client = crate::net::client();

    // Check all three networks concurrently
    let (mainnet, testnet, futurenet) = tokio::join!(
        check_network(&NETWORKS[0], &client),
        check_network(&NETWORKS[1], &client),
        check_network(&NETWORKS[2], &client),
    );

    let all_down = matches!(mainnet.status, NetworkStatus::Down)
        && matches!(testnet.status, NetworkStatus::Down)
        && matches!(futurenet.status, NetworkStatus::Down);

    let mut networks: Vec<NetworkInfo> = if all_down {
        // Serve from cache when offline
        if let Some(cache) = load_cache() {
            eprintln!(
                "{}",
                "Network unreachable — showing cached data from "
                    .yellow()
                    .to_string()
                    + &cache.cached_at.format("%Y-%m-%d %H:%M:%S UTC").to_string()
            );
            cache
                .networks
                .into_iter()
                .map(|mut n| {
                    n.from_cache = true;
                    n
                })
                .collect()
        } else {
            vec![mainnet, testnet, futurenet]
        }
    } else {
        let result = vec![mainnet, testnet, futurenet];
        save_cache(&result);
        result
    };

    if json {
        print_json(&networks)?;
    } else {
        print_table(&networks);
    }

    Ok(())
}

fn print_json(networks: &[NetworkInfo]) -> Result<()> {
    let output: Vec<serde_json::Value> = networks
        .iter()
        .map(|n| {
            serde_json::json!({
                "name": n.name,
                "network_type": n.network_type,
                "endpoint": n.endpoint,
                "status": n.status.to_string(),
                "last_ledger": n.last_ledger,
                "last_ledger_close_time": n.last_ledger_close_time.map(|t| t.to_rfc3339()),
                "checked_at": n.checked_at.to_rfc3339(),
                "from_cache": n.from_cache,
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "networks": output }))
            .context("Failed to serialize network status")?
    );
    Ok(())
}

fn print_table(networks: &[NetworkInfo]) {
    println!("\n{}", "Network Status".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    for n in networks {
        let status_str = match n.status {
            NetworkStatus::Up => "UP".bold().green().to_string(),
            NetworkStatus::Down => "DOWN".bold().red().to_string(),
            NetworkStatus::Degraded => "DEGRADED".bold().yellow().to_string(),
        };

        let cache_badge = if n.from_cache {
            " (cached)".dimmed().to_string()
        } else {
            String::new()
        };

        println!(
            "\n  {} [{}]  type: {}{}",
            n.name.bold(),
            status_str,
            n.network_type,
            cache_badge
        );
        println!("  {}: {}", "Endpoint".bold(), n.endpoint);

        if let Some(ledger) = n.last_ledger {
            println!("  {}: {}", "Last ledger".bold(), ledger);
        }

        if let Some(ct) = n.last_ledger_close_time {
            println!(
                "  {}: {}",
                "Last block time".bold(),
                ct.format("%Y-%m-%d %H:%M:%S UTC")
            );
        }

        println!(
            "  {}: {}",
            "Checked at".bold(),
            n.checked_at.format("%Y-%m-%d %H:%M:%S UTC")
        );
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!();
}
