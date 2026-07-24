//! cache.rs — `soroban-registry cache` (#845)
//!
//! Manages the CLI's local disk cache of registry API responses.
//! The cache lives in `~/.soroban-registry/cache/` and stores JSON
//! responses keyed by URL. Each entry records its TTL and compression state.
//!
//! Subcommands: clear, status, configure, optimize, export

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ── Cache layout ──────────────────────────────────────────────────────────────

const CACHE_DIR_NAME: &str = ".soroban-registry/cache";
const CONFIG_FILE_NAME: &str = ".soroban-registry/cache-config.json";
const ENTRY_EXT: &str = ".cache.json";
const COMPRESSED_EXT: &str = ".cache.gz";

// ── Config types ──────────────────────────────────────────────────────────────

/// Persisted cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Default TTL in seconds for disk-cache entries.
    pub ttl_seconds: u64,
    /// Maximum total disk cache size in bytes (0 = unlimited).
    pub max_disk_bytes: u64,
    /// Whether compression is enabled for disk entries.
    pub compression: bool,
    /// Whether auto-refresh of stale entries is enabled.
    pub auto_refresh: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl_seconds: 300,           // 5 minutes
            max_disk_bytes: 52_428_800, // 50 MiB
            compression: false,
            auto_refresh: true,
        }
    }
}

// ── Entry metadata ────────────────────────────────────────────────────────────

/// Metadata stored alongside each cache entry.
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Cache key (URL or logical key).
    pub key: String,
    /// UNIX timestamp when the entry was written.
    pub created_at: u64,
    /// TTL in seconds applied when the entry was written.
    pub ttl_seconds: u64,
    /// Whether the entry body is gzip-compressed.
    pub compressed: bool,
    /// Byte size of the stored payload.
    pub size_bytes: u64,
    /// Hit count since creation.
    pub hits: u64,
    /// Cached response body (raw or base64 of compressed bytes).
    pub body: String,
}

impl CacheEntry {
    fn is_stale(&self) -> bool {
        let now = now_unix();
        now.saturating_sub(self.created_at) >= self.ttl_seconds
    }

    fn age_seconds(&self) -> u64 {
        now_unix().saturating_sub(self.created_at)
    }

    pub fn expires_in(&self) -> Option<u64> {
        let age = self.age_seconds();
        if age >= self.ttl_seconds {
            None
        } else {
            Some(self.ttl_seconds - age)
        }
    }
}

// ── Path helpers ──────────────────────────────────────────────────────────────

fn cache_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not resolve home directory")?;
    Ok(home.join(CACHE_DIR_NAME))
}

fn config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not resolve home directory")?;
    Ok(home.join(CONFIG_FILE_NAME))
}

fn key_to_filename(key: &str) -> String {
    // Replace URL-unsafe chars with underscores for filesystem safety.
    let safe: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    // Truncate to 180 chars to stay well under filesystem limits.
    format!("{}{}", &safe[..safe.len().min(180)], ENTRY_EXT)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

// ── Config I/O ────────────────────────────────────────────────────────────────

fn load_config() -> Result<CacheConfig> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(CacheConfig::default());
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read cache config: {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse cache config: {}", path.display()))
}

fn save_config(cfg: &CacheConfig) -> Result<()> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(cfg)?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write cache config: {}", path.display()))
}

// ── Entry I/O ─────────────────────────────────────────────────────────────────

fn load_entries(dir: &Path) -> Vec<CacheEntry> {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return vec![];
    };

    let mut entries = Vec::new();
    for item in read_dir.flatten() {
        let path = item.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.ends_with(ENTRY_EXT) && !name.ends_with(COMPRESSED_EXT) {
            continue;
        }
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(entry) = serde_json::from_str::<CacheEntry>(&raw) {
                entries.push(entry);
            }
        }
    }
    entries
}

