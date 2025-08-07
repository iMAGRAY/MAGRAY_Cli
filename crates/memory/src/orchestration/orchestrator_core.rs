//! Core Orchestrator - Минимальная ответственность только за координацию
//!
//! Применяет Single Responsibility Principle - только координация между компонентами.
//! Все сложная логика делегирована специализированным компонентам.

use anyhow::Result;
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};
use tokio::sync::Semaphore;
use tracing::info;

use super::{
    circuit_breaker_manager::CircuitBreakerManager,
    metrics_collector::MetricsCollector,
    operation_executor::OperationExecutor,
    retry_handler::RetryHandler,
    traits::{
        BackupCoordinator, EmbeddingCoordinator, HealthCoordinator, PromotionCoordinator,
        ResourceCoordinator, SearchCoordinator,
    },
};

/// Минимальный оркестратор с единственной ответственностью - координация
pub struct OrchestratorCore {
    // Координаторы (dependency injection)
    pub embedding: Arc<dyn EmbeddingCoordinator>,
    pub search: Arc<dyn SearchCoordinator>,
    pub health: Arc<dyn HealthCoordinator>,
    pub promotion: Arc<dyn PromotionCoordinator>,
    pub resources: Arc<dyn ResourceCoordinator>,
    pub backup: Arc<dyn BackupCoordinator>,

    // Вспомогательные компоненты (композиция)
    circuit_breaker_manager: Arc<CircuitBreakerManager>,
    metrics_collector: Arc<MetricsCollector>,
    operation_executor: Arc<OperationExecutor>,
    retry_handler: Arc<RetryHandler>,

    // Состояние
    ready: Arc<AtomicBool>,
    start_time: Instant,
    operation_limiter: Arc<Semaphore>,
}

impl OrchestratorCore {
    /// Создать оркестратор из готовых компонентов (Constructor Injection)
    pub fn new(
        embedding: Arc<dyn EmbeddingCoordinator>,
        search: Arc<dyn SearchCoordinator>,
        health: Arc<dyn HealthCoordinator>,
        promotion: Arc<dyn PromotionCoordinator>,
        resources: Arc<dyn ResourceCoordinator>,
        backup: Arc<dyn BackupCoordinator>,
        circuit_breaker_manager: Arc<CircuitBreakerManager>,
        metrics_collector: Arc<MetricsCollector>,
        operation_executor: Arc<OperationExecutor>,
        retry_handler: Arc<RetryHandler>,
    ) -> Self {
        info!("🚀 Создание OrchestratorCore с dependency injection");

        Self {
            embedding,
            search,
            health,
            promotion,
            resources,
            backup,
            circuit_breaker_manager,
            metrics_collector,
            operation_executor,
            retry_handler,
            ready: Arc::new(AtomicBool::new(false)),
            start_time: Instant::now(),
            operation_limiter: Arc::new(Semaphore::new(100)),
        }
    }

    /// Инициализация через делегацию
    pub async fn initialize(&self) -> Result<()> {
        info!("🔄 Инициализация оркестратора через делегацию");

        // Делегируем инициализацию специализированному компоненту
        self.operation_executor
            .initialize_all_coordinators(
                &*self.embedding,
                &*self.search,
                &*self.health,
                &*self.promotion,
                &*self.resources,
                &*self.backup,
            )
            .await?;

        self.ready.store(true, std::sync::atomic::Ordering::Release);
        info!("✅ Оркестратор инициализирован");

        Ok(())
    }

    /// Получить метрики через делегацию
    pub async fn get_metrics(&self) -> serde_json::Value {
        self.metrics_collector
            .collect_all_metrics(
                &*self.embedding,
                &*self.search,
                &*self.health,
                &*self.promotion,
                &*self.resources,
                &*self.backup,
                &self.circuit_breaker_manager,
                &self.start_time,
                &self.ready,
                &self.operation_limiter,
            )
            .await
    }

    /// Graceful shutdown через делегацию
    pub async fn shutdown(&self) -> Result<()> {
        info!("🛡️ Graceful shutdown через делегацию");

        self.ready
            .store(false, std::sync::atomic::Ordering::Release);

        self.operation_executor
            .shutdown_all_coordinators(
                &*self.embedding,
                &*self.search,
                &*self.health,
                &*self.promotion,
                &*self.resources,
                &*self.backup,
            )
            .await?;

        info!("✅ Оркестратор остановлен");
        Ok(())
    }

    /// Проверить готовность
    pub fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Acquire)
    }
}
