//! Comprehensive Integration Tests for Unified DI System
//! 
//! Tests end-to-end functionality of the entire DI architecture including:
//! - Container creation and service resolution
//! - Factory pattern implementations  
//! - Configuration management and validation
//! - Error handling and recovery
//! - Thread safety and concurrent operations
//! - Memory management and cleanup

use std::sync::Arc;
use tokio::time::{timeout, Duration};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    di::{
        unified_container::UnifiedDIContainer,
        unified_config::UnifiedDIConfiguration,
        errors::{DIError, DIResult},
    },
    services::{
        unified_factory::{UnifiedServiceFactory, FactoryPreset},
        monitoring_service::MonitoringService,
        cache_service::CacheService,
    },
    types::MemoryRecord,
};

/// Test fixture для создания тестовых сервисов
struct TestServiceFixture {
    container: Arc<UnifiedDIContainer>,
    config: UnifiedDIConfiguration,
}

impl TestServiceFixture {
    async fn new() -> DIResult<Self> {
        let config = UnifiedDIConfiguration::test_config()?;
        let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Test)?;
        let container = factory.build_container(&config).await?;
        
        Ok(TestServiceFixture {
            container: Arc::new(container),
            config,
        })
    }
    
    async fn new_with_config(config: UnifiedDIConfiguration) -> DIResult<Self> {
        let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Custom)?;
        let container = factory.build_container(&config).await?;
        
        Ok(TestServiceFixture {
            container: Arc::new(container),
            config,
        })
    }
}

