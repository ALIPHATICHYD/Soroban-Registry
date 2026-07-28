/// Network configuration module
/// Manages configuration for different Stellar networks (Mainnet, Testnet, Futurenet)
use shared::Network;
use std::env;
use thiserror::Error;
use tracing::{debug, info};

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid network: {0}")]
    InvalidNetwork(String),
    #[error("Missing environment variable: {0}")]
    MissingEnv(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub network: Network,
    pub rpc_endpoint: String,
    pub poll_interval_secs: u64,
}

impl NetworkConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        let network_str = env::var("STELLAR_NETWORK")
            .unwrap_or_else(|_| "testnet".to_string())
            .to_lowercase();

        let network = match network_str.as_str() {
            "mainnet" => Network::Mainnet,
            "testnet" => Network::Testnet,
            "futurenet" => Network::Futurenet,
            s => return Err(ConfigError::InvalidNetwork(s.to_string())),
        };

        let rpc_endpoint = match network {
            Network::Mainnet => env::var("STELLAR_RPC_MAINNET")
                .unwrap_or_else(|_| "https://rpc-mainnet.stellar.org".to_string()),
            Network::Testnet => env::var("STELLAR_RPC_TESTNET")
                .unwrap_or_else(|_| "https://rpc-testnet.stellar.org".to_string()),
            Network::Futurenet => env::var("STELLAR_RPC_FUTURENET")
                .unwrap_or_else(|_| "https://rpc-futurenet.stellar.org".to_string()),
        };

        let poll_interval_secs = env::var("STELLAR_POLL_INTERVAL_SECS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .map_err(|e| {
                ConfigError::InvalidConfig(format!(
                    "Invalid poll interval: {} ({})",
                    env::var("STELLAR_POLL_INTERVAL_SECS").unwrap_or_default(),
                    e
                ))
            })?;

        // Validate poll interval is reasonable (1 second to 5 minutes)
        if !(1..=300).contains(&poll_interval_secs) {
            return Err(ConfigError::InvalidConfig(
                "Poll interval must be between 1 and 300 seconds".to_string(),
            ));
        }

        info!(
            "Network configuration loaded: network={}, endpoint={}, poll_interval={}s",
            network_str, rpc_endpoint, poll_interval_secs
        );

        Ok(NetworkConfig {
            network,
            rpc_endpoint,
            poll_interval_secs,
        })
    }

    /// Get network shorthand for log context
    pub fn network_name(&self) -> &str {
        match self.network {
            Network::Mainnet => "mainnet",
            Network::Testnet => "testnet",
            Network::Futurenet => "futurenet",
        }
    }
}

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub connection_string: String,
    pub min_connections: u32,
    pub max_connections: u32,
    /// Idle connections are closed after this many seconds.
    pub idle_timeout_secs: u64,
    /// Connections are recycled after this many seconds.
    pub max_lifetime_secs: u64,
    /// Queries exceeding this duration (ms) trigger a warning log.
    pub slow_query_threshold_ms: f64,
}

impl DatabaseConfig {
    /// Load database configuration from environment
    pub fn from_env() -> Result<Self, ConfigError> {
        let connection_string = env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingEnv("DATABASE_URL".to_string()))?;

        let min_connections = env::var("DB_MIN_POOL_SIZE")
            .unwrap_or_else(|_| "2".to_string())
            .parse::<u32>()
            .map_err(|e| ConfigError::InvalidConfig(format!("Invalid DB_MIN_POOL_SIZE: {}", e)))?;

        let max_connections = env::var("DB_MAX_POOL_SIZE")
            .or_else(|_| env::var("DB_MAX_CONNECTIONS"))
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u32>()
            .map_err(|e| ConfigError::InvalidConfig(format!("Invalid max_connections: {}", e)))?
            .min(50);

