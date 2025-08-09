// Модуль orchestration для координации компонентов memory системы
// Применяет SOLID принципы через декомпозицию на специализированные модули

#[cfg(all(not(feature = "minimal"), feature = "backup-restore"))]
mod backup_coordinator;
mod embedding_coordinator;
mod health_manager;
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
mod promotion_coordinator;
mod resource_controller;
mod retry_handler;
mod search_coordinator;
pub mod traits;

// === SOLID-совместимая архитектура ===
pub mod background_task_manager;
#[cfg(feature = "legacy-orchestrator")]
pub mod circuit_breaker_manager;
#[cfg(feature = "legacy-orchestrator")]
pub mod coordinator_registry;
pub mod health_checker;
#[cfg(feature = "legacy-orchestrator")]
pub mod lifecycle_manager;
#[cfg(feature = "legacy-orchestrator")]
pub mod metrics_collector;
#[cfg(feature = "legacy-orchestrator")]
pub mod operation_executor;
#[cfg(feature = "legacy-orchestrator")]
pub mod orchestration_facade;
#[cfg(feature = "legacy-orchestrator")]
pub mod orchestration_lifecycle_manager;
#[cfg(feature = "legacy-orchestrator")]
pub mod orchestrator_core;

// Legacy поддержка - оригинальный God Object (deprecated)
#[cfg(feature = "legacy-orchestrator")]
mod memory_orchestrator;

// === Public API ===

// Coordinator traits
pub use traits::Coordinator;

// Core coordinator implementations
#[cfg(all(not(feature = "minimal"), feature = "backup-restore"))]
pub use backup_coordinator::BackupCoordinator;
pub use embedding_coordinator::EmbeddingCoordinator;
pub use health_manager::HealthManager;
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
pub use promotion_coordinator::PromotionCoordinator;
pub use resource_controller::ResourceController;
pub use retry_handler::{RetryHandler, RetryPolicy, RetryResult};
pub use search_coordinator::SearchCoordinator;

// === NEW SOLID Architecture (Recommended) ===

// Circuit breaker management
#[cfg(feature = "legacy-orchestrator")]
pub use circuit_breaker_manager::{
    CircuitBreakerConfig, CircuitBreakerManager, CircuitBreakerManagerTrait,
    CircuitBreakerStatistics, CircuitBreakerStatus,
};

// Lifecycle management
#[cfg(feature = "legacy-orchestrator")]
pub use orchestration_lifecycle_manager::{
    CoordinatorRegistry as LifecycleCoordinatorRegistry,
    LifecycleManager as OrchestrationLifecycleManagerTrait, OrchestrationLifecycleManager,
};

// Operation execution
#[cfg(feature = "legacy-orchestrator")]
pub use operation_executor::{
    CoordinatorDependencies, OperationExecutor, OperationExecutorTrait, OperationMetrics,
};

// Metrics collection
#[cfg(feature = "legacy-orchestrator")]
pub use metrics_collector::{
    CircuitBreakerMetric, CircuitBreakerStatus as MetricsCircuitBreakerStatus, CoordinatorMetrics,
    MetricType, MetricsCollector, OrchestrationMetrics, TimestampedMetric,
};

// Health checking
pub use health_checker::{
    HealthCheckConfig, HealthChecker, HealthLevel, HealthStatus, SystemDiagnostics,
};

// Lifecycle management
#[cfg(feature = "legacy-orchestrator")]
pub use lifecycle_manager::LifecycleManager;

// Coordinator registry
#[cfg(feature = "legacy-orchestrator")]
pub use coordinator_registry::{CoordinatorRegistry, ReadinessStatus};

// Main facade (recommended entry point)
#[cfg(feature = "legacy-orchestrator")]
pub use orchestration_facade::{MemoryOrchestrator, OrchestrationFacade};

// New SOLID components
pub use background_task_manager::BackgroundTaskManager;
#[cfg(feature = "legacy-orchestrator")]
pub use orchestrator_core::OrchestratorCore;

// === Legacy Support (Deprecated)
#[cfg(feature = "legacy-orchestrator")]
#[deprecated(
    since = "0.1.0",
    note = "Use OrchestrationFacade instead - provides same API with better architecture"
)]
pub use memory_orchestrator::MemoryOrchestrator as LegacyMemoryOrchestrator;
