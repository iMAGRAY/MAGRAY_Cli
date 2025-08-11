#![cfg(all(not(feature = "minimal"), feature = "persistence"))]

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::metrics::MetricsCollector;
use crate::storage::VectorStore;
use crate::types::{Layer, Record};

/// Configuration for batch operations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BatchConfig {
    /// Maximum batch size before automatic flush
    pub max_batch_size: usize,
    /// Maximum time to wait before flushing
    pub flush_interval: Duration,
    /// Number of worker threads for parallel processing
    pub worker_threads: usize,
    /// Enable async background flushing
    pub async_flush: bool,
    /// Maximum pending batches in queue
    pub max_queue_size: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 1000,
            flush_interval: Duration::from_secs(5),
            worker_threads: 4,
            async_flush: true,
            max_queue_size: 10,
        }
    }
}

impl BatchConfig {
    pub fn production() -> Self {
        Self {
            max_batch_size: 5000,                   // Большие батчи для производительности
            flush_interval: Duration::from_secs(2), // Более быстрая обработка
            worker_threads: 8,                      // Больше потоков для production
            async_flush: true,
            max_queue_size: 50, // Больше очередь для peak loads
        }
    }

    pub fn minimal() -> Self {
        Self {
            max_batch_size: 100,                     // Маленькие батчи
            flush_interval: Duration::from_secs(10), // Реже flush для экономии ресурсов
            worker_threads: 1,                       // Минимум потоков
            async_flush: false,                      // Синхронная обработка для простоты
            max_queue_size: 2,                       // Минимальная очередь
        }
    }
}

/// Statistics for batch operations
#[derive(Debug, Default, Clone)]
pub struct BatchStats {
    pub total_batches: u64,
    pub total_records: u64,
    pub failed_batches: u64,
    pub avg_batch_size: f32,
    pub avg_flush_time_ms: f32,
    pub pending_records: usize,
}

/// A batch of records grouped by layer
#[derive(Debug)]
struct RecordBatch {
    layer: Layer,
    records: Vec<Record>,
}

/// Manages efficient batch operations for the memory system
pub struct BatchOperationManager {
    store: Arc<VectorStore>,
    config: BatchConfig,
    pending_batches: Arc<RwLock<HashMap<Layer, Vec<Record>>>>,
    stats: Arc<Mutex<BatchStats>>,
    metrics: Option<Arc<MetricsCollector>>,
    flush_sender: Option<mpsc::Sender<RecordBatch>>,
}

impl BatchOperationManager {
    pub fn new(
        store: Arc<VectorStore>,
        config: BatchConfig,
        metrics: Option<Arc<MetricsCollector>>,
    ) -> Self {
        Self {
            store,
            config,
            pending_batches: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(Mutex::new(BatchStats::default())),
            metrics,
            flush_sender: None,
        }
    }

    /// Start the batch manager with background flushing
    pub async fn start(&mut self) -> Result<()> {
        if self.config.async_flush {
            let (tx, rx) = mpsc::channel::<RecordBatch>(self.config.max_queue_size);
            self.flush_sender = Some(tx);

            // Start background flush worker
            let store = self.store.clone();
            let stats = self.stats.clone();
            let metrics = self.metrics.clone();
            let config = self.config.clone();

            tokio::spawn(async move {
                Self::flush_worker(rx, store, stats, metrics, config).await;
            });

            // Start periodic flush timer
            let pending = self.pending_batches.clone();
            let sender = self.flush_sender.clone();
            let flush_interval = self.config.flush_interval;

            tokio::spawn(async move {
                Self::periodic_flush(pending, sender, flush_interval).await;
            });

            info!("Batch operation manager started with async flushing");
        } else {
            info!("Batch operation manager started in sync mode");
        }

        Ok(())
    }

    /// Add a single record to the batch
    pub async fn add(&self, record: Record) -> Result<()> {
        self.add_batch(vec![record]).await
    }

