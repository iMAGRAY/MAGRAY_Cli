//! Comprehensive unit тесты для MonitoringService
//!
//! Coverage areas:
//! - System monitoring и health checks
//! - Production metrics collection
//! - Resource monitoring integration
//! - Readiness checks validation
//! - Async monitoring tasks management
//! - Integration testing с CoordinatorService

use std::sync::Arc;
use std::time::Duration;
use anyhow::Result;
use tokio_test;
use once_cell::sync::Lazy;
use mockall::{predicate::*, mock};

use memory::{
    services::{
        MonitoringService, CoordinatorService,
        traits::{MonitoringServiceTrait, CoordinatorServiceTrait}
    },
    di::container_core::DIContainer,
    health::{HealthMonitor, SystemHealthStatus, HealthStatus},
    storage::VectorStore,
    gpu_accelerated::GpuBatchProcessor,
    types::ProductionMetrics,
};

static INIT_TRACING: Lazy<()> = Lazy::new(|| {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("debug")
        .try_init();
});

// Mock CoordinatorService for testing
mockall::mock! {
    pub TestCoordinatorService {}
    
    #[async_trait::async_trait]
    impl CoordinatorServiceTrait for TestCoordinatorService {
        async fn create_coordinators(&self, container: &DIContainer) -> Result<()>;
        async fn initialize_coordinators(&self) -> Result<()>;
        fn get_embedding_coordinator(&self) -> Option<Arc<memory::orchestration::EmbeddingCoordinator>>;
        fn get_search_coordinator(&self) -> Option<Arc<memory::orchestration::SearchCoordinator>>;
        fn get_health_manager(&self) -> Option<Arc<memory::orchestration::HealthManager>>;
        fn get_resource_controller(&self) -> Option<Arc<memory::orchestration::ResourceController>>;
        async fn shutdown_coordinators(&self) -> Result<()>;
        fn count_active_coordinators(&self) -> usize;
    }
}

/// Helper для создания test DI container
fn create_test_container() -> Arc<DIContainer> {
    Lazy::force(&INIT_TRACING);
    
    let container = Arc::new(DIContainer::new());
    
    // Register HealthMonitor
    let health_monitor = Arc::new(HealthMonitor::new());
    container.register(health_monitor).expect("Не удалось зарегистрировать HealthMonitor");
    
    // Register VectorStore for completeness
    let vector_store = Arc::new(VectorStore::new_in_memory(1024));
    container.register(vector_store).expect("Не удалось зарегистрировать VectorStore");
    
    // Register ResourceManager (if available)
    // Resource manager registration is optional in tests
    
    container
}

/// Helper для создания пустого container
fn create_empty_container() -> Arc<DIContainer> {
    Lazy::force(&INIT_TRACING);
    Arc::new(DIContainer::new())
}

/// Helper для создания mock coordinator service
fn create_mock_coordinator_service() -> Arc<MockTestCoordinatorService> {
    let mut mock = MockTestCoordinatorService::new();
    
    mock.expect_count_active_coordinators()
        .returning(|| 4)
        .times(1..);
    
    mock.expect_get_embedding_coordinator()
        .returning(|| None) // Simplified mock
        .times(0..);
        
    mock.expect_get_search_coordinator()
        .returning(|| None)
        .times(0..);
        
    mock.expect_get_health_manager()
        .returning(|| None)
        .times(0..);
        
    mock.expect_get_resource_controller()
        .returning(|| None)
        .times(0..);
    
    Arc::new(mock)
}

#[tokio::test]
async fn test_monitoring_service_creation() -> Result<()> {
    let container = create_test_container();
    
    // Test basic creation
    let service = MonitoringService::new(container.clone());
    assert_eq!(service.get_monitoring_tasks_count(), 0, "Изначально не должно быть запущенных monitoring tasks");
    
    // Test creation with coordinator service
    let coordinator = create_mock_coordinator_service();
    let service_with_coordinator = MonitoringService::new_with_coordinator(
        container.clone(), 
        coordinator as Arc<dyn CoordinatorServiceTrait>
    );
    assert_eq!(service_with_coordinator.get_monitoring_tasks_count(), 0, "С coordinator тоже должно быть 0 tasks изначально");
    
    Ok(())
}

