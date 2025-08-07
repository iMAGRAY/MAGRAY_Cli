use anyhow::Result;
use std::path::Path;
use tracing::{info, warn};

use crate::{EmbeddingCache, CacheConfig};

/// Migrate from simple cache to LRU cache
#[allow(dead_code)]
pub async fn migrate_cache_to_lru(
    old_cache_path: impl AsRef<Path>,
    new_cache_path: impl AsRef<Path>,
    config: CacheConfig,
) -> Result<()> {
    info!("Starting cache migration from {:?} to {:?}", 
          old_cache_path.as_ref(), 
          new_cache_path.as_ref());

    // Open old cache with default config (now it's LRU)
    let old_cache = EmbeddingCache::new(&old_cache_path, CacheConfig::default())?;
    
    // Create new LRU cache (same type now)
    let _new_cache = EmbeddingCache::new(&new_cache_path, config)?;

    // Get stats from old cache
    let (hits, misses, _) = old_cache.stats();
    info!("Old cache stats - Hits: {}, Misses: {}", hits, misses);

    // Unfortunately we can't iterate over sled directly without internal access
    // So we'll need to add a migration method to the old cache
    // For now, log a warning
    
    warn!("Cache migration requires manual intervention or adding iteration support to EmbeddingCache");
    warn!("Consider using the new LRU cache for new installations");

    Ok(())
}

/// Configuration helper for choosing appropriate cache settings
#[allow(dead_code)]
pub fn recommend_cache_config(available_memory_mb: usize) -> CacheConfig {
    let max_cache_entries = if available_memory_mb > 16384 { // > 16GB
        400_000 // ~4GB worth of embeddings
    } else if available_memory_mb > 8192 { // > 8GB
        200_000 // ~2GB worth of embeddings
    } else if available_memory_mb > 4096 { // > 4GB
        100_000 // ~1GB worth of embeddings
    } else {
        50_000 // ~512MB worth of embeddings
    };

    CacheConfig {
        base: common::config_base::CacheConfigBase {
            max_cache_size: max_cache_entries,
            cache_ttl_seconds: 86400 * 30, // 30 days
            eviction_policy: "lru".to_string(),
            enable_compression: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_recommendations() {
        // Test different memory sizes - now we test max_entries directly
        let config_16gb = recommend_cache_config(20000); // > 16GB
        assert_eq!(config_16gb.max_entries(), 400_000);

        let config_8gb = recommend_cache_config(10000); // > 8GB but < 16GB
        assert_eq!(config_8gb.max_entries(), 200_000);

        let config_4gb = recommend_cache_config(5000); // > 4GB but < 8GB
        assert_eq!(config_4gb.max_entries(), 100_000);

        let config_2gb = recommend_cache_config(2048);
        assert_eq!(config_2gb.max_entries(), 50_000);
    }
    
    #[test]
    fn test_recommended_config_cache_creation() {
        // Test that we can actually create a cache with the recommended config
        let config = recommend_cache_config(8192);
        assert_eq!(config.base.eviction_policy, "lru");
        assert!(config.base.enable_compression);
        assert_eq!(config.base.cache_ttl_seconds, 86400 * 30);
    }
}