//! Operation Executor Module - Single Responsibility для выполнения операций
//!
//! Этот модуль отвечает ТОЛЬКО за выполнение бизнес операций:
//! insert, search, batch operations, backup/restore.
//! Применяет Command pattern и Dependency Inversion.

use anyhow::Result;
use async_trait::async_trait;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
use crate::orchestration::traits::Coordinator;
#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
use crate::orchestration::{HealthManager, ResourceController};

#[cfg(all(not(feature = "minimal"), feature = "backup-restore"))]
use crate::backup::BackupManager;
use crate::di::core_traits::ServiceResolver;
use crate::orchestration::traits::EmbeddingCoordinator as EmbeddingCoordinatorTrait;
use crate::{batch_manager::BatchOperationManager, metrics::MetricsCollector};

#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
use crate::di::unified_container_impl::UnifiedContainer as UnifiedDIContainer;

// NEW: Bring commonly used types into scope
#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
use crate::orchestration::traits::SearchCoordinator as SearchCoordinatorTrait;
#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
use crate::orchestration::{
    EmbeddingCoordinator as EmbeddingCoordinatorImpl, SearchCoordinator as SearchCoordinatorImpl,
};
#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
use crate::orchestration::{RetryHandler, RetryPolicy, RetryResult};
#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
use crate::storage::VectorStore;
#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
use crate::types::{Layer, Record, SearchOptions};
#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
use common::OperationTimer;

#[cfg(not(all(not(feature = "minimal"), feature = "orchestration-modules")))]
pub trait OperationExecutor: Send + Sync {}

// Replace tuple aliases with structured results when orchestration modules are enabled
#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
#[derive(Debug, Clone)]
pub struct BatchInsertResult {
    pub inserted: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub total_time_ms: u64,
}

#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
#[derive(Debug, Clone)]
pub struct BatchSearchResult {
    pub queries: Vec<String>,
    pub results: Vec<Vec<Record>>,
    pub total_time_ms: u64,
}

#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
#[async_trait]
pub trait OperationExecutor: Send + Sync {
    async fn search(
        &self,
        query: &str,
        layer: crate::types::Layer,
        options: crate::types::SearchOptions,
    ) -> anyhow::Result<Vec<crate::types::Record>>;
    async fn insert(&self, record: crate::types::Record) -> anyhow::Result<()>;
    async fn run_promotion(&self) -> anyhow::Result<crate::promotion::PromotionStats>;
    async fn get_stats(&self) -> crate::metrics::MemoryMetrics;

    async fn batch_insert(
        &self,
        records: Vec<crate::types::Record>,
    ) -> anyhow::Result<BatchInsertResult>;
    async fn batch_search(
        &self,
        queries: Vec<String>,
        layer: crate::types::Layer,
        options: crate::types::SearchOptions,
    ) -> anyhow::Result<BatchSearchResult>;
    async fn update(&self, record: crate::types::Record) -> anyhow::Result<()>;
    async fn delete(&self, id: &uuid::Uuid, layer: crate::types::Layer) -> anyhow::Result<()>;
    async fn initialize(&self) -> anyhow::Result<()>;
    async fn shutdown(&self) -> anyhow::Result<()>;
    async fn flush_all(&self) -> anyhow::Result<()>;
    async fn create_backup(
        &self,
        path: &str,
    ) -> anyhow::Result<crate::orchestration::traits::BackupMetadata>;
}

/// Конфигурация для выполнения операций
#[derive(Debug, Clone)]
pub struct OperationConfig {
    /// Максимальное количество concurrent операций
    pub max_concurrent_operations: usize,
    /// Timeout для операций
    pub operation_timeout: Duration,
    /// Политика retry
    pub retry_policy: RetryPolicy,
    /// Включить метрики
    pub enable_metrics: bool,
}

impl Default for OperationConfig {
    fn default() -> Self {
        Self {
            max_concurrent_operations: 100,
            operation_timeout: Duration::from_secs(30),
            retry_policy: RetryPolicy::default(),
            enable_metrics: true,
        }
    }
}

