//! contract_interaction.rs — `soroban-registry contract interaction <ADDRESS>` (#835)
//!
//! View and analyze a contract's interactions: recent calls with timestamps,
//! caller frequency, function-call distribution, and success/error rates.

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;

/// `soroban-registry contract interaction <ADDRESS> [--limit N] [--json]`
pub async fn run(api_url: &str, address: &str, limit: u32, json: bool) -> Result<()> {
    let client = crate::net::client();
    let url = format!(
        "{}/api/contracts/{}/interactions?limit={}",
        api_url.trim_end_matches('/'),
        address,
        limit
    );
    log::debug!("GET {}", url);

    let resp = client
        .get(&url)
        .send_with_retry()
        .await
        .context("Failed to reach the registry API. Is the registry running?")?;
    let status = resp.status();
    let value: Value = resp.json().await.unwrap_or(Value::Null);
    if status.as_u16() == 404 {
        anyhow::bail!("no interaction data found for {}", address);
    }
    if !status.is_success() {
        anyhow::bail!("contract interaction failed ({}): {}", status, value);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    println!("{} {}", "Interactions for".bold(), address.cyan());

    if let Some(total) = value.get("totalCalls").and_then(Value::as_u64) {
        let success = value
            .get("successRate")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        println!(
            "  total calls: {}   success rate: {:.1}%",
            total,
            success * 100.0
        );
    }

    if let Some(recent) = value.get("recent").and_then(Value::as_array) {
        println!("\n  {}", "Recent:".bold());
        for it in recent.iter().take(limit as usize) {
            let func = it.get("function").and_then(Value::as_str).unwrap_or("?");
            let caller = it.get("caller").and_then(Value::as_str).unwrap_or("?");
            let when = it.get("timestamp").and_then(Value::as_str).unwrap_or("");
            let ok = it.get("success").and_then(Value::as_bool).unwrap_or(true);
            let mark = if ok { "✓".green() } else { "✗".red() };
            println!(
                "    {} {}  {}  {}",
                mark,
                func,
                caller.dimmed(),
                when.dimmed()
            );
        }
    }

    if let Some(dist) = value.get("functionDistribution").and_then(Value::as_object) {
        println!("\n  {}", "Function call distribution:".bold());
        let mut rows: Vec<(&String, u64)> = dist
            .iter()
            .map(|(k, v)| (k, v.as_u64().unwrap_or(0)))
            .collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1));
        for (func, count) in rows {
            println!("    {:>6}  {}", count, func);
        }
    }

    if let Some(callers) = value.get("topCallers").and_then(Value::as_array) {
        println!("\n  {}", "Top callers:".bold());
        for c in callers {
            let addr = c.get("caller").and_then(Value::as_str).unwrap_or("?");
            let count = c.get("count").and_then(Value::as_u64).unwrap_or(0);
            println!("    {:>6}  {}", count, addr.dimmed());
        }
    }
    Ok(())
}
