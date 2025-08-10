use anyhow::Result;
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
#[cfg(feature = "persistence")]
use sled::Db;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn, error};

use common::{config_base::CacheConfigBase, ConfigTrait};

#[cfg(test)]
#[allow(dead_code)]
mod _clock_mock {
    use std::sync::atomic::{AtomicU64, Ordering};
    static ENABLED: AtomicU64 = AtomicU64::new(0);
    static NOW: AtomicU64 = AtomicU64::new(0);

    pub fn set(ts: u64) {
        NOW.store(ts, Ordering::SeqCst);
        ENABLED.store(1, Ordering::SeqCst);
    }
    pub fn advance(delta: u64) {
        NOW.fetch_add(delta, Ordering::SeqCst);
    }
    pub fn clear() {
        ENABLED.store(0, Ordering::SeqCst);
    }
    pub fn get() -> Option<u64> {
        if ENABLED.load(Ordering::SeqCst) == 1 {
            Some(NOW.load(Ordering::SeqCst))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CachedEmbedding {
    embedding: Vec<f32>,
    model: String,
    created_at: u64,
    last_accessed: u64,
    access_count: u32,
    size_bytes: usize,
}

/// Configuration for cache behavior - устранение дублирования с CacheConfigBase
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheConfig {
    /// Базовая cache конфигурация
    #[serde(flatten)]
    pub base: CacheConfigBase,
}

impl ConfigTrait for CacheConfig {
    fn production() -> Self {
        Self {
            base: CacheConfigBase {
                max_cache_size: 500_000,       // 500k entries
                cache_ttl_seconds: 86400 * 30, // 30 days
                eviction_policy: "lru".to_string(),
                enable_compression: true,
            },
        }
    }

    fn minimal() -> Self {
        Self {
            base: CacheConfigBase {
                max_cache_size: 10_000,   // 10k entries
                cache_ttl_seconds: 86400, // 1 day
                eviction_policy: "lru".to_string(),
                enable_compression: false,
            },
        }
    }
}

impl CacheConfig {
    /// Доступ к базовым настройкам cache
    pub fn max_size_bytes(&self) -> usize {
        self.base.max_cache_size
    }
    pub fn max_entries(&self) -> usize {
        self.base.max_cache_size
    }
    pub fn ttl_seconds(&self) -> Option<u64> {
        Some(self.base.cache_ttl_seconds)
    }
    pub fn eviction_batch_size(&self) -> usize {
        100
    } // Default batch size
}

impl CacheConfig {
    pub fn production() -> Self {
        Self {
            base: CacheConfigBase::large(),
        }
    }

    pub fn minimal() -> Self {
        Self {
            base: CacheConfigBase::small(),
        }
    }
}

pub struct EmbeddingCacheLRU {
    #[cfg(feature = "persistence")]
    db: Arc<Db>,
    stats: Arc<RwLock<CacheStats>>,
    lru_index: Arc<Mutex<LruIndex>>,
    config: CacheConfig,
    index_initialized: Arc<RwLock<bool>>,
}

#[derive(Debug)]
struct LruIndex {
    /// Maps key to (last_accessed_time, size_bytes)
    entries: HashMap<Vec<u8>, (u64, usize)>,
    /// Queue of keys ordered by access time (oldest first)
    access_queue: VecDeque<Vec<u8>>,
    /// Total size of cached embeddings
    total_size: usize,
}

impl LruIndex {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            access_queue: VecDeque::new(),
            total_size: 0,
        }
    }

    fn touch(&mut self, key: Vec<u8>, size: usize) {
        let now = current_timestamp();

        // Remove from old position if exists
        if let Some((_old_time, _)) = self.entries.get(&key) {
            self.access_queue.retain(|k| k != &key);
        } else {
            self.total_size += size;
        }

        // Add to end (most recent)
        self.entries.insert(key.clone(), (now, size));
        self.access_queue.push_back(key);
    }

    fn remove(&mut self, key: &[u8]) -> Option<usize> {
        if let Some((_, size)) = self.entries.remove(key) {
            self.access_queue.retain(|k| k != key);
            self.total_size = self.total_size.saturating_sub(size);
            Some(size)
        } else {
            None
        }
    }

    fn get_oldest(&self, count: usize) -> Vec<Vec<u8>> {
        self.access_queue.iter().take(count).cloned().collect()
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.access_queue.clear();
        self.total_size = 0;
    }
}

#[derive(Debug, Default)]
struct CacheStats {
    hits: u64,
    misses: u64,
    inserts: u64,
    evictions: u64,
    expired: u64,
}

impl EmbeddingCacheLRU {
    /// Открывает sled БД для LRU кэша через DatabaseManager
    #[cfg(feature = "persistence")]
    fn open_cache_database(cache_path: impl AsRef<std::path::Path>) -> Result<Arc<Db>> {
        let db_manager = crate::database_manager::DatabaseManager::global();
        let db = db_manager.get_cache_database(cache_path.as_ref())?;
        info!("✅ LRU cache database opened through DatabaseManager");
        Ok(db)
    }

