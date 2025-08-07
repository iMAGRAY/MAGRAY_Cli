//! 🚫 DI ERROR HANDLING SYSTEM
//!
//! Unified error handling для всего DI кода проекта MAGRAY CLI.
//! Заменяет все .unwrap() вызовы на безопасные Result<T, DIError> варианты.
//!
//! # КРИТИЧЕСКАЯ ЦЕЛЬ
//!
//! Устранить все 1054+ .unwrap() вызовов в проекте, начиная с DI системы.
//! Каждый .unwrap() - это потенциальный panic в production.
//!
//! # АРХИТЕКТУРА ОШИБОК
//!
//! - **DIError**: Main error enum для всех DI операций
//! - **CoordinatorError**: Ошибки координаторов и factory
//! - **LifecycleError**: Ошибки lifecycle management  
//! - **MetricsError**: Ошибки metrics collection
//! - **ValidationError**: Ошибки dependency validation
//! - **ConfigurationError**: Ошибки configuration system
//!
//! # ERROR HANDLING PATTERNS
//!
//! ```rust
//! // ❌ ОПАСНО - может привести к panic
//! let coordinator = coordinators.as_ref().unwrap().embedding_coordinator.clone();
//!
//! // ✅ БЕЗОПАСНО - graceful error handling  
//! let coordinator = coordinators.as_ref()
//!     .ok_or(DIError::CoordinatorNotInitialized("embedding_coordinator".to_string()))?
//!     .embedding_coordinator.clone();
//!
//! // ❌ ОПАСНО - async unwrap без context
//! manager.initialize(|| async { Ok(()) }).await.unwrap();
//!
//! // ✅ БЕЗОПАСНО - async error handling с context
//! manager.initialize(|| async { Ok(()) }).await
//!     .with_context(|| "Failed to initialize lifecycle manager")?;
//! ```
//!
//! # ИНТЕГРАЦИЯ С EXISTING CODE
//!
//! Все новые error types имеют conversions в anyhow::Error для backward compatibility.
//! Существующий код может продолжать использовать anyhow::Result<T>.

use anyhow::Context;
use thiserror::Error;

/// Основной error type для всех DI операций в проекте MAGRAY CLI
///
/// Заменяет .unwrap() паттерны на structured error handling.
/// Содержит все возможные ошибки DI системы с rich context.
#[derive(Debug, Error, Clone)]
pub enum DIError {
    /// Ошибки создания и инициализации координаторов
    #[error("Coordinator error: {message}")]
    Coordinator {
        message: String,
        coordinator_type: String,
        operation: String,
    },

    /// Ошибки lifecycle management (initialize, recover, shutdown)
    #[error("Lifecycle error during {operation}: {message}")]
    Lifecycle {
        message: String,
        operation: String,
        component: Option<String>,
    },

    /// Ошибки metrics collection и reporting
    #[error("Metrics error: {message}")]
    Metrics {
        message: String,
        metric_type: String,
        value: Option<String>,
    },

    /// Ошибки dependency validation и graph operations
    #[error("Dependency validation error: {message}")]
    DependencyValidation {
        message: String,
        dependency_type: Option<String>,
        operation: String,
    },

    /// Ошибки конфигурации DI системы
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        config_type: String,
        field: Option<String>,
    },

    /// Ошибки factory operations
    #[error("Factory error: {message}")]
    Factory {
        message: String,
        factory_type: String,
        service_type: String,
    },

    /// Ошибки инициализации компонентов
    #[error("Component '{component}' not initialized: {reason}")]
    ComponentNotInitialized {
        component: String,
        reason: String,
        suggestion: Option<String>,
    },

    /// Ошибки timeout operations
    #[error("Operation timed out after {timeout_ms}ms: {operation}")]
    Timeout {
        operation: String,
        timeout_ms: u64,
        component: Option<String>,
    },

    /// Ошибки resource management
    #[error("Resource error: {message}")]
    Resource {
        message: String,
        resource_type: String,
        operation: String,
    },

    /// Ошибки concurrent operations
    #[error("Concurrency error: {message}")]
    Concurrency {
        message: String,
        operation: String,
        thread_info: Option<String>,
    },

    /// Unexpected state errors
    #[error("Invalid state: {message}")]
    InvalidState {
        message: String,
        expected_state: String,
        actual_state: String,
    },

    /// Graph operation errors (для dependency validation)
    #[error("Graph operation failed: {message}")]
    GraphOperation {
        message: String,
        operation: String,
        node_type: Option<String>,
    },
}

