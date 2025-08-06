//! Comprehensive unit тесты для ResilienceService
//!
//! Coverage areas:
//! - Circuit breaker pattern implementation
//! - Error handling и failure tracking
//! - Recovery timeout mechanics
//! - Production metrics integration
//! - Concurrent failure scenarios
//! - Property-based testing для resilience patterns

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use tokio_test;
use proptest::prelude::*;
use once_cell::sync::Lazy;

use memory::{
    services::{ResilienceService, traits::{ResilienceServiceTrait, ProductionMetrics}},
};

static INIT_TRACING: Lazy<()> = Lazy::new(|| {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();
});

#[tokio::test]
async fn test_resilience_service_creation() -> Result<()> {
    Lazy::force(&INIT_TRACING);
    
    // Test default creation
    let service = ResilienceService::new();
    assert!(!service.get_circuit_breaker_status().await, "Circuit breaker должен быть изначально закрыт");
    
    // Test custom creation
    let custom_service = ResilienceService::new_with_threshold(3, Duration::from_secs(30));
    assert!(!custom_service.get_circuit_breaker_status().await, "Custom circuit breaker должен быть изначально закрыт");
    
    // Test production creation
    let production_service = ResilienceService::new_production();
    assert!(!production_service.get_circuit_breaker_status().await, "Production circuit breaker должен быть изначально закрыт");
    
    // Test test creation
    let test_service = ResilienceService::new_for_tests();
    assert!(!test_service.get_circuit_breaker_status().await, "Test circuit breaker должен быть изначально закрыт");
    
    Ok(())
}

#[tokio::test]
async fn test_circuit_breaker_basic_functionality() -> Result<()> {
    let service = ResilienceService::new_with_threshold(3, Duration::from_secs(1));
    
    // Initially closed
    assert!(service.check_circuit_breaker().await.is_ok(), "Circuit breaker должен быть изначально доступен");
    assert!(!service.get_circuit_breaker_status().await, "Circuit breaker должен быть изначально закрыт");
    
    // Record successful operations
    service.record_successful_operation(Duration::from_millis(100)).await;
    assert!(service.check_circuit_breaker().await.is_ok(), "Circuit breaker должен оставаться доступным после успешной операции");
    
    Ok(())
}

#[tokio::test]
async fn test_circuit_breaker_failure_threshold() -> Result<()> {
    let service = ResilienceService::new_with_threshold(3, Duration::from_secs(1));
    
    // Record failures up to threshold
    service.record_failed_operation(Duration::from_millis(100)).await;
    assert!(!service.get_circuit_breaker_status().await, "Circuit breaker должен оставаться закрытым после 1 ошибки");
    
    service.record_failed_operation(Duration::from_millis(100)).await;
    assert!(!service.get_circuit_breaker_status().await, "Circuit breaker должен оставаться закрытым после 2 ошибок");
    
    service.record_failed_operation(Duration::from_millis(100)).await;
    assert!(service.get_circuit_breaker_status().await, "Circuit breaker должен открыться после 3 ошибок");
    
    // Check should fail when circuit breaker is open
    let check_result = service.check_circuit_breaker().await;
    assert!(check_result.is_err(), "Check должен завершаться с ошибкой когда circuit breaker открыт");
    
    Ok(())
}

#[tokio::test]
async fn test_circuit_breaker_recovery() -> Result<()> {
    let service = ResilienceService::new_with_threshold(2, Duration::from_millis(100)); // Short recovery time for test
    
    // Open circuit breaker
    service.record_failed_operation(Duration::from_millis(50)).await;
    service.record_failed_operation(Duration::from_millis(50)).await;
    assert!(service.get_circuit_breaker_status().await, "Circuit breaker должен быть открыт");
    
    // Wait for recovery timeout
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    // Circuit breaker should recover after timeout
    assert!(service.check_circuit_breaker().await.is_ok(), "Circuit breaker должен восстановиться после timeout");
    assert!(!service.get_circuit_breaker_status().await, "Circuit breaker должен быть закрыт после восстановления");
    
    Ok(())
}

