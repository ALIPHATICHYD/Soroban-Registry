use moka::future::Cache as MokaCache;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;

/// Cache configuration options
#[derive(Clone, Debug)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_capacity: u64,
    pub redis_enabled: bool,
    pub redis_url: Option<String>,
    pub contracts_ttl: u64,
    /// TTL for contract metadata in Redis (seconds). Default: 3600 (1 hour)
    pub metadata_ttl_secs: u64,
    /// TTL for ABI data in Redis (seconds). Default: 86400 (24 hours)
    pub abi_ttl_secs: u64,
    /// TTL for search results in Redis (seconds). Default: 300 (5 minutes)
    pub search_ttl_secs: u64,
    /// TTL for stats/analytics in Redis (seconds). Default: 300 (5 minutes)
    pub stats_ttl_secs: u64,
    /// Optional override for the verification cache max capacity (weighted bytes).
    /// When unset, defaults to `max_capacity`, preserving prior behavior.
    pub verification_max_capacity: Option<u64>,
    /// Optional override for the verification cache time-to-live, in seconds.
    /// When unset, defaults to 7 days (604800), preserving prior behavior.
    pub verification_ttl_secs: Option<u64>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_capacity: 10_000,
            redis_enabled: false,
            redis_url: None,
            contracts_ttl: 3600,
            metadata_ttl_secs: 3600,
            abi_ttl_secs: 86400,
            search_ttl_secs: 300,
            stats_ttl_secs: 300,
            verification_max_capacity: None,
            verification_ttl_secs: None,
        }
    }
}

impl CacheConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(enabled_str) = std::env::var("CACHE_ENABLED") {
            config.enabled = enabled_str.to_lowercase() == "true";
        }

        if let Ok(capacity_str) = std::env::var("CACHE_MAX_CAPACITY") {
            if let Ok(capacity) = capacity_str.parse::<u64>() {
                config.max_capacity = capacity;
            }
        }

        if let Ok(redis_enabled_str) = std::env::var("REDIS_ENABLED") {
            config.redis_enabled = redis_enabled_str.to_lowercase() == "true";
        }

        if let Ok(ttl_str) = std::env::var("CONTRACTS_CACHE_TTL") {
            if let Ok(ttl) = ttl_str.parse::<u64>() {
                config.contracts_ttl = ttl;
                config.metadata_ttl_secs = ttl;
            }
        }

        if let Ok(ttl_str) = std::env::var("ABI_CACHE_TTL") {
            if let Ok(ttl) = ttl_str.parse::<u64>() {
                config.abi_ttl_secs = ttl;
            }
        }

        if let Ok(ttl_str) = std::env::var("SEARCH_CACHE_TTL") {
            if let Ok(ttl) = ttl_str.parse::<u64>() {
                config.search_ttl_secs = ttl;
            }
        }

        if let Ok(ttl_str) = std::env::var("STATS_CACHE_TTL") {
            if let Ok(ttl) = ttl_str.parse::<u64>() {
                config.stats_ttl_secs = ttl;
            }
        }

        if let Ok(cap_str) = std::env::var("VERIFICATION_CACHE_MAX_CAPACITY") {
            if let Ok(cap) = cap_str.parse::<u64>() {
                config.verification_max_capacity = Some(cap);
            }
        }

        if let Ok(ttl_str) = std::env::var("VERIFICATION_CACHE_TTL") {
            if let Ok(ttl) = ttl_str.parse::<u64>() {
                config.verification_ttl_secs = Some(ttl);
            }
        }

        config.redis_url = std::env::var("REDIS_URL").ok();

        tracing::info!(
            "Cache config loaded: enabled={}, capacity={}, redis_enabled={}",
            config.enabled,
            config.max_capacity,
            config.redis_enabled
        );

        config
    }
}

pub struct CacheLayer {
    pub abi_cache: MokaCache<String, String>,
    pub verification_cache: MokaCache<String, String>,
    pub generic_cache: MokaCache<String, String>,
    pub contracts_cache: MokaCache<String, String>,
    pub contract_access_cache: MokaCache<String, bool>,
    config: CacheConfig,
    pub redis_cm: Option<ConnectionManager>,
}

