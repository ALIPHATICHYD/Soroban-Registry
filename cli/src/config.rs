#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

const DEFAULT_API_BASE: &str = "http://localhost:3001";
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const CONFIG_DIR_NAME: &str = ".soroban-registry";
const CONFIG_FILE_NAME: &str = "config.toml";
const LEGACY_CONFIG_FILE_NAME: &str = ".soroban-registry.toml";
const ENV_NETWORK: &str = "SOROBAN_REGISTRY_NETWORK";
const ENV_API_URL: &str = "SOROBAN_REGISTRY_API_URL";
const ENV_API_BASE: &str = "SOROBAN_REGISTRY_API_BASE";
const ENV_TIMEOUT: &str = "SOROBAN_REGISTRY_TIMEOUT";
const ENV_PROFILE: &str = "SOROBAN_REGISTRY_PROFILE";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    Futurenet,
    Auto, // Issue #78: Added Auto routing variant
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Network::Mainnet => write!(f, "mainnet"),
            Network::Testnet => write!(f, "testnet"),
            Network::Futurenet => write!(f, "futurenet"),
            Network::Auto => write!(f, "auto"), // Issue #78
        }
    }
}

impl FromStr for Network {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mainnet" => Ok(Network::Mainnet),
            "testnet" => Ok(Network::Testnet),
            "futurenet" => Ok(Network::Futurenet),
            "auto" => Ok(Network::Auto),
            _ => anyhow::bail!(
                "Invalid network: {}. Allowed values: mainnet, testnet, futurenet, auto",
                s
            ),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct ConfigFile {
    current_profile: Option<String>,
    defaults: Option<DefaultsSection>,
    profiles: Option<std::collections::HashMap<String, DefaultsSection>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct DefaultsSection {
    network: Option<String>,
    api_base: Option<String>,
    timeout: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub network: Network,
    pub api_base: String,
    pub timeout: u64,
}

pub fn resolve_network(
    cli_network: Option<String>,
    cli_profile: Option<String>,
) -> Result<Network> {
    let cfg = resolve_runtime_config(cli_network, None, None, cli_profile)?;
    Ok(cfg.network)
}

pub fn resolve_profile_overrides(
    config: &ConfigFile,
    cli_profile: Option<String>,
    env_profile: Option<String>,
) -> (Option<String>, DefaultsSection) {
    let active_profile_name = cli_profile
        .or(env_profile)
        .or_else(|| config.current_profile.clone());

    let mut defaults = config.defaults.clone().unwrap_or_default();

    if let Some(profile_name) = &active_profile_name {
        if let Some(profiles) = &config.profiles {
            if let Some(profile) = profiles.get(profile_name) {
                if let Some(n) = &profile.network {
                    defaults.network = Some(n.clone());
                }
                if let Some(a) = &profile.api_base {
                    defaults.api_base = Some(a.clone());
                }
                if let Some(t) = profile.timeout {
                    defaults.timeout = Some(t);
                }
            }
        }
    }

    (active_profile_name, defaults)
}

pub fn resolve_runtime_config(
    cli_network: Option<String>,
    cli_api_base: Option<String>,
    cli_timeout: Option<u64>,
    cli_profile: Option<String>,
) -> Result<RuntimeConfig> {
    let env_overrides = read_env_overrides()?;
    let config = load_config_file_safely().unwrap_or_default();

    let (_, defaults) =
        resolve_profile_overrides(&config, cli_profile, env_overrides.profile.clone());

    resolve_runtime_config_with_sources(
        cli_network,
        cli_api_base,
        cli_timeout,
        env_overrides,
        defaults,
    )
}

pub fn show_config() -> Result<()> {
    migrate_legacy_config()?;
    let path = config_file_path().context("Could not determine home directory")?;
    let config = load_config_file_safely().unwrap_or_default();

    println!("Config file: {}", path.display());
    let active_profile = config.current_profile.as_deref().unwrap_or("default");
    println!("Active profile: {}", active_profile);

    let defaults = config.defaults.unwrap_or_default();
    println!(
        "defaults.network = {}",
        defaults.network.as_deref().unwrap_or("testnet")
    );
    println!(
        "defaults.api_base = {}",
        defaults.api_base.as_deref().unwrap_or(DEFAULT_API_BASE)
    );
    println!(
        "defaults.timeout = {}",
        defaults.timeout.unwrap_or(DEFAULT_TIMEOUT_SECS)
    );

    if let Some(profiles) = config.profiles {
        for (name, prof) in profiles {
            println!("\n[profile: {}]", name);
            if let Some(n) = &prof.network {
                println!("  network = {}", n);
            }
            if let Some(a) = &prof.api_base {
                println!("  api_base = {}", a);
            }
            if let Some(t) = prof.timeout {
                println!("  timeout = {}", t);
            }
        }
    }

    Ok(())
}

pub fn set_profile(profile_name: &str) -> Result<()> {
    migrate_legacy_config()?;
    let path = config_file_path().context("Could not determine home directory")?;
    ensure_config_file_exists(&path)?;

    let mut config = load_config_file(&path).unwrap_or_default();
    config.current_profile = Some(profile_name.to_string());

    let toml_str = toml::to_string_pretty(&config)?;
    fs::write(&path, toml_str)?;
    println!("Active profile set to '{}'", profile_name);
    Ok(())
}

fn load_config_file_safely() -> Result<ConfigFile> {
    migrate_legacy_config()?;
    let path = match config_file_path() {
        Some(p) => p,
        None => return Ok(ConfigFile::default()),
    };

    if !path.exists() {
        return Ok(ConfigFile::default());
    }

    load_config_file(&path)
}

pub fn edit_config() -> Result<()> {
    migrate_legacy_config()?;
    let path = config_file_path().context("Could not determine home directory")?;
    ensure_config_file_exists(&path)?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("Failed to launch editor `{}`", editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with non-zero status");
    }

    Ok(())
}

fn load_defaults_section() -> Result<DefaultsSection> {
    Ok(load_config_file_safely()?.defaults.unwrap_or_default())
}

#[derive(Debug, Clone, Default)]
struct EnvOverrides {
    profile: Option<String>,
    network: Option<String>,
    api_base: Option<String>,
    timeout: Option<u64>,
}

fn resolve_runtime_config_with_sources(
    cli_network: Option<String>,
    cli_api_base: Option<String>,
    cli_timeout: Option<u64>,
    env_overrides: EnvOverrides,
    defaults: DefaultsSection,
) -> Result<RuntimeConfig> {
    let network_raw = cli_network
        .or(env_overrides.network)
        .or(defaults.network)
        .unwrap_or_else(|| "testnet".to_string());
    let network = network_raw.parse::<Network>()?;

    let api_base_raw = cli_api_base
        .or(env_overrides.api_base)
        .or(defaults.api_base)
        .unwrap_or_else(|| DEFAULT_API_BASE.to_string());
    let api_base = validate_api_base(&api_base_raw)?;

    let timeout = cli_timeout
        .or(env_overrides.timeout)
        .or(defaults.timeout)
        .unwrap_or(DEFAULT_TIMEOUT_SECS);
    validate_timeout(timeout)?;

    Ok(RuntimeConfig {
        network,
        api_base,
        timeout,
    })
}

fn read_env_overrides() -> Result<EnvOverrides> {
    let profile = read_env_string(ENV_PROFILE);
    let network = read_env_string(ENV_NETWORK);

    // Keep support for the existing API URL variable while also allowing API_BASE.
    let api_base = read_env_string(ENV_API_URL).or_else(|| read_env_string(ENV_API_BASE));

    let timeout = match read_env_string(ENV_TIMEOUT) {
        Some(value) => Some(parse_timeout(&value, ENV_TIMEOUT)?),
        None => None,
    };

    Ok(EnvOverrides {
        profile,
        network,
        api_base,
        timeout,
    })
}

fn read_env_string(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn parse_timeout(raw: &str, source: &str) -> Result<u64> {
    let timeout = raw.parse::<u64>().with_context(|| {
        format!(
            "Invalid value for {}: `{}` (expected positive integer)",
            source, raw
        )
    })?;
    validate_timeout(timeout)?;
    Ok(timeout)
}

fn validate_timeout(timeout: u64) -> Result<()> {
    if timeout == 0 {
        anyhow::bail!("timeout must be greater than 0");
    }
    Ok(())
}

fn validate_api_base(raw: &str) -> Result<String> {
    let api_base = raw.trim();
    if api_base.is_empty() {
        anyhow::bail!("api_base must not be empty");
    }
    if !(api_base.starts_with("http://") || api_base.starts_with("https://")) {
        anyhow::bail!("api_base must start with http:// or https://");
    }
    Ok(api_base.to_string())
}

fn load_config_file(path: &Path) -> Result<ConfigFile> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file at {:?}", path))?;
    toml::from_str(&content).with_context(|| "Failed to parse config file")
}

fn ensure_config_file_exists(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {:?}", parent))?;
    }

    let default_content = r#"[defaults]
network = "testnet"
api_base = "http://localhost:3001"
timeout = 30
"#;
    fs::write(path, default_content)
        .with_context(|| format!("Failed to write default config to {:?}", path))?;

    Ok(())
}

pub fn config_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| config_file_path_for(&home))
}

