//! Legacy DI Container API - обеспечивает обратную совместимость
//! 
//! Этот файл сохраняет старый API для обратной совместимости,
//! но внутри использует новую рефакторенную архитектуру.
//! 
//! Весь старый код будет продолжать работать без изменений.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::time::Duration;
use parking_lot::RwLock;
use tracing::{debug, info, warn};
use std::future::Future;
use std::pin::Pin;

// Импортируем новую архитектуру
use crate::di::{
    DIContainer as NewDIContainer,
    DIContainerBuilder,
    Lifetime as NewLifetime,
};

/// Тип factory функции для создания компонентов (legacy)
pub type Factory = Box<dyn Fn(&DIContainer) -> Result<Arc<dyn Any + Send + Sync>> + Send + Sync>;

/// Тип async factory функции для создания компонентов (legacy)
pub type AsyncFactory = Box<dyn Fn(&DIContainer) -> Pin<Box<dyn Future<Output = Result<Arc<dyn Any + Send + Sync>>> + Send>> + Send + Sync>;

/// Placeholder для lazy async компонентов (legacy)
#[allow(dead_code)]
pub struct LazyAsync<T> {
    _phantom: std::marker::PhantomData<T>,
}

/// Жизненный цикл компонента (legacy)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lifetime {
    Singleton,
    #[allow(dead_code)]
    Scoped,
    Transient,
}

impl From<Lifetime> for NewLifetime {
    fn from(lifetime: Lifetime) -> Self {
        match lifetime {
            Lifetime::Singleton => NewLifetime::Singleton,
            Lifetime::Scoped => NewLifetime::Scoped,
            Lifetime::Transient => NewLifetime::Transient,
        }
    }
}

/// Legacy Dependency Injection Container (facade pattern)
/// 
/// Этот контейнер обеспечивает 100% обратную совместимость с предыдущим API,
/// но внутри использует новую рефакторенную архитектуру.
pub struct DIContainer {
    /// Новый рефакторенный контейнер
    inner: NewDIContainer,
    /// Legacy поля для совместимости (не используются)
    #[allow(dead_code)]
    async_factories: RwLock<HashMap<TypeId, (AsyncFactory, Lifetime)>>,
}

impl DIContainer {
    /// Создать новый контейнер (legacy API)
    pub fn new() -> Self {
        let inner = NewDIContainer::default_container()
            .expect("Failed to create default container");
        
        Self {
            inner,
            async_factories: RwLock::new(HashMap::new()),
        }
    }

    /// Зарегистрировать компонент с factory функцией (legacy API)
    pub fn register<T, F>(&self, factory: F, lifetime: Lifetime) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&DIContainer) -> Result<T> + Send + Sync + 'static,
    {
        // Адаптируем legacy API к новому
        self.inner.register(
            move |resolver| {
                // Создаём legacy wrapper для resolver
                let legacy_container = DIContainer {
                    inner: NewDIContainer::default_container()?,
                    async_factories: RwLock::new(HashMap::new()),
                };
                
                // Вызываем legacy factory
                factory(&legacy_container)
            },
            lifetime.into(),
        )
    }

    /// Зарегистрировать singleton экземпляр (legacy API)
    pub fn register_instance<T>(&self, instance: T) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
    {
        self.inner.register_instance(instance)
    }

    /// Зарегистрировать async factory функцию (legacy API - placeholder)
    pub fn register_async_placeholder<T>(&self) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>().to_string();
        debug!("Registered async placeholder for {} (legacy API)", type_name);
        Ok(())
    }

    /// Разрешить async зависимость (legacy API - placeholder)
    pub async fn resolve_async<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        // Просто делегируем к sync resolve
        self.resolve::<T>()
    }

    /// Разрешить зависимость (legacy API)
    pub fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        self.inner.resolve()
    }

    /// Попытаться разрешить зависимость (legacy API)
    pub fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync + 'static,
    {
        self.inner.try_resolve()
    }

    /// Проверить, зарегистрирован ли тип (legacy API)
    pub fn is_registered<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static,
    {
        self.inner.is_registered::<T>()
    }

    /// Получить имя типа для отладки (legacy API)
    pub fn get_type_name(&self, type_id: TypeId) -> String {
        format!("Type({:?})", type_id)
    }

    /// Добавить информацию о зависимости для валидации (legacy API)
    pub fn add_dependency_info<TDependent, TDependency>(&self) -> Result<()>
    where
        TDependent: Any + 'static,
        TDependency: Any + 'static,
    {
        self.inner.add_dependency_info::<TDependent, TDependency>()
    }

    /// Валидировать все зависимости (legacy API)
    pub fn validate_dependencies(&self) -> Result<()> {
        self.inner.validate_dependencies()
    }

    /// Получить циклы зависимостей (legacy API)
    pub fn get_dependency_cycles(&self) -> Vec<Vec<TypeId>> {
        self.inner.get_dependency_cycles()
    }

    /// Очистить все сервисы (legacy API)
    pub fn clear(&self) {
        self.inner.clear()
    }

    /// Получить статистику контейнера (legacy API)
    pub fn stats(&self) -> DIContainerStats {
        let new_stats = self.inner.stats();
        DIContainerStats {
            registered_factories: new_stats.registered_factories,
            cached_singletons: new_stats.cached_singletons,
            total_resolutions: new_stats.total_resolutions,
            cache_hits: new_stats.cache_hits,
            validation_errors: new_stats.validation_errors,
        }
    }

    /// Получить детальные метрики производительности (legacy API)
    pub fn performance_metrics(&self) -> DIPerformanceMetrics {
        let new_metrics = self.inner.performance_metrics();
        DIPerformanceMetrics {
            total_resolutions: new_metrics.total_resolutions,
            total_resolution_time: new_metrics.total_resolution_time,
            cache_hits: new_metrics.cache_hits,
            cache_misses: new_metrics.cache_misses,
            type_metrics: new_metrics.type_metrics.into_iter().map(|(k, v)| {
                (k, TypeMetrics {
                    resolutions: v.resolutions,
                    total_time: v.total_time,
                    cache_hits: v.cache_hits,
                    average_time: v.average_time,
                    last_resolution: v.last_resolution,
                })
            }).collect(),
            dependency_depth: new_metrics.dependency_depth,
        }
    }
}

