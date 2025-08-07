pub mod comprehensive_errors;
pub mod config_base;
pub mod macros;
pub mod service_traits;
pub mod structured_logging;

#[cfg(test)]
pub mod test_utils;

pub use structured_logging::{
    init_structured_logging, ExecutionContext, LoggingConfig, OperationTimer, PerformanceMetrics,
    RequestContext, StructuredLogEntry,
};

pub use comprehensive_errors::{
    AIError, AIResult, ConfigError, ConfigResult, CriticalError, CriticalResult, ErrorContext,
    FileSystemError, FileSystemResult, LLMError, LLMResult, MagrayCoreError, MagrayCoreResult,
    MemoryError, MemoryResult, NetworkError, NetworkResult, RecoveryStrategy, ResourceError,
    ResourceResult, SafeUnwrap,
};

// Re-export service traits
pub use service_traits::{
    BaseService, CacheService, CacheStats, CircuitBreakerService, CircuitBreakerState, ConfigTrait,
    ConfigurableService, LifecycleService, LifecycleState, MetricsService, PoolStats,
    PooledService, RetryConfig, RetryableService, ServiceCoordinator, ServiceFactory,
};

// Re-export config base components
pub use config_base::{
    BatchConfigBase, CacheConfigBase, CircuitBreakerConfigBase, ConfigComposition, GpuConfigBase,
    MonitoringConfigBase, NetworkConfigBase, RetryConfigBase, StorageConfigBase, TimeoutConfigBase,
};

// Macros are automatically available at crate root due to #[macro_export]