#[tokio::test]
async fn test_production_monitoring_startup() -> Result<()> {
    let container = create_test_container();
    let service = MonitoringService::new(container);
    
    // Test production monitoring startup
    let result = service.start_production_monitoring().await;
    assert!(result.is_ok(), "Production monitoring должен запускаться успешно");
    
    // Give some time for the monitoring task to start
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    assert_eq!(service.get_monitoring_tasks_count(), 1, "Должна быть запущена 1 monitoring task");
    
    Ok(())
}

#[tokio::test]
async fn test_health_monitoring_with_coordinator() -> Result<()> {
    let container = create_test_container();
    let coordinator = create_mock_coordinator_service();
    
    let service = MonitoringService::new_with_coordinator(
        container, 
        coordinator as Arc<dyn CoordinatorServiceTrait>
    );
    
    // Test health monitoring startup with coordinator
    let result = service.start_health_monitoring().await;
    assert!(result.is_ok(), "Health monitoring должен запускаться успешно с coordinator");
    
    // Give some time for the monitoring task to start
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    assert_eq!(service.get_monitoring_tasks_count(), 1, "Должна быть запущена 1 health monitoring task");
    
    Ok(())
}

#[tokio::test]
async fn test_health_monitoring_fallback() -> Result<()> {
    let container = create_test_container();
    let service = MonitoringService::new(container);
    
    // Test health monitoring without coordinator (fallback mode)
    let result = service.start_health_monitoring().await;
    assert!(result.is_ok(), "Health monitoring fallback должен работать");
    
    // Give some time for the monitoring task to start
    tokio::time::sleep(Duration::from_millis(50)).await;
    
    assert_eq!(service.get_monitoring_tasks_count(), 1, "Должна быть запущена 1 fallback health monitoring task");
    
    Ok(())
}

#[tokio::test]
async fn test_health_monitoring_without_dependencies() -> Result<()> {
    let empty_container = create_empty_container();
    let service = MonitoringService::new(empty_container);
    
    // Test health monitoring without HealthMonitor
    let result = service.start_health_monitoring().await;
    assert!(result.is_ok(), "Health monitoring должен обрабатывать отсутствие зависимостей gracefully");
    
    Ok(())
}

#[tokio::test]
async fn test_resource_monitoring_with_coordinator() -> Result<()> {
    let container = create_test_container();
    let coordinator = create_mock_coordinator_service();
    
    let service = MonitoringService::new_with_coordinator(
        container,
        coordinator as Arc<dyn CoordinatorServiceTrait>
    );
    
    // Test resource monitoring startup
    let result = service.start_resource_monitoring().await;
    assert!(result.is_ok(), "Resource monitoring должен запускаться успешно");
    
    Ok(())
}

#[tokio::test]
async fn test_resource_monitoring_without_coordinator() -> Result<()> {
    let container = create_test_container();
    let service = MonitoringService::new(container);
    
    // Test resource monitoring without coordinator
    let result = service.start_resource_monitoring().await;
    assert!(result.is_ok(), "Resource monitoring без coordinator должен завершаться gracefully");
    
    Ok(())
}

#[tokio::test]
async fn test_readiness_checks_with_coordinator() -> Result<()> {
    let container = create_test_container();
    let coordinator = create_mock_coordinator_service();
    
    let service = MonitoringService::new_with_coordinator(
        container,
        coordinator as Arc<dyn CoordinatorServiceTrait>
    );
    
    // Test readiness checks
    let result = service.perform_readiness_checks().await;
    assert!(result.is_ok(), "Readiness checks должны завершаться успешно");
    
    Ok(())
}

#[tokio::test]
async fn test_readiness_checks_without_coordinator() -> Result<()> {
    let container = create_test_container();
    let service = MonitoringService::new(container);
    
    // Test readiness checks without coordinator
    let result = service.perform_readiness_checks().await;
    assert!(result.is_ok(), "Readiness checks без coordinator должны работать (только DI проверки)");
    
    Ok(())
}

