//! Специализированные сервисы для декомпозиции DIMemoryService God Object
//! 
//! Архитектура основана на принципах SOLID:
//! - Single Responsibility: каждый сервис отвечает за одну область
//! - Open/Closed: расширяемость через trait абстракции  
//! - Liskov Substitution: заменяемые реализации через traits
//! - Interface Segregation: минимальные специализированные интерфейсы
//! - Dependency Inversion: зависимость от абстракций, не от конкретных типов

pub mod core_memory_service;
pub mod coordinator_service;
pub mod resilience_service;
pub mod monitoring_service;
pub mod cache_service;
pub mod traits;
pub mod service_factory;
pub mod refactored_di_memory_service;

// Re-export основных trait интерфейсов
pub use traits::{
    CoreMemoryServiceTrait,
    CoordinatorServiceTrait,
    ResilienceServiceTrait,
    MonitoringServiceTrait,
    CacheServiceTrait,
};

// Re-export реализаций сервисов
pub use core_memory_service::CoreMemoryService;
pub use coordinator_service::CoordinatorService;
pub use resilience_service::ResilienceService;
pub use monitoring_service::MonitoringService;
pub use cache_service::CacheService;

// Re-export service factory для DI интеграции
pub use service_factory::{ServiceFactory, ServiceCollection, ServiceFactoryConfig};

// Re-export refactored service
pub use refactored_di_memory_service::{RefactoredDIMemoryService, RefactoredDIMemoryServiceBuilder};