    /// Add multiple records to the batch
    pub async fn add_batch(&self, records: Vec<Record>) -> Result<()> {
        if records.is_empty() {
            debug!("Attempted to add empty batch, skipping");
            return Ok(());
        }

        debug!("Adding batch of {} records", records.len());

        // Group by layer
        let mut by_layer: HashMap<Layer, Vec<Record>> = HashMap::new();
        for record in records {
            by_layer.entry(record.layer).or_default().push(record);
        }

        let mut pending = self.pending_batches.write().await;
        let mut needs_flush = Vec::new();

        for (layer, mut new_records) in by_layer {
            let batch = pending.entry(layer).or_default();
            batch.append(&mut new_records);

            if batch.len() >= self.config.max_batch_size {
                needs_flush.push(layer);
            }
        }

        // Update stats with error handling
        {
            let pending_count: usize = pending.values().map(|v| v.len()).sum();
            let mut stats = self.stats.lock().await;
            stats.pending_records = pending_count;
            debug!("Updated pending records count to {}", pending_count);
        }

        drop(pending);

        // Flush full batches
        for layer in needs_flush {
            self.flush_layer(layer).await?;
        }

        Ok(())
    }

    /// Manually flush all pending batches
    pub async fn flush_all(&self) -> Result<()> {
        info!("Starting manual flush of all pending batches");
        let layers: Vec<Layer> = {
            let pending = self.pending_batches.read().await;
            let layer_count = pending.len();
            debug!("Found {} layers with pending batches", layer_count);
            pending.keys().cloned().collect()
        };

        let mut flush_errors = Vec::new();
        for layer in layers {
            if let Err(e) = self.flush_layer(layer).await {
                error!("Failed to flush layer {:?}: {}", layer, e);
                flush_errors.push((layer, e));
            } else {
                debug!("Successfully flushed layer {:?}", layer);
            }
        }

        if !flush_errors.is_empty() {
            let error_msg = format!(
                "Failed to flush {} layers: {:?}",
                flush_errors.len(),
                flush_errors
            );
            error!("{}", error_msg);
            return Err(anyhow::anyhow!(error_msg));
        }

        info!("Successfully flushed all pending batches");

        Ok(())
    }

    /// Get current statistics
    pub async fn stats(&self) -> BatchStats {
        self.stats.lock().await.clone()
    }

    // Private methods

