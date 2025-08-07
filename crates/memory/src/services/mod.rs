//! Специализированные сервисы для декомпозиции DIMemoryService God Object
//!
//! Архитектура основана на принципах SOLID:
//! - Single Responsibility: каждый сервис отвечает за одну область
//! - Open/Closed: расширяемость через trait абстракции  
//! - Liskov Substitution: заменяемые реализации через traits
//! - Interface Segregation: минимальные специализированные интерфейсы
//! - Dependency Inversion: зависимость от абстракций, не от конкретных типов

pub mod cache_service;
pub mod coordinator_service;
pub mod core_memory_service;
pub mod factory_traits;
pub mod monitoring_service;
pub mod refactored_di_memory_service;
pub mod resilience_service;
pub mod service_factory;
pub mod traits;
pub mod unified_factory;

// Re-export основных trait интерфейсов
pub use traits::{
    CacheServiceTrait, CoordinatorServiceTrait, CoreMemoryServiceTrait, MonitoringServiceTrait,
    ResilienceServiceTrait,
};

// Re-export реализаций сервисов
pub use cache_service::CacheService;
pub use coordinator_service::CoordinatorService;
pub use core_memory_service::CoreMemoryService;
pub use monitoring_service::MonitoringService;
pub use resilience_service::ResilienceService;

// Re-export service factory для DI интеграции
pub use service_factory::{ServiceCollection, ServiceFactory, ServiceFactoryConfig};

// Re-export unified factory architecture
pub use unified_factory::{
    UnifiedFactoryConfig, UnifiedFactoryConfigBuilder, UnifiedServiceCollection,
    UnifiedServiceFactory, UnifiedServiceStatistics,
};

// Re-export factory traits для расширяемости
pub use factory_traits::{
    BaseFactory, CoordinatorFactory as CoordinatorFactoryTrait, CoreServiceFactory, FactoryError,
    FactoryPreset, FactoryResult, ServiceCollectionFactory, SpecializedComponentAvailability,
    SpecializedComponentFactory, SpecializedFactoryConfig, TestFactory,
};

// Re-export refactored service
pub use refactored_di_memory_service::{
    RefactoredDIMemoryService, RefactoredDIMemoryServiceBuilder,
};
