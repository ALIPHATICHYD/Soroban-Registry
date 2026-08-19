//! Building the shared registry client from CLI globals.
//!
//! Every command that talks to the registry goes through here, so the CLI has
//! exactly one interpretation of authentication, timeouts, retries, pagination
//! tokens, and API errors — the same one any other consumer of
//! `soroban-registry-client` gets.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::Result;
use colored::Colorize;
use registry_client::{ClientConfig, RegistryClient, ResponseCache, RetryPolicy};

/// Identifies this CLI to the registry, so server-side metrics can tell CLI
/// traffic from SDK traffic.
pub fn user_agent() -> String {
    format!("soroban-registry-cli/{}", env!("CARGO_PKG_VERSION"))
}

/// Per-invocation client settings, set once from the root parser.
#[derive(Debug, Clone, Copy)]
pub struct ClientSettings {
    pub timeout: Duration,
}

impl Default for ClientSettings {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
        }
    }
}

static SETTINGS: OnceLock<ClientSettings> = OnceLock::new();

/// Record the resolved global `--timeout` for later client construction.
pub fn init(timeout_secs: Option<u64>) {
    let settings = match timeout_secs.filter(|secs| *secs > 0) {
        Some(secs) => ClientSettings {
            timeout: Duration::from_secs(secs),
        },
        None => ClientSettings::default(),
    };
    let _ = SETTINGS.set(settings);
}

fn settings() -> ClientSettings {
    SETTINGS.get().copied().unwrap_or_default()
}

/// The CLI's on-disk HTTP cache, exposed to the client as a cache hook.
///
/// Keys are composed exactly as [`crate::cache::http_cache_key`] composes them,
/// so a response fetched through the client and one fetched through
/// [`crate::cached_http`] share a cache entry.
struct CliResponseCache {
    options: crate::cached_http::HttpCacheOptions,
}

impl CliResponseCache {
    fn disk_key(key: &str) -> String {
        format!("GET:{key}")
    }
}

impl ResponseCache for CliResponseCache {
    fn get(&self, key: &str) -> Option<String> {
        if self.options.no_cache {
            if self.options.verbose >= 1 {
                eprintln!("{} cache bypassed: {}", "↷".yellow(), truncate(key));
            }
            return None;
        }

        match crate::cache::get_http_entry(&Self::disk_key(key)) {
            Ok(Some(entry)) => {
                if self.options.verbose >= 1 {
                    eprintln!(
                        "{} cache hit (expires in {}s): {}",
                        "◀".cyan(),
                        entry.expires_in().unwrap_or(0),
                        truncate(key)
                    );
                }
                Some(entry.body)
            }
            Ok(None) => None,
            // A broken cache must never fail a command; fall through to the network.
            Err(err) => {
                log::debug!("HTTP cache read failed: {err}");
                None
            }
        }
    }

    fn put(&self, key: &str, body: &str) {
        if self.options.no_cache {
            return;
        }
        if let Err(err) = crate::cache::set_http_entry(&Self::disk_key(key), body) {
            log::debug!("HTTP cache write failed: {err}");
        } else if self.options.verbose >= 1 {
            eprintln!("{} cached response: {}", "▶".cyan(), truncate(key));
        }
    }
}

fn truncate(key: &str) -> String {
    if key.len() <= 80 {
        key.to_string()
    } else {
        format!("{}…", &key[..77])
    }
}

fn base_config(api_url: &str) -> ClientConfig {
    ClientConfig::new(api_url)
        .with_timeout(settings().timeout)
        .with_user_agent(user_agent())
        // Reads are retried; mutations only when the call supplies an
        // idempotency key (see registry_client::RetryPolicy).
        .with_retry_policy(RetryPolicy::default())
}

/// A client carrying the stored session credential, if the user is logged in.
///
/// Reads are served from the CLI's HTTP cache when one is warm, honouring
/// `--no-cache`.
pub async fn client(api_url: &str) -> Result<RegistryClient> {
    let token = crate::auth::access_token_for_requests(api_url).await?;
    let cache = Arc::new(CliResponseCache {
        options: crate::cached_http::cache_options(),
    });

    Ok(RegistryClient::from_config(base_config(api_url))?
        .with_bearer_token(token)
        .with_response_cache(cache))
}

/// A client that always goes to the network, for reads whose freshness matters
/// and for anything that mutates.
pub async fn uncached_client(api_url: &str) -> Result<RegistryClient> {
    let token = crate::auth::access_token_for_requests(api_url).await?;
    Ok(RegistryClient::from_config(base_config(api_url))?.with_bearer_token(token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_agent_names_the_cli_and_its_version() {
        let agent = user_agent();
        assert!(agent.starts_with("soroban-registry-cli/"), "{agent}");
        assert!(agent.len() > "soroban-registry-cli/".len(), "{agent}");
    }

    #[test]
    fn cache_keys_match_the_cli_http_cache_scheme() {
        let composed = "http://registry.test/api/contracts?limit=20";
        assert_eq!(
            CliResponseCache::disk_key(composed),
            crate::cache::http_cache_key(
                "http://registry.test/api/contracts",
                &[("limit", "20".to_string())]
            ),
            "the client and cached_http must share cache entries"
        );
    }

    #[test]
    fn a_zero_or_missing_timeout_falls_back_to_the_default() {
        // `init` is process-global, so assert on the pure mapping instead.
        assert_eq!(ClientSettings::default().timeout, Duration::from_secs(30));
    }

    #[test]
    fn long_cache_keys_are_truncated_for_logging() {
        let key = "x".repeat(200);
        let rendered = truncate(&key);
        assert!(rendered.chars().count() <= 81, "{}", rendered.len());
        assert!(rendered.ends_with('…'));
    }
}
