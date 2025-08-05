use anyhow::Result;
use memory::{DIContainer, DIMemoryService, default_config};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_async_di_creation() -> Result<()> {
    let config = default_config()?;
    let service = DIMemoryService::new(config).await?;
    
    assert!(service.di_container().is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_async_factory_pattern() -> Result<()> {
    let container = Arc::new(DIContainer::new());
    
    // Проверка что async фабрика работает корректно
    container.register_factory::<String>(move || {
        Box::new(move || Ok(Arc::new("test".to_string())))
    });
    
    let result = container.resolve::<String>()?;
    assert_eq!(*result, "test");
    
    Ok(())
}

#[tokio::test]
async fn test_no_runtime_conflict() -> Result<()> {
    // Тест что мы можем создать сервис внутри async контекста
    let handle = tokio::spawn(async {
        let config = default_config().unwrap();
        DIMemoryService::new(config).await
    });
    
    let result = timeout(Duration::from_secs(5), handle).await??;
    assert!(result.is_ok());
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_di_creation() -> Result<()> {
    // Создаём несколько сервисов параллельно
    let mut handles = vec![];
    
    for i in 0..5 {
        let handle = tokio::spawn(async move {
            let config = default_config().unwrap();
            let service = DIMemoryService::new(config).await.unwrap();
            (i, service.di_container().is_some())
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let (idx, has_container) = handle.await?;
        assert!(has_container, "Service {} should have DI container", idx);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_lazy_initialization() -> Result<()> {
    let config = default_config()?;
    let service = DIMemoryService::new(config).await?;
    
    // Первое обращение инициализирует кэш
    let start = std::time::Instant::now();
    let _ = service.search("test", Default::default()).await?;
    let first_call = start.elapsed();
    
    // Второе обращение должно быть быстрее
    let start = std::time::Instant::now();
    let _ = service.search("test", Default::default()).await?;
    let second_call = start.elapsed();
    
    // Второй вызов должен быть минимум в 2 раза быстрее
    assert!(second_call < first_call / 2);
    
    Ok(())
}

#[tokio::test]
async fn test_graceful_fallback() -> Result<()> {
    let container = Arc::new(DIContainer::new());
    
    // Не регистрируем компонент
    let optional = container.try_resolve::<String>();
    assert!(optional.is_none());
    
    // Регистрируем и проверяем что теперь работает
    container.register_factory::<String>(move || {
        Box::new(move || Ok(Arc::new("found".to_string())))
    });
    
    let optional = container.try_resolve::<String>();
    assert!(optional.is_some());
    assert_eq!(*optional.unwrap(), "found");
    
    Ok(())
}

#[tokio::test]
async fn test_di_performance_metrics() -> Result<()> {
    let container = Arc::new(DIContainer::new());
    
    // Регистрируем несколько компонентов
    for i in 0..10 {
        let value = format!("component_{}", i);
        container.register_factory::<String>(move || {
            let v = value.clone();
            Box::new(move || Ok(Arc::new(v.clone())))
        });
    }
    
    let stats = container.stats();
    assert_eq!(stats.total_registrations, 10);
    
    // Резолвим компоненты
    for _ in 0..5 {
        let _ = container.resolve::<String>()?;
    }
    
    let metrics = container.performance_metrics();
    assert_eq!(metrics.total_resolutions, 5);
    assert!(metrics.avg_resolution_time_ns > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_async_component_initialization() -> Result<()> {
    use memory::MemoryDIConfigurator;
    
    let container = Arc::new(DIContainer::new());
    let config = default_config()?;
    
    // Синхронная конфигурация
    MemoryDIConfigurator::configure_full(container.clone(), config.clone())?;
    
    // Асинхронная инициализация
    MemoryDIConfigurator::configure_async_components(container.clone(), config).await?;
    
    // Проверяем что все компоненты зарегистрированы
    let stats = container.stats();
    assert!(stats.total_registrations > 5); // Должно быть минимум 5 компонентов
    
    Ok(())
}

#[tokio::test] 
async fn test_error_handling() -> Result<()> {
    let container = Arc::new(DIContainer::new());
    
    // Попытка резолвить незарегистрированный тип
    let result = container.resolve::<i32>();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not registered"));
    
    Ok(())
}

#[tokio::test]
async fn test_thread_safety() -> Result<()> {
    let container = Arc::new(DIContainer::new());
    
    // Регистрация из разных потоков
    let container1 = container.clone();
    let handle1 = tokio::spawn(async move {
        container1.register_factory::<String>(move || {
            Box::new(move || Ok(Arc::new("thread1".to_string())))
        });
    });
    
    let container2 = container.clone();
    let handle2 = tokio::spawn(async move {
        container2.register_factory::<i32>(move || {
            Box::new(move || Ok(Arc::new(42)))
        });
    });
    
    handle1.await?;
    handle2.await?;
    
    // Проверяем что оба типа зарегистрированы
    assert!(container.resolve::<String>().is_ok());
    assert!(container.resolve::<i32>().is_ok());
    
    Ok(())
}