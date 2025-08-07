//! DI Container Cache Manager - оптимизированное кэширование экземпляров
//!
//! ЕДИНСТВЕННАЯ ОТВЕТСТВЕННОСТЬ: Кэширование singleton и scoped экземпляров
//! PERFORMANCE FOCUS: Lock-free patterns, memory-efficient storage

use parking_lot::RwLock;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::{debug, warn};

use super::traits::Lifetime;

/// Cache entry для компонентов
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub instance: Arc<dyn Any + Send + Sync>,
    pub created_at: Instant,
    pub access_count: u64,
    pub last_access: Instant,
}

impl CacheEntry {
    pub fn new(instance: Arc<dyn Any + Send + Sync>) -> Self {
        let now = Instant::now();
        Self {
            instance,
            created_at: now,
            access_count: 1,
            last_access: now,
        }
    }

    pub fn accessed(&mut self) {
        self.access_count += 1;
        self.last_access = Instant::now();
    }

    /// Определить, устарел ли entry
    pub fn is_expired(&self, max_age: Duration) -> bool {
        self.created_at.elapsed() > max_age
    }

    /// Определить, неактивен ли entry
    pub fn is_inactive(&self, max_idle: Duration) -> bool {
        self.last_access.elapsed() > max_idle
    }
}

/// Конфигурация кэша
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_size: usize,
    pub max_age: Duration,
    pub max_idle_time: Duration,
    pub cleanup_interval: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 1000,
            max_age: Duration::from_secs(3600),         // 1 hour
            max_idle_time: Duration::from_secs(600),    // 10 minutes
            cleanup_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Высокопроизводительный кэш-менеджер для DI контейнера
pub struct ContainerCache {
    /// Cached singleton instances
    singleton_cache: RwLock<HashMap<TypeId, CacheEntry>>,
    /// Cached scoped instances
    scoped_cache: RwLock<HashMap<TypeId, CacheEntry>>,
    /// Configuration
    config: CacheConfig,
    /// Last cleanup time
    last_cleanup: RwLock<Instant>,
}

