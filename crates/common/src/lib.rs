pub mod comprehensive_errors;
pub mod config_base;
pub mod event_bus;
pub mod events;
pub mod input_validation;
pub mod macros;
pub mod policy;
pub mod sandbox_config;
pub mod scheduler;
pub mod service_macros;
pub mod service_traits;
pub mod structured_logging;
pub mod topics;

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

// SECURITY: Export input validation functions
pub use input_validation::{
    validate_input_command, validate_input_path, validate_input_string, validate_input_url,
    InputValidator, ValidationResult,
};

// Re-export service traits
pub use service_traits::{
    BaseService, BuildableService, CacheService, CacheStats as CacheLayerStats,
    CircuitBreakerService, CircuitBreakerState, ClearableService, ConfigTrait,
    ConfigurationProfile, ExecutableService, HealthCheckService, InitializableService,
    LifecycleService, LifecycleState, MetricsService, PoolStats, PooledService, RetryConfig,
    RetryableService, SearchPage, SearchableService, ServiceCoordinator, StatisticsProvider,
};

// Re-export config base components
pub use config_base::{
    BatchConfigBase, CacheConfigBase, CircuitBreakerConfigBase, ConfigComposition, GpuConfigBase,
    MonitoringConfigBase, NetworkConfigBase, RetryConfigBase, StorageConfigBase, TimeoutConfigBase,
};

// Macros are automatically available at crate root due to #[macro_export]
