use memory::health::*;
use memory::types::*;
use std::collections::HashMap;
use chrono::{Utc, Duration as ChronoDuration};

#[test]
fn test_health_monitor_creation() {
    let config = HealthConfig::default();
    let monitor = HealthMonitor::new(config);
    
    assert!(monitor.is_healthy());
    assert_eq!(monitor.get_status(), HealthStatus::Healthy);
    assert_eq!(monitor.get_active_checks(), 0);
}

#[test]
fn test_health_config_validation() {
    let mut config = HealthConfig::default();
    
    // Valid config
    assert!(config.validate().is_ok());
    
    // Invalid check interval
    config.check_interval_seconds = 0;
    assert!(config.validate().is_err());
    
    // Reset and test invalid memory threshold
    config = HealthConfig::default();
    config.memory_threshold_percent = 150; // Over 100%
    assert!(config.validate().is_err());
    
    config.memory_threshold_percent = 0; // 0%
    assert!(config.validate().is_err());
    
    // Reset and test invalid response time threshold
    config = HealthConfig::default();
    config.max_response_time_ms = 0;
    assert!(config.validate().is_err());
}

#[tokio::test]
async fn test_health_check_registration() {
    let mut monitor = HealthMonitor::new(HealthConfig::default());
    
    let check_id = monitor.register_check(
        "memory_usage",
        CheckType::Memory,
        Duration::from_secs(30),
        Box::new(|_| Box::pin(async {
            Ok(CheckResult {
                status: HealthStatus::Healthy,
                message: "Memory usage normal".to_string(),
                metrics: HashMap::new(),
                timestamp: Utc::now(),
            })
        }))
    );
    
    assert!(!check_id.is_empty());
    assert_eq!(monitor.get_active_checks(), 1);
    
    let success = monitor.unregister_check(&check_id);
    assert!(success);
    assert_eq!(monitor.get_active_checks(), 0);
}

#[tokio::test]
async fn test_health_check_execution() {
    let mut monitor = HealthMonitor::new(HealthConfig::default());
    
    let check_id = monitor.register_check(
        "test_check",
        CheckType::Custom,
        Duration::from_secs(5),
        Box::new(|_| Box::pin(async {
            Ok(CheckResult {
                status: HealthStatus::Healthy,
                message: "Test check passed".to_string(),
                metrics: {
                    let mut metrics = HashMap::new();
                    metrics.insert("test_metric".to_string(), 42.0);
                    metrics
                },
                timestamp: Utc::now(),
            })
        }))
    );
    
    let result = monitor.run_check(&check_id).await;
    assert!(result.is_ok());
    
    let check_result = result.unwrap();
    assert_eq!(check_result.status, HealthStatus::Healthy);
    assert!(check_result.message.contains("Test check passed"));
    assert_eq!(check_result.metrics.get("test_metric"), Some(&42.0));
}

#[tokio::test]
async fn test_health_check_failure() {
    let mut monitor = HealthMonitor::new(HealthConfig::default());
    
    let check_id = monitor.register_check(
        "failing_check",
        CheckType::Custom,
        Duration::from_secs(5),
        Box::new(|_| Box::pin(async {
            Ok(CheckResult {
                status: HealthStatus::Unhealthy,
                message: "Check failed deliberately".to_string(),
                metrics: HashMap::new(),
                timestamp: Utc::now(),
            })
        }))
    );
    
    let result = monitor.run_check(&check_id).await;
    assert!(result.is_ok());
    
    let check_result = result.unwrap();
    assert_eq!(check_result.status, HealthStatus::Unhealthy);
    assert!(check_result.message.contains("failed"));
    
    // Overall monitor status should be affected
    let overall_status = monitor.get_detailed_status().await;
    assert!(overall_status.failed_checks > 0);
}

#[tokio::test]
async fn test_health_check_timeout() {
    let config = HealthConfig {
        check_interval_seconds: 10,
        memory_threshold_percent: 80,
        max_response_time_ms: 100, // Very short timeout
        alert_on_failure: true,
        persist_results: false,
    };
    
    let mut monitor = HealthMonitor::new(config);
    
    let check_id = monitor.register_check(
        "slow_check",
        CheckType::Performance,
        Duration::from_secs(5),
        Box::new(|_| Box::pin(async {
            // Simulate slow operation
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok(CheckResult {
                status: HealthStatus::Healthy,
                message: "Slow check completed".to_string(),
                metrics: HashMap::new(),
                timestamp: Utc::now(),
            })
        }))
    );
    
    let result = monitor.run_check_with_timeout(&check_id, Duration::from_millis(50)).await;
    // Should timeout
    assert!(result.is_err() || result.unwrap().status == HealthStatus::Unhealthy);
}

