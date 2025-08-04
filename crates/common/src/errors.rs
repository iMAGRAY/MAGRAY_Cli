use std::fmt;
use thiserror::Error;

/// Основная иерархия ошибок для MAGRAY CLI
// @component: {"k":"C","id":"magray_error_types","t":"Comprehensive error type system","m":{"cur":0,"tgt":95,"u":"%"},"f":["errors","production","monitoring"]}
#[derive(Error, Debug)]
pub enum MagrayError {
    // === Системные ошибки ===
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    // === Бизнес-логика ===
    
    #[error("Validation error: {0}")]
    Validation(ValidationError),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Resource conflict: {0}")]
    Conflict(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Operation timeout: {0}")]
    Timeout(String),
    
    // === AI/ML specific ===
    
    #[error("Embedding error: {0}")]
    Embedding(#[from] EmbeddingError),
    
    #[error("Model loading error: {0}")]
    ModelLoad(String),
    
    #[error("GPU error: {0}")]
    Gpu(#[from] GpuError),
    
    // === Memory system ===
    
    #[error("Memory error: {0}")]
    Memory(#[from] MemoryError),
    
    #[error("Cache error: {0}")]
    Cache(#[from] CacheError),
    
    #[error("Index error: {0}")]
    Index(#[from] IndexError),
    
    // === Общие ===
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("{0}")]
    Custom(String),
}

/// Database-specific errors
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Query failed: {0}")]
    QueryFailed(String),
    
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),
    
    #[error("Schema mismatch: expected {expected}, got {actual}")]
    SchemaMismatch { expected: String, actual: String },
    
    #[error("Database locked: {0}")]
    Locked(String),
    
    #[error("Corruption detected: {0}")]
    Corrupted(String),
}

/// Network-specific errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection refused: {0}")]
    ConnectionRefused(String),
    
    #[error("DNS resolution failed: {0}")]
    DnsFailure(String),
    
    #[error("Request timeout after {0}s")]
    Timeout(u64),
    
    #[error("HTTP error {code}: {message}")]
    Http { code: u16, message: String },
    
    #[error("SSL/TLS error: {0}")]
    TlsError(String),
}

/// Validation errors
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid input: {field} - {reason}")]
    InvalidInput { field: String, reason: String },
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Value out of range: {field} (got {value}, expected {min}-{max})")]
    OutOfRange {
        field: String,
        value: String,
        min: String,
        max: String,
    },
    
    #[error("Invalid format: {field} - expected {expected}")]
    InvalidFormat { field: String, expected: String },
    
    #[error("Size limit exceeded: {field} (max {max_size} bytes)")]
    SizeExceeded { field: String, max_size: usize },
}

/// AI/Embedding errors
#[derive(Error, Debug)]
pub enum EmbeddingError {
    #[error("Model not loaded: {0}")]
    ModelNotLoaded(String),
    
    #[error("Invalid dimensions: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    
    #[error("Tokenization failed: {0}")]
    TokenizationFailed(String),
    
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
    
    #[error("Batch size exceeded: max {max}, got {actual}")]
    BatchSizeExceeded { max: usize, actual: usize },
}

/// GPU-specific errors
#[derive(Error, Debug)]
pub enum GpuError {
    #[error("GPU not available: {0}")]
    NotAvailable(String),
    
    #[error("Out of GPU memory: required {required}MB, available {available}MB")]
    OutOfMemory { required: u64, available: u64 },
    
    #[error("CUDA error: {0}")]
    CudaError(String),
    
    #[error("GPU timeout after {0}s")]
    Timeout(u64),
    
    #[error("GPU compute capability {actual} < required {required}")]
    InsufficientCapability { required: String, actual: String },
}

/// Memory system errors
#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Memory limit exceeded: {used}MB / {limit}MB")]
    LimitExceeded { used: u64, limit: u64 },
    
    #[error("Allocation failed: {0}")]
    AllocationFailed(String),
    
    #[error("Promotion failed: {0}")]
    PromotionFailed(String),
    
    #[error("Layer not initialized: {0:?}")]
    LayerNotInitialized(crate::Layer),
}

/// Cache errors
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache miss for key: {0}")]
    Miss(String),
    
    #[error("Cache corrupted: {0}")]
    Corrupted(String),
    
    #[error("Cache full: size {current}MB exceeds limit {limit}MB")]
    Full { current: u64, limit: u64 },
    
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
    
    #[error("Eviction failed: {0}")]
    EvictionFailed(String),
}

/// Vector index errors
#[derive(Error, Debug)]
pub enum IndexError {
    #[error("Index not built")]
    NotBuilt,
    
    #[error("Index corrupted: {0}")]
    Corrupted(String),
    
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    
    #[error("Dimension mismatch: index has {index_dim}, query has {query_dim}")]
    DimensionMismatch { index_dim: usize, query_dim: usize },
    
    #[error("Too many results requested: {requested} > max {max}")]
    TooManyResults { requested: usize, max: usize },
}

/// Trait для определения retriable ошибок
pub trait IsRetriable {
    fn is_retriable(&self) -> bool;
}

