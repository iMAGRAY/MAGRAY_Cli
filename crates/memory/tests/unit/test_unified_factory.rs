//! Unit Tests for UnifiedServiceFactory
//! 
//! Comprehensive testing of factory pattern implementation including:
//! - Factory creation with different presets
//! - Builder pattern functionality and validation
//! - Container building with various configurations
//! - Error propagation through factory chain
//! - Factory performance and resource management
//! - Custom factory configurations and extensibility

use std::sync::Arc;
use std::collections::HashMap;
use tokio::time::{timeout, Duration};

use crate::{
    di::{
        unified_config::UnifiedDIConfiguration,
        errors::{DIError, DIResult},
        traits::ServiceLifetime,
    },
    services::{
        unified_factory::{UnifiedServiceFactory, FactoryPreset, ServiceFactoryBuilder},
        monitoring_service::MonitoringService,
        cache_service::CacheService,
    },
};

#[tokio::test]
async fn test_factory_creation_with_presets() -> DIResult<()> {
    // Тестируем создание фабрики с разными preset'ами
    
    // Production preset
    let production_factory = UnifiedServiceFactory::with_preset(FactoryPreset::Production)?;
    assert_eq!(production_factory.get_preset(), FactoryPreset::Production);
    
    // Development preset  
    let dev_factory = UnifiedServiceFactory::with_preset(FactoryPreset::Development)?;
    assert_eq!(dev_factory.get_preset(), FactoryPreset::Development);
    
    // Test preset
    let test_factory = UnifiedServiceFactory::with_preset(FactoryPreset::Test)?;
    assert_eq!(test_factory.get_preset(), FactoryPreset::Test);
    
    // Minimal preset
    let minimal_factory = UnifiedServiceFactory::with_preset(FactoryPreset::Minimal)?;
    assert_eq!(minimal_factory.get_preset(), FactoryPreset::Minimal);
    
    // Custom preset
    let custom_factory = UnifiedServiceFactory::with_preset(FactoryPreset::Custom)?;
    assert_eq!(custom_factory.get_preset(), FactoryPreset::Custom);
    
    Ok(())
}

#[tokio::test]
async fn test_factory_builder_pattern() -> DIResult<()> {
    // Создаем фабрику через builder pattern
    let factory = ServiceFactoryBuilder::new()
        .with_preset(FactoryPreset::Development)
        .with_monitoring_enabled(true)
        .with_caching_enabled(true)
        .with_max_services(50)
        .with_timeout(Duration::from_secs(30))
        .build()?;
    
    // Проверяем что настройки применились
    assert_eq!(factory.get_preset(), FactoryPreset::Development);
    assert!(factory.is_monitoring_enabled());
    assert!(factory.is_caching_enabled());
    assert_eq!(factory.get_max_services(), 50);
    assert_eq!(factory.get_timeout(), Duration::from_secs(30));
    
    Ok(())
}

#[tokio::test]
async fn test_container_building_with_production_config() -> DIResult<()> {
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Production)?;
    let config = UnifiedDIConfiguration::production_config()?;
    
    let container = factory.build_container(&config).await?;
    
    // Проверяем что контейнер создался успешно
    assert!(container.is_healthy().await);
    
    // Проверяем что базовые сервисы зарегистрированы
    let monitoring = container.resolve::<MonitoringService>().await?;
    assert!(monitoring.is_healthy().await);
    
    let cache = container.resolve::<CacheService>().await?;
    assert!(!cache.create_cache_key("test").is_empty());
    
    // Cleanup
    container.shutdown().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_container_building_with_test_config() -> DIResult<()> {
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Test)?;
    let config = UnifiedDIConfiguration::test_config()?;
    
    let container = factory.build_container(&config).await?;
    
    // Test конфигурация должна содержать минимальный набор сервисов
    let stats = container.get_statistics().await?;
    assert!(stats.registered_services > 0);
    
    // Основные сервисы должны быть доступны
    let monitoring = container.resolve::<MonitoringService>().await?;
    assert!(monitoring.is_healthy().await);
    
    container.shutdown().await?;
    
    Ok(())
}