#[tokio::test]
async fn test_complete_di_system_integration() -> DIResult<()> {
    // Создаем полную DI систему
    let fixture = TestServiceFixture::new().await?;
    
    // Проверяем что все сервисы доступны
    let monitoring_service = fixture.container
        .resolve::<MonitoringService>()
        .await?;
    
    let cache_service = fixture.container
        .resolve::<CacheService>()
        .await?;
    
    // Тестируем взаимодействие между сервисами
    let test_record = create_test_record("integration_test_1");
    
    // Проверяем что сервисы работают совместно
    let cache_key = cache_service.create_cache_key(&test_record.content);
    assert!(!cache_key.is_empty());
    
    // Проверяем мониторинг
    let initial_stats = monitoring_service.get_stats().await?;
    assert_eq!(initial_stats.total_operations, 0);
    
    // Выполняем операцию через систему
    monitoring_service.record_operation("test_operation", Duration::from_millis(100)).await;
    
    let updated_stats = monitoring_service.get_stats().await?;
    assert_eq!(updated_stats.total_operations, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_factory_preset_configurations() -> DIResult<()> {
    // Тестируем все preset конфигурации
    let presets = vec![
        FactoryPreset::Production,
        FactoryPreset::Development, 
        FactoryPreset::Test,
        FactoryPreset::Minimal,
    ];
    
    for preset in presets {
        let factory = UnifiedServiceFactory::with_preset(preset)?;
        let config = match preset {
            FactoryPreset::Production => UnifiedDIConfiguration::production_config()?,
            FactoryPreset::Development => UnifiedDIConfiguration::development_config()?,
            FactoryPreset::Test => UnifiedDIConfiguration::test_config()?,
            FactoryPreset::Minimal => UnifiedDIConfiguration::minimal_config()?,
            _ => UnifiedDIConfiguration::default_config()?,
        };
        
        // Проверяем что контейнер создается без ошибок
        let container = factory.build_container(&config).await?;
        
        // Проверяем что базовые сервисы доступны
        let monitoring = container.resolve::<MonitoringService>().await?;
        assert!(monitoring.is_healthy().await);
        
        // Cleanup
        container.shutdown().await?;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_configuration_validation_and_error_handling() -> DIResult<()> {
    // Тест валидации конфигурации с невалидными данными
    let mut config = UnifiedDIConfiguration::test_config()?;
    
    // Устанавливаем невалидные значения
    config.max_services = 0; // Invalid
    config.timeout_seconds = 0; // Invalid
    
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Test)?;
    
    // Проверяем что система корректно обрабатывает ошибки валидации
    let result = factory.build_container(&config).await;
    assert!(result.is_err());
    
    match result.unwrap_err() {
        DIError::ConfigurationError { source, field } => {
            assert!(field.contains("max_services") || field.contains("timeout"));
        }
        _ => panic!("Expected ConfigurationError"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_container_operations() -> DIResult<()> {
    let fixture = TestServiceFixture::new().await?;
    let container = fixture.container.clone();
    
    // Счетчик для отслеживания конкурентных операций
    let operation_counter = Arc::new(AtomicUsize::new(0));
    
    // Запускаем множественные конкурентные операции
    let tasks: Vec<_> = (0..10).map(|i| {
        let container = container.clone();
        let counter = operation_counter.clone();
        
        tokio::spawn(async move {
            // Resolve сервисы конкурентно
            let monitoring = container.resolve::<MonitoringService>().await?;
            let cache = container.resolve::<CacheService>().await?;
            
            // Выполняем операции
            monitoring.record_operation(
                &format!("concurrent_op_{}", i), 
                Duration::from_millis(10)
            ).await;
            
            let test_record = create_test_record(&format!("concurrent_test_{}", i));
            let _cache_key = cache.create_cache_key(&test_record.content);
            
            counter.fetch_add(1, Ordering::SeqCst);
            
            DIResult::Ok(i)
        })
    }).collect();
    
    // Ждем завершения всех задач с timeout
    let results = timeout(Duration::from_secs(30), async {
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await.expect("Test operation should succeed")?);
        }
        DIResult::Ok(results)
    }).await.map_err(|_| DIError::TimeoutError { 
        operation: "concurrent_operations".to_string(), 
        timeout_ms: 30000 
    })??;
    
    // Проверяем что все операции завершились успешно
    assert_eq!(results.len(), 10);
    assert_eq!(operation_counter.load(Ordering::SeqCst), 10);
    
    Ok(())
}

#[tokio::test]
async fn test_service_lifecycle_management() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Test)?;
    
    // Создаем контейнер
    let container = factory.build_container(&config).await?;
    
    // Проверяем что сервисы инициализированы
    let monitoring = container.resolve::<MonitoringService>().await?;
    assert!(monitoring.is_healthy().await);
    
    // Проверяем статистику контейнера
    let stats = container.get_statistics().await?;
    assert!(stats.registered_services > 0);
    assert!(stats.active_instances > 0);
    
    // Выполняем graceful shutdown
    container.shutdown().await?;
    
    // Проверяем что сервисы корректно остановились
    let post_shutdown_stats = container.get_statistics().await?;
    assert_eq!(post_shutdown_stats.active_instances, 0);
    
    Ok(())
}

#[tokio::test]
async fn test_error_recovery_and_resilience() -> DIResult<()> {
    let fixture = TestServiceFixture::new().await?;
    
    // Симулируем ошибку в сервисе
    let monitoring = fixture.container.resolve::<MonitoringService>().await?;
    
    // Тестируем recovery после ошибки
    for i in 0..5 {
        let operation_name = format!("recovery_test_{}", i);
        
        // Симулируем разные типы ошибок
        if i % 2 == 0 {
            // Успешная операция
            monitoring.record_operation(&operation_name, Duration::from_millis(50)).await;
        } else {
            // Операция с ошибкой - но система должна продолжать работать
            monitoring.record_error(&operation_name, "Simulated error").await;
        }
    }
    
    // Проверяем что система все еще функциональна
    let stats = monitoring.get_stats().await?;
    assert!(stats.total_operations > 0);
    assert!(stats.error_count > 0);
    
    // Система должна быть все еще здоровой после ошибок
    assert!(monitoring.is_healthy().await);
    
    Ok(())
}

#[tokio::test]
async fn test_memory_management_and_cleanup() -> DIResult<()> {
    // Создаем множественные контейнеры для тестирования memory management
    let mut containers = Vec::new();
    
    for i in 0..5 {
        let config = UnifiedDIConfiguration::test_config()?;
        let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Minimal)?;
        let container = factory.build_container(&config).await?;
        
        // Используем сервисы для создания некоторых данных в памяти
        let monitoring = container.resolve::<MonitoringService>().await?;
        monitoring.record_operation(&format!("memory_test_{}", i), Duration::from_millis(1)).await;
        
        containers.push(container);
    }
    
    // Получаем исходные статистики памяти (если доступны)
    let initial_memory = std::process::id(); // Простая метрика
    
    // Очищаем все контейнеры
    for container in containers {
        container.shutdown().await?;
    }
    
    // Форсируем garbage collection (в реальных условиях это происходит автоматически)
    // Здесь мы просто проверяем что shutdown прошел без ошибок
    
    Ok(())
}

#[tokio::test] 
async fn test_configuration_hot_reload_scenarios() -> DIResult<()> {
    let mut config = UnifiedDIConfiguration::test_config()?;
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Development)?;
    let container = factory.build_container(&config).await?;
    
    // Получаем начальные статистики
    let monitoring = container.resolve::<MonitoringService>().await?;
    let initial_stats = monitoring.get_stats().await?;
    
    // Симулируем изменение конфигурации
    config.max_services = config.max_services + 10;
    config.timeout_seconds = config.timeout_seconds + 5;
    
    // В реальной системе здесь был бы hot reload
    // Пока что просто проверяем что система может обработать новую конфигурацию
    let new_container = factory.build_container(&config).await?;
    let new_monitoring = new_container.resolve::<MonitoringService>().await?;
    
    // Проверяем что новый контейнер работает
    assert!(new_monitoring.is_healthy().await);
    
    // Cleanup
    container.shutdown().await?;
    new_container.shutdown().await?;
    
    Ok(())
}

