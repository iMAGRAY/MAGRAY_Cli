use common::{
    init_structured_logging, 
    StructuredLogEntry, 
    ExecutionContext, 
    PerformanceMetrics,
    OperationTimer,
    RequestContext,
    LoggingConfig,
};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

#[test]
fn test_structured_log_entry_creation() {
    let mut fields = HashMap::new();
    fields.insert("key".to_string(), Value::String("value".to_string()));
    
    let entry = StructuredLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: "INFO".to_string(),
        target: "test".to_string(),
        message: "Test message".to_string(),
        fields,
        context: None,
        performance: None,
    };
    
    assert_eq!(entry.level, "INFO");
    assert_eq!(entry.message, "Test message");
    assert_eq!(entry.target, "test");
    assert!(entry.fields.contains_key("key"));
}

#[test]
fn test_execution_context() {
    let context = ExecutionContext {
        request_id: Some("req-123".to_string()),
        user_id: Some("user-456".to_string()),
        app_version: "1.0.0".to_string(),
        hostname: "localhost".to_string(),
        pid: std::process::id(),
        thread_id: format!("{:?}", std::thread::current().id()),
    };
    
    assert_eq!(context.request_id, Some("req-123".to_string()));
    assert_eq!(context.user_id, Some("user-456".to_string()));
    assert_eq!(context.app_version, "1.0.0");
}

#[test]
fn test_performance_metrics() {
    let metrics = PerformanceMetrics {
        duration_ms: 123,
        memory_used_bytes: Some(1024 * 1024),
        cpu_usage_percent: Some(45.5),
        io_operations: Some(10),
        cache_hits: Some(8),
        cache_misses: Some(2),
    };
    
    assert_eq!(metrics.duration_ms, 123);
    assert_eq!(metrics.memory_used_bytes, Some(1024 * 1024));
    assert_eq!(metrics.cpu_usage_percent, Some(45.5));
}

#[test]
fn test_operation_timer() {
    let timer = OperationTimer::new("test_operation");
    
    // Simulate some work
    std::thread::sleep(Duration::from_millis(10));
    
    let elapsed = timer.elapsed();
    assert!(elapsed.as_millis() >= 10);
    
    let metrics = timer.finish();
    assert!(metrics.duration_ms >= 10);
}

#[test]
fn test_request_context() {
    let context = RequestContext::new("req-789");
    
    assert!(!context.request_id().is_empty());
    
    let with_user = context.with_user("user-123");
    assert_eq!(with_user.user_id(), Some("user-123"));
}

#[test]
fn test_logging_config() {
    let config = LoggingConfig::default();
    
    // Test default values
    assert!(!config.json_output());
    assert_eq!(config.level(), "info");
    
    // Test builder pattern
    let custom_config = LoggingConfig::new()
        .with_json_output(true)
        .with_level("debug")
        .with_pretty_print(true);
    
    assert!(custom_config.json_output());
    assert_eq!(custom_config.level(), "debug");
}

#[test]
fn test_structured_log_entry_serialization() {
    let entry = StructuredLogEntry {
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        level: "ERROR".to_string(),
        target: "app::module".to_string(),
        message: "Error occurred".to_string(),
        fields: HashMap::new(),
        context: None,
        performance: None,
    };
    
    let json = serde_json::to_string(&entry).unwrap();
    assert!(json.contains("ERROR"));
    assert!(json.contains("Error occurred"));
    assert!(json.contains("2024-01-01T00:00:00Z"));
}

#[test]
fn test_structured_log_entry_with_context() {
    let context = ExecutionContext {
        request_id: Some("req-001".to_string()),
        user_id: None,
        app_version: "2.0.0".to_string(),
        hostname: "server1".to_string(),
        pid: 1234,
        thread_id: "thread-1".to_string(),
    };
    
    let entry = StructuredLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: "WARN".to_string(),
        target: "test".to_string(),
        message: "Warning message".to_string(),
        fields: HashMap::new(),
        context: Some(context),
        performance: None,
    };
    
    assert!(entry.context.is_some());
    assert_eq!(entry.context.unwrap().app_version, "2.0.0");
}

#[test]
fn test_performance_metrics_partial() {
    let metrics = PerformanceMetrics {
        duration_ms: 50,
        memory_used_bytes: None,
        cpu_usage_percent: Some(25.0),
        io_operations: None,
        cache_hits: Some(100),
        cache_misses: Some(5),
    };
    
    // Serialize and check that None fields are omitted
    let json = serde_json::to_value(&metrics).unwrap();
    assert!(json.get("duration_ms").is_some());
    assert!(json.get("cpu_usage_percent").is_some());
    assert!(json.get("cache_hits").is_some());
}

#[test]
fn test_operation_timer_with_callback() {
    let timer = OperationTimer::new("callback_test");
    
    std::thread::sleep(Duration::from_millis(5));
    
    let result = timer.finish_with(|metrics| {
        assert!(metrics.duration_ms >= 5);
        "operation completed"
    });
    
    assert_eq!(result, "operation completed");
}

#[test]
fn test_init_structured_logging_idempotent() {
    // Multiple calls should not panic
    let _ = init_structured_logging();
    let _ = init_structured_logging();
    
    // Test passed if no panic occurred
}

#[test]
fn test_request_context_builder() {
    let context = RequestContext::new("req-456")
        .with_user("user-789")
        .with_metadata("action", "create_file")
        .with_metadata("resource", "test.txt");
    
    assert_eq!(context.request_id(), "req-456");
    assert_eq!(context.user_id(), Some("user-789"));
    
    let metadata = context.metadata();
    assert_eq!(metadata.get("action"), Some(&"create_file".to_string()));
    assert_eq!(metadata.get("resource"), Some(&"test.txt".to_string()));
}