fn delete_entry_file(dir: &Path, key: &str) -> bool {
    let filename = key_to_filename(key);
    let path = dir.join(&filename);
    fs::remove_file(path).is_ok()
}

// ── Public entry points ───────────────────────────────────────────────────────

/// `soroban-registry cache clear [--level disk|all] [--key <key>]`
pub fn clear(level: &str, key: Option<&str>) -> Result<()> {
    let dir = cache_dir()?;

    if !dir.exists() {
        println!();
        println!(
            "  {} Cache directory does not exist — nothing to clear.",
            "·".dimmed()
        );
        println!();
        return Ok(());
    }

    if let Some(k) = key {
        // Clear a single entry.
        if delete_entry_file(&dir, k) {
            println!();
            println!(
                "  {} Entry '{}' removed from disk cache.",
                "✔".green().bold(),
                k.bold()
            );
            println!();
        } else {
            println!();
            println!("  {} No cache entry found for '{}'.", "·".dimmed(), k);
            println!();
        }
        return Ok(());
    }

    match level {
        "memory" | "in-memory" => {
            // In-memory cache is per-process; clearing it here is a no-op from the CLI.
            println!();
            println!(
                "  {} In-memory cache is per-process and is cleared automatically when the CLI exits.",
                "·".dimmed()
            );
            println!();
        }
        "remote" => {
            println!();
            println!(
                "  {} Remote cache invalidation is not supported from the CLI.",
                "·".dimmed()
            );
            println!();
        }
        _ => {
            // Disk or all.
            let entries = load_entries(&dir);
            let count = entries.len();
            let total_bytes: u64 = entries.iter().map(|e| e.size_bytes).sum();

            fs::remove_dir_all(&dir)
                .with_context(|| format!("Failed to clear cache dir: {}", dir.display()))?;
            fs::create_dir_all(&dir)
                .with_context(|| format!("Failed to recreate cache dir: {}", dir.display()))?;

            println!();
            println!("  {} Disk cache cleared.", "✔".green().bold());
            println!(
                "     Removed {} entr{} ({}).",
                count,
                if count == 1 { "y" } else { "ies" },
                format_bytes(total_bytes)
            );
            println!();
        }
    }

    Ok(())
}

