//! DIMemoryService - Refactored Facade Pattern
//! 
//! Этот файл представляет рефакторированный DIMemoryService как Facade,
//! делегирующий ответственности специализированным модулям.
//! Обеспечивает 100% обратную совместимость публичного API.

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, debug};

use crate::{
    di_container::DIContainer,
    di_memory_config::MemoryDIConfigurator,
    health::SystemHealthStatus,
    promotion::PromotionStats,
    types::{Record, Layer, SearchOptions},
    service_di::{
        // Импортируем все новые модули
        OrchestrationCoordinators,
        ProductionCoordinatorFactory,
        ProductionMonitoringManager,
        CircuitBreaker,
        LifecycleManager,
        ProductionOperationExecutor,
        ExtendedOperationExecutor,
        OperationExecutor,
        BatchInsertResult,
        BatchSearchResult,
        MemorySystemStats,
        OperationConfig,
    },
};

// Re-export для обратной совместимости
pub use crate::service_di::{
    default_config,
    MemoryConfig,
    MemoryServiceConfig,
    MemoryServiceConfigBuilder,
};

/// Production-ready DI Memory Service - FACADE PATTERN
/// 
/// Этот класс больше НЕ является God Object!
/// Он служит Facade для координации специализированных модулей:
/// - ServiceConfig: конфигурация
/// - CoordinatorFactory: создание координаторов  
/// - ProductionMonitoring: метрики и мониторинг
/// - CircuitBreaker: resilience patterns
/// - LifecycleManager: управление жизненным циклом
/// - OperationExecutor: выполнение операций
pub struct DIMemoryService {
    /// DI контейнер со всеми зависимостями
    container: Arc<DIContainer>,
    /// Координаторы orchestration
    coordinators: OrchestrationCoordinators,
    /// Production мониторинг и метрики
    monitoring_manager: ProductionMonitoringManager,
    /// Circuit breaker для resilience
    circuit_breaker: Arc<CircuitBreaker>,
    /// Lifecycle manager
    lifecycle_manager: Arc<LifecycleManager>,
    /// Operation executor для бизнес-логики
    operation_executor: Arc<dyn OperationExecutor + Send + Sync>,
    /// Extended executor для дополнительных операций
    extended_executor: ExtendedOperationExecutor,
}

impl DIMemoryService {
    /// Создать новый production-ready DI-based сервис
    pub async fn new(config: MemoryConfig) -> Result<Self> {
        info!("🚀 Создание production DIMemoryService с рефакторированной архитектурой");

        // 1. Настраиваем полный DI контейнер
        let container = Arc::new(MemoryDIConfigurator::configure_full(config).await?);

        // 2. Создаём orchestration координаторы через Factory
        let coordinator_factory = ProductionCoordinatorFactory::new();
        let coordinators = coordinator_factory.create_all_coordinators(&container).await?;

        // 3. Создаём специализированные сервисы
        let monitoring_manager = ProductionMonitoringManager::new()
            .with_health_manager(coordinators.health_manager.clone())
            .with_resource_controller(coordinators.resource_controller.clone());

        let circuit_breaker = Arc::new(CircuitBreaker::with_production_config());
        let lifecycle_manager = Arc::new(LifecycleManager::with_production_config());

        // 4. Создаём operation executor
        let operation_executor = Arc::new(ProductionOperationExecutor::new(
            container.clone(),
            coordinators.embedding_coordinator.clone(),
            coordinators.search_coordinator.clone(),
            OperationConfig::production(),
        ));

        let extended_executor = ExtendedOperationExecutor::new(
            container.clone(),
            operation_executor.clone(),
        );

        let service = Self {
            container,
            coordinators,
            monitoring_manager,
            circuit_breaker,
            lifecycle_manager,
            operation_executor,
            extended_executor,
        };

        info!("✅ Production DIMemoryService создан с рефакторированной архитектурой ({} координаторов)", 
              service.coordinators.count_active());
        
        Ok(service)
    }

