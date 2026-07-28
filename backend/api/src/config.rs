use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct FeatureFlagEntry {
    pub key: String,
    pub is_enabled: bool,
    pub rollout_percentage: u8,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub elasticsearch_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub host: String,
    pub log_level: String,
    #[serde(default = "default_cache_url")]
    pub redis_url: String,
    /// JSON-encoded feature flags configuration.
    /// Format: [{"key":"flag_name","is_enabled":true,"rollout_percentage":100}]
    #[serde(default)]
    pub feature_flags_json: String,
}

fn default_cache_url() -> String {
    "redis://localhost:6379".to_string()
}

pub fn load_config() -> Result<AppConfig> {
    dotenv::dotenv().ok();

    let config = envy::from_env::<AppConfig>()
        .context("Failed to load configuration from environment variables")?;

    validate_config(&config)?;

    Ok(config)
}

pub fn parse_feature_flags(json_str: &str) -> Vec<FeatureFlagEntry> {
    if json_str.is_empty() {
        return Vec::new();
    }
    serde_json::from_str(json_str).unwrap_or_else(|e| {
        tracing::warn!("Failed to parse FEATURE_FLAGS_JSON: {}. Using defaults.", e);
        Vec::new()
    })
}

fn validate_config(config: &AppConfig) -> Result<()> {
    if config.jwt_secret.len() < 32 {
        anyhow::bail!("JWT_SECRET must be at least 32 characters long for security");
    }

    if !config.database_url.starts_with("postgres://")
        && !config.database_url.starts_with("postgresql://")
    {
        anyhow::bail!("DATABASE_URL must be a valid postgres connection string");
    }

    warn_if_database_tls_disabled(&config.database_url);

    Ok(())
}

/// Encryption in transit to the database (#895): Postgres connections should use
/// TLS in any non-local deployment. We can't force it here without breaking local
/// dev, but we surface a clear warning so misconfiguration is visible in logs.
fn warn_if_database_tls_disabled(database_url: &str) {
    let is_local = database_url.contains("@localhost")
        || database_url.contains("@127.0.0.1")
        || database_url.contains("@db")
        || database_url.contains("@postgres");
    let requests_tls = database_url.contains("sslmode=require")
        || database_url.contains("sslmode=verify-ca")
        || database_url.contains("sslmode=verify-full");

    if !is_local && !requests_tls {
        tracing::warn!(
            "DATABASE_URL does not request TLS (sslmode=require/verify-full). \
             Enable TLS so data in transit to Postgres is encrypted (#895)."
        );
    }
}
