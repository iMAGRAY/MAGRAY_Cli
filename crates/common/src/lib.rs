pub mod structured_logging;
pub mod comprehensive_errors;
pub mod macros;
pub mod service_traits;
pub mod config_base;

pub use structured_logging::{
    init_structured_logging, 
    LoggingConfig, 
    StructuredLogEntry,
    ExecutionContext,
    PerformanceMetrics,
    OperationTimer,
    RequestContext,
};

pub use comprehensive_errors::{
    MagrayCoreError, MagrayCoreResult,
    MemoryError, MemoryResult,
    AIError, AIResult,
    LLMError, LLMResult,
    ResourceError, ResourceResult,
    ConfigError, ConfigResult,
    NetworkError, NetworkResult,
    FileSystemError, FileSystemResult,
    CriticalError, CriticalResult,
    RecoveryStrategy,
    ErrorContext,
    SafeUnwrap,
};

// Re-export service traits
pub use service_traits::{
    BaseService, ConfigurableService, MetricsService,
    CircuitBreakerService, RetryableService, LifecycleService,
    ServiceCoordinator, ServiceFactory, PooledService, CacheService,
    ConfigTrait, CircuitBreakerState, RetryConfig, LifecycleState,
    PoolStats, CacheStats,
};

// Re-export config base components  
pub use config_base::{
    BatchConfigBase, TimeoutConfigBase, CacheConfigBase, GpuConfigBase,
    CircuitBreakerConfigBase, RetryConfigBase, MonitoringConfigBase,
    StorageConfigBase, NetworkConfigBase, ConfigComposition,
};

// Macros are automatically available at crate root due to #[macro_export]