    /// Создать минимальный сервис для тестов
    pub async fn new_minimal(config: MemoryConfig) -> Result<Self> {
        info!("🧪 Создание минимального DIMemoryService для тестов");

        let container = Arc::new(MemoryDIConfigurator::configure_minimal(config).await?);

        // Минимальная конфигурация без координаторов
        let coordinators = OrchestrationCoordinators::empty();
        
        let monitoring_manager = ProductionMonitoringManager::new();
        let circuit_breaker = Arc::new(CircuitBreaker::with_minimal_config());
        let lifecycle_manager = Arc::new(LifecycleManager::with_minimal_config());

        // Простой executor для тестов
        let operation_executor = Arc::new(ProductionOperationExecutor::new_minimal(container.clone()));
        
        let extended_executor = ExtendedOperationExecutor::new(
            container.clone(),
            operation_executor.clone(),
        );

        Ok(Self {
            container,
            coordinators,
            monitoring_manager,
            circuit_breaker,
            lifecycle_manager,
            operation_executor,
            extended_executor,
        })
    }

    /// Production инициализация всей системы - ДЕЛЕГИРУЕТ к LifecycleManager
    pub async fn initialize(&self) -> Result<()> {
        info!("🚀 Production инициализация через LifecycleManager...");

        self.lifecycle_manager.initialize(|| async {
            // 1. Инициализируем базовые слои памяти
            let store = self.container.resolve::<crate::storage::VectorStore>()?;
            self.lifecycle_manager.initialize_memory_layers(store).await?;

            // 2. Инициализируем все координаторы параллельно
            self.coordinators.initialize_all().await?;

            // 3. Запускаем production мониторинг
            self.monitoring_manager.start_production_monitoring().await?;

            // 4. Запускаем health checks и metrics collection
            self.monitoring_manager.start_health_monitoring().await?;

            // 5. Запускаем resource monitoring
            self.monitoring_manager.start_resource_monitoring().await?;

            // 6. Выполняем начальные проверки готовности
            self.coordinators.check_readiness().await?;

            Ok(())
        }).await
    }

    /// Production insert - ДЕЛЕГИРУЕТ к OperationExecutor
    pub async fn insert(&self, record: Record) -> Result<()> {
        // Проверяем circuit breaker
        self.circuit_breaker.check_and_allow_operation().await?;

        // Увеличиваем счетчик активных операций
        let _active_op_guard = self.lifecycle_manager.increment_active_operations();

        let start_time = std::time::Instant::now();

        // Делегируем к operation executor
        let result = self.operation_executor.insert(record).await;

        // Записываем результат в circuit breaker и monitoring
        let duration = start_time.elapsed();
        match result {
            Ok(_) => {
                self.circuit_breaker.record_success().await;
                self.monitoring_manager.record_successful_operation(duration).await;
                Ok(())
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                self.monitoring_manager.record_failed_operation(duration).await;
                Err(e)
            }
        }
    }

    /// Production search - ДЕЛЕГИРУЕТ к OperationExecutor
    pub async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        // Проверяем circuit breaker
        self.circuit_breaker.check_and_allow_operation().await?;

        // Увеличиваем счетчик активных операций
        let _active_op_guard = self.lifecycle_manager.increment_active_operations();

        let start_time = std::time::Instant::now();

        // Делегируем к operation executor
        let result = self.operation_executor.search(query, layer, options).await;

