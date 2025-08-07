use chrono::Utc;
use common::structured_logging::*;
use serde_json::{json, Value};
use std::collections::HashMap;

// Тесты для StructuredLogEntry
#[test]
fn test_structured_log_entry_full() {
    let mut fields = HashMap::new();
    fields.insert("component".to_string(), json!("test-component"));
    fields.insert("action".to_string(), json!("test-action"));
    fields.insert("count".to_string(), json!(42));

    let context = ExecutionContext {
        request_id: Some("req-123".to_string()),
        user_id: Some("user-456".to_string()),
        app_version: "1.0.0".to_string(),
        hostname: "test-host".to_string(),
        pid: 12345,
        thread_id: "thread-1".to_string(),
    };

    let performance = PerformanceMetrics {
        duration_ms: 150,
        cpu_usage_percent: Some(45.5),
        memory_used_bytes: Some(256 * 1024 * 1024), // 256MB в байтах
        io_operations: Some(100),
        cache_hits: Some(80),
        cache_misses: Some(20),
    };

    let entry = StructuredLogEntry {
        timestamp: Utc::now().to_rfc3339(),
        level: "INFO".to_string(),
        target: "test::module".to_string(),
        message: "Test log message".to_string(),
        fields,
        context: Some(context),
        performance: Some(performance),
    };

    // Проверяем сериализацию
    let json_str = serde_json::to_string(&entry).unwrap();
    assert!(json_str.contains("test-component"));
    assert!(json_str.contains("req-123"));
    assert!(json_str.contains("150"));

    // Проверяем десериализацию
    let deserialized: StructuredLogEntry = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.level, "INFO");
    assert_eq!(deserialized.message, "Test log message");
}

#[test]
fn test_structured_log_entry_minimal() {
    let entry = StructuredLogEntry {
        timestamp: Utc::now().to_rfc3339(),
        level: "ERROR".to_string(),
        target: "app".to_string(),
        message: "Error occurred".to_string(),
        fields: HashMap::new(),
        context: None,
        performance: None,
    };

    let json_str = serde_json::to_string(&entry).unwrap();
    assert!(json_str.contains("ERROR"));
    assert!(json_str.contains("Error occurred"));
    assert!(!json_str.contains("context"));
    assert!(!json_str.contains("performance"));
}

// Тесты для ExecutionContext
#[test]
fn test_execution_context_complete() {
    let context = ExecutionContext {
        request_id: Some("unique-request-id".to_string()),
        user_id: Some("user123".to_string()),
        app_version: "2.1.0".to_string(),
        hostname: "prod-server-01".to_string(),
        pid: 54321,
        thread_id: "main-thread".to_string(),
    };

    let json = serde_json::to_value(&context).unwrap();
    assert_eq!(json["request_id"], "unique-request-id");
    assert_eq!(json["user_id"], "user123");
    assert_eq!(json["app_version"], "2.1.0");
    assert_eq!(json["hostname"], "prod-server-01");
    assert_eq!(json["pid"], 54321);
    assert_eq!(json["thread_id"], "main-thread");
}

#[test]
fn test_execution_context_partial() {
    let context = ExecutionContext {
        request_id: None,
        user_id: None,
        app_version: "1.0.0".to_string(),
        hostname: "localhost".to_string(),
        pid: 1000,
        thread_id: "thread-1".to_string(),
    };

    let json = serde_json::to_value(&context).unwrap();
    assert_eq!(json["request_id"], Value::Null);
    assert_eq!(json["user_id"], Value::Null);
}

// Тесты для PerformanceMetrics
#[test]
fn test_performance_metrics_all_fields() {
    let metrics = PerformanceMetrics {
        duration_ms: 250,
        cpu_usage_percent: Some(75.5),
        memory_used_bytes: Some(512 * 1024 * 1024), // 512MB в байтах
        io_operations: Some(200),
        cache_hits: Some(150),
        cache_misses: Some(50),
    };

    let json = serde_json::to_value(&metrics).unwrap();
    assert_eq!(json["duration_ms"], 250);
    assert_eq!(json["cpu_usage_percent"], 75.5);
    assert_eq!(json["memory_used_bytes"], 512 * 1024 * 1024);
    assert_eq!(json["io_operations"], 200);
    assert_eq!(json["cache_hits"], 150);
    assert_eq!(json["cache_misses"], 50);
}

#[test]
fn test_performance_metrics_partial() {
    let metrics = PerformanceMetrics {
        duration_ms: 100,
        cpu_usage_percent: None,
        memory_used_bytes: None,
        io_operations: None,
        cache_hits: None,
        cache_misses: None,
    };

    let json_str = serde_json::to_string(&metrics).unwrap();
    assert!(json_str.contains("100"));
    assert!(!json_str.contains("cpu_usage_percent"));
}

// Тесты для RequestContext
#[test]
fn test_request_context_complete() {
    let mut context = RequestContext::new("req-789");
    context = context.with_user("user123");
    context = context.with_metadata("source", "api");
    context = context.with_metadata("version", "v1");

    assert_eq!(context.request_id(), "req-789");
    assert_eq!(context.user_id(), Some("user123"));
    assert_eq!(context.metadata().get("source"), Some(&"api".to_string()));
    assert_eq!(context.metadata().get("version"), Some(&"v1".to_string()));
}

#[test]
fn test_request_context_minimal() {
    let context = RequestContext::new("test-request");

    assert_eq!(context.request_id(), "test-request");
    assert_eq!(context.user_id(), None);
    assert!(context.metadata().is_empty());
}

