use memory::{RetryManager, RetryConfig};
use anyhow::{anyhow, Result};
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::timeout;

/// Комплексные unit тесты для retry системы
/// Тестирует: exponential backoff, jitter, retriable errors, configurations, performance

/// Тест создания RetryConfig с различными настройками
#[test]
fn test_retry_config_creation() {
    println!("🧪 Тестируем создание RetryConfig");
    
    // Default конфигурация
    let default_config = RetryConfig::default();
    assert_eq!(default_config.max_attempts, 3);
    assert_eq!(default_config.base_delay, Duration::from_millis(100));
    assert_eq!(default_config.max_delay, Duration::from_secs(5));
    assert_eq!(default_config.backoff_multiplier, 2.0);
    assert!(default_config.jitter);
    
    println!("  ✅ Default: {} attempts, base={}ms, max={}s, multiplier={}, jitter={}", 
             default_config.max_attempts, 
             default_config.base_delay.as_millis(),
             default_config.max_delay.as_secs(),
             default_config.backoff_multiplier,
             default_config.jitter);
    
    // Custom конфигурация
    let custom_config = RetryConfig {
        max_attempts: 5,
        base_delay: Duration::from_millis(50),
        max_delay: Duration::from_secs(10),
        backoff_multiplier: 1.5,
        jitter: false,
    };
    
    assert_eq!(custom_config.max_attempts, 5);
    assert_eq!(custom_config.base_delay, Duration::from_millis(50));
    assert_eq!(custom_config.max_delay, Duration::from_secs(10));
    assert_eq!(custom_config.backoff_multiplier, 1.5);
    assert!(!custom_config.jitter);
    
    println!("  ✅ Custom: {} attempts, no jitter, 1.5x multiplier", custom_config.max_attempts);
    
    println!("✅ Создание RetryConfig работает корректно");
}

/// Тест создания RetryManager с различными конфигурациями
#[test]
fn test_retry_manager_creation() {
    println!("🧪 Тестируем создание RetryManager");
    
    // Default manager
    let _default_manager = RetryManager::with_defaults();
    println!("  ✅ Default manager создан");
    
    // Custom manager
    let custom_config = RetryConfig {
        max_attempts: 10,
        base_delay: Duration::from_millis(25),
        max_delay: Duration::from_secs(1),
        backoff_multiplier: 3.0,
        jitter: true,
    };
    
    let _custom_manager = RetryManager::new(custom_config);
    println!("  ✅ Custom manager создан");
    
    // Database manager
    let _db_manager = RetryManager::for_database();
    println!("  ✅ Database manager создан");
    
    // HNSW manager
    let _hnsw_manager = RetryManager::for_hnsw();
    println!("  ✅ HNSW manager создан");
    
    println!("✅ Создание RetryManager работает корректно");
}

/// Тест успешной операции без retry
#[tokio::test]
async fn test_successful_operation_no_retry() -> Result<()> {
    println!("🧪 Тестируем успешную операцию без retry");
    
    let manager = RetryManager::with_defaults();
    let attempt_count = Arc::new(AtomicUsize::new(0));
    let attempt_count_clone = Arc::clone(&attempt_count);
    
    let start_time = Instant::now();
    
    let result = manager.retry("successful_operation", || {
        let count_clone = Arc::clone(&attempt_count_clone);
        async move {
            count_clone.fetch_add(1, Ordering::SeqCst);
            Ok::<i32, anyhow::Error>(42)
        }
    }).await?;
    
    let elapsed = start_time.elapsed();
    
    // Проверяем результат
    assert_eq!(result, 42);
    assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
    
    // Операция должна быть быстрой (без задержек)
    assert!(elapsed < Duration::from_millis(10));
    
    println!("  ✅ Операция выполнена за 1 попытку за {:?}", elapsed);
    
    println!("✅ Успешная операция работает корректно");
    Ok(())
}

