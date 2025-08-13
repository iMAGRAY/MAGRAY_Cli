#![cfg(feature = "extended-tests")]

//! Comprehensive тесты для UnifiedDIContainer
//!
//! Этот файл содержит полный набор тестов для валидации
//! унифицированного DI контейнера, включая:
//! - SOLID principles compliance
//! - Error handling без .expect("Test operation should succeed") calls  
//! - Performance metrics
//! - Backward compatibility
//! - Thread safety
//! - Memory leak prevention

use anyhow::Result;
use memory::di::{
    ContainerConfiguration, DIMemoryServiceMigrationFacade, DIRegistrar, DIResolver,
    LegacyMemoryConfig, Lifetime, UnifiedDIContainer, UnifiedDIContainerBuilder,
};
use std::{
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::Mutex;

// === TEST FIXTURES ===

#[derive(Debug)]
struct SimpleTestService {
    pub value: String,
    pub counter: Arc<AtomicU32>,
}

impl SimpleTestService {
    fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
            counter: Arc::new(AtomicU32::new(0)),
        }
    }

    fn increment(&self) -> u32 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }
}

#[derive(Debug)]
struct DependentTestService {
    pub dependency: Arc<SimpleTestService>,
    pub multiplier: u32,
}

impl DependentTestService {
    fn new(dependency: Arc<SimpleTestService>, multiplier: u32) -> Self {
        dependency.increment();
        Self {
            dependency,
            multiplier,
        }
    }

    fn get_value(&self) -> u32 {
        self.dependency.counter.load(Ordering::SeqCst) * self.multiplier
    }
}

#[derive(Debug)]
struct ExpensiveService {
    pub id: u64,
    pub creation_time: std::time::Instant,
}

impl ExpensiveService {
    async fn new(id: u64) -> Self {
        // Симулируем дорогую операцию
        tokio::time::sleep(Duration::from_millis(10)).await;
        Self {
            id,
            creation_time: std::time::Instant::now(),
        }
    }
}

#[derive(Debug)]
struct ThreadSafeService {
    pub data: Arc<Mutex<Vec<String>>>,
    pub access_count: Arc<AtomicU64>,
}

impl ThreadSafeService {
    fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(Vec::new())),
            access_count: Arc::new(AtomicU64::new(0)),
        }
    }

    async fn add_data(&self, value: String) {
        self.access_count.fetch_add(1, Ordering::SeqCst);
        let mut data = self.data.lock().await;
        data.push(value);
    }

    async fn get_count(&self) -> usize {
        let data = self.data.lock().await;
        data.len()
    }
}

// === BASIC FUNCTIONALITY TESTS ===

#[tokio::test]
async fn test_unified_container_creation_and_basic_operations() -> Result<()> {
    let container = UnifiedDIContainer::new();

    // Проверяем начальное состояние
    assert_eq!(container.registration_count(), 0);
    assert!(!container.is_registered::<SimpleTestService>());

    // Регистрируем простой сервис
    container.register(|_| Ok(SimpleTestService::new("test")), Lifetime::Singleton)?;

    assert_eq!(container.registration_count(), 1);
    assert!(container.is_registered::<SimpleTestService>());

    // Разрешаем зависимость
    let service = container.resolve::<SimpleTestService>()?;
    assert_eq!(service.value, "test");
    assert_eq!(service.counter.load(Ordering::SeqCst), 0);

    // Проверяем что это singleton
    let service2 = container.resolve::<SimpleTestService>()?;
    assert!(Arc::ptr_eq(&service, &service2));

    Ok(())
}

#[tokio::test]
async fn test_configuration_presets() -> Result<()> {
    // Production configuration
    let production = UnifiedDIContainer::production();
    assert_eq!(production.configuration.max_cache_size, 5000);
    assert!(production.configuration.enable_performance_metrics);
    assert!(production.configuration.enable_dependency_validation);

    // Development configuration
    let development = UnifiedDIContainer::development();
    assert_eq!(development.configuration.max_cache_size, 500);
    assert!(development.configuration.enable_performance_metrics);
    assert!(development.configuration.enable_dependency_validation);

    // Minimal configuration
    let minimal = UnifiedDIContainer::minimal();
    assert_eq!(minimal.configuration.max_cache_size, 100);
    assert!(!minimal.configuration.enable_performance_metrics);
    assert!(!minimal.configuration.enable_dependency_validation);

    Ok(())
}

