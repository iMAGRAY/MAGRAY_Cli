use anyhow::{Context, Result};
use parking_lot::{RwLock, Mutex};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedEmbedding {
    embedding: Vec<f32>,
    model: String,
    created_at: u64,
    last_accessed: u64,
    access_count: u32,
    size_bytes: usize,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum cache size in bytes
    pub max_size_bytes: usize,
    /// Maximum number of entries
    pub max_entries: usize,
    /// TTL for cache entries in seconds (None = no expiration)
    pub ttl_seconds: Option<u64>,
    /// Number of entries to evict at once when cache is full
    pub eviction_batch_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: 1024 * 1024 * 1024, // 1GB
            max_entries: 100_000,
            ttl_seconds: Some(86400 * 7), // 7 days
            eviction_batch_size: 100,
        }
    }
}

// @component: {"k":"C","id":"embedding_cache_lru","t":"LRU cache with eviction policy","m":{"cur":90,"tgt":100,"u":"%"},"f":["cache","lru","eviction"]}
pub struct EmbeddingCacheLRU {
    db: Arc<Db>,
    stats: Arc<RwLock<CacheStats>>,
    lru_index: Arc<Mutex<LruIndex>>,
    config: CacheConfig,
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
        self.access_queue.iter()
            .take(count)
            .cloned()
            .collect()
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
    /// Открывает sled БД для LRU кэша с crash recovery
    fn open_cache_database(cache_path: impl AsRef<Path>) -> Result<Db> {
        use sled::Config;
        
        let config = Config::new()
            .path(cache_path.as_ref())
            .mode(sled::Mode::HighThroughput)
            .flush_every_ms(Some(3000))      // LRU кэш реже flush
            .use_compression(true)
            .compression_factor(19);
            
        let db = config.open().context("Failed to open LRU cache database")?;
        info!("LRU cache database opened with crash recovery");
        Ok(db)
    }

    pub fn new(cache_path: impl AsRef<Path>, config: CacheConfig) -> Result<Self> {
        let cache_path = cache_path.as_ref();
        
        // Create directory if it doesn't exist
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        info!("Opening LRU embedding cache at: {:?}", cache_path);
        info!("Cache config: max_size={}MB, max_entries={}, ttl={:?}s", 
              config.max_size_bytes / 1024 / 1024,
              config.max_entries,
              config.ttl_seconds);
        
        let db = Self::open_cache_database(cache_path)?;

        let cache = Self {
            db: Arc::new(db),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            lru_index: Arc::new(Mutex::new(LruIndex::new())),
            config,
        };

        // Rebuild LRU index from existing data
        cache.rebuild_index()?;

        Ok(cache)
    }

    /// Rebuild the LRU index from database
    fn rebuild_index(&self) -> Result<()> {
        let mut index = self.lru_index.lock();
        index.clear();

        for item in self.db.iter() {
            let (key, value) = item?;
            if let Ok(cached) = bincode::deserialize::<CachedEmbedding>(&value) {
                index.touch(key.to_vec(), cached.size_bytes);
            }
        }

        info!("Rebuilt LRU index: {} entries, {} MB total", 
              index.entries.len(),
              index.total_size / 1024 / 1024);

        Ok(())
    }

    pub fn get(&self, text: &str, model: &str) -> Option<Vec<f32>> {
        let key = self.make_key(text, model);
        
        match self.db.get(&key) {
            Ok(Some(bytes)) => {
                match bincode::deserialize::<CachedEmbedding>(&bytes) {
                    Ok(mut cached) => {
                        // Check TTL
                        if let Some(ttl) = self.config.ttl_seconds {
                            let age = current_timestamp() - cached.created_at;
                            if age > ttl {
                                debug!("Cache entry expired: age={} > ttl={}", age, ttl);
                                self.stats.write().expired += 1;
                                let _ = self.remove_entry(&key);
                                return None;
                            }
                        }

                        // Update access stats
                        cached.last_accessed = current_timestamp();
                        cached.access_count += 1;
                        
                        // Update in database
                        if let Ok(updated_bytes) = bincode::serialize(&cached) {
                            let _ = self.db.insert(&key, updated_bytes);
                        }

                        // Update LRU index
                        self.lru_index.lock().touch(key.to_vec(), cached.size_bytes);

                        self.stats.write().hits += 1;
                        debug!("Cache hit for text hash: {}", self.hash_text(text));
                        Some(cached.embedding)
                    }
                    Err(e) => {
                        debug!("Failed to deserialize cached embedding: {}", e);
                        None
                    }
                }
            }
            _ => {
                self.stats.write().misses += 1;
                None
            }
        }
    }

    pub fn insert(&self, text: &str, model: &str, embedding: Vec<f32>) -> Result<()> {
        let key = self.make_key(text, model);
        let size_bytes = embedding.len() * std::mem::size_of::<f32>() + 256; // Overhead
        
        // Check if we need to evict entries
        self.maybe_evict(size_bytes)?;

        let cached = CachedEmbedding {
            embedding,
            model: model.to_string(),
            created_at: current_timestamp(),
            last_accessed: current_timestamp(),
            access_count: 1,
            size_bytes,
        };
        
        let bytes = bincode::serialize(&cached)?;
        self.db.insert(&key, bytes)?;
        
        // Update LRU index
        self.lru_index.lock().touch(key.to_vec(), size_bytes);
        
        self.stats.write().inserts += 1;
        debug!("Cached embedding for text hash: {}", self.hash_text(text));
        
        Ok(())
    }

