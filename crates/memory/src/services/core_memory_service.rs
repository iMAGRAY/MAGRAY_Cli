//! CoreMemoryService - базовые операции с памятью
//!
//! Single Responsibility: только CRUD операции с данными
//! - insert/search/update/delete
//! - batch операции
//! - взаимодействие с VectorStore через DI

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

use crate::{
    di::UnifiedContainer,
    di::core_traits::TypeSafeResolver,
    orchestration::SearchCoordinator,
    types::Record,
    VectorStore,
    MetricsCollector,
    Layer,
    SearchOptions,
    BatchInsertResult,
    BatchSearchResult,
    CoreMemoryServiceTrait,
};
use crate::batch_manager::BatchOperationManager;
use common::OperationTimer;

/// Реализация core memory операций
/// Отвечает ТОЛЬКО за базовые операции с данными
#[allow(dead_code)]
pub struct CoreMemoryService {
    /// Type-safe resolver для разрешения зависимостей (объект-безопасный)
    resolver: TypeSafeResolver,
    /// Semaphore для ограничения concurrent операций
    operation_limiter: Arc<Semaphore>,
}

impl CoreMemoryService {
    /// Создать новый CoreMemoryService с type-safe resolver
    pub fn new(container: Arc<UnifiedContainer>, max_concurrent_operations: usize) -> Self {
        info!(
            "🗃️ Создание CoreMemoryService с лимитом {} concurrent операций и object-safe resolver",
            max_concurrent_operations
        );

        // Создаем type-safe resolver из контейнера
        let resolver = container.as_object_safe_resolver();

        Self {
            resolver,
            operation_limiter: Arc::new(Semaphore::new(max_concurrent_operations)),
        }
    }

    /// Создать минимальный вариант для тестов
    pub fn new_minimal(container: Arc<UnifiedContainer>) -> Self {
        Self::new(container, 10) // Небольшой лимит для тестов
    }

    /// Создать production вариант
    pub fn new_production(container: Arc<UnifiedContainer>) -> Self {
        Self::new(container, 100) // Высокий лимит для production
    }

    /// Получить VectorStore через type-safe resolver
    fn get_vector_store(&self) -> Result<Arc<VectorStore>> {
        self.resolver.resolve::<VectorStore>()
    }

    /// Получить BatchOperationManager если доступен
    #[allow(dead_code)]
    fn get_batch_manager(&self) -> Option<Arc<BatchOperationManager>> {
        self.resolver.try_resolve::<BatchOperationManager>()
    }

    /// Получить MetricsCollector если доступен  
    #[allow(dead_code)]
    fn get_metrics_collector(&self) -> Option<Arc<MetricsCollector>> {
        self.resolver.try_resolve::<MetricsCollector>()
    }
}

#[async_trait]
impl CoreMemoryServiceTrait for CoreMemoryService {
    /// Вставить одну запись
    #[allow(dead_code)]
    async fn insert(&self, record: Record) -> Result<()> {
        let _timer = OperationTimer::new("core_memory_insert");

        // Получаем permit для ограничения concurrency
        let _permit = self
            .operation_limiter
            .acquire()
            .await
            .map_err(|e| anyhow::anyhow!("Не удалось получить permit для insert: {}", e))?;

        debug!("🔄 CoreMemoryService: insert записи {}", record.id);

        let store = self.get_vector_store()?;

        // Используем batch manager если доступен
        if let Some(batch_manager) = self.get_batch_manager() {
            debug!("🔄 Insert через batch manager");
            batch_manager.add(record.clone()).await?;
        } else {
            debug!("🔄 Прямой insert в store");
            store.insert(&record).await?;
        }

        // Обновляем метрики
        if let Some(metrics) = self.get_metrics_collector() {
            let duration = std::time::Duration::from_millis(1); // Примерное время
            metrics.record_vector_insert(duration);
        }

        debug!("✅ CoreMemoryService: запись {} вставлена", record.id);
        Ok(())
    }

    /// Вставить несколько записей батчем
    #[allow(dead_code)]
    async fn insert_batch(&self, records: Vec<Record>) -> Result<()> {
        let _timer = OperationTimer::new("core_memory_insert_batch");
        let batch_size = records.len();

        debug!("🔄 CoreMemoryService: batch insert {} записей", batch_size);

        let store = self.get_vector_store()?;

        if let Some(batch_manager) = self.get_batch_manager() {
            batch_manager.add_batch(records).await?;
            debug!("✅ Batch обработан через batch manager");
        } else {
            // Fallback на прямую вставку
            let refs: Vec<&Record> = records.iter().collect();
            store.insert_batch(&refs).await?;
            debug!("✅ Batch обработан напрямую через store");
        }

        // Обновляем метрики
        if let Some(metrics) = self.get_metrics_collector() {
            let avg_time = std::time::Duration::from_millis(batch_size as u64);
            for _ in 0..batch_size {
                metrics.record_vector_insert(avg_time / batch_size as u32);
            }
        }

        info!(
            "✅ CoreMemoryService: {} записей вставлено батчем",
            batch_size
        );
        Ok(())
    }