    /// Flush a specific layer's batch
    async fn flush_layer(&self, layer: Layer) -> Result<()> {
        debug!("Starting flush for layer {:?}", layer);
        let batch = {
            let mut pending = match self.pending_batches.try_write() {
                Ok(guard) => guard,
                Err(_) => {
                    warn!("Failed to acquire write lock for flush_layer, using blocking write");
                    self.pending_batches.write().await
                }
            };
            pending.remove(&layer)
        };

        if let Some(records) = batch {
            if records.is_empty() {
                debug!("No records to flush for layer {:?}, skipping", layer);
                return Ok(());
            }

            if self.config.async_flush {
                // Send to background worker
                if let Some(sender) = &self.flush_sender {
                    let batch = RecordBatch {
                        layer,
                        records: records.clone(), // Clone to avoid move for fallback
                    };

                    if let Err(e) = sender.send(batch).await {
                        warn!("Failed to send batch to flush worker: {}", e);
                        // Try to flush synchronously as fallback
                        warn!(
                            "Attempting synchronous fallback flush for {} records",
                            records.len()
                        );
                        let start = Instant::now();
                        let refs: Vec<&Record> = records.iter().collect();
                        match self.store.insert_batch(&refs).await {
                            Ok(_) => {
                                info!(
                                    "Fallback synchronous flush successful for {} records",
                                    records.len()
                                );
                                self.update_stats(records.len(), start.elapsed());
                            }
                            Err(sync_err) => {
                                error!(
                                    "Both async and sync batch flush failed: async={}, sync={}",
                                    e, sync_err
                                );
                                return Err(anyhow::anyhow!(
                                    "Complete batch flush failure: {}",
                                    sync_err
                                ));
                            }
                        }
                    }
                }
            } else {
                // Flush synchronously with error handling
                let start = Instant::now();
                let refs: Vec<&Record> = records.iter().collect();
                match self.store.insert_batch(&refs).await {
                    Ok(_) => {
                        let duration = start.elapsed();
                        let batch_size = records.len();
                        info!(
                            "Synchronously flushed {} records for layer {:?} in {:?}",
                            batch_size, layer, duration
                        );
                        self.update_stats(batch_size, duration);
                    }
                    Err(e) => {
                        error!(
                            "Synchronous batch flush failed for layer {:?}: {}",
                            layer, e
                        );
                        // Update failed batch stats
                        if let Ok(mut stats) = self.stats.try_lock() {
                            stats.failed_batches += 1;
                        }
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Background worker for flushing batches
    async fn flush_worker(
        mut rx: mpsc::Receiver<RecordBatch>,
        store: Arc<VectorStore>,
        stats: Arc<Mutex<BatchStats>>,
        metrics: Option<Arc<MetricsCollector>>,
        config: BatchConfig,
    ) {
        info!("Batch flush worker started");

        // Process batches with parallelism
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config.worker_threads));

        while let Some(batch) = rx.recv().await {
            // Try to acquire semaphore permit with graceful error handling
            let permit = match semaphore.clone().acquire_owned().await {
                Ok(permit) => permit,
                Err(e) => {
                    error!(
                        "Failed to acquire semaphore permit for batch processing: {}",
                        e
                    );
                    // Update failed batch stats
                    let mut stats_guard = stats.lock().await;
                    stats_guard.failed_batches += 1;
                    if let Some(metrics) = &metrics {
                        metrics.record_error(format!("Semaphore acquisition failed: {e}"));
                    }
                    continue;
                }
            };
            let store = store.clone();
            let stats = stats.clone();
            let metrics = metrics.clone();

            tokio::spawn(async move {
                let start = Instant::now();
                let batch_size = batch.records.len();
                let layer = batch.layer;

                // Convert to references
                let refs: Vec<&Record> = batch.records.iter().collect();

                match store.insert_batch(&refs).await {
                    Ok(_) => {
                        let duration = start.elapsed();
                        debug!(
                            "Flushed {} records to layer {:?} in {:?}",
                            batch_size, layer, duration
                        );

                        // Update stats
                        let mut stats_guard = stats.lock().await;
                        stats_guard.total_batches += 1;
                        stats_guard.total_records += batch_size as u64;

                        // Update moving average
                        let n = stats_guard.total_batches as f32;
                        stats_guard.avg_batch_size =
                            (stats_guard.avg_batch_size * (n - 1.0) + batch_size as f32) / n;
                        stats_guard.avg_flush_time_ms = (stats_guard.avg_flush_time_ms * (n - 1.0)
                            + duration.as_millis() as f32)
                            / n;

                        // Record metrics
                        if let Some(metrics) = &metrics {
                            for _ in 0..batch_size {
                                metrics.record_vector_insert(duration / batch_size as u32);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to flush batch: {}", e);
                        let mut stats_guard = stats.lock().await;
                        stats_guard.failed_batches += 1;

                        if let Some(metrics) = &metrics {
                            metrics.record_error(format!("Batch flush failed: {e}"));
                        }
                    }
                }

                drop(permit);
            });
        }

        info!("Batch flush worker stopped");
    }

    /// Periodic flush timer
    async fn periodic_flush(
        pending: Arc<RwLock<HashMap<Layer, Vec<Record>>>>,
        sender: Option<mpsc::Sender<RecordBatch>>,
        flush_interval: Duration,
    ) {
        let mut interval = interval(flush_interval);

        loop {
            interval.tick().await;

            if let Some(sender) = &sender {
                let batches_to_flush = {
                    let mut pending_guard = pending.write().await;
                    let mut batches = Vec::new();

                    for (layer, records) in pending_guard.drain() {
                        if !records.is_empty() {
                            batches.push(RecordBatch { layer, records });
                        }
                    }

                    batches
                };

                for batch in batches_to_flush {
                    if let Err(e) = sender.send(batch).await {
                        warn!("Failed to send periodic batch: {}. Channel likely closed, stopping periodic flush", e);
                        error!("Periodic flush worker terminating due to channel closure");
                        break;
                    }
                }
            }
        }
    }

    /// Update statistics after a flush
    fn update_stats(&self, batch_size: usize, duration: Duration) {
        let stats_result = match self.stats.try_lock() {
            Ok(mut stats) => {
                stats.total_batches += 1;
                stats.total_records += batch_size as u64;

                // Update moving averages
                let n = stats.total_batches as f32;
                stats.avg_batch_size = (stats.avg_batch_size * (n - 1.0) + batch_size as f32) / n;
                stats.avg_flush_time_ms =
                    (stats.avg_flush_time_ms * (n - 1.0) + duration.as_millis() as f32) / n;

                debug!("Updated batch stats: total_batches={}, total_records={}, avg_batch_size={:.1}, avg_flush_time_ms={:.1}", 
                       stats.total_batches, stats.total_records, stats.avg_batch_size, stats.avg_flush_time_ms);
                Ok(())
            }
            Err(_) => {
                warn!("Failed to acquire stats lock for update, stats may be inconsistent");
                Err("Stats lock contention")
            }
        };

        // Update pending count with separate lock to avoid deadlock
        if stats_result.is_ok() {
            if let Ok(pending) = self.pending_batches.try_read() {
                let pending_count: usize = pending.values().map(|v| v.len()).sum();
                if let Ok(mut stats) = self.stats.try_lock() {
                    stats.pending_records = pending_count;
                }
            }
        }

        // Record metrics with error handling
        if let Some(metrics) = &self.metrics {
            for _ in 0..batch_size {
                metrics.record_vector_insert(duration / batch_size.max(1) as u32);
            }
        }
    }
}

/// Builder for creating optimized batch operations
pub struct BatchOperationBuilder {
    records: Vec<Record>,
    config: Option<BatchConfig>,
}

impl Default for BatchOperationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchOperationBuilder {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            config: None,
        }
    }

    pub fn with_config(mut self, config: BatchConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn add_record(mut self, record: Record) -> Self {
        self.records.push(record);
        self
    }

    pub fn add_records(mut self, records: Vec<Record>) -> Self {
        self.records.extend(records);
        self
    }

    pub fn group_by_layer(&self) -> HashMap<Layer, Vec<&Record>> {
        let mut grouped: HashMap<Layer, Vec<&Record>> = HashMap::new();
        for record in &self.records {
            grouped.entry(record.layer).or_default().push(record);
        }
        grouped
    }

    pub fn optimize_for_locality(mut self) -> Self {
        self.records.sort_by_key(|r| r.layer);
        self
    }

    pub fn build(self) -> Vec<Record> {
        self.records
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::VectorStore;
    use crate::types::{Layer, Record};
    use anyhow::Result;
    use uuid::Uuid;

    async fn create_test_store() -> Result<Arc<VectorStore>> {
        let temp_dir = std::env::temp_dir().join("test_batch_manager");
        std::fs::create_dir_all(&temp_dir)?;
        let store = VectorStore::new(&temp_dir).await?;
        Ok(Arc::new(store))
    }

    fn create_test_record(layer: Layer) -> Record {
        Record {
            id: Uuid::new_v4(),
            text: "Test record".to_string(),
            embedding: vec![0.1; 1024],
            layer,
            kind: "test".to_string(),
            tags: vec![],
            project: "test".to_string(),
            session: "test".to_string(),
            score: 0.8,
            ts: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
            access_count: 0,
        }
    }

    #[tokio::test]
    async fn test_batch_manager_sync() -> Result<()> {
        let store = create_test_store().await?;
        let config = BatchConfig {
            max_batch_size: 10,
            async_flush: false,
            ..Default::default()
        };

        let manager = BatchOperationManager::new(store, config, None);

        // Add records
        for i in 0..25 {
            manager
                .add(create_test_record(Layer::Interact))
                .await
                .map_err(|e| anyhow::anyhow!("Failed to add record {}: {}", i, e))?;
        }

        // Check stats
        let stats = manager.stats().await;
        assert_eq!(stats.total_batches, 2); // Should have flushed 2 batches
        assert_eq!(stats.total_records, 20); // 2 * 10
        assert_eq!(stats.pending_records, 5); // 5 still pending

        // Flush remaining
        manager
            .flush_all()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to flush remaining records: {}", e))?;

        let final_stats = manager.stats().await;
        assert_eq!(final_stats.total_batches, 3);
        assert_eq!(final_stats.total_records, 25);
        assert_eq!(final_stats.pending_records, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_batch_builder() -> Result<()> {
        let builder = BatchOperationBuilder::new()
            .add_record(create_test_record(Layer::Interact))
            .add_record(create_test_record(Layer::Insights))
            .add_record(create_test_record(Layer::Interact))
            .add_record(create_test_record(Layer::Assets));

        let grouped = builder.group_by_layer();

        assert_eq!(grouped.len(), 3);

        // Use proper error handling instead of unwrap
        let interact_records = grouped
            .get(&Layer::Interact)
            .ok_or_else(|| anyhow::anyhow!("Interact layer not found in grouped records"))?;
        assert_eq!(interact_records.len(), 2);

        let insights_records = grouped
            .get(&Layer::Insights)
            .ok_or_else(|| anyhow::anyhow!("Insights layer not found in grouped records"))?;
        assert_eq!(insights_records.len(), 1);

        let assets_records = grouped
            .get(&Layer::Assets)
            .ok_or_else(|| anyhow::anyhow!("Assets layer not found in grouped records"))?;
        assert_eq!(assets_records.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_batch_manager_error_resilience() -> Result<()> {
        let store = create_test_store().await?;
        let config = BatchConfig {
            max_batch_size: 5,
            async_flush: false,
            worker_threads: 2,
            ..Default::default()
        };

        let manager = BatchOperationManager::new(store, config, None);

        // Test adding empty batch
        manager.add_batch(vec![]).await?;
        let stats = manager.stats().await;
        assert_eq!(stats.total_batches, 0);

        // Test adding valid records
        let mut records = Vec::new();
        for i in 0..7 {
            let mut record = create_test_record(Layer::Interact);
            record.text = format!("Test record {}", i);
            records.push(record);
        }

        manager.add_batch(records).await?;

        // Check stats after batch addition
        let stats = manager.stats().await;

        // Since we added all 7 records at once, they might all be processed in one batch
        // or split depending on the internal logic
        if stats.total_batches == 1 {
            // All records processed at once
            assert_eq!(
                stats.total_records, 7,
                "Expected all 7 records to be processed"
            );
            assert_eq!(stats.pending_records, 0, "Expected 0 records to be pending");
        } else {
            // Records split into batches
            assert_eq!(stats.total_batches, 1, "Expected 1 batch to be flushed");
            assert_eq!(stats.total_records, 5, "Expected 5 records to be flushed");
            assert_eq!(stats.pending_records, 2, "Expected 2 records to be pending");
        }

        // Test manual flush of remaining
        manager.flush_all().await?;
        let final_stats = manager.stats().await;

        // Final check - all records should be processed and no pending
        assert_eq!(
            final_stats.total_records, 7,
            "Expected 7 total records after flush_all"
        );
        assert_eq!(
            final_stats.pending_records, 0,
            "Expected 0 pending records after flush_all"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_batch_builder_optimization() -> Result<()> {
        let builder = BatchOperationBuilder::new()
            .add_record(create_test_record(Layer::Assets))
            .add_record(create_test_record(Layer::Interact))
            .add_record(create_test_record(Layer::Insights))
            .add_record(create_test_record(Layer::Interact))
            .optimize_for_locality();

        let grouped = builder.group_by_layer();
        assert_eq!(grouped.len(), 3);

        // Verify all layers are present
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            assert!(
                grouped.contains_key(&layer),
                "Layer {:?} should be present",
                layer
            );
        }

        Ok(())
    }
}