/// Specific error type для coordinator operations
#[derive(Debug, Error, Clone)]
pub enum CoordinatorError {
    #[error("Embedding coordinator not available: {reason}")]
    EmbeddingCoordinatorUnavailable { reason: String },

    #[error("Search coordinator not available: {reason}")]
    SearchCoordinatorUnavailable { reason: String },

    #[error("Promotion coordinator not available: {reason}")]
    PromotionCoordinatorUnavailable { reason: String },

    #[error("Backup coordinator not available: {reason}")]
    BackupCoordinatorUnavailable { reason: String },

    #[error("Coordinator factory failed: {factory_type} -> {service_type}")]
    FactoryFailed {
        factory_type: String,
        service_type: String,
        underlying_error: String,
    },

    #[error("Invalid coordinator configuration: {field}")]
    InvalidConfiguration { field: String, value: String },
}

/// Specific error type для lifecycle operations
#[derive(Debug, Error, Clone)]
pub enum LifecycleError {
    #[error("Initialization failed for {component}: {reason}")]
    InitializationFailed { component: String, reason: String },

    #[error("Recovery failed for {component}: {reason}")]
    RecoveryFailed { component: String, reason: String },

    #[error("Shutdown failed for {component}: {reason}")]
    ShutdownFailed { component: String, reason: String },

    #[error("Invalid lifecycle state transition: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Component {component} is in invalid state: {state}")]
    InvalidComponentState { component: String, state: String },

    #[error("Lifecycle timeout: {operation} took longer than {timeout_ms}ms")]
    Timeout { operation: String, timeout_ms: u64 },
}

/// Specific error type для metrics operations
#[derive(Debug, Error, Clone)]
pub enum MetricsError {
    #[error("Metrics calculation failed: {metric_type}")]
    CalculationFailed {
        metric_type: String,
        details: String,
    },

    #[error("Metrics collection error: {collector_type}")]
    CollectionFailed {
        collector_type: String,
        reason: String,
    },

    #[error("Invalid metrics value: {metric_type} = {value}")]
    InvalidValue { metric_type: String, value: String },

    #[error("Metrics reporter error: {reporter_type}")]
    ReporterError {
        reporter_type: String,
        operation: String,
    },

    #[error("Metrics not available: {metric_type}")]
    NotAvailable { metric_type: String },

    #[error("Metrics aggregation failed: {operation}")]
    AggregationFailed { operation: String, count: usize },
}

/// Specific error type для dependency validation
#[derive(Debug, Error, Clone)]
pub enum ValidationError {
    #[error("Circular dependency detected: {cycle}")]
    CircularDependency { cycle: String },

    #[error("Dependency not found: {dependency_type}")]
    DependencyNotFound { dependency_type: String },

    #[error("Graph operation failed: {operation}")]
    GraphOperationFailed { operation: String, details: String },

    #[error("Dependency resolution failed: {type_name}")]
    ResolutionFailed { type_name: String, reason: String },

    #[error("Invalid dependency relationship: {from} -> {to}")]
    InvalidRelationship { from: String, to: String },

    #[error("Dependency graph is corrupted: {details}")]
    GraphCorrupted { details: String },
}

impl DIError {
    /// Create coordinator error with context
    pub fn coordinator_error(
        message: impl Into<String>,
        coordinator_type: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        DIError::Coordinator {
            message: message.into(),
            coordinator_type: coordinator_type.into(),
            operation: operation.into(),
        }
    }

