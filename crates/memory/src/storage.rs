use anyhow::Result;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info};

use crate::metrics::{MetricsCollector, TimedOperation};
use crate::types::{Layer, Record};
use crate::vector_index_hnswlib::{VectorIndexHnswRs, HnswRsConfig};
// use crate::vector_index_hnsw_simple::{VectorIndexHnswSimple, HnswSimpleConfig};

// @component: {"k":"C","id":"vector_store","t":"Vector storage with HNSW","m":{"cur":65,"tgt":100,"u":"%"},"f":["storage","hnsw"]}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredRecord {
    pub record: Record,
}

// @component: VectorStore
// @file: crates/memory/src/storage.rs:16-290
// @status: WORKING
// @performance: O(log n) with HNSW index, O(n) fallback
// @dependencies: sled(✅), bincode(✅), instant-distance(✅)
// @tests: ❌ No performance tests
// @production_ready: 65%
// @issues: Index rebuild on batch insert, no incremental updates
// @upgrade_path: Add incremental index updates, performance tests
// @bottleneck: Index rebuild on batch operations
// @upgrade_effort: 1-2 days
pub struct VectorStore {
    db: Arc<Db>,
    indices: HashMap<Layer, Arc<VectorIndexHnswRs>>,
    metrics: Option<Arc<MetricsCollector>>,
}

impl VectorStore {
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db_path = db_path.as_ref();
        
        // Create directory if it doesn't exist
        if !db_path.exists() {
            std::fs::create_dir_all(db_path)?;
        }

        info!("Opening vector store at: {:?}", db_path);
        let db = sled::open(db_path)?;

