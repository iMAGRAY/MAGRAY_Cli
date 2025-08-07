//! Service DI Module - Decomposed God Object
//! 
//! Этот модуль представляет декомпозированную архитектуру service_di.rs,
//! разбитую на специализированные модули по принципам SOLID.

// === Core Modules (SOLID-compliant) ===
pub mod service_config;
pub mod coordinator_factory;
pub mod production_monitoring;
pub mod circuit_breaker;
pub mod lifecycle_manager;
pub mod operation_executor;
// facade.rs удален - используется unified_container

// === Re-exports для удобства ===
pub use service_config::{
    MemoryServiceConfig, 
    MemoryServiceConfigBuilder, 
    ServiceConfigType, 
    ServiceConfigFactory,
    DefaultServiceConfigFactory,
    default_config,
    MemoryConfig, // Backward compatibility
};

pub use coordinator_factory::{
    CoordinatorFactory,
    OrchestrationCoordinators,
    ProductionCoordinatorFactory,
    TestCoordinatorFactory,
};

pub use production_monitoring::{
    ProductionMetrics,
    MetricsCollector,
    ProductionMetricsCollector,
    ProductionMonitoringManager,
};

pub use circuit_breaker::{
    CircuitBreaker,
    CircuitBreakerConfig,
    CircuitBreakerState,
    CircuitBreakerStats,
};

pub use lifecycle_manager::{
    LifecycleManager,
    LifecycleConfig,
    LifecycleState,
    OperationStats,
};

pub use operation_executor::{
    OperationExecutor,
    ProductionOperationExecutor,
    SimpleOperationExecutor,
    ExtendedOperationExecutor,
    OperationConfig,
    BatchInsertResult,
    BatchSearchResult,
};

// === Main type alias для замены God Object ===
// DIMemoryService теперь в unified_container.rs

// === Backward Compatibility Types ===
use crate::{
    health::SystemHealthStatus,
    promotion::PromotionStats,
    batch_manager::BatchStats,
    gpu_accelerated::BatchProcessorStats,
};

/// Статистика всей memory системы (backward compatibility)
#[derive(Debug)]
pub struct MemorySystemStats {
    pub health_status: anyhow::Result<SystemHealthStatus>,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_size: u64,
    pub promotion_stats: PromotionStats,
    pub batch_stats: BatchStats,
    pub gpu_stats: Option<BatchProcessorStats>,
    pub di_container_stats: crate::DIContainerStats,
}

impl Default for MemorySystemStats {
    fn default() -> Self {
        Self {
            health_status: Err(anyhow::anyhow!("Health status not available")),
            cache_hits: 0,
            cache_misses: 0,
            cache_size: 0,
            promotion_stats: PromotionStats::default(),
            batch_stats: BatchStats::default(),
            gpu_stats: None,
            di_container_stats: crate::DIContainerStats {
                registered_factories: 0,
                cached_singletons: 0,
                total_resolutions: 0,
                cache_hits: 0,
                validation_errors: 0,
            },
        }
    }
}