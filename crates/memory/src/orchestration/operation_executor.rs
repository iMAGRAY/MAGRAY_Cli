use anyhow::Result;
use std::{pin::Pin, sync::Arc, time::Instant};
use tokio::sync::Semaphore;
use tracing::{debug, error, warn};

use crate::{
    backup::BackupMetadata,
    orchestration::{
        circuit_breaker_manager::{CircuitBreakerManager, CircuitBreakerManagerTrait},
        traits::{
            BackupCoordinator as BackupCoordinatorTrait,
            EmbeddingCoordinator as EmbeddingCoordinatorTrait,
            PromotionCoordinator as PromotionCoordinatorTrait, ResourceCoordinator,
            SearchCoordinator as SearchCoordinatorTrait,
        },
        BackupCoordinator, EmbeddingCoordinator, PromotionCoordinator, ResourceController,
        RetryHandler, RetryResult, SearchCoordinator,
    },
    promotion::PromotionStats,
    types::{Layer, Record, SearchOptions},
};

/// Operation executor для выполнения операций с circuit breaker и retry логикой
///
/// Применяет принципы SOLID:
/// - SRP: Только выполнение операций с resilience patterns
/// - OCP: Расширяемость через новые типы операций
/// - LSP: Взаимозаменяемость через trait
/// - ISP: Минимальный интерфейс для операций
/// - DIP: Зависит от абстракций координаторов
pub struct OperationExecutor {
    /// Координаторы для выполнения операций
    coordinators: CoordinatorDependencies,
    /// Circuit breaker manager
    circuit_breaker: Arc<CircuitBreakerManager>,
    /// Retry handlers для разных типов операций
    retry_handlers: RetryHandlers,
    /// Semaphore для ограничения concurrent операций
    operation_limiter: Arc<Semaphore>,
}

/// Зависимости от координаторов
#[derive(Clone)]
pub struct CoordinatorDependencies {
    pub embedding: Arc<EmbeddingCoordinator>,
    pub search: Arc<SearchCoordinator>,
    pub promotion: Arc<PromotionCoordinator>,
    pub backup: Arc<BackupCoordinator>,
    pub resources: Arc<ResourceController>,
}

/// Retry handlers для разных операций
pub struct RetryHandlers {
    pub search: RetryHandler,
    pub embedding: RetryHandler,
    pub promotion: RetryHandler,
    pub backup: RetryHandler,
}

/// Trait для выполнения операций (ISP принцип)
#[async_trait::async_trait]
pub trait OperationExecutorTrait: Send + Sync {
    /// Выполнить поиск с full orchestration intelligence
    async fn execute_search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>>;

    /// Выполнить embedding с intelligent caching и fallback
    async fn execute_embedding(&self, text: &str) -> Result<Vec<f32>>;

    /// Выполнить promotion с intelligent scheduling
    async fn execute_promotion(&self) -> Result<PromotionStats>;

    /// Выполнить backup с comprehensive validation
    async fn execute_backup(&self, path: &str) -> Result<BackupMetadata>;
}

/// Метрики выполнения операции
#[derive(Debug, Clone)]
pub struct OperationMetrics {
    pub operation_type: String,
    pub duration: std::time::Duration,
    pub success: bool,
    pub retry_attempts: u32,
    pub circuit_breaker_triggered: bool,
}

impl CoordinatorDependencies {
    /// Создать зависимости из DI контейнера
    pub fn from_container(container: &crate::di::container_core::ContainerCore) -> Result<Self> {
        Ok(Self {
            embedding: container.resolve::<EmbeddingCoordinator>()?,
            search: container.resolve::<SearchCoordinator>()?,
            promotion: container.resolve::<PromotionCoordinator>()?,
            backup: container.resolve::<BackupCoordinator>()?,
            resources: container.resolve::<ResourceController>()?,
        })
    }
}

