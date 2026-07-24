//! user_profile.rs — `soroban-registry profile` (#841)
//!
//! Manages publisher profiles in the Soroban Registry.
//! Supports viewing, editing, and exporting profile data,
//! as well as listing contracts published under a profile.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

// ── API types ─────────────────────────────────────────────────────────────────

/// Mirrors the backend `Publisher` model fields returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublisherProfile {
    pub id: String,
    pub stellar_address: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub github_url: Option<String>,
    pub website: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Slim contract summary used in `list-contracts` output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractSummary {
    pub id: String,
    pub name: String,
    pub contract_id: Option<String>,
    pub network: Option<String>,
    pub is_verified: bool,
    pub health_score: Option<i64>,
    pub created_at: Option<String>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to build HTTP client")
}

/// Read the API key from the local user config (~/.soroban-registry/config.json).
fn api_key() -> Option<String> {
    crate::user_config::load().ok()?.api_key
}

/// Apply a Bearer token header when an API key is available.
fn apply_auth(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    if let Some(key) = api_key() {
        builder.bearer_auth(key)
    } else {
        builder
    }
}

/// Fetch a publisher by UUID.
async fn fetch_publisher_by_id(
    client: &reqwest::Client,
    api_url: &str,
    id: &str,
) -> Result<PublisherProfile> {
    let url = format!("{}/api/publishers/{}", api_url, id);
    log::debug!("GET {}", url);

    let res = apply_auth(client.get(&url))
        .send()
        .await
        .context("Failed to connect to registry API")?;

    let status = res.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("Publisher '{}' not found in registry.", id);
    }
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        anyhow::bail!("Registry API error ({}): {}", status, body);
    }

    res.json::<PublisherProfile>()
        .await
        .context("Failed to parse publisher response")
}

/// Look up a publisher by stellar address via the contracts search endpoint.
/// Returns the publisher ID if found.
async fn find_publisher_id_by_address(
    client: &reqwest::Client,
    api_url: &str,
    address: &str,
) -> Result<String> {
    let url = format!(
        "{}/api/contracts?publisher_address={}&limit=1",
        api_url, address
    );
    log::debug!("GET {}", url);

    let res = client
        .get(&url)
        .send()
        .await
        .context("Failed to connect to registry API")?;

    if !res.status().is_success() {
        if res.status() == reqwest::StatusCode::NOT_FOUND {
            anyhow::bail!(
                "Publisher not found for address '{}'. Verify the address is registered on the Stellar network and has published at least one contract.",
                address
            );
        }
        anyhow::bail!(
            "Could not look up publisher for address '{}'. Registry API error: HTTP {}",
            address,
            res.status()
        );
    }

    let raw: Value = res
        .json()
        .await
        .context("Failed to parse contract search response")?;

    // Paginated response: { items: [...] }
    if let Some(items) = raw["items"].as_array() {
        if let Some(first) = items.first() {
            if let Some(pid) = first["publisher_id"].as_str() {
                return Ok(pid.to_string());
            }
        }
    }

    anyhow::bail!(
        "No contracts found for address '{}'. This address may not be registered as a publisher, or it has no published contracts yet.\n\
         Register as a publisher by publishing a contract: soroban-registry publish --contract-id <ID> --name <NAME> --publisher <ADDRESS>",
        address
    )
}

/// Resolve the publisher to show: address/UUID arg → publisher profile.
/// If no address is given, falls back to the stellar_address in local config.
async fn resolve_profile(
    client: &reqwest::Client,
    api_url: &str,
    address: Option<&str>,
) -> Result<PublisherProfile> {
    let target = match address {
        Some(a) => a.to_string(),
        None => {
            anyhow::bail!(
                "No address provided and no stellar address stored in local config.\n\
                 Run: soroban-registry profile view --address <YOUR_STELLAR_ADDRESS>"
            );
        }
    };

    // If it looks like a UUID, fetch directly; otherwise look up by stellar address.
    if is_uuid(&target) {
        fetch_publisher_by_id(client, api_url, &target).await
    } else {
        let publisher_id = find_publisher_id_by_address(client, api_url, &target).await?;
        fetch_publisher_by_id(client, api_url, &publisher_id).await
    }
}

