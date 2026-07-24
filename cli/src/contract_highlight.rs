//! contract_highlight.rs — `soroban-registry contract highlight [ADDRESS]` (#832)
//!
//! Manage featured/highlighted contracts in the registry. Supports the
//! actions add | remove | list | check against `/api/contracts/highlights`.
//! Mutating actions require curator authentication (sent as a bearer token).

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::{json, Value};

fn highlights_url(api_url: &str) -> String {
    format!("{}/api/contracts/highlights", api_url.trim_end_matches('/'))
}

/// `soroban-registry contract highlight [ADDRESS] --action <add|remove|list|check> [--token <t>] [--json]`
pub async fn run(
    api_url: &str,
    address: Option<&str>,
    action: &str,
    token: Option<&str>,
    json: bool,
) -> Result<()> {
    let client = crate::net::client();
    let base = highlights_url(api_url);

    let resp = match action {
        "list" => client.get(&base).send_with_retry().await,
        "check" => {
            let addr = address.context("`check` requires a contract ADDRESS")?;
            client
                .get(format!("{}/{}", base, addr))
                .send_with_retry()
                .await
        }
        "add" => {
            let addr = address.context("`add` requires a contract ADDRESS")?;
            let mut req = client.post(&base).json(&json!({ "address": addr }));
            if let Some(t) = token {
                req = req.bearer_auth(t);
            }
            req.send_with_retry().await
        }
        "remove" => {
            let addr = address.context("`remove` requires a contract ADDRESS")?;
            let mut req = client.delete(format!("{}/{}", base, addr));
            if let Some(t) = token {
                req = req.bearer_auth(t);
            }
            req.send_with_retry().await
        }
        other => anyhow::bail!(
            "unknown highlight action '{}' (use add|remove|list|check)",
            other
        ),
    }
    .context("Failed to reach the registry API. Is the registry running?")?;

    let status = resp.status();
    let value: Value = resp.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        anyhow::bail!(
            "contract highlight {} failed ({}): {}",
            action,
            status,
            value
        );
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    match action {
        "list" => {
            let items = value
                .get("highlights")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if items.is_empty() {
                println!("{}", "No highlighted contracts.".dimmed());
            } else {
                println!("{}", "Highlighted contracts:".bold());
                for it in items {
                    let addr = it.get("address").and_then(Value::as_str).unwrap_or("?");
                    let since = it
                        .get("highlightedAt")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    println!("  {}  {}", addr.cyan(), since.dimmed());
                }
            }
        }
        "check" => {
            let on = value
                .get("highlighted")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            println!(
                "{} {}",
                address.unwrap_or(""),
                if on {
                    "is highlighted".green()
                } else {
                    "is not highlighted".dimmed()
                }
            );
        }
        "add" => println!(
            "{} highlighted {}",
            "✓".green().bold(),
            address.unwrap_or("")
        ),
        "remove" => println!(
            "{} removed highlight for {}",
            "✓".green().bold(),
            address.unwrap_or("")
        ),
        _ => {}
    }
    Ok(())
}