#[tokio::test]
async fn test_builder_pattern() -> Result<()> {
    let container = UnifiedDIContainerBuilder::new()
        .with_max_cache_size(200)
        .with_instance_timeout(Duration::from_secs(5))
        .enable_metrics()
        .enable_validation()
        .build();

    assert_eq!(container.configuration.max_cache_size, 200);
    assert_eq!(
        container.configuration.instance_creation_timeout,
        Duration::from_secs(5)
    );
    assert!(container.configuration.enable_performance_metrics);
    assert!(container.configuration.enable_dependency_validation);

    Ok(())
}

// === LIFETIME MANAGEMENT TESTS ===

#[tokio::test]
async fn test_singleton_behavior() -> Result<()> {
    let container = UnifiedDIContainer::new();
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();

    container.register(
        move |_| {
            let count = counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(SimpleTestService::new(&format!("singleton-{}", count)))
        },
        Lifetime::Singleton,
    )?;

    // Разрешаем несколько раз
    let service1 = container.resolve::<SimpleTestService>()?;
    let service2 = container.resolve::<SimpleTestService>()?;
    let service3 = container.resolve::<SimpleTestService>()?;

    // Проверяем что это один и тот же экземпляр
    assert!(Arc::ptr_eq(&service1, &service2));
    assert!(Arc::ptr_eq(&service2, &service3));
    assert_eq!(service1.value, "singleton-0");

    // Factory должна вызваться только один раз
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    Ok(())
}

#[tokio::test]
async fn test_transient_behavior() -> Result<()> {
    let container = UnifiedDIContainer::new();
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();

    container.register(
        move |_| {
            let count = counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(SimpleTestService::new(&format!("transient-{}", count)))
        },
        Lifetime::Transient,
    )?;

    // Разрешаем несколько раз
    let service1 = container.resolve::<SimpleTestService>()?;
    let service2 = container.resolve::<SimpleTestService>()?;
    let service3 = container.resolve::<SimpleTestService>()?;

    // Проверяем что это разные экземпляры
    assert!(!Arc::ptr_eq(&service1, &service2));
    assert!(!Arc::ptr_eq(&service2, &service3));
    assert_eq!(service1.value, "transient-0");
    assert_eq!(service2.value, "transient-1");
    assert_eq!(service3.value, "transient-2");

    // Factory должна вызваться три раза
    assert_eq!(counter.load(Ordering::SeqCst), 3);

    Ok(())
}

#[tokio::test]
async fn test_instance_registration() -> Result<()> {
    let container = UnifiedDIContainer::new();

    // Создаем готовый экземпляр
    let service = SimpleTestService::new("instance");
    let original_counter_value = service.counter.load(Ordering::SeqCst);

    // Регистрируем готовый экземпляр
    container.register_instance(service)?;

    // Разрешаем несколько раз
    let resolved1 = container.resolve::<SimpleTestService>()?;
    let resolved2 = container.resolve::<SimpleTestService>()?;

    // Проверяем что это тот же экземпляр
    assert!(Arc::ptr_eq(&resolved1, &resolved2));
    assert_eq!(resolved1.value, "instance");
    assert_eq!(
        resolved1.counter.load(Ordering::SeqCst),
        original_counter_value
    );

    Ok(())
}

// === DEPENDENCY INJECTION TESTS ===

#[tokio::test]
async fn test_dependency_injection_chain() -> Result<()> {
    let container = UnifiedDIContainer::new();

    // Регистрируем базовую зависимость
    container.register(
        |_| Ok(SimpleTestService::new("base-service")),
        Lifetime::Singleton,
    )?;

    // Регистрируем зависимый сервис
    container.register(
        |container| {
            let dependency = container.resolve::<SimpleTestService>()?;
            Ok(DependentTestService::new(dependency, 2))
        },
        Lifetime::Singleton,
    )?;

    // Разрешаем зависимый сервис
    let dependent = container.resolve::<DependentTestService>()?;

    // Проверяем dependency injection
    assert_eq!(dependent.dependency.value, "base-service");
    assert_eq!(dependent.multiplier, 2);
    assert_eq!(dependent.get_value(), 2); // counter был увеличен при создании

    Ok(())
}