    /// Check if eviction is needed and perform it
    fn maybe_evict(&self, needed_size: usize) -> Result<()> {
        let mut index = self.lru_index.lock();
        
        // Check size constraint
        let mut entries_to_evict = Vec::new();
        
        if index.total_size + needed_size > self.config.max_size_bytes {
            let target_size = self.config.max_size_bytes * 8 / 10; // Free up to 80%
            let mut current_size = index.total_size;
            
            for key in index.get_oldest(self.config.eviction_batch_size) {
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
        if index.entries.len() + 1 > self.config.max_entries {
            let additional = index.get_oldest(self.config.eviction_batch_size);
            for key in additional {
                if !entries_to_evict.contains(&key) {
                    entries_to_evict.push(key);
                }
            }
        }

        // Perform eviction
        if !entries_to_evict.is_empty() {
            warn!("Evicting {} cache entries to make room", entries_to_evict.len());
            let mut stats = self.stats.write();
            
            for key in entries_to_evict {
                index.remove(&key);
                let _ = self.db.remove(&key);
                stats.evictions += 1;
            }
        }

        Ok(())
    }

    /// Remove an entry from cache
    fn remove_entry(&self, key: &[u8]) -> Result<()> {
        self.lru_index.lock().remove(key);
        self.db.remove(key)?;
        Ok(())
    }

    pub fn get_batch(&self, texts: &[String], model: &str) -> Vec<Option<Vec<f32>>> {
        texts.iter()
            .map(|text| self.get(text, model))
            .collect()
    }

    pub fn insert_batch(&self, items: Vec<(&str, Vec<f32>)>, model: &str) -> Result<()> {
        for (text, embedding) in items {
            self.insert(text, model, embedding)?;
        }
        self.db.flush()?;
        Ok(())
    }

    pub fn stats(&self) -> (u64, u64, u64) {
        let stats = self.stats.read();
        let size = self.lru_index.lock().total_size as u64;
        (stats.hits, stats.misses, size)
    }

    pub fn detailed_stats(&self) -> CacheStatsReport {
        let stats = self.stats.read();
        let index = self.lru_index.lock();
        
        CacheStatsReport {
            hits: stats.hits,
            misses: stats.misses,
            inserts: stats.inserts,
            evictions: stats.evictions,
            expired: stats.expired,
            entries: index.entries.len(),
            total_size_bytes: index.total_size,
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
        self.db.clear()?;
        self.db.flush()?;
        self.lru_index.lock().clear();
        *self.stats.write() = CacheStats::default();
        info!("LRU embedding cache cleared");
        Ok(())
    }

    pub fn size(&self) -> Result<u64> {
        Ok(self.lru_index.lock().entries.len() as u64)
    }

    /// Remove expired entries
    pub fn cleanup_expired(&self) -> Result<u64> {
        if self.config.ttl_seconds.is_none() {
            return Ok(0);
        }

        let ttl = self.config.ttl_seconds.unwrap();
        let now = current_timestamp();
        let mut expired_count = 0;
        let mut keys_to_remove = Vec::new();

        for item in self.db.iter() {
            let (key, value) = item?;
            if let Ok(cached) = bincode::deserialize::<CachedEmbedding>(&value) {
                if now - cached.created_at > ttl {
                    keys_to_remove.push(key.to_vec());
                }
            }
        }

        for key in keys_to_remove {
            self.remove_entry(&key)?;
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
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::time::Duration;

    #[test]
    fn test_lru_eviction() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let max_entries = 3;
        let config = CacheConfig {
            max_size_bytes: 1024, // Very small for testing
            max_entries: 3,
            ttl_seconds: None,
            eviction_batch_size: 2,
        };
        
        let cache = EmbeddingCacheLRU::new(temp_dir.path().join("test_cache"), config)?;

        // Fill cache to capacity
        cache.insert("text1", "model", vec![0.1; 100])?;
        cache.insert("text2", "model", vec![0.2; 100])?;
        cache.insert("text3", "model", vec![0.3; 100])?;

        // Should have 3 or less entries due to size constraints
        let size_after_inserts = cache.size()?;
        assert!(size_after_inserts <= 3, "Cache size {} exceeds max_entries", size_after_inserts);

        // Access text1 to make it more recent
        let _ = cache.get("text1", "model");

        // Insert new item - should evict text2 and text3
        cache.insert("text4", "model", vec![0.4; 100])?;

        // After eviction, we should have fewer entries
        let size_after_eviction = cache.size()?;
        assert!(size_after_eviction <= max_entries as u64, "Cache size {} exceeds max_entries after eviction", size_after_eviction);
        
        // At least one of the original entries should be evicted
        let text1_exists = cache.get("text1", "model").is_some();
        let text2_exists = cache.get("text2", "model").is_some();
        let text3_exists = cache.get("text3", "model").is_some();
        let text4_exists = cache.get("text4", "model").is_some();
        
        assert!(text4_exists, "Newly inserted text4 should exist");
        assert!(!text1_exists || !text2_exists || !text3_exists, "At least one old entry should be evicted");

        let stats = cache.detailed_stats();
        assert!(stats.evictions > 0);

        Ok(())
    }

    #[test]
    fn test_ttl_expiration() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let config = CacheConfig {
            max_size_bytes: 1024 * 1024,
            max_entries: 100,
            ttl_seconds: Some(1), // 1 second TTL
            eviction_batch_size: 10,
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
}