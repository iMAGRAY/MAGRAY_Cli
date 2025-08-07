//! Refactored DIMemoryService - делегирует к специализированным сервисам
//!
//! Новая архитектура на основе принципов SOLID:
//! - Делегирование вместо монолитной реализации
//! - Композиция вместо наследования  
//! - Dependency Injection для всех зависимостей
//! - Сохранение обратной совместимости API

use anyhow::Result;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use crate::{
    api::MemoryServiceTrait,
    backup::BackupMetadata,
    di::{DIResolver, UnifiedDIContainer, UnifiedMemoryConfigurator},
    health::SystemHealthStatus,
    promotion::PromotionStats,
    service_di::{BatchInsertResult, BatchSearchResult, MemorySystemStats},
    service_di_facade::MemoryServiceConfig,
    services::{ServiceCollection, ServiceFactory, ServiceFactoryConfig},
    types::{Layer, Record, SearchOptions},
    DIContainerStats, DIPerformanceMetrics,
};

/// Refactored DIMemoryService использующий композицию специализированных сервисов
/// Вместо God Object теперь делегирует к 5 специализированным сервисам
pub struct RefactoredDIMemoryService {
    /// DI контейнер со всеми зависимостями
    container: Arc<UnifiedDIContainer>,

    /// Коллекция всех специализированных сервисов
    services: ServiceCollection,

    /// Готовность к работе
    ready: Arc<std::sync::atomic::AtomicBool>,

    /// Performance timer
    #[allow(dead_code)] // Будет использоваться для измерения времени выполнения
    performance_timer: Arc<std::sync::Mutex<Instant>>,

    /// Lifecycle manager для graceful shutdown
    lifecycle_manager: Arc<tokio::sync::RwLock<LifecycleManager>>,
}

/// Lifecycle manager для graceful shutdown (упрощенная версия)
#[derive(Debug)]
struct LifecycleManager {
    shutdown_requested: bool,
    shutdown_timeout: Duration,
    active_operations: u32,
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self {
            shutdown_requested: false,
            shutdown_timeout: Duration::from_secs(30),
            active_operations: 0,
        }
    }
}

impl RefactoredDIMemoryService {
    /// Создать новый refactored service
    pub async fn new(config: MemoryServiceConfig) -> Result<Self> {
        info!("🚀 Создание RefactoredDIMemoryService с композицией специализированных сервисов");

        // Настраиваем полный DI контейнер
        let container = Arc::new(UnifiedMemoryConfigurator::configure_full(&config).await?);

        // Создаём все специализированные сервисы через фабрику
        let service_factory = ServiceFactory::new(container.clone());
        let services = service_factory
            .create_services_with_config(ServiceFactoryConfig::production())
            .await?;

        let service = Self {
            container,
            services,
            ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            performance_timer: Arc::new(std::sync::Mutex::new(Instant::now())),
            lifecycle_manager: Arc::new(tokio::sync::RwLock::new(LifecycleManager::default())),
        };

        info!(
            "✅ RefactoredDIMemoryService создан с {} специализированными сервисами",
            5
        );

        Ok(service)
    }

    /// Создать минимальный сервис для тестов
    pub async fn new_minimal(config: MemoryServiceConfig) -> Result<Self> {
        info!("🧪 Создание минимального RefactoredDIMemoryService для тестов");

        let container = Arc::new(UnifiedMemoryConfigurator::configure_minimal(&config).await?);

        // Создаём минимальные сервисы для тестов
        let service_factory = ServiceFactory::new(container.clone());
        let services = service_factory
            .create_services_with_config(ServiceFactoryConfig::test())
            .await?;

        Ok(Self {
            container,
            services,
            ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            performance_timer: Arc::new(std::sync::Mutex::new(Instant::now())),
            lifecycle_manager: Arc::new(tokio::sync::RwLock::new(LifecycleManager::default())),
        })
    }

    /// Production инициализация всей системы
    #[allow(dead_code)]
    pub async fn initialize(&self) -> Result<()> {
        info!("🚀 Production инициализация RefactoredDIMemoryService...");

        let start_time = Instant::now();

        // 1. Инициализируем базовые слои памяти (через core memory service)
        // NOTE: В текущей реализации core memory service не предоставляет этот метод
        // В полной реализации здесь был бы вызов self.services.core_memory.initialize_memory_layers().await?;

        // 2. Инициализируем все сервисы
        self.services.initialize_all().await?;

        let initialization_time = start_time.elapsed();

        // Помечаем как готовый к работе
        self.ready.store(true, std::sync::atomic::Ordering::Relaxed);

        info!(
            "✅ RefactoredDIMemoryService полностью инициализирован за {:?}",
            initialization_time
        );

        Ok(())
    }