/// `soroban-registry cache status [--json]`
pub fn status(json: bool) -> Result<()> {
    let dir = cache_dir()?;
    let cfg = load_config()?;

    let entries = if dir.exists() {
        load_entries(&dir)
    } else {
        vec![]
    };

    let total_entries = entries.len();
    let stale_entries = entries.iter().filter(|e| e.is_stale()).count();
    let fresh_entries = total_entries - stale_entries;
    let total_bytes: u64 = entries.iter().map(|e| e.size_bytes).sum();
    let total_hits: u64 = entries.iter().map(|e| e.hits).sum();
    let compressed_count = entries.iter().filter(|e| e.compressed).count();

    // Hit rate: total_hits / (total_hits + total_entries) as a rough proxy.
    let hit_rate = if total_hits + total_entries as u64 > 0 {
        (total_hits as f64 / (total_hits + total_entries as u64) as f64) * 100.0
    } else {
        0.0
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "total_entries": total_entries,
                "fresh_entries": fresh_entries,
                "stale_entries": stale_entries,
                "compressed_entries": compressed_count,
                "total_bytes": total_bytes,
                "hit_rate_pct": (hit_rate * 10.0).round() / 10.0,
                "total_hits": total_hits,
                "config": {
                    "ttl_seconds": cfg.ttl_seconds,
                    "max_disk_bytes": cfg.max_disk_bytes,
                    "compression": cfg.compression,
                    "auto_refresh": cfg.auto_refresh
                }
            }))?
        );
        return Ok(());
    }

    println!();
    println!("{}", "Cache Status".bold().cyan());
    println!("{}", "═".repeat(55).cyan());

    // Stats
    println!("  {}", "Disk Cache".bold().underline());
    println!(
        "  {:<28} {}",
        "Entries (total):".bold(),
        total_entries.to_string().bright_blue()
    );
    println!(
        "  {:<28} {}",
        "  Fresh:".bold(),
        fresh_entries.to_string().green()
    );
    println!(
        "  {:<28} {}",
        "  Stale:".bold(),
        if stale_entries > 0 {
            stale_entries.to_string().yellow()
        } else {
            stale_entries.to_string().dimmed()
        }
    );
    println!(
        "  {:<28} {}",
        "  Compressed:".bold(),
        compressed_count.to_string().dimmed()
    );
    println!(
        "  {:<28} {}",
        "Total size:".bold(),
        format_bytes(total_bytes).bright_blue()
    );
    if cfg.max_disk_bytes > 0 {
        let pct = (total_bytes as f64 / cfg.max_disk_bytes as f64 * 100.0).min(100.0);
        println!(
            "  {:<28} {} / {} ({:.1}%)",
            "Disk usage:".bold(),
            format_bytes(total_bytes),
            format_bytes(cfg.max_disk_bytes),
            pct
        );
    }
    println!("  {:<28} {:.1}%", "Hit rate (approx):".bold(), hit_rate);
    println!();

    println!("  {}", "Configuration".bold().underline());
    println!("  {:<28} {}s", "TTL:".bold(), cfg.ttl_seconds);
    println!(
        "  {:<28} {}",
        "Max disk:".bold(),
        if cfg.max_disk_bytes == 0 {
            "unlimited".to_string()
        } else {
            format_bytes(cfg.max_disk_bytes)
        }
    );
    println!(
        "  {:<28} {}",
        "Compression:".bold(),
        if cfg.compression {
            "enabled".green().to_string()
        } else {
            "disabled".dimmed().to_string()
        }
    );
    println!(
        "  {:<28} {}",
        "Auto-refresh stale:".bold(),
        if cfg.auto_refresh {
            "enabled".green().to_string()
        } else {
            "disabled".dimmed().to_string()
        }
    );
    println!();

    // Stale warning
    if stale_entries > 0 {
        println!(
            "  {} {} stale entr{} detected. Run {} to clean up.",
            "⚠".yellow().bold(),
            stale_entries,
            if stale_entries == 1 { "y" } else { "ies" },
            "soroban-registry cache optimize".bold()
        );
        println!();
    }

    println!("{}", "═".repeat(55).cyan());
    println!();
    Ok(())
}

/// `soroban-registry cache configure [--ttl <secs>] [--max-size <bytes>]
///                                    [--compression on|off] [--auto-refresh on|off]`
pub fn configure(
    ttl: Option<u64>,
    max_size: Option<u64>,
    compression: Option<&str>,
    auto_refresh: Option<&str>,
    json: bool,
) -> Result<()> {
    let mut cfg = load_config()?;
    let mut changed = false;

    if let Some(t) = ttl {
        if t == 0 {
            anyhow::bail!("TTL must be at least 1 second.");
        }
        cfg.ttl_seconds = t;
        changed = true;
    }
    if let Some(m) = max_size {
        cfg.max_disk_bytes = m;
        changed = true;
    }
    if let Some(c) = compression {
        cfg.compression = parse_bool_flag("--compression", c)?;
        changed = true;
    }
    if let Some(a) = auto_refresh {
        cfg.auto_refresh = parse_bool_flag("--auto-refresh", a)?;
        changed = true;
    }

    if !changed {
        // Print current config.
        if json {
            println!("{}", serde_json::to_string_pretty(&cfg)?);
        } else {
            print_config(&cfg);
        }
        return Ok(());
    }

    save_config(&cfg)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&cfg)?);
    } else {
        println!();
        println!("  {} Cache configuration updated.", "✔".green().bold());
        print_config(&cfg);
    }

    Ok(())
}

