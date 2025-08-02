use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedEmbedding {
    embedding: Vec<f32>,
    model: String,
    created_at: i64,
}

// @component: {"k":"C","id":"embedding_cache","t":"Embedding cache with sled","m":{"cur":85,"tgt":95,"u":"%"},"f":["cache","persistence"]}
pub struct EmbeddingCache {
    db: Arc<Db>,
    stats: Arc<RwLock<CacheStats>>,
}

#[derive(Debug, Default)]
struct CacheStats {
    hits: u64,
    misses: u64,
    inserts: u64,
}

impl EmbeddingCache {
    /// Открывает sled БД для кэша с crash recovery
    fn open_cache_database(cache_path: impl AsRef<Path>) -> Result<Db> {
        use sled::Config;
        
        let config = Config::new()
            .path(cache_path.as_ref())
            .mode(sled::Mode::HighThroughput)
            .flush_every_ms(Some(2000))      // Кэш может быть менее частым
            .use_compression(true)
            .compression_factor(19);
            
        let db = config.open().context("Failed to open cache database")?;
        info!("Cache database opened with crash recovery");
        Ok(db)
    }

    pub fn new(cache_path: impl AsRef<Path>) -> Result<Self> {
        let cache_path = cache_path.as_ref();
        
        // Create directory if it doesn't exist
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        info!("Opening embedding cache at: {:?}", cache_path);
        let db = Self::open_cache_database(cache_path)?;

        Ok(Self {
            db: Arc::new(db),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        })
    }

    pub fn get(&self, text: &str, model: &str) -> Option<Vec<f32>> {
        let key = self.make_key(text, model);
        
        match self.db.get(&key) {
            Ok(Some(bytes)) => {
                match bincode::deserialize::<CachedEmbedding>(&bytes) {
                    Ok(cached) => {
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
        let cached = CachedEmbedding {
            embedding,
            model: model.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };
        
        let bytes = bincode::serialize(&cached)?;
        self.db.insert(key, bytes)?;
        
        self.stats.write().inserts += 1;
        debug!("Cached embedding for text hash: {}", self.hash_text(text));
        
        Ok(())
    }

    pub fn get_batch(&self, texts: &[String], model: &str) -> Vec<Option<Vec<f32>>> {
        texts.iter()
            .map(|text| self.get(text, model))
            .collect()
    }

    pub fn insert_batch(&self, items: Vec<(&str, Vec<f32>)>, model: &str) -> Result<()> {
        let batch: Vec<_> = items
            .into_iter()
            .map(|(text, embedding)| {
                let key = self.make_key(text, model);
                let cached = CachedEmbedding {
                    embedding,
                    model: model.to_string(),
                    created_at: chrono::Utc::now().timestamp(),
                };
                
                bincode::serialize(&cached)
                    .map(|bytes| (key, bytes))
            })
            .collect::<Result<Vec<_>, _>>()?;

        for (key, bytes) in batch {
            self.db.insert(key, bytes)?;
            self.stats.write().inserts += 1;
        }

        self.db.flush()?;
        Ok(())
    }

    pub fn stats(&self) -> (u64, u64, u64) {
        let stats = self.stats.read();
        (stats.hits, stats.misses, stats.inserts)
    }

    pub fn hit_rate(&self) -> f64 {
        let stats = self.stats.read();
        let total = stats.hits + stats.misses;
        if total == 0 {
            0.0
        } else {
            stats.hits as f64 / total as f64
        }
    }

    pub fn clear(&self) -> Result<()> {
        self.db.clear()?;
        self.db.flush()?;
        *self.stats.write() = CacheStats::default();
        info!("Embedding cache cleared");
        Ok(())
    }

    pub fn size(&self) -> Result<u64> {
        Ok(self.db.len() as u64)
    }

    fn make_key(&self, text: &str, model: &str) -> Vec<u8> {
        let text_hash = self.hash_text(text);
        format!("{}:{}", model, text_hash).into_bytes()
    }

    fn hash_text(&self, text: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_operations() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cache = EmbeddingCache::new(temp_dir.path().join("test_cache"))?;

        let text = "Hello, world!";
        let model = "test-model";
        let embedding = vec![0.1, 0.2, 0.3];

        // Test miss
        assert!(cache.get(text, model).is_none());

        // Test insert and hit
        cache.insert(text, model, embedding.clone())?;
        assert_eq!(cache.get(text, model), Some(embedding));

        // Test stats
        let (hits, misses, inserts) = cache.stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
        assert_eq!(inserts, 1);

        Ok(())
    }
}