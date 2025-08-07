//! Refactored Dependency Injection system следующий принципам SOLID
//!
//! Этот модуль демонстрирует правильную декомпозицию God Object (di_container.rs)
//! с применением всех принципов SOLID:
//!
//! - **SRP (Single Responsibility)**: Каждый модуль имеет единственную ответственность
//! - **OCP (Open/Closed)**: Система расширяется через traits без изменения существующего кода
//! - **LSP (Liskov Substitution)**: Все реализации traits взаимозаменяемы
//! - **ISP (Interface Segregation)**: Traits разделены по функциональности
//! - **DIP (Dependency Inversion)**: Зависимости инжектируются через abstractions
//!
//! ## Архитектура модулей:
//!
//! ```text
//! di/
//! ├── traits.rs              - Trait абстракции (ISP, DIP)
//! ├── container_core.rs      - Основная DI логика (SRP)  
//! ├── lifetime_manager.rs    - Управление жизненным циклом (SRP, OCP)
//! ├── dependency_validator.rs - Валидация зависимостей (SRP)
//! ├── metrics_collector.rs   - Сбор метрик (SRP, DIP)
//! ├── container_builder.rs   - Builder pattern + Facade (SRP)
//! └── mod.rs                 - Public API и re-exports
//! ```
//!
//! ## Применение принципов SOLID:
//!
//! ### 1. Single Responsibility Principle (SRP)
//! - `ContainerCore`: только регистрация и разрешение
//! - `LifetimeManager`: только управление жизненным циклом  
//! - `DependencyValidator`: только валидация циклов
//! - `MetricsCollector`: только сбор и агрегация метрик
//! - `ConfigPresets`: только предустановленные конфигурации
//! - `MetricsReporter`: только сбор и отчетность метрик
//! - `ConfigLoader`: только загрузка конфигурационных файлов
//! - `ContainerRegistry`: только регистрация и поиск контейнеров
//! - `ServiceLocator`: только поиск и разрешение сервисов
//!
//! ### 2. Open/Closed Principle (OCP)  
//! - `LifetimeStrategy` trait позволяет добавлять новые lifetime стратегии
//! - `MetricsReporter` trait позволяет добавлять новые способы сбора метрик
//! - `CompositeMetricsReporter` демонстрирует расширение без изменений
//!
//! ### 3. Liskov Substitution Principle (LSP)
//! - Все реализации traits полностью взаимозаменяемы
//! - `NullDependencyValidator` и `NullMetricsReporter` для отключения функций
//!
//! ### 4. Interface Segregation Principle (ISP)
//! - `DIResolver` - только для разрешения зависимостей
//! - `DIRegistrar` - только для регистрации
//! - Клиенты зависят только от нужных им интерфейсов
//!
//! ### 5. Dependency Inversion Principle (DIP)
//! - `ContainerCore` зависит от abstractions (`LifetimeManager`, `DependencyValidator`, `MetricsReporter`)
//! - Конкретные реализации инжектируются через конструктор
//! - Нет прямых зависимостей на concrete types
//!
//! ## Обратная совместимость:
//!
//! Старый API через `DIContainer` facade полностью сохранен.
//! Все существующие тесты и код продолжат работать без изменений.

// === НОВАЯ ЕДИНАЯ АРХИТЕКТУРА ===
pub mod core_traits; // Single Source of Truth для всех DI abstractions
pub mod unified_container_impl; // ЕДИНСТВЕННАЯ КОРРЕКТНАЯ РЕАЛИЗАЦИЯ DI Container

// === Object Safety Solution ===
pub mod object_safe_resolver; // Type-erased resolver для dyn compatibility

// === Старые модули (deprecated) ===
pub mod container_builder;
pub mod container_core;
pub mod dependency_validator;
pub mod errors;
pub mod lifetime_manager;
pub mod metrics_collector;
pub mod migration_facade;
pub mod traits;
pub mod unified_container;

// NEW UNIFIED CONFIGURATION SYSTEM - объединяет все разрозненные configs
pub mod config_compatibility;
pub mod config_loader;
pub mod config_presets;
pub mod config_validation;
pub mod unified_config;

// OPTIMIZED MODULAR ARCHITECTURE - разделение God Objects
pub mod container_cache;
pub mod container_configuration;
pub mod optimized_unified_container;

// Re-export основных типов для удобства использования
pub use traits::{
    DIContainerStats, DIPerformanceMetrics, DIRegistrar, DIResolver, Lifetime, LifetimeManager,
    MetricsReporter, TypeMetrics,
};