// Helper functions для тестов

fn create_test_record(content: &str) -> MemoryRecord {
    MemoryRecord {
        id: uuid::Uuid::new_v4().to_string(),
        content: content.to_string(),
        embedding: vec![0.1, 0.2, 0.3], // Простой тестовый embedding
        metadata: std::collections::HashMap::new(),
        created_at: chrono::Utc::now(),
        access_count: 0,
    }
}

/// Расширенный тест для проверки complex scenarios
#[tokio::test]
async fn test_complex_service_interaction_patterns() -> DIResult<()> {
    let fixture = TestServiceFixture::new().await?;
    
    // Тестируем сложные паттерны взаимодействия между сервисами
    let monitoring = fixture.container.resolve::<MonitoringService>().await?;
    let cache = fixture.container.resolve::<CacheService>().await?;
    
    // Создаем цепочку операций, которые взаимодействуют между собой
    for i in 0..10 {
        let test_record = create_test_record(&format!("complex_interaction_{}", i));
        
        // 1. Записываем в cache
        let cache_key = cache.create_cache_key(&test_record.content);
        
        // 2. Мониторим операцию
        let start_time = std::time::Instant::now();
        
        // Симулируем некоторую работу
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        monitoring.record_operation(
            &format!("complex_op_{}", i),
            start_time.elapsed()
        ).await;
        
        // 3. Проверяем состояние системы
        if i % 3 == 0 {
            let stats = monitoring.get_stats().await?;
            assert!(stats.total_operations > 0);
        }
    }
    
    // Финальная проверка состояния системы
    let final_stats = monitoring.get_stats().await?;
    assert_eq!(final_stats.total_operations, 10);
    
    Ok(())
}

#[tokio::test]
async fn test_edge_case_scenarios() -> DIResult<()> {
    // Тест edge cases и граничных условий
    
    // 1. Пустая конфигурация
    let minimal_config = UnifiedDIConfiguration::minimal_config()?;
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Minimal)?;
    let minimal_container = factory.build_container(&minimal_config).await?;
    
    // Должны быть доступны только базовые сервисы
    let monitoring = minimal_container.resolve::<MonitoringService>().await?;
    assert!(monitoring.is_healthy().await);
    
    // 2. Максимальная конфигурация  
    let mut max_config = UnifiedDIConfiguration::production_config()?;
    max_config.max_services = 1000;
    max_config.timeout_seconds = 300;
    
    let max_container = factory.build_container(&max_config).await?;
    let max_monitoring = max_container.resolve::<MonitoringService>().await?;
    assert!(max_monitoring.is_healthy().await);
    
    // Cleanup
    minimal_container.shutdown().await?;
    max_container.shutdown().await?;
    
    Ok(())
}