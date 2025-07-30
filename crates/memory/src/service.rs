use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info};

use crate::{
    cache::EmbeddingCache,
    metrics::{MetricsCollector, LayerMetrics},
    promotion::{PromotionEngine, PromotionStats},
    promotion_optimized::{OptimizedPromotionEngine, OptimizedPromotionStats},
    storage::VectorStore,
    types::{Layer, PromotionConfig, Record, SearchOptions},
};

use ai::{AiConfig, OptimizedEmbeddingService, ModelLoader, RerankingService};

pub struct MemoryService {
    store: Arc<VectorStore>,
    cache: Arc<EmbeddingCache>,
    promotion: Arc<PromotionEngine>,
    optimized_promotion: Arc<OptimizedPromotionEngine>,
    embedding_service: Arc<OptimizedEmbeddingService>,
    reranking_service: Option<Arc<RerankingService>>,
    metrics: Option<Arc<MetricsCollector>>,
}

pub struct MemoryConfig {
    pub db_path: PathBuf,
    pub cache_path: PathBuf,
    pub promotion: PromotionConfig,
    pub ai_config: AiConfig,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        let base_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ourcli");

        Self {
            db_path: base_dir.join("hnswdb"), // Изменено с lancedb на hnswdb
            cache_path: base_dir.join("cache").join("embeddings"),
            promotion: PromotionConfig::default(),
            ai_config: AiConfig::default(),
        }
    }
}

impl MemoryService {
    pub async fn new(config: MemoryConfig) -> Result<Self> {
        info!("Initializing memory service");

        // Initialize storage
        let store = Arc::new(VectorStore::new(&config.db_path).await?);
        
        // Initialize all layer tables
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            store.init_layer(layer).await?;
        }

        // Initialize cache
        let cache = Arc::new(EmbeddingCache::new(&config.cache_path)?);

        // Initialize AI services with fallback to mock
        let model_loader = ModelLoader::new(&config.ai_config.models_dir)?;
        
        let embedding_service = match OptimizedEmbeddingService::new(
            config.ai_config.embedding.clone(),
        ) {
            Ok(service) => {
                info!("Optimized embedding service initialized successfully");
                Arc::new(service)
            }
            Err(e) => {
                debug!("Failed to initialize embedding service: {}", e);
                return Err(anyhow::anyhow!("Failed to initialize embedding service: {}", e));
            }
        };

        // Try to initialize reranking service with fallback to mock
        let reranking_service = match RerankingService::new(
            &model_loader,
            config.ai_config.reranking.clone(),
        ) {
            Ok(service) => {
                info!("Real reranking service initialized successfully");
                Some(Arc::new(service))
            }
            Err(e) => {
                debug!("Failed to initialize real reranking service: {}, trying mock", e);
                match RerankingService::new_mock(config.ai_config.reranking.clone()) {
                    Ok(mock_service) => Some(Arc::new(mock_service)),
                    Err(mock_e) => {
                        debug!("Failed to initialize mock reranking service: {}, continuing without reranking", mock_e);
                        None
                    }
                }
            }
        };

        // Initialize promotion engines (both legacy and optimized)
        let promotion = Arc::new(PromotionEngine::new(
            store.clone(),
            config.promotion.clone(),
        ));

        // Initialize optimized promotion engine with time-based indexing
        let promotion_db = sled::open(config.db_path.join("promotion_indices"))?;
        let optimized_promotion = Arc::new(OptimizedPromotionEngine::new(
            store.clone(),
            config.promotion,
            Arc::new(promotion_db)
        ).await?);

        info!("✅ OptimizedPromotionEngine successfully integrated into MemoryService");