fn is_uuid(s: &str) -> bool {
    // Simple heuristic: UUIDs are 36 chars with dashes at positions 8,13,18,23
    s.len() == 36
        && s.chars().enumerate().all(|(i, c)| {
            if i == 8 || i == 13 || i == 18 || i == 23 {
                c == '-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
}

// ── Public entry points ───────────────────────────────────────────────────────

/// `soroban-registry profile view [--address <addr>] [--json]`
pub async fn view(api_url: &str, address: Option<&str>, json: bool) -> Result<()> {
    let client = build_client()?;
    let profile = resolve_profile(&client, api_url, address).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&profile)?);
    } else {
        print_profile_human(&profile);
    }

    Ok(())
}

/// `soroban-registry profile edit [--name X] [--bio X] [--website X] ...`
pub async fn edit(
    api_url: &str,
    name: Option<&str>,
    bio: Option<&str>,
    website: Option<&str>,
    email: Option<&str>,
    github: Option<&str>,
    avatar: Option<&str>,
) -> Result<()> {
    if [name, bio, website, email, github, avatar]
        .iter()
        .all(|v| v.is_none())
    {
        anyhow::bail!(
            "No fields specified. Use one or more flags:\n\
             --name, --bio, --website, --email, --github, --avatar"
        );
    }

    let client = build_client()?;

    // Build the update payload with only the provided fields.
    let mut payload = serde_json::Map::new();
    if let Some(v) = name {
        payload.insert("username".to_string(), json!(v));
    }
    if let Some(v) = bio {
        payload.insert("bio".to_string(), json!(v));
    }
    if let Some(v) = website {
        payload.insert("website".to_string(), json!(v));
    }
    if let Some(v) = email {
        payload.insert("email".to_string(), json!(v));
    }
    if let Some(v) = github {
        payload.insert("github_url".to_string(), json!(v));
    }
    if let Some(v) = avatar {
        payload.insert("avatar_url".to_string(), json!(v));
    }

    let key = api_key().context(
        "API key required to edit profile.\n\
         Set it with: soroban-registry config set api-key <YOUR_KEY>",
    )?;

    // The registry uses POST /api/publishers to upsert via stellar address.
    // We require the stellar address to be stored or provided.
    let stellar_address = std::env::var("SOROBAN_STELLAR_ADDRESS").ok().context(
        "SOROBAN_STELLAR_ADDRESS environment variable is required to identify your profile.\n\
         Export it before running: export SOROBAN_STELLAR_ADDRESS=<YOUR_ADDRESS>",
    )?;

    payload.insert("stellar_address".to_string(), json!(stellar_address));

    let url = format!("{}/api/publishers", api_url);
    log::debug!("POST {}", url);

    let res = client
        .post(&url)
        .bearer_auth(&key)
        .json(&Value::Object(payload))
        .send()
        .await
        .context("Failed to connect to registry API")?;

    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        anyhow::bail!("Registry API error ({}): {}", status, body);
    }

    let updated: PublisherProfile = res
        .json()
        .await
        .context("Failed to parse updated publisher response")?;

    println!();
    println!("{}", "Profile Updated".bold().cyan());
    println!("{}", "═".repeat(50).cyan());
    print_profile_human(&updated);

    Ok(())
}

/// `soroban-registry profile update --field <key> --value <val>`
pub async fn update_field(api_url: &str, field: &str, value: &str) -> Result<()> {
    let (name, bio, website, email, github, avatar) = match field {
        "name" => (Some(value), None, None, None, None, None),
        "bio" => (None, Some(value), None, None, None, None),
        "website" => (None, None, Some(value), None, None, None),
        "email" => (None, None, None, Some(value), None, None),
        "github" => (None, None, None, None, Some(value), None),
        "avatar" => (None, None, None, None, None, Some(value)),
        _ => anyhow::bail!(
            "Unknown profile field '{}'. Valid fields: name | bio | website | email | github | avatar",
            field
        ),
    };

    edit(api_url, name, bio, website, email, github, avatar).await
}

