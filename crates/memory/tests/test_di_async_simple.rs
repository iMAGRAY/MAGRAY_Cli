use anyhow::Result;
use memory::{DIContainer, DIMemoryService, default_config};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_basic_async_di_creation() -> Result<()> {
    // Простой тест создания DI сервиса
    let config = default_config()?;
    let service = DIMemoryService::new(config).await?;
    
    // Проверяем что сервис создался
    assert!(service.di_container().is_some());
    
    Ok(())
}

#[tokio::test]
async fn test_di_container_factory() -> Result<()> {
    // Тест фабрики в DI контейнере
    let container = Arc::new(DIContainer::new());
    
    // Регистрируем простую фабрику
    container.register_factory::<String>(move || {
        Box::new(move || Ok(Arc::new("test value".to_string())))
    });
    
    // Резолвим значение
    let value = container.resolve::<String>()?;
    assert_eq!(*value, "test value");
    
    Ok(())
}

#[tokio::test]
async fn test_no_runtime_conflict_in_async() -> Result<()> {
    // Проверяем что можем создать сервис внутри async контекста без конфликта runtime
    let handle = tokio::spawn(async {
        let config = default_config().unwrap();
        DIMemoryService::new(config).await
    });
    
    // Ждём с таймаутом
    let result = timeout(Duration::from_secs(10), handle).await??;
    assert!(result.is_ok(), "Service creation failed: {:?}", result.err());
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_services_concurrent() -> Result<()> {
    // Создаём несколько сервисов параллельно
    let mut handles = vec![];
    
    for i in 0..3 {
        let handle = tokio::spawn(async move {
            let config = default_config().unwrap();
            match DIMemoryService::new(config).await {
                Ok(service) => Ok((i, service.di_container().is_some())),
                Err(e) => Err(anyhow::anyhow!("Service {} failed: {}", i, e))
            }
        });
        handles.push(handle);
    }
    
    // Ждём все handles
    for handle in handles {
        let (idx, has_container) = handle.await??;
        assert!(has_container, "Service {} should have DI container", idx);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_graceful_fallback_in_di() -> Result<()> {
    let container = Arc::new(DIContainer::new());
    
    // try_resolve для незарегистрированного типа должен вернуть None
    let optional = container.try_resolve::<i64>();
    assert!(optional.is_none());
    
    // Регистрируем тип
    container.register_factory::<i64>(move || {
        Box::new(move || Ok(Arc::new(42i64)))
    });
    
    // Теперь должен найти
    let optional = container.try_resolve::<i64>();
    assert!(optional.is_some());
    assert_eq!(*optional.unwrap(), 42);
    
    Ok(())
}

#[tokio::test]
async fn test_di_performance_metrics_basic() -> Result<()> {
    let container = Arc::new(DIContainer::new());
    
    // Регистрируем компоненты
    for i in 0..5 {
        let value = format!("component_{}", i);
        container.register_factory::<String>(move || {
            let v = value.clone();
            Box::new(move || Ok(Arc::new(v.clone())))
        });
    }
    
    // Проверяем статистику
    let stats = container.stats();
    assert_eq!(stats.total_registrations, 5);
    
    // Резолвим несколько раз
    for _ in 0..3 {
        let _ = container.resolve::<String>()?;
    }
    
    // Проверяем метрики
    let metrics = container.performance_metrics();
    assert_eq!(metrics.total_resolutions, 3);
    assert!(metrics.avg_resolution_time_ns > 0);
    
    Ok(())
}

#[tokio::test]
async fn test_basic_memory_operations() -> Result<()> {
    let config = default_config()?;
    let service = DIMemoryService::new(config).await?;
    
    // Добавляем запись
    let embedding = vec![0.1f32; 1024];
    service.add(
        "test document",
        embedding.clone(),
        memory::Layer::Interact,
        None
    ).await?;
    
    // Ищем
    let results = service.search("test", Default::default()).await?;
    assert!(!results.is_empty());
    
    // Проверяем статистику
    let stats = service.stats().await?;
    assert_eq!(stats.total_entries, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling_in_di() -> Result<()> {
    let container = Arc::new(DIContainer::new());
    
    // Попытка резолвить незарегистрированный тип
    let result = container.resolve::<Vec<u8>>();
    assert!(result.is_err());
    
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("not registered"));
    
    Ok(())
}

#[tokio::test]
async fn test_thread_safety_of_di() -> Result<()> {
    let container = Arc::new(DIContainer::new());
    
    // Регистрируем из разных задач
    let c1 = container.clone();
    let h1 = tokio::spawn(async move {
        c1.register_factory::<String>(move || {
            Box::new(move || Ok(Arc::new("from_task1".to_string())))
        });
    });
    
    let c2 = container.clone();
    let h2 = tokio::spawn(async move {
        c2.register_factory::<i32>(move || {
            Box::new(move || Ok(Arc::new(123i32)))
        });
    });
    
    h1.await?;
    h2.await?;
    
    // Проверяем что оба типа доступны
    let string_val = container.resolve::<String>()?;
    let int_val = container.resolve::<i32>()?;
    
    assert_eq!(*string_val, "from_task1");
    assert_eq!(*int_val, 123);
    
    Ok(())
}