impl IsRetriable for MagrayError {
    fn is_retriable(&self) -> bool {
        match self {
            // Network errors are usually retriable
            MagrayError::Network(_) => true,
            
            // Database locks are retriable
            MagrayError::Database(DatabaseError::Locked(_)) => true,
            
            // Timeouts are retriable
            MagrayError::Timeout(_) => true,
            
            // GPU OOM might be retriable after cleanup
            MagrayError::Gpu(GpuError::OutOfMemory { .. }) => true,
            
            // Most other errors are not retriable
            _ => false,
        }
    }
}

/// Trait для определения recoverable ошибок
pub trait IsRecoverable {
    fn is_recoverable(&self) -> bool;
}

impl IsRecoverable for MagrayError {
    fn is_recoverable(&self) -> bool {
        match self {
            // Can fallback to CPU
            MagrayError::Gpu(_) => true,
            
            // Can use cached value
            MagrayError::Network(_) => true,
            MagrayError::Embedding(_) => true,
            
            // Can retry with smaller batch
            MagrayError::Memory(MemoryError::LimitExceeded { .. }) => true,
            
            // Critical errors are not recoverable
            MagrayError::Database(DatabaseError::Corrupted(_)) => false,
            MagrayError::Index(IndexError::Corrupted(_)) => false,
            
            _ => false,
        }
    }
}

/// Error severity для alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl MagrayError {
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            // Critical - требует немедленного внимания
            MagrayError::Database(DatabaseError::Corrupted(_)) => ErrorSeverity::Critical,
            MagrayError::Index(IndexError::Corrupted(_)) => ErrorSeverity::Critical,
            MagrayError::Internal(_) => ErrorSeverity::Critical,
            
            // High - важные ошибки
            MagrayError::Database(_) => ErrorSeverity::High,
            MagrayError::Configuration(_) => ErrorSeverity::High,
            MagrayError::PermissionDenied(_) => ErrorSeverity::High,
            
            // Medium - operational issues
            MagrayError::Memory(_) => ErrorSeverity::Medium,
            MagrayError::Gpu(_) => ErrorSeverity::Medium,
            MagrayError::Timeout(_) => ErrorSeverity::Medium,
            
            // Low - expected errors
            MagrayError::NotFound(_) => ErrorSeverity::Low,
            MagrayError::Validation(_) => ErrorSeverity::Low,
            MagrayError::Cache(CacheError::Miss(_)) => ErrorSeverity::Low,
            
            _ => ErrorSeverity::Medium,
        }
    }
    
    pub fn error_code(&self) -> &'static str {
        match self {
            MagrayError::Io(_) => "IO_ERROR",
            MagrayError::Database(_) => "DB_ERROR",
            MagrayError::Network(_) => "NET_ERROR",
            MagrayError::Validation(_) => "VALIDATION_ERROR",
            MagrayError::NotFound(_) => "NOT_FOUND",
            MagrayError::Conflict(_) => "CONFLICT",
            MagrayError::PermissionDenied(_) => "PERMISSION_DENIED",
            MagrayError::Timeout(_) => "TIMEOUT",
            MagrayError::Embedding(_) => "EMBEDDING_ERROR",
            MagrayError::ModelLoad(_) => "MODEL_LOAD_ERROR",
            MagrayError::Gpu(_) => "GPU_ERROR",
            MagrayError::Memory(_) => "MEMORY_ERROR",
            MagrayError::Cache(_) => "CACHE_ERROR",
            MagrayError::Index(_) => "INDEX_ERROR",
            MagrayError::Internal(_) => "INTERNAL_ERROR",
            MagrayError::Configuration(_) => "CONFIG_ERROR",
            MagrayError::Serialization(_) => "SERIALIZATION_ERROR",
            MagrayError::Custom(_) => "CUSTOM_ERROR",
        }
    }
}

/// Result type alias для удобства
pub type MagrayResult<T> = Result<T, MagrayError>;

/// Extension trait для добавления контекста
pub trait ErrorContext<T> {
    fn context_lazy<F>(self, f: F) -> MagrayResult<T>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Into<MagrayError>,
{
    fn context_lazy<F>(self, f: F) -> MagrayResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let base_error = e.into();
            MagrayError::Custom(format!("{}: {}", f(), base_error))
        })
    }
}

/// Макрос для быстрого создания ошибок
#[macro_export]
macro_rules! magray_error {
    ($variant:ident, $($arg:tt)*) => {
        $crate::errors::MagrayError::$variant(format!($($arg)*))
    };
}

/// Макрос для bail с MagrayError
#[macro_export]
macro_rules! magray_bail {
    ($variant:ident, $($arg:tt)*) => {
        return Err($crate::magray_error!($variant, $($arg)*).into())
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_severity() {
        let corruption = MagrayError::Database(DatabaseError::Corrupted("test".into()));
        assert_eq!(corruption.severity(), ErrorSeverity::Critical);
        
        let not_found = MagrayError::NotFound("test".into());
        assert_eq!(not_found.severity(), ErrorSeverity::Low);
    }
    
    #[test]
    fn test_retriable_errors() {
        let network_err = MagrayError::Network(NetworkError::Timeout(30));
        assert!(network_err.is_retriable());
        
        let corruption = MagrayError::Database(DatabaseError::Corrupted("test".into()));
        assert!(!corruption.is_retriable());
    }
    
    #[test]
    fn test_error_codes() {
        let gpu_err = MagrayError::Gpu(GpuError::NotAvailable("test".into()));
        assert_eq!(gpu_err.error_code(), "GPU_ERROR");
    }
}