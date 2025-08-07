//! Service DI Module - Decomposed God Object
//!
//! Этот модуль представляет декомпозированную архитектуру service_di.rs,
//! разбитую на специализированные модули по принципам SOLID.

// === Core Modules (SOLID-compliant) ===
pub mod circuit_breaker;
pub mod coordinator_factory;
pub mod lifecycle_manager;
pub mod operation_executor;
pub mod production_monitoring;
pub mod service_config;
// facade.rs удален - используется unified_container

// === Re-exports для удобства ===
pub use service_config::{
    default_config,
    DefaultServiceConfigFactory,
    MemoryConfig, // Backward compatibility
    MemoryServiceConfig,
    MemoryServiceConfigBuilder,
    ServiceConfigFactory,
    ServiceConfigType,
};

pub use coordinator_factory::{
    CoordinatorFactory, OrchestrationCoordinators, ProductionCoordinatorFactory,
    TestCoordinatorFactory,
};

pub use production_monitoring::{
    MetricsCollector, ProductionMetrics, ProductionMetricsCollector, ProductionMonitoringManager,
};

pub use circuit_breaker::{
    CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState, CircuitBreakerStats,
};

pub use lifecycle_manager::{LifecycleConfig, LifecycleManager, LifecycleState, OperationStats};

pub use operation_executor::{
    BatchInsertResult, BatchSearchResult, ExtendedOperationExecutor, OperationConfig,
    OperationExecutor, ProductionOperationExecutor, SimpleOperationExecutor,
};

// === Main type alias для замены God Object ===
// DIMemoryService теперь в unified_container.rs

// === Backward Compatibility Types ===
use crate::{
    batch_manager::BatchStats, gpu_accelerated::BatchProcessorStats, health::SystemHealthStatus,
    promotion::PromotionStats,
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