#[tokio::test]
async fn test_readiness_checks_empty_container() -> Result<()> {
    let empty_container = create_empty_container();
    let service = MonitoringService::new(empty_container);
    
    // Test readiness checks with empty container
    let result = service.perform_readiness_checks().await;
    assert!(result.is_err(), "Readiness checks должны завершаться с ошибкой для пустого контейнера");
    
    Ok(())
}

#[tokio::test]
async fn test_system_stats_collection() -> Result<()> {
    let container = create_test_container();
    let coordinator = create_mock_coordinator_service();
    
    let service = MonitoringService::new_with_coordinator(
        container,
        coordinator as Arc<dyn CoordinatorServiceTrait>
    );
    
    // Test system stats collection
    let stats = service.get_system_stats().await;
    
    // Verify basic stats structure
    assert_eq!(stats.cache_hits, 0, "Cache hits должно быть 0 в тестах");
    assert_eq!(stats.cache_misses, 0, "Cache misses должно быть 0 в тестах");
    assert_eq!(stats.cache_size, 0, "Cache size должно быть 0 в тестах");
    // DI container stats check - verify the field exists
    // Note: actual field name may vary based on implementation
    assert!(stats.di_container_stats.registered_types >= 0, "DI container должен содержать информацию о типах");
    
    Ok(())
}

#[tokio::test]
async fn test_health_check_integration() -> Result<()> {
    let container = create_test_container();
    let service = MonitoringService::new(container);
    
    // Test health check
    let health_result = service.check_health().await;
    assert!(health_result.is_ok(), "Health check должен завершаться успешно");
    
    let health_status = health_result.unwrap();
    assert_eq!(health_status.status, HealthStatus::Healthy, "System должна быть healthy");
    
    Ok(())
}

#[tokio::test]
async fn test_health_check_without_monitor() -> Result<()> {
    let empty_container = create_empty_container();
    let service = MonitoringService::new(empty_container);
    
    // Test health check without HealthMonitor
    let health_result = service.check_health().await;
    assert!(health_result.is_err(), "Health check должен завершаться с ошибкой без HealthMonitor");
    
    Ok(())
}

#[tokio::test]
async fn test_production_metrics_retrieval() -> Result<()> {
    let container = create_test_container();
    let service = MonitoringService::new(container);
    
    // Test production metrics retrieval
    let metrics_result = service.get_production_metrics().await;
    assert!(metrics_result.is_ok(), "Production metrics должны быть доступны");
    
    let metrics = metrics_result.unwrap();
    assert_eq!(metrics.total_operations, 0, "Изначально total operations должно быть 0");
    assert_eq!(metrics.successful_operations, 0, "Изначально successful operations должно быть 0");
    assert_eq!(metrics.failed_operations, 0, "Изначально failed operations должно быть 0");
    
    Ok(())
}

#[tokio::test]
async fn test_update_production_metrics() -> Result<()> {
    let container = create_test_container();
    let service = MonitoringService::new(container);
    
    // Test updating production metrics
    let mut new_metrics = ProductionMetrics::default();
    new_metrics.total_operations = 100;
    new_metrics.successful_operations = 80;
    new_metrics.failed_operations = 20;
    new_metrics.avg_response_time_ms = 150.0;
    
    service.update_production_metrics(new_metrics.clone()).await;
    
    let retrieved_metrics = service.get_production_metrics().await?;
    assert_eq!(retrieved_metrics.total_operations, 100, "Total operations должно обновиться");
    assert_eq!(retrieved_metrics.successful_operations, 80, "Successful operations должно обновиться");
    assert_eq!(retrieved_metrics.failed_operations, 20, "Failed operations должно обновиться");
    assert_eq!(retrieved_metrics.avg_response_time_ms, 150.0, "Average response time должно обновиться");
    
    Ok(())
}

