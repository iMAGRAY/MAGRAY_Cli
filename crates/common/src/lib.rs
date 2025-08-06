pub mod structured_logging;
pub mod comprehensive_errors;

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