#[tokio::test]
async fn test_memory_health_check() {
    let monitor = HealthMonitor::new(HealthConfig::default());
    
    let memory_check = monitor.create_memory_check(80); // 80% threshold
    let result = memory_check.execute().await;
    
    assert!(result.is_ok());
    let check_result = result.unwrap();
    
    assert!(matches!(check_result.status, HealthStatus::Healthy | HealthStatus::Warning | HealthStatus::Unhealthy));
    assert!(check_result.message.contains("memory") || check_result.message.contains("Memory"));
    assert!(check_result.metrics.contains_key("memory_usage_percent"));
}

#[tokio::test]
async fn test_performance_health_check() {
    let monitor = HealthMonitor::new(HealthConfig::default());
    
    let perf_check = monitor.create_performance_check();
    let result = perf_check.execute().await;
    
    assert!(result.is_ok());
    let check_result = result.unwrap();
    
    assert!(matches!(check_result.status, HealthStatus::Healthy | HealthStatus::Warning));
    assert!(check_result.metrics.contains_key("response_time_ms"));
    assert!(check_result.metrics.contains_key("cpu_usage_percent"));
}

#[tokio::test]
async fn test_storage_health_check() {
    let monitor = HealthMonitor::new(HealthConfig::default());
    
    let storage_check = monitor.create_storage_check();
    let result = storage_check.execute().await;
    
    assert!(result.is_ok());
    let check_result = result.unwrap();
    
    assert!(matches!(check_result.status, HealthStatus::Healthy | HealthStatus::Warning | HealthStatus::Unhealthy));
    assert!(check_result.metrics.contains_key("disk_usage_percent"));
    assert!(check_result.metrics.contains_key("available_space_mb"));
}