/// `soroban-registry cache optimize [--json]`
/// Removes stale entries and enforces the max disk size limit.
pub fn optimize(json: bool) -> Result<()> {
    let dir = cache_dir()?;
    let cfg = load_config()?;

    if !dir.exists() {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "removed_stale": 0,
                    "removed_oversized": 0,
                    "bytes_freed": 0
                }))?
            );
        } else {
            println!();
            println!(
                "  {} Cache directory is empty — nothing to optimize.",
                "·".dimmed()
            );
            println!();
        }
        return Ok(());
    }

    let mut entries = load_entries(&dir);
    let initial_count = entries.len();
    let initial_bytes: u64 = entries.iter().map(|e| e.size_bytes).sum();

    // 1. Remove stale entries.
    let mut removed_stale = 0usize;
    entries.retain(|e| {
        if e.is_stale() {
            delete_entry_file(&dir, &e.key);
            removed_stale += 1;
            false
        } else {
            true
        }
    });

    // 2. Enforce max disk size: evict oldest entries first.
    let mut removed_oversized = 0usize;
    if cfg.max_disk_bytes > 0 {
        let mut total: u64 = entries.iter().map(|e| e.size_bytes).sum();
        // Sort oldest first for eviction.
        entries.sort_by_key(|e| e.created_at);
        while total > cfg.max_disk_bytes {
            if let Some(oldest) = entries.first() {
                total = total.saturating_sub(oldest.size_bytes);
                delete_entry_file(&dir, &oldest.key);
                removed_oversized += 1;
                entries.remove(0);
            } else {
                break;
            }
        }
    }

    let final_bytes: u64 = entries.iter().map(|e| e.size_bytes).sum();
    let bytes_freed = initial_bytes.saturating_sub(final_bytes);
    let total_removed = removed_stale + removed_oversized;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "entries_before": initial_count,
                "entries_after": entries.len(),
                "removed_stale": removed_stale,
                "removed_oversized": removed_oversized,
                "bytes_freed": bytes_freed,
                "bytes_freed_human": format_bytes(bytes_freed)
            }))?
        );
        return Ok(());
    }

    println!();
    println!("{}", "Cache Optimized".bold().cyan());
    println!("{}", "═".repeat(50).cyan());
    println!(
        "  {:<28} {}",
        "Stale entries removed:".bold(),
        removed_stale.to_string().yellow()
    );
    println!(
        "  {:<28} {}",
        "Oversized entries evicted:".bold(),
        removed_oversized.to_string().yellow()
    );
    println!(
        "  {:<28} {}",
        "Space reclaimed:".bold(),
        format_bytes(bytes_freed).green()
    );
    println!(
        "  {:<28} {}",
        "Entries remaining:".bold(),
        entries.len().to_string().bright_blue()
    );

    if total_removed == 0 {
        println!();
        println!("  {} Cache is already optimal.", "✔".green().bold());
    }

    println!("{}", "═".repeat(50).cyan());
    println!();
    Ok(())
}

/// `soroban-registry cache export [--format json|csv] [--include-stale]`
pub fn export(format: &str, include_stale: bool) -> Result<()> {
    let dir = cache_dir()?;

    let mut entries = if dir.exists() {
        load_entries(&dir)
    } else {
        vec![]
    };

    if !include_stale {
        entries.retain(|e| !e.is_stale());
    }

    match format {
        "csv" => {
            println!("key,created_at,ttl_seconds,size_bytes,compressed,hits,stale,expires_in");
            for e in &entries {
                println!(
                    "{},{},{},{},{},{},{},{}",
                    e.key,
                    e.created_at,
                    e.ttl_seconds,
                    e.size_bytes,
                    e.compressed,
                    e.hits,
                    e.is_stale(),
                    e.expires_in()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "expired".to_string()),
                );
            }
        }
        _ => {
            let out: Vec<serde_json::Value> = entries
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "key": e.key,
                        "created_at": e.created_at,
                        "ttl_seconds": e.ttl_seconds,
                        "size_bytes": e.size_bytes,
                        "compressed": e.compressed,
                        "hits": e.hits,
                        "stale": e.is_stale(),
                        "expires_in_seconds": e.expires_in()
                    })
                })
                .collect();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "entries": out,
                    "count": out.len(),
                    "exported_at": now_unix()
                }))?
            );
        }
    }

    Ok(())
}

