//! Unit Tests for DIError System
//! 
//! Comprehensive testing of error handling including:
//! - Error type creation and classification
//! - Error chaining and context preservation
//! - Error conversion and propagation
//! - Error recovery strategies and fallback mechanisms
//! - Error logging and debugging information
//! - Custom error types and user-defined errors

use std::sync::Arc;
use std::io;

use crate::{
    di::{
        errors::{DIError, DIResult, ErrorContext, ErrorSeverity, ErrorRecoveryStrategy},
        unified_container::UnifiedDIContainer,
        unified_config::UnifiedDIConfiguration,
    },
    services::monitoring_service::MonitoringService,
};

#[tokio::test]
async fn test_error_type_creation_and_classification() -> Result<(), Box<dyn std::error::Error>> {
    // ServiceNotFound error
    let service_not_found = DIError::ServiceNotFound {
        service_name: "TestService".to_string(),
    };
    
    assert_eq!(service_not_found.severity(), ErrorSeverity::High);
    assert!(service_not_found.is_recoverable());
    assert!(service_not_found.to_string().contains("TestService"));
    
    // ServiceCreationFailed error
    let io_error = Box::new(io::Error::new(io::ErrorKind::PermissionDenied, "Access denied"));
    let creation_failed = DIError::ServiceCreationFailed {
        service_name: "FailedService".to_string(),
        source: io_error,
    };
    
    assert_eq!(creation_failed.severity(), ErrorSeverity::Critical);
    assert!(!creation_failed.is_recoverable());
    assert!(creation_failed.to_string().contains("FailedService"));
    
    // ConfigurationError
    let config_error = DIError::ConfigurationError {
        field: "max_services".to_string(),
        source: Box::new(io::Error::new(io::ErrorKind::InvalidInput, "Invalid value")),
    };
    
    assert_eq!(config_error.severity(), ErrorSeverity::High);
    assert!(config_error.is_recoverable());
    assert!(config_error.to_string().contains("max_services"));
    
    // DependencyError
    let dependency_error = DIError::DependencyError {
        service_name: "ServiceA".to_string(),
        dependency_name: "ServiceB".to_string(),
        reason: "Circular dependency detected".to_string(),
    };
    
    assert_eq!(dependency_error.severity(), ErrorSeverity::Critical);
    assert!(!dependency_error.is_recoverable());
    
    // TimeoutError
    let timeout_error = DIError::TimeoutError {
        operation: "container_initialization".to_string(),
        timeout_ms: 30000,
    };
    
    assert_eq!(timeout_error.severity(), ErrorSeverity::Medium);
    assert!(timeout_error.is_recoverable());
    
    Ok(())
}

#[tokio::test]
async fn test_error_chaining_and_context_preservation() -> Result<(), Box<dyn std::error::Error>> {
    // Создаем цепочку ошибок
    let root_cause = io::Error::new(io::ErrorKind::ConnectionRefused, "Database connection failed");
    
    let service_error = DIError::ServiceCreationFailed {
        service_name: "DatabaseService".to_string(),
        source: Box::new(root_cause),
    };
    
    let container_error = DIError::ContainerError {
        operation: "service_initialization".to_string(),
        source: Box::new(service_error),
    };
    
    // Проверяем что контекст сохранился через всю цепочку
    let error_string = container_error.to_string();
    assert!(error_string.contains("service_initialization"));
    assert!(error_string.contains("DatabaseService"));
    assert!(error_string.contains("Database connection failed"));
    
    // Проверяем что можно получить source errors
    let source = container_error.source().expect("Test operation should succeed");
    assert!(source.to_string().contains("DatabaseService"));
    
    Ok(())
}

#[tokio::test]
async fn test_error_context_enhancement() -> Result<(), Box<dyn std::error::Error>> {
    let mut error_context = ErrorContext::new();
    
    // Добавляем контекстную информацию
    error_context.add_context("service_name", "TestService");
    error_context.add_context("operation", "initialization");
    error_context.add_context("timestamp", "2024-01-01T12:00:00Z");
    error_context.add_context("thread_id", "main");
    
    // Создаем ошибку с контекстом
    let base_error = DIError::ServiceNotFound {
        service_name: "TestService".to_string(),
    };
    
    let enhanced_error = base_error.with_context(error_context);
    
    // Проверяем что контекст присутствует
    let debug_info = enhanced_error.get_debug_info();
    assert!(debug_info.contains("service_name: TestService"));
    assert!(debug_info.contains("operation: initialization"));
    assert!(debug_info.contains("timestamp: 2024-01-01T12:00:00Z"));
    assert!(debug_info.contains("thread_id: main"));
    
    Ok(())
}

