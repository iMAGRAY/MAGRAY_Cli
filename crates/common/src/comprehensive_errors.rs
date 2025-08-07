//! Comprehensive Error Handling System for MAGRAY CLI
//!
//! This module provides a comprehensive error hierarchy and handling mechanisms
//! to replace all critical .unwrap() calls throughout the codebase.

use std::fmt;
use std::sync::PoisonError;
use thiserror::Error;

/// Top-level error type for MAGRAY CLI applications
#[derive(Debug, Error)]
pub enum MagrayCoreError {
    #[error("Memory system error: {0}")]
    Memory(#[from] MemoryError),

    #[error("AI/ML system error: {0}")]
    AI(#[from] AIError),

    #[error("LLM communication error: {0}")]
    LLM(#[from] LLMError),

    #[error("System resource error: {0}")]
    Resource(#[from] ResourceError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Network operation error: {0}")]
    Network(#[from] NetworkError),

    #[error("Service operation timeout")]
    Timeout,

    #[error("Operation cancelled by user")]
    OperationCancelled,

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("File system error: {0}")]
    FileSystem(#[from] FileSystemError),

    #[error("Critical system error: {0}")]
    Critical(#[from] CriticalError),
}

/// Memory subsystem errors
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Vector store operation failed: {reason}")]
    VectorStore { reason: String },

    #[error("HNSW index error: {operation} failed - {reason}")]
    HNSWIndex { operation: String, reason: String },

    #[error("Embedding generation failed: {reason}")]
    EmbeddingGeneration { reason: String },

    #[error("Promotion engine error: {reason}")]
    Promotion { reason: String },

    #[error("Cache operation failed: {cache_type} - {reason}")]
    Cache { cache_type: String, reason: String },

    #[error("Database operation failed: {operation} - {reason}")]
    Database { operation: String, reason: String },

    #[error("Transaction failed: {transaction_id} - {reason}")]
    Transaction {
        transaction_id: String,
        reason: String,
    },

    #[error(
        "Memory pool exhausted: requested {requested_bytes} bytes, available {available_bytes}"
    )]
    PoolExhausted {
        requested_bytes: usize,
        available_bytes: usize,
    },

    #[error("Lock contention: {resource} - {timeout_ms}ms timeout exceeded")]
    LockTimeout { resource: String, timeout_ms: u64 },
}

/// AI/ML subsystem errors  
#[derive(Debug, Error)]
pub enum AIError {
    #[error("Model loading failed: {model_name} - {reason}")]
    ModelLoad { model_name: String, reason: String },

    #[error("GPU operation failed: {operation} - {reason}")]
    GPU { operation: String, reason: String },

    #[error("ONNX runtime error: {reason}")]
    ONNX { reason: String },

    #[error("TensorRT error: {reason}")]
    TensorRT { reason: String },

    #[error("Tokenization failed: {reason}")]
    Tokenization { reason: String },

    #[error("Batch processing failed: batch_size={batch_size} - {reason}")]
    BatchProcessing { batch_size: usize, reason: String },

    #[error("Memory pool error: {pool_type} - {reason}")]
    MemoryPool { pool_type: String, reason: String },

    #[error("Device fallback triggered: {from_device} -> {to_device} - {reason}")]
    DeviceFallback {
        from_device: String,
        to_device: String,
        reason: String,
    },

    #[error("Model compatibility error: expected {expected}, got {actual}")]
    ModelCompatibility { expected: String, actual: String },
}

/// LLM communication errors
#[derive(Debug, Error)]
pub enum LLMError {
    #[error("API request failed: {provider} - {status_code}")]
    APIRequest { provider: String, status_code: u16 },

    #[error("Authentication failed: {provider} - {reason}")]
    Authentication { provider: String, reason: String },

    #[error("Rate limit exceeded: {provider} - retry after {retry_after_seconds}s")]
    RateLimit {
        provider: String,
        retry_after_seconds: u64,
    },

    #[error("Response parsing failed: {reason}")]
    ResponseParsing { reason: String },

    #[error("Context length exceeded: {requested} > {max_allowed}")]
    ContextLength {
        requested: usize,
        max_allowed: usize,
    },

    #[error("Model not available: {model_name} - {reason}")]
    ModelUnavailable { model_name: String, reason: String },

    #[error("Streaming error: {reason}")]
    Streaming { reason: String },
}

/// System resource errors
#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("Memory exhausted: requested {requested_mb}MB, available {available_mb}MB")]
    MemoryExhausted {
        requested_mb: usize,
        available_mb: usize,
    },

    #[error("CPU overload: current load {current_percent}% > threshold {threshold_percent}%")]
    CPUOverload {
        current_percent: f64,
        threshold_percent: f64,
    },

    #[error("GPU memory exhausted: requested {requested_mb}MB, available {available_mb}MB")]
    GPUMemoryExhausted {
        requested_mb: usize,
        available_mb: usize,
    },

    #[error("Disk space insufficient: required {required_mb}MB, available {available_mb}MB")]
    DiskSpaceInsufficient { required_mb: u64, available_mb: u64 },

    #[error("Thread pool exhausted: {active_threads}/{max_threads} threads")]
    ThreadPoolExhausted {
        active_threads: usize,
        max_threads: usize,
    },

    #[error("Resource lock poisoned: {resource} - {details}")]
    LockPoisoned { resource: String, details: String },
}

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required config: {config_key}")]
    MissingRequired { config_key: String },

    #[error("Invalid config value: {config_key} = '{value}' - {reason}")]
    InvalidValue {
        config_key: String,
        value: String,
        reason: String,
    },

    #[error("Config file not found: {file_path}")]
    FileNotFound { file_path: String },

    #[error("Config parsing failed: {format} - {reason}")]
    ParsingFailed { format: String, reason: String },

    #[error("Environment variable error: {var_name} - {reason}")]
    EnvVarError { var_name: String, reason: String },

    #[error("Config validation failed: {reason}")]
    ValidationFailed { reason: String },
}

/// Network operation errors
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Connection failed: {endpoint} - {reason}")]
    Connection { endpoint: String, reason: String },

