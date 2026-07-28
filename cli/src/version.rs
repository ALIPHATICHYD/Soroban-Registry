use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use colored::*;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
struct UpdateCache {
    last_checked_at: Option<String>,
    latest_version: Option<String>,
    changelog: Option<String>,
}

fn cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".soroban-registry").join("version_cache.json"))
}

fn load_cache() -> UpdateCache {
    let Some(path) = cache_path() else {
        return UpdateCache::default();
    };
    if !path.exists() {
        return UpdateCache::default();
    }
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<UpdateCache>(&raw).ok())
        .unwrap_or_default()
}

fn save_cache(cache: &UpdateCache) -> Result<()> {
    if let Some(path) = cache_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(cache)?)?;
    }
    Ok(())
}

pub async fn check_version(
    check_updates: bool,
    auto_update: bool,
    rollback: Option<String>,
) -> Result<()> {
    println!("\n{}", "Soroban Registry CLI".bold().cyan());
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    if let Some(target) = rollback {
        println!("Rollback target: {}", target.yellow());
        println!(
            "Status: {}",
            "Rollback requested (manual install required)".yellow()
        );
        println!(
            "Hint: reinstall previous release/tag via cargo install --git ... --tag <version>"
        );
        println!();
        return Ok(());
    }

    if !check_updates {
        println!("Status:  {}", "Update check skipped".yellow());
        println!();
        return Ok(());
    }

    let latest = latest_release_with_cache().await?;
    if let Some((latest_version, changelog)) = latest {
        let current = env!("CARGO_PKG_VERSION");
        let latest_semver = Version::parse(latest_version.trim_start_matches('v')).ok();
        let current_semver = Version::parse(current).ok();
        if latest_semver
            .zip(current_semver)
            .map(|(l, c)| l > c)
            .unwrap_or(false)
        {
            println!("Status:  {}", "Update available".yellow());
            println!("Latest:  {}", latest_version.bold());
            if let Some(notes) = changelog {
                println!("Changelog:\n{}", notes);
            }
            if auto_update {
                println!("Auto-update: {}", "enabled".green());
                println!("Run: cargo install --git https://github.com/ALIPHATICHYD/Soroban-Registry soroban-registry-cli");
            }
        } else {
            println!("Status:  {}", "Up to date".green());
        }
    } else {
        println!("Status:  {}", "Could not determine latest release".yellow());
    }
    println!();
    Ok(())
}

async fn latest_release_with_cache() -> Result<Option<(String, Option<String>)>> {
    let mut cache = load_cache();
    if let Some(last_checked) = cache.last_checked_at.as_deref() {
        if let Ok(last) = DateTime::parse_from_rfc3339(last_checked) {
            if Utc::now() - last.with_timezone(&Utc) < Duration::hours(6) {
                if let Some(version) = cache.latest_version.clone() {
                    return Ok(Some((version, cache.changelog.clone())));
                }
            }
        }
    }

    let url = "https://api.github.com/repos/ALIPHATICHYD/Soroban-Registry/releases/latest";
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", "soroban-registry-cli")
        .send()
        .await?;
    if !resp.status().is_success() {
        return Ok(cache.latest_version.map(|v| (v, cache.changelog)));
    }
    let body: serde_json::Value = resp.json().await?;
    let tag = body["tag_name"].as_str().map(|s| s.to_string());
    let notes = body["body"].as_str().map(|s| s.to_string());
    cache.last_checked_at = Some(Utc::now().to_rfc3339());
    cache.latest_version = tag.clone();
    cache.changelog = notes.clone();
    let _ = save_cache(&cache);
    Ok(tag.map(|v| (v, notes)))
}
