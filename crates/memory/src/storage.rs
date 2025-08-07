use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info, warn};

use crate::flush_config::FlushConfig;
use crate::metrics::{MetricsCollector, TimedOperation};
use crate::transaction::{TransactionGuard, TransactionManager};
use crate::types::{Layer, Record};
use crate::vector_index_hnswlib::{HnswRsConfig, VectorIndexHnswRs};
use crate::{
    health::{ComponentType, HealthMonitor},
    health_metric,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredRecord {
    pub record: Record,
}

pub struct VectorStore {
    db: Arc<Db>,
    indices: HashMap<Layer, Arc<VectorIndexHnswRs>>,
    metrics: Arc<RwLock<Option<Arc<MetricsCollector>>>>,
    health_monitor: Option<Arc<HealthMonitor>>,
    transaction_manager: Arc<TransactionManager>,
    // RwLock для координации batch операций и предотвращения race conditions
    batch_lock: Arc<RwLock<()>>,
    // Отслеживание изменений для инкрементальных обновлений
    change_tracker: Arc<RwLock<HashMap<Layer, ChangeTracker>>>,
    // Глобальный счетчик версий для отслеживания изменений
    version_counter: Arc<std::sync::atomic::AtomicU64>,
    // Журнал изменений для инкрементальных индексов
    change_log: Arc<RwLock<Vec<ChangeLogEntry>>>,
}

/// Запись в журнале изменений
#[derive(Debug, Clone)]
struct ChangeLogEntry {
    version: u64,
    layer: Layer,
    record: Record,
}

/// Отслеживает изменения в слое для умной синхронизации
#[derive(Debug)]
struct ChangeTracker {
    /// Последний известный размер дерева
    last_known_tree_size: usize,
    /// Последний известный размер индекса  
    last_known_index_size: usize,
    /// Время последней синхронизации
    last_sync_timestamp: std::time::Instant,
    /// Количество изменений с последней синхронизации
    pending_changes: usize,
}

impl Default for ChangeTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangeTracker {
    fn new() -> Self {
        Self {
            last_known_tree_size: 0,
            last_known_index_size: 0,
            last_sync_timestamp: std::time::Instant::now(),
            pending_changes: 0,
        }
    }

    fn record_change(&mut self) {
        self.pending_changes += 1;
    }

    fn needs_sync(&self, threshold: usize) -> bool {
        self.pending_changes >= threshold || self.last_sync_timestamp.elapsed().as_secs() > 300
        // 5 минут максимум
    }

    fn reset_after_sync(&mut self, tree_size: usize, index_size: usize) {
        self.last_known_tree_size = tree_size;
        self.last_known_index_size = index_size;
        self.last_sync_timestamp = std::time::Instant::now();
        self.pending_changes = 0;
    }
}

impl VectorStore {
    /// Открывает sled БД через DatabaseManager для предотвращения concurrent access issues
    fn open_database_with_recovery(
        db_path: impl AsRef<Path>,
        _flush_config: &FlushConfig,
    ) -> Result<Arc<Db>> {
        let db_manager = crate::database_manager::DatabaseManager::global();
        let db = db_manager.get_database(db_path.as_ref())?;

        // Проверяем целостность после открытия
        if let Err(e) = db.checksum() {
            error!("Database checksum failed: {}", e);
            info!("Attempting automatic recovery...");

            // Пытаемся восстановить БД
            // В sled recovery происходит автоматически при следующем открытии
            return Err(anyhow::anyhow!("Database corruption detected. Please restart the application for automatic recovery."));
        }

        info!("✅ Vector database opened through DatabaseManager");
        Ok(db)
    }

    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        Self::with_config(db_path, HnswRsConfig::default()).await
    }

    pub async fn with_config(
        db_path: impl AsRef<Path>,
        default_config: HnswRsConfig,
    ) -> Result<Self> {
        let db_path = db_path.as_ref();

        // Create directory if it doesn't exist
        if !db_path.exists() {
            std::fs::create_dir_all(db_path)?;
        }

        info!("Opening vector store at: {:?}", db_path);
        let flush_config = FlushConfig::from_env();
        let db = Self::open_database_with_recovery(db_path, &flush_config)?;

        // Initialize indices for each layer with hnsw_rs config
        let mut indices = HashMap::new();
        let mut change_trackers = HashMap::new();
        let index_config = default_config;

        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let index = VectorIndexHnswRs::new(index_config.clone())?;
            indices.insert(layer, Arc::new(index));
            change_trackers.insert(layer, ChangeTracker::new());
        }

        Ok(Self {
            db,
            indices,
            metrics: Arc::new(RwLock::new(None)),
            health_monitor: None,
            transaction_manager: Arc::new(TransactionManager::new()),
            batch_lock: Arc::new(RwLock::new(())),
            change_tracker: Arc::new(RwLock::new(change_trackers)),
            version_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            change_log: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Set the health monitor
    pub fn set_health_monitor(&mut self, health_monitor: Arc<HealthMonitor>) {
        self.health_monitor = Some(health_monitor);
    }

    pub async fn init_layer(&self, layer: Layer) -> Result<()> {
        // Create tree for layer if it doesn't exist
        self.db.open_tree(layer.table_name())?;

        // Rebuild index for this layer
        self.rebuild_index(layer).await?;

        info!("Initialized layer {:?}", layer);
        Ok(())
    }

    /// Smart incremental index synchronization - избегает O(n) операций
    async fn rebuild_index(&self, layer: Layer) -> Result<()> {
        let tree = self.get_tree(layer).await?;

        if let Some(index) = self.indices.get(&layer) {
            let index_size = index.len();
            let tree_size = tree.len();

            // Только полная перестройка при КРИТИЧЕСКОЙ рассинхронизации
            if index_size == 0 && tree_size > 100 {
                info!(
                    "Critical desync detected for layer {:?}: rebuilding {} records",
                    layer, tree_size
                );

                // Batch-загрузка для минимизации времени блокировки
                let mut embeddings = Vec::with_capacity(tree_size.min(10000)); // Лимит памяти
                let mut batch_count = 0;

                for result in tree.iter() {
                    let (key, value) = result?;
                    if let Ok(stored) = bincode::deserialize::<StoredRecord>(&value) {
                        let id = String::from_utf8_lossy(&key).to_string();
                        embeddings.push((id, stored.record.embedding));

                        // Обрабатываем батчами для предотвращения OOM
                        if embeddings.len() >= 5000 {
                            if batch_count == 0 {
                                index.clear(); // Очищаем только один раз
                            }
                            index.add_batch(embeddings.clone())?;
                            embeddings.clear();
                            batch_count += 1;
                            debug!("Processed batch {} for layer {:?}", batch_count, layer);
                        }
                    }
                }

                // Финальный batch
                if !embeddings.is_empty() {
                    if batch_count == 0 {
                        index.clear();
                    }
                    index.add_batch(embeddings)?;
                }

                info!(
                    "Full rebuild completed for layer {:?}: {} batches",
                    layer,
                    batch_count + 1
                );
            } else {
                // УМНАЯ инкрементальная синхронизация - O(delta) вместо O(n)
                self.smart_incremental_sync(layer, index).await?;
            }
        }

        Ok(())
    }

    /// Умная инкрементальная синхронизация - только недостающие записи
    async fn smart_incremental_sync(
        &self,
        layer: Layer,
        index: &Arc<VectorIndexHnswRs>,
    ) -> Result<()> {
        let tree = self.get_tree(layer).await?;
        let mut sync_operations = Vec::new();
        let mut checked_count = 0;

        // Проверяем только новые записи (используем cursor для оптимизации)
        for result in tree.iter() {
            checked_count += 1;

            // Батчим проверки для снижения lock contention
            if checked_count % 100 == 0 {
                tokio::task::yield_now().await; // Позволяем другим задачам работать
            }

            let (key, value) = result?;
            let id = String::from_utf8_lossy(&key).to_string();

            // Быстрая проверка существования в индексе
            if !index.contains(&id) {
                if let Ok(stored) = bincode::deserialize::<StoredRecord>(&value) {
                    sync_operations.push((id, stored.record.embedding));

                    // Ограничиваем размер batch'а для контроля памяти
                    if sync_operations.len() >= 1000 {
                        break;
                    }
                }
            }
        }

        if !sync_operations.is_empty() {
            info!(
                "Smart sync for layer {:?}: adding {} missing records (checked {} total)",
                layer,
                sync_operations.len(),
                checked_count
            );
            index.add_batch(sync_operations)?;
        } else {
            debug!("Layer {:?} index is fully synchronized", layer);
        }

        Ok(())
    }

    pub async fn get_tree(&self, layer: Layer) -> Result<sled::Tree> {
        Ok(self.db.open_tree(layer.table_name())?)
    }

    /// Public method to iterate over layer records for metrics
    pub async fn iter_layer(
        &self,
        layer: Layer,
    ) -> Result<impl Iterator<Item = sled::Result<(sled::IVec, sled::IVec)>>> {
        let tree = self.get_tree(layer).await?;
        Ok(tree.iter())
    }

    pub async fn insert(&self, record: &Record) -> Result<()> {
        let _start = Instant::now();

        // Проверяем лимиты перед вставкой
        self.check_insert_limits(1)?;

        // Start timing
        let metrics = self.metrics.read().clone();
        let _timer = metrics
            .as_ref()
            .map(|m| TimedOperation::new(m, "vector_insert"));

        let tree = self.get_tree(record.layer).await?;

        let stored = StoredRecord {
            record: record.clone(),
        };

        let key = record.id.as_bytes();
        let value = bincode::serialize(&stored)?;
        tree.insert(key, value)?;

        // Add to vector index
        if let Some(index) = self.indices.get(&record.layer) {
            index.add(record.id.to_string(), record.embedding.clone())?;
        }

        // Отслеживаем изменение для умной синхронизации
        self.record_layer_change(record.layer);

        // Логируем изменение для версионирования
        self.log_change(record.layer, record);

        self.db.flush()?;

        Ok(())
    }

    pub async fn search(
        &self,
        query_embedding: &[f32],
        layer: Layer,
        limit: usize,
    ) -> Result<Vec<Record>> {
        let start = Instant::now();

        // Start timing
        let metrics = self.metrics.read().clone();
        let _timer = metrics
            .as_ref()
            .map(|m| TimedOperation::new(m, "vector_search"));

        // Use the new vector index which handles linear vs HNSW automatically
        if let Some(index) = self.indices.get(&layer) {
            let results = index.search(query_embedding, limit)?;

            // Get full records for the results
            let tree = self.get_tree(layer).await?;
            let mut records = Vec::new();

            for (id_str, score) in results {
                // Parse UUID from string
                if let Ok(uuid) = uuid::Uuid::parse_str(&id_str) {
                    if let Some(value) = tree.get(uuid.as_bytes())? {
                        if let Ok(stored) = bincode::deserialize::<StoredRecord>(&value) {
                            let mut record = stored.record;
                            record.score = score;
                            records.push(record);
                        } else {
                            debug!("Failed to deserialize record: {}", &id_str);
                        }
                    } else {
                        debug!(
                            "Record not found in tree: {} (looked up UUID: {})",
                            &id_str, uuid
                        );
                    }
                } else {
                    debug!("Failed to parse UUID from string: {}", &id_str);
                }
            }

            let duration = start.elapsed();
            let success = true;

            // Record health metrics
            if let Some(ref health) = self.health_monitor {
                health.record_operation(
                    ComponentType::VectorStore,
                    success,
                    duration.as_secs_f64() * 1000.0,
                    None,
                );

                // Record specific search latency metric
                let search_latency_metric = health_metric!(
                    ComponentType::VectorStore,
                    "search_latency_ms",
                    duration.as_secs_f64() * 1000.0,
                    "ms",
                    100.0, // Warning: > 100ms
                    500.0  // Critical: > 500ms
                );
                let _ = health.record_metric(search_latency_metric);

                // Record result count
                let result_count_metric = health_metric!(
                    ComponentType::VectorStore,
                    "search_result_count",
                    records.len() as f64,
                    "count"
                );
                let _ = health.record_metric(result_count_metric);
            }

            info!(
                "Search completed: {} records retrieved from layer {:?} in {:.2}ms",
                records.len(),
                layer,
                duration.as_secs_f64() * 1000.0
            );
            Ok(records)
        } else {
            let duration = start.elapsed();

            // Record failed operation
            if let Some(ref health) = self.health_monitor {
                health.record_operation(
                    ComponentType::VectorStore,
                    false,
                    duration.as_secs_f64() * 1000.0,
                    Some("No index found for layer".to_string()),
                );
            }

            info!("No index found for layer {:?}", layer);
            Ok(Vec::new())
        }
    }

    pub async fn update_access(&self, layer: Layer, id: &str) -> Result<()> {
        let tree = self.get_tree(layer).await?;

        if let Some(value) = tree.get(id.as_bytes())? {
            if let Ok(mut stored) = bincode::deserialize::<StoredRecord>(&value) {
                stored.record.access_count += 1;
                stored.record.last_access = chrono::Utc::now();

                let new_value = bincode::serialize(&stored)?;
                tree.insert(id.as_bytes(), new_value)?;
            }
        }

        Ok(())
    }

    pub async fn delete_expired(
        &self,
        layer: Layer,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<usize> {
        let tree = self.get_tree(layer).await?;
        let mut count = 0;
        let mut to_delete = Vec::new();

        for result in tree.iter() {
            let (key, value) = result?;
            if let Ok(stored) = bincode::deserialize::<StoredRecord>(&value) {
                if stored.record.ts < before {
                    to_delete.push(key.to_vec());
                    count += 1;
                }
            }
        }

        for key in to_delete {
            tree.remove(key)?;
        }

        // Record expired deletions
        if count > 0 {
            if let Some(metrics) = &*self.metrics.read() {
                metrics.record_expired(count as u64);
            }
        }

        Ok(count)
    }

    pub async fn get_by_id(&self, id: &uuid::Uuid, layer: Layer) -> Result<Option<Record>> {
        let tree = self.get_tree(layer).await?;

        if let Some(value) = tree.get(id.as_bytes())? {
            if let Ok(stored) = bincode::deserialize::<StoredRecord>(&value) {
                return Ok(Some(stored.record));
            }
        }

        Ok(None)
    }

    /// Delete a record by ID
    pub async fn delete_by_id(&self, id: &uuid::Uuid, layer: Layer) -> Result<bool> {
        let tree = self.get_tree(layer).await?;
        let key = id.as_bytes();

        let existed = tree.remove(key)?.is_some();

        // Also remove from vector index
        if existed {
            if let Some(index) = self.indices.get(&layer) {
                let _ = index.remove(&id.to_string());
            }

            // Record delete metric
            if let Some(metrics) = &*self.metrics.read() {
                metrics.record_vector_delete();
            }
        }

        Ok(existed)
    }

    /// Get records for promotion (high score, accessed frequently)
    /// DEPRECATED: Use PromotionEngine.find_candidates_by_time() for O(log n) performance
    #[deprecated(
        note = "This method uses O(n) scanning. Use PromotionEngine for better performance"
    )]
    pub async fn get_promotion_candidates(
        &self,
        layer: Layer,
        before: chrono::DateTime<chrono::Utc>,
        min_score: f32,
        min_access_count: u32,
        limit: Option<usize>,
    ) -> Result<Vec<Record>> {
        let tree = self.get_tree(layer).await?;
        let mut candidates = Vec::new();

        for result in tree.iter() {
            let (_, value) = result?;
            if let Ok(stored) = bincode::deserialize::<StoredRecord>(&value) {
                let record = &stored.record;

                // Check all criteria
                if record.ts < before
                    && record.score >= min_score
                    && record.access_count >= min_access_count
                {
                    candidates.push(record.clone());
                }
            }
        }

        // Sort by promotion score (highest first) with proper NaN handling
        candidates.sort_by(|a, b| {
            let score_a = calculate_promotion_priority(a);
            let score_b = calculate_promotion_priority(b);
            score_b.partial_cmp(&score_a).unwrap_or_else(|| {
                warn!(
                    "NaN values detected in promotion scoring for records {} and {}",
                    a.id, b.id
                );
                if score_a.is_nan() && score_b.is_nan() {
                    std::cmp::Ordering::Equal
                } else if score_a.is_nan() {
                    std::cmp::Ordering::Less // NaN records go to end (lower priority)
                } else {
                    std::cmp::Ordering::Greater
                }
            })
        });

        // Apply limit if specified
        if let Some(limit) = limit {
            candidates.truncate(limit);
        }

        debug!(
            "Found {} promotion candidates in layer {:?}",
            candidates.len(),
            layer
        );
        Ok(candidates)
    }

    /// Вставить несколько записей (batch operation)
    pub async fn insert_batch(&self, records: &[&Record]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        // Group records by layer for efficient processing
        let mut records_by_layer: HashMap<Layer, Vec<&Record>> = HashMap::new();
        for record in records {
            records_by_layer
                .entry(record.layer)
                .or_default()
                .push(record);
        }

        for (layer, layer_records) in records_by_layer {
            let tree = self.get_tree(layer).await?;

            // Prepare data for batch insertion
            let mut stored_records = Vec::new();
            let mut vector_batch = Vec::new();

            for record in &layer_records {
                let key = record.id.to_string();
                let stored = StoredRecord {
                    record: (*record).clone(),
                };
                let value = bincode::serialize(&stored)?;

                // Store in database
                tree.insert(key.as_bytes(), value)?;

                // Prepare for vector index
                stored_records.push((key.clone(), (*record).clone()));
                vector_batch.push((key, record.embedding.clone()));
            }

            // Add batch to vector index if it exists
            if let Some(index) = self.indices.get(&layer) {
                index.add_batch(vector_batch)?;
            }

            // Update change tracker
            if let Some(tracker) = self.change_tracker.write().get_mut(&layer) {
                for _ in &layer_records {
                    tracker.record_change();
                }
            }

            // Record metrics (estimate duration)
            let duration = std::time::Duration::from_millis(1); // Приблизительно
            if let Some(metrics) = &*self.metrics.read() {
                for _ in &layer_records {
                    metrics.record_vector_insert(duration);
                }
            }
        }

        Ok(())
    }

    /// Начать транзакцию
    pub fn begin_transaction(&self) -> Result<TransactionGuard<'_>> {
        TransactionGuard::new(&self.transaction_manager)
    }

    /// Выполнить операции транзакции
    #[allow(clippy::await_holding_lock)]
    pub async fn insert_batch_atomic(&self, records: &[&Record]) -> Result<()> {
        // Используем batch lock для предотвращения race conditions
        let _guard = self.batch_lock.write();

        self.insert_batch(records).await
    }

    /// Получить статистику транзакций
    pub fn transaction_stats(&self) -> usize {
        self.transaction_manager.active_count()
    }

    /// Установить максимальное количество элементов для всех индексов
    pub async fn set_max_elements(&mut self, max_elements: usize) -> Result<()> {
        info!(
            "Setting max elements limit to {} for all layers",
            max_elements
        );

        // Создаём новые индексы с обновленным лимитом
        let mut new_indices = HashMap::new();

        for (layer, old_index) in &self.indices {
            // Получаем текущую конфигурацию
            let mut new_config = old_index.config().clone();
            new_config.max_elements = max_elements;

            // Создаём новый индекс
            let new_index = VectorIndexHnswRs::new(new_config)?;

            // Переносим существующие данные если они есть
            if !old_index.is_empty() {
                info!(
                    "Migrating {} vectors from layer {:?} to new index",
                    old_index.len(),
                    layer
                );

                // Собираем все векторы из дерева
                let tree = self.get_tree(*layer).await?;
                let mut vectors_to_migrate = Vec::new();

                for result in tree.iter() {
                    let (key, value) = result?;
                    if let Ok(stored) = bincode::deserialize::<StoredRecord>(&value) {
                        let id = String::from_utf8_lossy(&key).to_string();
                        if old_index.contains(&id) {
                            vectors_to_migrate.push((id, stored.record.embedding));
                        }
                    }
                }

                // Добавляем в новый индекс
                if !vectors_to_migrate.is_empty() {
                    new_index.add_batch(vectors_to_migrate)?;
                }
            }

            new_indices.insert(*layer, Arc::new(new_index));
        }

        // Заменяем старые индексы новыми
        self.indices = new_indices;

        info!(
            "Successfully reconfigured all indices with new max_elements: {}",
            max_elements
        );
        Ok(())
    }

    /// Получить текущую статистику использования памяти
    pub fn memory_stats(&self) -> MemoryStats {
        let mut total_vectors = 0;
        let mut layer_stats = HashMap::new();

        for (layer, index) in &self.indices {
            let count = index.len();
            total_vectors += count;

            layer_stats.insert(
                *layer,
                LayerMemoryStats {
                    vector_count: count,
                    estimated_memory_mb: (count * 1024 * 4 / 1024 / 1024) as f64, // Приблизительно
                },
            );
        }

        MemoryStats {
            total_vectors,
            layer_stats,
            estimated_total_memory_mb: (total_vectors * 1024 * 4 / 1024 / 1024) as f64,
        }
    }

    /// Проверить, не превышены ли лимиты памяти
    pub fn check_memory_limits(&self, max_vectors: usize) -> Result<()> {
        let stats = self.memory_stats();

        if stats.total_vectors >= max_vectors {
            return Err(anyhow::anyhow!(
                "Vector limit exceeded: {} >= {} max",
                stats.total_vectors,
                max_vectors
            ));
        }

        Ok(())
    }

    /// Проверить лимиты перед вставкой новых записей
    fn check_insert_limits(&self, count: usize) -> Result<()> {
        // Проверяем каждый индекс отдельно
        for index in self.indices.values() {
            let config = index.config();
            let current = index.len();
            let new_total = current + count;

            if new_total > config.max_elements {
                return Err(anyhow::anyhow!(
                    "Index capacity exceeded: {} + {} > {} max elements",
                    current,
                    count,
                    config.max_elements
                ));
            }
        }

        Ok(())
    }

    /// Получить процент заполненности индексов
    pub fn capacity_usage(&self) -> HashMap<Layer, f64> {
        let mut usage = HashMap::new();

        for (layer, index) in &self.indices {
            let current = index.len() as f64;
            let max = index.config().max_elements as f64;
            let percent = if max > 0.0 {
                (current / max) * 100.0
            } else {
                0.0
            };
            usage.insert(*layer, percent);
        }

        usage
    }
}