    #[error("Timeout: {operation} - {timeout_seconds}s")]
    Timeout {
        operation: String,
        timeout_seconds: u64,
    },

    #[error("DNS resolution failed: {hostname} - {reason}")]
    DNSResolution { hostname: String, reason: String },

    #[error("SSL/TLS error: {reason}")]
    SSL { reason: String },

    #[error("HTTP error: {status_code} - {reason}")]
    HTTP { status_code: u16, reason: String },
}

/// File system errors
#[derive(Debug, Error)]
pub enum FileSystemError {
    #[error("File not found: {path}")]
    NotFound { path: String },

    #[error("Permission denied: {path} - {operation}")]
    PermissionDenied { path: String, operation: String },

    #[error("Disk full: {path} - {attempted_size} bytes")]
    DiskFull { path: String, attempted_size: u64 },

    #[error("Corrupted file: {path} - {reason}")]
    Corrupted { path: String, reason: String },

    #[error("Lock file error: {path} - {reason}")]
    LockFile { path: String, reason: String },

    #[error("IO error: {operation} on {path} - {reason}")]
    IO {
        operation: String,
        path: String,
        reason: String,
    },
}

/// Critical system errors that may require shutdown
#[derive(Debug, Error)]
pub enum CriticalError {
    #[error("System memory critically low: {available_mb}MB remaining")]
    MemoryCritical { available_mb: usize },

    #[error("Core service failure: {service} - {reason}")]
    CoreServiceFailure { service: String, reason: String },

    #[error("Data corruption detected: {component} - {details}")]
    DataCorruption { component: String, details: String },

    #[error("Security breach detected: {incident_type} - {details}")]
    SecurityBreach {
        incident_type: String,
        details: String,
    },

    #[error("Unrecoverable error: {reason}")]
    Unrecoverable { reason: String },
}

/// Result type aliases for convenience
pub type MagrayCoreResult<T> = Result<T, MagrayCoreError>;
pub type MemoryResult<T> = Result<T, MemoryError>;
pub type AIResult<T> = Result<T, AIError>;
pub type LLMResult<T> = Result<T, LLMError>;
pub type ResourceResult<T> = Result<T, ResourceError>;
pub type ConfigResult<T> = Result<T, ConfigError>;
pub type NetworkResult<T> = Result<T, NetworkError>;
pub type FileSystemResult<T> = Result<T, FileSystemError>;
pub type CriticalResult<T> = Result<T, CriticalError>;

/// Error recovery strategies
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// Retry the operation with exponential backoff
    Retry {
        max_attempts: u32,
        base_delay_ms: u64,
    },
    /// Fallback to alternative approach
    Fallback { alternative: String },
    /// Graceful degradation
    Degrade { reduced_functionality: String },
    /// Fail fast - don't attempt recovery
    FailFast,
    /// Log and continue
    LogAndContinue,
}