#[tokio::test]
async fn test_error_conversion_and_propagation() -> Result<(), Box<dyn std::error::Error>> {
    // Тестируем конвертацию разных типов ошибок в DIError
    
    // IO Error -> DIError
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let di_error: DIError = io_error.into();
    
    match di_error {
        DIError::IoError { source, .. } => {
            assert!(source.to_string().contains("File not found"));
        }
        _ => panic!("Expected IoError variant"),
    }
    
    // std::error::Error -> DIError через generic conversion
    let generic_error = Box::new(io::Error::new(io::ErrorKind::Other, "Generic error")) 
        as Box<dyn std::error::Error + Send + Sync>;
    let converted_error = DIError::from_generic_error("test_operation", generic_error);
    
    assert!(converted_error.to_string().contains("test_operation"));
    assert!(converted_error.to_string().contains("Generic error"));
    
    Ok(())
}

#[tokio::test]
async fn test_error_recovery_strategies() -> Result<(), Box<dyn std::error::Error>> {
    // Тестируем разные стратегии восстановления после ошибок
    
    let recoverable_error = DIError::TimeoutError {
        operation: "service_resolution".to_string(),
        timeout_ms: 1000,
    };
    
    // Стратегия retry
    let retry_strategy = recoverable_error.get_recovery_strategy();
    match retry_strategy {
        ErrorRecoveryStrategy::Retry { max_attempts, delay_ms } => {
            assert!(max_attempts > 0);
            assert!(delay_ms > 0);
        }
        _ => panic!("Expected Retry strategy for timeout error"),
    }
    
    let critical_error = DIError::ServiceCreationFailed {
        service_name: "CriticalService".to_string(),
        source: Box::new(io::Error::new(io::ErrorKind::PermissionDenied, "Access denied")),
    };
    
    // Стратегия fail fast для критических ошибок
    let fail_strategy = critical_error.get_recovery_strategy();
    match fail_strategy {
        ErrorRecoveryStrategy::FailFast => {
            // Правильная стратегия для критических ошибок
        }
        _ => panic!("Expected FailFast strategy for critical error"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_error_logging_and_debugging() -> Result<(), Box<dyn std::error::Error>> {
    let error = DIError::DependencyError {
        service_name: "ServiceA".to_string(),
        dependency_name: "ServiceB".to_string(),
        reason: "Circular dependency in initialization chain".to_string(),
    };
    
    // Тестируем различные уровни детализации логирования
    let brief_log = error.to_brief_string();
    assert!(brief_log.len() < error.to_string().len());
    assert!(brief_log.contains("ServiceA"));
    assert!(brief_log.contains("ServiceB"));
    
    let detailed_log = error.to_detailed_string();
    assert!(detailed_log.len() > error.to_string().len());
    assert!(detailed_log.contains("Circular dependency"));
    assert!(detailed_log.contains("severity"));
    assert!(detailed_log.contains("recoverable"));
    
    let debug_info = error.get_debug_info();
    assert!(debug_info.contains("error_type"));
    assert!(debug_info.contains("timestamp"));
    assert!(debug_info.contains("thread_info"));
    
    Ok(())
}

#[tokio::test]
async fn test_error_aggregation_and_batch_handling() -> Result<(), Box<dyn std::error::Error>> {
    // Создаем множественные ошибки
    let errors = vec![
        DIError::ServiceNotFound { 
            service_name: "Service1".to_string() 
        },
        DIError::ServiceNotFound { 
            service_name: "Service2".to_string() 
        },
        DIError::ConfigurationError {
            field: "timeout".to_string(),
            source: Box::new(io::Error::new(io::ErrorKind::InvalidInput, "Invalid timeout")),
        },
    ];
    
    // Агрегируем ошибки
    let aggregated_error = DIError::create_aggregate_error("batch_operation", errors);
    
    match aggregated_error {
        DIError::AggregateError { operation, errors, .. } => {
            assert_eq!(operation, "batch_operation");
            assert_eq!(errors.len(), 3);
            
            let error_summary = aggregated_error.to_string();
            assert!(error_summary.contains("Service1"));
            assert!(error_summary.contains("Service2"));
            assert!(error_summary.contains("timeout"));
        }
        _ => panic!("Expected AggregateError"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_error_metrics_and_monitoring() -> Result<(), Box<dyn std::error::Error>> {
    let mut error_metrics = DIError::create_metrics_collector();
    
    // Записываем различные типы ошибок
    let error1 = DIError::ServiceNotFound { service_name: "Test1".to_string() };
    let error2 = DIError::TimeoutError { operation: "test_op".to_string(), timeout_ms: 1000 };
    let error3 = DIError::ServiceNotFound { service_name: "Test2".to_string() };
    
    error_metrics.record_error(&error1);
    error_metrics.record_error(&error2);
    error_metrics.record_error(&error3);
    
    // Получаем статистику ошибок
    let stats = error_metrics.get_statistics();
    
    assert_eq!(stats.total_errors, 3);
    assert_eq!(stats.error_counts.get("ServiceNotFound").unwrap_or(&0), &2);
    assert_eq!(stats.error_counts.get("TimeoutError").unwrap_or(&0), &1);
    
    // Проверяем severity distribution
    assert_eq!(stats.severity_distribution.get(&ErrorSeverity::High).unwrap_or(&0), &2);
    assert_eq!(stats.severity_distribution.get(&ErrorSeverity::Medium).unwrap_or(&0), &1);
    
    Ok(())
}

#[tokio::test]
async fn test_error_in_real_container_scenarios() -> DIResult<()> {
    // Интеграционный тест ошибок в реальных сценариях работы с контейнером
    
    let config = UnifiedDIConfiguration::test_config()?;
    let container = UnifiedDIContainer::new(config).await?;
    
    // Тестируем ServiceNotFound error
    let not_found_result = container.resolve::<MonitoringService>().await;
    assert!(not_found_result.is_err());
    
    match not_found_result.unwrap_err() {
        DIError::ServiceNotFound { service_name } => {
            assert!(service_name.contains("MonitoringService"));
        }
        _ => panic!("Expected ServiceNotFound error"),
    }
    
    // Тестируем ошибки после shutdown
    container.shutdown().await?;
    
    let post_shutdown_result = container.resolve::<MonitoringService>().await;
    assert!(post_shutdown_result.is_err());
    
    match post_shutdown_result.unwrap_err() {
        DIError::ContainerError { operation, .. } => {
            assert!(operation.contains("resolve") || operation.contains("shutdown"));
        }
        _ => panic!("Expected ContainerError after shutdown"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_custom_error_types_and_extensions() -> Result<(), Box<dyn std::error::Error>> {
    // Тестируем расширение системы ошибок пользовательскими типами
    
    #[derive(Debug)]
    struct CustomBusinessError {
        code: String,
        message: String,
    }
    
    impl std::fmt::Display for CustomBusinessError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Business Error [{}]: {}", self.code, self.message)
        }
    }
    
    impl std::error::Error for CustomBusinessError {}
    
    let custom_error = CustomBusinessError {
        code: "BUSINESS_001".to_string(),
        message: "Invalid business rule violation".to_string(),
    };
    
    // Конвертируем в DIError
    let di_error = DIError::BusinessRuleViolation {
        rule_name: "CustomRule".to_string(),
        source: Box::new(custom_error),
    };
    
    // Проверяем что custom error информация сохранилась
    assert!(di_error.to_string().contains("BUSINESS_001"));
    assert!(di_error.to_string().contains("Invalid business rule violation"));
    assert!(di_error.to_string().contains("CustomRule"));
    
    // Проверяем severity и recovery для бизнес-ошибок
    assert_eq!(di_error.severity(), ErrorSeverity::Medium);
    assert!(di_error.is_recoverable());
    
    Ok(())
}

#[tokio::test]
async fn test_error_serialization_for_logging() -> Result<(), Box<dyn std::error::Error>> {
    let error = DIError::ServiceCreationFailed {
        service_name: "SerializationTestService".to_string(),
        source: Box::new(io::Error::new(io::ErrorKind::Other, "Serialization test error")),
    };
    
    // JSON serialization для structured logging
    let json_representation = error.to_json()?;
    assert!(json_representation.contains("SerializationTestService"));
    assert!(json_representation.contains("Serialization test error"));
    assert!(json_representation.contains("severity"));
    
    // Structured logging format
    let structured_log = error.to_structured_log();
    assert!(structured_log.contains("service_name"));
    assert!(structured_log.contains("error_type"));
    assert!(structured_log.contains("timestamp"));
    assert!(structured_log.contains("is_recoverable"));
    
    Ok(())
}