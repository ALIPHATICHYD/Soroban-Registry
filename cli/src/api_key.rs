//! api_key.rs — `soroban-registry api-key [create|list|delete|revoke]` (#842)
//!
//! CLI-based API key management for programmatic / automation access. Keys are
//! created with optional expiry and scopes, listed, deleted, and revoked
//! against the registry's `/api/api-keys` endpoints.

use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::{json, Value};

fn base(api_url: &str) -> String {
    format!("{}/api/api-keys", api_url.trim_end_matches('/'))
}

/// `soroban-registry api-key create [--expires <when>] [--scopes a,b,c] [--json]`
pub async fn create(
    api_url: &str,
    expires: Option<&str>,
    scopes: Option<&str>,
    json: bool,
) -> Result<()> {
    let client = crate::net::client();
    let scope_list: Vec<String> = scopes
        .map(|s| {
            s.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let body = json!({ "expires": expires, "scopes": scope_list });
    let resp = client
        .post(base(api_url))
        .json(&body)
        .send_with_retry()
        .await
        .context("Failed to reach the registry API. Is the registry running?")?;

    let status = resp.status();
    let value: Value = resp.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        anyhow::bail!("api-key create failed ({}): {}", status, value);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }
    println!("{} API key created", "✓".green().bold());
    if let Some(key) = value.get("key").and_then(Value::as_str) {
        println!("  key:     {}", key.yellow());
        println!(
            "  {}",
            "store this now — it will not be shown again".dimmed()
        );
    }
    if let Some(id) = value.get("id").and_then(Value::as_str) {
        println!("  id:      {}", id);
    }
    if let Some(exp) = value.get("expiresAt").and_then(Value::as_str) {
        println!("  expires: {}", exp);
    }
    Ok(())
}

/// `soroban-registry api-key list [--json]`
pub async fn list(api_url: &str, json: bool) -> Result<()> {
    let client = crate::net::client();
    let resp = client
        .get(base(api_url))
        .send_with_retry()
        .await
        .context("Failed to reach the registry API. Is the registry running?")?;
    let status = resp.status();
    let value: Value = resp.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        anyhow::bail!("api-key list failed ({}): {}", status, value);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }
    let keys = value
        .get("keys")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if keys.is_empty() {
        println!("{}", "No API keys.".dimmed());
        return Ok(());
    }
    println!("{}", "API keys:".bold());
    for k in keys {
        let id = k.get("id").and_then(Value::as_str).unwrap_or("?");
        let scopes = k
            .get("scopes")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();
        let status = k.get("status").and_then(Value::as_str).unwrap_or("active");
        let last_used = k
            .get("lastUsedAt")
            .and_then(Value::as_str)
            .unwrap_or("never");
        println!(
            "  {}  [{}]  scopes=[{}]  last_used={}",
            id, status, scopes, last_used
        );
    }
    Ok(())
}

/// `soroban-registry api-key delete <id>` / `revoke <id>`
pub async fn delete(api_url: &str, id: &str, revoke_only: bool, json: bool) -> Result<()> {
    let client = crate::net::client();
    let url = format!("{}/{}", base(api_url), id);
    let resp = if revoke_only {
        // Revoke disables the key but keeps the audit record.
        client
            .post(format!("{}/revoke", url))
            .send_with_retry()
            .await
    } else {
        client.delete(&url).send_with_retry().await
    }
    .context("Failed to reach the registry API. Is the registry running?")?;

    let status = resp.status();
    if !status.is_success() {
        let value: Value = resp.json().await.unwrap_or(Value::Null);
        anyhow::bail!(
            "api-key {} failed ({}): {}",
            if revoke_only { "revoke" } else { "delete" },
            status,
            value
        );
    }
    if json {
        println!(
            "{}",
            json!({ "id": id, "action": if revoke_only { "revoked" } else { "deleted" } })
        );
    } else {
        println!(
            "{} API key {} {}",
            "✓".green().bold(),
            id,
            if revoke_only { "revoked" } else { "deleted" }
        );
    }
    Ok(())
}
