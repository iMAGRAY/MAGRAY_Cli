//! Unit Tests for UnifiedDIContainer
//! 
//! Isolated testing of container functionality including:
//! - Service registration and resolution
//! - Lifetime management (Singleton, Transient, Scoped) 
//! - Dependency injection and circular dependency detection
//! - Container statistics and health monitoring
//! - Thread safety and concurrent access
//! - Error handling and recovery scenarios

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{timeout, Duration};

use crate::{
    di::{
        unified_container::UnifiedDIContainer,
        unified_config::UnifiedDIConfiguration,
        errors::{DIError, DIResult},
        traits::{DIResolver, ServiceLifetime},
    },
    services::{
        monitoring_service::MonitoringService,
        cache_service::CacheService,
    },
};

#[derive(Debug, Clone)]
struct MockSimpleService {
    id: String,
    creation_count: Arc<AtomicUsize>,
}

impl MockSimpleService {
    fn new(id: String) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        COUNTER.fetch_add(1, Ordering::SeqCst);
        
        Self {
            id,
            creation_count: Arc::new(AtomicUsize::new(COUNTER.load(Ordering::SeqCst))),
        }
    }
    
    fn get_id(&self) -> &str {
        &self.id
    }
    
    fn get_creation_count(&self) -> usize {
        self.creation_count.load(Ordering::SeqCst)
    }
}

#[derive(Debug)]
struct MockDependentService {
    dependency: Arc<MockSimpleService>,
    instance_id: String,
}

impl MockDependentService {
    fn new(dependency: Arc<MockSimpleService>) -> Self {
        Self {
            dependency,
            instance_id: uuid::Uuid::new_v4().to_string(),
        }
    }
    
    fn get_dependency_id(&self) -> &str {
        self.dependency.get_id()
    }
    
    fn get_instance_id(&self) -> &str {
        &self.instance_id
    }
}

// Test container creation and basic functionality
#[tokio::test]
async fn test_container_creation_and_initialization() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let container = UnifiedDIContainer::new(config).await?;
    
    // Проверяем что контейнер создался правильно
    assert!(!container.is_shutting_down());
    
    // Проверяем базовые статистики
    let stats = container.get_statistics().await?;
    assert_eq!(stats.registered_services, 0);
    assert_eq!(stats.active_instances, 0);
    
    Ok(())
}

#[tokio::test]
async fn test_service_registration_and_resolution() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let mut container = UnifiedDIContainer::new(config).await?;
    
    // Регистрируем простой сервис
    let service_factory = || Ok(Arc::new(MockSimpleService::new("test_service_1".to_string())));
    container.register_singleton::<MockSimpleService, _>("MockSimpleService", service_factory)?;
    
    // Проверяем что сервис зарегистрирован
    let stats = container.get_statistics().await?;
    assert_eq!(stats.registered_services, 1);
    
    // Резолвим сервис
    let service = container.resolve::<MockSimpleService>().await?;
    assert_eq!(service.get_id(), "test_service_1");
    
    // Проверяем что инстанс создался
    let updated_stats = container.get_statistics().await?;
    assert_eq!(updated_stats.active_instances, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_singleton_lifetime_behavior() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let mut container = UnifiedDIContainer::new(config).await?;
    
    // Регистрируем singleton сервис
    let service_factory = || Ok(Arc::new(MockSimpleService::new("singleton_test".to_string())));
    container.register_singleton::<MockSimpleService, _>("MockSimpleService", service_factory)?;
    
    // Резолвим сервис несколько раз
    let service1 = container.resolve::<MockSimpleService>().await?;
    let service2 = container.resolve::<MockSimpleService>().await?;
    let service3 = container.resolve::<MockSimpleService>().await?;
    
    // Проверяем что это один и тот же инстанс (singleton behavior)
    assert_eq!(service1.get_creation_count(), service2.get_creation_count());
    assert_eq!(service2.get_creation_count(), service3.get_creation_count());
    
    // Должен быть только один активный инстанс
    let stats = container.get_statistics().await?;
    assert_eq!(stats.active_instances, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_transient_lifetime_behavior() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let mut container = UnifiedDIContainer::new(config).await?;
    
    // Регистрируем transient сервис
    let service_factory = || Ok(Arc::new(MockSimpleService::new("transient_test".to_string())));
    container.register_transient::<MockSimpleService, _>("MockSimpleService", service_factory)?;
    
    // Резолвим сервис несколько раз
    let service1 = container.resolve::<MockSimpleService>().await?;
    let service2 = container.resolve::<MockSimpleService>().await?;
    let service3 = container.resolve::<MockSimpleService>().await?;
    
    // Проверяем что это разные инстансы (transient behavior)
    assert_ne!(service1.get_creation_count(), service2.get_creation_count());
    assert_ne!(service2.get_creation_count(), service3.get_creation_count());
    
    Ok(())
}

#[tokio::test]
async fn test_dependency_injection() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let mut container = UnifiedDIContainer::new(config).await?;
    
    // Регистрируем зависимость
    let dependency_factory = || Ok(Arc::new(MockSimpleService::new("dependency_service".to_string())));
    container.register_singleton::<MockSimpleService, _>("MockSimpleService", dependency_factory)?;
    
    // Регистрируем сервис с зависимостью
    let dependent_factory = {
        let container_ref = &container;
        move || {
            let dependency = futures::executor::block_on(container_ref.resolve::<MockSimpleService>())?;
            Ok(Arc::new(MockDependentService::new(dependency)))
        }
    };
    container.register_singleton::<MockDependentService, _>("MockDependentService", dependent_factory)?;
    
    // Резолвим зависимый сервис
    let dependent = container.resolve::<MockDependentService>().await?;
    
    // Проверяем что зависимость была инъектирована правильно
    assert_eq!(dependent.get_dependency_id(), "dependency_service");
    
    Ok(())
}