    /// Create lifecycle error with context
    pub fn lifecycle_error(
        message: impl Into<String>,
        operation: impl Into<String>,
        component: Option<impl Into<String>>,
    ) -> Self {
        DIError::Lifecycle {
            message: message.into(),
            operation: operation.into(),
            component: component.map(|c| c.into()),
        }
    }

    /// Create metrics error with context
    pub fn metrics_error(
        message: impl Into<String>,
        metric_type: impl Into<String>,
        value: Option<impl Into<String>>,
    ) -> Self {
        DIError::Metrics {
            message: message.into(),
            metric_type: metric_type.into(),
            value: value.map(|v| v.into()),
        }
    }

    /// Create dependency validation error
    pub fn validation_error(
        message: impl Into<String>,
        dependency_type: Option<impl Into<String>>,
        operation: impl Into<String>,
    ) -> Self {
        DIError::DependencyValidation {
            message: message.into(),
            dependency_type: dependency_type.map(|dt| dt.into()),
            operation: operation.into(),
        }
    }

    /// Create component not initialized error
    pub fn component_not_initialized(
        component: impl Into<String>,
        reason: impl Into<String>,
        suggestion: Option<impl Into<String>>,
    ) -> Self {
        DIError::ComponentNotInitialized {
            component: component.into(),
            reason: reason.into(),
            suggestion: suggestion.map(|s| s.into()),
        }
    }

    /// Create timeout error
    pub fn timeout_error(
        operation: impl Into<String>,
        timeout_ms: u64,
        component: Option<impl Into<String>>,
    ) -> Self {
        DIError::Timeout {
            operation: operation.into(),
            timeout_ms,
            component: component.map(|c| c.into()),
        }
    }

    /// Create graph operation error
    pub fn graph_operation_error(
        message: impl Into<String>,
        operation: impl Into<String>,
        node_type: Option<impl Into<String>>,
    ) -> Self {
        DIError::GraphOperation {
            message: message.into(),
            operation: operation.into(),
            node_type: node_type.map(|nt| nt.into()),
        }
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            DIError::Coordinator { .. } => false, // Coordinator errors usually require restart
            DIError::Lifecycle { .. } => true,    // Lifecycle can often be retried
            DIError::Metrics { .. } => true,      // Metrics errors don't affect core functionality
            DIError::DependencyValidation { .. } => false, // Dependency issues need code changes
            DIError::Configuration { .. } => false, // Config errors need manual fix
            DIError::Factory { .. } => false,     // Factory errors usually indicate code issues
            DIError::ComponentNotInitialized { .. } => true, // Can try to reinitialize
            DIError::Timeout { .. } => true,      // Timeouts can be retried
            DIError::Resource { .. } => true,     // Resource issues may be temporary
            DIError::Concurrency { .. } => true,  // Concurrency issues can often be retried
            DIError::InvalidState { .. } => false, // State issues need investigation
            DIError::GraphOperation { .. } => false, // Graph operations are deterministic
        }
    }

    /// Get error category for monitoring/alerting
    pub fn category(&self) -> &'static str {
        match self {
            DIError::Coordinator { .. } => "coordinator",
            DIError::Lifecycle { .. } => "lifecycle",
            DIError::Metrics { .. } => "metrics",
            DIError::DependencyValidation { .. } => "validation",
            DIError::Configuration { .. } => "configuration",
            DIError::Factory { .. } => "factory",
            DIError::ComponentNotInitialized { .. } => "initialization",
            DIError::Timeout { .. } => "timeout",
            DIError::Resource { .. } => "resource",
            DIError::Concurrency { .. } => "concurrency",
            DIError::InvalidState { .. } => "state",
            DIError::GraphOperation { .. } => "graph",
        }
    }
}