impl CacheLayer {
    pub async fn new(config: CacheConfig) -> Self {
        // 24-hour TTL for ABI, max size configurable default 10GB but we use the config max_capacity
        let abi_cache = MokaCache::builder()
            .max_capacity(config.max_capacity)
            .weigher(|_k, v: &String| -> u32 { v.len().try_into().unwrap_or(u32::MAX) })
            .time_to_live(Duration::from_secs(24 * 3600))
            .build();

        // Verification result cache, keyed by bytecode_hash.
        // Capacity and TTL are operationally configurable (VERIFICATION_CACHE_MAX_CAPACITY,
        // VERIFICATION_CACHE_TTL); the defaults preserve prior behavior — capacity follows
        // `max_capacity` and the TTL stays at 7 days.
        let verification_max_capacity = config
            .verification_max_capacity
            .unwrap_or(config.max_capacity);
        let verification_ttl_secs = config.verification_ttl_secs.unwrap_or(7 * 24 * 3600);
        let verification_cache = MokaCache::builder()
            .max_capacity(verification_max_capacity)
            .weigher(|_k, v: &String| -> u32 { v.len().try_into().unwrap_or(u32::MAX) })
            .time_to_live(Duration::from_secs(verification_ttl_secs))
            .build();

        // Generic cache for namespace-keyed entries (e.g., contract graphs)
        // Default 1-hour TTL, configurable per-entry
        let generic_cache = MokaCache::builder()
            .max_capacity(config.max_capacity)
            .weigher(|_k, v: &String| -> u32 { v.len().try_into().unwrap_or(u32::MAX) })
            .time_to_live(Duration::from_secs(3600))
            .support_invalidation_closures()
            .build();

        let contracts_cache = MokaCache::builder()
            .max_capacity(config.max_capacity)
            .weigher(|_k, v: &String| -> u32 { v.len().try_into().unwrap_or(u32::MAX) })
            .time_to_live(Duration::from_secs(config.metadata_ttl_secs))
            .build();

        let contract_access_cache = MokaCache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(Duration::from_secs(60))
            .build();

        let redis_cm = if config.redis_enabled {
            if let Some(url) = &config.redis_url {
                match redis::Client::open(url.as_str()) {
                    Ok(client) => match client.get_connection_manager().await {
                        Ok(cm) => Some(cm),
                        Err(e) => {
                            tracing::error!("Failed to get Redis connection manager: {}", e);
                            None
                        }
                    },
                    Err(e) => {
                        tracing::error!("Failed to open Redis client: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        Self {
            abi_cache,
            verification_cache,
            generic_cache,
            contracts_cache,
            contract_access_cache,
            redis_cm,
            config,
        }
    }

    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    pub async fn get_abi(&self, contract_id: &str, bypass_cache: bool) -> Option<String> {
        if !self.config.enabled || bypass_cache {
            if bypass_cache {
                tracing::debug!("Bypassing cache for contract_id: {}", contract_id);
            }
            return None;
        }

        // L1: Moka
        if let Some(abi) = self.abi_cache.get(contract_id).await {
            crate::metrics::ABI_CACHE_HITS.inc();
            return Some(abi);
        }

        // L2: Redis
        if let Some(cm) = &self.redis_cm {
            let key = format!("abi:{}", contract_id);
            let mut conn = cm.clone();
            match conn.get::<_, Option<String>>(&key).await {
                Ok(Some(val)) => {
                    crate::metrics::REDIS_CACHE_HITS.inc();
                    crate::metrics::ABI_CACHE_HITS.inc();
                    // Backfill L1
                    self.abi_cache
                        .insert(contract_id.to_string(), val.clone())
                        .await;
                    return Some(val);
                }
                Ok(None) => {
                    crate::metrics::REDIS_CACHE_MISSES.inc();
                }
                Err(e) => {
                    tracing::warn!("Redis get_abi error: {}", e);
                }
            }
        }

        crate::metrics::ABI_CACHE_MISSES.inc();
        None
    }

    pub async fn put_abi(&self, contract_id: &str, abi: String) {
        if !self.config.enabled {
            return;
        }

        // L1
        self.abi_cache
            .insert(contract_id.to_string(), abi.clone())
            .await;

        // L2: Redis with 24h TTL
        if let Some(cm) = &self.redis_cm {
            let key = format!("abi:{}", contract_id);
            let mut conn = cm.clone();
            if let Err(e) = conn
                .set_ex::<_, _, ()>(&key, &abi, self.config.abi_ttl_secs as u64)
                .await
            {
                tracing::warn!("Redis put_abi error: {}", e);
            }
        }
    }

    pub async fn invalidate_abi(&self, contract_id: &str) {
        if !self.config.enabled {
            return;
        }

        self.abi_cache.invalidate(contract_id).await;

        if let Some(cm) = &self.redis_cm {
            let key = format!("abi:{}", contract_id);
            let mut conn = cm.clone();
            if let Err(e) = conn.del::<_, ()>(&key).await {
                tracing::warn!("Redis invalidate_abi error: {}", e);
            }
        }
    }

    pub async fn get_verification(&self, bytecode_hash: &str) -> Option<String> {
        if !self.config.enabled {
            return None;
        }
        let result = self.verification_cache.get(bytecode_hash).await;
        if result.is_some() {
            crate::metrics::VERIFICATION_CACHE_HITS.inc();
        } else {
            crate::metrics::VERIFICATION_CACHE_MISSES.inc();
        }
        result
    }

    pub async fn put_verification(&self, bytecode_hash: &str, result: String) {
        if !self.config.enabled {
            return;
        }
        self.verification_cache
            .insert(bytecode_hash.to_string(), result)
            .await;
    }

    pub async fn invalidate_verification(&self, bytecode_hash: &str) {
        if !self.config.enabled {
            return;
        }
        self.verification_cache.invalidate(bytecode_hash).await;
    }

    // Generic cache methods with namespace support
    pub async fn get(&self, ns: &str, key: &str) -> (Option<String>, bool) {
        if !self.config.enabled {
            return (None, false);
        }

        let namespaced_key = format!("{}:{}", ns, key);
        let result = self.generic_cache.get(&namespaced_key).await;
        let hit = result.is_some();

        if hit {
            crate::metrics::CACHE_HITS.inc();
        } else {
            crate::metrics::CACHE_MISSES.inc();
        }

        (result, hit)
    }

    pub async fn put(&self, ns: &str, key: &str, value: String, _ttl: Option<Duration>) {
        if !self.config.enabled {
            return;
        }

        let namespaced_key = format!("{}:{}", ns, key);

        // Note: moka doesn't support per-entry TTL easily, so we use the cache-wide TTL
        // For custom TTL support, we'd need to use entry_by_ref with expiration policy
        // For now, we'll insert with the default TTL configured for generic_cache
        self.generic_cache.insert(namespaced_key, value).await;
    }

    pub async fn invalidate(&self, ns: &str, key: &str) {
        if !self.config.enabled {
            return;
        }

        let namespaced_key = format!("{}:{}", ns, key);
        self.generic_cache.invalidate(&namespaced_key).await;
    }

    pub async fn should_refresh_contract_access(&self, contract_id: &str) -> bool {
        if !self.config.enabled {
            return true;
        }

        if self.contract_access_cache.get(contract_id).await.is_some() {
            return false;
        }

        self.contract_access_cache
            .insert(contract_id.to_string(), true)
            .await;
        true
    }

    pub async fn get_contracts(&self, key: &str) -> Option<String> {
        if !self.config.enabled {
            return None;
        }

        // L1: Moka
        if let Some(val) = self.contracts_cache.get(key).await {
            crate::metrics::CONTRACTS_CACHE_HITS.inc();
            return Some(val);
        }

        // L2: Redis
        if let Some(cm) = &self.redis_cm {
            let rkey = format!("contracts:{}", key);
            let mut conn = cm.clone();
            match conn.get::<_, Option<String>>(&rkey).await {
                Ok(Some(val)) => {
                    crate::metrics::REDIS_CACHE_HITS.inc();
                    crate::metrics::CONTRACTS_CACHE_HITS.inc();
                    self.contracts_cache
                        .insert(key.to_string(), val.clone())
                        .await;
                    return Some(val);
                }
                Ok(None) => {
                    crate::metrics::REDIS_CACHE_MISSES.inc();
                }
                Err(e) => {
                    tracing::warn!("Redis get_contracts error: {}", e);
                }
            }
        }

        crate::metrics::CONTRACTS_CACHE_MISSES.inc();
        None
    }

    pub async fn put_contracts(&self, key: String, value: String) {
        if !self.config.enabled {
            return;
        }

        // L1
        self.contracts_cache
            .insert(key.clone(), value.clone())
            .await;

        // L2: Redis with metadata TTL (1h)
        if let Some(cm) = &self.redis_cm {
            let rkey = format!("contracts:{}", key);
            let mut conn = cm.clone();
            if let Err(e) = conn
                .set_ex::<_, _, ()>(&rkey, &value, self.config.metadata_ttl_secs as u64)
                .await
            {
                tracing::warn!("Redis put_contracts error: {}", e);
            }
        }
    }

    pub async fn invalidate_contracts(&self) {
        if !self.config.enabled {
            return;
        }

        self.contracts_cache.invalidate_all();
        // Also clear per-contract metadata from the generic cache
        self.generic_cache
            .invalidate_entries_if(|k, _| k.starts_with("meta:"))
            .ok();

        // Flush the contracts:* and meta:* namespaces from Redis
        if let Some(cm) = &self.redis_cm {
            let mut conn = cm.clone();
            for pattern in &["contracts:*", "meta:*"] {
                match conn.keys::<_, Vec<String>>(pattern).await {
                    Ok(keys) if !keys.is_empty() => {
                        if let Err(e) = conn.del::<_, ()>(keys).await {
                            tracing::warn!(
                                "Redis invalidate_contracts del error ({}): {}",
                                pattern,
                                e
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Redis invalidate_contracts scan error ({}): {}", pattern, e)
                    }
                    _ => {}
                }
            }
        }
    }

    /// Get a single contract's metadata by its contract_id string.
    pub async fn get_contract_meta(&self, contract_id: &str) -> Option<String> {
        if !self.config.enabled {
            return None;
        }

        let ns_key = format!("meta:{}", contract_id);

        // L1
        if let Some(val) = self.generic_cache.get(&ns_key).await {
            crate::metrics::CONTRACTS_CACHE_HITS.inc();
            return Some(val);
        }

        // L2: Redis
        if let Some(cm) = &self.redis_cm {
            let mut conn = cm.clone();
            match conn.get::<_, Option<String>>(&ns_key).await {
                Ok(Some(val)) => {
                    crate::metrics::REDIS_CACHE_HITS.inc();
                    crate::metrics::CONTRACTS_CACHE_HITS.inc();
                    self.generic_cache.insert(ns_key, val.clone()).await;
                    return Some(val);
                }
                Ok(None) => {
                    crate::metrics::REDIS_CACHE_MISSES.inc();
                }
                Err(e) => {
                    tracing::warn!("Redis get_contract_meta error: {}", e);
                }
            }
        }

        crate::metrics::CONTRACTS_CACHE_MISSES.inc();
        None
    }

    pub async fn put_contract_meta(&self, contract_id: &str, value: String) {
        if !self.config.enabled {
            return;
        }

        let ns_key = format!("meta:{}", contract_id);
        self.generic_cache
            .insert(ns_key.clone(), value.clone())
            .await;

        if let Some(cm) = &self.redis_cm {
            let mut conn = cm.clone();
            if let Err(e) = conn
                .set_ex::<_, _, ()>(&ns_key, &value, self.config.metadata_ttl_secs as u64)
                .await
            {
                tracing::warn!("Redis put_contract_meta error: {}", e);
            }
        }
    }

    pub async fn invalidate_contract_meta(&self, contract_id: &str) {
        if !self.config.enabled {
            return;
        }

        let ns_key = format!("meta:{}", contract_id);
        self.generic_cache.invalidate(&ns_key).await;

        if let Some(cm) = &self.redis_cm {
            let mut conn = cm.clone();
            if let Err(e) = conn.del::<_, ()>(&ns_key).await {
                tracing::warn!("Redis invalidate_contract_meta error: {}", e);
            }
        }
    }

    /// Get a cached vulnerability assessment. These use the 24h ABI cache
    /// because vulnerability assessments have the same TTL requirement.
    pub async fn get_vulnerability_assessment(&self, contract_id: &str) -> Option<String> {
        if !self.config.enabled {
            return None;
        }

        let ns_key = format!("vulnerability_assessment:{}", contract_id);

        if let Some(val) = self.abi_cache.get(&ns_key).await {
            crate::metrics::CACHE_HITS.inc();
            return Some(val);
        }

        if let Some(cm) = &self.redis_cm {
            let mut conn = cm.clone();
            match conn.get::<_, Option<String>>(&ns_key).await {
                Ok(Some(val)) => {
                    crate::metrics::REDIS_CACHE_HITS.inc();
                    crate::metrics::CACHE_HITS.inc();
                    self.abi_cache.insert(ns_key, val.clone()).await;
                    return Some(val);
                }
                Ok(None) => {
                    crate::metrics::REDIS_CACHE_MISSES.inc();
                }
                Err(e) => {
                    tracing::warn!("Redis get_vulnerability_assessment error: {}", e);
                }
            }
        }

        crate::metrics::CACHE_MISSES.inc();
        None
    }

    pub async fn put_vulnerability_assessment(&self, contract_id: &str, value: String) {
        if !self.config.enabled {
            return;
        }

        let ns_key = format!("vulnerability_assessment:{}", contract_id);
        self.abi_cache.insert(ns_key.clone(), value.clone()).await;

        if let Some(cm) = &self.redis_cm {
            let mut conn = cm.clone();
            if let Err(e) = conn.set_ex::<_, _, ()>(&ns_key, &value, 24 * 3600).await {
                tracing::warn!("Redis put_vulnerability_assessment error: {}", e);
            }
        }
    }

    /// Cache search results keyed by a query fingerprint (SHA-256 hex of the serialized params).
    pub async fn get_search(&self, fingerprint: &str) -> Option<String> {
        if !self.config.enabled {
            return None;
        }

        let ns_key = format!("search:{}", fingerprint);

        if let Some(val) = self.generic_cache.get(&ns_key).await {
            crate::metrics::CACHE_HITS.inc();
            return Some(val);
        }

        if let Some(cm) = &self.redis_cm {
            let mut conn = cm.clone();
            match conn.get::<_, Option<String>>(&ns_key).await {
                Ok(Some(val)) => {
                    crate::metrics::REDIS_CACHE_HITS.inc();
                    crate::metrics::CACHE_HITS.inc();
                    self.generic_cache.insert(ns_key, val.clone()).await;
                    return Some(val);
                }
                Ok(None) => {
                    crate::metrics::REDIS_CACHE_MISSES.inc();
                }
                Err(e) => {
                    tracing::warn!("Redis get_search error: {}", e);
                }
            }
        }

        crate::metrics::CACHE_MISSES.inc();
        None
    }

    pub async fn put_search(&self, fingerprint: &str, value: String) {
        if !self.config.enabled {
            return;
        }

        let ns_key = format!("search:{}", fingerprint);
        self.generic_cache
            .insert(ns_key.clone(), value.clone())
            .await;

        if let Some(cm) = &self.redis_cm {
            let mut conn = cm.clone();
            if let Err(e) = conn
                .set_ex::<_, _, ()>(&ns_key, &value, self.config.search_ttl_secs as u64)
                .await
            {
                tracing::warn!("Redis put_search error: {}", e);
            }
        }
    }

    /// Cache stats/analytics results.
    pub async fn get_stats(&self, key: &str) -> Option<String> {
        if !self.config.enabled {
            return None;
        }

        let ns_key = format!("stats:{}", key);

        if let Some(val) = self.generic_cache.get(&ns_key).await {
            crate::metrics::CACHE_HITS.inc();
            return Some(val);
        }

        if let Some(cm) = &self.redis_cm {
            let mut conn = cm.clone();
            match conn.get::<_, Option<String>>(&ns_key).await {
                Ok(Some(val)) => {
                    crate::metrics::REDIS_CACHE_HITS.inc();
                    crate::metrics::CACHE_HITS.inc();
                    self.generic_cache.insert(ns_key, val.clone()).await;
                    return Some(val);
                }
                Ok(None) => {
                    crate::metrics::REDIS_CACHE_MISSES.inc();
                }
                Err(e) => {
                    tracing::warn!("Redis get_stats error: {}", e);
                }
            }
        }

        crate::metrics::CACHE_MISSES.inc();
        None
    }

    pub async fn put_stats(&self, key: &str, value: String) {
        if !self.config.enabled {
            return;
        }

        let ns_key = format!("stats:{}", key);
        self.generic_cache
            .insert(ns_key.clone(), value.clone())
            .await;

        if let Some(cm) = &self.redis_cm {
            let mut conn = cm.clone();
            if let Err(e) = conn
                .set_ex::<_, _, ()>(&ns_key, &value, self.config.stats_ttl_secs as u64)
                .await
            {
                tracing::warn!("Redis put_stats error: {}", e);
            }
        }
    }

    pub async fn ping(&self) -> anyhow::Result<()> {
        if self.config.redis_enabled {
            if let Some(cm) = &self.redis_cm {
                let mut conn = cm.clone();
                let _: () = redis::cmd("PING").query_async(&mut conn).await?;
            }
        }
        Ok(())
    }

    /// Starts an asynchronous startup warmup task querying the top 1000 contracts
    pub fn warm_up(self: Arc<Self>, pool: PgPool) {
        if !self.config.enabled {
            return;
        }
        tokio::spawn(async move {
            tracing::info!("Starting startup cache warmup...");
            let top_contracts: Vec<(uuid::Uuid, String, Option<String>)> = sqlx::query_as(
                r#"
                SELECT c.id, c.contract_id, c.wasm_hash
                FROM contracts c
                LEFT JOIN contract_interactions ci ON c.id = ci.contract_id
                GROUP BY c.id
                ORDER BY COUNT(ci.id) DESC
                LIMIT 1000
                "#,
            )
            .fetch_all(&pool)
            .await
            .unwrap_or_default();

            let warmed = top_contracts.len();

            for (id, contract_id, wasm_hash) in top_contracts {
                // Warm ABI cache (24h TTL)
                if let Ok(Some(abi)) = sqlx::query_scalar::<_, serde_json::Value>(
                    "SELECT abi FROM contract_abis WHERE contract_id = $1 ORDER BY created_at DESC LIMIT 1"
                )
                .bind(id)
                .fetch_optional(&pool).await {
                    self.put_abi(&contract_id, abi.to_string()).await;
                }

                // Warm contract metadata cache (1h TTL)
                if let Ok(Some(contract_json)) = sqlx::query_scalar::<_, serde_json::Value>(
                    "SELECT row_to_json(c) FROM contracts c WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(&pool)
                .await
                {
                    self.put_contract_meta(&contract_id, contract_json.to_string())
                        .await;
                    self.put_contract_meta(&id.to_string(), contract_json.to_string())
                        .await;
                }

                if let Some(w_hash) = wasm_hash {
                    if let Ok(Some(ver_res)) = sqlx::query_scalar::<_, String>(
                        "SELECT status::text FROM formal_verification_results LIMIT 1",
                    )
                    .fetch_optional(&pool)
                    .await
                    {
                        self.verification_cache
                            .insert(w_hash.clone(), ver_res)
                            .await;
                    }
                }
            }
            tracing::info!("Completed startup cache warmup ({} contracts).", warmed);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_abi_cache() {
        let config = CacheConfig {
            enabled: true,
            max_capacity: 100,
            ..Default::default()
        };
        let cache = CacheLayer::new(config).await;

        cache.put_abi("contract_1", "abi_json_1".to_string()).await;

        let val = cache.get_abi("contract_1", false).await;
        assert_eq!(val, Some("abi_json_1".to_string()));

        cache.invalidate_abi("contract_1").await;

        let val2 = cache.get_abi("contract_1", false).await;
        assert!(val2.is_none());
    }

    #[tokio::test]
    async fn test_verification_cache() {
        let config = CacheConfig {
            enabled: true,
            max_capacity: 100,
            ..Default::default()
        };
        let cache = CacheLayer::new(config).await;

        cache
            .put_verification("hash_1", "result_1".to_string())
            .await;

        let val = cache.get_verification("hash_1").await;
        assert_eq!(val, Some("result_1".to_string()));

        cache.invalidate_verification("hash_1").await;

        let val2 = cache.get_verification("hash_1").await;
        assert!(val2.is_none());
    }

    #[tokio::test]
    async fn test_verification_cache_honors_configured_capacity_and_ttl() {
        // Explicit overrides are reflected in the resolved config, and the
        // cache serves hits within the configured TTL window.
        let config = CacheConfig {
            enabled: true,
            max_capacity: 100,
            verification_max_capacity: Some(42),
            verification_ttl_secs: Some(3600),
            ..Default::default()
        };
        assert_eq!(config.verification_max_capacity, Some(42));
        assert_eq!(config.verification_ttl_secs, Some(3600));

        let cache = CacheLayer::new(config).await;
        cache
            .put_verification("hash_hit", "result_hit".to_string())
            .await;
        assert_eq!(
            cache.get_verification("hash_hit").await,
            Some("result_hit".to_string())
        );
    }

    #[tokio::test]
    async fn test_verification_cache_defaults_preserve_previous_behavior() {
        // With no overrides the verification cache keeps the historical
        // defaults (capacity follows `max_capacity`, 7-day TTL).
        let config = CacheConfig {
            enabled: true,
            max_capacity: 100,
            ..Default::default()
        };
        assert_eq!(config.verification_max_capacity, None);
        assert_eq!(config.verification_ttl_secs, None);

        let cache = CacheLayer::new(config).await;
        cache
            .put_verification("hash_default", "result_default".to_string())
            .await;
        assert_eq!(
            cache.get_verification("hash_default").await,
            Some("result_default".to_string())
        );
    }

    #[tokio::test]
    async fn test_verification_cache_evicts_after_configured_ttl() {
        // A short configured TTL causes entries to expire.
        let config = CacheConfig {
            enabled: true,
            max_capacity: 100,
            verification_ttl_secs: Some(1),
            ..Default::default()
        };
        let cache = CacheLayer::new(config).await;
        cache
            .put_verification("hash_expire", "result_expire".to_string())
            .await;
        assert_eq!(
            cache.get_verification("hash_expire").await,
            Some("result_expire".to_string())
        );

        tokio::time::sleep(Duration::from_millis(1_100)).await;
        cache.verification_cache.run_pending_tasks().await;
        assert!(cache.get_verification("hash_expire").await.is_none());
    }

    #[tokio::test]
    async fn test_disabled_cache() {
        let config = CacheConfig {
            enabled: false,
            max_capacity: 100,
            ..Default::default()
        };
        let cache = CacheLayer::new(config).await;

        cache.put_abi("c1", "v1".to_string()).await;
        let val = cache.get_abi("c1", false).await;
        assert!(val.is_none());

        cache.put_verification("h1", "v1".to_string()).await;
        let val2 = cache.get_verification("h1").await;
        assert!(val2.is_none());
    }

    #[tokio::test]
    async fn test_generic_cache() {
        let config = CacheConfig {
            enabled: true,
            max_capacity: 100,
            ..Default::default()
        };
        let cache = CacheLayer::new(config).await;

        // Test put and get
        cache
            .put("system", "dependency_graph", "graph_data".to_string(), None)
            .await;

        let (val, hit) = cache.get("system", "dependency_graph").await;
        assert_eq!(val, Some("graph_data".to_string()));
        assert!(hit);

        // Test cache miss
        let (val2, hit2) = cache.get("system", "nonexistent").await;
        assert!(val2.is_none());
        assert!(!hit2);

        // Test invalidate
        cache.invalidate("system", "dependency_graph").await;
        let (val3, hit3) = cache.get("system", "dependency_graph").await;
        assert!(val3.is_none());
        assert!(!hit3);
    }

    #[tokio::test]
    async fn test_generic_cache_namespace_isolation() {
        let config = CacheConfig {
            enabled: true,
            max_capacity: 100,
            ..Default::default()
        };
        let cache = CacheLayer::new(config).await;

        // Put same key in different namespaces
        cache
            .put("ns1", "key1", "value_ns1".to_string(), None)
            .await;
        cache
            .put("ns2", "key1", "value_ns2".to_string(), None)
            .await;

        // Verify namespace isolation
        let (val1, _) = cache.get("ns1", "key1").await;
        let (val2, _) = cache.get("ns2", "key1").await;

        assert_eq!(val1, Some("value_ns1".to_string()));
        assert_eq!(val2, Some("value_ns2".to_string()));

        // Invalidate one namespace shouldn't affect the other
        cache.invalidate("ns1", "key1").await;
        let (val1_after, _) = cache.get("ns1", "key1").await;
        let (val2_after, _) = cache.get("ns2", "key1").await;

        assert!(val1_after.is_none());
        assert_eq!(val2_after, Some("value_ns2".to_string()));
    }

    #[tokio::test]
    async fn test_generic_cache_disabled() {
        let config = CacheConfig {
            enabled: false,
            max_capacity: 100,
            ..Default::default()
        };
        let cache = CacheLayer::new(config).await;

        cache
            .put("system", "key1", "value1".to_string(), None)
            .await;
        let (val, hit) = cache.get("system", "key1").await;

        assert!(val.is_none());
        assert!(!hit);
    }

    #[tokio::test]
    async fn test_contract_access_refresh_is_debounced() {
        let cache = CacheLayer::new(CacheConfig {
            enabled: true,
            max_capacity: 100,
            redis_enabled: false,
            redis_url: None,
            ..Default::default()
        })
        .await;

        assert!(cache.should_refresh_contract_access("contract-1").await);
        assert!(!cache.should_refresh_contract_access("contract-1").await);
        assert!(cache.should_refresh_contract_access("contract-2").await);
    }
}