#[tokio::test]
async fn test_circuit_breaker_reset() -> Result<()> {
    let service = ResilienceService::new_with_threshold(2, Duration::from_secs(10)); // Long recovery time
    
    // Open circuit breaker
    service.record_failed_operation(Duration::from_millis(50)).await;
    service.record_failed_operation(Duration::from_millis(50)).await;
    assert!(service.get_circuit_breaker_status().await, "Circuit breaker должен быть открыт");
    
    // Manual reset
    let reset_result = service.reset_circuit_breaker().await;
    assert!(reset_result.is_ok(), "Reset должен завершиться успешно");
    assert!(!service.get_circuit_breaker_status().await, "Circuit breaker должен быть закрыт после reset");
    
    // Should be available immediately
    assert!(service.check_circuit_breaker().await.is_ok(), "Circuit breaker должен быть доступен после reset");
    
    Ok(())
}

#[tokio::test]
async fn test_successful_operation_resets_failure_count() -> Result<()> {
    let service = ResilienceService::new_with_threshold(3, Duration::from_secs(1));
    
    // Record some failures (but not enough to open circuit breaker)
    service.record_failed_operation(Duration::from_millis(50)).await;
    service.record_failed_operation(Duration::from_millis(50)).await;
    
    let (failure_count, _) = service.get_failure_stats().await;
    assert_eq!(failure_count, 2, "Failure count должен быть 2");
    
    // Record successful operation
    service.record_successful_operation(Duration::from_millis(50)).await;
    
    let (failure_count_after, _) = service.get_failure_stats().await;
    assert_eq!(failure_count_after, 0, "Failure count должен сброситься после успешной операции");
    
    assert!(!service.get_circuit_breaker_status().await, "Circuit breaker должен оставаться закрытым");
    
    Ok(())
}

#[tokio::test]
async fn test_set_failure_threshold() -> Result<()> {
    let service = ResilienceService::new_with_threshold(5, Duration::from_secs(1));
    
    // Change threshold to 2
    let result = service.set_failure_threshold(2).await;
    assert!(result.is_ok(), "Set threshold должен завершиться успешно");
    
    // Test new threshold
    service.record_failed_operation(Duration::from_millis(50)).await;
    assert!(!service.get_circuit_breaker_status().await, "Circuit breaker должен оставаться закрытым после 1 ошибки");
    
    service.record_failed_operation(Duration::from_millis(50)).await;
    assert!(service.get_circuit_breaker_status().await, "Circuit breaker должен открыться после 2 ошибок с новым threshold");
    
    Ok(())
}

#[tokio::test]
async fn test_production_metrics_integration() -> Result<()> {
    let service = ResilienceService::new();
    
    // Record operations and verify metrics
    service.record_successful_operation(Duration::from_millis(100)).await;
    service.record_successful_operation(Duration::from_millis(200)).await;
    service.record_failed_operation(Duration::from_millis(50)).await;
    
    let metrics = service.get_production_metrics().await;
    
    assert_eq!(metrics.total_operations, 3, "Total operations должно быть 3");
    assert_eq!(metrics.successful_operations, 2, "Successful operations должно быть 2");
    assert_eq!(metrics.failed_operations, 1, "Failed operations должно быть 1");
    assert!(metrics.avg_response_time_ms > 0.0, "Average response time должно быть больше 0");
    
    Ok(())
}

#[tokio::test]
async fn test_circuit_breaker_trips_metric() -> Result<()> {
    let service = ResilienceService::new_with_threshold(2, Duration::from_secs(1));
    
    // Open circuit breaker
    service.record_failed_operation(Duration::from_millis(50)).await;
    service.record_failed_operation(Duration::from_millis(50)).await;
    
    let metrics = service.get_production_metrics().await;
    assert_eq!(metrics.circuit_breaker_trips, 1, "Circuit breaker trips должно быть 1");
    
    // Reset and trip again
    service.reset_circuit_breaker().await?;
    service.record_failed_operation(Duration::from_millis(50)).await;
    service.record_failed_operation(Duration::from_millis(50)).await;
    
    let metrics_after = service.get_production_metrics().await;
    assert_eq!(metrics_after.circuit_breaker_trips, 2, "Circuit breaker trips должно быть 2 после второго срабатывания");
    
    Ok(())
}