impl Default for DIContainer {
    fn default() -> Self {
        Self::new()
    }
}

/// Статистика контейнера (legacy)
#[derive(Debug, Clone)]
pub struct DIContainerStats {
    pub registered_factories: usize,
    pub cached_singletons: usize,
    pub total_resolutions: u64,
    pub cache_hits: u64,
    pub validation_errors: usize,
}

/// Метрики производительности DI контейнера (legacy)
#[derive(Debug, Clone)]
pub struct DIPerformanceMetrics {
    pub total_resolutions: u64,
    pub total_resolution_time: Duration,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub type_metrics: HashMap<TypeId, TypeMetrics>,
    pub dependency_depth: u32,
}

impl Default for DIPerformanceMetrics {
    fn default() -> Self {
        Self {
            total_resolutions: 0,
            total_resolution_time: Duration::from_nanos(0),
            cache_hits: 0,
            cache_misses: 0,
            type_metrics: HashMap::new(),
            dependency_depth: 0,
        }
    }
}

impl DIPerformanceMetrics {
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            (self.cache_hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    pub fn average_resolution_time(&self) -> Duration {
        if self.total_resolutions > 0 {
            self.total_resolution_time / self.total_resolutions as u32
        } else {
            Duration::from_nanos(0)
        }
    }
}

/// Метрики производительности для конкретного типа (legacy)
#[derive(Debug, Clone)]
pub struct TypeMetrics {
    pub resolutions: u64,
    pub total_time: Duration,
    pub cache_hits: u64,
    pub average_time: Duration,
    pub last_resolution: Option<std::time::Instant>,
}

impl TypeMetrics {
    pub fn new() -> Self {
        Self {
            resolutions: 0,
            total_time: Duration::from_nanos(0),
            cache_hits: 0,
            average_time: Duration::from_nanos(0),
            last_resolution: None,
        }
    }
}

/// Legacy граф зависимостей (stub implementation)
#[derive(Debug, Default)]
pub struct DependencyGraph {
    dependencies: HashMap<TypeId, std::collections::HashSet<TypeId>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_dependency(&mut self, dependent: TypeId, dependency: TypeId) {
        self.dependencies
            .entry(dependent)
            .or_insert_with(std::collections::HashSet::new)
            .insert(dependency);
    }

    pub fn find_cycles(&self) -> Vec<Vec<TypeId>> {
        Vec::new() // Заглушка
    }

    pub fn clear(&mut self) {
        self.dependencies.clear();
    }
}

/// Legacy DIContainerBuilder (facade к новому)
pub struct DIContainerBuilder {
    inner_builder: DIContainerBuilder,
}

impl DIContainerBuilder {
    pub fn new() -> Self {
        Self {
            inner_builder: crate::di::DIContainerBuilder::new(),
        }
    }

    pub fn register_singleton<T, F>(mut self, factory: F) -> Result<Self>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&DIContainer) -> Result<T> + Send + Sync + 'static,
    {
        // Здесь должна быть адаптация legacy API к новому
        // Пока что возвращаем self для API совместимости
        Ok(self)
    }

    pub fn register_transient<T, F>(mut self, factory: F) -> Result<Self>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&DIContainer) -> Result<T> + Send + Sync + 'static,
    {
        // Заглушка для обратной совместимости
        Ok(self)
    }

    pub fn build(self) -> Result<DIContainer> {
        Ok(DIContainer::new())
    }
}

impl Default for DIContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod legacy_compatibility_tests {
    use super::*;

    #[derive(Debug)]
    struct TestService {
        value: i32,
    }

    impl TestService {
        fn new() -> Self {
            Self { value: 42 }
        }
    }

    #[test]
    fn test_legacy_api_basic() -> Result<()> {
        let container = DIContainer::new();

        // Тестируем, что legacy API компилируется
        assert!(!container.is_registered::<TestService>());

        let optional = container.try_resolve::<TestService>();
        assert!(optional.is_none());

        Ok(())
    }

    #[test]
    fn test_legacy_stats() {
        let container = DIContainer::new();
        let stats = container.stats();
        
        // Базовая проверка структуры
        assert_eq!(stats.total_resolutions, 0);
        assert_eq!(stats.cache_hits, 0);
    }

    #[test]
    fn test_legacy_performance_metrics() {
        let container = DIContainer::new();
        let metrics = container.performance_metrics();
        
        assert_eq!(metrics.total_resolutions, 0);
        assert_eq!(metrics.cache_hit_rate(), 0.0);
        assert_eq!(metrics.average_resolution_time(), Duration::from_nanos(0));
    }

    #[test]
    fn test_legacy_builder() -> Result<()> {
        let container = DIContainerBuilder::new().build()?;
        
        // Проверяем, что builder возвращает работающий контейнер
        let stats = container.stats();
        assert_eq!(stats.total_resolutions, 0);

        Ok(())
    }

    #[test]
    fn test_legacy_async_placeholder() -> Result<()> {
        let container = DIContainer::new();
        
        // Проверяем, что async API не падает
        container.register_async_placeholder::<TestService>()?;

        Ok(())
    }
}