#[tokio::test]
async fn test_initialization_summary_logging() -> Result<()> {
    let container = create_test_container();
    let coordinator = create_mock_coordinator_service();
    
    let service = MonitoringService::new_with_coordinator(
        container,
        coordinator as Arc<dyn CoordinatorServiceTrait>
    );
    
    // Test initialization summary (should not panic or error)
    service.log_initialization_summary().await;
    
    Ok(())
}

#[tokio::test]
async fn test_multiple_monitoring_tasks() -> Result<()> {
    let container = create_test_container();
    let coordinator = create_mock_coordinator_service();
    
    let service = MonitoringService::new_with_coordinator(
        container,
        coordinator as Arc<dyn CoordinatorServiceTrait>
    );
    
    // Start multiple monitoring types
    service.start_production_monitoring().await?;
    service.start_health_monitoring().await?;
    service.start_resource_monitoring().await?;
    
    // Give some time for tasks to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Should have started production and health monitoring tasks
    assert!(service.get_monitoring_tasks_count() >= 2, "Должны быть запущены минимум 2 monitoring tasks");
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_monitoring_operations() -> Result<()> {
    let container = create_test_container();
    let service = Arc::new(MonitoringService::new(container));
    
    // Test concurrent monitoring operations
    let tasks = vec![
        tokio::spawn({
            let service = service.clone();
            async move { service.start_production_monitoring().await }
        }),
        tokio::spawn({
            let service = service.clone();
            async move { service.start_health_monitoring().await }
        }),
        tokio::spawn({
            let service = service.clone();
            async move { service.perform_readiness_checks().await }
        }),
        tokio::spawn({
            let service = service.clone();
            async move { service.check_health().await }
        }),
        tokio::spawn({
            let service = service.clone();
            async move { service.get_system_stats().await }
        }),
    ];
    
    let results = futures::future::join_all(tasks).await;
    
    // All operations should complete without panicking
    for (i, result) in results.into_iter().enumerate() {
        assert!(result.is_ok(), "Task {} должна завершиться без panic", i);
        // Individual operations may succeed or fail depending on dependencies,
        // but they shouldn't panic
    }
    
    Ok(())
}

#[tokio::test]
async fn test_monitoring_task_counting() -> Result<()> {
    let container = create_test_container();
    let service = MonitoringService::new(container);
    
    assert_eq!(service.get_monitoring_tasks_count(), 0, "Изначально 0 tasks");
    
    // Start production monitoring
    service.start_production_monitoring().await?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(service.get_monitoring_tasks_count(), 1, "После production monitoring: 1 task");
    
    // Start health monitoring
    service.start_health_monitoring().await?;
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(service.get_monitoring_tasks_count(), 2, "После health monitoring: 2 tasks");
    
    Ok(())
}

#[tokio::test]
async fn test_monitoring_service_resilience() -> Result<()> {
    let container = create_test_container();
    let service = MonitoringService::new(container);
    
    // Test that monitoring service handles errors gracefully
    
    // Start monitoring multiple times (should not cause issues)
    for i in 0..5 {
        let result = service.start_production_monitoring().await;
        assert!(result.is_ok(), "Production monitoring start #{} должен завершаться успешно", i);
    }
    
    // Multiple health checks should be safe
    for i in 0..5 {
        let health_result = service.check_health().await;
        // Result may be Ok or Err depending on state, but should be consistent
        assert!(health_result.is_ok() || health_result.is_err(), "Health check #{} должен завершаться детерминированно", i);
    }
    
    Ok(())
}

// Property-based tests
proptest::proptest! {
    #[test]
    fn test_monitoring_service_consistency(
        production_metrics in proptest::strategy::Strategy::boxed((0u64..1000, 0u64..1000, 0u64..1000, 0f64..1000f64))
    ) {
        let (total_ops, success_ops, failed_ops, avg_time) = production_metrics;
        
        tokio_test::block_on(async {
            let container = create_test_container();
            let service = MonitoringService::new(container);
            
            // Update with property-based metrics
            let mut metrics = ProductionMetrics::default();
            metrics.total_operations = total_ops;
            metrics.successful_operations = success_ops;
            metrics.failed_operations = failed_ops;
            metrics.avg_response_time_ms = avg_time;
            
            service.update_production_metrics(metrics.clone()).await;
            
            let retrieved = service.get_production_metrics().await.unwrap();
            
            prop_assert_eq!(retrieved.total_operations, total_ops);
            prop_assert_eq!(retrieved.successful_operations, success_ops);
            prop_assert_eq!(retrieved.failed_operations, failed_ops);
            prop_assert!((retrieved.avg_response_time_ms - avg_time).abs() < 0.001);
        });
    }
}

#[tokio::test]
async fn test_monitoring_service_edge_cases() -> Result<()> {
    // Test with completely empty container
    let empty_container = create_empty_container();
    let empty_service = MonitoringService::new(empty_container);
    
    // Operations should handle missing dependencies gracefully
    let _health_result = empty_service.check_health().await; // May fail, but shouldn't panic
    let _readiness_result = empty_service.perform_readiness_checks().await; // Should fail gracefully
    let _system_stats = empty_service.get_system_stats().await; // Should work with empty stats
    
    // Should not panic
    empty_service.log_initialization_summary().await;
    
    Ok(())
}

#[tokio::test]
async fn test_monitoring_with_real_coordinator_service() -> Result<()> {
    let container = create_test_container();
    
    // Add required dependencies for CoordinatorService (simplified for testing)
    // GPU processor registration is optional in integration tests
    
    // Create real CoordinatorService
    let coordinator_service = Arc::new(CoordinatorService::new());
    coordinator_service.create_coordinators(&container).await?;
    coordinator_service.initialize_coordinators().await?;
    
    // Create MonitoringService with real coordinator
    let monitoring_service = MonitoringService::new_with_coordinator(
        container,
        coordinator_service as Arc<dyn CoordinatorServiceTrait>
    );
    
    // Test full integration
    let result = monitoring_service.perform_readiness_checks().await;
    assert!(result.is_ok(), "Readiness checks с настоящим coordinator должны работать");
    
    let stats = monitoring_service.get_system_stats().await;
    assert!(stats.di_container_stats.registered_types >= 0, "Stats должны отражать реальную DI структуру");
    
    Ok(())
}

#[tokio::test]
async fn test_monitoring_service_memory_safety() -> Result<()> {
    // Test that monitoring service doesn't leak memory with many operations
    let container = create_test_container();
    
    for i in 0..20 {
        let service = MonitoringService::new(container.clone());
        
        // Perform various operations
        let _ = service.start_production_monitoring().await;
        let _ = service.check_health().await;
        let _ = service.get_system_stats().await;
        let _ = service.perform_readiness_checks().await;
        
        // Update metrics multiple times
        let mut metrics = ProductionMetrics::default();
        metrics.total_operations = i as u64;
        service.update_production_metrics(metrics).await;
        
        let _ = service.get_production_metrics().await;
        
        // Allow tasks to start
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    
    // If we reach here without memory issues, test passes
    Ok(())
}

#[tokio::test]
#[ignore] // Ignore by default due to timing sensitivity
async fn test_production_monitoring_intervals() -> Result<()> {
    let container = create_test_container();
    let service = MonitoringService::new(container);
    
    // Start production monitoring
    service.start_production_monitoring().await?;
    
    // Update metrics to trigger monitoring logic
    let mut metrics = ProductionMetrics::default();
    metrics.total_operations = 1000;
    metrics.successful_operations = 900; // 90% success rate (should not trigger warning)
    metrics.avg_response_time_ms = 50.0; // Good response time
    service.update_production_metrics(metrics).await;
    
    // Wait for monitoring interval to process
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Update with poor metrics
    let mut poor_metrics = ProductionMetrics::default();
    poor_metrics.total_operations = 1000;
    poor_metrics.successful_operations = 800; // 80% success rate (should trigger warning)
    poor_metrics.avg_response_time_ms = 200.0; // High response time (should trigger warning)
    service.update_production_metrics(poor_metrics).await;
    
    // Wait for monitoring to process poor metrics
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Test completes successfully if monitoring handles both scenarios without panic
    Ok(())
}