//! Comprehensive tests for ResilienceService
//! 
//! Покрывает:
//! - Unit тесты для DefaultResilienceService
//! - Retry logic с exponential backoff
//! - Jitter применение для избежания thundering herd
//! - Error scenarios и максимальные попытки
//! - Статистика retry операций
//! - Performance и delay calculations

use cli::services::resilience::{
    ResilienceService, DefaultResilienceService, RetryConfig, ResilienceStats
};
use cli::services::types::OperationResult;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio;

// @component: {"k":"T","id":"resilience_service_tests","t":"Comprehensive resilience service tests","m":{"cur":95,"tgt":100,"u":"%"},"f":["test","unit","retry","exponential_backoff","jitter","coverage"]}

/// Mock operation для тестирования retry логики
struct MockOperation {
    /// Количество попыток до успеха
    fail_attempts: usize,
    /// Текущая попытка
    current_attempt: Arc<std::sync::atomic::AtomicUsize>,
    /// Результат операции после успеха
    success_result: String,
    /// Должна ли операция всегда падать
    always_fail: bool,
    /// Задержка выполнения операции
    execution_delay: Option<Duration>,
}

impl MockOperation {
    fn new(success_result: String) -> Self {
        Self {
            fail_attempts: 0,
            current_attempt: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            success_result,
            always_fail: false,
            execution_delay: None,
        }
    }
    
    fn with_failure_count(mut self, fail_attempts: usize) -> Self {
        self.fail_attempts = fail_attempts;
        self
    }
    
    fn with_always_fail(mut self) -> Self {
        self.always_fail = true;
        self
    }
    
    fn with_delay(mut self, delay: Duration) -> Self {
        self.execution_delay = Some(delay);
        self
    }
    
    async fn execute(&self) -> Result<String> {
        let attempt = self.current_attempt.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if let Some(delay) = self.execution_delay {
            tokio::time::sleep(delay).await;
        }
        
        if self.always_fail {
            return Err(anyhow!("Operation configured to always fail"));
        }
        
        if attempt < self.fail_attempts {
            return Err(anyhow!("Operation failing on attempt {}", attempt + 1));
        }
        
        Ok(self.success_result.clone())
    }
    
