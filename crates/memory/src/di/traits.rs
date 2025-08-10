use anyhow::Result;
use std::{
    any::{Any, TypeId},
    sync::Arc,
    time::Duration,
};

use super::object_safe_resolver::ObjectSafeResolver;

/// Основной trait для Dependency Injection контейнера
/// Применяет принцип Interface Segregation (ISP)
pub trait DIResolver {
    /// Разрешить зависимость
    fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static;

    /// Попытаться разрешить зависимость (возвращает None если не найдена)
    fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: Any + Send + Sync + 'static;

    /// Проверить, зарегистрирован ли тип
    fn is_registered<T>(&self) -> bool
    where
        T: Any + Send + Sync + 'static;
}

/// Trait для регистрации компонентов
/// Применяет принцип Interface Segregation (ISP)
pub trait DIRegistrar {
    /// Зарегистрировать компонент с factory функцией
    fn register<T, F>(&self, factory: F, lifetime: Lifetime) -> Result<()>
    where
        T: Any + Send + Sync + 'static,
        F: Fn(&dyn ObjectSafeResolver) -> Result<T> + Send + Sync + 'static;

    /// Зарегистрировать singleton экземпляр
    fn register_instance<T>(&self, instance: T) -> Result<()>
    where
        T: Any + Send + Sync + 'static;
}

/// Trait для управления жизненным циклом компонентов
/// Применяет принципы Open/Closed и Dependency Inversion
pub trait LifetimeManager: Send + Sync {
    /// Получить экземпляр с учетом жизненного цикла
    fn get_instance<T>(
        &self,
        type_id: TypeId,
        factory: &dyn Fn() -> Result<Arc<dyn Any + Send + Sync>>,
        lifetime: Lifetime,
    ) -> Result<Arc<T>>
    where
        T: Any + Send + Sync + 'static;

    /// Очистить кэши для всех типов lifetimes
    fn clear_caches(&self);

    /// Очистить кэш для конкретного типа
    fn clear_type_cache(&self, type_id: TypeId);
}

/// Trait для валидации зависимостей
/// Применяет принцип Single Responsibility
pub trait DependencyValidator: Send + Sync {
    /// Добавить информацию о зависимости между типами
    fn add_dependency(&self, dependent: TypeId, dependency: TypeId) -> Result<()>;

    /// Валидировать все зависимости на отсутствие циклов
    fn validate(&self) -> Result<()>;

    /// Получить все найденные циклы зависимостей
    fn get_cycles(&self) -> Vec<Vec<TypeId>>;

    /// Очистить граф зависимостей
    fn clear(&self);
}

/// Trait для сбора метрик производительности
/// Применяет принцип Dependency Inversion
pub trait MetricsReporter: Send + Sync {
    /// Зафиксировать время разрешения зависимости
    fn record_resolution(&self, type_id: TypeId, duration: Duration, from_cache: bool);

    /// Зафиксировать регистрацию нового типа
    fn record_registration(&self, type_id: TypeId);

    /// Получить статистику контейнера
    fn get_stats(&self) -> DIContainerStats;

    /// Получить детальные метрики производительности
    fn get_performance_metrics(&self) -> DIPerformanceMetrics;

    /// Очистить все метрики
    fn clear_metrics(&self);
}

/// Жизненный цикл компонента
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lifetime {
    /// Singleton - один экземпляр на всё приложение
    Singleton,
    /// Scoped - один экземпляр на scope
    Scoped,
    /// Transient - новый экземпляр каждый раз
    Transient,
}

/// Статистика контейнера
#[derive(Debug, Clone)]
pub struct DIContainerStats {
    pub registered_factories: usize,
    pub cached_singletons: usize,
    pub total_resolutions: u64,
    pub cache_hits: u64,
    pub validation_errors: usize,
}

/// Метрики производительности DI контейнера
#[derive(Debug, Clone)]
pub struct DIPerformanceMetrics {
    pub total_resolutions: u64,
    pub total_resolution_time: Duration,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub error_count: u64,
    pub type_metrics: std::collections::HashMap<TypeId, TypeMetrics>,
    pub dependency_depth: u32,
}

impl Default for DIPerformanceMetrics {
    fn default() -> Self {
        Self {
            total_resolutions: 0,
            total_resolution_time: Duration::from_nanos(0),
            cache_hits: 0,
            cache_misses: 0,
            error_count: 0,
            type_metrics: std::collections::HashMap::new(),
            dependency_depth: 0,
        }
    }
}

impl DIPerformanceMetrics {
    /// Рассчитать процент попаданий в кэш
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            (self.cache_hits as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Получить среднее время разрешения в микросекундах
    pub fn avg_resolve_time_us(&self) -> f64 {
        if self.total_resolutions > 0 {
            let avg_duration = self.total_resolution_time / self.total_resolutions as u32;
            avg_duration.as_nanos() as f64 / 1000.0
        } else {
            0.0
        }
    }

    /// Получить топ N самых медленных типов
    pub fn slowest_types(&self, limit: usize) -> Vec<(TypeId, TypeMetrics)> {
        let mut types: Vec<_> = self
            .type_metrics
            .iter()
            .map(|(&type_id, metrics)| (type_id, metrics.clone()))
            .collect();

        // Сортируем по среднему времени (от медленнее к быстрее)
        types.sort_by(|a, b| b.1.average_time.cmp(&a.1.average_time));

        types.into_iter().take(limit).collect()
    }

    /// Добавить поля total_resolves и factory_creates для обратной совместимости
    /// (они отображаются на существующие поля)
    pub fn total_resolves(&self) -> u64 {
        self.total_resolutions
    }

    pub fn factory_creates(&self) -> u64 {
        // factory_creates можно приблизительно оценить как количество cache misses
        // (поскольку cache miss означает что нужно создать новый экземпляр)
        self.cache_misses
    }
}

/// Метрики производительности для конкретного типа
#[derive(Debug, Clone)]
pub struct TypeMetrics {
    pub resolutions: u64,
    pub total_time: Duration,
    pub cache_hits: u64,
    pub average_time: Duration,
    pub last_resolution: Option<std::time::Instant>,
    pub error_count: u64,
}

impl TypeMetrics {
    pub fn new() -> Self {
        Self {
            resolutions: 0,
            total_time: Duration::from_nanos(0),
            cache_hits: 0,
            average_time: Duration::from_nanos(0),
            last_resolution: None,
            error_count: 0,
        }
    }
}

impl Default for TypeMetrics {
    fn default() -> Self { Self::new() }
}
