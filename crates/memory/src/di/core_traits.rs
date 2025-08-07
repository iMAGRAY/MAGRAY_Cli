//! Core DI Traits - Single Source of Truth для всей DI системы
//!
//! Этот модуль определяет единственные корректные абстракции для DI,
//! заменяющие множественные дублированные интерфейсы в проекте.
//!
//! АРХИТЕКТУРНЫЕ ПРИНЦИПЫ:
//! - Interface Segregation Principle: Каждый trait имеет единственную ответственность
//! - Dependency Inversion Principle: Все зависят от этих абстракций, а не от конкретных типов
//! - Single Responsibility Principle: Разделены регистрация, разрешение, валидация

use crate::di::errors::{DIError, ValidationError};
use anyhow::Result;
use std::any::{Any, TypeId};
use std::sync::Arc;

/// === GENERIC SERVICE REGISTRY ===
/// Generic методы для type-safe регистрации (НЕ object-safe)
pub trait ServiceRegistry: Send + Sync {
    /// Зарегистрировать transient сервис
    fn register_transient<T, F>(&self, factory: F) -> Result<(), DIError>
    where
        T: Send + Sync + 'static,
        F: Fn(&Self) -> Result<T, DIError> + Send + Sync + 'static;

    /// Зарегистрировать singleton сервис
    fn register_singleton<T, F>(&self, factory: F) -> Result<(), DIError>
    where
        T: Send + Sync + 'static,
        F: Fn(&Self) -> Result<T, DIError> + Send + Sync + 'static;

    /// Проверить зарегистрирован ли тип
    fn is_registered<T: 'static>(&self) -> bool;

    /// Получить количество зарегистрированных типов
    fn registration_count(&self) -> usize;
}

/// === OBJECT-SAFE SERVICE REGISTRY ===
/// Non-generic методы для dynamic dispatch (object-safe)
pub trait DynamicServiceRegistry: Send + Sync {
    /// Зарегистрировать type-erased сервис
    fn register_type_erased(
        &self,
        type_id: TypeId,
        type_name: &str,
        factory: ServiceFactory,
        lifetime: LifetimeStrategy,
    ) -> Result<(), DIError>;

    /// Проверить зарегистрирован ли тип по TypeId
    fn is_type_registered(&self, type_id: TypeId) -> bool;

    /// Получить количество зарегистрированных типов
    fn registration_count(&self) -> usize;

    /// Получить список зарегистрированных типов
    fn get_registered_types(&self) -> Vec<(TypeId, String)>;
}

/// === GENERIC SERVICE RESOLVER ===
/// Generic методы для type-safe разрешения (НЕ object-safe)
pub trait ServiceResolver: Send + Sync {
    /// Разрешить зависимость (основной метод)
    fn resolve<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, DIError>;

    /// Попытаться разрешить зависимость (возвращает Option)
    fn try_resolve<T: Send + Sync + 'static>(&self) -> Option<Arc<T>>;

    /// Разрешить с указанием lifetime стратегии
    fn resolve_with_strategy<T: Send + Sync + 'static>(
        &self,
        strategy: LifetimeStrategy,
    ) -> Result<Arc<T>, DIError>;
}

/// === OBJECT-SAFE SERVICE RESOLVER ===
/// Non-generic методы для dynamic dispatch (object-safe)
pub trait DynamicServiceResolver: Send + Sync {
    /// Разрешить type-erased зависимость по TypeId
    fn resolve_type_erased(&self, type_id: TypeId) -> Result<Arc<dyn Any + Send + Sync>, DIError>;

    /// Попытаться разрешить type-erased зависимость
    fn try_resolve_type_erased(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>>;

    /// Разрешить с lifetime стратегией
    fn resolve_with_lifetime(
        &self,
        type_id: TypeId,
        strategy: LifetimeStrategy,
    ) -> Result<Arc<dyn Any + Send + Sync>, DIError>;

    /// Проверить возможность разрешения типа
    fn can_resolve(&self, type_id: TypeId) -> bool;
}

/// === CORE ABSTRACTION: Dependency Validator ===
/// Отвечает ТОЛЬКО за валидацию зависимостей (SRP)
pub trait DependencyValidator: Send + Sync {
    /// Валидировать отсутствие циркулярных зависимостей
    fn validate_no_cycles(&self) -> Result<(), ValidationError>;