        // Initialize indices for each layer with hnsw_rs config
        let mut indices = HashMap::new();
        let index_config = HnswRsConfig {
            dimension: 1024, // BGE-M3 фактическая размерность из config.json
            max_connections: 24,
            ef_construction: 400,
            ef_search: 100,
            max_elements: 100_000,
            max_layers: 16,
            use_parallel: true,
        };
        
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            let index = VectorIndexHnswRs::new(index_config.clone())?;
            indices.insert(layer, Arc::new(index));
        }

        Ok(Self {
            db: Arc::new(db),
            indices,
            metrics: None,
        })
    }

    /// Set the metrics collector
    pub fn set_metrics(&mut self, metrics: Arc<MetricsCollector>) {
        self.metrics = Some(metrics);
    }
    
    pub async fn init_layer(&self, layer: Layer) -> Result<()> {
        // Create tree for layer if it doesn't exist
        self.db.open_tree(layer.table_name())?;
        
        // Rebuild index for this layer
        self.rebuild_index(layer).await?;
        
        info!("Initialized layer {:?}", layer);
        Ok(())
    }
    
    /// Rebuild the vector index for a specific layer
    /// Now supports incremental updates - only rebuilds if necessary
    async fn rebuild_index(&self, layer: Layer) -> Result<()> {
        let tree = self.get_tree(layer).await?;
        
        if let Some(index) = self.indices.get(&layer) {
            let index_size = index.len();
            let tree_size = tree.len();
            
            // Only rebuild if there's a significant mismatch
            if index_size == 0 || (tree_size > 0 && index_size < tree_size / 2) {
                info!("Full rebuild needed for layer {:?}: index_size={}, tree_size={}", 
                      layer, index_size, tree_size);
                
                let mut embeddings = Vec::new();
                
                // Collect all embeddings from the layer
                for result in tree.iter() {
                    let (key, value) = result?;
                    if let Ok(stored) = bincode::deserialize::<StoredRecord>(&value) {
                        let id = String::from_utf8_lossy(&key).to_string();
                        embeddings.push((id, stored.record.embedding));
                    }
                }
                
                // Build the index using batch add
                index.clear(); // Clear existing data
                if !embeddings.is_empty() {
                    index.add_batch(embeddings)?;
                }
                debug!("Rebuilt index for layer {:?}", layer);
            } else {
                // Incremental sync: add missing records
                let mut missing_count = 0;
                let mut missing_embeddings = Vec::new();
                
                for result in tree.iter() {
                    let (key, value) = result?;
                    let id = String::from_utf8_lossy(&key).to_string();
                    
                    // Check if this ID exists in the index
                    if !index.contains(&id) {
                        if let Ok(stored) = bincode::deserialize::<StoredRecord>(&value) {
                            missing_embeddings.push((id, stored.record.embedding));
                            missing_count += 1;
                        }
                    }
                }
                
                if missing_count > 0 {
                    info!("Incremental update for layer {:?}: adding {} missing records", 
                          layer, missing_count);
                    index.add_batch(missing_embeddings)?;
                } else {
                    debug!("Index for layer {:?} is up to date", layer);
                }
            }
        }
        
        Ok(())
    }

    async fn get_tree(&self, layer: Layer) -> Result<sled::Tree> {
        Ok(self.db.open_tree(layer.table_name())?)
    }
    
    /// Public method to iterate over layer records for metrics
    pub async fn iter_layer(&self, layer: Layer) -> Result<impl Iterator<Item = sled::Result<(sled::IVec, sled::IVec)>>> {
        let tree = self.get_tree(layer).await?;
        Ok(tree.iter())
    }

    pub async fn insert(&self, record: &Record) -> Result<()> {
        // Start timing
        let _timer = self.metrics.as_ref().map(|m| TimedOperation::new(m, "vector_insert"));
        
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
        
        debug!("Inserted record {} into layer {:?}", record.id, record.layer);
        Ok(())
    }

    pub async fn insert_batch(&self, records: &[&Record]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        // Start timing
        let start = Instant::now();

        // Group by layer
        let mut by_layer: HashMap<Layer, Vec<&Record>> = HashMap::new();
        for record in records {
            by_layer.entry(record.layer).or_default().push(*record);
        }

        // Insert each layer's batch
        for (layer, layer_records) in by_layer {
            let tree = self.get_tree(layer).await?;
            let mut batch = sled::Batch::default();
            
            let mut embeddings = Vec::new();
            
            for record in &layer_records {
                let stored = StoredRecord {
                    record: (*record).clone(),
                };
                
                let key = record.id.as_bytes();
                let value = bincode::serialize(&stored)?;
                batch.insert(key, value);
                
                // Collect embeddings for index update
                embeddings.push((record.id.to_string(), record.embedding.clone()));
            }
            
            tree.apply_batch(batch)?;
            
            if let Some(index) = self.indices.get(&layer) {
                index.add_batch(embeddings)?;
            }
        }

        self.db.flush()?;
        
        // Record batch insert metrics
        if let Some(metrics) = &self.metrics {
            let duration = start.elapsed();
            for _ in records {
                metrics.record_vector_insert(duration / records.len() as u32);
            }
        }
        
        Ok(())
    }

    pub async fn search(
        &self,
        query_embedding: &[f32],
        layer: Layer,
        limit: usize,
    ) -> Result<Vec<Record>> {
        // Start timing
        let _timer = self.metrics.as_ref().map(|m| TimedOperation::new(m, "vector_search"));
        
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
                            debug!("Failed to deserialize record: {}", id_str);
                        }
                    } else {
                        debug!("Record not found in tree: {} (looked up UUID: {})", id_str, uuid);
                    }
                } else {
                    debug!("Failed to parse UUID from string: {}", id_str);
                }
            }
            
            info!("Search completed: {} records retrieved from layer {:?}", records.len(), layer);
            Ok(records)
        } else {
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

    pub async fn delete_expired(&self, layer: Layer, before: chrono::DateTime<chrono::Utc>) -> Result<usize> {
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
            if let Some(metrics) = &self.metrics {
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
            if let Some(metrics) = &self.metrics {
                metrics.record_vector_delete();
            }
        }
        
        Ok(existed)
    }
    
    /// Get records for promotion (high score, accessed frequently)
    pub async fn get_promotion_candidates(
        &self,
        layer: Layer,
        before: chrono::DateTime<chrono::Utc>,
        min_score: f32,
        min_access_count: u32,
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
        
        // Sort by promotion score (highest first)
        candidates.sort_by(|a, b| {
            let score_a = calculate_promotion_priority(a);
            let score_b = calculate_promotion_priority(b);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        debug!("Found {} promotion candidates in layer {:?}", candidates.len(), layer);
        Ok(candidates)
    }
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