// ── Internal utilities ────────────────────────────────────────────────────────

fn parse_bool_flag(flag: &str, value: &str) -> Result<bool> {
    match value.to_lowercase().as_str() {
        "on" | "true" | "1" | "yes" => Ok(true),
        "off" | "false" | "0" | "no" => Ok(false),
        _ => anyhow::bail!("{} must be on|off (got '{}')", flag, value),
    }
}

fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;
    if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn print_config(cfg: &CacheConfig) {
    println!();
    println!("{}", "Cache Configuration".bold().cyan());
    println!("{}", "═".repeat(50).cyan());
    println!("  {:<28} {}s", "TTL:".bold(), cfg.ttl_seconds);
    println!(
        "  {:<28} {}",
        "Max disk size:".bold(),
        if cfg.max_disk_bytes == 0 {
            "unlimited".to_string()
        } else {
            format_bytes(cfg.max_disk_bytes)
        }
    );
    println!(
        "  {:<28} {}",
        "Compression:".bold(),
        if cfg.compression {
            "on".green().to_string()
        } else {
            "off".dimmed().to_string()
        }
    );
    println!(
        "  {:<28} {}",
        "Auto-refresh stale:".bold(),
        if cfg.auto_refresh {
            "on".green().to_string()
        } else {
            "off".dimmed().to_string()
        }
    );
    println!("{}", "═".repeat(50).cyan());
    println!();
}

pub struct CachedEntry {
    pub result: serde_json::Value,
    pub detail: Option<serde_json::Value>,
}

// ── Runtime HTTP cache (#972) ─────────────────────────────────────────────────

/// Build a deterministic cache key for a GET request.
pub fn http_cache_key(url: &str, query: &[(&str, String)]) -> String {
    let mut pairs: Vec<(String, String)> = query
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    let query_str = pairs
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&");
    if query_str.is_empty() {
        format!("GET:{url}")
    } else {
        format!("GET:{url}?{query_str}")
    }
}

fn verify_cache_key(address: &str, network: &str) -> String {
    format!("verify:{address}:{network}")
}

fn read_entry_from_disk(key: &str) -> Result<Option<CacheEntry>> {
    let dir = cache_dir()?;
    if !dir.exists() {
        return Ok(None);
    }
    let path = dir.join(key_to_filename(key));
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read cache entry: {}", path.display()))?;
    let entry: CacheEntry = serde_json::from_str(&raw)
        .with_context(|| format!("Failed to parse cache entry: {}", path.display()))?;
    Ok(Some(entry))
}

fn write_entry_to_disk(key: &str, body: &str) -> Result<()> {
    let dir = cache_dir()?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create cache dir: {}", dir.display()))?;

    let cfg = load_config()?;
    let entry = CacheEntry {
        key: key.to_string(),
        created_at: now_unix(),
        ttl_seconds: cfg.ttl_seconds,
        compressed: false,
        size_bytes: body.len() as u64,
        hits: 0,
        body: body.to_string(),
    };

    enforce_max_disk_size(&dir, &cfg)?;

    let path = dir.join(key_to_filename(key));
    fs::write(&path, serde_json::to_string_pretty(&entry)?)
        .with_context(|| format!("Failed to write cache entry: {}", path.display()))?;
    Ok(())
}

