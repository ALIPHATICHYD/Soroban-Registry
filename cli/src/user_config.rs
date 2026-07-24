use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const CONFIG_DIR_NAME: &str = ".soroban-registry";
const USER_CONFIG_FILE_NAME: &str = "config.json";
const LEGACY_USER_CONFIG_FILE_NAME: &str = ".soroban-registry.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserConfig {
    pub api_key: Option<String>,
    pub default_network: String,
    pub output_format: String,
    pub sort_preference: String,
    pub update_checks_enabled: bool,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            default_network: "testnet".to_string(),
            output_format: "table".to_string(),
            sort_preference: "created_at:desc".to_string(),
            update_checks_enabled: true,
        }
    }
}

pub fn config_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(CONFIG_DIR_NAME).join(USER_CONFIG_FILE_NAME))
}

fn legacy_config_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(LEGACY_USER_CONFIG_FILE_NAME))
}

pub fn load() -> Result<UserConfig> {
    migrate_legacy_file_if_present()?;
    let Some(path) = config_file_path() else {
        return Ok(UserConfig::default());
    };
    if !path.exists() {
        return Ok(UserConfig::default());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read user config: {}", path.display()))?;
    let parsed = serde_json::from_str::<UserConfig>(&raw)
        .with_context(|| format!("Failed to parse user config JSON: {}", path.display()))?;
    Ok(parsed)
}

pub fn save(config: &UserConfig) -> Result<()> {
    let Some(path) = config_file_path() else {
        anyhow::bail!("Could not resolve home directory for user config");
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(config)?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write user config: {}", path.display()))?;
    Ok(())
}

pub fn set_key(key: &str, value: &str) -> Result<()> {
    let mut config = load()?;
    apply_set(&mut config, key, value)?;
    save(&config)
}

pub fn get_key(key: &str) -> Result<Option<String>> {
    let config = load()?;
    Ok(match key {
        "api-key" => config.api_key,
        "default-network" => Some(config.default_network),
        "output-format" => Some(config.output_format),
        "sort-preference" => Some(config.sort_preference),
        "update-checks-enabled" => Some(config.update_checks_enabled.to_string()),
        _ => None,
    })
}

pub fn list() -> Result<UserConfig> {
    load()
}

pub fn reset_to_defaults() -> Result<UserConfig> {
    let defaults = UserConfig::default();
    save(&defaults)?;
    Ok(defaults)
}

pub fn validate_key(key: &str) -> Result<()> {
    match key {
        "api-key" | "default-network" | "output-format" | "sort-preference" | "update-checks-enabled" => Ok(()),
        _ => anyhow::bail!(
            "Invalid config key '{}'. Allowed keys: api-key, default-network, output-format, sort-preference, update-checks-enabled",
            key
        ),
    }
}

fn apply_set(config: &mut UserConfig, key: &str, value: &str) -> Result<()> {
    validate_key(key)?;
    match key {
        "api-key" => {
            config.api_key = if value.trim().is_empty() {
                None
            } else {
                Some(value.trim().to_string())
            };
        }
        "default-network" => {
            let normalized = value.trim().to_lowercase();
            if !matches!(
                normalized.as_str(),
                "mainnet" | "testnet" | "futurenet" | "auto"
            ) {
                anyhow::bail!("default-network must be one of: mainnet, testnet, futurenet, auto");
            }
            config.default_network = normalized;
        }
        "output-format" => {
            let normalized = value.trim().to_lowercase();
            if !matches!(normalized.as_str(), "table" | "json" | "csv") {
                anyhow::bail!("output-format must be one of: table, json, csv");
            }
            config.output_format = normalized;
        }
        "sort-preference" => {
            if value.trim().is_empty() {
                anyhow::bail!("sort-preference cannot be empty");
            }
            config.sort_preference = value.trim().to_string();
        }
        "update-checks-enabled" => {
            let normalized = value.trim().to_lowercase();
            config.update_checks_enabled = match normalized.as_str() {
                "true" | "1" | "yes" | "on" => true,
                "false" | "0" | "no" | "off" => false,
                _ => anyhow::bail!("update-checks-enabled must be a boolean (true/false)"),
            };
        }
        _ => {}
    }
    Ok(())
}

fn migrate_legacy_file_if_present() -> Result<()> {
    let Some(legacy_path) = legacy_config_file_path() else {
        return Ok(());
    };
    let Some(new_path) = config_file_path() else {
        return Ok(());
    };

    if !legacy_path.exists() || new_path.exists() {
        return Ok(());
    }

    if let Some(parent) = new_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }
    fs::rename(&legacy_path, &new_path).or_else(|_| {
        fs::copy(&legacy_path, &new_path)?;
        fs::remove_file(&legacy_path)?;
        Ok::<(), std::io::Error>(())
    })?;
    Ok(())
}