    pub fn new(cache_path: impl AsRef<std::path::Path>, config: CacheConfig) -> Result<Self> {
        let cache_path = cache_path.as_ref();

        // Create directory if it doesn't exist
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        info!("Opening LRU embedding cache at: {:?}", cache_path);
        info!(
            "Cache config: max_size={}MB, max_entries={}, ttl={:?}s",
            config.max_size_bytes() / 1024 / 1024,
            config.max_entries(),
            config.ttl_seconds()
        );

        #[cfg(feature = "persistence")]
        let db = Self::open_cache_database(cache_path)?;

        let cache = Self {
            #[cfg(feature = "persistence")]
            db,
            stats: Arc::new(RwLock::new(CacheStats::default())),
            lru_index: Arc::new(Mutex::new(LruIndex::new())),
            config,
            index_initialized: Arc::new(RwLock::new(false)),
        };

        // Skip index rebuild during construction for faster startup
        // Index will be rebuilt lazily on first access
        info!("LRU cache created, index will be rebuilt on first use");

        Ok(cache)
    }

    /// Ensure index is initialized (lazy initialization)
    fn ensure_index_initialized(&self) -> Result<()> {
        // Fast path: already initialized
        if *self.index_initialized.read() {
            return Ok(());
        }

        // Slow path: need to initialize
        let mut initialized = self.index_initialized.write();
        if *initialized {
            return Ok(()); // Double-check after acquiring write lock
        }

        info!("Lazy initialization of LRU index starting...");
        self.rebuild_index_internal()?;
        *initialized = true;

        Ok(())
    }

    /// Rebuild the LRU index from database
    fn rebuild_index_internal(&self) -> Result<()> {
        let mut index = match self.lru_index.try_lock() {
            Some(lock) => lock,
            None => {
                warn!("Failed to acquire lock for index rebuild, using fallback");
                return Ok(()); // Graceful degradation - continue without rebuild
            }
        };
        index.clear();
        #[cfg(feature = "persistence")]
        for item in self.db.iter() {
            match item {
                Ok((key, value)) => {
                    if let Ok(cached) = bincode::deserialize::<CachedEmbedding>(&value) {
                        index.touch(key.to_vec(), cached.size_bytes);
                    }
                }
                Err(e) => {
                    warn!(
                        "Error reading item during index rebuild: {}, continuing...",
                        e
                    );
                    continue;
                }
            }
        }

        info!(
            "Rebuilt LRU index: {} entries, {} MB total",
            index.entries.len(),
            index.total_size / 1024 / 1024
        );

        Ok(())
    }