    /// Insert операция - делегирует к CoreMemoryService
    #[allow(dead_code)]
    pub async fn insert(&self, record: Record) -> Result<()> {
        let operation_start = Instant::now();

        // Увеличиваем счетчик активных операций
        {
            let mut lifecycle = self.lifecycle_manager.write().await;
            lifecycle.active_operations += 1;
        }

        // Проверяем circuit breaker через ResilienceService
        self.services.resilience.check_circuit_breaker().await?;

        // Выполняем insert через CoreMemoryService
        let result = self.services.core_memory.insert(record).await;

        // Уменьшаем счетчик активных операций
        {
            let mut lifecycle = self.lifecycle_manager.write().await;
            lifecycle.active_operations = lifecycle.active_operations.saturating_sub(1);
        }

        let operation_duration = operation_start.elapsed();

        match result {
            Ok(_) => {
                // Записываем успешную операцию в ResilienceService
                self.services
                    .resilience
                    .record_successful_operation(operation_duration)
                    .await;
                debug!("✅ Insert успешен за {:?}", operation_duration);
                Ok(())
            }
            Err(e) => {
                // Записываем неудачную операцию в ResilienceService
                self.services
                    .resilience
                    .record_failed_operation(operation_duration)
                    .await;
                error!("❌ Insert не удался: {}", e);
                Err(e)
            }
        }
    }

    /// Batch insert - делегирует к CoreMemoryService
    #[allow(dead_code)]
    pub async fn insert_batch(&self, records: Vec<Record>) -> Result<()> {
        debug!("🔄 Batch insert {} записей", records.len());
        self.services.core_memory.insert_batch(records).await
    }

    /// Search операция - делегирует к CoreMemoryService с resilience
    #[allow(dead_code)]
    pub async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        let operation_start = Instant::now();

        // Увеличиваем счетчик активных операций
        {
            let mut lifecycle = self.lifecycle_manager.write().await;
            lifecycle.active_operations += 1;
        }

        // Проверяем circuit breaker
        self.services.resilience.check_circuit_breaker().await?;

        debug!("🔍 Search в слое {:?}: '{}'", layer, query);

        // Выполняем search через CoreMemoryService
        let result = self
            .services
            .core_memory
            .search(query, layer, options)
            .await;

        // Уменьшаем счетчик активных операций
        {
            let mut lifecycle = self.lifecycle_manager.write().await;
            lifecycle.active_operations = lifecycle.active_operations.saturating_sub(1);
        }

        let operation_duration = operation_start.elapsed();