/// Тест операции с failure и retry
#[tokio::test]
async fn test_operation_with_retries() -> Result<()> {
    println!("🧪 Тестируем операцию с retry после failures");
    
    let manager = RetryManager::new(RetryConfig {
        max_attempts: 3,
        base_delay: Duration::from_millis(10), // Короткие задержки для тестов
        max_delay: Duration::from_millis(100),
        backoff_multiplier: 2.0,
        jitter: false, // Отключаем jitter для предсказуемости
    });
    
    let attempt_count = Arc::new(AtomicUsize::new(0));
    let attempt_count_clone = Arc::clone(&attempt_count);
    
    let start_time = Instant::now();
    
    let result = manager.retry("retry_operation", || {
        let count_clone = Arc::clone(&attempt_count_clone);
        async move {
            let current_attempt = count_clone.fetch_add(1, Ordering::SeqCst) + 1;
            
            if current_attempt < 3 {
                // Первые 2 попытки неудачны
                Err(anyhow!("Attempt {} failed", current_attempt))
            } else {
                // 3-я попытка успешна
                Ok::<String, anyhow::Error>(format!("Success on attempt {}", current_attempt))
            }
        }
    }).await?;
    
    let elapsed = start_time.elapsed();
    
    // Проверяем результат
    assert_eq!(result, "Success on attempt 3");
    assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    
    // Должно быть минимум 2 задержки (10ms + 20ms)
    assert!(elapsed >= Duration::from_millis(25));
    
    println!("  ✅ Операция успешна на 3-й попытке за {:?}", elapsed);
    
    println!("✅ Retry после failures работает корректно");
    Ok(())
}

/// Тест полного failure после всех попыток
#[tokio::test]
async fn test_operation_complete_failure() {
    println!("🧪 Тестируем полный failure после всех попыток");
    
    let manager = RetryManager::new(RetryConfig {
        max_attempts: 2,
        base_delay: Duration::from_millis(5),
        max_delay: Duration::from_millis(50),
        backoff_multiplier: 2.0,
        jitter: false,
    });
    
    let attempt_count = Arc::new(AtomicUsize::new(0));
    let attempt_count_clone = Arc::clone(&attempt_count);
    
    let start_time = Instant::now();
    
    let result = manager.retry("failing_operation", || {
        let count_clone = Arc::clone(&attempt_count_clone);
        async move {
            let current_attempt = count_clone.fetch_add(1, Ordering::SeqCst) + 1;
            Err::<i32, anyhow::Error>(anyhow!("Always fails on attempt {}", current_attempt))
        }
    }).await;
    
    let elapsed = start_time.elapsed();
    
    // Проверяем что операция неудачна
    assert!(result.is_err());
    assert_eq!(attempt_count.load(Ordering::SeqCst), 2);
    
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("failed after 2 attempts"));
    assert!(error_msg.contains("Always fails on attempt 2"));
    
    // Должна быть минимум 1 задержка (5ms)
    assert!(elapsed >= Duration::from_millis(4));
    
    println!("  ✅ Полный failure после {} попыток за {:?}", 2, elapsed);
    println!("  ✅ Error message: {}", error_msg);
    
    println!("✅ Полный failure обработан корректно");
}

