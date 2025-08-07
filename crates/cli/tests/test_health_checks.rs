use chrono::Utc;
use cli::health_checks::{HealthCheckResult, HealthCheckSystem, HealthStatus};
use std::collections::HashMap;

#[test]
fn test_health_status_display() {
    use std::fmt::Write;

    let healthy = HealthStatus::Healthy;
    let degraded = HealthStatus::Degraded;
    let unhealthy = HealthStatus::Unhealthy;

    // Test that display formatting works (colors might not be visible in tests)
    let healthy_str = format!("{}", healthy);
    let degraded_str = format!("{}", degraded);
    let unhealthy_str = format!("{}", unhealthy);

    assert!(!healthy_str.is_empty());
    assert!(!degraded_str.is_empty());
    assert!(!unhealthy_str.is_empty());
}

#[test]
fn test_health_check_result_creation() {
    let mut metadata = HashMap::new();
    metadata.insert("cpu_usage".to_string(), serde_json::json!("45%"));
    metadata.insert("memory_usage".to_string(), serde_json::json!("2.1GB"));

    let result = HealthCheckResult {
        component: "test_component".to_string(),
        status: HealthStatus::Healthy,
        message: "All systems operational".to_string(),
        latency_ms: 150,
        metadata,
        timestamp: Utc::now(),
    };

    assert_eq!(result.component, "test_component");
    assert_eq!(result.status, HealthStatus::Healthy);
    assert_eq!(result.message, "All systems operational");
    assert_eq!(result.latency_ms, 150);
    assert_eq!(result.metadata.len(), 2);
    assert!(result.timestamp.timestamp() > 0);
}

#[test]
fn test_health_status_equality() {
    assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
    assert_eq!(HealthStatus::Degraded, HealthStatus::Degraded);
    assert_eq!(HealthStatus::Unhealthy, HealthStatus::Unhealthy);

    assert_ne!(HealthStatus::Healthy, HealthStatus::Degraded);
    assert_ne!(HealthStatus::Degraded, HealthStatus::Unhealthy);
    assert_ne!(HealthStatus::Healthy, HealthStatus::Unhealthy);
}

#[test]
fn test_health_check_system_creation() {
    let system = HealthCheckSystem::new();

    // System should be created successfully
    // Internal structure is not directly testable, but creation should not panic
}

#[test]
fn test_health_check_result_serialization() {
    let mut metadata = HashMap::new();
    metadata.insert("test_key".to_string(), serde_json::json!("test_value"));

    let result = HealthCheckResult {
        component: "serialization_test".to_string(),
        status: HealthStatus::Degraded,
        message: "Test serialization".to_string(),
        latency_ms: 250,
        metadata,
        timestamp: Utc::now(),
    };

    // Test serialization
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("serialization_test"));
    assert!(json.contains("Test serialization"));
    assert!(json.contains("Degraded"));

    // Test deserialization
    let deserialized: HealthCheckResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.component, result.component);
    assert_eq!(deserialized.status, result.status);
    assert_eq!(deserialized.message, result.message);
    assert_eq!(deserialized.latency_ms, result.latency_ms);
}

#[test]
fn test_health_status_variants() {
    // Test all variants exist and are distinct
    let statuses = [
        HealthStatus::Healthy,
        HealthStatus::Degraded,
        HealthStatus::Unhealthy,
    ];

    // All should be distinct
    for (i, status1) in statuses.iter().enumerate() {
        for (j, status2) in statuses.iter().enumerate() {
            if i == j {
                assert_eq!(status1, status2);
            } else {
                assert_ne!(status1, status2);
            }
        }
    }
}