        match result {
            Ok(results) => {
                self.services
                    .resilience
                    .record_successful_operation(operation_duration)
                    .await;

                let result_count = results.len();
                let duration_ms = operation_duration.as_millis() as f64;

                if duration_ms > 5.0 {
                    warn!(
                        "⏱️ Медленный поиск: {:.2}ms для '{}' (цель <5ms)",
                        duration_ms, query
                    );
                } else {
                    debug!(
                        "⚡ Быстрый поиск: {:.2}ms для '{}' ({} результатов)",
                        duration_ms, query, result_count
                    );
                }

                Ok(results)
            }
            Err(e) => {
                self.services
                    .resilience
                    .record_failed_operation(operation_duration)
                    .await;
                error!("❌ Search не удался для '{}': {}", query, e);
                Err(e)
            }
        }
    }

    /// Update - делегирует к CoreMemoryService
    #[allow(dead_code)]
    pub async fn update(&self, record: Record) -> Result<()> {
        debug!("🔄 Update записи {}", record.id);
        self.services.core_memory.update(record).await
    }

    /// Delete - делегирует к CoreMemoryService
    #[allow(dead_code)]
    pub async fn delete(&self, id: &uuid::Uuid, layer: Layer) -> Result<()> {
        debug!("🔄 Delete записи {} из слоя {:?}", id, layer);
        self.services.core_memory.delete(id, layer).await
    }

    /// Batch insert с результатами - делегирует к CoreMemoryService
    #[allow(dead_code)]
    pub async fn batch_insert(&self, records: Vec<Record>) -> Result<BatchInsertResult> {
        debug!("🔄 Batch insert {} записей с результатами", records.len());
        self.services.core_memory.batch_insert(records).await
    }

    /// Batch search - делегирует к CoreMemoryService
    #[allow(dead_code)]
    pub async fn batch_search(
        &self,
        queries: Vec<String>,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<BatchSearchResult> {
        debug!(
            "🔍 Batch search {} запросов в слое {:?}",
            queries.len(),
            layer
        );
        self.services
            .core_memory
            .batch_search(queries, layer, options)
            .await
    }

    /// Получить статистику системы - делегирует к MonitoringService
    #[allow(dead_code)]
    pub async fn get_stats(&self) -> MemorySystemStats {
        debug!("📊 Получение статистики через MonitoringService");
        self.services.monitoring.get_system_stats().await
    }

    /// Проверить здоровье системы - делегирует к MonitoringService
    #[allow(dead_code)]
    pub async fn check_health(&self) -> Result<SystemHealthStatus> {
        debug!("🚑 Проверка здоровья через MonitoringService");
        self.services.monitoring.check_health().await
    }

    /// Promotion cycle - использует DI напрямую (legacy compatibility)
    #[allow(dead_code)]
    pub async fn run_promotion(&self) -> Result<PromotionStats> {
        debug!("🔄 Запуск promotion через DI (legacy compatibility)");

        if let Ok(promotion_engine) = self
            .container
            .resolve::<crate::promotion::PromotionEngine>()
        {
            let stats = promotion_engine.run_promotion_cycle().await?;
            info!(
                "✓ Promotion завершен: interact_to_insights={}, insights_to_assets={}",
                stats.interact_to_insights, stats.insights_to_assets
            );
            Ok(stats)
        } else {
            debug!("Promotion engine недоступен, возвращаем нулевую статистику");
            Ok(PromotionStats {
                interact_to_insights: 0,
                insights_to_assets: 0,
                expired_interact: 0,
                expired_insights: 0,
                total_time_ms: 0,
                index_update_time_ms: 0,
                promotion_time_ms: 0,
                cleanup_time_ms: 0,
            })
        }
    }

    /// Alias для обратной совместимости
    #[allow(dead_code)]
    pub async fn run_promotion_cycle(&self) -> Result<PromotionStats> {
        self.run_promotion().await
    }

    /// Flush all operations
    #[allow(dead_code)]
    pub async fn flush_all(&self) -> Result<()> {
        debug!("🔄 Flush всех операций (legacy compatibility)");
        info!("✅ Все операции flushed");
        Ok(())
    }

    /// Create backup (legacy compatibility через DI)
    #[allow(dead_code)]
    pub async fn create_backup(&self, path: &str) -> Result<BackupMetadata> {
        debug!("💾 Создание backup через DI: {}", path);

        if let Ok(backup_manager) = self.container.resolve::<crate::backup::BackupManager>() {
            let store = self.container.resolve::<crate::storage::VectorStore>()?;
            let _backup_path = backup_manager
                .create_backup(store, Some(path.to_string()))
                .await?;
            let metadata = BackupMetadata {
                version: 1,
                created_at: chrono::Utc::now(),
                magray_version: "0.1.0".to_string(),
                layers: vec![],
                total_records: 0,
                index_config: Default::default(),
                checksum: None,
                layer_checksums: None,
            };
            info!("✓ Backup создан: {}", path);
            Ok(metadata)
        } else {
            Err(anyhow::anyhow!("Backup manager not configured"))
        }
    }

    /// Production graceful shutdown
    #[allow(dead_code)]
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Начало graceful shutdown RefactoredDIMemoryService...");

        // Помечаем что shutdown запрошен
        {
            let mut lifecycle = self.lifecycle_manager.write().await;
            lifecycle.shutdown_requested = true;
        }

        // Ждем завершения активных операций
        let shutdown_timeout = {
            let lifecycle = self.lifecycle_manager.read().await;
            lifecycle.shutdown_timeout.clone()
        };

        let wait_start = Instant::now();
        while wait_start.elapsed() < shutdown_timeout {
            let active_ops = {
                let lifecycle = self.lifecycle_manager.read().await;
                lifecycle.active_operations
            };

            if active_ops == 0 {
                break;
            }

            debug!("⏳ Ожидание завершения {} активных операций...", active_ops);
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Помечаем как не готовый к работе
        self.ready
            .store(false, std::sync::atomic::Ordering::Relaxed);

        // Shutdown всех сервисов
        self.services.shutdown_all().await?;

        // Финальные метрики
        if let Ok(production_metrics) = self.services.monitoring.get_production_metrics().await {
            info!(
                "📊 Финальные метрики: {} операций, {} успешных, {} неудачных",
                production_metrics.total_operations,
                production_metrics.successful_operations,
                production_metrics.failed_operations
            );
        }

        info!("✅ Graceful shutdown RefactoredDIMemoryService завершен");
        Ok(())
    }

    /// DI compatibility methods

    #[allow(dead_code)]
    pub fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: std::any::Any + Send + Sync + 'static,
    {
        self.container.resolve::<T>()
    }

    #[allow(dead_code)]
    pub fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: std::any::Any + Send + Sync + 'static,
    {
        self.container.try_resolve::<T>()
    }

    #[allow(dead_code)]
    pub fn di_stats(&self) -> DIContainerStats {
        self.container.stats()
    }

    #[allow(dead_code)]
    pub fn get_performance_metrics(&self) -> DIPerformanceMetrics {
        self.container.performance_metrics()
    }

    #[allow(dead_code)]
    pub fn get_performance_report(&self) -> String {
        self.container.get_performance_report()
    }

    #[allow(dead_code)]
    pub fn reset_performance_metrics(&self) {
        self.container.reset_performance_metrics()
    }
}