#[tokio::test]
async fn test_resilience_stats() -> Result<()> {
    let service = ResilienceService::new_with_threshold(3, Duration::from_millis(500));
    
    // Record some operations
    service.record_successful_operation(Duration::from_millis(100)).await;
    service.record_failed_operation(Duration::from_millis(200)).await;
    service.record_successful_operation(Duration::from_millis(150)).await;
    
    let stats = service.get_resilience_stats().await;
    
    assert!(!stats.circuit_breaker_open, "Circuit breaker должен быть закрыт");
    assert_eq!(stats.failure_count, 0, "Failure count должен быть 0 после успешной операции");
    assert_eq!(stats.failure_threshold, 3, "Failure threshold должен быть 3");
    assert_eq!(stats.recovery_timeout, Duration::from_millis(500), "Recovery timeout должен совпадать");
    assert_eq!(stats.total_operations, 3, "Total operations должно быть 3");
    assert_eq!(stats.successful_operations, 2, "Successful operations должно быть 2");
    assert_eq!(stats.failed_operations, 1, "Failed operations должно быть 1");
    
    let expected_success_rate = (2.0 / 3.0) * 100.0;
    assert!((stats.success_rate - expected_success_rate).abs() < 0.1, "Success rate должен быть около 66.67%");
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let service = Arc::new(ResilienceService::new_with_threshold(10, Duration::from_secs(1)));
    
    // Test concurrent successful operations
    let success_tasks: Vec<_> = (0..20)
        .map(|i| {
            let service_clone = service.clone();
            tokio::spawn(async move {
                service_clone.record_successful_operation(Duration::from_millis(i * 5)).await;
            })
        })
        .collect();
    
    futures::future::join_all(success_tasks).await;
    
    let metrics = service.get_production_metrics().await;
    assert_eq!(metrics.successful_operations, 20, "Все 20 успешных операций должны быть записаны");
    
    // Test concurrent failed operations
    let failure_tasks: Vec<_> = (0..5)
        .map(|i| {
            let service_clone = service.clone();
            tokio::spawn(async move {
                service_clone.record_failed_operation(Duration::from_millis(i * 5 + 100)).await;
            })
        })
        .collect();
    
    futures::future::join_all(failure_tasks).await;
    
    let final_metrics = service.get_production_metrics().await;
    assert_eq!(final_metrics.total_operations, 25, "Total operations должно быть 25");
    assert_eq!(final_metrics.failed_operations, 5, "Failed operations должно быть 5");
    
    Ok(())
}

#[tokio::test]
async fn test_exponential_moving_average() -> Result<()> {
    let service = ResilienceService::new();
    
    // Record operations with different durations
    service.record_successful_operation(Duration::from_millis(100)).await;
    let metrics1 = service.get_production_metrics().await;
    assert_eq!(metrics1.avg_response_time_ms, 100.0, "First measurement should be exact");
    
    service.record_successful_operation(Duration::from_millis(200)).await;
    let metrics2 = service.get_production_metrics().await;
    
    // With alpha = 0.1, new average = 0.1 * 200 + 0.9 * 100 = 110
    let expected_avg = 0.1 * 200.0 + 0.9 * 100.0;
    assert!((metrics2.avg_response_time_ms - expected_avg).abs() < 0.1, "EMA должен правильно вычисляться");
    
    Ok(())
}