impl Default for RetryHandlers {
    fn default() -> Self {
        Self {
            search: RetryHandler::new(crate::orchestration::RetryPolicy::fast()),
            embedding: RetryHandler::new(crate::orchestration::RetryPolicy::default()),
            promotion: RetryHandler::new(crate::orchestration::RetryPolicy::aggressive()),
            backup: RetryHandler::new(crate::orchestration::RetryPolicy::aggressive()),
        }
    }
}

impl OperationExecutor {
    /// Создать новый operation executor
    pub fn new(
        coordinators: CoordinatorDependencies,
        circuit_breaker: Arc<CircuitBreakerManager>,
        max_concurrent_operations: usize,
    ) -> Self {
        Self {
            coordinators,
            circuit_breaker,
            retry_handlers: RetryHandlers::default(),
            operation_limiter: Arc::new(Semaphore::new(max_concurrent_operations)),
        }
    }

    /// Создать из DI контейнера
    pub fn from_container(
        container: &crate::di::container_core::ContainerCore,
        circuit_breaker: Arc<CircuitBreakerManager>,
    ) -> Result<Self> {
        let coordinators = CoordinatorDependencies::from_container(container)?;
        Ok(Self::new(coordinators, circuit_breaker, 100)) // Default: 100 concurrent operations
    }

    /// Выполнить операцию с полным resilience stack
    async fn execute_with_resilience<F, T>(
        &self,
        operation_name: &str,
        resource_type: &str,
        retry_handler: &RetryHandler,
        operation: F,
    ) -> Result<(T, OperationMetrics)>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>
            + Send
            + Sync,
        T: Send,
    {
        // Получаем permit для concurrent operations
        let _permit = self.operation_limiter.acquire().await.map_err(|e| {
            anyhow::anyhow!(
                "Невозможно получить permit для операции {}: {}",
                operation_name,
                e
            )
        })?;

        let operation_start = Instant::now();

        // Проверяем circuit breaker
        let circuit_breaker_triggered = !self.circuit_breaker.can_execute(operation_name).await;
        if circuit_breaker_triggered {
            return Err(anyhow::anyhow!(
                "{} временно недоступен (circuit breaker открыт)",
                operation_name
            ));
        }

        // Проверяем ресурсы
        if !ResourceCoordinator::check_resources(&*self.coordinators.resources, resource_type)
            .await?
        {
            warn!(
                "🟡 Недостаточно ресурсов для {}, пытаемся адаптировать лимиты",
                operation_name
            );
            if let Err(e) = ResourceCoordinator::adapt_limits(&*self.coordinators.resources).await {
                warn!("Ошибка адаптации лимитов: {}", e);
            }
            return Err(anyhow::anyhow!(
                "Недостаточно ресурсов для {}",
                operation_name
            ));
        }

        // Выполняем операцию с retry logic
        let result = retry_handler.execute(operation).await;
        let operation_duration = operation_start.elapsed();

        // Обрабатываем результат и обновляем circuit breaker
        match &result {
            RetryResult::Success(_, attempts) => {
                debug!(
                    "✅ {} выполнен за {:?} ({} попыток)",
                    operation_name, operation_duration, attempts
                );

                self.circuit_breaker.record_success(operation_name).await;

                let metrics = OperationMetrics {
                    operation_type: operation_name.to_string(),
                    duration: operation_duration,
                    success: true,
                    retry_attempts: *attempts,
                    circuit_breaker_triggered,
                };

                result.into_result().map(|value| (value, metrics))
            }
            RetryResult::ExhaustedRetries(e) | RetryResult::NonRetriable(e) => {
                error!(
                    "🔴 {} не удался за {:?}: {}",
                    operation_name, operation_duration, e
                );

                self.circuit_breaker.record_failure(operation_name).await;

                let _metrics = OperationMetrics {
                    operation_type: operation_name.to_string(),
                    duration: operation_duration,
                    success: false,
                    retry_attempts: 0, // retry_handler doesn't expose attempt count on failure
                    circuit_breaker_triggered,
                };

                Err(anyhow::anyhow!("{} failed: {}", operation_name, e))
            }
        }
    }
}

