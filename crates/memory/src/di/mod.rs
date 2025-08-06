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
//! - `MetricsReporter`: только сбор и отчетность метрик
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

pub mod traits;
pub mod container_core;
pub mod lifetime_manager;
pub mod dependency_validator;
pub mod metrics_collector;
pub mod container_builder;

// Re-export основных типов для удобства использования
pub use traits::{
    DIResolver, DIRegistrar, LifetimeManager, DependencyValidator, MetricsReporter,
    Lifetime, DIContainerStats, DIPerformanceMetrics, TypeMetrics,
};

pub use container_core::ContainerCore;
pub use lifetime_manager::{LifetimeManagerImpl, ExtensibleLifetimeManager, LifetimeStrategy};
pub use dependency_validator::{DependencyValidatorImpl, DependencyGraph, DependencyGraphStats};
pub use metrics_collector::{MetricsReporterImpl, CompositeMetricsReporter, TimingStatsReport};
pub use container_builder::{DIContainer, DIContainerBuilder};

// Legacy compatibility - старый API остается доступным
pub use container_builder::DIContainer as DIContainerLegacy;

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
    let core = ContainerCore::new(
        lifetime_manager,
        dependency_validator,
        metrics_reporter,
    );
    DIContainer::new(core)
}

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

        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Singleton,
        )?;

        let service = container.resolve::<TestService>()?;
        assert_eq!(service.value, 42);

        Ok(())
    }

    #[test]
    fn test_minimal_container() -> Result<()> {
        let container = create_minimal_container()?;

        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Singleton,
        )?;

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

        let container = create_custom_container(
            lifetime_manager,
            dependency_validator,
            metrics_reporter,
        );

        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Singleton,
        )?;

        let service = container.resolve::<TestService>()?;
        assert_eq!(service.value, 42);

        Ok(())
    }

    #[test]
    fn test_dependency_chain() -> Result<()> {
        let container = create_default_container()?;

        // Регистрируем базовый сервис
        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Singleton,
        )?;

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

        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Singleton,
        )?;

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

        container.register(
            |_| Ok(TestService::new()),
            Lifetime::Singleton,
        )?;

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