#[tokio::test]
async fn test_health_check_history() {
    let mut monitor = HealthMonitor::new(HealthConfig {
        persist_results: true,
        ..Default::default()
    });
    
    let check_id = monitor.register_check(
        "history_test",
        CheckType::Custom,
        Duration::from_secs(5),
        Box::new(|_| Box::pin(async {
            Ok(CheckResult {
                status: HealthStatus::Healthy,
                message: "History test".to_string(),
                metrics: HashMap::new(),
                timestamp: Utc::now(),
            })
        }))
    );
    
    // Run check multiple times
    for _ in 0..5 {
        monitor.run_check(&check_id).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    let history = monitor.get_check_history(&check_id);
    assert!(history.len() >= 3); // Should have some history
    
    // Results should be chronologically ordered
    for i in 1..history.len() {
        assert!(history[i].timestamp >= history[i-1].timestamp);
    }
}

#[tokio::test]
async fn test_health_alerts() {
    let mut monitor = HealthMonitor::new(HealthConfig {
        alert_on_failure: true,
        ..Default::default()
    });
    
    let check_id = monitor.register_check(
        "alert_test",
        CheckType::Custom,
        Duration::from_secs(5),
        Box::new(|_| Box::pin(async {
            Ok(CheckResult {
                status: HealthStatus::Unhealthy,
                message: "Alert test failure".to_string(),
                metrics: HashMap::new(),
                timestamp: Utc::now(),
            })
        }))
    );
    
    monitor.run_check(&check_id).await.unwrap();
    
    let alerts = monitor.get_pending_alerts();
    assert!(!alerts.is_empty());
    
    let alert = &alerts[0];
    assert_eq!(alert.severity, AlertSeverity::High);
    assert!(alert.message.contains("Alert test failure"));
    assert_eq!(alert.check_name, "alert_test");
}

#[tokio::test]
async fn test_health_metrics_aggregation() {
    let mut monitor = HealthMonitor::new(HealthConfig::default());
    
    // Register multiple checks
    for i in 0..3 {
        let check_name = format!("metrics_test_{}", i);
        let metric_value = (i + 1) as f64 * 10.0;
        
        monitor.register_check(
            &check_name,
            CheckType::Performance,
            Duration::from_secs(5),
            Box::new(move |_| {
                let value = metric_value;
                Box::pin(async move {
                    let mut metrics = HashMap::new();
                    metrics.insert("response_time".to_string(), value);
                    metrics.insert("throughput".to_string(), value * 2.0);
                    
                    Ok(CheckResult {
                        status: HealthStatus::Healthy,
                        message: "Metrics test".to_string(),
                        metrics,
                        timestamp: Utc::now(),
                    })
                })
            })
        );
    }
    
    // Run all checks
    monitor.run_all_checks().await;
    
    let aggregated_metrics = monitor.get_aggregated_metrics();
    assert!(aggregated_metrics.contains_key("response_time"));
    assert!(aggregated_metrics.contains_key("throughput"));
    
    // Should have aggregated values from all checks
    assert!(aggregated_metrics["response_time"] > 0.0);
    assert!(aggregated_metrics["throughput"] > 0.0);
}

#[tokio::test]
async fn test_health_check_dependencies() {
    let mut monitor = HealthMonitor::new(HealthConfig::default());
    
    // Register primary check
    let primary_id = monitor.register_check(
        "primary_check",
        CheckType::Storage,
        Duration::from_secs(5),
        Box::new(|_| Box::pin(async {
            Ok(CheckResult {
                status: HealthStatus::Healthy,
                message: "Primary check OK".to_string(),
                metrics: HashMap::new(),
                timestamp: Utc::now(),
            })
        }))
    );
    
    // Register dependent check
    let dependent_id = monitor.register_dependent_check(
        "dependent_check",
        CheckType::Custom,
        Duration::from_secs(5),
        vec![primary_id.clone()],
        Box::new(|deps| Box::pin(async move {
            let primary_healthy = deps.iter().all(|result| result.status == HealthStatus::Healthy);
            
            Ok(CheckResult {
                status: if primary_healthy { HealthStatus::Healthy } else { HealthStatus::Unhealthy },
                message: if primary_healthy { "Dependent check OK".to_string() } else { "Dependency failed".to_string() },
                metrics: HashMap::new(),
                timestamp: Utc::now(),
            })
        }))
    );
    
    // Run checks with dependency resolution
    monitor.run_checks_with_dependencies().await;
    
    let dependent_result = monitor.get_last_check_result(&dependent_id);
    assert!(dependent_result.is_some());
    assert_eq!(dependent_result.unwrap().status, HealthStatus::Healthy);
}

#[test]
fn test_health_status_ordering() {
    assert!(HealthStatus::Healthy > HealthStatus::Warning);
    assert!(HealthStatus::Warning > HealthStatus::Unhealthy);
    assert!(HealthStatus::Unhealthy > HealthStatus::Critical);
    
    // Test equality
    assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
    assert_eq!(HealthStatus::Warning, HealthStatus::Warning);
    assert_eq!(HealthStatus::Unhealthy, HealthStatus::Unhealthy);
    assert_eq!(HealthStatus::Critical, HealthStatus::Critical);
}

#[test]
fn test_check_type_categories() {
    let performance_types = vec![CheckType::Performance, CheckType::Memory];
    let system_types = vec![CheckType::Storage, CheckType::Network];
    let custom_types = vec![CheckType::Custom];
    
    for check_type in performance_types {
        assert!(check_type.is_performance_related());
    }
    
    for check_type in system_types {
        assert!(check_type.is_system_related());
    }
    
    for check_type in custom_types {
        assert!(check_type.is_custom());
    }
}

#[tokio::test]
async fn test_health_check_circuit_breaker() {
    let config = HealthConfig {
        check_interval_seconds: 1,
        memory_threshold_percent: 80,
        max_response_time_ms: 1000,
        alert_on_failure: true,
        persist_results: true,
    };
    
    let mut monitor = HealthMonitor::new(config);
    
    // Register a check that fails consistently
    let mut failure_count = 0;
    let check_id = monitor.register_check(
        "circuit_breaker_test",
        CheckType::Custom,
        Duration::from_secs(1),
        Box::new(move |_| {
            failure_count += 1;
            Box::pin(async move {
                if failure_count < 5 {
                    Ok(CheckResult {
                        status: HealthStatus::Unhealthy,
                        message: format!("Failure #{}", failure_count),
                        metrics: HashMap::new(),
                        timestamp: Utc::now(),
                    })
                } else {
                    // Recover after 5 failures
                    Ok(CheckResult {
                        status: HealthStatus::Healthy,
                        message: "Recovered".to_string(),
                        metrics: HashMap::new(),
                        timestamp: Utc::now(),
                    })
                }
            })
        })
    );
    
    // Run check multiple times to trigger circuit breaker
    for _ in 0..10 {
        monitor.run_check(&check_id).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    let circuit_state = monitor.get_circuit_breaker_state(&check_id);
    assert!(circuit_state.is_some());
    
    // Should eventually recover
    let final_result = monitor.run_check(&check_id).await.unwrap();
    assert!(matches!(final_result.status, HealthStatus::Healthy | HealthStatus::Warning));
}

#[tokio::test]
async fn test_health_dashboard_data() {
    let mut monitor = HealthMonitor::new(HealthConfig::default());
    
    // Register various types of checks
    let checks = vec![
        ("memory_check", CheckType::Memory),
        ("storage_check", CheckType::Storage),
        ("performance_check", CheckType::Performance),
        ("network_check", CheckType::Network),
    ];
    
    for (name, check_type) in checks {
        monitor.register_check(
            name,
            check_type,
            Duration::from_secs(5),
            Box::new(|_| Box::pin(async {
                Ok(CheckResult {
                    status: HealthStatus::Healthy,
                    message: "Dashboard test".to_string(),
                    metrics: {
                        let mut metrics = HashMap::new();
                        metrics.insert("uptime_seconds".to_string(), 3600.0);
                        metrics.insert("requests_per_second".to_string(), 150.0);
                        metrics
                    },
                    timestamp: Utc::now(),
                })
            }))
        );
    }
    
    monitor.run_all_checks().await;
    
    let dashboard_data = monitor.get_dashboard_data();
    assert_eq!(dashboard_data.total_checks, 4);
    assert_eq!(dashboard_data.healthy_checks, 4);
    assert_eq!(dashboard_data.failed_checks, 0);
    assert!(dashboard_data.uptime_seconds > 0.0);
    assert!(!dashboard_data.recent_metrics.is_empty());
}

#[tokio::test]
async fn test_health_check_concurrent_execution() {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    let monitor = Arc::new(Mutex::new(HealthMonitor::new(HealthConfig::default())));
    let mut handles = vec![];
    
    // Register multiple checks concurrently
    for i in 0..10 {
        let monitor_clone = Arc::clone(&monitor);
        let handle = tokio::spawn(async move {
            let mut mon = monitor_clone.lock().await;
            let check_name = format!("concurrent_check_{}", i);
            
            let check_id = mon.register_check(
                &check_name,
                CheckType::Custom,
                Duration::from_secs(1),
                Box::new(move |_| {
                    let thread_id = i;
                    Box::pin(async move {
                        // Simulate some work
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        
                        Ok(CheckResult {
                            status: HealthStatus::Healthy,
                            message: format!("Concurrent check {} completed", thread_id),
                            metrics: HashMap::new(),
                            timestamp: Utc::now(),
                        })
                    })
                })
            );
            
            // Run the check
            mon.run_check(&check_id).await.unwrap();
            check_id
        });
        handles.push(handle);
    }
    
    let mut check_ids = vec![];
    for handle in handles {
        let check_id = handle.await.unwrap();
        check_ids.push(check_id);
    }
    
    // Verify all checks were registered and executed
    let final_monitor = monitor.lock().await;
    assert_eq!(final_monitor.get_active_checks(), 10);
    
    for check_id in check_ids {
        let result = final_monitor.get_last_check_result(&check_id);
        assert!(result.is_some());
        assert_eq!(result.unwrap().status, HealthStatus::Healthy);
    }
}

#[test]
fn test_health_check_result_serialization() {
    let mut metrics = HashMap::new();
    metrics.insert("cpu_usage".to_string(), 65.5);
    metrics.insert("memory_usage".to_string(), 78.2);
    
    let result = CheckResult {
        status: HealthStatus::Warning,
        message: "System resources under pressure".to_string(),
        metrics,
        timestamp: Utc::now(),
    };
    
    // Test JSON serialization
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("Warning"));
    assert!(json.contains("System resources"));
    assert!(json.contains("cpu_usage"));
    assert!(json.contains("65.5"));
    
    // Test deserialization
    let deserialized: CheckResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.status, HealthStatus::Warning);
    assert_eq!(deserialized.message, result.message);
    assert_eq!(deserialized.metrics.get("cpu_usage"), Some(&65.5));
    assert_eq!(deserialized.metrics.get("memory_usage"), Some(&78.2));
}