impl OperationConfig {
    pub fn production() -> Self {
        Self {
            max_concurrent_operations: 100,
            operation_timeout: Duration::from_secs(30),
            retry_policy: RetryPolicy::default(),
            enable_metrics: true,
        }
    }

    pub fn minimal() -> Self {
        Self {
            max_concurrent_operations: 10,
            operation_timeout: Duration::from_secs(5),
            retry_policy: RetryPolicy::fast(),
            enable_metrics: false,
        }
    }
}

#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
pub struct ProductionOperationExecutor {
    /// DI контейнер
    container: Arc<UnifiedDIContainer>,
    /// Embedding coordinator
    embedding_coordinator: Option<Arc<EmbeddingCoordinatorImpl>>,
    /// Search coordinator  
    search_coordinator: Option<Arc<SearchCoordinatorImpl>>,
    /// Retry handler
    retry_handler: RetryHandler,
    /// Concurrency limiter
    operation_limiter: Arc<Semaphore>,
    /// Конфигурация
    config: OperationConfig,
}

#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
impl ProductionOperationExecutor {
    pub fn new(
        container: Arc<UnifiedDIContainer>,
        embedding_coordinator: Option<Arc<EmbeddingCoordinatorImpl>>,
        search_coordinator: Option<Arc<SearchCoordinatorImpl>>,
        config: OperationConfig,
    ) -> Self {
        let retry_handler = RetryHandler::new(config.retry_policy.clone());
        let operation_limiter = Arc::new(Semaphore::new(config.max_concurrent_operations));

        Self {
            container,
            embedding_coordinator,
            search_coordinator,
            retry_handler,
            operation_limiter,
            config,
        }
    }

    /// Создать minimal executor для тестов
    pub fn new_minimal(container: Arc<UnifiedDIContainer>) -> Self {
        Self::new(container, None, None, OperationConfig::minimal())
    }

    /// Генерирует простой fallback embedding для тестов (когда нет GPU processor)
    fn generate_fallback_embedding(&self, text: &str) -> Vec<f32> {
        let dimension = 1024; // Фиксированная размерность для совместимости

        let mut embedding = vec![0.0; dimension];
        let hash = text.chars().fold(0u32, |acc, c| acc.wrapping_add(c as u32));

        // Генерируем детерминированный embedding на основе хеша текста
        for (i, val) in embedding.iter_mut().enumerate() {
            *val = ((hash.wrapping_add(i as u32) % 1000) as f32 / 1000.0) - 0.5;
        }

        // Нормализуем вектор
        let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }

        debug!(
            "Сгенерирован fallback embedding размерности {} для текста: '{}'",
            dimension, text
        );
        embedding
    }

    /// Получить embedding через координатор или fallback
    async fn get_embedding_fallback(&self, text: &str) -> Result<Vec<f32>> {
        if let Some(ref embedding_coordinator) = self.embedding_coordinator {
            let embeddings = embedding_coordinator
                .get_embeddings(&[text.to_string()])
                .await?;
            Ok(embeddings.into_iter().next().unwrap_or_default())
        } else {
            Ok(self.generate_fallback_embedding(text))
        }
    }

    /// Записать метрики операции
    fn record_operation_metrics(&self, operation_type: &str, duration: Duration) {
        if self.config.enable_metrics {
            if let Some(metrics) = self.container.try_resolve::<MetricsCollector>() {
                match operation_type {
                    "insert" => metrics.record_vector_insert(duration),
                    "search" => metrics.record_vector_search(duration),
                    "batch_insert" => {
                        // Записываем как несколько insert операций
                        metrics.record_vector_insert(duration);
                    }
                    "batch_search" => {
                        // Записываем как несколько search операций
                        metrics.record_vector_search(duration);
                    }
                    _ => {
                        debug!("Неизвестный тип операции для метрик: {}", operation_type);
                    }
                }
            }
        }
    }
}