// Re-export Object Safety solution
pub use object_safe_resolver::{
    ObjectSafeResolver, ResolverDiagnostics, ServiceLocatorResolver, TypeSafeResolver,
};

// Re-export error types для всего DI кода
pub use errors::{
    CoordinatorError, DIContextExt, DIError, LifecycleError, MetricsError, ValidationError,
};

pub use container_builder::{DIContainer, DIContainerBuilder};
pub use container_core::ContainerCore;
pub use dependency_validator::{DependencyGraph, DependencyGraphStats, DependencyValidatorImpl};
pub use lifetime_manager::{ExtensibleLifetimeManager, LifetimeManagerImpl, LifetimeStrategy};
pub use metrics_collector::{CompositeMetricsReporter, MetricsReporterImpl, TimingStatsReport};
pub use migration_facade::{
    DIMemoryServiceFacadeCompatible, DIMemoryServiceMigrationBuilder,
    DIMemoryServiceMigrationFacade, DIMemoryServiceOriginalCompatible,
    DIMemoryServiceRefactoredCompatible, LegacyMemoryConfig,
};
pub use unified_container::{
    ComponentFactory, UnifiedDIContainer, UnifiedDIContainerBuilder, UnifiedMemoryConfigurator,
};

// NEW UNIFIED CONFIGURATION SYSTEM - re-exports
pub use config_loader::{ConfigArgs, ConfigTemplateGenerator, ConfigurationLoader};
pub use config_presets::{ConfigBuilder, ConfigPresets};
pub use config_validation::ConfigurationValidator;
pub use unified_config::{
    AuthenticationConfig, ConfigurationMetadata, CoreSystemConfig, DatabaseConfig, Environment,
    FeatureFlags, MemorySystemConfig, OrchestrationConfig, PerformanceConfig,
    PerformanceThresholds, ProfilingConfig, RateLimitConfig, SecurityConfig,
    UnifiedDIConfiguration, ValidationError as ConfigValidationError, ValidationReport,
    ValidationWarning,
};

// Legacy compatibility - старый API остается доступным
pub use container_builder::DIContainer as DIContainerLegacy;

// NEW UNIFIED API - заменяет все дублирования
pub use unified_container::UnifiedDIContainer as DIContainerUnified;

// OPTIMIZED MODULAR COMPONENTS - blazingly fast, separated concerns
pub use container_cache::ContainerCache;
pub use container_configuration::{
    CacheConfiguration, DIConfigurationBuilder, DIContainerConfiguration, LogLevel,
    MonitoringConfiguration, PerformanceConfiguration, ValidationConfiguration,
};

// Уникальные exports из container_cache и configuration
pub use container_cache::{CacheConfig, CacheStats};
// DIContainerConfiguration уже импортируется выше в строке 145
pub use traits::DependencyValidator;
// LifetimeStrategy уже импортирован выше
pub use optimized_unified_container::{
    ContainerPreset, OptimizedContainerBuilder, OptimizedUnifiedContainer,
};

// MIGRATION FACADES - для постепенного перехода
pub use migration_facade::{
    DIMemoryServiceMigrationBuilder as DIMemoryServiceBuilder,
    DIMemoryServiceMigrationFacade as DIMemoryServiceUnified,
    DIMemoryServiceMigrationFacade as DIMemoryService,
};

/// Создать DI контейнер с настройками по умолчанию
///
/// Это convenience функция для быстрого создания контейнера
/// с включенными валидацией и метриками.
pub fn create_default_container() -> anyhow::Result<DIContainer> {
    DIContainer::default_container()
}

/// Создать минимальный DI контейнер без валидации и метрик
///
/// Полезно для production сценариев где нужна максимальная производительность
/// и валидация уже была выполнена в development.
pub fn create_minimal_container() -> anyhow::Result<DIContainer> {
    DIContainer::builder()
        .with_validation(false)
        .with_metrics(false)
        .build()
}

/// Создать контейнер с custom конфигурацией компонентов
///
/// Демонстрирует принцип Dependency Inversion - можно инжектировать
/// любые реализации основных компонентов.
pub fn create_custom_container(
    lifetime_manager: std::sync::Arc<LifetimeManagerImpl>,
    dependency_validator: std::sync::Arc<DependencyValidatorImpl>,
    metrics_reporter: std::sync::Arc<MetricsReporterImpl>,
) -> DIContainer {
    let core = ContainerCore::new(lifetime_manager, dependency_validator, metrics_reporter);
    DIContainer::new(core)
}