    /// Добавить информацию о зависимости для валидации
    fn add_dependency_by_type_id(
        &self,
        from_type: TypeId,
        to_type: TypeId,
    ) -> Result<(), ValidationError>;

    /// Получить список обнаруженных циклов
    fn get_cycles(&self) -> Vec<Vec<TypeId>>;

    /// Валидировать все зарегистрированные типы могут быть разрешены
    fn validate_all_resolvable_by_type_id(
        &self,
        type_ids: &[TypeId],
    ) -> Result<(), ValidationError>;
}

/// === EXTENSION TRAIT WITH GENERIC METHODS ===
/// Generic методы для compile-time type safety (НЕ object-safe)
pub trait DependencyValidatorExt: DependencyValidator {
    /// Helper для добавления зависимости с типами времени компиляции
    fn add_dependency<From: 'static, To: 'static>(&self) -> Result<(), ValidationError> {
        self.add_dependency_by_type_id(TypeId::of::<From>(), TypeId::of::<To>())
    }

    /// Валидировать все зарегистрированные типы могут быть разрешены (generic версия)
    fn validate_all_resolvable(
        &self,
        resolver: &dyn DynamicServiceResolver,
    ) -> Result<(), ValidationError>;
}

/// === CORE ABSTRACTION: Container Metrics ===
/// Отвечает ТОЛЬКО за сбор метрик и статистики (SRP)
pub trait ContainerMetrics: Send + Sync {
    /// Записать успешное разрешение
    fn record_resolution_success(&self, type_id: TypeId, duration_ns: u64);

    /// Записать неудачное разрешение
    fn record_resolution_failure(&self, type_id: TypeId, error: &DIError);

    /// Записать cache hit
    fn record_cache_hit(&self, type_id: TypeId);

    /// Получить статистику разрешений
    fn get_resolution_stats(&self) -> ResolutionStats;

    /// Получить статистику кэша
    fn get_cache_stats(&self) -> CacheStats;

    /// Сбросить все метрики
    fn reset_metrics(&self);
}

/// === GENERIC CONTAINER INTERFACE ===
/// Композиция generic interfaces для type-safe использования
pub trait DIContainer: ServiceRegistry + ServiceResolver + Send + Sync {
    /// Получить validator (может быть отключен в production)
    fn validator(&self) -> Option<&dyn DependencyValidator>;

    /// Получить metrics collector (может быть отключен в production)
    fn metrics(&self) -> Option<&dyn ContainerMetrics>;

    /// Создать child scope с тем же типом
    fn create_typed_scope(&self) -> Self
    where
        Self: Sized + Clone;

    /// Получить dynamic interface для object-safe операций
    fn as_dynamic(&self) -> &dyn DynamicDIContainer;
}

/// === OBJECT-SAFE CONTAINER INTERFACE ===
/// Композиция object-safe interfaces для dynamic dispatch
pub trait DynamicDIContainer:
    DynamicServiceRegistry + DynamicServiceResolver + Send + Sync
{
    /// Получить validator
    fn validator(&self) -> Option<&dyn DependencyValidator>;

    /// Получить metrics collector
    fn metrics(&self) -> Option<&dyn ContainerMetrics>;

    /// Создать child scope (возвращает object-safe интерфейс)
    fn create_dynamic_scope(&self) -> Box<dyn DynamicDIContainer>;

    /// Получить имя контейнера для отладки
    fn container_name(&self) -> &str;

    /// Получить конфигурацию контейнера
    fn get_configuration(&self) -> ContainerConfiguration;
}

/// === LIFETIME STRATEGIES ===
/// Стратегии управления жизненным циклом объектов
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifetimeStrategy {
    /// Новый экземпляр при каждом запросе
    Transient,
    /// Один экземпляр для всего приложения
    Singleton,
    /// Один экземпляр в рамках scope
    Scoped,
}

/// === PERFORMANCE METRICS TYPES ===
#[derive(Debug, Default, Clone)]
pub struct ResolutionStats {
    pub total_resolutions: u64,
    pub successful_resolutions: u64,
    pub failed_resolutions: u64,
    pub avg_resolution_time_ns: u64,
    pub max_resolution_time_ns: u64,
    pub resolutions_by_type: std::collections::HashMap<TypeId, u64>,
}

#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_size: usize,
    pub cache_hit_rate: f64,
}