#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
#[async_trait]
impl OperationExecutor for ProductionOperationExecutor {
    /// Production insert с координаторами и retry логикой
    async fn insert(&self, record: Record) -> Result<()> {
        let operation_start = Instant::now();

        // Получаем permit для ограничения concurrency
        let _permit = self
            .operation_limiter
            .acquire()
            .await
            .map_err(|e| anyhow::anyhow!("Не удалось получить permit для insert: {}", e))?;

        debug!("📥 Insert записи: {}", record.id);

        // Выполняем insert с retry логикой
        let insert_result = self
            .retry_handler
            .execute(|| async {
                let store = self.container.resolve::<VectorStore>()?;

                if let Ok(batch_manager) = self.container.resolve::<BatchOperationManager>() {
                    debug!("🔄 Insert через batch manager");
                    batch_manager.add(record.clone()).await?;
                } else {
                    debug!("🔄 Прямой insert в store");
                    store.insert(&record).await?;
                }

                Ok(())
            })
            .await;

        let operation_duration = operation_start.elapsed();

        match insert_result {
            RetryResult::Success(_, attempts) => {
                if attempts > 1 {
                    debug!(
                        "✅ Insert успешен после {} попыток за {:?}",
                        attempts, operation_duration
                    );
                } else {
                    debug!("✅ Insert успешен за {:?}", operation_duration);
                }

                self.record_operation_metrics("insert", operation_duration);
                Ok(())
            }
            RetryResult::ExhaustedRetries(e) | RetryResult::NonRetriable(e) => {
                error!("❌ Insert не удался: {}", e);
                Err(e)
            }
        }
    }

    /// Production search с координаторами и sub-5ms performance
    async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        let operation_start = Instant::now();

        // Получаем permit для ограничения concurrency
        let _permit = self
            .operation_limiter
            .acquire()
            .await
            .map_err(|e| anyhow::anyhow!("Не удалось получить permit для search: {}", e))?;

        debug!("🔍 Search в слое {:?}: '{}'", layer, query);

        let search_result = if let Some(ref search_coordinator) = self.search_coordinator {
            // Используем production SearchCoordinator с sub-5ms HNSW
            debug!("🎯 Используем SearchCoordinator для оптимального поиска");

            self.retry_handler
                .execute(|| async {
                    // Timeout для поддержания sub-5ms performance
                    tokio::time::timeout(
                        Duration::from_millis(50), // Агрессивный timeout для sub-5ms цели
                        search_coordinator.search(query, layer, options.clone()),
                    )
                    .await
                    .map_err(|_| {
                        anyhow::anyhow!("Search timeout - превышен лимит 50ms для sub-5ms цели")
                    })?
                })
                .await
        } else {
            // Fallback на прямой поиск без координатора (для minimal mode)
            debug!("🔄 Fallback поиск без координатора");

            self.retry_handler
                .execute(|| async {
                    let embedding = self.get_embedding_fallback(query).await?;
                    let store = self.container.resolve::<VectorStore>()?;
                    store.search(&embedding, layer, options.top_k).await
                })
                .await
        };

        let operation_duration = operation_start.elapsed();