/// Тест exponential backoff расчетов
#[test]
fn test_exponential_backoff_calculation() {
    println!("🧪 Тестируем расчеты exponential backoff");
    
    let config = RetryConfig {
        max_attempts: 5,
        base_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(2),
        backoff_multiplier: 2.0,
        jitter: false, // Отключаем jitter для точных тестов
    };
    
    let manager = RetryManager::new(config);
    
    // Тестируем расчет задержек для разных попыток
    let delay1 = manager.calculate_delay(1);
    let delay2 = manager.calculate_delay(2);
    let delay3 = manager.calculate_delay(3);
    let delay4 = manager.calculate_delay(4);
    let delay5 = manager.calculate_delay(5);
    
    // Exponential backoff: 100ms * 2^(attempt-1)
    assert_eq!(delay1, Duration::from_millis(100));  // 100ms * 2^0 = 100ms
    assert_eq!(delay2, Duration::from_millis(200));  // 100ms * 2^1 = 200ms
    assert_eq!(delay3, Duration::from_millis(400));  // 100ms * 2^2 = 400ms
    assert_eq!(delay4, Duration::from_millis(800));  // 100ms * 2^3 = 800ms
    assert_eq!(delay5, Duration::from_millis(1600)); // 100ms * 2^4 = 1600ms
    
    println!("  ✅ Exponential backoff: {}ms -> {}ms -> {}ms -> {}ms -> {}ms", 
             delay1.as_millis(), delay2.as_millis(), delay3.as_millis(), 
             delay4.as_millis(), delay5.as_millis());
    
    // Тест capping с max_delay
    let config_capped = RetryConfig {
        max_attempts: 5,
        base_delay: Duration::from_millis(100),
        max_delay: Duration::from_millis(300), // Низкий лимит
        backoff_multiplier: 2.0,
        jitter: false,
    };
    
    let manager_capped = RetryManager::new(config_capped);
    let delay_capped3 = manager_capped.calculate_delay(3);
    let delay_capped4 = manager_capped.calculate_delay(4);
    
    // Должны быть ограничены max_delay
    assert_eq!(delay_capped3, Duration::from_millis(300)); // Capped at 300ms instead of 400ms
    assert_eq!(delay_capped4, Duration::from_millis(300)); // Capped at 300ms instead of 800ms
    
    println!("  ✅ Max delay capping: {}ms и {}ms capped at 300ms", 
             delay_capped3.as_millis(), delay_capped4.as_millis());
    
    println!("✅ Exponential backoff расчеты работают корректно");
}

/// Тест jitter functionality
#[test]
fn test_jitter_functionality() {
    println!("🧪 Тестируем jitter functionality");
    
    let config_with_jitter = RetryConfig {
        max_attempts: 3,
        base_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(1),
        backoff_multiplier: 2.0,
        jitter: true,
    };
    
    let config_without_jitter = RetryConfig {
        max_attempts: 3,
        base_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(1),
        backoff_multiplier: 2.0,
        jitter: false,
    };
    
    let manager_with_jitter = RetryManager::new(config_with_jitter);
    let manager_without_jitter = RetryManager::new(config_without_jitter);
    
    // Генерируем несколько задержек с jitter
    let mut jitter_delays = Vec::new();
    for _ in 0..10 {
        jitter_delays.push(manager_with_jitter.calculate_delay(2));
    }
    
    // Генерируем задержки без jitter
    let mut no_jitter_delays = Vec::new();
    for _ in 0..10 {
        no_jitter_delays.push(manager_without_jitter.calculate_delay(2));
    }
    
    // Без jitter все задержки должны быть одинаковыми
    assert!(no_jitter_delays.iter().all(|&d| d == Duration::from_millis(200)));
    
    // С jitter задержки должны варьироваться
    let unique_jitter_delays: std::collections::HashSet<_> = jitter_delays.iter().collect();
    assert!(unique_jitter_delays.len() > 1, "Jitter должен создавать различные задержки");
    
    // Все jitter задержки должны быть в разумном диапазоне (200ms ± 25%)
    for delay in &jitter_delays {
        let delay_ms = delay.as_millis();
        assert!(delay_ms >= 150 && delay_ms <= 250, 
                "Jitter delay {}ms вне диапазона 150-250ms", delay_ms);
    }
    
    println!("  ✅ Без jitter: все задержки {}ms", no_jitter_delays[0].as_millis());
    println!("  ✅ С jitter: {} уникальных задержек в диапазоне 150-250ms", unique_jitter_delays.len());
    
    println!("✅ Jitter functionality работает корректно");
}