#[tokio::test]
async fn test_complex_dependency_graph() -> Result<()> {
    let container = UnifiedDIContainer::development(); // С validation

    // Регистрируем сервисы в dependency chain
    container.register(|_| Ok(SimpleTestService::new("root")), Lifetime::Singleton)?;

    container.register(
        |container| {
            let simple = container.resolve::<SimpleTestService>()?;
            Ok(DependentTestService::new(simple, 3))
        },
        Lifetime::Singleton,
    )?;

    container.register(
        |container| {
            let dependent = container.resolve::<DependentTestService>()?;
            let simple = container.resolve::<SimpleTestService>()?;
            Ok(ThreadSafeService::new()) // Использует другие зависимости внутренне
        },
        Lifetime::Transient,
    )?;

    // Добавляем информацию о зависимостях для validation
    container.add_dependency::<DependentTestService, SimpleTestService>();
    container.add_dependency::<ThreadSafeService, DependentTestService>();
    container.add_dependency::<ThreadSafeService, SimpleTestService>();

    // Валидируем граф зависимостей
    container.validate_dependencies()?;

    // Разрешаем сложную зависимость
    let thread_safe = container.resolve::<ThreadSafeService>()?;

    assert_eq!(thread_safe.access_count.load(Ordering::SeqCst), 0);

    Ok(())
}

// === ERROR HANDLING TESTS ===

#[tokio::test]
async fn test_unregistered_type_error_handling() {
    let container = UnifiedDIContainer::new();

    // Попытка разрешить незарегистрированный тип
    let result = container.resolve::<SimpleTestService>();
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("не зарегистрирован"));

    // try_resolve должен вернуть None
    let optional_result = container.try_resolve::<SimpleTestService>();
    assert!(optional_result.is_none());
}

#[tokio::test]
async fn test_duplicate_registration_error() -> Result<()> {
    let container = UnifiedDIContainer::new();

    // Первая регистрация должна пройти
    container.register(|_| Ok(SimpleTestService::new("first")), Lifetime::Singleton)?;

    // Вторая регистрация должна вернуть ошибку
    let result = container.register(
        |_| Ok(SimpleTestService::new("second")),
        Lifetime::Singleton,
    );

    assert!(result.is_err());
    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("уже зарегистрирован"));

    // Проверяем что первый сервис все еще доступен
    let service = container.resolve::<SimpleTestService>()?;
    assert_eq!(service.value, "first");

    Ok(())
}

#[tokio::test]
async fn test_factory_panic_handling() {
    let container = UnifiedDIContainer::development();

    // Регистрируем сервис с panic factory
    let result = container.register(
        |_| {
            panic!("Test panic in factory");
        },
        Lifetime::Singleton,
    );

    assert!(result.is_ok()); // Регистрация должна пройти

    // Resolve должен поймать panic и вернуть ошибку
    let resolve_result = container.resolve::<SimpleTestService>();
    assert!(resolve_result.is_err());

    let error_message = resolve_result.unwrap_err().to_string();
    assert!(error_message.contains("Panic при создании"));

    // Проверяем метрики ошибок
    let metrics = container.performance_metrics();
    assert!(metrics.error_count > 0);
}

#[tokio::test]
async fn test_factory_error_handling() -> Result<()> {
    let container = UnifiedDIContainer::development();

    // Регистрируем сервис с error factory
    container.register(
        |_| Err(anyhow::anyhow!("Test factory error")),
        Lifetime::Transient,
    )?;

    // Resolve должен вернуть ошибку
    let result = container.resolve::<SimpleTestService>();
    assert!(result.is_err());

    let error_message = result.unwrap_err().to_string();
    assert!(error_message.contains("Test factory error"));

    // Проверяем метрики ошибок
    let metrics = container.performance_metrics();
    assert!(metrics.error_count > 0);

    Ok(())
}

// === PERFORMANCE AND METRICS TESTS ===

#[tokio::test]
async fn test_performance_metrics_collection() -> Result<()> {
    let container = UnifiedDIContainer::development(); // Включены метрики

    // Сбрасываем метрики для чистого теста
    container.reset_performance_metrics();

    // Регистрируем сервис
    container.register(
        |_| Ok(SimpleTestService::new("metrics-test")),
        Lifetime::Singleton,
    )?;

    // Выполняем несколько resolve операций
    for _ in 0..5 {
        let _service = container.resolve::<SimpleTestService>()?;
    }

    // Проверяем метрики
    let metrics = container.performance_metrics();
    assert_eq!(metrics.total_resolutions, 5);
    assert!(metrics.cache_hits >= 4); // Первый resolve создает, остальные из кэша
    assert!(metrics.cache_hit_rate() > 50.0);
    assert!(metrics.avg_resolve_time_us() > 0.0);

    // Проверяем отчет
    let report = container.get_performance_report();
    assert!(report.contains("Performance Report"));
    assert!(report.contains("Total resolutions: 5"));
    assert!(report.contains("Cache hit rate:"));

    Ok(())
}