/// Error context for better debugging and recovery
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub component: String,
    pub recovery_strategy: RecoveryStrategy,
    pub user_facing_message: Option<String>,
    pub tech_details: Option<String>,
    pub correlation_id: Option<String>,
}

impl ErrorContext {
    pub fn new(operation: &str, component: &str) -> Self {
        Self {
            operation: operation.to_string(),
            component: component.to_string(),
            recovery_strategy: RecoveryStrategy::FailFast,
            user_facing_message: None,
            tech_details: None,
            correlation_id: None,
        }
    }

    pub fn with_recovery(mut self, strategy: RecoveryStrategy) -> Self {
        self.recovery_strategy = strategy;
        self
    }

    pub fn with_user_message(mut self, message: &str) -> Self {
        self.user_facing_message = Some(message.to_string());
        self
    }

    pub fn with_details(mut self, details: &str) -> Self {
        self.tech_details = Some(details.to_string());
        self
    }

    pub fn with_correlation_id(mut self, id: &str) -> Self {
        self.correlation_id = Some(id.to_string());
        self
    }
}

/// Convert poison errors to resource errors
impl<T> From<PoisonError<T>> for ResourceError {
    fn from(error: PoisonError<T>) -> Self {
        ResourceError::LockPoisoned {
            resource: "unknown".to_string(),
            details: format!("Lock poisoned: {}", error),
        }
    }
}

/// Helper macros for error creation
#[macro_export]
macro_rules! memory_error {
    ($variant:ident { $($field:ident: $value:expr),+ }) => {
        MemoryError::$variant { $($field: $value),+ }
    };
}

#[macro_export]
macro_rules! ai_error {
    ($variant:ident { $($field:ident: $value:expr),+ }) => {
        AIError::$variant { $($field: $value),+ }
    };
}

#[macro_export]
macro_rules! llm_error {
    ($variant:ident { $($field:ident: $value:expr),+ }) => {
        LLMError::$variant { $($field: $value),+ }
    };
}

#[macro_export]
macro_rules! resource_error {
    ($variant:ident { $($field:ident: $value:expr),+ }) => {
        ResourceError::$variant { $($field: $value),+ }
    };
}

/// Safe unwrap alternatives
pub trait SafeUnwrap<T> {
    fn safe_unwrap_or_error(self, error: MagrayCoreError) -> MagrayCoreResult<T>;
    fn safe_unwrap_or_log(self, context: &str, default: T) -> T;
}

impl<T> SafeUnwrap<T> for Option<T> {
    fn safe_unwrap_or_error(self, error: MagrayCoreError) -> MagrayCoreResult<T> {
        self.ok_or(error)
    }

    fn safe_unwrap_or_log(self, context: &str, default: T) -> T {
        match self {
            Some(value) => value,
            None => {
                tracing::warn!("Option unwrap failed in {}, using default", context);
                default
            }
        }
    }
}

impl<T, E: fmt::Debug> SafeUnwrap<T> for Result<T, E> {
    fn safe_unwrap_or_error(self, error: MagrayCoreError) -> MagrayCoreResult<T> {
        self.map_err(|_| error)
    }

    fn safe_unwrap_or_log(self, context: &str, default: T) -> T {
        match self {
            Ok(value) => value,
            Err(e) => {
                tracing::warn!(
                    "Result unwrap failed in {}: {:?}, using default",
                    context,
                    e
                );
                default
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_hierarchy() {
        let memory_err = MemoryError::VectorStore {
            reason: "test".to_string(),
        };
        let core_err = MagrayCoreError::Memory(memory_err);

        assert!(format!("{}", core_err).contains("Memory system error"));
    }

    #[test]
    fn test_safe_unwrap() {
        let opt: Option<i32> = None;
        let result = opt.safe_unwrap_or_log("test context", 42);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_error_context() {
        let ctx = ErrorContext::new("insert", "vector_store")
            .with_recovery(RecoveryStrategy::Retry {
                max_attempts: 3,
                base_delay_ms: 100,
            })
            .with_user_message("Vector insertion failed");

        assert_eq!(ctx.operation, "insert");
        assert_eq!(ctx.component, "vector_store");
        assert!(ctx.user_facing_message.is_some());
    }
}
