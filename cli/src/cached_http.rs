//! Cached HTTP GET helper for read-only registry API requests (#972).

use crate::cache;
use crate::net::RequestBuilderExt;
use anyhow::{Context, Result};
use reqwest::StatusCode;
use std::sync::OnceLock;

/// Per-invocation cache settings set from the CLI root parser.
#[derive(Debug, Clone, Copy)]
pub struct HttpCacheOptions {
    pub no_cache: bool,
    pub verbose: u8,
}

impl Default for HttpCacheOptions {
    fn default() -> Self {
        Self {
            no_cache: false,
            verbose: 0,
        }
    }
}

static CACHE_OPTS: OnceLock<HttpCacheOptions> = OnceLock::new();

pub fn init(options: HttpCacheOptions) {
    let _ = CACHE_OPTS.set(options);
}

fn options() -> HttpCacheOptions {
    CACHE_OPTS.get().copied().unwrap_or_default()
}

/// Perform a GET request, returning cached body when available.
pub async fn cached_get(url: &str, query: &[(&str, String)]) -> Result<(StatusCode, String)> {
    let opts = options();
    let cache_key = cache::http_cache_key(url, query);

    if !opts.no_cache {
        if let Some(entry) = cache::get_http_entry(&cache_key)? {
            if opts.verbose >= 1 {
                eprintln!(
                    "{} cache hit (expires in {}s): {}",
                    "◀".cyan(),
                    entry.expires_in().unwrap_or(0),
                    truncate_key(&cache_key)
                );
            }
            return Ok((StatusCode::OK, entry.body));
        }
        if opts.verbose >= 2 {
            log::debug!("Cache miss: {}", cache_key);
        }
    } else if opts.verbose >= 1 {
        eprintln!(
            "{} cache bypassed: {}",
            "↷".yellow(),
            truncate_key(&cache_key)
        );
    }

    let client = crate::net::client();
    let response = client
        .get(url)
        .query(query)
        .send_with_retry()
        .await
        .with_context(|| format!("GET {url}"))?;

    let status = response.status();
    let body = response.text().await?;

    if !opts.no_cache && status.is_success() {
        cache::set_http_entry(&cache_key, &body)?;
        if opts.verbose >= 1 {
            eprintln!(
                "{} cached response: {}",
                "▶".cyan(),
                truncate_key(&cache_key)
            );
        }
    }

    Ok((status, body))
}

/// GET a URL with no query parameters.
pub async fn cached_get_simple(url: &str) -> Result<(StatusCode, String)> {
    cached_get(url, &[]).await
}

fn truncate_key(key: &str) -> String {
    if key.len() <= 80 {
        key.to_string()
    } else {
        format!("{}…", &key[..77])
    }
}

use colored::Colorize;
