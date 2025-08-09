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
#[cfg(not(feature = "minimal"))]
pub mod unified_container_impl; // ЕДИНСТВЕННАЯ КОРРЕКТНАЯ РЕАЛИЗАЦИЯ DI Container

// === Object Safety Solution ===
pub mod object_safe_resolver; // Type-erased resolver для dyn compatibility

// === НОВЫЕ ДЕКОМПОЗИРОВАННЫЕ МОДУЛИ ===
pub mod container_metrics_impl;
pub mod dependency_graph_validator; // Валидация циклических зависимостей
// (отключаем устаревшие/вторые реализации registry/resolver, чтобы исключить конфликты)
// #[cfg(not(feature = "minimal"))]
// pub mod memory_configurator; // Настройка memory компонентов в DI
// #[cfg(not(feature = "minimal"))]
// pub mod service_registry_impl; // Регистрация сервисов
// #[cfg(not(feature = "minimal"))]
// pub mod service_resolver_impl; // Разрешение зависимостей // Сбор метрик производительности

// === Старые модули (deprecated) — отключены для единой реализации ===
// pub mod container_builder;
// pub mod container_core;
// pub mod dependency_validator;
// pub mod errors;
// pub mod lifetime_manager;
// pub mod metrics_collector;
// pub mod migration_facade;
// pub mod traits;
// pub mod unified_container;

// NEW UNIFIED CONFIGURATION SYSTEM - отключаем конфликтующие реализации
// pub mod config_compatibility;
// pub mod config_loader;
// pub mod config_presets;
// pub mod config_validation;
// pub mod unified_config;

// OPTIMIZED MODULAR ARCHITEКTURE - отключаем конфликтующие реализации
// pub mod container_cache;
// pub mod container_configuration;
// pub mod container_factory;
// pub mod optimized_unified_container;

// === Подключаем необходимые модули ошибок и трейтов ===
pub mod errors;
pub mod traits;

// Re-export основных типов для удобства использования
pub use traits::{
    DIContainerStats, DIPerformanceMetrics, DIRegistrar, DIResolver, Lifetime, TypeMetrics,
};

// Re-export Object Safety solution
pub use object_safe_resolver::{
    ObjectSafeResolver, ResolverDiagnostics, ServiceLocatorResolver, TypeSafeResolver,
};

// Re-export error types для всего DI кода
pub use errors::{
    CoordinatorError, DIContextExt, DIError, LifecycleError, MetricsError, ValidationError,
};

// Совместимые алиасы для единой реализации
#[cfg(not(feature = "minimal"))]
pub use unified_container_impl::{
    container_builder,
    create_container,
    create_development_container,
    create_production_container,
    create_test_container,
    development_builder,
    production_builder,
    UnifiedContainer,
    UnifiedContainerBuilder,
};

// Единые алиасы для внешнего API (совместимость со старым кодом)
#[cfg(not(feature = "minimal"))]
pub type DIContainer = UnifiedContainer;
#[cfg(not(feature = "minimal"))]
pub type DIContainerBuilder = UnifiedContainerBuilder;

// === НОВЫЙ СТАНДАРТНЫЙ API (заменяет все дублирования) ===
/// Стандартный DI контейнер для всего проекта
#[cfg(not(feature = "minimal"))]
pub type StandardContainer = UnifiedContainer;