/// `soroban-registry profile list-contracts [--address <addr>] [--limit N] [--format fmt]`
pub async fn list_contracts(
    api_url: &str,
    address: Option<&str>,
    limit: usize,
    format: &str,
    json: bool,
) -> Result<()> {
    let client = build_client()?;

    let publisher_id = if let Some(addr) = address {
        if is_uuid(addr) {
            addr.to_string()
        } else {
            find_publisher_id_by_address(&client, api_url, addr).await?
        }
    } else {
        anyhow::bail!(
            "No address provided.\n\
             Run: soroban-registry profile list-contracts --address <STELLAR_ADDRESS>"
        )
    };

    let url = format!(
        "{}/api/publishers/{}/contracts?limit={}",
        api_url, publisher_id, limit
    );
    log::debug!("GET {}", url);

    let res = apply_auth(client.get(&url))
        .send()
        .await
        .context("Failed to connect to registry API")?;

    let status = res.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("Publisher not found for the given address '{}'. Verify the address is registered on the Stellar network.", address.unwrap_or("unknown"));
    }
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        anyhow::bail!("Registry API error ({}): {}", status, body);
    }

    let raw: Value = res
        .json()
        .await
        .context("Failed to parse contracts response")?;

    let items = raw["items"]
        .as_array()
        .or_else(|| raw.as_array())
        .cloned()
        .unwrap_or_default();

    let contracts: Vec<ContractSummary> = items
        .iter()
        .map(|c| ContractSummary {
            id: c["id"].as_str().unwrap_or("").to_string(),
            name: c["name"].as_str().unwrap_or("(unnamed)").to_string(),
            contract_id: c["contract_id"].as_str().map(str::to_string),
            network: c["network"].as_str().map(str::to_string),
            is_verified: c["is_verified"].as_bool().unwrap_or(false),
            health_score: c["health_score"].as_i64(),
            created_at: c["created_at"].as_str().map(str::to_string),
        })
        .collect();

    if json || format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "publisher_id": publisher_id,
                "contracts": contracts,
                "count": contracts.len()
            }))?
        );
        return Ok(());
    }

    if format == "csv" {
        print_contracts_csv(&contracts);
        return Ok(());
    }

    print_contracts_table(&contracts, &publisher_id);
    Ok(())
}