// Automatic conversions from specific errors to DIError
impl From<CoordinatorError> for DIError {
    fn from(err: CoordinatorError) -> Self {
        match err {
            CoordinatorError::EmbeddingCoordinatorUnavailable { reason } => {
                DIError::coordinator_error(reason, "embedding_coordinator", "access")
            }
            CoordinatorError::SearchCoordinatorUnavailable { reason } => {
                DIError::coordinator_error(reason, "search_coordinator", "access")
            }
            CoordinatorError::PromotionCoordinatorUnavailable { reason } => {
                DIError::coordinator_error(reason, "promotion_coordinator", "access")
            }
            CoordinatorError::BackupCoordinatorUnavailable { reason } => {
                DIError::coordinator_error(reason, "backup_coordinator", "access")
            }
            CoordinatorError::FactoryFailed {
                factory_type,
                service_type,
                underlying_error,
            } => DIError::Factory {
                message: underlying_error,
                factory_type,
                service_type,
            },
            CoordinatorError::InvalidConfiguration { field, value } => DIError::Configuration {
                message: format!("Invalid value '{}' for field '{}'", value, field),
                config_type: "coordinator_config".to_string(),
                field: Some(field),
            },
        }
    }
}

impl From<LifecycleError> for DIError {
    fn from(err: LifecycleError) -> Self {
        match err {
            LifecycleError::InitializationFailed { component, reason } => {
                DIError::lifecycle_error(reason, "initialize", Some(component))
            }
            LifecycleError::RecoveryFailed { component, reason } => {
                DIError::lifecycle_error(reason, "recover", Some(component))
            }
            LifecycleError::ShutdownFailed { component, reason } => {
                DIError::lifecycle_error(reason, "shutdown", Some(component))
            }
            LifecycleError::InvalidStateTransition { from, to } => DIError::InvalidState {
                message: format!("Invalid transition from '{}' to '{}'", from, to),
                expected_state: to,
                actual_state: from,
            },
            LifecycleError::InvalidComponentState { component, state } => DIError::InvalidState {
                message: format!("Component '{}' in invalid state '{}'", component, state),
                expected_state: "initialized".to_string(),
                actual_state: state,
            },
            LifecycleError::Timeout {
                operation,
                timeout_ms,
            } => DIError::timeout_error(operation, timeout_ms, None::<String>),
        }
    }
}

impl From<MetricsError> for DIError {
    fn from(err: MetricsError) -> Self {
        match err {
            MetricsError::CalculationFailed {
                metric_type,
                details,
            } => DIError::metrics_error(details, metric_type, None::<String>),
            MetricsError::CollectionFailed {
                collector_type,
                reason,
            } => DIError::metrics_error(reason, collector_type, None::<String>),
            MetricsError::InvalidValue { metric_type, value } => DIError::metrics_error(
                format!("Invalid value: {}", value),
                metric_type,
                Some(value),
            ),
            MetricsError::ReporterError {
                reporter_type,
                operation,
            } => DIError::metrics_error(
                format!("Reporter error during {}", operation),
                reporter_type,
                None::<String>,
            ),
            MetricsError::NotAvailable { metric_type } => {
                DIError::metrics_error("Metric not available", metric_type, None::<String>)
            }
            MetricsError::AggregationFailed { operation, count } => DIError::metrics_error(
                format!("Aggregation of {} items failed", count),
                operation,
                Some(count.to_string()),
            ),
        }
    }
}

impl From<ValidationError> for DIError {
    fn from(err: ValidationError) -> Self {
        match err {
            ValidationError::CircularDependency { cycle } => DIError::validation_error(
                format!("Circular dependency: {}", cycle),
                None::<String>,
                "cycle_detection",
            ),
            ValidationError::DependencyNotFound { dependency_type } => DIError::validation_error(
                "Dependency not found",
                Some(dependency_type),
                "resolution",
            ),
            ValidationError::GraphOperationFailed { operation, details } => {
                DIError::graph_operation_error(details, operation, None::<String>)
            }
            ValidationError::ResolutionFailed { type_name, reason } => {
                DIError::validation_error(reason, Some(type_name), "resolution")
            }
            ValidationError::InvalidRelationship { from, to } => DIError::validation_error(
                format!("Invalid relationship: {} -> {}", from, to),
                None::<String>,
                "relationship_validation",
            ),
            ValidationError::GraphCorrupted { details } => {
                DIError::graph_operation_error(details, "integrity_check", None::<String>)
            }
        }
    }
}