#[tokio::test]
async fn test_circular_dependency_detection() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let mut container = UnifiedDIContainer::new(config).await?;
    
    // Регистрируем сервисы с циклической зависимостью
    // ServiceA зависит от ServiceB, ServiceB зависит от ServiceA
    
    // Это должно привести к ошибке при resolution
    let result = container.detect_circular_dependencies().await;
    
    // В данном случае циклических зависимостей пока нет
    assert!(result.is_ok());
    
    // TODO: Реализовать более сложный тест с реальными циклическими зависимостями
    // когда такая функциональность будет добавлена в контейнер
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_service_resolution() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let mut container = UnifiedDIContainer::new(config).await?;
    
    // Регистрируем singleton сервис
    let service_factory = || Ok(Arc::new(MockSimpleService::new("concurrent_test".to_string())));
    container.register_singleton::<MockSimpleService, _>("MockSimpleService", service_factory)?;
    
    let container = Arc::new(container);
    let resolution_counter = Arc::new(AtomicUsize::new(0));
    
    // Запускаем множественные конкурентные resolution операции
    let tasks: Vec<_> = (0..20).map(|i| {
        let container = container.clone();
        let counter = resolution_counter.clone();
        
        tokio::spawn(async move {
            let service = container.resolve::<MockSimpleService>().await?;
            counter.fetch_add(1, Ordering::SeqCst);
            
            // Все должны получить один и тот же singleton инстанс
            DIResult::Ok((i, service.get_creation_count()))
        })
    }).collect();
    
    // Ждем завершения всех операций
    let results = timeout(Duration::from_secs(10), async {
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await.expect("Test operation should succeed")?);
        }
        DIResult::Ok(results)
    }).await.map_err(|_| DIError::TimeoutError {
        operation: "concurrent_resolution".to_string(),
        timeout_ms: 10000
    })??;
    
    // Проверяем результаты
    assert_eq!(results.len(), 20);
    assert_eq!(resolution_counter.load(Ordering::SeqCst), 20);
    
    // Все должны иметь один и тот же creation_count (singleton)
    let first_count = results[0].1;
    for (_, count) in results {
        assert_eq!(count, first_count);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_container_statistics_tracking() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let mut container = UnifiedDIContainer::new(config).await?;
    
    // Исходное состояние
    let initial_stats = container.get_statistics().await?;
    assert_eq!(initial_stats.registered_services, 0);
    assert_eq!(initial_stats.active_instances, 0);
    assert_eq!(initial_stats.total_resolutions, 0);
    
    // Регистрируем сервисы
    let service_factory1 = || Ok(Arc::new(MockSimpleService::new("stats_test_1".to_string())));
    let service_factory2 = || Ok(Arc::new(MockSimpleService::new("stats_test_2".to_string())));
    
    container.register_singleton::<MockSimpleService, _>("Service1", service_factory1)?;
    container.register_transient::<MockSimpleService, _>("Service2", service_factory2)?;
    
    let after_registration_stats = container.get_statistics().await?;
    assert_eq!(after_registration_stats.registered_services, 2);
    assert_eq!(after_registration_stats.active_instances, 0); // Еще не резолвились
    
    // Резолвим сервисы
    let _service1 = container.resolve_named::<MockSimpleService>("Service1").await?;
    let _service2_a = container.resolve_named::<MockSimpleService>("Service2").await?;
    let _service2_b = container.resolve_named::<MockSimpleService>("Service2").await?;
    
    let final_stats = container.get_statistics().await?;
    assert_eq!(final_stats.registered_services, 2);
    assert_eq!(final_stats.total_resolutions, 3);
    
    Ok(())
}

