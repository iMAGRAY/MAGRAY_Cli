// Модуль orchestration для координации компонентов memory системы
// Применяет SOLID принципы через декомпозицию на специализированные модули

pub mod traits;
mod retry_handler;
mod embedding_coordinator;
mod search_coordinator;
mod health_manager;
mod promotion_coordinator;
mod resource_controller;
mod backup_coordinator;

// === SOLID-совместимая архитектура ===
pub mod circuit_breaker_manager;
pub mod orchestration_lifecycle_manager;
pub mod operation_executor;
pub mod metrics_collector;
pub mod coordinator_registry;
pub mod orchestration_facade;

// Legacy поддержка - оригинальный God Object (deprecated)
mod memory_orchestrator;

// === Public API ===

// Coordinator traits
pub use traits::Coordinator;

// Core coordinator implementations
pub use retry_handler::{RetryHandler, RetryPolicy, RetryResult};
pub use embedding_coordinator::EmbeddingCoordinator;
pub use search_coordinator::SearchCoordinator;
pub use health_manager::HealthManager;
pub use promotion_coordinator::PromotionCoordinator;
pub use resource_controller::ResourceController;
pub use backup_coordinator::BackupCoordinator;

// === NEW SOLID Architecture (Recommended) ===

// Circuit breaker management
pub use circuit_breaker_manager::{
    CircuitBreakerManager, CircuitBreakerManagerTrait,
    CircuitBreakerConfig, CircuitBreakerStatistics, CircuitBreakerStatus
};

// Lifecycle management
pub use orchestration_lifecycle_manager::{
    OrchestrationLifecycleManager, LifecycleManager,
    CoordinatorRegistry as LifecycleCoordinatorRegistry
};

// Operation execution
pub use operation_executor::{
    OperationExecutor, OperationExecutorTrait,
    OperationMetrics, CoordinatorDependencies
};

// Metrics collection
pub use metrics_collector::{
    MetricsCollector, MetricsCollectorTrait,
    OrchestrationMetrics, PerformanceMetrics, AvailabilityMetrics
};

// Coordinator registry
pub use coordinator_registry::{
    CoordinatorRegistry, CoordinatorRegistryTrait, CoordinatorRegistryFactory,
    ReadinessStatus, ValidationResult
};

// Main facade (recommended entry point)
pub use orchestration_facade::{OrchestrationFacade, MemoryOrchestrator};

// === Legacy Support (Deprecated) ===
#[deprecated(since = "0.1.0", note = "Use OrchestrationFacade instead - provides same API with better architecture")]
pub use memory_orchestrator::MemoryOrchestrator as LegacyMemoryOrchestrator;