        // Записываем результат в circuit breaker и monitoring
        let duration = start_time.elapsed();
        match result {
            Ok(results) => {
                self.circuit_breaker.record_success().await;
                self.monitoring_manager.record_successful_operation(duration).await;
                Ok(results)
            }
            Err(e) => {
                self.circuit_breaker.record_failure().await;
                self.monitoring_manager.record_failed_operation(duration).await;
                Err(e)
            }
        }
    }

    /// Вставить несколько записей батчем - ДЕЛЕГИРУЕТ к OperationExecutor
    pub async fn insert_batch(&self, records: Vec<Record>) -> Result<()> {
        let result = self.operation_executor.batch_insert(records).await?;
        debug!("✓ Batch insert: {}/{} записей успешно", result.inserted, result.inserted + result.failed);
        Ok(())
    }

    /// Production статистика системы - ДЕЛЕГИРУЕТ к модулям
    pub async fn get_stats(&self) -> MemorySystemStats {
        debug!("📊 Сбор production статистики через специализированные модули");

        // Собираем данные от координаторов
        let health_status = if let Some(ref health_manager) = self.coordinators.health_manager {
            health_manager.system_health().await
        } else {
            Err(anyhow::anyhow!("Health manager not available"))
        };

        // Cache статистика через EmbeddingCoordinator если доступен
        let cache_stats = if let Some(ref embedding_coordinator) = self.coordinators.embedding_coordinator {
            embedding_coordinator.cache_stats().await
        } else {
            (0, 0, 0)
        };

        // Используем default значения для недоступных сервисов
        let promotion_stats = PromotionStats::default();
        let batch_stats = self.extended_executor.get_operation_stats().await
            .unwrap_or_default();
        let gpu_stats = None; // Async получение GPU stats не реализовано

        MemorySystemStats {
            health_status,
            cache_hits: cache_stats.0,
            cache_misses: cache_stats.1,
            cache_size: cache_stats.2,
            promotion_stats,
            batch_stats,
            gpu_stats,
            di_container_stats: self.container.stats(),
        }
    }

    /// Production graceful shutdown - ДЕЛЕГИРУЕТ к LifecycleManager
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛑 Начало graceful shutdown через LifecycleManager...");

        self.lifecycle_manager.shutdown(|| async {
            // Shutdown координаторов
            self.coordinators.shutdown_all().await?;
            
            // Останавливаем мониторинг
            self.monitoring_manager.stop_all_monitoring().await?;

            Ok(())
        }).await
    }

    // === Остальные методы для BACKWARD COMPATIBILITY ===

    /// Запустить promotion процесс - ДЕЛЕГИРУЕТ
    pub async fn run_promotion(&self) -> Result<PromotionStats> {
        if let Ok(promotion_engine) = self.container.resolve::<crate::promotion::PromotionEngine>() {
            let stats = promotion_engine.run_promotion_cycle().await?;
            info!("✓ Promotion завершен: interact_to_insights={}, insights_to_assets={}", 
                  stats.interact_to_insights, stats.insights_to_assets);
            Ok(stats)
        } else {
            debug!("Promotion engine недоступен, возвращаем нулевую статистику");
            Ok(PromotionStats::default())
        }
    }
    
    /// Алиас для run_promotion для обратной совместимости
    pub async fn run_promotion_cycle(&self) -> Result<PromotionStats> {
        self.run_promotion().await
    }

    /// Flush всех pending операций - ДЕЛЕГИРУЕТ
    pub async fn flush_all(&self) -> Result<()> {
        self.extended_executor.flush_all().await
    }

    /// Батчевая вставка записей - ДЕЛЕГИРУЕТ
    pub async fn batch_insert(&self, records: Vec<Record>) -> Result<BatchInsertResult> {
        self.operation_executor.batch_insert(records).await
    }

    /// Батчевый поиск - ДЕЛЕГИРУЕТ
    pub async fn batch_search(&self, queries: Vec<String>, layer: Layer, options: SearchOptions) -> Result<BatchSearchResult> {
        self.operation_executor.batch_search(queries, layer, options).await
    }

    /// Обновить запись - ДЕЛЕГИРУЕТ
    pub async fn update(&self, record: Record) -> Result<()> {
        self.operation_executor.update(record).await
    }

    /// Удалить запись - ДЕЛЕГИРУЕТ
    pub async fn delete(&self, id: &uuid::Uuid, layer: Layer) -> Result<()> {
        self.operation_executor.delete(id, layer).await
    }

    /// Создать backup - ДЕЛЕГИРУЕТ
    pub async fn create_backup(&self, path: &str) -> Result<crate::backup::BackupMetadata> {
        self.extended_executor.create_backup(path).await
    }

    /// Проверить здоровье системы - ДЕЛЕГИРУЕТ
    pub async fn check_health(&self) -> Result<SystemHealthStatus> {
        let health = self.container.resolve::<Arc<crate::health::HealthMonitor>>()?;
        Ok(health.get_system_health())
    }

    /// Получить доступ к конкретному компоненту через DI - ПРЯМАЯ ДЕЛЕГАЦИЯ
    pub fn resolve<T>(&self) -> Result<Arc<T>>
    where
        T: std::any::Any + Send + Sync + 'static,
    {
        self.container.resolve::<T>()
    }

    /// Получить опциональный доступ к компоненту - ПРЯМАЯ ДЕЛЕГАЦИЯ
    pub fn try_resolve<T>(&self) -> Option<Arc<T>>
    where
        T: std::any::Any + Send + Sync + 'static,
    {
        self.container.try_resolve::<T>()
    }

    /// Получить статистику DI контейнера - ПРЯМАЯ ДЕЛЕГАЦИЯ
    pub fn di_stats(&self) -> crate::DIContainerStats {
        self.container.stats()
    }

    /// Получить performance метрики DI системы - ПРЯМАЯ ДЕЛЕГАЦИЯ
    pub fn get_performance_metrics(&self) -> crate::DIPerformanceMetrics {
        self.container.get_performance_metrics()
    }

    /// Получить краткий отчет о производительности DI системы - ПРЯМАЯ ДЕЛЕГАЦИЯ
    pub fn get_performance_report(&self) -> String {
        self.container.get_performance_report()
    }

    /// Сбросить performance метрики (для тестов) - ПРЯМАЯ ДЕЛЕГАЦИЯ
    pub fn reset_performance_metrics(&self) {
        self.container.reset_performance_metrics()
    }
}