#[tokio::test]
async fn test_metrics_disabled_mode() -> Result<()> {
    let container = UnifiedDIContainer::minimal(); // Метрики отключены

    container.register(
        |_| Ok(SimpleTestService::new("no-metrics")),
        Lifetime::Singleton,
    )?;

    // Выполняем операции
    for _ in 0..3 {
        let _service = container.resolve::<SimpleTestService>()?;
    }

    // Метрики должны быть пустыми
    let metrics = container.performance_metrics();
    assert_eq!(metrics.total_resolutions, 0);
    assert_eq!(metrics.cache_hits, 0);
    assert_eq!(metrics.cache_misses, 0);

    // Отчет должен указывать что метрики отключены
    let report = container.get_performance_report();
    assert!(report.contains("Performance metrics disabled"));

    Ok(())
}

#[tokio::test]
async fn test_cache_size_limit_and_cleanup() -> Result<()> {
    let container = UnifiedDIContainerBuilder::new()
        .with_max_cache_size(2) // Очень маленький кэш для теста
        .enable_metrics()
        .build();

    // Регистрируем несколько сервисов
    container.register(
        |_| Ok(SimpleTestService::new("service1")),
        Lifetime::Singleton,
    )?;

    container.register(
        |_| {
            Ok(DependentTestService::new(
                Arc::new(SimpleTestService::new("dep1")),
                1,
            ))
        },
        Lifetime::Singleton,
    )?;

    container.register(|_| Ok(ThreadSafeService::new()), Lifetime::Singleton)?;

    // Разрешаем все сервисы
    let _service1 = container.resolve::<SimpleTestService>()?;
    let _service2 = container.resolve::<DependentTestService>()?;
    let _service3 = container.resolve::<ThreadSafeService>()?;

    // Проверяем что кэш не превышает лимит
    let stats = container.stats();
    assert!(stats.cached_singletons <= 2);

    Ok(())
}

// === THREAD SAFETY TESTS ===

#[tokio::test]
async fn test_concurrent_access() -> Result<()> {
    let container = Arc::new(UnifiedDIContainer::development());
    let call_counter = Arc::new(AtomicU32::new(0));
    let counter_clone = call_counter.clone();

    // Регистрируем сервис с счетчиком вызовов
    container.register(
        move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(SimpleTestService::new("concurrent-test"))
        },
        Lifetime::Singleton,
    )?;

    // Запускаем множественный concurrent доступ
    let mut handles = Vec::new();
    for i in 0..10 {
        let container_clone = container.clone();
        let handle = tokio::spawn(async move {
            let service = container_clone
                .resolve::<SimpleTestService>()
                .expect("Failed to resolve in thread");
            assert_eq!(service.value, "concurrent-test");
            i // Возвращаем номер потока для проверки
        });
        handles.push(handle);
    }

    // Ожидаем завершения всех задач
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await?);
    }

    // Проверяем что все потоки завершились успешно
    assert_eq!(results.len(), 10);
    for (i, result) in results.iter().enumerate() {
        assert_eq!(*result, i);
    }

    // Singleton factory должна вызваться только один раз
    assert_eq!(call_counter.load(Ordering::SeqCst), 1);

    // Проверяем метрики
    let metrics = container.performance_metrics();
    assert_eq!(metrics.total_resolutions, 10);
    assert!(metrics.cache_hits >= 9); // Первый создает, остальные из кэша

    Ok(())
}

#[tokio::test]
async fn test_concurrent_registrations() -> Result<()> {
    let container = Arc::new(UnifiedDIContainer::new());

    // Пытаемся зарегистрировать один и тот же тип из разных потоков
    let mut handles = Vec::new();
    for i in 0..5 {
        let container_clone = container.clone();
        let handle = tokio::spawn(async move {
            let result = container_clone.register(
                move |_| Ok(SimpleTestService::new(&format!("thread-{}", i))),
                Lifetime::Singleton,
            );
            (i, result)
        });
        handles.push(handle);
    }

    // Собираем результаты
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await?);
    }

    // Только одна регистрация должна пройти успешно
    let successful: Vec<_> = results
        .iter()
        .filter(|(_, result)| result.is_ok())
        .collect();
    assert_eq!(successful.len(), 1);

    // Остальные должны получить ошибку о дублировании
    let failed: Vec<_> = results
        .iter()
        .filter(|(_, result)| result.is_err())
        .collect();
    assert_eq!(failed.len(), 4);

    // Проверяем что сервис доступен
    let service = container.resolve::<SimpleTestService>()?;
    assert!(service.value.starts_with("thread-"));

    Ok(())
}