    /// Поиск по запросу
    /// NOTE: Базовая реализация без координаторов, embedding генерируется fallback методом
    #[allow(dead_code)]
    async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        let _timer = OperationTimer::new("core_memory_search");

        // Получаем permit для ограничения concurrency
        let _permit = self
            .operation_limiter
            .acquire()
            .await
            .map_err(|e| anyhow::anyhow!("Не удалось получить permit для search: {}", e))?;

        debug!(
            "🔍 CoreMemoryService: поиск в слое {:?}: '{}'",
            layer, query
        );

        // Генерируем простой fallback embedding (без координаторов)
        let embedding = self.generate_simple_embedding(query);

        let store = self.get_vector_store()?;
        let results = store.search(&embedding, layer, options.top_k).await?;

        // Обновляем метрики
        if let Some(metrics) = self.get_metrics_collector() {
            let duration = std::time::Duration::from_millis(5); // Примерное время
            metrics.record_vector_search(duration);
        }

        debug!(
            "✅ CoreMemoryService: найдено {} результатов для '{}'",
            results.len(),
            query
        );
        Ok(results)
    }

    /// Обновить запись
    #[allow(dead_code)]
    async fn update(&self, record: Record) -> Result<()> {
        let _timer = OperationTimer::new("core_memory_update");
        let store = self.get_vector_store()?;

        debug!("🔄 CoreMemoryService: обновление записи {}", record.id);

        // Сначала удаляем старую версию
        store.delete_by_id(&record.id, record.layer).await?;
        // Затем вставляем новую
        store.insert(&record).await?;

        debug!("✅ CoreMemoryService: запись {} обновлена", record.id);
        Ok(())
    }

    /// Удалить запись
    #[allow(dead_code)]
    async fn delete(&self, id: &uuid::Uuid, layer: Layer) -> Result<()> {
        let _timer = OperationTimer::new("core_memory_delete");
        let store = self.get_vector_store()?;

        debug!(
            "🔄 CoreMemoryService: удаление записи {} из слоя {:?}",
            id, layer
        );
        store.delete_by_id(id, layer).await?;

        debug!("✅ CoreMemoryService: запись {} удалена", id);
        Ok(())
    }

    /// Батчевая вставка с подробными результатами
    #[allow(dead_code)]
    async fn batch_insert(&self, records: Vec<Record>) -> Result<BatchInsertResult> {
        let timer = OperationTimer::new("core_memory_batch_insert");
        let total_records = records.len();
        let mut inserted = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        debug!(
            "🔄 CoreMemoryService: батчевая вставка {} записей",
            total_records
        );

        // Используем batch manager если доступен
        if let Some(batch_manager) = self.get_batch_manager() {
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
            let store = self.get_vector_store()?;
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

        if failed > 0 {
            warn!(
                "⚠️ CoreMemoryService: батчевая вставка {}/{} успешно, {} ошибок за {}мс",
                inserted, total_records, failed, elapsed
            );
        } else {
            info!(
                "✅ CoreMemoryService: батчевая вставка {}/{} успешно за {}мс",
                inserted, total_records, elapsed
            );
        }

        Ok(BatchInsertResult {
            inserted,
            failed,
            errors,
            total_time_ms: elapsed,
        })
    }

    /// Батчевый поиск
    #[allow(dead_code)]
    async fn batch_search(
        &self,
        queries: Vec<String>,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<BatchSearchResult> {
        let timer = OperationTimer::new("core_memory_batch_search");
        let mut results = Vec::new();

        debug!(
            "🔍 CoreMemoryService: батчевый поиск {} запросов в слое {:?}",
            queries.len(),
            layer
        );

        for query in &queries {
            let search_results = self.search(query, layer, options.clone()).await?;
            results.push(search_results);
        }

        let elapsed = timer.elapsed().as_millis() as u64;
        info!(
            "✅ CoreMemoryService: батчевый поиск завершен за {}мс",
            elapsed
        );

        Ok(BatchSearchResult {
            queries,
            results,
            total_time_ms: elapsed,
        })
    }
}

impl CoreMemoryService {
    /// Генерировать простой embedding для fallback поиска
    /// Используется когда координаторы недоступны
    #[allow(dead_code)]
    fn generate_simple_embedding(&self, text: &str) -> Vec<f32> {
        // Определяем размерность из конфигурации (должно быть 1024 для наших тестов)
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
            "🔧 CoreMemoryService: сгенерирован simple embedding размерности {} для текста: '{}'",
            dimension, text
        );
        embedding
    }
}
