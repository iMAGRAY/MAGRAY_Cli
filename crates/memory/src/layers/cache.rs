//! Cache Layer Implementation - LRU кэширование embeddings
//!
//! LRUCacheLayer инкапсулирует все операции кэширования embeddings
//! для минимизации обращений к AI сервисам и ускорения поиска.
//!
//! RESPONSIBILITIES:
//! - LRU кэширование embeddings по ключам
//! - TTL (Time To Live) management
//! - Persistence кэша на диск для warm restarts
//! - Batch операции для производительности
//! - Cache warming strategies

use anyhow::{Result, Context};
use async_trait::async_trait;
use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

use crate::layers::{CacheLayer, LayerHealth, LayerHealthStatus, CacheConfig};

/// LRU Cache implementation для Cache Layer
/// 
/// Фокусируется ТОЛЬКО на кэшировании embeddings:
/// - Memory-efficient LRU eviction
/// - TTL based expiration
/// - Persistent cache storage
/// - Production optimizations
pub struct LRUCacheLayer {
    config: CacheConfig,
    cache: Arc<RwLock<CacheStorage>>,
    stats: Arc<RwLock<CacheStats>>,
    persistence_enabled: bool,
}

/// Внутреннее хранилище кэша с LRU логикой
#[derive(Debug)]
struct CacheStorage {
    entries: HashMap<String, CacheEntry>,
    lru_order: BTreeMap<DateTime<Utc>, String>, // access_time -> key
    size_bytes: usize,
}

/// Запись в кэше с метаданными
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    embedding: Vec<f32>,
    created_at: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    access_count: u32,
    size_bytes: usize,
}

/// Статистики кэша для monitoring
#[derive(Debug, Default)]
struct CacheStats {
    hits: u64,
    misses: u64,
    evictions: u64,
    expired: u64,
    total_get_operations: u64,
    total_put_operations: u64,
    cache_warming_operations: u64,
}

impl LRUCacheLayer {
    /// Создать новый LRU cache layer с опциональной persistence
    pub async fn new(config: CacheConfig) -> Result<Arc<Self>> {
        info!("💾 Инициализация LRU Cache Layer (max_size={}, ttl={}s)", 
              config.max_size, config.ttl_seconds);

        let persistence_enabled = config.cache_path.is_some();
        
        if persistence_enabled {
            if let Some(ref cache_path) = config.cache_path {
                // Создаем директорию для persistent cache
                if let Some(parent) = cache_path.parent() {
                    tokio::fs::create_dir_all(parent).await
                        .context("Не удалось создать директорию для кэша")?;
                }
                info!("💿 Persistent кэш включен: {:?}", cache_path);
            }
        }

        let cache_layer = Arc::new(Self {
            config: config.clone(),
            cache: Arc::new(RwLock::new(CacheStorage {
                entries: HashMap::new(),
                lru_order: BTreeMap::new(),
                size_bytes: 0,
            })),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            persistence_enabled,
        });

        // Загружаем persistent cache если доступен
        if persistence_enabled {
            if let Err(e) = cache_layer.load_persistent_cache().await {
                warn!("⚠️ Не удалось загрузить persistent cache: {}", e);
                // Продолжаем работу без persistent cache
            }
        }

        info!("✅ LRU Cache Layer успешно инициализирован");
        Ok(cache_layer)
    }

    /// Создать хэш ключ для embedding cache
    fn create_cache_key(text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        let result = hasher.finalize();
        format!("emb_{:x}", result)[..32].to_string() // Берем первые 32 символа для компактности
    }