// anyhow автоматически поддерживает From<E> для любых типов реализующих std::error::Error
// Поэтому эта реализация не нужна

/// Helper trait для добавления DI context к anyhow errors
pub trait DIContextExt<T> {
    fn di_context(self, message: &str) -> anyhow::Result<T>;
    fn di_with_context<F>(self, f: F) -> anyhow::Result<T>
    where
        F: FnOnce() -> String;
}

impl<T> DIContextExt<T> for Result<T, DIError> {
    fn di_context(self, message: &str) -> anyhow::Result<T> {
        self.map_err(anyhow::Error::from)
            .with_context(|| message.to_string())
    }

    fn di_with_context<F>(self, f: F) -> anyhow::Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(anyhow::Error::from).with_context(f)
    }
}

// Removed generic impl to avoid trait conflicts with Result<T, DIError>
// Use anyhow::Context directly for other error types

/// Convenience macro для создания DIError с context
#[macro_export]
macro_rules! di_error {
    (coordinator: $msg:expr, $coord_type:expr, $op:expr) => {
        $crate::di::errors::DIError::coordinator_error($msg, $coord_type, $op)
    };

    (lifecycle: $msg:expr, $op:expr) => {
        $crate::di::errors::DIError::lifecycle_error($msg, $op, None::<String>)
    };

    (lifecycle: $msg:expr, $op:expr, $component:expr) => {
        $crate::di::errors::DIError::lifecycle_error($msg, $op, Some($component))
    };

    (metrics: $msg:expr, $metric_type:expr) => {
        $crate::di::errors::DIError::metrics_error($msg, $metric_type, None::<String>)
    };

    (validation: $msg:expr, $op:expr) => {
        $crate::di::errors::DIError::validation_error($msg, None::<String>, $op)
    };

    (not_initialized: $component:expr, $reason:expr) => {
        $crate::di::errors::DIError::component_not_initialized($component, $reason, None::<String>)
    };

    (timeout: $op:expr, $timeout_ms:expr) => {
        $crate::di::errors::DIError::timeout_error($op, $timeout_ms, None::<String>)
    };

    (graph: $msg:expr, $op:expr) => {
        $crate::di::errors::DIError::graph_operation_error($msg, $op, None::<String>)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = DIError::coordinator_error(
            "Test coordinator error",
            "embedding_coordinator",
            "initialization",
        );

        assert_eq!(error.category(), "coordinator");
        assert!(!error.is_recoverable());
        assert!(error.to_string().contains("embedding_coordinator"));
    }

    #[test]
    fn test_error_conversions() {
        let coord_err = CoordinatorError::EmbeddingCoordinatorUnavailable {
            reason: "Not initialized".to_string(),
        };

        let di_err: DIError = coord_err.into();
        assert_eq!(di_err.category(), "coordinator");

        let anyhow_err: anyhow::Error = di_err.into();
        assert!(anyhow_err.to_string().contains("Not initialized"));
    }

    #[test]
    fn test_macro_usage() {
        let error = di_error!(
            coordinator: "Test message",
            "test_coordinator",
            "test_operation"
        );

        match error {
            DIError::Coordinator {
                coordinator_type, ..
            } => {
                assert_eq!(coordinator_type, "test_coordinator");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_error_chaining() {
        let result: Result<(), DIError> = Err(DIError::component_not_initialized(
            "test_component",
            "initialization failed",
            Some("Call initialize() first"),
        ));

        let chained = result.di_context("During system startup");
        assert!(chained.is_err());

        let error_msg = chained.unwrap_err().to_string();
        assert!(error_msg.contains("During system startup"));
        assert!(error_msg.contains("test_component"));
    }
}