/// === NEW UNIFIED CONTAINER FACTORY FUNCTIONS ===

/// Создать унифицированный DI контейнер по умолчанию
///
/// ЗАМЕНЯЕТ все существующие DIContainer создания.
/// Использует оптимизированные настройки для баланса производительности и функциональности.
pub fn create_unified_container() -> UnifiedDIContainer {
    UnifiedDIContainer::new()
}

/// === UNIFIED CONFIGURATION FACTORY FUNCTIONS ===

/// Загрузить конфигурацию используя auto-detection и environment variables
///
/// Автоматически определяет лучший preset на основе окружения и применяет
/// все переопределения из environment variables.
pub fn load_auto_configuration() -> anyhow::Result<UnifiedDIConfiguration> {
    let loader = ConfigurationLoader::new();
    loader.load()
}

/// Загрузить конфигурацию из указанного файла
///
/// Поддерживает TOML, JSON и YAML форматы.
/// Применяет environment variable overrides после загрузки.
pub fn load_configuration_from_file<P: AsRef<std::path::Path>>(
    path: P,
) -> anyhow::Result<UnifiedDIConfiguration> {
    let loader = ConfigurationLoader::new();
    let mut config = loader.load_from_file(path)?;
    config.apply_env_overrides()?;
    Ok(config)
}

/// Создать конфигурацию с custom настройками
///
/// Удобный способ создания конфигурации с помощью builder pattern.
pub fn build_custom_configuration() -> ConfigBuilder {
    ConfigBuilder::new()
}

/// Валидировать конфигурацию и получить отчет
///
/// Выполняет comprehensive validation включая cross-component compatibility.
pub fn validate_configuration(config: &UnifiedDIConfiguration) -> anyhow::Result<ValidationReport> {
    let validator = ConfigurationValidator::new();
    validator.validate(config)
}

/// Сгенерировать template конфигурационного файла
///
/// Полезно для создания базовых конфигурационных файлов.
/// Поддерживаемые форматы: "toml", "json", "yaml"
pub fn generate_config_template(format: &str, preset: Option<&str>) -> anyhow::Result<String> {
    ConfigTemplateGenerator::generate_template(format, preset)
}

// === НОВЫЕ CORE TRAITS RE-EXPORTS (Single Source of Truth) ===
pub use core_traits::{
    // Builder interfaces
    ContainerBuilder,
    ContainerMetrics,
    // Unified container interface
    DIContainer as CoreDIContainer,
    MetricsConfig,
    // Performance metrics
    ResolutionStats,
    // Factory function type
    ServiceFactory,
    // Service locator (use sparingly!)
    ServiceLocator,
    // Core service interfaces
    ServiceRegistry,
    ServiceResolver,
    ValidationConfig,
};

// === ЕДИНСТВЕННАЯ КОРРЕКТНАЯ РЕАЛИЗАЦИЯ ===
pub use unified_container_impl::{
    container_builder,
    // Factory functions
    create_container,
    development_builder,
    production_builder,
    UnifiedContainer,
    UnifiedContainerBuilder,
};

// Дублированные factory функции - оставляем только из unified_container_impl
pub use unified_container_impl::{
    create_development_container, create_production_container, create_test_container,
};

// === НОВЫЙ СТАНДАРТНЫЙ API (заменяет все дублирования) ===
/// Стандартный DI контейнер для всего проекта
pub type StandardContainer = UnifiedContainer;
/// Стандартный builder для всего проекта  
pub type StandardContainerBuilder = UnifiedContainerBuilder;

#[cfg(test)]
mod integration_tests {
    use super::*;
    use anyhow::Result;

    #[derive(Debug)]
    struct TestService {
        value: i32,
    }

    impl TestService {
        fn new() -> Self {
            Self { value: 42 }
        }
    }

    #[derive(Debug)]
    struct DependentService {
        #[allow(dead_code)]
        test_service: std::sync::Arc<TestService>,
        value: i32,
    }

    impl DependentService {
        fn new(test_service: std::sync::Arc<TestService>) -> Self {
            Self {
                value: test_service.value * 2,
                test_service,
            }
        }
    }

    #[test]
    fn test_default_container() -> Result<()> {
        let container = create_default_container()?;

        container.register(|_| Ok(TestService::new()), Lifetime::Singleton)?;

        let service = container.resolve::<TestService>()?;
        assert_eq!(service.value, 42);

        Ok(())
    }