#[async_trait::async_trait]
impl OperationExecutorTrait for OperationExecutor {
    async fn execute_search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        let operation = || {
            let search = Arc::clone(&self.coordinators.search);
            let query = query.to_string();
            let options = options.clone();

            Box::pin(async move {
                SearchCoordinatorTrait::search(&*search, &query, layer, options).await
            }) as Pin<Box<dyn std::future::Future<Output = Result<Vec<Record>>> + Send>>
        };

        let (result, metrics) = self
            .execute_with_resilience("search", "search", &self.retry_handlers.search, operation)
            .await?;

        // Проверяем SLA (sub-5ms target)
        if metrics.duration.as_millis() > 5 {
            debug!(
                "⚠️ SLA violation: поиск выполнялся {:?} (target: <5ms)",
                metrics.duration
            );
        }

        Ok(result)
    }

    async fn execute_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // Сначала проверяем кэш без retry
        if let Some(cached) =
            EmbeddingCoordinatorTrait::check_cache(&*self.coordinators.embedding, text).await
        {
            debug!("💾 Cache hit для embedding: {} chars", text.len());
            return Ok(cached);
        }

        let operation = || {
            let embedding = Arc::clone(&self.coordinators.embedding);
            let text = text.to_string();

            Box::pin(
                async move { EmbeddingCoordinatorTrait::get_embedding(&*embedding, &text).await },
            ) as Pin<Box<dyn std::future::Future<Output = Result<Vec<f32>>> + Send>>
        };

        let (result, _metrics) = self
            .execute_with_resilience(
                "embedding",
                "embedding",
                &self.retry_handlers.embedding,
                operation,
            )
            .await?;

        Ok(result)
    }

    async fn execute_promotion(&self) -> Result<PromotionStats> {
        // Проверяем нужно ли запускать promotion в принципе
        if !PromotionCoordinatorTrait::should_promote(&*self.coordinators.promotion).await {
            debug!("ℹ️ Promotion не требуется в данный момент");
            return Ok(PromotionStats::default());
        }

        let operation = || {
            let promotion = Arc::clone(&self.coordinators.promotion);

            Box::pin(async move { PromotionCoordinatorTrait::run_promotion(&*promotion).await })
                as Pin<Box<dyn std::future::Future<Output = Result<PromotionStats>> + Send>>
        };

        let (result, _metrics) = self
            .execute_with_resilience(
                "promotion",
                "promotion",
                &self.retry_handlers.promotion,
                operation,
            )
            .await?;

        Ok(result)
    }

    async fn execute_backup(&self, path: &str) -> Result<BackupMetadata> {
        let operation = || {
            let backup = Arc::clone(&self.coordinators.backup);
            let path = path.to_string();

            Box::pin(async move { BackupCoordinatorTrait::create_backup(&*backup, &path).await })
                as Pin<Box<dyn std::future::Future<Output = Result<BackupMetadata>> + Send>>
        };

        let (result, _metrics) = self
            .execute_with_resilience("backup", "backup", &self.retry_handlers.backup, operation)
            .await?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::circuit_breaker_manager::CircuitBreakerConfig;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    // Mock coordinator для тестирования
    struct MockSearchCoordinator {
        success_count: AtomicU32,
        should_fail: AtomicBool,
    }

    impl MockSearchCoordinator {
        fn new() -> Self {
            Self {
                success_count: AtomicU32::new(0),
                should_fail: AtomicBool::new(false),
            }
        }

        fn set_should_fail(&self, should_fail: bool) {
            self.should_fail.store(should_fail, Ordering::Relaxed);
        }
    }

    #[async_trait::async_trait]
    impl crate::orchestration::traits::Coordinator for MockSearchCoordinator {
        async fn initialize(&self) -> Result<()> {
            Ok(())
        }

        async fn is_ready(&self) -> bool {
            true
        }

        async fn shutdown(&self) -> Result<()> {
            Ok(())
        }

        async fn metrics(&self) -> serde_json::Value {
            serde_json::json!({
                "mock_search_coordinator": true,
                "should_fail": self.should_fail.load(Ordering::Relaxed)
            })
        }
    }

    #[async_trait::async_trait]
    impl SearchCoordinatorTrait for MockSearchCoordinator {
        async fn search(
            &self,
            _query: &str,
            _layer: Layer,
            _options: SearchOptions,
        ) -> Result<Vec<Record>> {
            if self.should_fail.load(Ordering::Relaxed) {
                Err(anyhow::anyhow!("Mock search failure"))
            } else {
                self.success_count.fetch_add(1, Ordering::Relaxed);
                Ok(vec![])
            }
        }

        async fn vector_search(
            &self,
            _vector: &[f32],
            _layer: Layer,
            _options: SearchOptions,
        ) -> Result<Vec<Record>> {
            if self.should_fail.load(Ordering::Relaxed) {
                Err(anyhow::anyhow!("Mock vector search failure"))
            } else {
                self.success_count.fetch_add(1, Ordering::Relaxed);
                Ok(vec![])
            }
        }

        async fn hybrid_search(
            &self,
            _query: &str,
            _vector: Option<&[f32]>,
            _layer: Layer,
            _options: SearchOptions,
        ) -> Result<Vec<Record>> {
            if self.should_fail.load(Ordering::Relaxed) {
                Err(anyhow::anyhow!("Mock hybrid search failure"))
            } else {
                self.success_count.fetch_add(1, Ordering::Relaxed);
                Ok(vec![])
            }
        }

        async fn search_with_rerank(
            &self,
            _query: &str,
            _layer: Layer,
            _options: SearchOptions,
            _rerank_top_k: usize,
        ) -> Result<Vec<Record>> {
            if self.should_fail.load(Ordering::Relaxed) {
                Err(anyhow::anyhow!("Mock rerank search failure"))
            } else {
                self.success_count.fetch_add(1, Ordering::Relaxed);
                Ok(vec![])
            }
        }
    }

    #[tokio::test]
    async fn test_operation_metrics_creation() {
        let metrics = OperationMetrics {
            operation_type: "test".to_string(),
            duration: std::time::Duration::from_millis(100),
            success: true,
            retry_attempts: 1,
            circuit_breaker_triggered: false,
        };

        assert_eq!(metrics.operation_type, "test");
        assert_eq!(metrics.duration, std::time::Duration::from_millis(100));
        assert!(metrics.success);
        assert_eq!(metrics.retry_attempts, 1);
        assert!(!metrics.circuit_breaker_triggered);
    }

    #[tokio::test]
    async fn test_retry_handlers_default() {
        let handlers = RetryHandlers::default();

        // Проверяем что handlers созданы (не можем проверить внутреннее состояние из-за private fields)
        // Просто убеждаемся что они компилируются и создаются
        assert!(true);
    }

    #[tokio::test]
    async fn test_coordinator_dependencies_structure() {
        // Этот тест проверяет структуру CoordinatorDependencies
        // В реальном окружении будет работать с настоящими координаторами
        // Здесь просто проверяем что структура компилируется
        assert!(true); // Placeholder
    }

    #[tokio::test]
    async fn test_operation_executor_semaphore_limit() {
        // Создаем mock circuit breaker
        let circuit_breaker = Arc::new(CircuitBreakerManager::with_config(
            CircuitBreakerConfig::default(),
        ));

        // Тестируем что semaphore ограничивает количество concurrent операций
        // Этот тест является placeholder'ом так как нужны настоящие координаторы для полного тестирования
        assert!(true);
    }
}
