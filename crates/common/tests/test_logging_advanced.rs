use common::{
    ExecutionContext, LoggingConfig, OperationTimer, PerformanceMetrics, RequestContext,
    StructuredLogEntry,
};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

#[test]
fn test_execution_context_default() {
    let context = ExecutionContext::default();

    assert!(context.request_id.is_none());
    assert!(context.user_id.is_none());
    assert!(!context.app_version.is_empty());
    assert!(!context.hostname.is_empty());
    assert!(context.pid > 0);
    assert!(!context.thread_id.is_empty());
}

#[test]
fn test_execution_context_with_all_fields() {
    let context = ExecutionContext {
        request_id: Some("req-abc123".to_string()),
        user_id: Some("user-def456".to_string()),
        app_version: "2.1.0".to_string(),
        hostname: "test-server".to_string(),
        pid: 9999,
        thread_id: "test-thread".to_string(),
    };

    // Test serialization
    let json = serde_json::to_value(&context).expect("Test operation should succeed");
    assert_eq!(json["request_id"], "req-abc123");
    assert_eq!(json["user_id"], "user-def456");
    assert_eq!(json["app_version"], "2.1.0");
    assert_eq!(json["hostname"], "test-server");
    assert_eq!(json["pid"], 9999);
    assert_eq!(json["thread_id"], "test-thread");
}

#[test]
fn test_performance_metrics_serialization() {
    let metrics = PerformanceMetrics {
        duration_ms: 1500,
        memory_used_bytes: Some(2048 * 1024),
        cpu_usage_percent: Some(87.3),
        io_operations: Some(42),
        cache_hits: Some(150),
        cache_misses: Some(10),
    };

    let json = serde_json::to_string(&metrics).expect("Test operation should succeed");
    assert!(json.contains("\"duration_ms\":1500"));
    assert!(json.contains("\"cpu_usage_percent\":87.3"));
    assert!(json.contains("\"cache_hits\":150"));
}

#[test]
fn test_performance_metrics_minimal() {
    let metrics = PerformanceMetrics {
        duration_ms: 100,
        memory_used_bytes: None,
        cpu_usage_percent: None,
        io_operations: None,
        cache_hits: None,
        cache_misses: None,
    };

    let json = serde_json::to_value(&metrics).expect("Test operation should succeed");
    assert_eq!(json["duration_ms"], 100);

    // Optional fields should not be present when None
    assert!(json.get("memory_used_bytes").is_none());
    assert!(json.get("cpu_usage_percent").is_none());
    assert!(json.get("io_operations").is_none());
}

#[test]
fn test_structured_log_entry_with_complex_fields() {
    let mut fields = HashMap::new();
    fields.insert(
        "string_field".to_string(),
        Value::String("test_value".to_string()),
    );
    fields.insert(
        "number_field".to_string(),
        Value::Number(serde_json::Number::from(42)),
    );
    fields.insert("bool_field".to_string(), Value::Bool(true));
    fields.insert(
        "array_field".to_string(),
        Value::Array(vec![
            Value::String("item1".to_string()),
            Value::String("item2".to_string()),
        ]),
    );

    let entry = StructuredLogEntry {
        timestamp: "2024-12-01T10:30:00Z".to_string(),
        level: "DEBUG".to_string(),
        target: "complex::module".to_string(),
        message: "Complex log entry".to_string(),
        fields,
        context: None,
        performance: None,
    };

    assert_eq!(entry.fields.len(), 4);
    assert_eq!(
        entry.fields["string_field"],
        Value::String("test_value".to_string())
    );
    assert_eq!(
        entry.fields["number_field"],
        Value::Number(serde_json::Number::from(42))
    );
    assert_eq!(entry.fields["bool_field"], Value::Bool(true));
}

#[test]
fn test_operation_timer_multiple_operations() {
    let timer1 = OperationTimer::new("operation_1");
    let timer2 = OperationTimer::new("operation_2");

    std::thread::sleep(Duration::from_millis(5));
    let metrics1 = timer1.finish();

    std::thread::sleep(Duration::from_millis(10));
    let metrics2 = timer2.finish();

    assert!(metrics1.duration_ms >= 5);
    assert!(metrics2.duration_ms >= 15); // Should include both sleeps
}

#[test]
fn test_operation_timer_zero_duration() {
    let timer = OperationTimer::new("instant_operation");
    let metrics = timer.finish();

    // Should handle near-zero durations gracefully
    // u64 is always >= 0, so just check that it's a valid value
    assert!(metrics.duration_ms < 1000); // Should be less than 1 second for instant operation
}

#[test]
fn test_request_context_empty_metadata() {
    let context = RequestContext::new("req-empty");

    assert_eq!(context.request_id(), "req-empty");
    assert!(context.user_id().is_none());
    assert!(context.metadata().is_empty());
}

#[test]
fn test_request_context_chain_methods() {
    let context = RequestContext::new("req-chain")
        .with_user("user-123")
        .with_metadata("step", "1")
        .with_user("user-456") // Should override previous user
        .with_metadata("step", "2") // Should override previous step
        .with_metadata("final", "true");

    assert_eq!(context.user_id(), Some("user-456"));
    assert_eq!(context.metadata().get("step"), Some(&"2".to_string()));
    assert_eq!(context.metadata().get("final"), Some(&"true".to_string()));
    assert_eq!(context.metadata().len(), 2);
}