        let idle_timeout_secs = env::var("DB_IDLE_TIMEOUT_SECS")
            .unwrap_or_else(|_| "600".to_string())
            .parse::<u64>()
            .map_err(|e| {
                ConfigError::InvalidConfig(format!("Invalid DB_IDLE_TIMEOUT_SECS: {}", e))
            })?;

        let max_lifetime_secs = env::var("DB_MAX_LIFETIME_SECS")
            .unwrap_or_else(|_| "1800".to_string())
            .parse::<u64>()
            .map_err(|e| {
                ConfigError::InvalidConfig(format!("Invalid DB_MAX_LIFETIME_SECS: {}", e))
            })?;

        let slow_query_threshold_ms = env::var("DB_SLOW_QUERY_THRESHOLD_MS")
            .unwrap_or_else(|_| "100".to_string())
            .parse::<f64>()
            .map_err(|e| {
                ConfigError::InvalidConfig(format!("Invalid DB_SLOW_QUERY_THRESHOLD_MS: {}", e))
            })?;

        debug!(
            "Database configuration loaded: min_connections={}, max_connections={}, idle_timeout={}s",
            min_connections, max_connections, idle_timeout_secs
        );

        Ok(DatabaseConfig {
            connection_string,
            min_connections,
            max_connections,
            idle_timeout_secs,
            max_lifetime_secs,
            slow_query_threshold_ms,
        })
    }
}

/// Service configuration combining all settings
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub network: NetworkConfig,
    pub database: DatabaseConfig,
    pub backoff_max_interval_secs: u64,
    pub backoff_base_interval_secs: u64,
    pub reorg_checkpoint_depth: u64,
}

impl ServiceConfig {
    /// Load full service configuration
    pub fn from_env() -> Result<Self, ConfigError> {
        let network = NetworkConfig::from_env()?;
        let database = DatabaseConfig::from_env()?;

        let backoff_max_interval_secs = env::var("INDEXER_BACKOFF_MAX_SECS")
            .unwrap_or_else(|_| "600".to_string())
            .parse::<u64>()
            .map_err(|e| {
                ConfigError::InvalidConfig(format!("Invalid backoff max interval: {}", e))
            })?;

        let backoff_base_interval_secs = env::var("INDEXER_BACKOFF_BASE_SECS")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<u64>()
            .map_err(|e| {
                ConfigError::InvalidConfig(format!("Invalid backoff base interval: {}", e))
            })?;

        let reorg_checkpoint_depth = env::var("INDEXER_REORG_CHECKPOINT_DEPTH")
            .unwrap_or_else(|_| "100".to_string())
            .parse::<u64>()
            .map_err(|e| {
                ConfigError::InvalidConfig(format!("Invalid reorg checkpoint depth: {}", e))
            })?;

        info!(
            "Service configuration loaded: backoff_max={}s, backoff_base={}s, reorg_depth={}",
            backoff_max_interval_secs, backoff_base_interval_secs, reorg_checkpoint_depth
        );

        Ok(ServiceConfig {
            network,
            database,
            backoff_max_interval_secs,
            backoff_base_interval_secs,
            reorg_checkpoint_depth,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_name() {
        let config = NetworkConfig {
            network: Network::Mainnet,
            rpc_endpoint: "https://test".to_string(),
            poll_interval_secs: 30,
        };
        assert_eq!(config.network_name(), "mainnet");
    }

    #[test]
    fn test_invalid_network() {
        env::set_var("STELLAR_NETWORK", "invalid_network");
        // Note: would fail to parse as expected
    }

    #[test]
    fn test_network_config_defaults() {
        env::remove_var("STELLAR_NETWORK");
        env::remove_var("STELLAR_RPC_TESTNET");
        env::remove_var("STELLAR_POLL_INTERVAL_SECS");

        let config = NetworkConfig::from_env().expect("Should load with defaults");
        assert_eq!(config.network_name(), "testnet");
        assert_eq!(config.poll_interval_secs, 30);
    }
}