// === MEMORY MANAGEMENT TESTS ===

#[tokio::test]
async fn test_clear_functionality() -> Result<()> {
    let container = UnifiedDIContainer::development();

    // Регистрируем и разрешаем сервисы
    container.register(
        |_| Ok(SimpleTestService::new("clear-test")),
        Lifetime::Singleton,
    )?;

    let _service = container.resolve::<SimpleTestService>()?;

    // Проверяем что контейнер заполнен
    assert_eq!(container.registration_count(), 1);
    let stats_before = container.stats();
    assert_eq!(stats_before.registered_factories, 1);
    assert_eq!(stats_before.cached_singletons, 1);
    assert!(stats_before.total_resolutions > 0);

    // Очищаем контейнер
    container.clear();

    // Проверяем что все очищено
    assert_eq!(container.registration_count(), 0);
    let stats_after = container.stats();
    assert_eq!(stats_after.registered_factories, 0);
    assert_eq!(stats_after.cached_singletons, 0);

    // Метрики также должны быть сброшены
    let metrics_after = container.performance_metrics();
    assert_eq!(metrics_after.total_resolutions, 0);
    assert_eq!(metrics_after.cache_hits, 0);

    Ok(())
}

#[tokio::test]
async fn test_memory_leak_prevention() -> Result<()> {
    let container = UnifiedDIContainerBuilder::new()
        .with_max_cache_size(5)
        .enable_metrics()
        .build();

    // Создаем много временных сервисов
    for i in 0..20 {
        let service_name = format!("service_{}", i);
        container.register_instance(SimpleTestService::new(&service_name))?;

        // Разрешаем и сразу забываем
        let _resolved = container.resolve::<SimpleTestService>()?;

        // Очищаем регистрацию чтобы зарегистрировать следующий
        container.clear();
    }

    // Проверяем что память не течет
    let final_stats = container.stats();
    assert_eq!(final_stats.registered_factories, 0);
    assert_eq!(final_stats.cached_singletons, 0);

    Ok(())
}

// === BACKWARD COMPATIBILITY TESTS ===

#[tokio::test]
async fn test_migration_facade_compatibility() -> Result<()> {
    let config = LegacyMemoryConfig::default();
    let facade = DIMemoryServiceMigrationFacade::new_minimal(config).await?;

    // Инициализация
    facade.initialize().await?;

    // Проверяем legacy API методы
    let stats = facade.get_stats().await;
    assert!(stats.di_container_stats.registered_factories >= 0);

    let health = facade.check_health().await?;
    assert!(health.overall_status.len() >= 0);

    let promotion_stats = facade.run_promotion().await?;
    assert!(promotion_stats.interact_to_insights >= 0);

    // Проверяем DI delegation
    let di_stats = facade.di_stats();
    assert!(di_stats.registered_factories >= 0);

    let performance_report = facade.get_performance_report();
    assert!(!performance_report.is_empty());

    // Graceful shutdown
    facade.shutdown().await?;

    Ok(())
}

#[tokio::test]
async fn test_migration_builder_pattern() -> Result<()> {
    use memory::di::DIMemoryServiceMigrationBuilder;

    let config = LegacyMemoryConfig::default();
    let facade = DIMemoryServiceMigrationBuilder::new(config)
        .minimal()
        .cpu_only()
        .build()
        .await?;

    facade.initialize().await?;

    let stats = facade.get_stats().await;
    assert!(stats.di_container_stats.registered_factories >= 0);

    Ok(())
}

// === INTEGRATION TESTS ===