#[derive(Debug)]
pub struct MemoryStats {
    pub total_vectors: usize,
    pub layer_stats: HashMap<Layer, LayerMemoryStats>,
    pub estimated_total_memory_mb: f64,
}

#[derive(Debug)]
pub struct LayerMemoryStats {
    pub vector_count: usize,
    pub estimated_memory_mb: f64,
}

/// Calculate promotion priority based on multiple factors
fn calculate_promotion_priority(record: &Record) -> f32 {
    use chrono::Utc;

    // Age factor (newer is better for promotion)
    let age_hours = (Utc::now() - record.ts).num_hours() as f32;
    let age_factor = 1.0 / (1.0 + age_hours / 168.0); // Decay over a week

    // Access factor (more access is better)
    let access_factor = (record.access_count as f32).ln_1p() / 10.0;

    // Recency of access (recent access is better)
    let access_recency_hours = (Utc::now() - record.last_access).num_hours() as f32;
    let recency_factor = 1.0 / (1.0 + access_recency_hours / 24.0);

    // Combined score with weights
    record.score * 0.4 + access_factor * 0.3 + recency_factor * 0.2 + age_factor * 0.1
}

impl VectorStore {
    /// Отслеживает изменение в слое для умной синхронизации
    fn record_layer_change(&self, layer: Layer) {
        if let Some(mut trackers) = self.change_tracker.try_write() {
            if let Some(tracker) = trackers.get_mut(&layer) {
                tracker.record_change();
            }
        }
    }