/// `soroban-registry profile export [--address <addr>] [--format json|csv]`
pub async fn export(api_url: &str, address: Option<&str>, format: &str) -> Result<()> {
    let client = build_client()?;
    let profile = resolve_profile(&client, api_url, address).await?;

    // Also fetch contracts for a complete export.
    let contracts_url = format!(
        "{}/api/publishers/{}/contracts?limit=100",
        api_url, profile.id
    );
    log::debug!("GET {}", contracts_url);

    let contracts: Vec<Value> = match apply_auth(client.get(&contracts_url))
        .send()
        .await
        .ok()
        .filter(|r| r.status().is_success())
    {
        Some(res) => res
            .json::<Value>()
            .await
            .ok()
            .and_then(|v| v["items"].as_array().or_else(|| v.as_array()).cloned())
            .unwrap_or_default(),
        None => vec![],
    };

    match format {
        "csv" => {
            // Profile fields as CSV header + row
            println!(
                "id,stellar_address,username,email,github_url,website,bio,avatar_url,created_at"
            );
            println!(
                "{},{},{},{},{},{},{},{},{}",
                profile.id,
                profile.stellar_address,
                profile.username.as_deref().unwrap_or(""),
                profile.email.as_deref().unwrap_or(""),
                profile.github_url.as_deref().unwrap_or(""),
                profile.website.as_deref().unwrap_or(""),
                profile.bio.as_deref().unwrap_or(""),
                profile.avatar_url.as_deref().unwrap_or(""),
                profile.created_at.as_deref().unwrap_or(""),
            );
            println!();
            println!("# Contracts");
            println!("id,name,contract_id,network,is_verified,health_score,created_at");
            for c in &contracts {
                println!(
                    "{},{},{},{},{},{},{}",
                    c["id"].as_str().unwrap_or(""),
                    c["name"].as_str().unwrap_or(""),
                    c["contract_id"].as_str().unwrap_or(""),
                    c["network"].as_str().unwrap_or(""),
                    c["is_verified"].as_bool().unwrap_or(false),
                    c["health_score"].as_i64().unwrap_or(0),
                    c["created_at"].as_str().unwrap_or(""),
                );
            }
        }
        _ => {
            // JSON
            let out = json!({
                "profile": profile,
                "contracts": contracts,
                "exported_at": chrono::Utc::now().to_rfc3339(),
            });
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
    }

    Ok(())
}

// ── Formatting ────────────────────────────────────────────────────────────────

fn print_profile_human(p: &PublisherProfile) {
    println!();
    println!("{}", "Publisher Profile".bold().cyan());
    println!("{}", "═".repeat(60).cyan());

    if let Some(name) = &p.username {
        println!("  {}  {}", "Name:".bold(), name.bold());
    }
    println!(
        "  {}  {}",
        "Address:".bold(),
        p.stellar_address.bright_black()
    );
    println!("  {}      {}", "ID:".bold(), p.id.dimmed());

    if let Some(email) = &p.email {
        println!("  {}   {}", "Email:".bold(), email);
    }
    if let Some(website) = &p.website {
        println!("  {} {}", "Website:".bold(), website.bright_blue());
    }
    if let Some(github) = &p.github_url {
        println!("  {}  {}", "GitHub:".bold(), github.bright_blue());
    }
    if let Some(bio) = &p.bio {
        println!("  {}     {}", "Bio:".bold(), bio.dimmed());
    }
    if let Some(avatar) = &p.avatar_url {
        println!("  {}  {}", "Avatar:".bold(), avatar.dimmed());
    }

    println!();

    if let Some(created) = &p.created_at {
        println!("  {} {}", "Registered:".bold(), created.dimmed());
    }

    println!("{}", "═".repeat(60).cyan());
    println!();
}

fn print_contracts_table(contracts: &[ContractSummary], publisher_id: &str) {
    println!();
    println!(
        "{}  {}",
        "Contracts for publisher".bold().cyan(),
        publisher_id.bright_black()
    );
    println!("{}", "═".repeat(90).cyan());

    if contracts.is_empty() {
        println!("  {}", "No contracts found.".dimmed());
    } else {
        println!(
            "  {:<44} {:<24} {:<10} {:<10} {:<6}",
            "Name".bold(),
            "Network".bold(),
            "Verified".bold(),
            "Health".bold(),
            "Created".bold()
        );
        println!("  {}", "─".repeat(86).dimmed());

        for c in contracts {
            let verified_label = if c.is_verified {
                "yes".green().bold()
            } else {
                "no".bright_black()
            };
            let health_label = match c.health_score {
                Some(h) if h >= 80 => h.to_string().green(),
                Some(h) if h >= 50 => h.to_string().yellow(),
                Some(h) => h.to_string().red(),
                None => "—".dimmed(),
            };
            let created = c
                .created_at
                .as_deref()
                .and_then(|s| s.get(..10))
                .unwrap_or("—");
            let network = c.network.as_deref().unwrap_or("—");

            println!(
                "  {:<44} {:<24} {:<10} {:<10} {:<6}",
                c.name.bold(),
                network,
                verified_label,
                health_label,
                created,
            );
        }
    }

    println!("{}", "═".repeat(90).cyan());
    println!();
}

fn print_contracts_csv(contracts: &[ContractSummary]) {
    println!("id,name,contract_id,network,is_verified,health_score,created_at");
    for c in contracts {
        println!(
            "{},{},{},{},{},{},{}",
            c.id,
            c.name,
            c.contract_id.as_deref().unwrap_or(""),
            c.network.as_deref().unwrap_or(""),
            c.is_verified,
            c.health_score.map(|h| h.to_string()).unwrap_or_default(),
            c.created_at.as_deref().unwrap_or(""),
        );
    }
}