fn config_file_path_for(base: &Path) -> PathBuf {
    base.join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME)
}

fn legacy_config_file_path_for(base: &Path) -> PathBuf {
    base.join(LEGACY_CONFIG_FILE_NAME)
}

fn migrate_legacy_config() -> Result<()> {
    let Some(home) = dirs::home_dir() else {
        return Ok(());
    };
    migrate_legacy_config_for(&home)
}

fn migrate_legacy_config_for(base: &Path) -> Result<()> {
    let legacy_path = legacy_config_file_path_for(base);
    let current_path = config_file_path_for(base);

    if !legacy_path.exists() || current_path.exists() {
        return Ok(());
    }

    if let Some(parent) = current_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {:?}", parent))?;
    }

    match fs::rename(&legacy_path, &current_path) {
        Ok(()) => Ok(()),
        Err(err) => {
            fs::copy(&legacy_path, &current_path).with_context(|| {
                format!(
                    "Failed to copy legacy config from {:?} to {:?}: {}",
                    legacy_path, current_path, err
                )
            })?;
            fs::remove_file(&legacy_path)
                .with_context(|| format!("Failed to remove legacy config at {:?}", legacy_path))?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_network_parsing() {
        assert_eq!("mainnet".parse::<Network>().unwrap(), Network::Mainnet);
        assert_eq!("testnet".parse::<Network>().unwrap(), Network::Testnet);
        assert_eq!("futurenet".parse::<Network>().unwrap(), Network::Futurenet);
        assert_eq!("auto".parse::<Network>().unwrap(), Network::Auto); // Issue #78
        assert_eq!("Mainnet".parse::<Network>().unwrap(), Network::Mainnet); // Case insensitive
        assert!("invalid".parse::<Network>().is_err());
    }

    #[test]
    fn test_load_config_file_with_defaults_section() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"current_profile = "dev"
[defaults]
network = "mainnet"
api_base = "http://localhost:9000"
timeout = 55

[profiles.dev]
network = "testnet"
"#,
        )
        .unwrap();

        let parsed = load_config_file(&config_path).unwrap();
        let defaults = parsed.defaults.unwrap();

        assert_eq!(defaults.network.as_deref(), Some("mainnet"));
        assert_eq!(defaults.api_base.as_deref(), Some("http://localhost:9000"));
        assert_eq!(defaults.timeout, Some(55));
        assert_eq!(parsed.current_profile.as_deref(), Some("dev"));
    }

    #[test]
    fn test_config_file_path_for_base() {
        let dir = tempdir().unwrap();
        let expected = dir.path().join(CONFIG_DIR_NAME).join(CONFIG_FILE_NAME);
        assert_eq!(config_file_path_for(dir.path()), expected);
    }

    #[test]
    fn test_migrate_legacy_config_for_moves_file() {
        let dir = tempdir().unwrap();
        let legacy_path = legacy_config_file_path_for(dir.path());
        let current_path = config_file_path_for(dir.path());
        fs::write(&legacy_path, "test = true").unwrap();

        migrate_legacy_config_for(dir.path()).unwrap();

        assert!(!legacy_path.exists());
        assert!(current_path.exists());
        assert_eq!(fs::read_to_string(&current_path).unwrap(), "test = true");
    }

    #[test]
    fn test_migrate_legacy_config_for_skips_when_current_exists() {
        let dir = tempdir().unwrap();
        let legacy_path = legacy_config_file_path_for(dir.path());
        let current_path = config_file_path_for(dir.path());
        if let Some(parent) = current_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&current_path, "current = true").unwrap();
        fs::write(&legacy_path, "legacy = true").unwrap();

        migrate_legacy_config_for(dir.path()).unwrap();

        assert!(legacy_path.exists());
        assert_eq!(fs::read_to_string(&current_path).unwrap(), "current = true");
    }

    #[test]
    fn test_resolve_runtime_precedence_cli_over_env_over_config() {
        let defaults = DefaultsSection {
            network: Some("futurenet".to_string()),
            api_base: Some("http://config.example".to_string()),
            timeout: Some(45),
        };
        let env = EnvOverrides {
            profile: None,
            network: Some("mainnet".to_string()),
            api_base: Some("https://env.example".to_string()),
            timeout: Some(90),
        };

        let cfg = resolve_runtime_config_with_sources(
            Some("testnet".to_string()),
            Some("https://cli.example".to_string()),
            Some(120),
            env,
            defaults,
        )
        .unwrap();

        assert_eq!(cfg.network, Network::Testnet);
        assert_eq!(cfg.api_base, "https://cli.example");
        assert_eq!(cfg.timeout, 120);
    }

    #[test]
    fn test_resolve_runtime_uses_env_when_cli_missing() {
        let defaults = DefaultsSection {
            network: Some("futurenet".to_string()),
            api_base: Some("http://config.example".to_string()),
            timeout: Some(45),
        };
        let env = EnvOverrides {
            profile: None,
            network: Some("mainnet".to_string()),
            api_base: Some("https://env.example".to_string()),
            timeout: Some(90),
        };

        let cfg = resolve_runtime_config_with_sources(None, None, None, env, defaults).unwrap();

        assert_eq!(cfg.network, Network::Mainnet);
        assert_eq!(cfg.api_base, "https://env.example");
        assert_eq!(cfg.timeout, 90);
    }

    #[test]
    fn test_resolve_runtime_rejects_invalid_api_base() {
        let defaults = DefaultsSection {
            network: Some("testnet".to_string()),
            api_base: Some("localhost:3001".to_string()),
            timeout: Some(30),
        };

        let err = resolve_runtime_config_with_sources(
            None,
            None,
            None,
            EnvOverrides::default(),
            defaults,
        )
        .unwrap_err();
        assert!(err
            .to_string()
            .contains("api_base must start with http:// or https://"));
    }

    #[test]
    fn test_resolve_runtime_rejects_zero_timeout() {
        let defaults = DefaultsSection {
            network: Some("testnet".to_string()),
            api_base: Some("http://localhost:3001".to_string()),
            timeout: Some(0),
        };

        let err = resolve_runtime_config_with_sources(
            None,
            None,
            None,
            EnvOverrides::default(),
            defaults,
        )
        .unwrap_err();
        assert!(err.to_string().contains("timeout must be greater than 0"));
    }

    #[test]
    fn test_resolve_profile_overrides_uses_cli() {
        let mut config = ConfigFile::default();
        let mut profiles = std::collections::HashMap::new();

        let mut prod = DefaultsSection::default();
        prod.network = Some("mainnet".to_string());
        prod.timeout = Some(100);
        profiles.insert("prod".to_string(), prod);

        config.profiles = Some(profiles);
        config.current_profile = Some("dev".to_string());

        let (active, defaults) = resolve_profile_overrides(&config, Some("prod".to_string()), None);

        assert_eq!(active.unwrap(), "prod");
        assert_eq!(defaults.network.unwrap(), "mainnet");
        assert_eq!(defaults.timeout.unwrap(), 100);
    }

    #[test]
    fn test_resolve_profile_overrides_uses_env() {
        let mut config = ConfigFile::default();
        let mut profiles = std::collections::HashMap::new();

        let mut dev = DefaultsSection::default();
        dev.network = Some("testnet".to_string());
        dev.api_base = Some("https://dev.example".to_string());
        profiles.insert("dev".to_string(), dev);

        config.profiles = Some(profiles);

        let (active, defaults) = resolve_profile_overrides(&config, None, Some("dev".to_string()));

        assert_eq!(active.unwrap(), "dev");
        assert_eq!(defaults.network.unwrap(), "testnet");
        assert_eq!(defaults.api_base.unwrap(), "https://dev.example");
    }

    #[test]
    fn test_resolve_profile_overrides_uses_config_current() {
        let mut config = ConfigFile::default();
        let mut profiles = std::collections::HashMap::new();

        let mut default_prof = DefaultsSection::default();
        default_prof.network = Some("futurenet".to_string());
        profiles.insert("default".to_string(), default_prof);

        config.profiles = Some(profiles);
        config.current_profile = Some("default".to_string());

        let (active, defaults) = resolve_profile_overrides(&config, None, None);

        assert_eq!(active.unwrap(), "default");
        assert_eq!(defaults.network.unwrap(), "futurenet");
    }
}
// TODO: migrated auth