impl ContainerCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            singleton_cache: RwLock::new(HashMap::new()),
            scoped_cache: RwLock::new(HashMap::new()),
            config,
            last_cleanup: RwLock::new(Instant::now()),
        }
    }

    /// Получить экземпляр из кэша
    pub fn get<T: Any + Send + Sync>(&self, type_id: TypeId, lifetime: Lifetime) -> Option<Arc<T>> {
        let mut cache_guard = match lifetime {
            Lifetime::Singleton => Some(self.singleton_cache.write()),
            Lifetime::Scoped => Some(self.scoped_cache.write()),
            Lifetime::Transient => None,
        };

        if let Some(ref mut cache) = cache_guard {
            if let Some(entry) = cache.get_mut(&type_id) {
                entry.accessed();

                // Try downcast
                if let Ok(instance) = entry.instance.clone().downcast::<T>() {
                    debug!("Cache hit for type: {:?}", type_id);
                    return Some(instance);
                } else {
                    warn!("Failed to downcast cached instance for type: {:?}", type_id);
                    cache.remove(&type_id);
                }
            }
        }

        debug!("Cache miss for type: {:?}", type_id);
        None
    }

    /// Сохранить экземпляр в кэш
    pub fn store<T: Any + Send + Sync>(
        &self,
        type_id: TypeId,
        instance: Arc<T>,
        lifetime: Lifetime,
    ) {
        match lifetime {
            Lifetime::Singleton => {
                let mut cache = self.singleton_cache.write();
                let entry = CacheEntry::new(instance as Arc<dyn Any + Send + Sync>);
                cache.insert(type_id, entry);
                debug!("Stored singleton in cache: {:?}", type_id);
            }
            Lifetime::Scoped => {
                let mut cache = self.scoped_cache.write();
                let entry = CacheEntry::new(instance as Arc<dyn Any + Send + Sync>);
                cache.insert(type_id, entry);
                debug!("Stored scoped instance in cache: {:?}", type_id);
            }
            Lifetime::Transient => {
                // Transient instances не кэшируются
                debug!("Skipping cache for transient type: {:?}", type_id);
            }
        }

        self.maybe_cleanup();
    }

    /// Очистить кэш для конкретного типа
    pub fn evict(&self, type_id: TypeId) {
        {
            let mut singleton_cache = self.singleton_cache.write();
            singleton_cache.remove(&type_id);
        }
        {
            let mut scoped_cache = self.scoped_cache.write();
            scoped_cache.remove(&type_id);
        }
        debug!("Evicted type from cache: {:?}", type_id);
    }

    /// Очистить весь кэш
    pub fn clear(&self) {
        {
            let mut singleton_cache = self.singleton_cache.write();
            singleton_cache.clear();
        }
        {
            let mut scoped_cache = self.scoped_cache.write();
            scoped_cache.clear();
        }
        debug!("Cleared all cache entries");
    }

    /// Очистить скопированные экземпляры
    pub fn clear_scoped(&self) {
        let mut scoped_cache = self.scoped_cache.write();
        scoped_cache.clear();
        debug!("Cleared scoped cache entries");
    }

    /// Получить статистику кэша
    pub fn stats(&self) -> CacheStats {
        let singleton_count = self.singleton_cache.read().len();
        let scoped_count = self.scoped_cache.read().len();

        CacheStats {
            singleton_instances: singleton_count,
            scoped_instances: scoped_count,
            total_instances: singleton_count + scoped_count,
            cache_size_limit: self.config.max_size,
            cache_utilization: ((singleton_count + scoped_count) as f64
                / self.config.max_size as f64
                * 100.0),
        }
    }

    /// Проверить, нужна ли очистка, и выполнить её
    fn maybe_cleanup(&self) {
        let should_cleanup = {
            let last_cleanup = self.last_cleanup.read();
            last_cleanup.elapsed() > self.config.cleanup_interval
        };

        if should_cleanup {
            self.cleanup_expired();
        }
    }

    /// Очистить устаревшие entries
    fn cleanup_expired(&self) {
        let now = Instant::now();

        // Очистка singleton cache
        {
            let mut singleton_cache = self.singleton_cache.write();
            let initial_size = singleton_cache.len();

            singleton_cache.retain(|_, entry| {
                !entry.is_expired(self.config.max_age)
                    && !entry.is_inactive(self.config.max_idle_time)
            });

            let removed = initial_size - singleton_cache.len();
            if removed > 0 {
                debug!("Cleaned up {} expired singleton entries", removed);
            }
        }

        // Очистка scoped cache
        {
            let mut scoped_cache = self.scoped_cache.write();
            let initial_size = scoped_cache.len();

            scoped_cache.retain(|_, entry| {
                !entry.is_expired(self.config.max_age)
                    && !entry.is_inactive(self.config.max_idle_time)
            });

            let removed = initial_size - scoped_cache.len();
            if removed > 0 {
                debug!("Cleaned up {} expired scoped entries", removed);
            }
        }

        // Обновить время последней очистки
        {
            let mut last_cleanup = self.last_cleanup.write();
            *last_cleanup = now;
        }
    }

    /// Принудительная очистка для освобождения памяти
    pub fn force_cleanup(&self) {
        self.cleanup_expired();

        // Если всё ещё превышаем лимит, удаляем наименее используемые
        self.evict_lru();
    }

    /// Удаление наименее используемых entries
    fn evict_lru(&self) {
        // Применить LRU eviction только если превышен лимит размера
        let total_size = {
            let singleton_count = self.singleton_cache.read().len();
            let scoped_count = self.scoped_cache.read().len();
            singleton_count + scoped_count
        };

        if total_size <= self.config.max_size {
            return;
        }

        let target_size = (self.config.max_size as f64 * 0.8) as usize;
        let to_remove = total_size - target_size;

        debug!("Starting LRU eviction: removing {} entries", to_remove);

        // Собираем все entries для сортировки по last_access
        let mut all_entries: Vec<(TypeId, Instant, bool)> = Vec::new(); // (type_id, last_access, is_singleton)

        {
            let singleton_cache = self.singleton_cache.read();
            for (type_id, entry) in singleton_cache.iter() {
                all_entries.push((*type_id, entry.last_access, true));
            }
        }

        {
            let scoped_cache = self.scoped_cache.read();
            for (type_id, entry) in scoped_cache.iter() {
                all_entries.push((*type_id, entry.last_access, false));
            }
        }

        // Сортировка по last_access (oldest first)
        all_entries.sort_by(|a, b| a.1.cmp(&b.1));

        // Удаляем oldest entries
        let mut removed = 0;
        for (type_id, _, is_singleton) in all_entries.into_iter().take(to_remove) {
            if is_singleton {
                let mut singleton_cache = self.singleton_cache.write();
                singleton_cache.remove(&type_id);
            } else {
                let mut scoped_cache = self.scoped_cache.write();
                scoped_cache.remove(&type_id);
            }
            removed += 1;
        }

        debug!("LRU eviction completed: removed {} entries", removed);
    }
}