/// === FACTORY FUNCTION TYPE ===
/// Стандартный тип для всех фабричных функций (принимает конкретный UnifiedContainer)
pub type ServiceFactory = Box<
    dyn Fn(
            &crate::di::unified_container_impl::UnifiedContainer,
        ) -> Result<Box<dyn Any + Send + Sync>, DIError>
        + Send
        + Sync,
>;

/// === CONTAINER BUILDER INTERFACE ===
/// Builder pattern для создания контейнеров с разными конфигурациями
pub trait ContainerBuilder {
    /// Включить/отключить валидацию зависимостей
    fn with_validation(self, enabled: bool) -> Self;

    /// Включить/отключить сбор метрик
    fn with_metrics(self, enabled: bool) -> Self;

    /// Зарегистрировать singleton при создании
    fn register_singleton<T, F>(self, factory: F) -> Result<Self, DIError>
    where
        T: Send + Sync + 'static,
        F: Fn(&dyn DynamicServiceResolver) -> Result<T, DIError> + Send + Sync + 'static,
        Self: Sized;

    /// Зарегистрировать transient при создании
    fn register_transient<T, F>(self, factory: F) -> Result<Self, DIError>
    where
        T: Send + Sync + 'static,
        F: Fn(&dyn DynamicServiceResolver) -> Result<T, DIError> + Send + Sync + 'static,
        Self: Sized;

    /// Создать контейнер (возвращает конкретный тип)
    fn build(self) -> Result<Box<dyn DynamicDIContainer>, DIError>;
}

/// === TYPE-SAFE SERVICE LOCATOR PATTERN ===
/// Безопасная альтернатива Service Locator антипаттерну
/// Используется только для bootstrap кода и интеграционных тестов
pub trait ServiceLocator {
    /// Получить сервис с проверкой типа во время компиляции
    fn get<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, DIError>;

    /// Проверить доступность сервиса
    fn contains<T: 'static>(&self) -> bool;
}

/// === CONFIGURATION TRAITS ===
/// Конфигурирование различных аспектов DI системы
pub trait ConfigurableContainer {
    /// Настроить валидацию
    fn configure_validation(&mut self, config: ValidationConfig);

    /// Настроить метрики
    fn configure_metrics(&mut self, config: MetricsConfig);

    /// Настроить кэширование
    fn configure_caching(&mut self, config: CacheConfig);
}

#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub enable_cycle_detection: bool,
    pub enable_resolution_validation: bool,
    pub strict_mode: bool,
}

#[derive(Debug, Clone)]
pub struct MetricsConfig {
    pub enable_timing: bool,
    pub enable_counting: bool,
    pub enable_detailed_stats: bool,
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub enable_singleton_cache: bool,
    pub cache_size_limit: Option<usize>,
    pub cache_ttl_seconds: Option<u64>,
}

/// === UNIFIED CONTAINER CONFIGURATION ===
#[derive(Debug, Clone)]
pub struct ContainerConfiguration {
    pub validation: ValidationConfig,
    pub metrics: MetricsConfig,
    pub cache: CacheConfig,
    pub name: String,
    pub debug_mode: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enable_cycle_detection: true,
            enable_resolution_validation: true,
            strict_mode: false,
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_timing: true,
            enable_counting: true,
            enable_detailed_stats: false,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_singleton_cache: true,
            cache_size_limit: Some(1000),
            cache_ttl_seconds: None,
        }
    }
}

impl Default for ContainerConfiguration {
    fn default() -> Self {
        Self {
            validation: ValidationConfig::default(),
            metrics: MetricsConfig::default(),
            cache: CacheConfig::default(),
            name: "UnifiedContainer".to_string(),
            debug_mode: cfg!(debug_assertions),
        }
    }
}