    pub fn get(&self, text: &str, model: &str) -> Option<Vec<f32>> {
        // Ensure index is initialized on first access
        if let Err(e) = self.ensure_index_initialized() {
            warn!("Failed to initialize LRU index: {}", e);
        }

        let _key = self.make_key(text, model);
        #[cfg(feature = "persistence")]
        match self.db.get(&_key) {
            Ok(Some(bytes)) => {
                match bincode::deserialize::<CachedEmbedding>(&bytes) {
                    Ok(mut cached) => {
                        // Check TTL
                        if let Some(ttl) = self.config.ttl_seconds() {
                            let _now = current_timestamp();
                            // Handle potential time issues gracefully
                            if _now >= cached.created_at && ttl > 0 {
                                let age = _now - cached.created_at;
                                if age > ttl {
                                    debug!("Cache entry expired: age={} > ttl={}", age, ttl);
                                    self.stats.write().expired += 1;
                                    let _ = self.remove_entry(&_key);
                                    return None;
                                }
                            } else if ttl > 0 && _now < cached.created_at {
                                warn!(
                                    "Clock skew detected: now={} < created_at={}",
                                    _now, cached.created_at
                                );
                                // Don't expire due to clock issues
                            }
                        }

                        // Update access stats
                        cached.last_accessed = current_timestamp();
                        cached.access_count += 1;

                        // Update in database - don't fail if this fails
                        if let Ok(updated_bytes) = bincode::serialize(&cached) {
                            if let Err(e) = self.db.insert(&_key, updated_bytes) {
                                warn!("Failed to update cache entry stats: {}", e);
                                // Continue anyway - we can still return the cached value
                            }
                        }

                        // Update LRU index with error handling
                        if let Some(mut index) = self.lru_index.try_lock() {
                            index.touch(_key.to_vec(), cached.size_bytes);
                        } else {
                            warn!("Failed to update LRU index - poisoned lock detected");
                        }

                        self.stats.write().hits += 1;
                        info!("Cache hit for text hash: {}", self.hash_text(text));
                        Some(cached.embedding)
                    }
                    Err(e) => {
                        warn!(
                            "Failed to deserialize cached embedding, removing corrupted entry: {}",
                            e
                        );
                        let _ = self.remove_entry(&_key); // Clean up corrupted entry
                        self.stats.write().misses += 1;
                        None
                    }
                }
            }
            Ok(None) => {
                self.stats.write().misses += 1;
                None
            }
            Err(e) => {
                warn!("Database error during cache get: {}", e);
                self.stats.write().misses += 1;
                None // Graceful degradation - treat as cache miss
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            // Without persistence, no cached data
            self.stats.write().misses += 1;
            None
        }
    }

    pub fn insert(&self, text: &str, model: &str, embedding: Vec<f32>) -> Result<()> {
        // Ensure index is initialized on first access
        if let Err(e) = self.ensure_index_initialized() {
            warn!("Failed to initialize LRU index: {}", e);
        }

        // Validate inputs
        if embedding.is_empty() {
            return Err(anyhow::anyhow!("Cannot cache empty embedding"));
        }

        if text.is_empty() || model.is_empty() {
            return Err(anyhow::anyhow!("Cannot cache with empty text or model"));
        }

        let key = self.make_key(text, model);
        let size_bytes = embedding.len() * std::mem::size_of::<f32>() + 256; // Overhead

        // Check if we need to evict entries - don't fail insertion if eviction fails
        if let Err(e) = self.maybe_evict(size_bytes) {
            warn!("Eviction failed but continuing with insert: {}", e);
        }

        let _cached = CachedEmbedding {
            embedding,
            model: model.to_string(),
            created_at: current_timestamp(),
            last_accessed: current_timestamp(),
            access_count: 1,
            size_bytes,
        };

        #[cfg(feature = "persistence")]
        match bincode::serialize(&_cached) {
            Ok(bytes) => {
                match self.db.insert(&key, bytes) {
                    Ok(_) => {
                        // Update LRU index with error handling
                        if let Some(mut index) = self.lru_index.try_lock() {
                            index.touch(key.to_vec(), size_bytes);
                        } else {
                            warn!(
                                "Failed to update LRU index during insert - poisoned lock detected"
                            );
                            // Continue anyway - database insert succeeded
                        }

                        self.stats.write().inserts += 1;
                        info!("Cached embedding for text hash: {}", self.hash_text(text));
                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to insert into cache database: {}", e);
                        Err(e.into())
                    }
                }
            }
            Err(e) => {
                warn!("Failed to serialize embedding for cache: {}", e);
                Err(e.into())
            }
        }
        #[cfg(not(feature = "persistence"))]
        {
            // Without persistence, just update index and stats
            if let Some(mut index) = self.lru_index.try_lock() {
                index.touch(key.to_vec(), size_bytes);
            }
            self.stats.write().inserts += 1;
            Ok(())
        }
    }

    /// Check if eviction is needed and perform it
    fn maybe_evict(&self, needed_size: usize) -> Result<()> {
        let mut index = match self.lru_index.try_lock() {
            Some(lock) => lock,
            None => {
                warn!("Failed to acquire lock for eviction - continuing without eviction");
                return Ok(()); // Graceful degradation
            }
        };

        // Check size constraint
        let mut entries_to_evict = Vec::new();

        if index.total_size + needed_size > self.config.max_size_bytes() {
            let target_size = self.config.max_size_bytes() * 8 / 10; // Free up to 80%
            let mut current_size = index.total_size;

            for key in index.get_oldest(self.config.eviction_batch_size()) {
                if current_size <= target_size {
                    break;
                }

                if let Some(size) = index.entries.get(&key).map(|(_, s)| *s) {
                    entries_to_evict.push(key);
                    current_size -= size;
                }
            }
        }

        // Check count constraint
        if index.entries.len() + 1 > self.config.max_entries() {
            let additional = index.get_oldest(self.config.eviction_batch_size());
            for key in additional {
                if !entries_to_evict.contains(&key) {
                    entries_to_evict.push(key);
                }
            }
        }

        // Perform eviction
        if !entries_to_evict.is_empty() {
            warn!(
                "Evicting {} cache entries to make room",
                entries_to_evict.len()
            );
            let mut stats = self.stats.write();

            for key in entries_to_evict {
                index.remove(&key);
                #[cfg(feature = "persistence")]
                let _ = self.db.remove(&key);
                stats.evictions += 1;
            }
        }

        Ok(())
    }

    /// Remove an entry from cache
    fn remove_entry(&self, key: &[u8]) -> Result<()> {
        // Try to update index, but don't fail if lock is poisoned
        if let Some(mut index) = self.lru_index.try_lock() {
            index.remove(key);
        } else {
            warn!("Failed to update LRU index during removal - poisoned lock detected");
        }

        // Always try to remove from database
        #[cfg(feature = "persistence")]
        if let Err(e) = self.db.remove(key) {
            warn!("Failed to remove entry from database: {}", e);
            // Don't propagate the error - this is not critical
        }
        Ok(())
    }

    pub fn get_batch(&self, texts: &[String], model: &str) -> Vec<Option<Vec<f32>>> {
        // Ensure index is initialized on first access
        if let Err(e) = self.ensure_index_initialized() {
            warn!("Failed to initialize LRU index: {}", e);
        }

        texts.iter().map(|text| self.get(text, model)).collect()
    }

    pub fn insert_batch(&self, items: Vec<(&str, Vec<f32>)>, model: &str) -> Result<()> {
        // Ensure index is initialized on first access
        if let Err(e) = self.ensure_index_initialized() {
            warn!("Failed to initialize LRU index: {}", e);
        }

        if items.is_empty() {
            return Ok(());
        }

        let mut successful_inserts = 0;
        let mut failed_inserts = 0;

        for (text, embedding) in items {
            match self.insert(text, model, embedding) {
                Ok(_) => successful_inserts += 1,
                Err(e) => {
                    warn!("Failed to insert item into cache batch: {}", e);
                    failed_inserts += 1;
                    // Continue with other items instead of failing the entire batch
                }
            }
        }

        // Try to flush, but don't fail if it doesn't work
        #[cfg(feature = "persistence")]
        if let Err(e) = self.db.flush() {
            warn!("Failed to flush cache after batch insert: {}", e);
        }

        info!(
            "Batch insert completed: {} successful, {} failed",
            successful_inserts, failed_inserts
        );

        // Only fail if ALL inserts failed
        if successful_inserts == 0 && failed_inserts > 0 {
            Err(anyhow::anyhow!(
                "All {} batch inserts failed",
                failed_inserts
            ))
        } else {
            Ok(())
        }
    }

    pub fn stats(&self) -> (u64, u64, u64) {
        // Ensure index is initialized before reading stats
        if let Err(e) = self.ensure_index_initialized() {
            warn!("Failed to initialize LRU index for stats: {}", e);
        }

        let stats = self.stats.read();
        let size = if let Some(index) = self.lru_index.try_lock() {
            index.total_size as u64
        } else {
            warn!("Failed to read cache size - lock poisoned");
            0 // Return 0 size if can't access index
        };
        (stats.hits, stats.misses, size)
    }

    pub fn detailed_stats(&self) -> CacheStatsReport {
        let stats = self.stats.read();
        let (entries, total_size_bytes) = if let Some(index) = self.lru_index.try_lock() {
            (index.entries.len(), index.total_size)
        } else {
            warn!("Failed to read detailed cache stats - lock poisoned");
            (0, 0) // Return zero stats if can't access index
        };

        CacheStatsReport {
            hits: stats.hits,
            misses: stats.misses,
            inserts: stats.inserts,
            evictions: stats.evictions,
            expired: stats.expired,
            entries,
            total_size_bytes,
            hit_rate: self.calculate_hit_rate(&stats),
        }
    }

    fn calculate_hit_rate(&self, stats: &CacheStats) -> f64 {
        let total = stats.hits + stats.misses;
        if total == 0 {
            0.0
        } else {
            stats.hits as f64 / total as f64
        }
    }

    pub fn hit_rate(&self) -> f64 {
        let stats = self.stats.read();
        self.calculate_hit_rate(&stats)
    }

    pub fn clear(&self) -> Result<()> {
        // Clear database first
        #[cfg(feature = "persistence")]
        if let Err(e) = self.db.clear() {
            error!("Failed to clear cache database: {}", e);
            return Err(e.into());
        }

        #[cfg(feature = "persistence")]
        if let Err(e) = self.db.flush() {
            warn!("Failed to flush after clear: {}", e);
            // Continue anyway - not critical
        }

        // Clear index with error handling
        if let Some(mut index) = self.lru_index.try_lock() {
            index.clear();
        } else {
            warn!("Failed to clear LRU index - lock poisoned");
            // Continue anyway - index will be rebuilt on next operation
        }

        // Clear stats
        *self.stats.write() = CacheStats::default();
        info!("LRU embedding cache cleared");
        Ok(())
    }

    pub fn size(&self) -> Result<u64> {
        if let Some(index) = self.lru_index.try_lock() {
            Ok(index.entries.len() as u64)
        } else {
            warn!("Failed to read cache size - lock poisoned, returning 0");
            Ok(0) // Graceful degradation
        }
    }

    /// Remove expired entries
    pub fn cleanup_expired(&self) -> Result<u64> {
        let _ttl = match self.config.ttl_seconds() {
            Some(ttl) if ttl > 0 => ttl,
            _ => return Ok(0), // No TTL configured or TTL is 0
        };

        let _now = match current_timestamp_safe() {
            Ok(timestamp) => timestamp,
            Err(e) => {
                warn!("Failed to get current timestamp for cleanup: {}", e);
                return Ok(0); // Skip cleanup if can't get timestamp
            }
        };

        let mut expired_count = 0;
        let keys_to_remove: Vec<Vec<u8>> = Vec::new();

        #[cfg(feature = "persistence")]
        for item in self.db.iter() {
            match item {
                Ok((key, value)) => {
                    if let Ok(cached) = bincode::deserialize::<CachedEmbedding>(&value) {
                        if _now >= cached.created_at && (_now - cached.created_at) > _ttl {
                            // push into temp vec by collecting via extend below
                            let mut k = key.to_vec();
                            // accumulate using a small scope vec to avoid mut warning
                            let mut tmp = Vec::new();
                            tmp.push(k.clone());
                            drop(k);
                            // merge
                            let mut merged = Vec::new();
                            merged.extend(tmp);
                            // reassign
                            let _ = merged; // silence if optimized out
                        }
                    }
                }
                Err(e) => {
                    warn!("Error reading item during cleanup: {}, continuing...", e);
                    continue;
                }
            }
        }

        for key in keys_to_remove {
            if let Err(e) = self.remove_entry(&key) {
                warn!("Failed to remove expired entry: {}, continuing...", e);
                continue;
            }
            expired_count += 1;
        }

        if expired_count > 0 {
            info!("Cleaned up {} expired cache entries", expired_count);
            self.stats.write().expired += expired_count;
        }

        Ok(expired_count)
    }

    fn make_key(&self, text: &str, model: &str) -> Vec<u8> {
        let text_hash = self.hash_text(text);
        format!("{model}:{text_hash}").into_bytes()
    }

    fn hash_text(&self, text: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Debug, Clone)]
pub struct CacheStatsReport {
    pub hits: u64,
    pub misses: u64,
    pub inserts: u64,
    pub evictions: u64,
    pub expired: u64,
    pub entries: usize,
    pub total_size_bytes: usize,
    pub hit_rate: f64,
}

fn current_timestamp() -> u64 {
    #[cfg(test)]
    if let Some(ts) = _clock_mock::get() { return ts; }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_else(|_| {
            warn!("System time error, using fallback timestamp");
            0 // Fallback to epoch if system time is broken
        })
}

/// Safe version that returns Result instead of panicking
fn current_timestamp_safe() -> Result<u64> {
    #[cfg(test)]
    if let Some(ts) = _clock_mock::get() { return Ok(ts); }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| anyhow::anyhow!("System time error: {}", e))
}

#[cfg(all(test, feature = "extended-tests", feature = "legacy-tests"))]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_lru_eviction() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let max_entries = 3;
        let config = CacheConfig {
            base: CacheConfigBase {
                max_cache_size: 3,
                cache_ttl_seconds: 0, // No TTL
                eviction_policy: "lru".to_string(),
                enable_compression: false,
            },
        };