impl Default for ContainerCache {
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

/// Статистика кэша
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub singleton_instances: usize,
    pub scoped_instances: usize,
    pub total_instances: usize,
    pub cache_size_limit: usize,
    pub cache_utilization: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    struct TestService {
        value: String,
    }

    #[test]
    fn test_singleton_caching() {
        let cache = ContainerCache::default();
        let type_id = TypeId::of::<TestService>();
        let instance = Arc::new(TestService {
            value: "test".to_string(),
        });

        // Store singleton
        cache.store(type_id, instance.clone(), Lifetime::Singleton);

        // Retrieve singleton
        let retrieved: Option<Arc<TestService>> = cache.get(type_id, Lifetime::Singleton);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, "test");
    }

    #[test]
    fn test_transient_not_cached() {
        let cache = ContainerCache::default();
        let type_id = TypeId::of::<TestService>();
        let instance = Arc::new(TestService {
            value: "test".to_string(),
        });

        // Store transient (should not cache)
        cache.store(type_id, instance.clone(), Lifetime::Transient);

        // Retrieve transient (should miss)
        let retrieved: Option<Arc<TestService>> = cache.get(type_id, Lifetime::Transient);
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_cache_eviction() {
        let cache = ContainerCache::default();
        let type_id = TypeId::of::<TestService>();
        let instance = Arc::new(TestService {
            value: "test".to_string(),
        });

        cache.store(type_id, instance, Lifetime::Singleton);

        // Verify stored
        assert!(cache
            .get::<TestService>(type_id, Lifetime::Singleton)
            .is_some());

        // Evict
        cache.evict(type_id);

        // Verify evicted
        assert!(cache
            .get::<TestService>(type_id, Lifetime::Singleton)
            .is_none());
    }

    #[test]
    fn test_cache_stats() {
        let cache = ContainerCache::default();
        let type_id = TypeId::of::<TestService>();
        let instance = Arc::new(TestService {
            value: "test".to_string(),
        });

        let stats_before = cache.stats();
        assert_eq!(stats_before.total_instances, 0);

        cache.store(type_id, instance, Lifetime::Singleton);

        let stats_after = cache.stats();
        assert_eq!(stats_after.singleton_instances, 1);
        assert_eq!(stats_after.total_instances, 1);
    }

    #[test]
    fn test_expired_cleanup() {
        let config = CacheConfig {
            max_age: Duration::from_millis(100),
            max_idle_time: Duration::from_millis(50),
            ..Default::default()
        };
        let cache = ContainerCache::new(config);
        let type_id = TypeId::of::<TestService>();
        let instance = Arc::new(TestService {
            value: "test".to_string(),
        });

        cache.store(type_id, instance, Lifetime::Singleton);

        // Should be available immediately
        assert!(cache
            .get::<TestService>(type_id, Lifetime::Singleton)
            .is_some());

        // Wait for expiration
        sleep(Duration::from_millis(150));

        // Force cleanup
        cache.force_cleanup();

        // Should be evicted
        assert!(cache
            .get::<TestService>(type_id, Lifetime::Singleton)
            .is_none());
    }
}