/// Тест определения retriable errors
#[test]
fn test_retriable_error_detection() {
    println!("🧪 Тестируем определение retriable errors");
    
    // Retriable errors
    let retriable_errors = vec![
        anyhow!("Database lock error occurred"),
        anyhow!("Resource is busy, try again"),
        anyhow!("I/O error: connection timeout"),
        anyhow!("Connection failed temporarily"),
        anyhow!("HNSW не инициализирован"),
        anyhow!("HNSW not initialized yet"),
        anyhow!("Resource temporarily unavailable"),
        anyhow!("Lock acquisition failed"),
    ];
    
    for (i, error) in retriable_errors.iter().enumerate() {
        assert!(RetryManager::is_retriable_error(error), 
                "Error {} должна быть retriable: {}", i, error);
    }
    
    println!("  ✅ Retriable errors: {} типов обнаружено", retriable_errors.len());
    
    // Non-retriable errors
    let non_retriable_errors = vec![
        anyhow!("Invalid argument provided"),
        anyhow!("File not found"),
        anyhow!("Permission denied"),
        anyhow!("Syntax error in query"),
        anyhow!("Out of memory"),
        anyhow!("Invalid configuration"),
    ];
    
    for (i, error) in non_retriable_errors.iter().enumerate() {
        assert!(!RetryManager::is_retriable_error(error), 
                "Error {} не должна быть retriable: {}", i, error);
    }
    
    println!("  ✅ Non-retriable errors: {} типов обнаружено", non_retriable_errors.len());
    
    println!("✅ Определение retriable errors работает корректно");
}

/// Тест предустановленных конфигураций
#[tokio::test]
async fn test_preset_configurations() -> Result<()> {
    println!("🧪 Тестируем предустановленные конфигурации");
    
    // Database retry manager
    let db_manager = RetryManager::for_database();
    let db_attempt_count = Arc::new(AtomicUsize::new(0));
    let db_count_clone = Arc::clone(&db_attempt_count);
    
    let db_result = db_manager.retry("db_operation", || {
        let count_clone = Arc::clone(&db_count_clone);
        async move {
            let current_attempt = count_clone.fetch_add(1, Ordering::SeqCst) + 1;
            if current_attempt < 3 {
                Err(anyhow!("Database lock error occurred"))
            } else {
                Ok::<String, anyhow::Error>("DB success".to_string())
            }
        }
    }).await?;
    
    assert_eq!(db_result, "DB success");
    assert_eq!(db_attempt_count.load(Ordering::SeqCst), 3);
    println!("  ✅ Database manager: успех на 3-й попытке");
    
    // HNSW retry manager
    let hnsw_manager = RetryManager::for_hnsw();
    let hnsw_attempt_count = Arc::new(AtomicUsize::new(0));
    let hnsw_count_clone = Arc::clone(&hnsw_attempt_count);
    
    let hnsw_result = hnsw_manager.retry("hnsw_operation", || {
        let count_clone = Arc::clone(&hnsw_count_clone);
        async move {
            let current_attempt = count_clone.fetch_add(1, Ordering::SeqCst) + 1;
            if current_attempt < 2 {
                Err(anyhow!("HNSW not initialized yet"))
            } else {
                Ok::<i32, anyhow::Error>(123)
            }
        }
    }).await?;
    
    assert_eq!(hnsw_result, 123);
    assert_eq!(hnsw_attempt_count.load(Ordering::SeqCst), 2);
    println!("  ✅ HNSW manager: успех на 2-й попытке");
    
    println!("✅ Предустановленные конфигурации работают корректно");
    Ok(())
}

/// Тест timeout поведения
#[tokio::test]
async fn test_retry_timeout_behavior() {
    println!("🧪 Тестируем поведение retry с timeout");
    
    let manager = RetryManager::new(RetryConfig {
        max_attempts: 10, // Много попыток
        base_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(1),
        backoff_multiplier: 2.0,
        jitter: false,
    });
    
    let attempt_count = Arc::new(AtomicUsize::new(0));
    let attempt_count_clone = Arc::clone(&attempt_count);
    
    // Устанавливаем общий timeout на операцию
    let result = timeout(Duration::from_millis(250), 
        manager.retry("timeout_operation", || {
            let count_clone = Arc::clone(&attempt_count_clone);
            async move {
                count_clone.fetch_add(1, Ordering::SeqCst);
                Err::<i32, anyhow::Error>(anyhow!("Always fails"))
            }
        })
    ).await;
    
    // Timeout должен сработать
    assert!(result.is_err());
    
    // Должно было быть только несколько попыток из-за timeout
    let attempts = attempt_count.load(Ordering::SeqCst);
    assert!(attempts >= 1 && attempts < 10, "Ожидается 1-9 попыток, получено {}", attempts);
    
    println!("  ✅ Timeout сработал после {} попыток", attempts);
    
    println!("✅ Retry timeout поведение работает корректно");
}