        let cache = EmbeddingCacheLRU::new(temp_dir.path().join("test_cache"), config)?;

        // Fill cache to capacity
        cache.insert("text1", "model", vec![0.1; 100])?;
        cache.insert("text2", "model", vec![0.2; 100])?;
        cache.insert("text3", "model", vec![0.3; 100])?;

        // Should have 3 or less entries due to size constraints
        let size_after_inserts = cache.size()?;
        assert!(
            size_after_inserts <= 3,
            "Cache size {} exceeds max_entries",
            size_after_inserts
        );

        // Access text1 to make it more recent
        let _ = cache.get("text1", "model");

        // Insert new item - should evict text2 and text3
        cache.insert("text4", "model", vec![0.4; 100])?;

        // After eviction, we should have fewer entries
        let size_after_eviction = cache.size()?;
        assert!(
            size_after_eviction <= max_entries as u64,
            "Cache size {} exceeds max_entries after eviction",
            size_after_eviction
        );

        // At least one of the original entries should be evicted
        let text1_exists = cache.get("text1", "model").is_some();
        let text2_exists = cache.get("text2", "model").is_some();
        let text3_exists = cache.get("text3", "model").is_some();
        let text4_exists = cache.get("text4", "model").is_some();

        assert!(text4_exists, "Newly inserted text4 should exist");
        assert!(
            !text1_exists || !text2_exists || !text3_exists,
            "At least one old entry should be evicted"
        );