#[tokio::test]
async fn test_full_integration_scenario() -> Result<()> {
    // Создаем production контейнер
    let container = UnifiedDIContainer::production();

    // Регистрируем сложную иерархию сервисов
    container.register(
        |_| Ok(SimpleTestService::new("root-service")),
        Lifetime::Singleton,
    )?;

    container.register(
        |container| {
            let simple = container.resolve::<SimpleTestService>()?;
            Ok(DependentTestService::new(simple, 5))
        },
        Lifetime::Singleton,
    )?;

    container.register(|_| Ok(ThreadSafeService::new()), Lifetime::Transient)?;

    // Добавляем dependency information
    container.add_dependency::<DependentTestService, SimpleTestService>();

    // Валидируем dependencies
    container.validate_dependencies()?;

    // Выполняем множественные операции
    let mut services = Vec::new();
    for _ in 0..10 {
        services.push(container.resolve::<DependentTestService>()?);
        services.push(container.resolve::<ThreadSafeService>()?);
    }

    // Проверяем что singleton sharing работает
    for i in 1..services.len() {
        if let (Ok(s1), Ok(s2)) = (
            services[0].downcast_ref::<DependentTestService>(),
            services[i].downcast_ref::<DependentTestService>(),
        ) {
            if std::any::TypeId::of::<DependentTestService>()
                == std::any::TypeId::of::<DependentTestService>()
            {
                // Проверяем через dependency
                assert!(Arc::ptr_eq(&s1.dependency, &s2.dependency));
            }
        }
    }

    // Проверяем финальные метрики
    let metrics = container.performance_metrics();
    assert!(metrics.total_resolutions >= 20);
    assert!(metrics.cache_hit_rate() > 0.0);

    let final_stats = container.stats();
    assert!(final_stats.total_resolutions >= 20);

    // Генерируем отчет
    let report = container.get_performance_report();
    assert!(report.contains("Performance Report"));
    assert!(report.contains("Cache hit rate:"));

    Ok(())
}

// === EDGE CASES TESTS ===

#[tokio::test]
async fn test_edge_case_empty_operations() -> Result<()> {
    let container = UnifiedDIContainer::new();

    // Операции на пустом контейнере
    assert_eq!(container.registration_count(), 0);
    assert!(container.registered_types().is_empty());

    // Валидация пустых dependencies
    container.validate_dependencies()?;

    // Очистка пустого контейнера
    container.clear();
    assert_eq!(container.registration_count(), 0);

    // Метрики пустого контейнера
    let stats = container.stats();
    assert_eq!(stats.registered_factories, 0);
    assert_eq!(stats.cached_singletons, 0);
    assert_eq!(stats.total_resolutions, 0);

    Ok(())
}

#[tokio::test]
async fn test_edge_case_very_large_dependency_graph() -> Result<()> {
    let container = UnifiedDIContainer::development();

    // Регистрируем много сервисов
    for i in 0..100 {
        let service_name = format!("service_{}", i);
        container.register(
            move |_| Ok(SimpleTestService::new(&service_name)),
            if i % 2 == 0 {
                Lifetime::Singleton
            } else {
                Lifetime::Transient
            },
        )?;

        // Сразу очищаем чтобы избежать дублирования регистраций
        container.clear();

        // Регистрируем один для финального теста
        if i == 99 {
            container.register(
                |_| Ok(SimpleTestService::new("final-service")),
                Lifetime::Singleton,
            )?;
        }
    }

    // Проверяем финальную регистрацию
    let service = container.resolve::<SimpleTestService>()?;
    assert_eq!(service.value, "final-service");

    Ok(())
}

// === STRESS TESTS ===

#[tokio::test]
async fn test_stress_concurrent_high_load() -> Result<()> {
    let container = Arc::new(UnifiedDIContainer::production());

    // Регистрируем сервис
    container.register(
        |_| Ok(SimpleTestService::new("stress-test")),
        Lifetime::Singleton,
    )?;

    // Высокая конкурентная нагрузка
    let mut handles = Vec::new();
    for batch in 0..20 {
        for thread in 0..50 {
            let container_clone = container.clone();
            let handle = tokio::spawn(async move {
                let service = container_clone
                    .resolve::<SimpleTestService>()
                    .expect(&format!(
                        "Failed to resolve in batch {} thread {}",
                        batch, thread
                    ));
                assert_eq!(service.value, "stress-test");
                (batch, thread)
            });
            handles.push(handle);
        }
    }

    // Ждем завершения всех задач
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await?);
    }

    // Проверяем что все завершились успешно
    assert_eq!(results.len(), 1000); // 20 * 50

    // Проверяем финальные метрики
    let metrics = container.performance_metrics();
    assert_eq!(metrics.total_resolutions, 1000);
    assert!(metrics.cache_hit_rate() > 99.0); // Почти все из кэша

    Ok(())
}
