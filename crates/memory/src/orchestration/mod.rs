// Модуль orchestration для координации компонентов memory системы
// Применяет SOLID принципы через декомпозицию на специализированные модули

mod backup_coordinator;
mod embedding_coordinator;
mod health_manager;
mod promotion_coordinator;
mod resource_controller;
mod retry_handler;
mod search_coordinator;
pub mod traits;

// === SOLID-совместимая архитектура ===
pub mod circuit_breaker_manager;
pub mod coordinator_registry;
pub mod health_checker;
pub mod lifecycle_manager;
pub mod metrics_collector;
pub mod operation_executor;
pub mod orchestration_facade;
pub mod orchestration_lifecycle_manager;

// Legacy поддержка - оригинальный God Object (deprecated)
mod memory_orchestrator;

// === Public API ===

// Coordinator traits
pub use traits::Coordinator;

// Core coordinator implementations
pub use backup_coordinator::BackupCoordinator;
pub use embedding_coordinator::EmbeddingCoordinator;
pub use health_manager::HealthManager;
pub use promotion_coordinator::PromotionCoordinator;
pub use resource_controller::ResourceController;
pub use retry_handler::{RetryHandler, RetryPolicy, RetryResult};
pub use search_coordinator::SearchCoordinator;

// === NEW SOLID Architecture (Recommended) ===

// Circuit breaker management
pub use circuit_breaker_manager::{
    CircuitBreakerConfig, CircuitBreakerManager, CircuitBreakerManagerTrait,
    CircuitBreakerStatistics, CircuitBreakerStatus,
};

// Lifecycle management
pub use orchestration_lifecycle_manager::{
    CoordinatorRegistry as LifecycleCoordinatorRegistry,
    LifecycleManager as OrchestrationLifecycleManagerTrait, OrchestrationLifecycleManager,
};

// Operation execution
pub use operation_executor::{
    CoordinatorDependencies, OperationExecutor, OperationExecutorTrait, OperationMetrics,
};

// Metrics collection
pub use metrics_collector::{
    CircuitBreakerMetric, CircuitBreakerStatus as MetricsCircuitBreakerStatus, CoordinatorMetrics,
    MetricType, MetricsCollector, OrchestrationMetrics, TimestampedMetric,
};

// Health checking
pub use health_checker::{
    HealthCheckConfig, HealthChecker, HealthLevel, HealthStatus, SystemDiagnostics,
};

// Lifecycle management
pub use lifecycle_manager::LifecycleManager;

// Coordinator registry
pub use coordinator_registry::{CoordinatorRegistry, ReadinessStatus};

// Main facade (recommended entry point)
pub use orchestration_facade::{MemoryOrchestrator, OrchestrationFacade};

// === Legacy Support (Deprecated) ===
#[deprecated(
    since = "0.1.0",
    note = "Use OrchestrationFacade instead - provides same API with better architecture"
)]
pub use memory_orchestrator::MemoryOrchestrator as LegacyMemoryOrchestrator;
