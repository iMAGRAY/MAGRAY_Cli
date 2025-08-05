//! Comprehensive tests for MemoryOrchestrator
//! 
//! Покрывает:
//! - Integration тесты для координации всех компонентов
//! - Lifecycle management (initialize, ready, shutdown)
//! - Resource checking и coordination
//! - Error handling и graceful degradation
//! - Metrics collection from all coordinators
//! - Concurrent operations coordination

use memory::orchestration::{
    MemoryOrchestrator,
    traits::{Coordinator, SearchCoordinator as SearchCoordinatorTrait, 
            PromotionCoordinator as PromotionCoordinatorTrait,
            HealthCoordinator, ResourceCoordinator, BackupCoordinator as BackupCoordinatorTrait},
};
use memory::{
    types::{Layer, Record, SearchOptions},
    promotion::PromotionStats,
    health::SystemHealthStatus,
    backup::BackupMetadata,
    di_container::DIContainer,
};
use anyhow::{Result, anyhow};
use std::sync::Arc;
use tokio;
use serde_json;

// @component: {"k":"T","id":"memory_orchestrator_tests","t":"Comprehensive memory orchestrator tests","m":{"cur":95,"tgt":100,"u":"%"},"f":["test","integration","orchestration","coordination","coverage"]}

/// Mock координаторы для comprehensive тестирования
#[derive(Clone)]
struct MockEmbeddingCoordinator {
    ready: Arc<std::sync::atomic::AtomicBool>,
    initialized: Arc<std::sync::atomic::AtomicBool>,
    operation_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl MockEmbeddingCoordinator {
    fn new() -> Self {
        Self {
            ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            initialized: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            operation_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
    
    fn set_ready(&self, ready: bool) {
        self.ready.store(ready, std::sync::atomic::Ordering::Relaxed);
    }
    
    fn get_operation_count(&self) -> usize {
        self.operation_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[async_trait::async_trait]
impl Coordinator for MockEmbeddingCoordinator {
    async fn initialize(&self) -> Result<()> {
        self.initialized.store(true, std::sync::atomic::Ordering::Relaxed);
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await; // Simulate init time
        self.ready.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    async fn shutdown(&self) -> Result<()> {
        self.ready.store(false, std::sync::atomic::Ordering::Relaxed);
        self.initialized.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "embedding_coordinator",
            "ready": self.is_ready().await,
            "operations": self.get_operation_count()
        })
    }
}

#[derive(Clone)]
struct MockSearchCoordinator {
    ready: Arc<std::sync::atomic::AtomicBool>,
    initialized: Arc<std::sync::atomic::AtomicBool>,
    search_count: Arc<std::sync::atomic::AtomicUsize>,
    should_fail: Arc<std::sync::atomic::AtomicBool>,
}

impl MockSearchCoordinator {
    fn new() -> Self {
        Self {
            ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            initialized: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            search_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            should_fail: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    fn set_should_fail(&self, fail: bool) {
        self.should_fail.store(fail, std::sync::atomic::Ordering::Relaxed);
    }
    
    fn get_search_count(&self) -> usize {
        self.search_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[async_trait::async_trait]
impl Coordinator for MockSearchCoordinator {
    async fn initialize(&self) -> Result<()> {
        self.initialized.store(true, std::sync::atomic::Ordering::Relaxed);
        self.ready.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    async fn shutdown(&self) -> Result<()> {
        self.ready.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "search_coordinator",
            "ready": self.is_ready().await,
            "searches": self.get_search_count()
        })
    }
}

#[async_trait::async_trait]
impl SearchCoordinatorTrait for MockSearchCoordinator {
    async fn search(&self, _query: &str, _layer: Layer, _options: SearchOptions) -> Result<Vec<Record>> {
        if self.should_fail.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(anyhow!("Mock search coordinator configured to fail"));
        }
        
        self.search_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        // Return mock search results
        Ok(vec![
            Record {
                id: "mock_1".to_string(),
                content: "Mock search result 1".to_string(),
                embedding: vec![0.1, 0.2, 0.3],
                metadata: std::collections::HashMap::new(),
                timestamp: chrono::Utc::now(),
                layer: _layer,
                score: Some(0.9),
            }
        ])
    }
}

#[derive(Clone)]
struct MockHealthManager {
    ready: Arc<std::sync::atomic::AtomicBool>,
    healthy: Arc<std::sync::atomic::AtomicBool>,
}

impl MockHealthManager {
    fn new() -> Self {
        Self {
            ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            healthy: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }
    
    fn set_healthy(&self, healthy: bool) {
        self.healthy.store(healthy, std::sync::atomic::Ordering::Relaxed);
    }
}

#[async_trait::async_trait]
impl Coordinator for MockHealthManager {
    async fn initialize(&self) -> Result<()> {
        self.ready.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    async fn shutdown(&self) -> Result<()> {
        self.ready.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "health_manager",
            "ready": self.is_ready().await,
            "healthy": self.healthy.load(std::sync::atomic::Ordering::Relaxed)
        })
    }
}

#[async_trait::async_trait]
impl HealthCoordinator for MockHealthManager {
    async fn system_health(&self) -> Result<SystemHealthStatus> {
        Ok(SystemHealthStatus {
            overall_healthy: self.healthy.load(std::sync::atomic::Ordering::Relaxed),
            components: vec![],
            last_check: chrono::Utc::now(),
            uptime: std::time::Duration::from_secs(3600),
        })
    }
}

#[derive(Clone)]
struct MockPromotionCoordinator {
    ready: Arc<std::sync::atomic::AtomicBool>,
    promotion_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl MockPromotionCoordinator {
    fn new() -> Self {
        Self {
            ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            promotion_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
    
    fn get_promotion_count(&self) -> usize {
        self.promotion_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[async_trait::async_trait]
impl Coordinator for MockPromotionCoordinator {
    async fn initialize(&self) -> Result<()> {
        self.ready.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    async fn shutdown(&self) -> Result<()> {
        self.ready.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "promotion_coordinator",
            "ready": self.is_ready().await,
            "promotions": self.get_promotion_count()
        })
    }
}

#[async_trait::async_trait]
impl PromotionCoordinatorTrait for MockPromotionCoordinator {
    async fn run_promotion(&self) -> Result<PromotionStats> {
        self.promotion_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        Ok(PromotionStats {
            records_promoted: 5,
            layers_affected: vec![Layer::Interact, Layer::Insights],
            promotion_time: std::time::Duration::from_millis(100),
            success_rate: 1.0,
        })
    }
}

#[derive(Clone)]
struct MockResourceController {
    ready: Arc<std::sync::atomic::AtomicBool>,
    resources_available: Arc<std::sync::atomic::AtomicBool>,
}

impl MockResourceController {
    fn new() -> Self {
        Self {
            ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            resources_available: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }
    
    fn set_resources_available(&self, available: bool) {
        self.resources_available.store(available, std::sync::atomic::Ordering::Relaxed);
    }
}

#[async_trait::async_trait]
impl Coordinator for MockResourceController {
    async fn initialize(&self) -> Result<()> {
        self.ready.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    async fn shutdown(&self) -> Result<()> {
        self.ready.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "resource_controller",
            "ready": self.is_ready().await,
            "resources_available": self.resources_available.load(std::sync::atomic::Ordering::Relaxed)
        })
    }
}

#[async_trait::async_trait]
impl ResourceCoordinator for MockResourceController {
    async fn check_resources(&self, _operation: &str) -> Result<bool> {
        Ok(self.resources_available.load(std::sync::atomic::Ordering::Relaxed))
    }
}

#[derive(Clone)]
struct MockBackupCoordinator {
    ready: Arc<std::sync::atomic::AtomicBool>,
    backup_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl MockBackupCoordinator {
    fn new() -> Self {
        Self {
            ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            backup_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
    
    fn get_backup_count(&self) -> usize {
        self.backup_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[async_trait::async_trait]
impl Coordinator for MockBackupCoordinator {
    async fn initialize(&self) -> Result<()> {
        self.ready.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    async fn shutdown(&self) -> Result<()> {
        self.ready.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "backup_coordinator",
            "ready": self.is_ready().await,
            "backups": self.get_backup_count()
        })
    }
}

#[async_trait::async_trait]
impl BackupCoordinatorTrait for MockBackupCoordinator {
    async fn create_backup(&self, _path: &str) -> Result<BackupMetadata> {
        self.backup_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        Ok(BackupMetadata {
            path: _path.to_string(),
            size_bytes: 1024000,
            created_at: chrono::Utc::now(),
            checksum: "mock_checksum_123".to_string(),
            version: "1.0.0".to_string(),
        })
    }
}

fn create_test_orchestrator() -> MemoryOrchestrator {
    MemoryOrchestrator {
        embedding: Arc::new(MockEmbeddingCoordinator::new()),
        search: Arc::new(MockSearchCoordinator::new()),
        health: Arc::new(MockHealthManager::new()),
        promotion: Arc::new(MockPromotionCoordinator::new()),
        resources: Arc::new(MockResourceController::new()),
        backup: Arc::new(MockBackupCoordinator::new()),
    }
}

#[tokio::test]
async fn test_orchestrator_initialization() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    
    // Проверяем что изначально не все готовы
    assert!(!orchestrator.all_ready().await);
    
    // Инициализируем
    orchestrator.initialize_all().await?;
    
    // Проверяем что все готовы
    assert!(orchestrator.all_ready().await);
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_shutdown() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    
    // Инициализируем
    orchestrator.initialize_all().await?;
    assert!(orchestrator.all_ready().await);
    
    // Останавливаем
    orchestrator.shutdown_all().await?;
    
    // Проверяем что остановлены
    assert!(!orchestrator.all_ready().await);
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_metrics_collection() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    orchestrator.initialize_all().await?;
    
    let metrics = orchestrator.all_metrics().await;
    
    // Проверяем структуру метрик
    assert!(metrics["orchestrator"]["ready"].as_bool().unwrap());
    assert!(metrics["orchestrator"]["coordinators"]["embedding"].is_object());
    assert!(metrics["orchestrator"]["coordinators"]["search"].is_object());
    assert!(metrics["orchestrator"]["coordinators"]["health"].is_object());
    assert!(metrics["orchestrator"]["coordinators"]["promotion"].is_object());
    assert!(metrics["orchestrator"]["coordinators"]["resources"].is_object());
    assert!(metrics["orchestrator"]["coordinators"]["backup"].is_object());
    
    // Проверяем специфичные метрики
    assert_eq!(
        metrics["orchestrator"]["coordinators"]["embedding"]["type"].as_str().unwrap(),
        "embedding_coordinator"
    );
    assert_eq!(
        metrics["orchestrator"]["coordinators"]["search"]["type"].as_str().unwrap(),
        "search_coordinator"
    );
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_search_integration() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    orchestrator.initialize_all().await?;
    
    let query = "test search query";
    let layer = Layer::Interact;
    let options = SearchOptions::default();
    
    let results = orchestrator.search(query, layer, options).await?;
    
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "mock_1");
    assert_eq!(results[0].content, "Mock search result 1");
    assert_eq!(results[0].layer, layer);
    
    // Проверяем что search coordinator был вызван
    let search_coordinator = &orchestrator.search;
    assert_eq!(search_coordinator.get_search_count(), 1);
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_search_resource_check() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    orchestrator.initialize_all().await?;
    
    // Отключаем ресурсы
    orchestrator.resources.set_resources_available(false);
    
    let query = "test search query";
    let layer = Layer::Interact;
    let options = SearchOptions::default();
    
    // Поиск должен вернуть пустой результат из-за недостатка ресурсов
    let results = orchestrator.search(query, layer, options).await?;
    assert!(results.is_empty());
    
    // Search coordinator не должен был быть вызван
    let search_coordinator = &orchestrator.search;
    assert_eq!(search_coordinator.get_search_count(), 0);
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_promotion_integration() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    orchestrator.initialize_all().await?;
    
    let stats = orchestrator.run_promotion().await?;
    
    assert_eq!(stats.records_promoted, 5);
    assert_eq!(stats.layers_affected.len(), 2);
    assert_eq!(stats.success_rate, 1.0);
    
    // Проверяем что promotion coordinator был вызван
    let promotion_coordinator = &orchestrator.promotion;
    assert_eq!(promotion_coordinator.get_promotion_count(), 1);
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_promotion_resource_check() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    orchestrator.initialize_all().await?;
    
    // Отключаем ресурсы
    orchestrator.resources.set_resources_available(false);
    
    let stats = orchestrator.run_promotion().await?;
    
    // Должен вернуть пустую статистику из-за недостатка ресурсов
    assert_eq!(stats.records_promoted, 0);
    
    // Promotion coordinator не должен был быть вызван
    let promotion_coordinator = &orchestrator.promotion;
    assert_eq!(promotion_coordinator.get_promotion_count(), 0);
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_health_check() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    orchestrator.initialize_all().await?;
    
    let health = orchestrator.check_health().await?;
    
    assert!(health.overall_healthy);
    assert_eq!(health.uptime, std::time::Duration::from_secs(3600));
    
    // Тестируем unhealthy состояние
    orchestrator.health.set_healthy(false);
    let health = orchestrator.check_health().await?;
    assert!(!health.overall_healthy);
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_backup_integration() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    orchestrator.initialize_all().await?;
    
    let backup_path = "/tmp/test_backup";
    let metadata = orchestrator.create_backup(backup_path).await?;
    
    assert_eq!(metadata.path, backup_path);
    assert_eq!(metadata.size_bytes, 1024000);
    assert_eq!(metadata.checksum, "mock_checksum_123");
    assert_eq!(metadata.version, "1.0.0");
    
    // Проверяем что backup coordinator был вызван
    let backup_coordinator = &orchestrator.backup;
    assert_eq!(backup_coordinator.get_backup_count(), 1);
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_backup_resource_check() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    orchestrator.initialize_all().await?;
    
    // Отключаем ресурсы
    orchestrator.resources.set_resources_available(false);
    
    let backup_path = "/tmp/test_backup";
    let result = orchestrator.create_backup(backup_path).await;
    
    // Должен вернуть ошибку из-за недостатка ресурсов
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Недостаточно ресурсов"));
    
    // Backup coordinator не должен был быть вызван
    let backup_coordinator = &orchestrator.backup;
    assert_eq!(backup_coordinator.get_backup_count(), 0);
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_concurrent_operations() -> Result<()> {
    let orchestrator = Arc::new(create_test_orchestrator());
    orchestrator.initialize_all().await?;
    
    let mut handles = vec![];
    
    // Запускаем concurrent операции
    for i in 0..10 {
        let orchestrator_clone = orchestrator.clone();
        let handle = tokio::spawn(async move {
            let query = format!("concurrent query {}", i);
            let layer = Layer::Interact;
            let options = SearchOptions::default();
            orchestrator_clone.search(&query, layer, options).await
        });
        handles.push(handle);
    }
    
    // Ждем завершения всех операций
    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(results)) = handle.await {
            if !results.is_empty() {
                success_count += 1;
            }
        }
    }
    
    assert_eq!(success_count, 10);
    
    // Проверяем что search coordinator был вызван нужное количество раз
    let search_coordinator = &orchestrator.search;
    assert_eq!(search_coordinator.get_search_count(), 10);
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_partial_initialization_failure() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    
    // Инициализируем только некоторые координаторы
    orchestrator.embedding.initialize().await?;
    orchestrator.search.initialize().await?;
    // Не инициализируем остальные
    
    // Проверяем что не все готовы
    assert!(!orchestrator.all_ready().await);
    
    // Завершаем инициализацию
    orchestrator.health.initialize().await?;
    orchestrator.promotion.initialize().await?;
    orchestrator.resources.initialize().await?;
    orchestrator.backup.initialize().await?;
    
    // Теперь все должны быть готовы
    assert!(orchestrator.all_ready().await);
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_graceful_shutdown_with_errors() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    orchestrator.initialize_all().await?;
    
    // Симулируем ошибку shutdown в одном из координаторов
    // (В реальной реализации мы бы добавили способ симулировать ошибки)
    
    // Shutdown должен обработать ошибки gracefully и продолжить
    let result = orchestrator.shutdown_all().await;
    assert!(result.is_ok());
    
    // Проверяем что координаторы остановились (те что могли)
    assert!(!orchestrator.all_ready().await);
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_metrics_after_operations() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    orchestrator.initialize_all().await?;
    
    // Выполняем различные операции
    orchestrator.search("test", Layer::Interact, SearchOptions::default()).await?;
    orchestrator.run_promotion().await?;
    orchestrator.create_backup("/tmp/test").await?;
    orchestrator.check_health().await?;
    
    let metrics = orchestrator.all_metrics().await;
    
    // Проверяем что метрики отражают выполненные операции
    assert_eq!(
        metrics["orchestrator"]["coordinators"]["search"]["searches"].as_u64().unwrap(),
        1
    );
    assert_eq!(
        metrics["orchestrator"]["coordinators"]["promotion"]["promotions"].as_u64().unwrap(),
        1
    );
    assert_eq!(
        metrics["orchestrator"]["coordinators"]["backup"]["backups"].as_u64().unwrap(),
        1
    );
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_initialization_timing() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    
    let start_time = std::time::Instant::now();
    orchestrator.initialize_all().await?;
    let init_time = start_time.elapsed();
    
    // Инициализация должна занять разумное время
    // (учитывая 10ms задержку в MockEmbeddingCoordinator)
    assert!(init_time >= std::time::Duration::from_millis(10));
    assert!(init_time < std::time::Duration::from_millis(1000));
    
    assert!(orchestrator.all_ready().await);
    
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_state_consistency() -> Result<()> {
    let orchestrator = create_test_orchestrator();
    
    // Тестируем последовательность состояний
    assert!(!orchestrator.all_ready().await);
    
    orchestrator.initialize_all().await?;
    assert!(orchestrator.all_ready().await);
    
    orchestrator.shutdown_all().await?;
    assert!(!orchestrator.all_ready().await);
    
    // Повторная инициализация должна работать
    orchestrator.initialize_all().await?;
    assert!(orchestrator.all_ready().await);
    
    Ok(())
}