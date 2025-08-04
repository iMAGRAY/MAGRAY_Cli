use anyhow::Result;

/// Common interface for embedding caches
pub trait EmbeddingCacheInterface: Send + Sync {
    /// Get an embedding from cache
    fn get(&self, text: &str, model: &str) -> Option<Vec<f32>>;
    
    /// Insert an embedding into cache
    fn insert(&self, text: &str, model: &str, embedding: Vec<f32>) -> Result<()>;
    
    /// Get multiple embeddings from cache
    fn get_batch(&self, texts: &[String], model: &str) -> Vec<Option<Vec<f32>>>;
    
    /// Insert multiple embeddings into cache
    fn insert_batch(&self, items: Vec<(&str, Vec<f32>)>, model: &str) -> Result<()>;
    
    /// Get cache statistics (hits, misses, size)
    fn stats(&self) -> (u64, u64, u64);
    
    /// Get cache hit rate
    fn hit_rate(&self) -> f64;
    
    /// Clear the cache
    fn clear(&self) -> Result<()>;
    
    /// Get number of entries in cache
    fn size(&self) -> Result<u64>;
<<<<<<< HEAD
    
    /// Test helper to check if cache is properly initialized
    fn is_null_check(&self) -> bool {
        false // По умолчанию считаем что cache инициализирован
    }
=======
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
}

// Implement the trait for simple cache
impl EmbeddingCacheInterface for crate::cache::EmbeddingCache {
    fn get(&self, text: &str, model: &str) -> Option<Vec<f32>> {
        self.get(text, model)
    }
    
    fn insert(&self, text: &str, model: &str, embedding: Vec<f32>) -> Result<()> {
        self.insert(text, model, embedding)
    }
    
    fn get_batch(&self, texts: &[String], model: &str) -> Vec<Option<Vec<f32>>> {
        self.get_batch(texts, model)
    }
    
    fn insert_batch(&self, items: Vec<(&str, Vec<f32>)>, model: &str) -> Result<()> {
        self.insert_batch(items, model)
    }
    
    fn stats(&self) -> (u64, u64, u64) {
        self.stats()
    }
    
    fn hit_rate(&self) -> f64 {
        self.hit_rate()
    }
    
    fn clear(&self) -> Result<()> {
        self.clear()
    }
    
    fn size(&self) -> Result<u64> {
        self.size()
    }
}

// Implement the trait for LRU cache
impl EmbeddingCacheInterface for crate::cache_lru::EmbeddingCacheLRU {
    fn get(&self, text: &str, model: &str) -> Option<Vec<f32>> {
        self.get(text, model)
    }
    
    fn insert(&self, text: &str, model: &str, embedding: Vec<f32>) -> Result<()> {
        self.insert(text, model, embedding)
    }
    
    fn get_batch(&self, texts: &[String], model: &str) -> Vec<Option<Vec<f32>>> {
        self.get_batch(texts, model)
    }
    
    fn insert_batch(&self, items: Vec<(&str, Vec<f32>)>, model: &str) -> Result<()> {
        self.insert_batch(items, model)
    }
    
    fn stats(&self) -> (u64, u64, u64) {
        self.stats()
    }
    
    fn hit_rate(&self) -> f64 {
        self.hit_rate()
    }
    
    fn clear(&self) -> Result<()> {
        self.clear()
    }
    
    fn size(&self) -> Result<u64> {
        self.size()
    }
}