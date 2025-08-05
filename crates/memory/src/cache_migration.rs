use anyhow::Result;
use std::path::Path;
use tracing::{info, warn};

use crate::{EmbeddingCache, EmbeddingCacheLRU, CacheConfig};

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

    // Open old cache
    let old_cache = EmbeddingCache::new(&old_cache_path)?;
    
    // Create new LRU cache
    let _new_cache = EmbeddingCacheLRU::new(&new_cache_path, config)?;

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
    let max_size_bytes = if available_memory_mb > 16384 { // > 16GB
        4_294_967_296 // 4GB cache
    } else if available_memory_mb > 8192 { // > 8GB
        2_147_483_648 // 2GB cache
    } else if available_memory_mb > 4096 { // > 4GB
        1_073_741_824 // 1GB cache
    } else {
        536_870_912 // 512MB cache
    };

    CacheConfig {
        max_size_bytes,
        max_entries: max_size_bytes / 10240, // Assume ~10KB per embedding
        ttl_seconds: Some(86400 * 30), // 30 days
        eviction_batch_size: 100,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_recommendation() {
        // Test different memory sizes
        let config_16gb = recommend_cache_config(20000); // > 16GB
        assert_eq!(config_16gb.max_size_bytes, 4_294_967_296);

        let config_8gb = recommend_cache_config(10000); // > 8GB but < 16GB
        assert_eq!(config_8gb.max_size_bytes, 2_147_483_648);

        let config_4gb = recommend_cache_config(5000); // > 4GB but < 8GB
        assert_eq!(config_4gb.max_size_bytes, 1_073_741_824);

        let config_2gb = recommend_cache_config(2048);
        assert_eq!(config_2gb.max_size_bytes, 536_870_912);
    }
}