#[tokio::test]
async fn test_edge_cases() -> Result<()> {
    let service = ResilienceService::new();
    
    // Test zero duration
    service.record_successful_operation(Duration::from_millis(0)).await;
    service.record_failed_operation(Duration::from_millis(0)).await;
    
    // Test very large duration
    service.record_successful_operation(Duration::from_secs(60)).await;
    
    let metrics = service.get_production_metrics().await;
    assert_eq!(metrics.total_operations, 3, "Все операции должны быть записаны");
    
    // Test threshold edge cases
    let edge_service = ResilienceService::new_with_threshold(0, Duration::from_millis(1));
    // Circuit breaker with threshold 0 should open immediately
    edge_service.record_failed_operation(Duration::from_millis(1)).await;
    assert!(edge_service.get_circuit_breaker_status().await, "Circuit breaker с threshold 0 должен открываться сразу");
    
    Ok(())
}

// Property-based tests
proptest::proptest! {
    #[test]
    fn test_circuit_breaker_properties(
        threshold in 1u32..20,
        recovery_ms in 1u64..1000,
        num_failures in 1usize..50,
    ) {
        tokio_test::block_on(async {
            let service = ResilienceService::new_with_threshold(threshold, Duration::from_millis(recovery_ms));
            
            // Record failures up to but not exceeding threshold
            let failures_to_record = std::cmp::min(num_failures, threshold as usize);
            
            for _ in 0..failures_to_record {
                service.record_failed_operation(Duration::from_millis(10)).await;
            }
            
            let (failure_count, _) = service.get_failure_stats().await;
            
            if failures_to_record < threshold as usize {
                prop_assert!(!service.get_circuit_breaker_status().await, "Circuit breaker не должен открываться до достижения threshold");
                prop_assert_eq!(failure_count, failures_to_record as u32, "Failure count должен соответствовать количеству ошибок");
            } else {
                prop_assert!(service.get_circuit_breaker_status().await, "Circuit breaker должен открываться при достижении threshold");
            }
        });
    }
    
    #[test]
    fn test_metrics_consistency(
        successful_ops in 0usize..100,
        failed_ops in 0usize..100,
    ) {
        tokio_test::block_on(async {
            let service = ResilienceService::new_with_threshold(1000, Duration::from_secs(1)); // High threshold to avoid circuit breaker
            
            // Record operations
            for _ in 0..successful_ops {
                service.record_successful_operation(Duration::from_millis(50)).await;
            }
            
            for _ in 0..failed_ops {
                service.record_failed_operation(Duration::from_millis(100)).await;
            }
            
            let metrics = service.get_production_metrics().await;
            
            prop_assert_eq!(metrics.successful_operations as usize, successful_ops, "Successful operations должны совпадать");
            prop_assert_eq!(metrics.failed_operations as usize, failed_ops, "Failed operations должны совпадать");
            prop_assert_eq!(metrics.total_operations as usize, successful_ops + failed_ops, "Total operations должно быть суммой");
            
            if metrics.total_operations > 0 {
                let expected_success_rate = (successful_ops as f64 / (successful_ops + failed_ops) as f64) * 100.0;
                prop_assert!((metrics.success_rate - expected_success_rate).abs() < 0.1, "Success rate должен быть правильно вычислен");
            }
        });
    }
    
    #[test]
    fn test_recovery_timeout_property(
        recovery_ms in 1u64..200,
        extra_wait_ms in 0u64..50,
    ) {
        tokio_test::block_on(async {
            let service = ResilienceService::new_with_threshold(1, Duration::from_millis(recovery_ms));
            
            // Open circuit breaker
            service.record_failed_operation(Duration::from_millis(10)).await;
            prop_assert!(service.get_circuit_breaker_status().await, "Circuit breaker должен быть открыт");
            
            // Wait for recovery + extra time
            tokio::time::sleep(Duration::from_millis(recovery_ms + extra_wait_ms)).await;
            
            // Circuit breaker should be recovered
            prop_assert!(service.check_circuit_breaker().await.is_ok(), "Circuit breaker должен восстановиться после timeout");
            prop_assert!(!service.get_circuit_breaker_status().await, "Circuit breaker должен быть закрыт после восстановления");
        });
    }
}