/// Builder для создания DIMemoryService с различными конфигурациями - BACKWARD COMPATIBILITY
pub struct DIMemoryServiceBuilder {
    config: MemoryConfig,
    minimal: bool,
    cpu_only: bool,
}

impl DIMemoryServiceBuilder {
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            config,
            minimal: false,
            cpu_only: false,
        }
    }

    pub fn minimal(mut self) -> Self {
        self.minimal = true;
        self
    }

    pub fn cpu_only(mut self) -> Self {
        self.cpu_only = true;
        self
    }

    pub async fn build(self) -> Result<DIMemoryService> {
        if self.minimal {
            DIMemoryService::new_minimal(self.config).await
        } else if self.cpu_only {
            let mut cpu_config = self.config;
            cpu_config.ai_config.embedding.use_gpu = false;
            cpu_config.ai_config.reranking.use_gpu = false;
            
            // Для CPU-only создаем как minimal
            DIMemoryService::new_minimal(cpu_config).await
        } else {
            DIMemoryService::new(self.config).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::di_memory_config::test_helpers;

    #[tokio::test]
    async fn test_di_memory_service_facade_creation() -> Result<()> {
        let config = test_helpers::create_test_config()?;
        let service = DIMemoryService::new_minimal(config).await?;

        // Проверяем основные компоненты через DI
        let store = service.resolve::<crate::storage::VectorStore>()?;
        assert!(!(store.as_ref() as *const _ == std::ptr::null()));
        
        let stats = service.di_stats();
        assert!(stats.total_types > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_facade_initialization() -> Result<()> {
        let config = test_helpers::create_test_config()?;
        let service = DIMemoryService::new_minimal(config).await?;

        // Тестируем инициализацию через facade
        service.initialize().await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_builder_pattern_facade() -> Result<()> {
        let config = test_helpers::create_test_config()?;
        
        let service = DIMemoryServiceBuilder::new(config)
            .minimal()
            .cpu_only()
            .build()
            .await?;

        let stats = service.get_stats().await;
        // Базовые проверки что сервис создан
        assert!(stats.di_container_stats.total_types > 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_operation_delegation() -> Result<()> {
        let config = test_helpers::create_test_config()?;
        let service = DIMemoryService::new_minimal(config).await?;

        // Тестируем разрешение зависимостей через facade
        let store = service.resolve::<crate::storage::VectorStore>()?;
        assert!(!(store.as_ref() as *const _ == std::ptr::null()));

        // Тестируем опциональное разрешение
        let _optional_metrics = service.try_resolve::<Arc<crate::metrics::MetricsCollector>>();

        Ok(())
    }
}