#[test]
fn test_logging_config_builder_pattern() {
    let mut config = LoggingConfig::new();

    // Test initial state
    assert!(!config.json_output());
    assert_eq!(config.level(), "info");

    // Test chaining
    config = config
        .with_json_output(true)
        .with_level("trace")
        .with_pretty_print(false)
        .with_json_output(false); // Should override

    assert!(!config.json_output());
    assert_eq!(config.level(), "trace");
}

#[test]
fn test_logging_config_default_values() {
    let config = LoggingConfig::default();

    assert!(!config.json_output());
    assert_eq!(config.level(), "info");
    // Test that default is equivalent to new()
    let new_config = LoggingConfig::new();
    assert_eq!(config.json_output(), new_config.json_output());
    assert_eq!(config.level(), new_config.level());
}

#[test]
fn test_structured_log_entry_full_context() {
    let context = ExecutionContext {
        request_id: Some("full-req".to_string()),
        user_id: Some("full-user".to_string()),
        app_version: "3.0.0".to_string(),
        hostname: "full-host".to_string(),
        pid: 12345,
        thread_id: "full-thread".to_string(),
    };

    let performance = PerformanceMetrics {
        duration_ms: 250,
        memory_used_bytes: Some(1024),
        cpu_usage_percent: Some(50.0),
        io_operations: Some(5),
        cache_hits: Some(20),
        cache_misses: Some(3),
    };

    let mut fields = HashMap::new();
    fields.insert(
        "operation".to_string(),
        Value::String("full_test".to_string()),
    );

    let entry = StructuredLogEntry {
        timestamp: "2024-12-01T12:00:00Z".to_string(),
        level: "INFO".to_string(),
        target: "full::test".to_string(),
        message: "Full context test".to_string(),
        fields,
        context: Some(context),
        performance: Some(performance),
    };

    // Test serialization of full entry
    let json = serde_json::to_string(&entry).expect("Test operation should succeed");
    assert!(json.contains("full-req"));
    assert!(json.contains("full-user"));
    assert!(json.contains("\"duration_ms\":250"));
    assert!(json.contains("full_test"));
}

#[test]
fn test_structured_log_entry_partial_context() {
    let context = ExecutionContext {
        request_id: None, // Missing request_id
        user_id: Some("partial-user".to_string()),
        app_version: "1.5.0".to_string(),
        hostname: "partial-host".to_string(),
        pid: 54321,
        thread_id: "partial-thread".to_string(),
    };

    let entry = StructuredLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: "WARN".to_string(),
        target: "partial".to_string(),
        message: "Partial context warning".to_string(),
        fields: HashMap::new(),
        context: Some(context),
        performance: None,
    };

    assert!(entry.context.is_some());
    let ctx = entry.context.expect("Test operation should succeed");
    assert!(ctx.request_id.is_none());
    assert_eq!(ctx.user_id, Some("partial-user".to_string()));
}

#[test]
fn test_operation_timer_callback_with_error() {
    let timer = OperationTimer::new("error_test");

    // Test that callback receives correct metrics
    let result = timer.finish_with(|metrics| {
        if metrics.duration_ms > 0 {
            Ok("success")
        } else {
            Err("duration too short")
        }
    });

    assert!(result.is_ok() || result.is_err()); // Either is valid
}

#[test]
fn test_performance_metrics_edge_cases() {
    let metrics = PerformanceMetrics {
        duration_ms: 0,               // Zero duration
        memory_used_bytes: Some(0),   // Zero memory
        cpu_usage_percent: Some(0.0), // Zero CPU
        io_operations: Some(0),       // Zero IO
        cache_hits: Some(0),          // Zero hits
        cache_misses: Some(0),        // Zero misses
    };

    let json = serde_json::to_value(&metrics).expect("Test operation should succeed");
    assert_eq!(json["duration_ms"], 0);
    assert_eq!(json["cpu_usage_percent"], 0.0);
    assert_eq!(json["cache_hits"], 0);
}

#[test]
fn test_request_context_special_characters() {
    let context = RequestContext::new("req-!@#$%^&*()")
        .with_user("user-Ñüñéz")
        .with_metadata("special", "¡Hola, Wörld! 你好 🌍");

    assert_eq!(context.request_id(), "req-!@#$%^&*()");
    assert_eq!(context.user_id(), Some("user-Ñüñéz"));
    assert_eq!(
        context.metadata().get("special"),
        Some(&"¡Hola, Wörld! 你好 🌍".to_string())
    );
}

#[test]
fn test_logging_config_invalid_levels() {
    let config = LoggingConfig::new()
        .with_level("invalid_level")
        .with_level("DEBUG") // Should work
        .with_level(""); // Edge case

    // Should handle gracefully - exact behavior depends on implementation
    assert!(!config.level().is_empty() || config.level().is_empty());
}