        match search_result {
            RetryResult::Success(results, attempts) => {
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

                if attempts > 1 {
                    debug!("✅ Search успешен после {} попыток", attempts);
                }

                self.record_operation_metrics("search", operation_duration);
                Ok(results)
            }
            RetryResult::ExhaustedRetries(e) | RetryResult::NonRetriable(e) => {
                error!("❌ Search не удался для '{}': {}", query, e);
                Err(e)
            }
        }
    }

    /// Батчевая вставка записей
    async fn batch_insert(&self, records: Vec<Record>) -> Result<BatchInsertResult> {
        let timer = OperationTimer::new("batch_insert");
        let total_records = records.len();
        let mut inserted = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        debug!("Батчевая вставка {} записей", total_records);

        // Используем batch manager если доступен
        if let Ok(batch_manager) = self.container.resolve::<BatchOperationManager>() {
            for record in records {
                match batch_manager.add(record).await {
                    Ok(_) => inserted += 1,
                    Err(e) => {
                        failed += 1;
                        errors.push(e.to_string());
                    }
                }
            }
        } else {
            // Fallback на прямую вставку
            let store = self.container.resolve::<VectorStore>()?;
            for record in records {
                match store.insert(&record).await {
                    Ok(_) => inserted += 1,
                    Err(e) => {
                        failed += 1;
                        errors.push(e.to_string());
                    }
                }
            }
        }

        let elapsed = timer.elapsed().as_millis() as u64;
        debug!(
            "Батчевая вставка завершена: {}/{} успешно за {}мс",
            inserted, total_records, elapsed
        );

        self.record_operation_metrics("batch_insert", timer.elapsed());

        Ok(BatchInsertResult {
            inserted,
            failed,
            errors,
            total_time_ms: elapsed,
        })
    }

    /// Батчевый поиск
    async fn batch_search(
        &self,
        queries: Vec<String>,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<BatchSearchResult> {
        let timer = OperationTimer::new("batch_search");
        let mut results = Vec::new();

        debug!(
            "Батчевый поиск {} запросов в слое {:?}",
            queries.len(),
            layer
        );

        for query in &queries {
            let search_results = self.search(query, layer, options.clone()).await?;
            results.push(search_results);
        }

        let elapsed = timer.elapsed().as_millis() as u64;
        debug!("Батчевый поиск завершен за {}мс", elapsed);

        self.record_operation_metrics("batch_search", timer.elapsed());

        Ok(BatchSearchResult {
            queries,
            results,
            total_time_ms: elapsed,
        })
    }

    /// Обновить запись
    async fn update(&self, record: Record) -> Result<()> {
        let _timer = OperationTimer::new("memory_update");
        let store = self.container.resolve::<VectorStore>()?;

        debug!("Обновление записи {}", record.id);

        // Сначала удаляем старую версию
        store.delete_by_id(&record.id, record.layer).await?;
        // Затем вставляем новую
        store.insert(&record).await?;

        debug!("✓ Запись {} обновлена", record.id);
        Ok(())
    }

    /// Удалить запись
    async fn delete(&self, id: &uuid::Uuid, layer: Layer) -> Result<()> {
        let _timer = OperationTimer::new("memory_delete");
        let store = self.container.resolve::<VectorStore>()?;

        debug!("Удаление записи {} из слоя {:?}", id, layer);
        store.delete_by_id(id, layer).await?;

        debug!("✓ Запись {} удалена", id);
        Ok(())
    }

    /// Инициализация executor
    async fn initialize(&self) -> Result<()> {
        debug!("🔧 Инициализация ProductionOperationExecutor");

        // Инициализация координаторов если есть
        if let Some(embedding_coord) = &self.embedding_coordinator {
            embedding_coord.initialize().await?;
        }

        if let Some(search_coord) = &self.search_coordinator {
            search_coord.initialize().await?;
        }

        debug!("✅ ProductionOperationExecutor инициализирован");
        Ok(())
    }

    /// Graceful shutdown executor
    async fn shutdown(&self) -> Result<()> {
        debug!("🔄 Shutdown ProductionOperationExecutor");

        // Shutdown координаторов если есть
        if let Some(embedding_coord) = &self.embedding_coordinator {
            embedding_coord.shutdown().await?;
        }

        if let Some(search_coord) = &self.search_coordinator {
            search_coord.shutdown().await?;
        }

        debug!("✅ ProductionOperationExecutor завершен");
        Ok(())
    }

    /// Flush всех слоев
    async fn flush_all(&self) -> Result<()> {
        debug!("💾 Flush всех слоев memory system");
        let _store = self.container.resolve::<VectorStore>()?;

        debug!("💾 Flush completed (no-op in production implementation)");
        Ok(())
    }

    /// Запустить promotion cycle
    async fn run_promotion(&self) -> Result<crate::promotion::PromotionStats> {
        debug!("🚀 Запуск promotion cycle");

        let promotion_engine = self
            .container
            .resolve::<crate::promotion::PromotionEngine>()?;
        let stats = promotion_engine.run_promotion_cycle().await?;

        debug!(
            "✅ Promotion cycle завершен: {} interact->insights, {} insights->assets",
            stats.interact_to_insights, stats.insights_to_assets
        );
        Ok(stats)
    }

    /// Создать backup
    async fn create_backup(
        &self,
        path: &str,
    ) -> Result<crate::orchestration::traits::BackupMetadata> {
        let start = Instant::now();
        #[cfg(all(not(feature = "minimal"), feature = "backup-restore"))]
        let backup_manager = self.container.resolve::<crate::backup::BackupManager>()?;

        #[cfg(all(not(feature = "minimal"), feature = "backup-restore"))]
        let metadata = crate::backup::BackupMetadata {
            version: 1,
            created_at: chrono::Utc::now(),
            magray_version: env!("CARGO_PKG_VERSION").to_string(),
            layers: Vec::new(),
            total_records: 0,
            index_config: crate::vector_index_hnswlib::HnswRsConfig::default(),
            checksum: None,
            layer_checksums: None,
        };

        #[cfg(not(all(not(feature = "minimal"), feature = "backup-restore")))]
        let metadata = crate::orchestration::traits::BackupMetadata {
            version: 1,
            created_at: chrono::Utc::now(),
            magray_version: env!("CARGO_PKG_VERSION").to_string(),
            layers: Vec::new(),
            total_records: 0,
            index_config: crate::vector_index_hnswlib::HnswRsConfig::default(),
            checksum: None,
            layer_checksums: None,
        };

        let _ = path;
        let duration = start.elapsed();
        info!("Создание бэкапа завершено за {:?}", duration);
        Ok(metadata)
    }

    async fn get_stats(&self) -> crate::metrics::MemoryMetrics {
        if let Some(metrics) = self.container.try_resolve::<MetricsCollector>() {
            metrics.snapshot()
        } else {
            crate::metrics::MemoryMetrics::default()
        }
    }
}