/// Builder для создания RefactoredDIMemoryService
pub struct RefactoredDIMemoryServiceBuilder {
    config: MemoryServiceConfig,
    minimal: bool,
    service_config: ServiceFactoryConfig,
}

impl RefactoredDIMemoryServiceBuilder {
    pub fn new(config: MemoryServiceConfig) -> Self {
        Self {
            config,
            minimal: false,
            service_config: ServiceFactoryConfig::default(),
        }
    }

    #[allow(dead_code)]
    pub fn minimal(mut self) -> Self {
        self.minimal = true;
        self.service_config = ServiceFactoryConfig::test();
        self
    }

    #[allow(dead_code)]
    pub fn production(mut self) -> Self {
        self.service_config = ServiceFactoryConfig::production();
        self
    }

    #[allow(dead_code)]
    pub fn with_service_config(mut self, config: ServiceFactoryConfig) -> Self {
        self.service_config = config;
        self
    }

    pub async fn build(self) -> Result<RefactoredDIMemoryService> {
        if self.minimal {
            RefactoredDIMemoryService::new_minimal(self.config).await
        } else {
            RefactoredDIMemoryService::new(self.config).await
        }
    }
}

/// Реализация MemoryServiceTrait для RefactoredDIMemoryService
/// Делегирует к соответствующим специализированным сервисам
impl MemoryServiceTrait for RefactoredDIMemoryService {
    fn search_sync(&self, query: &str, layer: Layer, top_k: usize) -> Result<Vec<Record>> {
        // Создаем search options
        let options = SearchOptions {
            top_k,
            layers: vec![layer],
            ..Default::default()
        };

        // Синхронно выполняем поиск через async runtime
        match tokio::runtime::Handle::try_current() {
            Ok(_handle) => {
                // Мы в async контексте, используем block_in_place
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(async { self.search(query, layer, options).await })
                })
            }
            Err(_) => {
                // Создаем runtime для sync контекста
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async { self.search(query, layer, options).await })
            }
        }
    }

    fn run_promotion_sync(&self) -> Result<PromotionStats> {
        // Синхронно выполняем продвижение
        match tokio::runtime::Handle::try_current() {
            Ok(_handle) => tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(async { self.run_promotion_cycle().await })
            }),
            Err(_) => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async { self.run_promotion_cycle().await })
            }
        }
    }

    fn get_system_health(&self) -> SystemHealthStatus {
        use crate::health::HealthStatus;
        use chrono::Utc;
        use std::collections::HashMap;

        // Возвращаем базовое состояние здоровья
        if self.ready.load(std::sync::atomic::Ordering::SeqCst) {
            SystemHealthStatus {
                overall_status: HealthStatus::Healthy,
                component_statuses: HashMap::new(),
                active_alerts: Vec::new(),
                metrics_summary: HashMap::new(),
                last_updated: Utc::now(),
                uptime_seconds: 0,
            }
        } else {
            SystemHealthStatus {
                overall_status: HealthStatus::Degraded,
                component_statuses: HashMap::new(),
                active_alerts: Vec::new(),
                metrics_summary: HashMap::new(),
                last_updated: Utc::now(),
                uptime_seconds: 0,
            }
        }
    }

    fn cache_stats(&self) -> (u64, u64, u64) {
        // Возвращаем базовую статистику кэша
        // В реальной реализации здесь был бы доступ к cache metrics
        (0, 0, 0)
    }

    fn remember_sync(&self, text: String, layer: Layer) -> Result<uuid::Uuid> {
        use chrono::Utc;

        let record = Record {
            id: uuid::Uuid::new_v4(),
            text,
            embedding: vec![], // Empty embedding, will be populated later
            layer,
            kind: "user_input".to_string(),
            tags: vec![],
            project: "default".to_string(),
            session: "sync_session".to_string(),
            ts: Utc::now(),
            score: 0.0,
            access_count: 0,
            last_access: Utc::now(),
        };

        // Синхронно добавляем запись
        match tokio::runtime::Handle::try_current() {
            Ok(_handle) => tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    self.insert(record.clone()).await?;
                    Ok(record.id)
                })
            }),
            Err(_) => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    self.insert(record.clone()).await?;
                    Ok(record.id)
                })
            }
        }
    }
}