        Ok(Self {
            store,
            cache,
            promotion,
            optimized_promotion,
            embedding_service,
            reranking_service,
            metrics: None,
        })
    }

    pub async fn insert(&self, mut record: Record) -> Result<()> {
        // Generate embedding if not provided
        if record.embedding.is_empty() {
            record.embedding = self.get_or_compute_embedding(&record.text).await?;
        }

        // Set defaults
        if record.id == uuid::Uuid::nil() {
            record.id = uuid::Uuid::new_v4();
        }
        if record.ts == chrono::DateTime::<chrono::Utc>::default() {
            record.ts = chrono::Utc::now();
        }
        record.last_access = record.ts;

        debug!("Inserting record {} into layer {:?}", record.id, record.layer);
        self.store.insert(&record).await?;

        Ok(())
    }

    pub async fn insert_batch(&self, records: Vec<Record>) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        // Process records to add embeddings
        let processed = futures::future::join_all(
            records.into_iter().map(|mut record| async move {
                if record.embedding.is_empty() {
                    record.embedding = self.get_or_compute_embedding(&record.text).await?;
                }
                if record.id == uuid::Uuid::nil() {
                    record.id = uuid::Uuid::new_v4();
                }
                if record.ts == chrono::DateTime::<chrono::Utc>::default() {
                    record.ts = chrono::Utc::now();
                }
                record.last_access = record.ts;
                Ok::<_, anyhow::Error>(record)
            })
        ).await;

        let processed: Vec<_> = processed.into_iter().collect::<Result<_>>()?;
        
        info!("Inserting batch of {} records", processed.len());
        self.store.insert_batch(&processed.iter().collect::<Vec<_>>()).await?;

        Ok(())
    }

    pub fn search(&self, query: &str) -> SearchBuilder<'_> {
        SearchBuilder::new(self, query.to_string())
    }

    pub async fn search_with_options(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        let query_embedding = self.get_or_compute_embedding(query).await?;
        
        let mut all_results = Vec::new();

        // Search each requested layer
        for layer in &options.layers {
            let mut results = self.store
                .search(&query_embedding, *layer, options.top_k)
                .await?;

            // Apply additional filters
            if !options.tags.is_empty() {
                results.retain(|r| {
                    options.tags.iter().any(|tag| r.tags.contains(tag))
                });
            }

            if let Some(ref project) = options.project {
                results.retain(|r| &r.project == project);
            }

            if options.score_threshold > 0.0 {
                results.retain(|r| r.score >= options.score_threshold);
            }

            all_results.extend(results);
        }

        // Sort by initial vector score  
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Second stage: reranking if available
        let final_results = if let Some(ref reranker) = self.reranking_service {
            if all_results.len() > 1 {
                debug!("Applying reranking to {} candidates", all_results.len());
                
                // Get more candidates for reranking
                let rerank_candidates = all_results.iter().take((options.top_k * 3).min(100)).cloned().collect::<Vec<_>>();
                
                // Extract texts for reranking
                let documents: Vec<String> = rerank_candidates
                    .iter()
                    .map(|r| r.text.clone())
                    .collect();

                // Rerank
                match reranker.rerank(query, &documents) {
                    Ok(rerank_results) => {
                        // Map reranked results back to records
                        let mut reranked_records = Vec::new();
                        for rerank_result in rerank_results.iter().take(options.top_k) {
                            if let Some(record) = rerank_candidates.get(rerank_result.original_index) {
                                let mut updated_record = record.clone();
                                updated_record.score = rerank_result.score;
                                reranked_records.push(updated_record);
                            }
                        }
                        reranked_records
                    }
                    Err(e) => {
                        debug!("Reranking failed: {}, using vector search results", e);
                        all_results.into_iter().take(options.top_k).collect()
                    }
                }
            } else {
                all_results.into_iter().take(options.top_k).collect()
            }
        } else {
            all_results.into_iter().take(options.top_k).collect()
        };

        // Update access stats (in production, this would be batched)
        for result in &final_results {
            self.store.update_access(result.layer, &result.id.to_string()).await?;
        }

        Ok(final_results)
    }

    pub async fn get_by_id(&self, id: &uuid::Uuid, layer: Layer) -> Result<Option<Record>> {
        self.store.get_by_id(id, layer).await
    }

    pub async fn run_promotion_cycle(&self) -> Result<PromotionStats> {
        info!("Running legacy promotion cycle");
        let start = std::time::Instant::now();
        
        let stats = self.promotion.run_promotion_cycle().await?;
        
        if let Some(ref metrics) = self.metrics {
            metrics.record_promotion_cycle(start.elapsed());
            metrics.record_promotion("interact", "insights", stats.interact_to_insights as u64);
            metrics.record_promotion("insights", "assets", stats.insights_to_assets as u64);
            metrics.record_expired((stats.expired_interact + stats.expired_insights) as u64);
        }
        
        Ok(stats)
    }

    /// Run optimized promotion cycle with time-based indexing
    pub async fn run_optimized_promotion_cycle(&self) -> Result<OptimizedPromotionStats> {
        info!("🚀 Running optimized promotion cycle with time-based indexing");
        let start = std::time::Instant::now();
        
        let stats = self.optimized_promotion.run_optimized_promotion_cycle().await?;
        
        if let Some(ref metrics) = self.metrics {
            metrics.record_promotion_cycle(start.elapsed());
            metrics.record_promotion("interact", "insights", stats.interact_to_insights as u64);
            metrics.record_promotion("insights", "assets", stats.insights_to_assets as u64);
            metrics.record_expired((stats.expired_interact + stats.expired_insights) as u64);
        }
        
        info!("✅ Optimized promotion cycle completed in {}ms", stats.total_time_ms);
        Ok(stats)
    }

    /// Get performance statistics from optimized promotion engine
    pub async fn get_promotion_performance_stats(&self) -> Result<crate::promotion_optimized::PromotionPerformanceStats> {
        self.optimized_promotion.get_performance_stats().await
    }

    pub fn cache_stats(&self) -> (u64, u64, u64) {
        self.cache.stats()
    }

    pub fn cache_hit_rate(&self) -> f64 {
        self.cache.hit_rate()
    }

    /// Enable metrics collection
    pub fn enable_metrics(&mut self) -> Arc<MetricsCollector> {
        let metrics = Arc::new(MetricsCollector::new());
        self.metrics = Some(metrics.clone());
        
        // Pass metrics to storage
        if let Some(store) = Arc::get_mut(&mut self.store) {
            store.set_metrics(metrics.clone());
        }
        
        metrics
    }

    /// Get metrics if enabled
    pub fn metrics(&self) -> Option<Arc<MetricsCollector>> {
        self.metrics.clone()
    }

    /// Collect and update layer metrics
    pub async fn update_layer_metrics(&self) -> Result<()> {
        if let Some(ref metrics) = self.metrics {
            for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
                let iter = self.store.iter_layer(layer).await?;
                let mut count = 0u64;
                let mut size = 0u64;
                let mut access_sum = 0u32;
                let mut oldest_ts = chrono::Utc::now();
                
                for item in iter {
                    if let Ok((_, value)) = item {
                        count += 1;
                        size += value.len() as u64;
                        
                        // Local struct for deserialization
                        #[derive(serde::Deserialize)]
                        struct StoredRecord {
                            record: Record,
                        }
                        if let Ok(stored) = bincode::deserialize::<StoredRecord>(&value) {
                            access_sum += stored.record.access_count;
                            if stored.record.ts < oldest_ts {
                                oldest_ts = stored.record.ts;
                            }
                        }
                    }
                }
                
                let layer_metrics = LayerMetrics {
                    record_count: count,
                    total_size_bytes: size,
                    avg_embedding_size: if count > 0 { 1024.0 } else { 0.0 }, // BGE-M3 размерность
                    avg_access_count: if count > 0 { access_sum as f32 / count as f32 } else { 0.0 },
                    oldest_record_age_hours: (chrono::Utc::now() - oldest_ts).num_hours() as f32,
                };
                
                let layer_name = match layer {
                    Layer::Interact => "interact",
                    Layer::Insights => "insights",
                    Layer::Assets => "assets",
                };
                metrics.update_layer_metrics(layer_name, layer_metrics);
            }
        }
        Ok(())
    }

    async fn get_or_compute_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let model = "default"; // In real impl, this would come from config

        // Check cache first
        if let Some(embedding) = self.cache.get(text, model) {
            if let Some(ref metrics) = self.metrics {
                metrics.record_cache_hit();
            }
            return Ok(embedding);
        }
        
        if let Some(ref metrics) = self.metrics {
            metrics.record_cache_miss();
        }

        // Compute embedding using AI service
        let embedding_result = self.embedding_service.embed(text)
            .map_err(|e| anyhow::anyhow!("Embedding generation failed: {}", e))?;
        
        let embedding = embedding_result.embedding;

        // Cache for future use
        self.cache.insert(text, model, embedding.clone())?;

        Ok(embedding)
    }
}

pub struct SearchBuilder<'a> {
    service: &'a MemoryService,
    query: String,
    options: SearchOptions,
}

impl<'a> SearchBuilder<'a> {
    fn new(service: &'a MemoryService, query: String) -> Self {
        Self {
            service,
            query,
            options: SearchOptions::default(),
        }
    }

    pub fn with_layers(mut self, layers: &[Layer]) -> Self {
        self.options.layers = layers.to_vec();
        self
    }

    pub fn with_layer(mut self, layer: Layer) -> Self {
        self.options.layers = vec![layer];
        self
    }

    pub fn top_k(mut self, k: usize) -> Self {
        self.options.top_k = k;
        self
    }

    pub fn min_score(mut self, threshold: f32) -> Self {
        self.options.score_threshold = threshold;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.options.tags = tags;
        self
    }

    pub fn in_project(mut self, project: String) -> Self {
        self.options.project = Some(project);
        self
    }

    pub async fn execute(self) -> Result<Vec<Record>> {
        self.service.search_with_options(&self.query, self.options).await
    }
}