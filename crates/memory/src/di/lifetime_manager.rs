use anyhow::{anyhow, Result};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use parking_lot::RwLock;
use tracing::debug;

use super::traits::{Lifetime, LifetimeManager, MetricsReporter, TypeMetrics};

/// Реализация менеджера жизненного цикла компонентов
/// Применяет принцип Single Responsibility (SRP)
/// Применяет принцип Open/Closed (OCP) - можно расширять новыми типами Lifetime
pub struct LifetimeManagerImpl {
    /// Кэш singleton экземпляров
    singletons: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    /// Кэш scoped экземпляров (будущее расширение)
    #[allow(dead_code)]
    scoped: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    /// Статистика использования кэшей
    cache_stats: RwLock<CacheStats>,
}

#[derive(Debug, Default, Clone)]
struct CacheStats {
    singleton_hits: u64,
    singleton_misses: u64,
    transient_creations: u64,
    total_cache_clears: u64,
}

impl LifetimeManagerImpl {
    /// Создать новый менеджер жизненного цикла
    pub fn new() -> Self {
        Self {
            singletons: RwLock::new(HashMap::new()),
            scoped: RwLock::new(HashMap::new()),
            cache_stats: RwLock::new(CacheStats::default()),
        }
    }

    /// Получить статистику использования кэшей
    pub fn get_cache_stats(&self) -> CacheStats {
        (*self.cache_stats.read()).clone()
    }

    /// Обработать Singleton lifetime
    fn handle_singleton<T>(
        &self,
        type_id: TypeId,
        factory: &dyn Fn() -> Result<Arc<dyn Any + Send + Sync>>,
    ) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        // Сначала пытаемся получить из кэша
        {
            let singletons = self.singletons.read();
            if let Some(cached) = singletons.get(&type_id) {
                // Пытаемся downcasting
                if let Some(typed) = cached.clone().downcast::<T>().ok() {
                    // Обновляем статистику
                    {
                        let mut stats = self.cache_stats.write();
                        stats.singleton_hits += 1;
                    }
                    debug!("Singleton cache hit for type {:?}", type_id);
                    return Ok(typed);
                }
            }
        }

        // Создаём новый экземпляр
        let instance = factory()?;
        
        // Пытаемся downcasting для проверки типа
        let typed_instance = instance
            .clone()
            .downcast::<T>()
            .map_err(|_| anyhow!("Failed to downcast to target type"))?;

        // Сохраняем в кэш
        {
            let mut singletons = self.singletons.write();
            singletons.insert(type_id, instance);
        }

        // Обновляем статистику
        {
            let mut stats = self.cache_stats.write();
            stats.singleton_misses += 1;
        }

        debug!("Created new singleton for type {:?}", type_id);
        Ok(typed_instance)
    }

    /// Обработать Transient lifetime
    fn handle_transient<T>(
        &self,
        factory: &dyn Fn() -> Result<Arc<dyn Any + Send + Sync>>,
    ) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let instance = factory()?;
        
        let typed_instance = instance
            .downcast::<T>()
            .map_err(|_| anyhow!("Failed to downcast to target type"))?;

        // Обновляем статистику
        {
            let mut stats = self.cache_stats.write();
            stats.transient_creations += 1;
        }

        debug!("Created new transient instance");
        Ok(typed_instance)
    }

    /// Обработать Scoped lifetime (будущее расширение)
    #[allow(dead_code)]
    fn handle_scoped<T>(
        &self,
        type_id: TypeId,
        factory: &dyn Fn() -> Result<Arc<dyn Any + Send + Sync>>,
    ) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        // Пока что реализуется так же, как Singleton
        // В будущем здесь будет логика scope management
        self.handle_singleton(type_id, factory)
    }
}

impl Default for LifetimeManagerImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl LifetimeManager for LifetimeManagerImpl {
    fn get_instance<T>(
        &self,
        type_id: TypeId,
        factory: &dyn Fn() -> Result<Arc<dyn Any + Send + Sync>>,
        lifetime: Lifetime,
    ) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let start_time = Instant::now();
        
        let result = match lifetime {
            Lifetime::Singleton => self.handle_singleton::<T>(type_id, factory),
            Lifetime::Transient => self.handle_transient::<T>(factory),
            Lifetime::Scoped => self.handle_scoped::<T>(type_id, factory),
        };

        let duration = start_time.elapsed();
        debug!("Instance resolution took {:?} for {:?} lifetime", duration, lifetime);

        result
    }

    fn clear_caches(&self) {
        {
            let mut singletons = self.singletons.write();
            singletons.clear();
        }
        {
            let mut scoped = self.scoped.write();
            scoped.clear();
        }
        {
            let mut stats = self.cache_stats.write();
            stats.total_cache_clears += 1;
        }
        
        debug!("All lifetime caches cleared");
    }

    fn clear_type_cache(&self, type_id: TypeId) {
        {
            let mut singletons = self.singletons.write();
            singletons.remove(&type_id);
        }
        {
            let mut scoped = self.scoped.write();
            scoped.remove(&type_id);
        }
        
        debug!("Cache cleared for type {:?}", type_id);
    }
}

/// Расширяемый LifetimeManager с поддержкой custom strategies
/// Демонстрирует принцип Open/Closed (OCP)
pub trait LifetimeStrategy: Send + Sync {
    /// Обработать запрос экземпляра для custom lifetime
    fn get_instance(
        &self,
        type_id: TypeId,
        factory: &dyn Fn() -> Result<Arc<dyn Any + Send + Sync>>,
    ) -> Result<Arc<dyn Any + Send + Sync>>;