#[tokio::test]
async fn test_circuit_breaker_state_transitions() -> Result<()> {
    let service = ResilienceService::new_with_threshold(2, Duration::from_millis(100));
    
    // State: CLOSED
    assert!(!service.get_circuit_breaker_status().await);
    assert!(service.check_circuit_breaker().await.is_ok());
    
    // CLOSED -> CLOSED (1 failure, below threshold)
    service.record_failed_operation(Duration::from_millis(10)).await;
    assert!(!service.get_circuit_breaker_status().await);
    assert!(service.check_circuit_breaker().await.is_ok());
    
    // CLOSED -> OPEN (2 failures, threshold reached)
    service.record_failed_operation(Duration::from_millis(10)).await;
    assert!(service.get_circuit_breaker_status().await);
    assert!(service.check_circuit_breaker().await.is_err());
    
    // OPEN -> CLOSED (after timeout)
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(service.check_circuit_breaker().await.is_ok());
    assert!(!service.get_circuit_breaker_status().await);
    
    // CLOSED -> CLOSED (successful operation)
    service.record_successful_operation(Duration::from_millis(10)).await;
    assert!(!service.get_circuit_breaker_status().await);
    assert!(service.check_circuit_breaker().await.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_circuit_breaker_operations() -> Result<()> {
    let service = Arc::new(ResilienceService::new_with_threshold(5, Duration::from_millis(100)));
    
    // Concurrent operations that should trip circuit breaker
    let tasks: Vec<_> = (0..10)
        .map(|_| {
            let service_clone = service.clone();
            tokio::spawn(async move {
                service_clone.record_failed_operation(Duration::from_millis(10)).await;
            })
        })
        .collect();
    
    futures::future::join_all(tasks).await;
    
    // Circuit breaker should be open after concurrent failures
    assert!(service.get_circuit_breaker_status().await, "Circuit breaker должен быть открыт после множественных concurrent ошибок");
    
    // Concurrent checks should all fail
    let check_tasks: Vec<_> = (0..5)
        .map(|_| {
            let service_clone = service.clone();
            tokio::spawn(async move {
                service_clone.check_circuit_breaker().await
            })
        })
        .collect();
    
    let check_results = futures::future::join_all(check_tasks).await;
    
    for result in check_results {
        let check_result = result.expect("Task должна завершиться без panic");
        assert!(check_result.is_err(), "Все concurrent checks должны завершаться с ошибкой когда circuit breaker открыт");
    }
    
    Ok(())
}

#[tokio::test]
async fn test_default_service() -> Result<()> {
    let service = ResilienceService::default();
    
    assert!(!service.get_circuit_breaker_status().await, "Default service должен иметь закрытый circuit breaker");
    assert!(service.check_circuit_breaker().await.is_ok(), "Default service должен быть доступен");
    
    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default for performance reasons
async fn stress_test_resilience_service() -> Result<()> {
    let service = Arc::new(ResilienceService::new_with_threshold(100, Duration::from_millis(500)));
    
    // High load stress test
    let tasks: Vec<_> = (0..1000)
        .map(|i| {
            let service_clone = service.clone();
            tokio::spawn(async move {
                if i % 5 == 0 {
                    // 20% failures
                    service_clone.record_failed_operation(Duration::from_millis(i % 100)).await;
                } else {
                    // 80% successes
                    service_clone.record_successful_operation(Duration::from_millis(i % 100)).await;
                }
            })
        })
        .collect();
    
    let start_time = std::time::Instant::now();
    futures::future::join_all(tasks).await;
    let duration = start_time.elapsed();
    
    println!("Stress test completed in {:?}", duration);
    
    let metrics = service.get_production_metrics().await;
    assert_eq!(metrics.total_operations, 1000, "Все операции должны быть обработаны");
    assert_eq!(metrics.successful_operations, 800, "80% операций должны быть успешными");
    assert_eq!(metrics.failed_operations, 200, "20% операций должны быть неудачными");
    
    Ok(())
}