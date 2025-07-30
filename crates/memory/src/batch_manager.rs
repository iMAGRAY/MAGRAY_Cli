use anyhow::Result;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{debug, info, warn};

use crate::types::{Layer, Record};
use crate::storage::VectorStore;
use crate::metrics::MetricsCollector;

/// Configuration for batch operations
#[derive(Debug, Clone)]
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
            return Ok(());
        }
        
        // Group by layer
        let mut by_layer: HashMap<Layer, Vec<Record>> = HashMap::new();
        for record in records {
            by_layer.entry(record.layer).or_default().push(record);
        }
        
        // Add to pending batches
        let mut pending = self.pending_batches.write();
        let mut needs_flush = Vec::new();
        
        for (layer, mut new_records) in by_layer {
            let batch = pending.entry(layer).or_default();
            batch.append(&mut new_records);
            
            // Check if batch needs flushing
            if batch.len() >= self.config.max_batch_size {
                needs_flush.push(layer);
            }
        }
        
        // Update stats
        {
            let mut stats = self.stats.lock();
            stats.pending_records = pending.values().map(|v| v.len()).sum();
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
        let layers: Vec<Layer> = {
            let pending = self.pending_batches.read();
            pending.keys().cloned().collect()
        };
        
        for layer in layers {
            self.flush_layer(layer).await?;
        }
        
        Ok(())
    }
    
    /// Get current statistics
    pub fn stats(&self) -> BatchStats {
        self.stats.lock().clone()
    }
    
    // Private methods
    
    /// Flush a specific layer's batch
    async fn flush_layer(&self, layer: Layer) -> Result<()> {
        let batch = {
            let mut pending = self.pending_batches.write();
            pending.remove(&layer)
        };
        
        if let Some(records) = batch {
            if records.is_empty() {
                return Ok(());
            }
            
            let batch_size = records.len();
            debug!("Flushing batch of {} records for layer {:?}", batch_size, layer);
            
            if self.config.async_flush {
                // Send to background worker
                if let Some(sender) = &self.flush_sender {
                    let batch = RecordBatch {
                        layer,
                        records,
                    };
                    
                    if let Err(e) = sender.send(batch).await {
                        warn!("Failed to send batch to flush worker: {}", e);
                        return Err(anyhow::anyhow!("Batch flush channel closed"));
                    }
                }
            } else {
                // Flush synchronously
                let start = Instant::now();
                let refs: Vec<&Record> = records.iter().collect();
                self.store.insert_batch(&refs).await?;
                
                // Update stats
                self.update_stats(batch_size, start.elapsed());
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
            let permit = semaphore.clone().acquire_owned().await.unwrap();
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
                        let mut stats_guard = stats.lock();
                        stats_guard.total_batches += 1;
                        stats_guard.total_records += batch_size as u64;
                        
                        // Update moving average
                        let n = stats_guard.total_batches as f32;
                        stats_guard.avg_batch_size = 
                            (stats_guard.avg_batch_size * (n - 1.0) + batch_size as f32) / n;
                        stats_guard.avg_flush_time_ms = 
                            (stats_guard.avg_flush_time_ms * (n - 1.0) + duration.as_millis() as f32) / n;
                        
                        // Record metrics
                        if let Some(metrics) = &metrics {
                            for _ in 0..batch_size {
                                metrics.record_vector_insert(duration / batch_size as u32);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to flush batch: {}", e);
                        let mut stats_guard = stats.lock();
                        stats_guard.failed_batches += 1;
                        
                        if let Some(metrics) = &metrics {
                            metrics.record_error(format!("Batch flush failed: {}", e));
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
                    let mut pending_guard = pending.write();
                    let mut batches = Vec::new();
                    
                    for (layer, records) in pending_guard.drain() {
                        if !records.is_empty() {
                            batches.push(RecordBatch {
                                layer,
                                records,
                            });
                        }
                    }
                    
                    batches
                };
                
                for batch in batches_to_flush {
                    if let Err(e) = sender.send(batch).await {
                        warn!("Failed to send periodic batch: {}", e);
                        break;
                    }
                }
            }
        }
    }
    
    /// Update statistics after a flush
    fn update_stats(&self, batch_size: usize, duration: Duration) {
        let mut stats = self.stats.lock();
        stats.total_batches += 1;
        stats.total_records += batch_size as u64;
        
        // Update moving averages
        let n = stats.total_batches as f32;
        stats.avg_batch_size = 
            (stats.avg_batch_size * (n - 1.0) + batch_size as f32) / n;
        stats.avg_flush_time_ms = 
            (stats.avg_flush_time_ms * (n - 1.0) + duration.as_millis() as f32) / n;
        
        // Update pending count
        let pending = self.pending_batches.read();
        stats.pending_records = pending.values().map(|v| v.len()).sum();
        
        // Record metrics
        if let Some(metrics) = &self.metrics {
            for _ in 0..batch_size {
                metrics.record_vector_insert(duration / batch_size as u32);
            }
        }
    }
}

/// Builder for creating optimized batch operations
pub struct BatchOperationBuilder {
    records: Vec<Record>,
    config: Option<BatchConfig>,
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
    
    pub fn add_records(mut self, mut records: Vec<Record>) -> Self {
        self.records.append(&mut records);
        self
    }
    
    /// Group records by layer for optimal insertion
    pub fn group_by_layer(self) -> HashMap<Layer, Vec<Record>> {
        let mut grouped = HashMap::new();
        for record in self.records {
            grouped.entry(record.layer).or_insert_with(Vec::new).push(record);
        }
        grouped
    }
    
    /// Sort records within each layer by ID for better locality
    pub fn optimize_for_locality(mut self) -> Self {
        self.records.sort_by_key(|r| (r.layer, r.id));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;
    
    async fn create_test_store() -> Arc<VectorStore> {
        let temp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(temp_dir.path().join("test_vectors")).await.unwrap();
        
        // Initialize layers
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            store.init_layer(layer).await.unwrap();
        }
        
        Arc::new(store)
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
    async fn test_batch_manager_sync() {
        let store = create_test_store().await;
        let config = BatchConfig {
            max_batch_size: 10,
            async_flush: false,
            ..Default::default()
        };
        
        let manager = BatchOperationManager::new(store, config, None);
        
        // Add records
        for _ in 0..25 {
            manager.add(create_test_record(Layer::Interact)).await.unwrap();
        }
        
        // Check stats
        let stats = manager.stats();
        assert_eq!(stats.total_batches, 2); // Should have flushed 2 batches
        assert_eq!(stats.total_records, 20); // 2 * 10
        assert_eq!(stats.pending_records, 5); // 5 still pending
        
        // Flush remaining
        manager.flush_all().await.unwrap();
        
        let final_stats = manager.stats();
        assert_eq!(final_stats.total_batches, 3);
        assert_eq!(final_stats.total_records, 25);
        assert_eq!(final_stats.pending_records, 0);
    }
    
    #[tokio::test]
    async fn test_batch_builder() {
        let builder = BatchOperationBuilder::new()
            .add_record(create_test_record(Layer::Interact))
            .add_record(create_test_record(Layer::Insights))
            .add_record(create_test_record(Layer::Interact))
            .add_record(create_test_record(Layer::Assets));
        
        let grouped = builder.group_by_layer();
        
        assert_eq!(grouped.len(), 3);
        assert_eq!(grouped.get(&Layer::Interact).unwrap().len(), 2);
        assert_eq!(grouped.get(&Layer::Insights).unwrap().len(), 1);
        assert_eq!(grouped.get(&Layer::Assets).unwrap().len(), 1);
    }
}