    /// Получить текущую версию данных
    pub fn get_version(&self) -> u64 {
        self.version_counter
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Получить изменения с определенной версии
    pub async fn get_changes_since(
        &self,
        since_version: u64,
    ) -> Result<HashMap<Layer, Vec<Record>>> {
        let change_log = self.change_log.read();
        let mut changes: HashMap<Layer, Vec<Record>> = HashMap::new();

        for entry in change_log.iter() {
            if entry.version > since_version {
                // Все операции сейчас - Insert
                changes
                    .entry(entry.layer)
                    .or_default()
                    .push(entry.record.clone());
            }
        }

        Ok(changes)
    }

    /// Получить общее количество записей во всех слоях
    pub async fn get_total_count(&self) -> Result<usize> {
        let mut total = 0;

        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let tree = self.get_tree(layer).await?;
            total += tree.len();
        }

        Ok(total)
    }

    /// Итерировать по записям слоя для индексации
    pub async fn iter_layer_records(&self, layer: Layer) -> Result<Vec<Record>> {
        let tree = self.get_tree(layer).await?;
        let mut records = Vec::new();

        for result in tree.iter() {
            let (_, value) = result?;
            if let Ok(stored) = bincode::deserialize::<StoredRecord>(&value) {
                records.push(stored.record);
            }
        }

        Ok(records)
    }