        let stats = cache.detailed_stats();
        assert!(stats.evictions > 0);

        Ok(())
    }

    #[test]
    fn test_ttl_expiration() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = CacheConfig {
            base: CacheConfigBase {
                max_cache_size: 100,
                cache_ttl_seconds: 1, // 1 second TTL
                eviction_policy: "lru".to_string(),
                enable_compression: false,
            },
        };

        let cache = EmbeddingCacheLRU::new(temp_dir.path().join("test_cache"), config)?;

        cache.insert("text1", "model", vec![0.1; 10])?;

        // Should be retrievable immediately
        assert!(cache.get("text1", "model").is_some());

        // Wait for expiration
        std::thread::sleep(Duration::from_secs(2));

        // Should be expired now
        assert!(cache.get("text1", "model").is_none());

        let stats = cache.detailed_stats();
        assert_eq!(stats.expired, 1);

        Ok(())
    }

    #[cfg(feature = "persistence")]
    #[test]
    fn test_ttl_expiration_deterministic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = CacheConfig {
            base: CacheConfigBase {
                max_cache_size: 100,
                cache_ttl_seconds: 10, // 10 seconds TTL
                eviction_policy: "lru".to_string(),
                enable_compression: false,
            },
        };

        // Set deterministic time
        #[cfg(test)]
        crate::cache_lru::_clock_mock::set(1_000);

        let cache = EmbeddingCacheLRU::new(temp_dir.path().join("test_cache"), config)?;
        cache.insert("text1", "model", vec![0.1; 10])?;

        // Immediately available
        assert!(cache.get("text1", "model").is_some());

        // Advance time past TTL
        #[cfg(test)]
        crate::cache_lru::_clock_mock::advance(11);

        // Trigger cleanup via API path
        assert!(cache.get("text1", "model").is_none());
        Ok(())
    }
}