    /// Проверить не истек ли TTL для entry
    fn is_expired(&self, entry: &CacheEntry) -> bool {
        if let Some(expires_at) = entry.expires_at {
            Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Вычислить размер embedding в байтах
    fn calculate_embedding_size(embedding: &[f32]) -> usize {
        embedding.len() * std::mem::size_of::<f32>() + 
        std::mem::size_of::<CacheEntry>() // Приблизительный размер metadata
    }

    /// Evict entries до достижения max_size
    async fn evict_lru_entries(&self, required_space: usize) -> Result<()> {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        
        while cache.size_bytes + required_space > self.config.max_size * 1024 * 1024 && !cache.lru_order.is_empty() {
            // Находим самый старый entry по last_accessed
            if let Some((oldest_access_time, key)) = cache.lru_order.iter().next() {
                let oldest_access_time = *oldest_access_time;
                let key = key.clone();
                
                // Удаляем entry
                if let Some(entry) = cache.entries.remove(&key) {
                    cache.size_bytes = cache.size_bytes.saturating_sub(entry.size_bytes);
                    stats.evictions += 1;
                    
                    debug!("🗑️ Evicted cache entry: {} (size: {} bytes)", key, entry.size_bytes);
                }
                
                cache.lru_order.remove(&oldest_access_time);
            }
        }
        
        Ok(())
    }

    /// Обновить LRU order для ключа
    async fn update_lru_order(&self, key: &str) {
        let mut cache = self.cache.write().await;
        let now = Utc::now();
        
        // Удаляем старую запись из LRU order
        if let Some(entry) = cache.entries.get(key) {
            cache.lru_order.remove(&entry.last_accessed);
        }
        
        // Обновляем last_accessed и добавляем в LRU order
        if let Some(entry) = cache.entries.get_mut(key) {
            entry.last_accessed = now;
            entry.access_count += 1;
            cache.lru_order.insert(now, key.to_string());
        }
    }

    /// Cleanup expired entries (background task)
    async fn cleanup_expired_entries(&self) -> Result<usize> {
        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;
        let now = Utc::now();
        let mut expired_count = 0;

        let mut keys_to_remove = Vec::new();
        
        for (key, entry) in &cache.entries {
            if let Some(expires_at) = entry.expires_at {
                if now > expires_at {
                    keys_to_remove.push(key.clone());
                }
            }
        }

        for key in keys_to_remove {
            if let Some(entry) = cache.entries.remove(&key) {
                cache.lru_order.remove(&entry.last_accessed);
                cache.size_bytes = cache.size_bytes.saturating_sub(entry.size_bytes);
                expired_count += 1;
            }
        }

        stats.expired += expired_count as u64;
        
        if expired_count > 0 {
            debug!("🧹 Cleaned up {} expired cache entries", expired_count);
        }
        
        Ok(expired_count)
    }

    /// Загрузить persistent cache с диска
    async fn load_persistent_cache(&self) -> Result<()> {
        if let Some(ref cache_path) = self.config.cache_path {
            let cache_file = cache_path.join("embeddings_cache.json");
            
            if !cache_file.exists() {
                debug!("📂 Persistent cache file не найден: {:?}", cache_file);
                return Ok(());
            }

            info!("📥 Загрузка persistent cache из {:?}", cache_file);
            
            let cache_data = tokio::fs::read_to_string(&cache_file).await
                .context("Не удалось прочитать cache file")?;
                
            let entries: HashMap<String, CacheEntry> = serde_json::from_str(&cache_data)
                .context("Не удалось десериализовать cache data")?;

            let mut cache = self.cache.write().await;
            let now = Utc::now();
            let mut loaded_count = 0;
            let mut size_bytes = 0;

            // Загружаем не истекшие entries
            for (key, entry) in entries {
                if let Some(expires_at) = entry.expires_at {
                    if now > expires_at {
                        continue; // Пропускаем истекшие
                    }
                }
                
                size_bytes += entry.size_bytes;
                cache.lru_order.insert(entry.last_accessed, key.clone());
                cache.entries.insert(key, entry);
                loaded_count += 1;
            }

            cache.size_bytes = size_bytes;
            info!("✅ Загружено {} cache entries из persistent storage", loaded_count);
        }
        
        Ok(())
    }

    /// Сохранить persistent cache на диск
    async fn save_persistent_cache(&self) -> Result<()> {
        if !self.persistence_enabled {
            return Ok(());
        }

        if let Some(ref cache_path) = self.config.cache_path {
            let cache_file = cache_path.join("embeddings_cache.json");
            
            let cache = self.cache.read().await;
            let cache_data = serde_json::to_string_pretty(&cache.entries)
                .context("Не удалось сериализовать cache data")?;
                
            tokio::fs::write(&cache_file, cache_data).await
                .context("Не удалось записать cache file")?;
                
            debug!("💾 Persistent cache сохранен: {} entries", cache.entries.len());
        }
        
        Ok(())
    }
}

#[async_trait]
impl CacheLayer for LRUCacheLayer {
    async fn get(&self, key: &str) -> Result<Option<Vec<f32>>> {
        let cache_key = Self::create_cache_key(key);
        
        // Обновляем статистики
        {
            let mut stats = self.stats.write().await;
            stats.total_get_operations += 1;
        }

        let cache = self.cache.read().await;
        
        if let Some(entry) = cache.entries.get(&cache_key) {
            // Проверяем TTL
            if self.is_expired(entry) {
                drop(cache); // Освобождаем read lock
                
                // Удаляем expired entry
                let mut cache_write = self.cache.write().await;
                cache_write.entries.remove(&cache_key);
                cache_write.lru_order.remove(&entry.last_accessed);
                cache_write.size_bytes = cache_write.size_bytes.saturating_sub(entry.size_bytes);
                
                let mut stats = self.stats.write().await;
                stats.misses += 1;
                stats.expired += 1;
                
                debug!("⏰ Cache miss: key expired '{}'", key);
                return Ok(None);
            }

            // Cache hit!
            let embedding = entry.embedding.clone();
            drop(cache); // Освобождаем read lock
            
            // Обновляем LRU order
            self.update_lru_order(&cache_key).await;
            
            // Обновляем статистики
            {
                let mut stats = self.stats.write().await;
                stats.hits += 1;
            }
            
            debug!("🎯 Cache hit for key '{}'", key);
            Ok(Some(embedding))
        } else {
            // Cache miss
            let mut stats = self.stats.write().await;
            stats.misses += 1;
            
            debug!("❌ Cache miss for key '{}'", key);
            Ok(None)
        }
    }

    async fn put(&self, key: &str, embedding: Vec<f32>) -> Result<()> {
        let cache_key = Self::create_cache_key(key);
        let embedding_size = Self::calculate_embedding_size(&embedding);
        let now = Utc::now();
        
        // Проверяем не превышает ли размер лимит
        if embedding_size > self.config.max_size * 1024 * 1024 {
            warn!("⚠️ Embedding слишком большой для кэширования: {} bytes", embedding_size);
            return Ok(()); // Не кэшируем слишком большие embeddings
        }

        // Evict entries если необходимо
        self.evict_lru_entries(embedding_size).await?;

        let expires_at = if self.config.ttl_seconds > 0 {
            Some(now + chrono::Duration::seconds(self.config.ttl_seconds as i64))
        } else {
            None
        };

        let entry = CacheEntry {
            embedding,
            created_at: now,
            last_accessed: now,
            expires_at,
            access_count: 0,
            size_bytes: embedding_size,
        };

        // Добавляем в cache
        {
            let mut cache = self.cache.write().await;
            
            // Удаляем старую запись если существует
            if let Some(old_entry) = cache.entries.remove(&cache_key) {
                cache.lru_order.remove(&old_entry.last_accessed);
                cache.size_bytes = cache.size_bytes.saturating_sub(old_entry.size_bytes);
            }
            
            cache.entries.insert(cache_key.clone(), entry);
            cache.lru_order.insert(now, cache_key.clone());
            cache.size_bytes += embedding_size;
        }

        // Обновляем статистики
        {
            let mut stats = self.stats.write().await;
            stats.total_put_operations += 1;
        }

        debug!("💾 Cached embedding for key '{}' (size: {} bytes)", key, embedding_size);
        Ok(())
    }

    async fn put_batch(&self, entries: &[(&str, &[f32])]) -> Result<()> {
        debug!("📦 Batch caching {} embeddings", entries.len());
        
        for (key, embedding) in entries {
            self.put(key, embedding.to_vec()).await?;
        }
        
        debug!("✅ Batch caching completed");
        Ok(())
    }

    async fn evict(&self, key: &str) -> Result<()> {
        let cache_key = Self::create_cache_key(key);
        
        let mut cache = self.cache.write().await;
        if let Some(entry) = cache.entries.remove(&cache_key) {
            cache.lru_order.remove(&entry.last_accessed);
            cache.size_bytes = cache.size_bytes.saturating_sub(entry.size_bytes);
            
            debug!("🗑️ Manually evicted cache entry for key '{}'", key);
            
            let mut stats = self.stats.write().await;
            stats.evictions += 1;
        }
        
        Ok(())
    }

    async fn prefetch(&self, keys: &[&str]) -> Result<()> {
        debug!("🔄 Prefetch requested for {} keys", keys.len());
        
        // В реальной реализации здесь был бы код для предзагрузки embeddings
        // через AI сервис, но в этом слое мы только управляем кэшем
        
        // Для демонстрации просто проверяем какие ключи уже в кэше
        let cache = self.cache.read().await;
        let mut cached_count = 0;
        
        for key in keys {
            let cache_key = Self::create_cache_key(key);
            if cache.entries.contains_key(&cache_key) {
                cached_count += 1;
            }
        }
        
        debug!("📊 Prefetch check: {}/{} keys already cached", cached_count, keys.len());
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        info!("🧹 Очистка всего кэша");
        
        let mut cache = self.cache.write().await;
        let entry_count = cache.entries.len();
        
        cache.entries.clear();
        cache.lru_order.clear();
        cache.size_bytes = 0;
        
        info!("✅ Кэш очищен: удалено {} entries", entry_count);
        Ok(())
    }

    fn stats(&self) -> (u64, u64, u64) {
        // Возвращаем статистики в неблокирующем режиме
        if let Ok(stats) = self.stats.try_read() {
            (stats.hits, stats.misses, self.cache.try_read().map_or(0, |c| c.entries.len() as u64))
        } else {
            (0, 0, 0) // Fallback если не можем получить lock
        }
    }

    async fn optimize(&self) -> Result<()> {
        info!("🔧 Оптимизация кэша...");
        
        // 1. Cleanup expired entries
        let expired_count = self.cleanup_expired_entries().await?;
        
        // 2. Save persistent cache
        if let Err(e) = self.save_persistent_cache().await {
            warn!("⚠️ Не удалось сохранить persistent cache: {}", e);
        }
        
        // 3. Показать статистики
        let (hits, misses, size) = self.stats();
        let hit_rate = if hits + misses > 0 { 
            (hits as f64 / (hits + misses) as f64) * 100.0 
        } else { 
            0.0 
        };
        
        info!("📊 Cache statistics: {} entries, {:.1}% hit rate, {} expired entries cleaned", 
              size, hit_rate, expired_count);
        
        info!("✅ Оптимизация кэша завершена");
        Ok(())
    }

    async fn warm_cache(&self, popular_keys: &[&str]) -> Result<()> {
        info!("🔥 Warming cache с {} популярными ключами", popular_keys.len());
        
        let mut stats = self.stats.write().await;
        stats.cache_warming_operations += 1;
        
        // В реальной реализации здесь был бы код для предварительного
        // получения embeddings для популярных ключей
        
        debug!("🔥 Cache warming completed (placeholder implementation)");
        Ok(())
    }
}

#[async_trait]
impl LayerHealth for LRUCacheLayer {
    async fn health_check(&self) -> Result<LayerHealthStatus> {
        let start = std::time::Instant::now();
        
        let mut healthy = true;
        let mut details = HashMap::new();
        
        // Проверяем основные метрики
        let (hits, misses, size) = self.stats();
        let cache_size_bytes = self.cache.read().await.size_bytes;
        
        details.insert("total_entries".to_string(), size.to_string());
        details.insert("cache_hits".to_string(), hits.to_string());
        details.insert("cache_misses".to_string(), misses.to_string());
        details.insert("cache_size_bytes".to_string(), cache_size_bytes.to_string());
        
        let hit_rate = if hits + misses > 0 { 
            (hits as f64 / (hits + misses) as f64) * 100.0 
        } else { 
            100.0 
        };
        details.insert("hit_rate_percent".to_string(), format!("{:.1}", hit_rate));
        
        // Тестируем cache операции
        let test_key = "health_check_test";
        let test_embedding = vec![0.1, 0.2, 0.3];
        
        if let Err(e) = self.put(test_key, test_embedding.clone()).await {
            healthy = false;
            details.insert("put_test_error".to_string(), e.to_string());
        } else if let Err(e) = self.get(test_key).await {
            healthy = false;
            details.insert("get_test_error".to_string(), e.to_string());
        } else {
            // Cleanup test entry
            let _ = self.evict(test_key).await;
        }
        
        let response_time_ms = start.elapsed().as_millis() as f32;

        Ok(LayerHealthStatus {
            layer_name: "LRUCacheLayer".to_string(),
            healthy,
            response_time_ms,
            error_rate: if healthy { 0.0 } else { 1.0 },
            last_check: Utc::now(),
            details,
        })
    }

    async fn ready_check(&self) -> Result<bool> {
        // Cache layer всегда готов - в худшем случае работает как pass-through
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn create_test_cache() -> Result<Arc<LRUCacheLayer>> {
        let config = CacheConfig {
            max_size: 1, // 1MB для тестов
            ttl_seconds: 3600,
            enable_prefetch: true,
            cache_path: None, // Без persistence для простоты
        };
        LRUCacheLayer::new(config).await
    }

    #[tokio::test]
    async fn test_cache_creation() -> Result<()> {
        let cache = create_test_cache().await?;
        assert!(cache.ready_check().await?);
        Ok(())
    }

    #[tokio::test]
    async fn test_cache_put_get() -> Result<()> {
        let cache = create_test_cache().await?;
        
        let key = "test_key";
        let embedding = vec![0.1, 0.2, 0.3, 0.4];
        
        // Test put
        cache.put(key, embedding.clone()).await?;
        
        // Test get
        let cached_embedding = cache.get(key).await?;
        assert!(cached_embedding.is_some());
        assert_eq!(cached_embedding.unwrap(), embedding);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_cache_miss() -> Result<()> {
        let cache = create_test_cache().await?;
        
        let result = cache.get("non_existent_key").await?;
        assert!(result.is_none());
        
        let (hits, misses, _) = cache.stats();
        assert_eq!(hits, 0);
        assert_eq!(misses, 1);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_cache_eviction() -> Result<()> {
        let mut config = CacheConfig::default();
        config.max_size = 1; // Очень маленький размер для тестирования eviction
        config.cache_path = None;
        
        let cache = LRUCacheLayer::new(config).await?;
        
        // Добавляем несколько больших embeddings для принуждения eviction
        for i in 0..10 {
            let key = format!("key_{}", i);
            let embedding = vec![i as f32; 1000]; // Большой embedding
            cache.put(&key, embedding).await?;
        }
        
        let (_, _, size) = cache.stats();
        assert!(size < 10); // Должно было произойти eviction
        
        Ok(())
    }

    #[tokio::test]
    async fn test_cache_ttl() -> Result<()> {
        let mut config = CacheConfig::default();
        config.ttl_seconds = 1; // 1 секунда TTL
        config.cache_path = None;
        
        let cache = LRUCacheLayer::new(config).await?;
        
        let key = "ttl_test_key";
        let embedding = vec![0.1, 0.2, 0.3];
        
        // Добавляем в cache
        cache.put(key, embedding.clone()).await?;
        
        // Проверяем что есть в cache
        let result = cache.get(key).await?;
        assert!(result.is_some());
        
        // Ждем истечения TTL
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Проверяем что expired
        let result_after_ttl = cache.get(key).await?;
        assert!(result_after_ttl.is_none());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_batch_operations() -> Result<()> {
        let cache = create_test_cache().await?;
        
        let entries = vec![
            ("key1", vec![0.1, 0.2].as_slice()),
            ("key2", vec![0.3, 0.4].as_slice()),
            ("key3", vec![0.5, 0.6].as_slice()),
        ];
        
        // Test batch put
        cache.put_batch(&entries).await?;
        
        // Verify all entries are cached
        for (key, expected_embedding) in entries {
            let cached = cache.get(key).await?;
            assert!(cached.is_some());
            assert_eq!(cached.unwrap(), expected_embedding.to_vec());
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_cache_clear() -> Result<()> {
        let cache = create_test_cache().await?;
        
        // Add some entries
        cache.put("key1", vec![0.1, 0.2]).await?;
        cache.put("key2", vec![0.3, 0.4]).await?;
        
        let (_, _, size_before) = cache.stats();
        assert_eq!(size_before, 2);
        
        // Clear cache
        cache.clear().await?;
        
        let (_, _, size_after) = cache.stats();
        assert_eq!(size_after, 0);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_health_check() -> Result<()> {
        let cache = create_test_cache().await?;
        
        let health = cache.health_check().await?;
        assert!(health.healthy);
        assert!(health.response_time_ms >= 0.0);
        assert!(health.details.contains_key("total_entries"));
        
        Ok(())
    }

    #[tokio::test]
    async fn test_persistent_cache() -> Result<()> {
        let temp_dir = tempdir()?;
        
        let config = CacheConfig {
            max_size: 1,
            ttl_seconds: 3600,
            enable_prefetch: true,
            cache_path: Some(temp_dir.path().to_path_buf()),
        };
        
        // Create cache and add some data
        {
            let cache = LRUCacheLayer::new(config.clone()).await?;
            cache.put("persistent_key", vec![1.0, 2.0, 3.0]).await?;
            cache.save_persistent_cache().await?;
        }
        
        // Create new cache instance and check if data persisted
        {
            let cache2 = LRUCacheLayer::new(config).await?;
            let result = cache2.get("persistent_key").await?;
            assert!(result.is_some());
            assert_eq!(result.unwrap(), vec![1.0, 2.0, 3.0]);
        }
        
        Ok(())
    }
}