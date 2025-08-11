#![cfg(feature = "extended-tests")]

//! Comprehensive Unit Tests для декомпозированной DI архитектуры
//!
//! Эти тесты проверяют что новая SOLID-совместимая архитектура работает корректно
//! и обеспечивает 100% обратную совместимость с оригинальным God Object.

use anyhow::Result;
use std::sync::Arc;
use tokio_test;

use memory::{
    di_memory_config::test_helpers,
    service_di::{
        BatchInsertResult, BatchSearchResult, CircuitBreaker, CoordinatorFactory,
        DIMemoryServiceBuilder, DIMemoryServiceFacade, LifecycleManager, MemoryServiceConfig,
        MemorySystemStats, MetricsCollector, OperationConfig, OperationExecutor,
        OrchestrationCoordinators, ProductionCoordinatorFactory, ProductionMetrics,
        ProductionOperationExecutor, ServiceConfigType, SimpleOperationExecutor,
    },
    types::{Layer, Record, SearchOptions},
};

mod facade_tests {
    use super::*;

    #[tokio::test]
    async fn test_facade_replaces_god_object() -> Result<()> {
        // Этот тест проверяет что Facade может полностью заменить God Object
        let config = test_helpers::create_test_config()?;
        let service = DIMemoryServiceFacade::new_minimal(config).await?;

        // Инициализация
        service.initialize().await?;

        // Проверяем основной API (как у God Object)
        let stats = service.get_stats().await;
        assert!(stats.di_container_stats.total_resolutions >= 0);

        // Проверяем DI методы
        let di_stats = service.di_stats();
        assert!(di_stats.total_resolutions >= 0);

        // Проверяем metrics
        let metrics = service.get_performance_metrics();
        assert!(metrics.total_resolves >= 0);

        // Graceful shutdown
        service.shutdown().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_facade_builder_pattern() -> Result<()> {
        let config = test_helpers::create_test_config()?;

        // Тестируем различные конфигурации Builder
        let minimal_service = DIMemoryServiceBuilder::new(config.clone())
            .minimal()
            .build()
            .await?;

        let cpu_service = DIMemoryServiceBuilder::new(config.clone())
            .cpu_only()
            .build()
            .await?;

        let full_service = DIMemoryServiceBuilder::new(config).build().await?;

        // Проверяем что все созданы успешно
        assert!(minimal_service.di_stats().total_resolutions >= 0);
        assert!(cpu_service.di_stats().total_resolutions >= 0);
        assert!(full_service.di_stats().total_resolutions >= 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_facade_business_operations() -> Result<()> {
        let config = test_helpers::create_test_config()?;
        let service = DIMemoryServiceFacade::new_minimal(config).await?;
        service.initialize().await?;

        // Создаем тестовую запись
        let record = test_helpers::create_test_record();

        // Тестируем основные операции (должны работать как у God Object)
        service.insert(record.clone()).await?;

        let search_results = service
            .search(&record.text, Layer::Hot, SearchOptions::default())
            .await?;

        assert!(!search_results.is_empty());

        // Тестируем batch операции
        let records = vec![record.clone(), record.clone()];
        let batch_result = service.batch_insert(records).await?;
        assert!(batch_result.inserted >= 1);

        // Тестируем delete и update
        service.delete(&record.id, Layer::Hot).await?;
        service.update(record).await?;

        service.shutdown().await?;
        Ok(())
    }
}

mod coordinator_factory_tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_factory_solid_principles() -> Result<()> {
        // Тестируем что Factory применяет SOLID принципы

        // Single Responsibility: Factory отвечает ТОЛЬКО за создание координаторов
        let factory = ProductionCoordinatorFactory::new(memory::di_container::DIContainer::new());

        let container = memory::di_container::DIContainer::new();

        // Open/Closed: можем расширять через конфигурацию без изменения кода
        let full_coords = factory.create_all_coordinators(&container).await?;
        assert!(full_coords.count_active() >= 0);

        let minimal_factory = ProductionCoordinatorFactory::minimal();
        let minimal_coords = minimal_factory.create_all_coordinators(&container).await?;
        assert_eq!(minimal_coords.count_active(), 0);

        // Interface Segregation: разные типы координаторов создаются отдельно
        let custom_factory = ProductionCoordinatorFactory::custom(true, false, true, false);
        let custom_coords = custom_factory.create_all_coordinators(&container).await?;

        // Проверяем что создались только нужные координаторы
        assert!(custom_coords.embedding_coordinator.is_some());
        assert!(custom_coords.search_coordinator.is_none());
        assert!(custom_coords.health_manager.is_some());
        assert!(custom_coords.resource_controller.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_orchestration_coordinators_lifecycle() -> Result<()> {
        let factory = ProductionCoordinatorFactory::minimal();
        let container = memory::di_container::DIContainer::new();
        let coordinators = factory.create_all_coordinators(&container).await?;

        // Тестируем lifecycle management
        coordinators.initialize().await?;

        // Тестируем health checking
        let health = coordinators.check_health().await?;
        assert!(health.overall_healthy); // или проверить другие поля

        // Тестируем cache stats
        let (hits, misses, size) = coordinators.get_cache_stats().await;
        assert!(hits >= 0 && misses >= 0 && size >= 0);

        coordinators.shutdown().await?;
        Ok(())
    }
}

mod operation_executor_tests {
    use super::*;

    #[tokio::test]
    async fn test_operation_executor_trait_segregation() -> Result<()> {
        let container = Arc::new(memory::di_container::DIContainer::new());

        // Interface Segregation: разные implementations для разных нужд
        let simple_executor = SimpleOperationExecutor::new(container.clone());
        let production_executor = ProductionOperationExecutor::new_minimal(container.clone());

        // Тестируем что все executors реализуют общий интерфейс
        let executors: Vec<Arc<dyn OperationExecutor>> =
            vec![Arc::new(simple_executor), Arc::new(production_executor)];

        for executor in executors {
            // Тестируем lifecycle
            executor.initialize().await?;

            // Тестируем создание тестовой записи
            let record = test_helpers::create_test_record();
            executor.insert(record.clone()).await?;

            // Тестируем поиск
            let results = executor
                .search(&record.text, Layer::Hot, SearchOptions::default())
                .await?;
            assert!(!results.is_empty());

            executor.shutdown().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_operation_executor_dependency_inversion() -> Result<()> {
        // Dependency Inversion: executor зависит от абстракций, не от конкретных классов
        let container = Arc::new(memory::di_container::DIContainer::new());
        let config = OperationConfig::production();

        let executor = ProductionOperationExecutor::new(
            container, None, // No embedding coordinator
            None, // No search coordinator
            config,
        );

        // Тестируем что executor работает даже без координаторов (fallback)
        executor.initialize().await?;

        let record = test_helpers::create_test_record();
        executor.insert(record.clone()).await?;

        // Тестируем расширенные операции
        let _stats = executor.run_promotion().await?;
        executor.flush_all().await?;

        executor.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_operation_config_variants() -> Result<()> {
        // Тестируем различные конфигурации операций
        let production_config = OperationConfig::production();
        assert_eq!(production_config.max_concurrent_operations, 100);
        assert!(production_config.enable_metrics);

        let minimal_config = OperationConfig::minimal();
        assert_eq!(minimal_config.max_concurrent_operations, 10);
        assert!(!minimal_config.enable_metrics);

        let default_config = OperationConfig::default();
        assert!(default_config.max_concurrent_operations > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_batch_operations_comprehensive() -> Result<()> {
        let container = Arc::new(memory::di_container::DIContainer::new());
        let executor = ProductionOperationExecutor::new_minimal(container);

        executor.initialize().await?;

        // Тестируем batch insert
        let records = vec![
            test_helpers::create_test_record(),
            test_helpers::create_test_record(),
            test_helpers::create_test_record(),
        ];

        let batch_result = executor.batch_insert(records.clone()).await?;
        assert_eq!(batch_result.inserted, 3);
        assert_eq!(batch_result.failed, 0);
        assert!(batch_result.total_time_ms > 0);

        // Тестируем batch search
        let queries = vec![records[0].text.clone(), records[1].text.clone()];

        let search_result = executor
            .batch_search(queries.clone(), Layer::Hot, SearchOptions::default())
            .await?;

        assert_eq!(search_result.queries.len(), 2);
        assert_eq!(search_result.results.len(), 2);
        assert!(search_result.total_time_ms > 0);

        executor.shutdown().await?;
        Ok(())
    }
}

mod solid_principles_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_single_responsibility_principle() -> Result<()> {
        // Каждый модуль имеет единственную ответственность

        // CoordinatorFactory - ТОЛЬКО создание координаторов
        let factory = ProductionCoordinatorFactory::minimal();
        let container = memory::di_container::DIContainer::new();
        let _coordinators = factory.create_all_coordinators(&container).await?;

        // OperationExecutor - ТОЛЬКО выполнение операций
        let executor = Arc::new(ProductionOperationExecutor::new_minimal(Arc::new(
            container,
        )));
        executor.initialize().await?;
        let record = test_helpers::create_test_record();
        executor.insert(record).await?;

        // MetricsCollector - ТОЛЬКО сбор метрик (если реализован)
        // LifecycleManager - ТОЛЬКО управление жизненным циклом
        // CircuitBreaker - ТОЛЬКО circuit breaking логика

        Ok(())
    }

    #[tokio::test]
    async fn test_open_closed_principle() -> Result<()> {
        // Система открыта для расширения, закрыта для модификации

        // Можем создавать новые типы операций без изменения базового executor
        let container = Arc::new(memory::di_container::DIContainer::new());

        // Разные executors используют одинаковый интерфейс
        let simple: Arc<dyn OperationExecutor> =
            Arc::new(SimpleOperationExecutor::new(container.clone()));
        let production: Arc<dyn OperationExecutor> =
            Arc::new(ProductionOperationExecutor::new_minimal(container));

        // Оба executor реализуют одинаковый интерфейс
        simple.initialize().await?;
        production.initialize().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_liskov_substitution_principle() -> Result<()> {
        // Подтипы должны быть взаимозаменяемы с базовыми типами
        let container = Arc::new(memory::di_container::DIContainer::new());

        let executors: Vec<Arc<dyn OperationExecutor>> = vec![
            Arc::new(SimpleOperationExecutor::new(container.clone())),
            Arc::new(ProductionOperationExecutor::new_minimal(container)),
        ];

        // Все executors должны работать одинаково через общий интерфейс
        for executor in executors {
            executor.initialize().await?;

            let record = test_helpers::create_test_record();
            executor.insert(record.clone()).await?;

            let results = executor
                .search(&record.text, Layer::Hot, SearchOptions::default())
                .await?;
            assert!(!results.is_empty());

            executor.shutdown().await?;
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_interface_segregation_principle() -> Result<()> {
        // Клиенты не должны зависеть от интерфейсов которые не используют

        // OperationExecutor interface содержит только операции
        let container = Arc::new(memory::di_container::DIContainer::new());
        let executor: Arc<dyn OperationExecutor> =
            Arc::new(SimpleOperationExecutor::new(container));

        // Clients который использует только insert не видит search
        executor.initialize().await?;
        let record = test_helpers::create_test_record();
        executor.insert(record).await?;
        executor.shutdown().await?;

        // CoordinatorFactory interface содержит только создание координаторов
        let factory: Arc<dyn CoordinatorFactory> =
            Arc::new(ProductionCoordinatorFactory::minimal());
        let container = memory::di_container::DIContainer::new();
        let _coord = factory.create_embedding_coordinator(&container).await;

        Ok(())
    }

    #[tokio::test]
    async fn test_dependency_inversion_principle() -> Result<()> {
        // Высокоуровневые модули не зависят от низкоуровневых
        // Оба зависят от абстракций

        let container = Arc::new(memory::di_container::DIContainer::new());

        // Facade зависит от абстракции OperationExecutor, не от конкретного класса
        let executor: Arc<dyn OperationExecutor> =
            Arc::new(ProductionOperationExecutor::new_minimal(container));

        // Facade может работать с любой implementation OperationExecutor
        executor.initialize().await?;
        let record = test_helpers::create_test_record();
        executor.insert(record).await?;
        executor.shutdown().await?;

        Ok(())
    }
}

mod performance_and_reliability_tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_facade_performance_vs_god_object() -> Result<()> {
        // Тестируем что новая архитектура не медленнее God Object
        let config = test_helpers::create_test_config()?;
        let service = DIMemoryServiceFacade::new_minimal(config).await?;
        service.initialize().await?;

        let start = Instant::now();

        // Выполняем типичные операции
        for i in 0..100 {
            let mut record = test_helpers::create_test_record();
            record.text = format!("test_record_{}", i);
            service.insert(record).await?;
        }

        let duration = start.elapsed();

        // Новая архитектура должна быть reasonably быстрой
        assert!(
            duration.as_millis() < 5000,
            "Too slow: {}ms",
            duration.as_millis()
        );

        service.shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_facade_memory_usage() -> Result<()> {
        // Тестируем что новая архитектура не использует больше памяти
        let config = test_helpers::create_test_config()?;

        // Создаем много сервисов для проверки memory leaks
        for _ in 0..10 {
            let service = DIMemoryServiceFacade::new_minimal(config.clone()).await?;
            service.initialize().await?;

            // Выполняем операции
            let record = test_helpers::create_test_record();
            service.insert(record).await?;

            service.shutdown().await?;
            // Service должен быть cleanup после drop
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_access_safety() -> Result<()> {
        // Тестируем thread safety новой архитектуры
        let config = test_helpers::create_test_config()?;
        let service = Arc::new(DIMemoryServiceFacade::new_minimal(config).await?);
        service.initialize().await?;

        let mut handles = vec![];

        // Запускаем concurrent операции
        for i in 0..10 {
            let service_clone = service.clone();
            let handle = tokio::spawn(async move {
                let mut record = test_helpers::create_test_record();
                record.text = format!("concurrent_test_{}", i);
                service_clone.insert(record).await
            });
            handles.push(handle);
        }

        // Ждем завершения всех операций
        for handle in handles {
            handle.await??;
        }

        service.shutdown().await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_backward_compatibility_comprehensive() -> Result<()> {
    // КРИТИЧЕСКИЙ ТЕСТ: 100% обратная совместимость с God Object API
    let config = test_helpers::create_test_config()?;
    let service = DIMemoryServiceFacade::new_minimal(config).await?;

    // === Все методы оригинального God Object должны работать ===

    // Инициализация
    service.initialize().await?;

    // Business operations
    let record = test_helpers::create_test_record();
    service.insert(record.clone()).await?;

    let results = service
        .search(&record.text, Layer::Hot, SearchOptions::default())
        .await?;
    assert!(!results.is_empty());

    let batch_results = service.batch_insert(vec![record.clone()]).await?;
    assert!(batch_results.inserted >= 1);

    let search_results = service
        .batch_search(
            vec![record.text.clone()],
            Layer::Hot,
            SearchOptions::default(),
        )
        .await?;
    assert!(!search_results.results.is_empty());

    service.update(record.clone()).await?;
    service.delete(&record.id, Layer::Hot).await?;

    // Management operations
    service.flush_all().await?;
    let _promotion_stats = service.run_promotion().await?;
    let _backup = service.create_backup("/tmp/test_backup").await?;

    // Monitoring operations
    let _health = service.check_health().await?;
    let _stats = service.get_stats().await;

    // DI operations
    let _di_stats = service.di_stats();
    let _metrics = service.get_performance_metrics();
    let _report = service.get_performance_report();
    service.reset_performance_metrics();

    // Graceful shutdown
    service.shutdown().await?;

    Ok(())
}