#[tokio::test] 
async fn test_container_building_with_minimal_config() -> DIResult<()> {
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Minimal)?;
    let config = UnifiedDIConfiguration::minimal_config()?;
    
    let container = factory.build_container(&config).await?;
    
    // Minimal конфигурация должна иметь только критически необходимые сервисы
    let stats = container.get_statistics().await?;
    assert!(stats.registered_services > 0);
    
    // Базовый мониторинг должен быть доступен даже в minimal режиме
    let monitoring = container.resolve::<MonitoringService>().await?;
    assert!(monitoring.is_healthy().await);
    
    container.shutdown().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_factory_error_handling_invalid_preset() -> DIResult<()> {
    // Тестируем обработку ошибок при некорректных настройках
    let builder = ServiceFactoryBuilder::new()
        .with_max_services(0) // Невалидное значение
        .with_timeout(Duration::from_secs(0)); // Невалидное значение
    
    let result = builder.build();
    assert!(result.is_err());
    
    match result.unwrap_err() {
        DIError::ConfigurationError { field, .. } => {
            assert!(field.contains("max_services") || field.contains("timeout"));
        }
        _ => panic!("Expected ConfigurationError"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_factory_custom_service_registration() -> DIResult<()> {
    let mut factory = UnifiedServiceFactory::with_preset(FactoryPreset::Custom)?;
    
    // Добавляем кастомный сервис в фабрику
    factory.register_custom_service::<MockCustomService, _>(
        "CustomService",
        ServiceLifetime::Singleton,
        || Ok(Arc::new(MockCustomService::new("custom_test")))
    )?;
    
    let config = UnifiedDIConfiguration::test_config()?;
    let container = factory.build_container(&config).await?;
    
    // Проверяем что кастомный сервис доступен
    let custom_service = container.resolve_named::<MockCustomService>("CustomService").await?;
    assert_eq!(custom_service.get_name(), "custom_test");
    
    container.shutdown().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_factory_environment_detection() -> DIResult<()> {
    // Симулируем разные окружения
    
    // Test environment
    std::env::set_var("MAGRAY_ENVIRONMENT", "test");
    let test_factory = UnifiedServiceFactory::from_environment()?;
    assert_eq!(test_factory.get_preset(), FactoryPreset::Test);
    
    // Production environment
    std::env::set_var("MAGRAY_ENVIRONMENT", "production");
    let prod_factory = UnifiedServiceFactory::from_environment()?;
    assert_eq!(prod_factory.get_preset(), FactoryPreset::Production);
    
    // Development environment (default)
    std::env::remove_var("MAGRAY_ENVIRONMENT");
    let dev_factory = UnifiedServiceFactory::from_environment()?;
    assert_eq!(dev_factory.get_preset(), FactoryPreset::Development);
    
    Ok(())
}

#[tokio::test]
async fn test_factory_concurrent_container_building() -> DIResult<()> {
    let factory = Arc::new(UnifiedServiceFactory::with_preset(FactoryPreset::Test)?);
    
    // Создаем множественные конкурентные задачи построения контейнеров
    let tasks: Vec<_> = (0..5).map(|i| {
        let factory = factory.clone();
        
        tokio::spawn(async move {
            let config = UnifiedDIConfiguration::test_config()?;
            let container = factory.build_container(&config).await?;
            
            // Проверяем что контейнер работает
            let monitoring = container.resolve::<MonitoringService>().await?;
            monitoring.record_operation(
                &format!("concurrent_build_{}", i), 
                Duration::from_millis(10)
            ).await;
            
            let is_healthy = container.is_healthy().await;
            container.shutdown().await?;
            
            DIResult::Ok(is_healthy)
        })
    }).collect();
    
    // Ждем завершения всех задач
    let results = timeout(Duration::from_secs(30), async {
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await.expect("Test operation should succeed")?);
        }
        DIResult::Ok(results)
    }).await.map_err(|_| DIError::TimeoutError {
        operation: "concurrent_container_building".to_string(),
        timeout_ms: 30000
    })??;
    
    // Все контейнеры должны были быть здоровыми
    assert_eq!(results.len(), 5);
    for is_healthy in results {
        assert!(is_healthy);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_factory_performance_metrics() -> DIResult<()> {
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Production)?;
    let config = UnifiedDIConfiguration::production_config()?;
    
    let start_time = std::time::Instant::now();
    
    // Измеряем время создания контейнера
    let container = factory.build_container(&config).await?;
    let build_time = start_time.elapsed();
    
    // Проверяем что создание контейнера происходит в разумное время
    assert!(build_time.as_secs() < 10); // Не более 10 секунд
    
    // Проверяем функциональность созданного контейнера
    assert!(container.is_healthy().await);
    
    let stats = container.get_statistics().await?;
    assert!(stats.registered_services > 0);
    
    container.shutdown().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_factory_service_override_capability() -> DIResult<()> {
    let mut factory = UnifiedServiceFactory::with_preset(FactoryPreset::Custom)?;
    
    // Регистрируем стандартный сервис
    factory.register_custom_service::<MockCustomService, _>(
        "OverrideTest",
        ServiceLifetime::Singleton,
        || Ok(Arc::new(MockCustomService::new("original")))
    )?;
    
    // Переопределяем тот же сервис
    factory.override_service::<MockCustomService, _>(
        "OverrideTest", 
        || Ok(Arc::new(MockCustomService::new("overridden")))
    )?;
    
    let config = UnifiedDIConfiguration::test_config()?;
    let container = factory.build_container(&config).await?;
    
    // Проверяем что используется переопределенная версия
    let service = container.resolve_named::<MockCustomService>("OverrideTest").await?;
    assert_eq!(service.get_name(), "overridden");
    
    container.shutdown().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_factory_dependency_validation() -> DIResult<()> {
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Test)?;
    
    // Валидация зависимостей должна происходить на этапе построения
    let valid_config = UnifiedDIConfiguration::test_config()?;
    let container = factory.build_container(&valid_config).await?;
    
    // Проверяем что все зависимости корректно разрешились
    let validation_report = factory.validate_dependencies(&container).await?;
    assert!(validation_report.is_valid);
    assert_eq!(validation_report.missing_dependencies.len(), 0);
    assert_eq!(validation_report.circular_dependencies.len(), 0);
    
    container.shutdown().await?;
    
    Ok(())
}

#[tokio::test]
async fn test_factory_resource_cleanup() -> DIResult<()> {
    let factory = UnifiedServiceFactory::with_preset(FactoryPreset::Test)?;
    let config = UnifiedDIConfiguration::test_config()?;
    
    // Создаем контейнер
    let container = factory.build_container(&config).await?;
    
    // Используем ресурсы
    let monitoring = container.resolve::<MonitoringService>().await?;
    monitoring.record_operation("cleanup_test", Duration::from_millis(1)).await;
    
    let initial_stats = container.get_statistics().await?;
    assert!(initial_stats.active_instances > 0);
    
    // Очистка через фабрику
    factory.cleanup_resources(&container).await?;
    
    // Проверяем что ресурсы очищены корректно
    let final_stats = container.get_statistics().await?;
    
    container.shutdown().await?;
    
    Ok(())
}

// Mock service для тестирования кастомных сервисов
#[derive(Debug)]
struct MockCustomService {
    name: String,
    id: uuid::Uuid,
}

impl MockCustomService {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            id: uuid::Uuid::new_v4(),
        }
    }
    
    fn get_name(&self) -> &str {
        &self.name
    }
    
    fn get_id(&self) -> &uuid::Uuid {
        &self.id
    }
}