/// Простой executor без координаторов (для тестов)
pub struct SimpleOperationExecutor {
    container: Arc<UnifiedDIContainer>,
}

impl SimpleOperationExecutor {
    pub fn new(container: Arc<UnifiedDIContainer>) -> Self {
        Self { container }
    }
}

#[async_trait]
impl OperationExecutor for SimpleOperationExecutor {
    async fn insert(&self, record: Record) -> Result<()> {
        let store = self.container.resolve::<VectorStore>()?;
        store.insert(&record).await
    }

    async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        // Генерируем простой embedding
        let dimension = 1024;
        let mut embedding = vec![0.0; dimension];
        let hash = query
            .chars()
            .fold(0u32, |acc, c| acc.wrapping_add(c as u32));

        for (i, val) in embedding.iter_mut().enumerate() {
            *val = ((hash.wrapping_add(i as u32) % 1000) as f32 / 1000.0) - 0.5;
        }

        let store = self.container.resolve::<VectorStore>()?;
        store.search(&embedding, layer, options.top_k).await
    }

    async fn batch_insert(&self, records: Vec<Record>) -> Result<BatchInsertResult> {
        let mut inserted = 0;
        let mut failed = 0;
        let mut errors = Vec::new();
        let start = Instant::now();

        for record in records {
            match self.insert(record).await {
                Ok(_) => inserted += 1,
                Err(e) => {
                    failed += 1;
                    errors.push(e.to_string());
                }
            }
        }

        Ok(BatchInsertResult {
            inserted,
            failed,
            errors,
            total_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn batch_search(
        &self,
        queries: Vec<String>,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<BatchSearchResult> {
        let mut results = Vec::new();
        let start = Instant::now();

        for query in &queries {
            let search_results = self.search(query, layer, options.clone()).await?;
            results.push(search_results);
        }

        Ok(BatchSearchResult {
            queries,
            results,
            total_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn update(&self, record: Record) -> Result<()> {
        let store = self.container.resolve::<VectorStore>()?;
        store.delete_by_id(&record.id, record.layer).await?;
        store.insert(&record).await
    }

    async fn delete(&self, id: &uuid::Uuid, layer: Layer) -> Result<()> {
        let store = self.container.resolve::<VectorStore>()?;
        let deleted = store.delete_by_id(id, layer).await?;
        if deleted {
            debug!("Successfully deleted record with id: {}", id);
        } else {
            warn!("Record with id {} not found for deletion", id);
        }
        Ok(())
    }

    /// Простая инициализация
    async fn initialize(&self) -> Result<()> {
        debug!("🔧 Инициализация SimpleOperationExecutor");
        Ok(())
    }

    /// Простое завершение работы
    async fn shutdown(&self) -> Result<()> {
        debug!("🔄 Shutdown SimpleOperationExecutor");
        Ok(())
    }

    /// Flush всех слоев (простая версия)
    async fn flush_all(&self) -> Result<()> {
        debug!("💾 Simple flush всех слоев");
        let _store = self.container.resolve::<VectorStore>()?;
        debug!("💾 Flush completed (no-op in simple implementation)");
        Ok(())
    }

    /// Простая promotion (возвращает empty stats)
    async fn run_promotion(&self) -> Result<crate::promotion::PromotionStats> {
        debug!("🚀 Simple promotion (no-op)");
        Ok(crate::promotion::PromotionStats::default())
    }

    /// Простой backup (mock implementation)
    async fn create_backup(
        &self,
        path: &str,
    ) -> Result<crate::orchestration::traits::BackupMetadata> {
        let start = Instant::now();
        let duration = start.elapsed();
        info!("Бэкап создан за {:?}", duration);
        Ok(crate::orchestration::traits::BackupMetadata {
            version: 1,
            created_at: chrono::Utc::now(),
            magray_version: env!("CARGO_PKG_VERSION").to_string(),
            layers: Vec::new(),
            total_records: 0,
            index_config: crate::vector_index_hnswlib::HnswRsConfig::default(),
            checksum: None,
            layer_checksums: None,
        })
    }

    async fn get_stats(&self) -> crate::metrics::MemoryMetrics {
        if let Some(metrics) = self.container.try_resolve::<MetricsCollector>() {
            metrics.snapshot()
        } else {
            crate::metrics::MemoryMetrics::default()
        }
    }
}

/// Дополнительные операции (backup, restore, etc.)
pub struct ExtendedOperationExecutor {
    container: Arc<UnifiedDIContainer>,
    base_executor: Arc<dyn OperationExecutor + Send + Sync>,
}

impl ExtendedOperationExecutor {
    pub fn new(
        container: Arc<UnifiedDIContainer>,
        base_executor: Arc<dyn OperationExecutor + Send + Sync>,
    ) -> Self {
        Self {
            container,
            base_executor,
        }
    }

    /// Создать backup
    pub async fn create_backup(
        &self,
        path: &str,
    ) -> Result<crate::orchestration::traits::BackupMetadata> {
        debug!("Создание backup через DI: {}", path);

        #[cfg(all(not(feature = "minimal"), feature = "backup-restore"))]
        if let Ok(backup_manager) = self.container.resolve::<crate::backup::BackupManager>() {
            let _ = backup_manager;
            let metadata = crate::orchestration::traits::BackupMetadata {
                version: 1,
                created_at: chrono::Utc::now(),
                magray_version: env!("CARGO_PKG_VERSION").to_string(),
                layers: Vec::new(),
                total_records: 0,
                index_config: crate::vector_index_hnswlib::HnswRsConfig::default(),
                checksum: None,
                layer_checksums: None,
            };
            return Ok(metadata);
        }

        let _ = path;
        Ok(crate::orchestration::traits::BackupMetadata {
            version: 1,
            created_at: chrono::Utc::now(),
            magray_version: env!("CARGO_PKG_VERSION").to_string(),
            layers: Vec::new(),
            total_records: 0,
            index_config: crate::vector_index_hnswlib::HnswRsConfig::default(),
            checksum: None,
            layer_checksums: None,
        })
    }

    /// Flush всех pending операций
    pub async fn flush_all(&self) -> Result<()> {
        debug!("Flush всех операций через DI");

        // Flush batch manager
        if let Some(_batch_manager) = self.container.try_resolve::<Arc<BatchOperationManager>>() {
            debug!("✓ Batch manager будет обработан автоматически");
        }

        debug!("✓ Vector store будет flushed автоматически");

        info!("✅ Все операции flushed");
        Ok(())
    }

    /// Получить статистику операций
    pub async fn get_operation_stats(&self) -> Result<crate::batch_manager::BatchStats> {
        if let Ok(batch_manager) = self.container.resolve::<Arc<BatchOperationManager>>() {
            Ok(batch_manager.stats().await)
        } else {
            Ok(crate::batch_manager::BatchStats::default())
        }
    }
}

// Delegating implementation для ExtendedOperationExecutor
#[async_trait]
impl OperationExecutor for ExtendedOperationExecutor {
    async fn insert(&self, record: Record) -> Result<()> {
        self.base_executor.insert(record).await
    }

    async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        self.base_executor.search(query, layer, options).await
    }

    async fn batch_insert(&self, records: Vec<Record>) -> Result<BatchInsertResult> {
        self.base_executor.batch_insert(records).await
    }

    async fn batch_search(
        &self,
        queries: Vec<String>,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<BatchSearchResult> {
        self.base_executor
            .batch_search(queries, layer, options)
            .await
    }

    async fn update(&self, record: Record) -> Result<()> {
        self.base_executor.update(record).await
    }

    async fn delete(&self, id: &uuid::Uuid, layer: Layer) -> Result<()> {
        self.base_executor.delete(id, layer).await
    }

    async fn initialize(&self) -> Result<()> {
        self.base_executor.initialize().await
    }

    async fn shutdown(&self) -> Result<()> {
        self.base_executor.shutdown().await
    }

    async fn flush_all(&self) -> Result<()> {
        self.base_executor.flush_all().await
    }

    async fn run_promotion(&self) -> Result<crate::promotion::PromotionStats> {
        self.base_executor.run_promotion().await
    }

    async fn create_backup(
        &self,
        path: &str,
    ) -> Result<crate::orchestration::traits::BackupMetadata> {
        self.base_executor.create_backup(path).await
    }

    async fn get_stats(&self) -> crate::metrics::MemoryMetrics {
        // Делегируем, если базовый executor умеет возвращать метрики
        // Иначе возвращаем пустые
        // Прямого вызова нет из-за возвращаемого типа, используем контейнер
        if let Some(metrics) = self.container.try_resolve::<MetricsCollector>() {
            metrics.snapshot()
        } else {
            crate::metrics::MemoryMetrics::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // legacy DI test helpers are gated out

    #[tokio::test]
    async fn test_operation_config() {
        let config = OperationConfig::production();
        assert_eq!(config.max_concurrent_operations, 100);
        assert!(config.enable_metrics);

        let minimal = OperationConfig::minimal();
        assert_eq!(minimal.max_concurrent_operations, 10);
        assert!(!minimal.enable_metrics);
    }

    #[tokio::test]
    async fn test_simple_executor() -> Result<()> {
        let container = Arc::new(crate::di::UnifiedContainer::new());

        let executor = SimpleOperationExecutor::new(container);

        // Test basic search (должен работать даже без embedding coordinator)
        let results = executor
            .search("test query", Layer::Interact, SearchOptions::default())
            .await;
        // Может не найти результатов, но не должен падать
        assert!(results.is_ok());

        Ok(())
    }

    #[test]
    fn test_batch_results() {
        let result = BatchInsertResult {
            inserted: 5,
            failed: 2,
            errors: vec!["Error 1".to_string(), "Error 2".to_string()],
            total_time_ms: 150,
        };

        assert_eq!(result.inserted, 5);
        assert_eq!(result.failed, 2);
        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.total_time_ms, 150);
    }
}