fn enforce_max_disk_size(dir: &Path, cfg: &CacheConfig) -> Result<()> {
    if cfg.max_disk_bytes == 0 {
        return Ok(());
    }
    let mut entries = load_entries(dir);
    let total: u64 = entries.iter().map(|e| e.size_bytes).sum();
    if total <= cfg.max_disk_bytes {
        return Ok(());
    }
    entries.sort_by_key(|e| e.created_at);
    let mut current = total;
    for entry in entries {
        if current <= cfg.max_disk_bytes {
            break;
        }
        delete_entry_file(dir, &entry.key);
        current = current.saturating_sub(entry.size_bytes);
    }
    Ok(())
}

fn touch_entry(key: &str, mut entry: CacheEntry) -> Result<CacheEntry> {
    entry.hits = entry.hits.saturating_add(1);
    let dir = cache_dir()?;
    let path = dir.join(key_to_filename(key));
    fs::write(&path, serde_json::to_string_pretty(&entry)?)
        .with_context(|| format!("Failed to update cache entry hits: {}", path.display()))?;
    Ok(entry)
}

/// Load a cached HTTP response body when fresh.
pub fn get_http_entry(key: &str) -> Result<Option<CacheEntry>> {
    let Some(entry) = read_entry_from_disk(key)? else {
        return Ok(None);
    };
    if entry.is_stale() {
        let dir = cache_dir()?;
        delete_entry_file(&dir, key);
        return Ok(None);
    }
    let entry = touch_entry(key, entry)?;
    Ok(Some(entry))
}

/// Store a successful GET response body.
pub fn set_http_entry(key: &str, body: &str) -> Result<()> {
    write_entry_to_disk(key, body)
}

/// Verification result cache (used by `contract verify`).
pub fn get(address: &str, network: &str) -> anyhow::Result<Option<CachedEntry>> {
    let key = verify_cache_key(address, network);
    let Some(entry) = get_http_entry(&key)? else {
        return Ok(None);
    };
    let wrapper: serde_json::Value = serde_json::from_str(&entry.body)?;
    Ok(Some(CachedEntry {
        result: wrapper
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        detail: wrapper.get("detail").cloned(),
    }))
}

pub fn set(
    address: &str,
    network: &str,
    result: serde_json::Value,
    detail: Option<serde_json::Value>,
) -> anyhow::Result<()> {
    let key = verify_cache_key(address, network);
    let wrapper = serde_json::json!({
        "result": result,
        "detail": detail,
    });
    set_http_entry(&key, &wrapper.to_string())
}

#[cfg(test)]
mod runtime_cache_tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn http_cache_key_is_stable() {
        let key_a = http_cache_key(
            "http://localhost/api/contracts",
            &[("query", "token".into()), ("limit", "10".into())],
        );
        let key_b = http_cache_key(
            "http://localhost/api/contracts",
            &[("limit", "10".into()), ("query", "token".into())],
        );
        assert_eq!(key_a, key_b);
    }

    #[test]
    fn set_and_get_round_trip() {
        let _guard = test_lock();
        let key = format!("test-entry-{}", now_unix());
        set_http_entry(&key, r#"{"ok":true}"#).unwrap();
        let entry = get_http_entry(&key).expect("read cache");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().body, r#"{"ok":true}"#);
        let dir = cache_dir().unwrap();
        delete_entry_file(&dir, &key);
    }

    #[test]
    fn stale_entry_is_evicted_on_read() {
        let _guard = test_lock();
        let key = format!("stale-entry-{}", now_unix());
        let dir = cache_dir().unwrap();
        fs::create_dir_all(&dir).unwrap();
        let stale = CacheEntry {
            key: key.clone(),
            created_at: now_unix().saturating_sub(10_000),
            ttl_seconds: 1,
            compressed: false,
            size_bytes: 2,
            hits: 0,
            body: "{}".to_string(),
        };
        let path = dir.join(key_to_filename(&key));
        fs::write(&path, serde_json::to_string(&stale).unwrap()).unwrap();
        assert!(get_http_entry(&key).unwrap().is_none());
        delete_entry_file(&dir, &key);
    }
}