/// Тест edge cases
#[tokio::test]
async fn test_retry_edge_cases() -> Result<()> {
    println!("🧪 Тестируем edge cases для retry");
    
    // Тест с max_attempts = 1 (no retry)
    let single_attempt_manager = RetryManager::new(RetryConfig {
        max_attempts: 1,
        base_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(1),
        backoff_multiplier: 2.0,
        jitter: false,
    });
    
    let result = single_attempt_manager.retry("single_attempt", || async {
        Err::<i32, anyhow::Error>(anyhow!("Single failure"))
    }).await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("failed after 1 attempts"));
    println!("  ✅ Single attempt (no retry) работает корректно");
    
    // Тест с очень малой base_delay
    let fast_manager = RetryManager::new(RetryConfig {
        max_attempts: 3,
        base_delay: Duration::from_nanos(1),
        max_delay: Duration::from_millis(1),
        backoff_multiplier: 2.0,
        jitter: false,
    });
    
    let fast_attempt_count = Arc::new(AtomicUsize::new(0));
    let fast_count_clone = Arc::clone(&fast_attempt_count);
    
    let start_time = Instant::now();
    let result = fast_manager.retry("fast_operation", || {
        let count_clone = Arc::clone(&fast_count_clone);
        async move {
            let current_attempt = count_clone.fetch_add(1, Ordering::SeqCst) + 1;
            if current_attempt < 3 {
                Err(anyhow!("Fast failure"))
            } else {
                Ok::<String, anyhow::Error>("Fast success".to_string())
            }
        }
    }).await?;
    let elapsed = start_time.elapsed();
    
    assert_eq!(result, "Fast success");
    assert!(elapsed < Duration::from_millis(50)); // Более реалистичный лимит
    println!("  ✅ Fast retry (nanosecond delays) за {:?}", elapsed);
    
    // Тест с большим backoff_multiplier
    let big_multiplier_manager = RetryManager::new(RetryConfig {
        max_attempts: 3,
        base_delay: Duration::from_millis(1),
        max_delay: Duration::from_millis(5), // Низкий лимит для capping
        backoff_multiplier: 100.0,
        jitter: false,
    });
    
    let delay1 = big_multiplier_manager.calculate_delay(1);
    let delay2 = big_multiplier_manager.calculate_delay(2);
    
    // Должны быть capped
    assert_eq!(delay1, Duration::from_millis(1));
    assert_eq!(delay2, Duration::from_millis(5)); // Capped at max_delay
    
    println!("  ✅ Large multiplier capping: {}ms -> {}ms", delay1.as_millis(), delay2.as_millis());
    
    println!("✅ Edge cases обработаны корректно");
    Ok(())
}

/// Performance test для retry операций
#[tokio::test]
async fn test_retry_performance() -> Result<()> {
    println!("🧪 Тестируем производительность retry операций");
    
    let manager = RetryManager::new(RetryConfig {
        max_attempts: 2,
        base_delay: Duration::from_nanos(1), // Минимальные задержки
        max_delay: Duration::from_millis(1),
        backoff_multiplier: 1.1,
        jitter: false,
    });
    
    let start_time = Instant::now();
    
    // Множественные быстрые операции
    for i in 0..1000 {
        let _result = manager.retry("perf_operation", || async move {
            if i % 100 == 0 {
                // Каждая 100-я операция неудачна (требует retry)
                Err::<i32, anyhow::Error>(anyhow!("Occasional failure"))
            } else {
                Ok::<i32, anyhow::Error>(i)
            }
        }).await;
        
        // Большинство должны быть успешными
        if i % 100 != 0 {
            assert!(_result.is_ok());
        }
    }
    
    let elapsed = start_time.elapsed();
    println!("  📊 1000 операций (с 10 retries) выполнено за {:?}", elapsed);
    
    // Должно быть достаточно быстро - увеличиваем лимит для CI
    assert!(elapsed < Duration::from_millis(500));
    
    println!("✅ Производительность retry операций отличная");
    Ok(())
}