#[test]
fn test_health_check_result_debug() {
    let result = HealthCheckResult {
        component: "debug_test".to_string(),
        status: HealthStatus::Healthy,
        message: "Debug formatting test".to_string(),
        latency_ms: 100,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    let debug_str = format!("{:?}", result);
    assert!(debug_str.contains("debug_test"));
    assert!(debug_str.contains("Healthy"));
    assert!(debug_str.contains("Debug formatting test"));
}

#[test]
fn test_health_check_result_clone() {
    let original = HealthCheckResult {
        component: "clone_test".to_string(),
        status: HealthStatus::Unhealthy,
        message: "Clone test".to_string(),
        latency_ms: 500,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    let cloned = original.clone();

    assert_eq!(original.component, cloned.component);
    assert_eq!(original.status, cloned.status);
    assert_eq!(original.message, cloned.message);
    assert_eq!(original.latency_ms, cloned.latency_ms);
    assert_eq!(original.timestamp, cloned.timestamp);
}

#[test]
fn test_health_check_result_with_complex_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("simple_string".to_string(), serde_json::json!("value"));
    metadata.insert("number".to_string(), serde_json::json!(42));
    metadata.insert("boolean".to_string(), serde_json::json!(true));
    metadata.insert("array".to_string(), serde_json::json!([1, 2, 3]));
    metadata.insert("object".to_string(), serde_json::json!({"nested": "value"}));

    let result = HealthCheckResult {
        component: "complex_metadata_test".to_string(),
        status: HealthStatus::Degraded,
        message: "Testing complex metadata".to_string(),
        latency_ms: 300,
        metadata,
        timestamp: Utc::now(),
    };

    assert_eq!(result.metadata.len(), 5);
    assert_eq!(
        result.metadata.get("simple_string"),
        Some(&serde_json::json!("value"))
    );
    assert_eq!(result.metadata.get("number"), Some(&serde_json::json!(42)));
    assert_eq!(
        result.metadata.get("boolean"),
        Some(&serde_json::json!(true))
    );
}

#[test]
fn test_health_status_serde() {
    // Test serialization of health status
    let healthy_json = serde_json::to_string(&HealthStatus::Healthy).unwrap();
    let degraded_json = serde_json::to_string(&HealthStatus::Degraded).unwrap();
    let unhealthy_json = serde_json::to_string(&HealthStatus::Unhealthy).unwrap();

    assert_eq!(healthy_json, "\"Healthy\"");
    assert_eq!(degraded_json, "\"Degraded\"");
    assert_eq!(unhealthy_json, "\"Unhealthy\"");

    // Test deserialization
    let healthy: HealthStatus = serde_json::from_str("\"Healthy\"").unwrap();
    let degraded: HealthStatus = serde_json::from_str("\"Degraded\"").unwrap();
    let unhealthy: HealthStatus = serde_json::from_str("\"Unhealthy\"").unwrap();

    assert_eq!(healthy, HealthStatus::Healthy);
    assert_eq!(degraded, HealthStatus::Degraded);
    assert_eq!(unhealthy, HealthStatus::Unhealthy);
}

#[test]
fn test_health_check_result_empty_metadata() {
    let result = HealthCheckResult {
        component: "empty_metadata_test".to_string(),
        status: HealthStatus::Healthy,
        message: "No metadata".to_string(),
        latency_ms: 50,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    assert!(result.metadata.is_empty());

    // Should still serialize/deserialize correctly
    let json = serde_json::to_string(&result).unwrap();
    let deserialized: HealthCheckResult = serde_json::from_str(&json).unwrap();
    assert!(deserialized.metadata.is_empty());
}

#[test]
fn test_health_check_result_zero_latency() {
    let result = HealthCheckResult {
        component: "zero_latency_test".to_string(),
        status: HealthStatus::Healthy,
        message: "Instant response".to_string(),
        latency_ms: 0,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    assert_eq!(result.latency_ms, 0);
}

#[test]
fn test_health_check_result_high_latency() {
    let result = HealthCheckResult {
        component: "high_latency_test".to_string(),
        status: HealthStatus::Degraded,
        message: "Slow response".to_string(),
        latency_ms: 5000,
        metadata: HashMap::new(),
        timestamp: Utc::now(),
    };

    assert_eq!(result.latency_ms, 5000);
    // High latency might indicate degraded performance
    assert_eq!(result.status, HealthStatus::Degraded);
}