    /// Очистить кэш для этой стратегии
    fn clear_cache(&self);
}

/// Расширенный LifetimeManager с поддержкой custom strategies
/// Демонстрирует принцип Open/Closed (OCP)
pub struct ExtensibleLifetimeManager {
    core_manager: LifetimeManagerImpl,
    custom_strategies: RwLock<HashMap<String, Arc<dyn LifetimeStrategy>>>,
}

impl ExtensibleLifetimeManager {
    pub fn new() -> Self {
        Self {
            core_manager: LifetimeManagerImpl::new(),
            custom_strategies: RwLock::new(HashMap::new()),
        }
    }

    /// Регистрировать custom lifetime strategy
    /// Применяет принцип Open/Closed (OCP)
    pub fn register_strategy(&self, name: String, strategy: Arc<dyn LifetimeStrategy>) {
        let mut strategies = self.custom_strategies.write();
        strategies.insert(name, strategy);
        debug!("Registered custom lifetime strategy");
    }

    /// Использовать custom strategy
    pub fn get_with_strategy<T>(
        &self,
        strategy_name: &str,
        type_id: TypeId,
        factory: &dyn Fn() -> Result<Arc<dyn Any + Send + Sync>>,
    ) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        let strategies = self.custom_strategies.read();
        if let Some(strategy) = strategies.get(strategy_name) {
            let instance = strategy.get_instance(type_id, factory)?;
            let typed_instance = instance
                .downcast::<T>()
                .map_err(|_| anyhow!("Failed to downcast to target type"))?;
            Ok(typed_instance)
        } else {
            Err(anyhow!("Strategy '{}' not found", strategy_name))
        }
    }
}

impl Default for ExtensibleLifetimeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LifetimeManager for ExtensibleLifetimeManager {
    fn get_instance<T>(
        &self,
        type_id: TypeId,
        factory: &dyn Fn() -> Result<Arc<dyn Any + Send + Sync>>,
        lifetime: Lifetime,
    ) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        // Делегируем основную логику core_manager
        self.core_manager.get_instance(type_id, factory, lifetime)
    }

    fn clear_caches(&self) {
        self.core_manager.clear_caches();
        
        // Очищаем кэши custom strategies
        let strategies = self.custom_strategies.read();
        for strategy in strategies.values() {
            strategy.clear_cache();
        }
    }

    fn clear_type_cache(&self, type_id: TypeId) {
        self.core_manager.clear_type_cache(type_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestService {
        value: i32,
    }

    impl TestService {
        fn new() -> Self {
            Self { value: 42 }
        }
    }

    #[test]
    fn test_singleton_caching() -> Result<()> {
        let manager = LifetimeManagerImpl::new();
        let type_id = TypeId::of::<TestService>();

        let factory = || -> Result<Arc<dyn Any + Send + Sync>> {
            Ok(Arc::new(TestService::new()))
        };

        // Первый запрос - должен создать экземпляр
        let instance1 = manager.get_instance::<TestService>(type_id, &factory, Lifetime::Singleton)?;
        
        // Второй запрос - должен вернуть тот же экземпляр
        let instance2 = manager.get_instance::<TestService>(type_id, &factory, Lifetime::Singleton)?;

        // Проверяем, что это тот же экземпляр (по адресу)
        assert!(Arc::ptr_eq(&instance1, &instance2));

        let stats = manager.get_cache_stats();
        assert_eq!(stats.singleton_hits, 1);
        assert_eq!(stats.singleton_misses, 1);

        Ok(())
    }

    #[test]
    fn test_transient_creation() -> Result<()> {
        let manager = LifetimeManagerImpl::new();
        let type_id = TypeId::of::<TestService>();

        let factory = || -> Result<Arc<dyn Any + Send + Sync>> {
            Ok(Arc::new(TestService::new()))
        };

        // Два запроса transient должны создать разные экземпляры
        let instance1 = manager.get_instance::<TestService>(type_id, &factory, Lifetime::Transient)?;
        let instance2 = manager.get_instance::<TestService>(type_id, &factory, Lifetime::Transient)?;

        // Проверяем, что это разные экземпляры
        assert!(!Arc::ptr_eq(&instance1, &instance2));

        let stats = manager.get_cache_stats();
        assert_eq!(stats.transient_creations, 2);

        Ok(())
    }

    #[test]
    fn test_cache_clearing() -> Result<()> {
        let manager = LifetimeManagerImpl::new();
        let type_id = TypeId::of::<TestService>();

        let factory = || -> Result<Arc<dyn Any + Send + Sync>> {
            Ok(Arc::new(TestService::new()))
        };

        // Создаём singleton
        let _instance1 = manager.get_instance::<TestService>(type_id, &factory, Lifetime::Singleton)?;
        
        // Очищаем кэш
        manager.clear_caches();
        
        // Следующий запрос должен создать новый экземпляр
        let _instance2 = manager.get_instance::<TestService>(type_id, &factory, Lifetime::Singleton)?;

        let stats = manager.get_cache_stats();
        assert_eq!(stats.singleton_misses, 2); // Два промаха из-за очистки кэша
        assert_eq!(stats.total_cache_clears, 1);

        Ok(())
    }
}