    #[test]
    fn test_minimal_container() -> Result<()> {
        let container = create_minimal_container()?;

        container.register(|_| Ok(TestService::new()), Lifetime::Singleton)?;

        let service = container.resolve::<TestService>()?;
        assert_eq!(service.value, 42);

        // Валидация и метрики отключены, но функциональность работает
        let stats = container.stats();
        assert_eq!(stats.total_resolutions, 0); // Метрики отключены

        Ok(())
    }

    #[test]
    fn test_custom_container() -> Result<()> {
        use std::sync::Arc;

        let lifetime_manager = Arc::new(LifetimeManagerImpl::new());
        let dependency_validator = Arc::new(DependencyValidatorImpl::new());
        let metrics_reporter = Arc::new(MetricsReporterImpl::new());

        let container =
            create_custom_container(lifetime_manager, dependency_validator, metrics_reporter);

        container.register(|_| Ok(TestService::new()), Lifetime::Singleton)?;

        let service = container.resolve::<TestService>()?;
        assert_eq!(service.value, 42);

        Ok(())
    }

    #[test]
    fn test_dependency_chain() -> Result<()> {
        let container = create_default_container()?;

        // Регистрируем базовый сервис
        container.register(|_| Ok(TestService::new()), Lifetime::Singleton)?;

        // Регистрируем зависимый сервис
        container.register(
            |resolver| {
                let test_service = resolver.resolve::<TestService>()?;
                Ok(DependentService::new(test_service))
            },
            Lifetime::Singleton,
        )?;

        let dependent = container.resolve::<DependentService>()?;
        assert_eq!(dependent.value, 84); // 42 * 2

        Ok(())
    }

    #[test]
    fn test_builder_integration() -> Result<()> {
        let container = DIContainer::builder()
            .with_validation(true)
            .with_metrics(true)
            .register_singleton(|_| Ok(TestService::new()))?
            .register_transient(|resolver| {
                let test_service = resolver.resolve::<TestService>()?;
                Ok(DependentService::new(test_service))
            })?
            .build()?;

        let dependent = container.resolve::<DependentService>()?;
        assert_eq!(dependent.value, 84);

        // Проверяем, что метрики собираются
        let stats = container.stats();
        assert!(stats.total_resolutions > 0);

        Ok(())
    }

    #[test]
    fn test_singleton_vs_transient() -> Result<()> {
        let container = DIContainer::builder()
            .register_singleton(|_| Ok(TestService::new()))?
            .build()?;

        // Два разрешения singleton должны вернуть тот же экземпляр
        let service1 = container.resolve::<TestService>()?;
        let service2 = container.resolve::<TestService>()?;

        // Проверяем по адресу, что это тот же экземпляр
        assert!(std::sync::Arc::ptr_eq(&service1, &service2));

        Ok(())
    }

    #[test]
    fn test_backwards_compatibility() -> Result<()> {
        // Тестируем, что старый API все еще работает
        let container = DIContainerLegacy::default_container()?;

        container.register(|_| Ok(TestService::new()), Lifetime::Singleton)?;

        assert!(container.is_registered::<TestService>());

        let service = container.resolve::<TestService>()?;
        assert_eq!(service.value, 42);

        let optional = container.try_resolve::<TestService>();
        assert!(optional.is_some());

        Ok(())
    }

    #[test]
    fn test_circular_dependency_detection() -> Result<()> {
        let container = create_default_container()?;

        // Добавляем циркулярную зависимость в граф
        container.add_dependency_info::<TestService, DependentService>()?;
        container.add_dependency_info::<DependentService, TestService>()?;

        // Валидация должна найти цикл
        let result = container.validate_dependencies();
        assert!(result.is_err());

        let cycles = container.get_dependency_cycles();
        assert!(!cycles.is_empty());

        Ok(())
    }

    #[test]
    fn test_performance_metrics() -> Result<()> {
        let container = create_default_container()?;

        container.register(|_| Ok(TestService::new()), Lifetime::Singleton)?;

        // Выполняем несколько разрешений
        for _ in 0..5 {
            let _service = container.resolve::<TestService>()?;
        }

        let metrics = container.performance_metrics();
        assert!(metrics.total_resolutions >= 5);
        assert!(metrics.cache_hits > 0); // Singleton должен кэшироваться

        Ok(())
    }
}