    /// Записать изменение в журнал
    fn log_change(&self, layer: Layer, record: &Record) {
        let version = self
            .version_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;

        if let Some(mut log) = self.change_log.try_write() {
            // Ограничиваем размер журнала (храним последние 10000 записей)
            if log.len() > 10000 {
                log.drain(0..5000);
            }

            log.push(ChangeLogEntry {
                version,
                layer,
                record: record.clone(),
            });
        }
    }

    /// Умная синхронизация только при необходимости
    pub async fn smart_sync_if_needed(&self, layer: Layer) -> Result<()> {
        let needs_sync = {
            let trackers = self.change_tracker.read();
            trackers
                .get(&layer)
                .map(|t| t.needs_sync(50)) // Синхронизируем при 50+ изменениях
                .unwrap_or(false)
        };

        if needs_sync {
            if let Some(index) = self.indices.get(&layer) {
                self.smart_incremental_sync(layer, index).await?;

                // Обновляем tracker после успешной синхронизации
                let tree = self.get_tree(layer).await?;
                let tree_size = tree.len();
                let index_size = index.len();

                let mut trackers = self.change_tracker.write();
                if let Some(tracker) = trackers.get_mut(&layer) {
                    tracker.reset_after_sync(tree_size, index_size);
                }
            }
        }

        Ok(())
    }
}