// Тесты для LoggingConfig
#[test]
fn test_logging_config_default() {
    let config = LoggingConfig::default();
    assert_eq!(config.level(), "info");
    assert!(!config.json_output());
}

#[test]
fn test_logging_config_builder() {
    let config = LoggingConfig::new()
        .with_level("debug")
        .with_json_output(true)
        .with_pretty_print(true);

    assert_eq!(config.level(), "debug");
    assert!(config.json_output());
}

// Тесты для OperationTimer
#[test]
fn test_operation_timer_basic() {
    let mut timer = OperationTimer::new("test_operation");
    timer.add_field("user_id", "12345");
    timer.add_field("items_count", 100);

    std::thread::sleep(std::time::Duration::from_millis(10));

    let metrics = timer.finish();
    assert!(metrics.duration_ms >= 10);
    assert!(metrics.memory_used_bytes.is_none());
    assert!(metrics.cpu_usage_percent.is_none());
}

#[test]
fn test_operation_timer_with_callback() {
    let timer = OperationTimer::new("callback_test");

    let result = timer.finish_with(|metrics| {
        assert!(metrics.duration_ms >= 0);
        "operation_completed"
    });

    assert_eq!(result, "operation_completed");
}

// Тесты для вспомогательных функций
#[test]
fn test_flatten_fields() {
    let mut fields = HashMap::new();
    fields.insert("key1".to_string(), json!("value1"));
    fields.insert("key2".to_string(), json!(42));
    fields.insert(
        "nested".to_string(),
        json!({
            "inner": "value"
        }),
    );

    let entry = StructuredLogEntry {
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        level: "INFO".to_string(),
        target: "test".to_string(),
        message: "Test".to_string(),
        fields,
        context: None,
        performance: None,
    };

    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["key1"], "value1");
    assert_eq!(json["key2"], 42);
}

// Тесты для обработки ошибок
#[test]
fn test_error_serialization() {
    let mut fields = HashMap::new();
    fields.insert("error_type".to_string(), json!("ValidationError"));
    fields.insert("error_code".to_string(), json!(400));
    fields.insert(
        "error_details".to_string(),
        json!({
            "field": "email",
            "reason": "Invalid format"
        }),
    );

    let entry = StructuredLogEntry {
        timestamp: Utc::now().to_rfc3339(),
        level: "ERROR".to_string(),
        target: "validation".to_string(),
        message: "Validation failed".to_string(),
        fields,
        context: None,
        performance: None,
    };

    let json_str = serde_json::to_string(&entry).unwrap();
    assert!(json_str.contains("ValidationError"));
    assert!(json_str.contains("400"));
    assert!(json_str.contains("Invalid format"));
}

// Тесты для edge cases
#[test]
fn test_empty_fields_map() {
    let entry = StructuredLogEntry {
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        level: "DEBUG".to_string(),
        target: "test".to_string(),
        message: "Empty fields test".to_string(),
        fields: HashMap::new(),
        context: None,
        performance: None,
    };

    let json = serde_json::to_value(&entry).unwrap();
    assert_eq!(json["message"], "Empty fields test");
    // Поскольку fields помечен как #[serde(flatten)], пустой HashMap не создаёт отдельное поле
}

#[test]
fn test_special_characters_in_fields() {
    let mut fields = HashMap::new();
    fields.insert("special\"key".to_string(), json!("value with \"quotes\""));
    fields.insert("unicode_🚀".to_string(), json!("emoji value 😊"));
    fields.insert("newline\nkey".to_string(), json!("line1\nline2"));

    let entry = StructuredLogEntry {
        timestamp: "2024-01-01T00:00:00Z".to_string(),
        level: "INFO".to_string(),
        target: "test".to_string(),
        message: "Special chars test".to_string(),
        fields,
        context: None,
        performance: None,
    };

    // Должно корректно сериализоваться
    let json_str = serde_json::to_string(&entry).unwrap();
    assert!(json_str.contains("emoji value"));

    // И десериализоваться обратно
    let deserialized: StructuredLogEntry = serde_json::from_str(&json_str).unwrap();
    assert!(deserialized.fields.contains_key("unicode_🚀"));
}

#[test]
fn test_large_performance_metrics() {
    let metrics = PerformanceMetrics {
        duration_ms: u64::MAX,
        cpu_usage_percent: Some(100.0), // f32::MAX слишком большое для процентов
        memory_used_bytes: Some(u64::MAX),
        io_operations: Some(u64::MAX),
        cache_hits: Some(u64::MAX),
        cache_misses: Some(u64::MAX),
    };

    // Должно корректно сериализоваться даже с максимальными значениями
    let json_str = serde_json::to_string(&metrics).unwrap();
    assert!(json_str.len() > 0);

    let deserialized: PerformanceMetrics = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.duration_ms, u64::MAX);
}

// Дополнительные тесты для init_structured_logging
#[test]
fn test_init_structured_logging_json() {
    let config = LoggingConfig::new()
        .with_json_output(true)
        .with_level("debug");

    // Не должно паниковать
    let _ = init_structured_logging_with_config(config);
}

#[test]
fn test_init_structured_logging_human_readable() {
    let config = LoggingConfig::new()
        .with_json_output(false)
        .with_level("info");

    // Не должно паниковать
    let _ = init_structured_logging_with_config(config);
}