#[tokio::test]
async fn test_container_shutdown_and_cleanup() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let mut container = UnifiedDIContainer::new(config).await?;
    
    // Регистрируем и резолвим сервисы
    let service_factory = || Ok(Arc::new(MockSimpleService::new("shutdown_test".to_string())));
    container.register_singleton::<MockSimpleService, _>("TestService", service_factory)?;
    let _service = container.resolve_named::<MockSimpleService>("TestService").await?;
    
    // Проверяем что контейнер активен
    assert!(!container.is_shutting_down());
    
    let before_shutdown_stats = container.get_statistics().await?;
    assert_eq!(before_shutdown_stats.active_instances, 1);
    
    // Выполняем shutdown
    container.shutdown().await?;
    
    // Проверяем состояние после shutdown
    assert!(container.is_shutting_down());
    
    let after_shutdown_stats = container.get_statistics().await?;
    assert_eq!(after_shutdown_stats.active_instances, 0);
    
    // Попытка резолва после shutdown должна вернуть ошибку
    let resolve_result = container.resolve_named::<MockSimpleService>("TestService").await;
    assert!(resolve_result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling_scenarios() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let mut container = UnifiedDIContainer::new(config).await?;
    
    // Тест 1: Resolution несуществующего сервиса
    let non_existent_result = container.resolve::<MockSimpleService>().await;
    assert!(non_existent_result.is_err());
    
    match non_existent_result.unwrap_err() {
        DIError::ServiceNotFound { service_name } => {
            assert!(service_name.contains("MockSimpleService"));
        }
        _ => panic!("Expected ServiceNotFound error"),
    }
    
    // Тест 2: Регистрация сервиса с невалидной фабрикой
    let failing_factory = || Err(DIError::ServiceCreationFailed {
        service_name: "FailingService".to_string(),
        source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Test error")),
    });
    
    container.register_singleton::<MockSimpleService, _>("FailingService", failing_factory)?;
    
    let failing_resolve = container.resolve_named::<MockSimpleService>("FailingService").await;
    assert!(failing_resolve.is_err());
    
    match failing_resolve.unwrap_err() {
        DIError::ServiceCreationFailed { service_name, .. } => {
            assert_eq!(service_name, "FailingService");
        }
        _ => panic!("Expected ServiceCreationFailed error"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_container_health_monitoring() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let mut container = UnifiedDIContainer::new(config).await?;
    
    // Проверяем изначальное здоровое состояние
    assert!(container.is_healthy().await);
    
    // Регистрируем сервис
    let service_factory = || Ok(Arc::new(MockSimpleService::new("health_test".to_string())));
    container.register_singleton::<MockSimpleService, _>("HealthService", service_factory)?;
    
    // Контейнер должен оставаться здоровым
    assert!(container.is_healthy().await);
    
    // Резолвим сервис
    let _service = container.resolve_named::<MockSimpleService>("HealthService").await?;
    assert!(container.is_healthy().await);
    
    // После shutdown контейнер не должен быть healthy
    container.shutdown().await?;
    assert!(!container.is_healthy().await);
    
    Ok(())
}

#[tokio::test]
async fn test_scoped_lifetime_behavior() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let mut container = UnifiedDIContainer::new(config).await?;
    
    // Регистрируем scoped сервис
    let service_factory = || Ok(Arc::new(MockSimpleService::new("scoped_test".to_string())));
    container.register_scoped::<MockSimpleService, _>("ScopedService", service_factory)?;
    
    // Создаем scope
    let scope1 = container.create_scope().await?;
    let scope2 = container.create_scope().await?;
    
    // В разных scope'ах должны быть разные инстансы
    let service1_a = scope1.resolve_named::<MockSimpleService>("ScopedService").await?;
    let service1_b = scope1.resolve_named::<MockSimpleService>("ScopedService").await?;
    let service2_a = scope2.resolve_named::<MockSimpleService>("ScopedService").await?;
    
    // В пределах одного scope - тот же инстанс
    assert_eq!(service1_a.get_creation_count(), service1_b.get_creation_count());
    
    // В разных scope'ах - разные инстансы  
    assert_ne!(service1_a.get_creation_count(), service2_a.get_creation_count());
    
    Ok(())
}

#[tokio::test]
async fn test_container_performance_metrics() -> DIResult<()> {
    let config = UnifiedDIConfiguration::test_config()?;
    let mut container = UnifiedDIContainer::new(config).await?;
    
    // Регистрируем сервис
    let service_factory = || Ok(Arc::new(MockSimpleService::new("perf_test".to_string())));
    container.register_singleton::<MockSimpleService, _>("PerfService", service_factory)?;
    
    let start_time = std::time::Instant::now();
    
    // Выполняем множественные resolution операции для измерения производительности
    for i in 0..100 {
        let _service = container.resolve_named::<MockSimpleService>("PerfService").await?;
        
        // Каждые 10 операций проверяем статистики
        if i % 10 == 0 {
            let stats = container.get_statistics().await?;
            assert!(stats.average_resolution_time_ms >= 0.0);
        }
    }
    
    let total_time = start_time.elapsed();
    let stats = container.get_statistics().await?;
    
    assert_eq!(stats.total_resolutions, 100);
    assert!(stats.average_resolution_time_ms > 0.0);
    assert!(total_time.as_millis() > 0);
    
    Ok(())
}