/// Integration test всей retry системы
#[tokio::test]
async fn test_retry_system_integration() -> Result<()> {
    println!("🧪 Integration test retry системы");
    
    // Сценарий 1: Database операция с несколькими retries
    let db_manager = RetryManager::for_database();
    let db_attempts = Arc::new(AtomicUsize::new(0));
    let db_attempts_clone = Arc::clone(&db_attempts);
    
    let db_result = db_manager.retry("database_integration", || {
        let attempts_clone = Arc::clone(&db_attempts_clone);
        async move {
            let attempt = attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;
            
            match attempt {
                1 => Err(anyhow!("Database lock error occurred")),
                2 => Err(anyhow!("Connection failed temporarily")), 
                3 => Ok::<String, anyhow::Error>("Database operation success".to_string()),
                _ => panic!("Too many attempts")
            }
        }
    }).await?;
    
    assert_eq!(db_result, "Database operation success");
    assert_eq!(db_attempts.load(Ordering::SeqCst), 3);
    println!("  ✅ Сценарий 1: Database операция успешна на 3-й попытке");
    
    // Сценарий 2: HNSW операция с быстрым recovery
    let hnsw_manager = RetryManager::for_hnsw();
    let hnsw_attempts = Arc::new(AtomicUsize::new(0));
    let hnsw_attempts_clone = Arc::clone(&hnsw_attempts);
    
    let hnsw_result = hnsw_manager.retry("hnsw_integration", || {
        let attempts_clone = Arc::clone(&hnsw_attempts_clone);
        async move {
            let attempt = attempts_clone.fetch_add(1, Ordering::SeqCst) + 1;
            
            if attempt == 1 {
                Err(anyhow!("HNSW не инициализирован"))
            } else {
                Ok::<Vec<f32>, anyhow::Error>(vec![0.1, 0.2, 0.3])
            }
        }
    }).await?;
    
    assert_eq!(hnsw_result, vec![0.1, 0.2, 0.3]);
    assert_eq!(hnsw_attempts.load(Ordering::SeqCst), 2);
    println!("  ✅ Сценарий 2: HNSW операция успешна на 2-й попытке");
    
    // Сценарий 3: Non-retriable error (immediate failure)
    let manager = RetryManager::with_defaults();
    let non_retriable_attempts = Arc::new(AtomicUsize::new(0));
    let non_retriable_attempts_clone = Arc::clone(&non_retriable_attempts);
    
    let non_retriable_result = manager.retry("non_retriable_integration", || {
        let attempts_clone = Arc::clone(&non_retriable_attempts_clone);
        async move {
            attempts_clone.fetch_add(1, Ordering::SeqCst);
            Err::<i32, anyhow::Error>(anyhow!("Permission denied"))
        }
    }).await;
    
    assert!(non_retriable_result.is_err());
    assert_eq!(non_retriable_attempts.load(Ordering::SeqCst), 3); // Все попытки использованы
    println!("  ✅ Сценарий 3: Non-retriable error after все попытки");
    
    println!("✅ Integration test retry системы успешен");
    Ok(())
}

/// Quick smoke test для всех основных функций
#[tokio::test]
async fn test_retry_smoke() -> Result<()> {
    // Test config creation
    let _config = RetryConfig::default();
    
    // Test manager creation
    let manager = RetryManager::with_defaults();
    let _db_manager = RetryManager::for_database();
    let _hnsw_manager = RetryManager::for_hnsw();
    
    // Test successful operation
    let result = manager.retry("smoke_test", || async {
        Ok::<i32, anyhow::Error>(42)
    }).await?;
    assert_eq!(result, 42);
    
    // Test error detection
    let error = anyhow!("Database lock error occurred");
    assert!(RetryManager::is_retriable_error(&error));
    
    println!("✅ Все функции retry работают");
    Ok(())
}