    fn get_attempt_count(&self) -> usize {
        self.current_attempt.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[test]
fn test_retry_config_defaults() {
    let config = RetryConfig::default();
    
    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.base_delay, Duration::from_millis(100));
    assert_eq!(config.max_delay, Duration::from_secs(30));
    assert_eq!(config.backoff_multiplier, 2.0);
    assert_eq!(config.jitter, true);
}

#[test]
fn test_delay_calculation_without_jitter() {
    let service = DefaultResilienceService::new();
    let config = RetryConfig {
        base_delay: Duration::from_millis(100),
        backoff_multiplier: 2.0,
        max_delay: Duration::from_secs(10),
        jitter: false,
        ..Default::default()
    };
    
    // Первая попытка (attempt = 1) не должна иметь delay
    let delay1 = service.calculate_delay(1, &config);
    assert_eq!(delay1, Duration::from_millis(100));
    
    // Вторая попытка: 100ms * 2^0 = 100ms
    let delay2 = service.calculate_delay(2, &config);
    assert_eq!(delay2, Duration::from_millis(200));
    
    // Третья попытка: 100ms * 2^1 = 200ms
    let delay3 = service.calculate_delay(3, &config);
    assert_eq!(delay3, Duration::from_millis(400));
}

#[test]
fn test_delay_calculation_with_max_limit() {
    let service = DefaultResilienceService::new();
    let config = RetryConfig {
        base_delay: Duration::from_millis(1000),
        backoff_multiplier: 10.0,
        max_delay: Duration::from_millis(2000),
        jitter: false,
        ..Default::default()
    };
    
    // Должно быть ограничено max_delay
    let delay = service.calculate_delay(5, &config);
    assert_eq!(delay, Duration::from_millis(2000));
}

#[test]
fn test_delay_calculation_with_jitter() {
    let service = DefaultResilienceService::new();
    let config = RetryConfig {
        base_delay: Duration::from_millis(1000),
        backoff_multiplier: 1.0,
        max_delay: Duration::from_secs(10),
        jitter: true,
        ..Default::default()
    };
    
    // С jitter результат должен быть в диапазоне [500ms, 1500ms]
    let delay = service.calculate_delay(1, &config);
    assert!(delay >= Duration::from_millis(500));
    assert!(delay <= Duration::from_millis(1500));
}

#[tokio::test]
async fn test_successful_operation_no_retries() -> Result<()> {
    let service = DefaultResilienceService::new();
    let config = RetryConfig::default();
    
    let mock_op = Arc::new(MockOperation::new("success".to_string()));
    let mock_op_clone = mock_op.clone();
    
    let result = service.execute_with_retry(
        move || {
            let op = mock_op_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    
    assert!(result.result.is_ok());
    assert_eq!(result.result.unwrap(), "success");
    assert_eq!(result.retries, 0);
    assert_eq!(mock_op.get_attempt_count(), 1);
    
    // Проверяем статистику
    let stats = service.get_resilience_stats().await;
    assert_eq!(stats.total_operations, 1);
    assert_eq!(stats.successful_operations, 1);
    assert_eq!(stats.failed_operations, 0);
    assert_eq!(stats.retried_operations, 0);
    
    Ok(())
}

#[tokio::test]
async fn test_operation_succeeds_after_retries() -> Result<()> {
    let service = DefaultResilienceService::new();
    let config = RetryConfig {
        max_attempts: 5,
        base_delay: Duration::from_millis(1), // Минимальная задержка для скорости теста
        jitter: false,
        ..Default::default()
    };
    
    let mock_op = Arc::new(MockOperation::new("success after retries".to_string())
        .with_failure_count(2)); // Fail первые 2 попытки, succeed на 3-й
    let mock_op_clone = mock_op.clone();
    
    let result = service.execute_with_retry(
        move || {
            let op = mock_op_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    
    assert!(result.result.is_ok());
    assert_eq!(result.result.unwrap(), "success after retries");
    assert_eq!(result.retries, 2); // 2 retry попытки
    assert_eq!(mock_op.get_attempt_count(), 3); // Всего 3 вызова
    
    // Проверяем статистику
    let stats = service.get_resilience_stats().await;
    assert_eq!(stats.total_operations, 1);
    assert_eq!(stats.successful_operations, 1);
    assert_eq!(stats.failed_operations, 0);
    assert_eq!(stats.retried_operations, 1);
    assert_eq!(stats.avg_retry_count, 2.0);
    
    Ok(())
}

#[tokio::test]
async fn test_operation_fails_after_max_retries() -> Result<()> {
    let service = DefaultResilienceService::new();
    let config = RetryConfig {
        max_attempts: 3,
        base_delay: Duration::from_millis(1),
        jitter: false,
        ..Default::default()
    };
    
    let mock_op = Arc::new(MockOperation::new("never succeeds".to_string())
        .with_always_fail());
    let mock_op_clone = mock_op.clone();
    
    let result = service.execute_with_retry(
        move || {
            let op = mock_op_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    
    assert!(result.result.is_err());
    assert_eq!(result.retries, 2); // 2 retry попытки (3 всего - 1 первоначальная)
    assert_eq!(mock_op.get_attempt_count(), 3); // Всего 3 вызова
    
    if let Err(e) = result.result {
        assert!(e.to_string().contains("always fail"));
    }
    
    // Проверяем статистику
    let stats = service.get_resilience_stats().await;
    assert_eq!(stats.total_operations, 1);
    assert_eq!(stats.successful_operations, 0);
    assert_eq!(stats.failed_operations, 1);
    assert_eq!(stats.retried_operations, 1);
    assert_eq!(stats.avg_retry_count, 2.0);
    
    Ok(())
}

#[tokio::test]
async fn test_operation_timing() -> Result<()> {
    let service = DefaultResilienceService::new();
    let config = RetryConfig {
        max_attempts: 2,
        base_delay: Duration::from_millis(10),
        jitter: false,
        ..Default::default()
    };
    
    let mock_op = Arc::new(MockOperation::new("timed operation".to_string())
        .with_delay(Duration::from_millis(50))
        .with_failure_count(1)); // Fail once, then succeed
    let mock_op_clone = mock_op.clone();
    
    let start_time = Instant::now();
    let result = service.execute_with_retry(
        move || {
            let op = mock_op_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    let total_time = start_time.elapsed();
    
    assert!(result.result.is_ok());
    assert_eq!(result.retries, 1);
    
    // Проверяем что время включает задержки
    // 2 операции по 50ms + 1 retry delay ~10ms = ~110ms минимум
    assert!(total_time >= Duration::from_millis(100));
    assert!(result.duration >= Duration::from_millis(100));
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let service = Arc::new(DefaultResilienceService::new());
    let config = RetryConfig {
        max_attempts: 3,
        base_delay: Duration::from_millis(1),
        jitter: false,
        ..Default::default()
    };
    
    let mut handles = vec![];
    
    // Создаем 10 concurrent операций
    for i in 0..10 {
        let service_clone = service.clone();
        let config_clone = config.clone();
        
        let handle = tokio::spawn(async move {
            let mock_op = Arc::new(MockOperation::new(format!("result_{}", i)));
            let mock_op_clone = mock_op.clone();
            
            service_clone.execute_with_retry(
                move || {
                    let op = mock_op_clone.clone();
                    async move { op.execute().await }
                },
                &config_clone,
            ).await
        });
        handles.push(handle);
    }
    
    // Ждем завершения всех операций
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(result)) = handle.await {
            if result.result.is_ok() {
                success_count += 1;
            }
        }
    }
    
    assert_eq!(success_count, 10);
    
    let stats = service.get_resilience_stats().await;
    assert_eq!(stats.total_operations, 10);
    assert_eq!(stats.successful_operations, 10);
    
    Ok(())
}

#[tokio::test]
async fn test_mixed_success_failure_statistics() -> Result<()> {
    let service = DefaultResilienceService::new();
    let config = RetryConfig {
        max_attempts: 3,
        base_delay: Duration::from_millis(1),
        jitter: false,
        ..Default::default()
    };
    
    // Операция 1: Успех без retry
    let op1 = Arc::new(MockOperation::new("success1".to_string()));
    let op1_clone = op1.clone();
    service.execute_with_retry(
        move || {
            let op = op1_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    
    // Операция 2: Успех после 1 retry
    let op2 = Arc::new(MockOperation::new("success2".to_string()).with_failure_count(1));
    let op2_clone = op2.clone();
    service.execute_with_retry(
        move || {
            let op = op2_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    
    // Операция 3: Успех после 2 retry
    let op3 = Arc::new(MockOperation::new("success3".to_string()).with_failure_count(2));
    let op3_clone = op3.clone();
    service.execute_with_retry(
        move || {
            let op = op3_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    
    // Операция 4: Полная неудача
    let op4 = Arc::new(MockOperation::new("failure".to_string()).with_always_fail());
    let op4_clone = op4.clone();
    service.execute_with_retry(
        move || {
            let op = op4_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    
    let stats = service.get_resilience_stats().await;
    
    assert_eq!(stats.total_operations, 4);
    assert_eq!(stats.successful_operations, 3);
    assert_eq!(stats.failed_operations, 1);
    assert_eq!(stats.retried_operations, 3); // op2, op3, op4 had retries
    
    // Средний count retry: (0 + 1 + 2 + 2) / 3 = 5/3 ≈ 1.67
    assert!((stats.avg_retry_count - 5.0/3.0).abs() < 0.01);
    
    Ok(())
}

#[tokio::test]
async fn test_stats_reset() -> Result<()> {
    let service = DefaultResilienceService::new();
    let config = RetryConfig::default();
    
    // Выполняем несколько операций
    let op1 = Arc::new(MockOperation::new("test1".to_string()));
    let op1_clone = op1.clone();
    service.execute_with_retry(
        move || {
            let op = op1_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    
    let op2 = Arc::new(MockOperation::new("test2".to_string()).with_failure_count(1));
    let op2_clone = op2.clone();
    service.execute_with_retry(
        move || {
            let op = op2_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    
    let stats_before = service.get_resilience_stats().await;
    assert_eq!(stats_before.total_operations, 2);
    
    // Сбрасываем статистику
    service.reset_stats().await;
    
    let stats_after = service.get_resilience_stats().await;
    assert_eq!(stats_after.total_operations, 0);
    assert_eq!(stats_after.successful_operations, 0);
    assert_eq!(stats_after.failed_operations, 0);
    assert_eq!(stats_after.retried_operations, 0);
    assert_eq!(stats_after.avg_retry_count, 0.0);
    
    Ok(())
}

#[tokio::test]
async fn test_custom_retry_config() -> Result<()> {
    let service = DefaultResilienceService::new();
    let config = RetryConfig {
        max_attempts: 5,
        base_delay: Duration::from_millis(50),
        max_delay: Duration::from_millis(200),
        backoff_multiplier: 1.5,
        jitter: false,
    };
    
    let mock_op = Arc::new(MockOperation::new("custom config test".to_string())
        .with_failure_count(3)); // Fail 3 times, succeed on 4th
    let mock_op_clone = mock_op.clone();
    
    let result = service.execute_with_retry(
        move || {
            let op = mock_op_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    
    assert!(result.result.is_ok());
    assert_eq!(result.retries, 3);
    assert_eq!(mock_op.get_attempt_count(), 4);
    
    Ok(())
}

#[tokio::test]
async fn test_single_attempt_config() -> Result<()> {
    let service = DefaultResilienceService::new();
    let config = RetryConfig {
        max_attempts: 1, // Только одна попытка
        ..Default::default()
    };
    
    let mock_op = Arc::new(MockOperation::new("single attempt".to_string())
        .with_always_fail());
    let mock_op_clone = mock_op.clone();
    
    let result = service.execute_with_retry(
        move || {
            let op = mock_op_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    
    assert!(result.result.is_err());
    assert_eq!(result.retries, 0); // Никаких retry
    assert_eq!(mock_op.get_attempt_count(), 1); // Только один вызов
    
    Ok(())
}

#[tokio::test]
async fn test_zero_delay_config() -> Result<()> {
    let service = DefaultResilienceService::new();
    let config = RetryConfig {
        max_attempts: 3,
        base_delay: Duration::from_millis(0),
        backoff_multiplier: 1.0,
        jitter: false,
        ..Default::default()
    };
    
    let mock_op = Arc::new(MockOperation::new("zero delay test".to_string())
        .with_failure_count(1));
    let mock_op_clone = mock_op.clone();
    
    let start_time = Instant::now();
    let result = service.execute_with_retry(
        move || {
            let op = mock_op_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    let total_time = start_time.elapsed();
    
    assert!(result.result.is_ok());
    assert_eq!(result.retries, 1);
    
    // С нулевой задержкой операция должна выполниться очень быстро
    assert!(total_time < Duration::from_millis(10));
    
    Ok(())
}

#[test]
fn test_resilience_stats_default() {
    let stats = ResilienceStats::default();
    
    assert_eq!(stats.total_operations, 0);
    assert_eq!(stats.successful_operations, 0);
    assert_eq!(stats.failed_operations, 0);
    assert_eq!(stats.retried_operations, 0);
    assert_eq!(stats.avg_retry_count, 0.0);
}

#[test]
fn test_retry_config_custom() {
    let config = RetryConfig {
        max_attempts: 10,
        base_delay: Duration::from_millis(500),
        max_delay: Duration::from_secs(60),
        backoff_multiplier: 3.0,
        jitter: false,
    };
    
    assert_eq!(config.max_attempts, 10);
    assert_eq!(config.base_delay, Duration::from_millis(500));
    assert_eq!(config.max_delay, Duration::from_secs(60));
    assert_eq!(config.backoff_multiplier, 3.0);
    assert_eq!(config.jitter, false);
}

#[tokio::test]
async fn test_exponential_backoff_timing() -> Result<()> {
    let service = DefaultResilienceService::new();
    let config = RetryConfig {
        max_attempts: 4,
        base_delay: Duration::from_millis(50),
        backoff_multiplier: 2.0,
        max_delay: Duration::from_secs(10),
        jitter: false,
    };
    
    let mock_op = Arc::new(MockOperation::new("backoff test".to_string())
        .with_failure_count(3)); // Fail 3 times, succeed on 4th
    let mock_op_clone = mock_op.clone();
    
    let start_time = Instant::now();
    let result = service.execute_with_retry(
        move || {
            let op = mock_op_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    let total_time = start_time.elapsed();
    
    assert!(result.result.is_ok());
    assert_eq!(result.retries, 3);
    
    // Ожидаемые задержки: 50ms + 100ms + 200ms = 350ms минимум
    assert!(total_time >= Duration::from_millis(300));
    
    Ok(())
}

#[tokio::test]
async fn test_operation_result_fields() -> Result<()> {
    let service = DefaultResilienceService::new();
    let config = RetryConfig {
        max_attempts: 3,
        base_delay: Duration::from_millis(10),
        jitter: false,
        ..Default::default()
    };
    
    let mock_op = Arc::new(MockOperation::new("field test".to_string())
        .with_failure_count(1));
    let mock_op_clone = mock_op.clone();
    
    let result = service.execute_with_retry(
        move || {
            let op = mock_op_clone.clone();
            async move { op.execute().await }
        },
        &config,
    ).await?;
    
    // Проверяем все поля OperationResult
    assert!(result.result.is_ok());
    assert_eq!(result.retries, 1);
    assert!(!result.from_cache); // ResilienceService не использует кэш
    assert!(result.duration > Duration::from_millis(0));
    
    if let Ok(value) = result.result {
        assert_eq!(value, "field test");
    }
    
    Ok(())
}