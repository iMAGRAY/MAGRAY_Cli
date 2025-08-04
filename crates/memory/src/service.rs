use anyhow::Result;
use std::path::{PathBuf, Path};
use std::sync::Arc;
use tracing::{debug, info, warn, error};

use crate::{
    cache::EmbeddingCache,
    cache_lru::{EmbeddingCacheLRU, CacheConfig as LruCacheConfig},
    cache_interface::EmbeddingCacheInterface,
    health::{HealthMonitor, HealthConfig, ComponentType, AlertSeverity, SystemHealthStatus},
    metrics::{MetricsCollector, LayerMetrics},
    notifications::NotificationManager,
    promotion::{PromotionEngine, PromotionStats},
    ml_promotion::{MLPromotionEngine, MLPromotionConfig, MLPromotionStats},
    streaming::{StreamingMemoryAPI, StreamingConfig},
    storage::VectorStore,
    types::{Layer, PromotionConfig, Record, SearchOptions},
    gpu_accelerated::{GpuBatchProcessor, BatchProcessorConfig},
    backup::{BackupManager, BackupMetadata},
    resource_manager::{ResourceManager, ResourceConfig, ResourceUsage},
    batch_manager::{BatchOperationManager, BatchConfig, BatchStats, BatchOperationBuilder},
};

use ai::{AiConfig, ModelLoader, RerankingService};
use common::OperationTimer;

// @component: {"k":"C","id":"memory_service","t":"Main memory service orchestrator","m":{"cur":70,"tgt":95,"u":"%"},"f":["memory","orchestration"]}

/// Создать конфигурацию по умолчанию для memory service
pub fn default_config() -> Result<MemoryConfig> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?
        .join("magray");
    
    Ok(MemoryConfig {
        db_path: cache_dir.join("memory.db"),
        cache_path: cache_dir.join("embeddings_cache"),
        promotion: PromotionConfig::default(),
        ml_promotion: Some(MLPromotionConfig::default()),
        streaming_config: Some(StreamingConfig::default()),
        ai_config: AiConfig::default(),
        health_config: HealthConfig::default(),
        notification_config: crate::notifications::NotificationConfig::default(),
        cache_config: CacheConfigType::Lru(LruCacheConfig::default()),
        batch_config: BatchConfig::default(),
        resource_config: ResourceConfig::default(),
        // Legacy поля для обратной совместимости
        #[allow(deprecated)]
        max_vectors: 1_000_000,
        #[allow(deprecated)]
        max_cache_size_bytes: 1024 * 1024 * 1024,
        #[allow(deprecated)]
        max_memory_usage_percent: Some(50),
    })
}
pub struct MemoryService {
    store: Arc<VectorStore>,
    cache: Arc<dyn EmbeddingCacheInterface>,
    promotion: Arc<PromotionEngine>,
    ml_promotion: Option<Arc<parking_lot::RwLock<MLPromotionEngine>>>,
    batch_processor: Arc<GpuBatchProcessor>,
    batch_manager: Arc<BatchOperationManager>,
    reranking_service: Option<Arc<RerankingService>>,
    metrics: Option<Arc<MetricsCollector>>,
    health_monitor: Arc<HealthMonitor>,
    notification_manager: Option<Arc<crate::notifications::NotificationManager>>,
    backup_manager: Arc<BackupManager>,
    resource_manager: Arc<parking_lot::RwLock<ResourceManager>>,
    config: MemoryConfig,
}


#[derive(Clone)]
pub struct MemoryConfig {
    pub db_path: PathBuf,
    pub cache_path: PathBuf,
    pub promotion: PromotionConfig,
    pub ml_promotion: Option<MLPromotionConfig>,
    pub streaming_config: Option<StreamingConfig>,
    pub ai_config: AiConfig,
    pub health_config: HealthConfig,
    pub notification_config: crate::notifications::NotificationConfig,
    pub cache_config: CacheConfigType,
    pub batch_config: BatchConfig,
    /// Конфигурация динамического управления ресурсами
    pub resource_config: ResourceConfig,
    /// Legacy поля для обратной совместимости
    #[deprecated(note = "Use resource_config instead")]
    pub max_vectors: usize,
    #[deprecated(note = "Use resource_config instead")]
    pub max_cache_size_bytes: usize,
    #[deprecated(note = "Use resource_config instead")]
    pub max_memory_usage_percent: Option<u8>,
}

#[derive(Debug, Clone)]
pub enum CacheConfigType {
    Simple,
    Lru(LruCacheConfig),
}

/// Результат batch insert операции
#[derive(Debug, Clone)]
pub struct BatchInsertResult {
    pub total_records: usize,
    pub successful_records: usize,
    pub failed_records: usize,
    pub duration: std::time::Duration,
    pub records_per_second: f64,
}

/// Результат batch search операции
#[derive(Debug, Clone)]
pub struct BatchSearchResult {
    pub total_queries: usize,
    pub successful_queries: usize,
    pub failed_queries: usize,
    pub results: Vec<Vec<Record>>,
    pub duration: std::time::Duration,
    pub queries_per_second: f64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        let base_dir = dirs::data_dir()
            .unwrap_or_else(|| {
                warn!("Could not determine system data directory, falling back to current directory");
                PathBuf::from(".")
            })
            .join("magray");

        Self {
            db_path: base_dir.join("hnswdb"),
            cache_path: base_dir.join("cache").join("embeddings"),
            promotion: PromotionConfig::default(),
            ml_promotion: Some(MLPromotionConfig::default()),
            streaming_config: Some(StreamingConfig::default()),
            ai_config: AiConfig::default(),
            health_config: HealthConfig::default(),
            notification_config: crate::notifications::NotificationConfig::default(),
            cache_config: CacheConfigType::Lru(LruCacheConfig::default()),
            batch_config: BatchConfig::default(),
            resource_config: ResourceConfig {
                base_max_vectors: 1_000_000,
                base_cache_size_bytes: 1024 * 1024 * 1024, // 1GB
                target_memory_usage_percent: 50,
                ..ResourceConfig::default()
            },
            // Legacy поля для обратной совместимости - используем значения по умолчанию
            #[allow(deprecated)]
            max_vectors: 1_000_000,
            #[allow(deprecated)]
            max_cache_size_bytes: 1024 * 1024 * 1024,
            #[allow(deprecated)]
            max_memory_usage_percent: Some(50),
        }
    }
}

impl MemoryService {
    /// Открывает sled БД для promotion engine через DatabaseManager
    fn open_promotion_database(db_path: impl AsRef<std::path::Path>) -> Result<Arc<sled::Db>> {
        let db_manager = crate::database_manager::DatabaseManager::global();
        let db = db_manager.get_system_database(db_path.as_ref())?;
        info!("✅ Promotion database opened through DatabaseManager");
        Ok(db)
    }

    pub async fn new(config: MemoryConfig) -> Result<Self> {
        info!("Initializing memory service with dynamic resource management");

        // Инициализируем ResourceManager для динамического управления лимитами
        let resource_manager = ResourceManager::new(config.resource_config.clone())?;
        let initial_limits = resource_manager.get_current_limits();
        
        info!("🎯 Dynamic resource limits: {} vectors, {:.1}MB cache", 
              initial_limits.max_vectors, initial_limits.cache_size_bytes as f64 / 1024.0 / 1024.0);

        // Initialize storage с динамическими лимитами
        let mut store = VectorStore::new(&config.db_path).await?;
        
        // Конфигурируем HNSW индексы с динамическими лимитами
        store.set_max_elements(initial_limits.max_vectors).await?;
        
        let mut store = Arc::new(store);
        
        // Initialize all layer tables
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            store.init_layer(layer).await?;
        }

        // Initialize cache based on config
        let cache: Arc<dyn EmbeddingCacheInterface> = match &config.cache_config {
            CacheConfigType::Simple => {
                info!("Using simple embedding cache");
                Arc::new(EmbeddingCache::new(&config.cache_path)?)
            }
            CacheConfigType::Lru(lru_config) => {
                info!("Using LRU embedding cache with eviction policy");
                Arc::new(EmbeddingCacheLRU::new(&config.cache_path, lru_config.clone())?)
            }
        };

        // Initialize AI services with GPU batch processor
        let _model_loader = ModelLoader::new(&config.ai_config.models_dir)?;
        
        // Initialize batch processor with GPU support
        let batch_config = BatchProcessorConfig {
            use_gpu_if_available: config.ai_config.embedding.use_gpu,
            max_batch_size: 128,
            batch_timeout_ms: 50,
            cache_embeddings: true,
        };
        
        let batch_processor = Arc::new(
            GpuBatchProcessor::new(
                batch_config,
                config.ai_config.embedding.clone(),
                cache.clone(),
            ).await?
        );
        
        info!("✅ Batch processor initialized (GPU: {})", batch_processor.has_gpu());

        // Initialize robust reranking service with graceful fallback
        let reranking_service = match RerankingService::new(&config.ai_config.reranking) {
            Ok(service) => {
                info!("✅ Real reranking service initialized successfully");
                Some(Arc::new(service))
            }
            Err(e) => {
                warn!("⚠️ Reranking service unavailable: {}. Using vector similarity only", e);
                // Вместо mock - используем None и полагаемся на vector search
                None
            }
        };

        // Initialize promotion engine with time-based indexing
        let promotion_db = Self::open_promotion_database(config.db_path.join("promotion_indices"))?;
        let promotion = Arc::new(PromotionEngine::new(
            store.clone(),
            config.promotion.clone(),
            promotion_db
        ).await?);

        // Initialize ML-based promotion engine if enabled
        let ml_promotion = if let Some(ml_config) = &config.ml_promotion {
            info!("🧠 Инициализация ML-based promotion engine");
            match MLPromotionEngine::new(store.clone(), ml_config.clone()).await {
                Ok(engine) => {
                    info!("✅ ML promotion engine инициализирован успешно");
                    Some(Arc::new(parking_lot::RwLock::new(engine)))
                }
                Err(e) => {
                    warn!("⚠️ Не удалось инициализировать ML promotion engine: {}. Используем стандартный promotion", e);
                    None
                }
            }
        } else {
            info!("🔧 ML promotion отключен, используем стандартный time-based promotion");
            None
        };

        // Initialize health monitoring
        let health_config = config.health_config.clone();
        let health_monitor = Arc::new(HealthMonitor::new(health_config));
        
        // Record initial component status
        health_monitor.record_operation(ComponentType::VectorStore, true, 0.0, None);
        health_monitor.record_operation(ComponentType::EmbeddingService, true, 0.0, None);
        health_monitor.record_operation(ComponentType::Cache, true, 0.0, None);
        health_monitor.record_operation(ComponentType::PromotionEngine, true, 0.0, None);
        
        if reranking_service.is_some() {
            health_monitor.record_operation(ComponentType::RerankingService, true, 0.0, None);
        }
        
        // Set health monitor in store для real-time monitoring
        if let Some(store_mut) = Arc::get_mut(&mut store) {
            store_mut.set_health_monitor(health_monitor.clone());
        }
        
        info!("✅ PromotionEngine successfully integrated into MemoryService");
        info!("✅ Health monitoring system initialized and running");
        
        // Инициализируем notification manager если включен
        let notification_manager = if config.notification_config.channels.is_empty() {
            info!("📧 Notifications disabled (no channels configured)");
            None
        } else {
            match crate::notifications::NotificationManager::new(config.notification_config.clone()) {
                Ok(nm) => {
                    info!("✅ Notification system initialized with {} channels", 
                          config.notification_config.channels.len());
                    // Связываем с health monitor
                    Some(Arc::new(nm))
                }
                Err(e) => {
                    warn!("Failed to initialize notifications: {}", e);
                    None
                }
            }
        };

        // Инициализируем backup manager
        let backup_dir = config.db_path.parent()
            .unwrap_or_else(|| {
                warn!("Could not determine parent directory for backup, using database path itself");
                &config.db_path
            })
            .join("backups");
        let backup_manager = Arc::new(BackupManager::new(backup_dir)?);
        
        // Initialize batch operation manager
        let batch_config = config.batch_config.clone();
        let mut batch_manager_instance = BatchOperationManager::new(
            store.clone(),
            batch_config,
            None, // Metrics will be set later
        );
        
        // Start batch manager если включен async flush
        if config.batch_config.async_flush {
            batch_manager_instance.start().await?;
            info!("✅ Batch operation manager started with async flushing");
        }
        
        let batch_manager = Arc::new(batch_manager_instance);

        Ok(Self {
            store,
            cache,
            promotion,
            ml_promotion,
            batch_processor,
            batch_manager,
            reranking_service,
            metrics: None,
            health_monitor,
            notification_manager,
            backup_manager,
            resource_manager: Arc::new(parking_lot::RwLock::new(resource_manager)),
            config,
        })
    }

    pub async fn insert(&self, mut record: Record) -> Result<()> {
        let mut timer = OperationTimer::new("memory_insert");
        timer.add_field("layer", format!("{:?}", record.layer));
        timer.add_field("text_length", record.text.len());
        
        let start_time = std::time::Instant::now();
        
        let result: Result<()> = async {
            // Generate embedding if not provided
            if record.embedding.is_empty() {
                let embed_timer = OperationTimer::new("compute_embedding");
                record.embedding = self.get_or_compute_embedding(&record.text).await?;
                embed_timer.finish();
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
            
            // Use retry for the database insertion
            let retry_manager = crate::retry::RetryManager::for_database();
            let record_clone = record.clone();
            retry_manager.retry("memory_insert", || {
                let record_ref = &record_clone;
                async move { self.store.insert(record_ref).await }
            }).await?;

            Ok(())
        }.await;
        
        // Record operation metrics
        let duration = start_time.elapsed().as_millis() as f64;
        match &result {
            Ok(_) => {
                self.health_monitor.record_operation(ComponentType::VectorStore, true, duration, None);
                // Record insert latency metric
                let metric = crate::health::HealthMetric {
                    component: ComponentType::VectorStore,
                    metric_name: "insert_latency".to_string(),
                    value: duration,
                    unit: "ms".to_string(),
                    timestamp: chrono::Utc::now(),
                    threshold_warning: Some(100.0),
                    threshold_critical: Some(500.0),
                };
                let _ = self.health_monitor.record_metric(metric);
            },
            Err(e) => {
                self.health_monitor.record_operation(ComponentType::VectorStore, false, duration, Some(e.to_string()));
            }
        }
        
        timer.finish_with_result(result.as_ref().map(|_| ()));
        result
    }

    pub async fn insert_batch(&self, records: Vec<Record>) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        // Collect texts that need embeddings
        let texts_to_embed: Vec<(usize, String)> = records.iter()
            .enumerate()
            .filter_map(|(i, r)| {
                if r.embedding.is_empty() {
                    Some((i, r.text.clone()))
                } else {
                    None
                }
            })
            .collect();
        
        // Generate embeddings in batch
        let embeddings = if !texts_to_embed.is_empty() {
            let texts: Vec<String> = texts_to_embed.iter()
                .map(|(_, text)| text.clone())
                .collect();
            self.batch_processor.embed_batch(texts).await?
        } else {
            Vec::new()
        };
        
        // Process records with embeddings
        let mut processed_records = records;
        for ((idx, _), embedding) in texts_to_embed.iter().zip(embeddings.iter()) {
            processed_records[*idx].embedding = embedding.clone();
        }
        
        // Set defaults for all records
        for record in &mut processed_records {
            if record.id == uuid::Uuid::nil() {
                record.id = uuid::Uuid::new_v4();
            }
            if record.ts == chrono::DateTime::<chrono::Utc>::default() {
                record.ts = chrono::Utc::now();
            }
            record.last_access = record.ts;
        }
        
        let processed = processed_records;
        
        info!("Inserting batch of {} records", processed.len());
        
        let retry_manager = crate::retry::RetryManager::for_database();
        let store_clone = self.store.clone();
        let processed_refs: Vec<&Record> = processed.iter().collect();
        
        retry_manager.retry("batch_insert", || {
            let store = store_clone.clone();
            let refs_copy = processed_refs.clone();
            async move {
                store.insert_batch(&refs_copy).await
            }
        }).await?;

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
        let mut timer = OperationTimer::new("memory_search");
        timer.add_field("query_length", query.len());
        timer.add_field("layers_count", options.layers.len());
        timer.add_field("top_k", options.top_k);
        
        let embed_timer = OperationTimer::new("search_embedding");
        let query_embedding = self.get_or_compute_embedding(query).await?;
        embed_timer.finish();
        
        let mut all_results = Vec::new();

        // Search each requested layer
        for layer in &options.layers {
            let retry_manager = crate::retry::RetryManager::for_hnsw();
            let layer_copy = *layer;
            let query_embedding_clone = query_embedding.clone();
            let store_clone = self.store.clone();
            let top_k = options.top_k;
            
            let mut results = retry_manager.retry("vector_search", || {
                let query_emb = query_embedding_clone.clone();
                let store = store_clone.clone();
                async move {
                    store.search(&query_emb, layer_copy, top_k).await
                }
            }).await?;

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

        // Sort by initial vector score with proper NaN handling
        all_results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or_else(|| {
                if a.score.is_nan() && b.score.is_nan() {
                    std::cmp::Ordering::Equal
                } else if a.score.is_nan() {
                    std::cmp::Ordering::Greater // NaN values go to end
                } else {
                    std::cmp::Ordering::Less
                }
            })
        });

        // Second stage: professional reranking if available, otherwise enhanced vector scoring
        let final_results = if let Some(ref reranker) = self.reranking_service {
            if all_results.len() > 1 {
                debug!("🔄 Applying neural reranking to {} candidates", all_results.len());
                
                // Get more candidates for reranking (3x for better recall)
                let rerank_candidates = all_results.iter().take((options.top_k * 3).min(200)).cloned().collect::<Vec<_>>();
                
                // Extract texts for reranking
                let documents: Vec<String> = rerank_candidates
                    .iter()
                    .map(|r| r.text.clone())
                    .collect();

                // Professional reranking with error handling
                match reranker.rerank(query, &documents) {
                    Ok(rerank_results) => {
                        info!("✅ Neural reranking successful: {} -> {} results", 
                              rerank_candidates.len(), rerank_results.len());
                        
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
                        warn!("⚠️ Reranking failed: {}, fallback to vector similarity", e);
                        self.enhanced_vector_ranking(query, all_results, options.top_k).await
                    }
                }
            } else {
                all_results.into_iter().take(options.top_k).collect()
            }
        } else {
            // Enhanced vector-only ranking when reranking unavailable  
            debug!("📊 Using enhanced vector similarity ranking");
            self.enhanced_vector_ranking(query, all_results, options.top_k).await
        };

        // Update access stats (in production, this would be batched)
        for result in &final_results {
            self.store.update_access(result.layer, &result.id.to_string()).await?;
        }

        timer.add_field("results_count", final_results.len());
        timer.finish();
        Ok(final_results)
    }

    pub async fn get_by_id(&self, id: &uuid::Uuid, layer: Layer) -> Result<Option<Record>> {
        self.store.get_by_id(id, layer).await
    }

    /// Run promotion cycle with time-based indexing
    pub async fn run_promotion_cycle(&self) -> Result<PromotionStats> {
        info!("🚀 Running promotion cycle with time-based indexing");
        let start = std::time::Instant::now();
        
        let stats = self.promotion.run_promotion_cycle().await?;
        
        if let Some(ref metrics) = self.metrics {
            metrics.record_promotion_cycle(start.elapsed());
            metrics.record_promotion("interact", "insights", stats.interact_to_insights as u64);
            metrics.record_promotion("insights", "assets", stats.insights_to_assets as u64);
            metrics.record_expired((stats.expired_interact + stats.expired_insights) as u64);
        }
        
        info!("✅ Promotion cycle completed in {}ms", stats.total_time_ms);
        Ok(stats)
    }

    /// Get performance statistics from promotion engine
    pub async fn get_promotion_performance_stats(&self) -> Result<crate::promotion::PromotionPerformanceStats> {
        self.promotion.get_performance_stats().await
    }

    /// Run ML-based promotion cycle if available, fallback to standard promotion
    #[allow(clippy::await_holding_lock)]
    pub async fn run_ml_promotion_cycle(&self) -> Result<MLPromotionStats> {
        if let Some(ref ml_promotion) = self.ml_promotion {
            info!("🧠 Запуск ML-based promotion цикла");
            let mut engine = ml_promotion.write();
            engine.run_ml_promotion_cycle().await
        } else {
            info!("🔧 ML promotion недоступен, используем стандартный promotion");
            // Fallback: конвертируем стандартные stats в ML stats
            let standard_stats = self.promotion.run_promotion_cycle().await?;
            Ok(MLPromotionStats {
                total_analyzed: standard_stats.interact_to_insights + standard_stats.insights_to_assets,
                promoted_interact_to_insights: standard_stats.interact_to_insights,
                promoted_insights_to_assets: standard_stats.insights_to_assets,
                ml_inference_time_ms: 0, // No ML inference
                feature_extraction_time_ms: 0,
                model_accuracy: 0.0,
                avg_confidence_score: 0.0,
                cache_hit_rate: 0.0,
                gpu_utilization: 0.0,
            })
        }
    }

    /// Check if ML promotion is available
    pub fn has_ml_promotion(&self) -> bool {
        self.ml_promotion.is_some()
    }

    /// Get ML promotion configuration if available
    pub fn get_ml_promotion_config(&self) -> Option<MLPromotionConfig> {
        self.config.ml_promotion.clone()
    }

    /// Enable/disable ML promotion
    pub async fn configure_ml_promotion(&mut self, config: Option<MLPromotionConfig>) -> Result<()> {
        self.config.ml_promotion = config.clone();
        
        if let Some(ml_config) = config {
            info!("🧠 Включение ML promotion с новой конфигурацией");
            match MLPromotionEngine::new(self.store.clone(), ml_config).await {
                Ok(engine) => {
                    self.ml_promotion = Some(Arc::new(parking_lot::RwLock::new(engine)));
                    info!("✅ ML promotion engine успешно переконфигурирован");
                }
                Err(e) => {
                    warn!("⚠️ Ошибка переконфигурации ML promotion: {}", e);
                    return Err(e);
                }
            }
        } else {
            info!("🔧 Отключение ML promotion");
            self.ml_promotion = None;
        }
        
        Ok(())
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
        
        // Pass metrics to storage через RwLock
        self.store.set_metrics(metrics.clone());
        
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
                
                for (_, value) in iter.flatten() {
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
        // Batch processor handles caching internally
        self.batch_processor.embed(text).await
    }
    
    /// Enhanced vector ranking - интеллектуальная замена mock reranking
    async fn enhanced_vector_ranking(&self, query: &str, mut results: Vec<Record>, top_k: usize) -> Vec<Record> {
        if results.len() <= 1 {
            return results.into_iter().take(top_k).collect();
        }
        
        debug!("🧠 Applying enhanced vector ranking to {} results", results.len());
        
        // Многофакторное ранжирование без neural reranker
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        
        for record in &mut results {
            let text_lower = record.text.to_lowercase();
            
            // 1. Базовый vector score (уже есть)
            let vector_score = record.score;
            
            // 2. Lexical overlap (BM25-style)
            let word_matches = query_words.iter()
                .filter(|word| text_lower.contains(*word))
                .count() as f32;
            let lexical_score = word_matches / query_words.len().max(1) as f32;
            
            // 3. Length normalization (средние тексты предпочтительнее)
            let text_len = record.text.len() as f32;
            let length_score = 1.0 / (1.0 + (text_len - 200.0).abs() / 100.0);
            
            // 4. Access pattern boost (популярные результаты)
            let access_boost = (record.access_count as f32).ln_1p() / 10.0;
            
            // 5. Recency factor (свежие результаты)
            let age_hours = (chrono::Utc::now() - record.ts).num_hours() as f32;
            let recency_score = 1.0 / (1.0 + age_hours / 24.0);
            
            // Комбинированный score с весами
            record.score = vector_score * 0.5 +        // Главный фактор
                          lexical_score * 0.2 +       // Точные совпадения слов
                          length_score * 0.1 +        // Оптимальная длина
                          access_boost * 0.1 +        // Популярность
                          recency_score * 0.1;        // Свежесть
        }
        
        // Сортируем по новому комбинированному score с обработкой NaN
        results.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or_else(|| {
                warn!("NaN values detected in enhanced ranking scores");
                if a.score.is_nan() && b.score.is_nan() {
                    std::cmp::Ordering::Equal
                } else if a.score.is_nan() {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Less
                }
            })
        });
        
        let final_results = results.into_iter().take(top_k).collect::<Vec<_>>();
        debug!("✅ Enhanced ranking completed: {} final results", final_results.len());
        
        final_results
    }

    /// Получает текущий health статус системы
    pub fn get_system_health(&self) -> SystemHealthStatus {
        self.health_monitor.get_system_health()
    }
    
    /// Получает health статус конкретного компонента
    pub fn get_component_health(&self, component: ComponentType) -> Option<crate::health::ComponentPerformanceStats> {
        self.health_monitor.get_component_performance(component)
    }
    
    /// Получает метрики компонента
    pub fn get_component_metrics(&self, component: ComponentType, metric_name: &str, limit: Option<usize>) -> Vec<crate::health::HealthMetric> {
        self.health_monitor.get_component_metrics(component, metric_name, limit)
    }
    
    /// Создает custom alert
    pub fn create_health_alert(&self, component: ComponentType, severity: AlertSeverity, title: String, description: String) {
        self.health_monitor.create_alert(component.clone(), severity.clone(), title.clone(), description.clone());
        
        // Отправляем через notification manager если включен
        if let Some(ref nm) = self.notification_manager {
            let alert = crate::health::HealthAlert {
                id: format!("{:?}_{}", component, chrono::Utc::now().timestamp()),
                component,
                severity,
                title,
                description,
                metric_value: None,
                threshold: None,
                timestamp: chrono::Utc::now(),
                resolved: false,
                resolved_at: None,
            };
            
            let nm = nm.clone();
            tokio::spawn(async move {
                if let Err(e) = nm.handle_alert(alert).await {
                    error!("Failed to send notification: {}", e);
                }
            });
        }
    }
    
    /// Проверяет здоровье всех компонентов системы памяти
    pub async fn run_health_check(&self) -> Result<SystemHealthStatus> {
        let start_time = std::time::Instant::now();
        
        // Тестируем VectorStore
        let vector_health = match self.store.search(&vec![0.1; 1024], Layer::Interact, 1).await {
            Ok(_) => {
                let duration = start_time.elapsed().as_millis() as f64;
                self.health_monitor.record_operation(ComponentType::VectorStore, true, duration, None);
                true
            },
            Err(e) => {
                let duration = start_time.elapsed().as_millis() as f64;
                self.health_monitor.record_operation(ComponentType::VectorStore, false, duration, Some(e.to_string()));
                false
            }
        };
        
        // Тестируем Cache
        let cache_health = match self.cache.get("test_key", "test_model") {
            Some(_) => {
                self.health_monitor.record_operation(ComponentType::Cache, true, 1.0, None);
                true
            },
            None => {
                self.health_monitor.record_operation(ComponentType::Cache, false, 1.0, Some("Cache miss".to_string()));
                false
            }
        };
        
        // Тестируем EmbeddingService через batch processor
        let embedding_health = match self.batch_processor.embed("test text").await {
            Ok(_) => {
                let duration = start_time.elapsed().as_millis() as f64;
                self.health_monitor.record_operation(ComponentType::EmbeddingService, true, duration, None);
                true
            },
            Err(e) => {
                let duration = start_time.elapsed().as_millis() as f64;
                self.health_monitor.record_operation(ComponentType::EmbeddingService, false, duration, Some(e.to_string()));
                false
            }
        };
        
        // Создаем alerts при проблемах
        if !vector_health {
            self.health_monitor.create_alert(
                ComponentType::VectorStore,
                AlertSeverity::Critical,
                "VectorStore Health Check Failed".to_string(),
                "VectorStore is not responding to basic operations".to_string()
            );
        }
        
        if !cache_health {
            self.health_monitor.create_alert(
                ComponentType::Cache,
                AlertSeverity::Warning,
                "Cache Health Check Failed".to_string(),
                "Embedding cache is not accessible".to_string()
            );
        }
        
        if !embedding_health {
            self.health_monitor.create_alert(
                ComponentType::EmbeddingService,
                AlertSeverity::Critical,
                "EmbeddingService Health Check Failed".to_string(),
                "Embedding service is not generating embeddings".to_string()
            );
        }
        
        info!("Health check completed - VectorStore: {}, Cache: {}, EmbeddingService: {}", 
              vector_health, cache_health, embedding_health);
        
        Ok(self.health_monitor.get_system_health())
    }
    
    /// Получить VectorStore для прямого доступа (используется в API)
    pub fn get_store(&self) -> Arc<VectorStore> {
        self.store.clone()
    }
    
    /// Получить среднюю статистику доступа для слоя
    pub async fn get_layer_average_access(&self, layer: Layer) -> Result<f32> {
        let mut total_access = 0u32;
        let mut count = 0usize;
        
        // Итерируемся по всем записям слоя
        let iter = self.store.iter_layer(layer).await?;
        for (_, value) in iter.flatten() {
            if let Ok(stored) = bincode::deserialize::<crate::storage::StoredRecord>(&value) {
                total_access += stored.record.access_count;
                count += 1;
            }
        }
        
        Ok(if count > 0 { total_access as f32 / count as f32 } else { 0.0 })
    }

    /// Создать backup системы памяти
    pub async fn create_backup(&self, name: Option<String>) -> Result<PathBuf> {
        info!("Creating memory backup...");
        let path = self.backup_manager.create_backup(self.store.clone(), name).await?;
        
        // Записываем в метрики
        if let Some(ref metrics) = self.metrics {
            metrics.record_vector_insert(std::time::Duration::from_millis(100));
        }
        
        Ok(path)
    }

    /// Восстановить из backup
    pub async fn restore_backup(&self, backup_path: impl AsRef<Path>) -> Result<BackupMetadata> {
        info!("Restoring from backup: {:?}", backup_path.as_ref());
        
        let metadata = self.backup_manager.restore_backup(self.store.clone(), backup_path).await?;
        
        // Перестраиваем индексы после восстановления
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            self.store.init_layer(layer).await?;
        }
        
        // Записываем в метрики
        if let Some(ref metrics) = self.metrics {
            metrics.record_vector_insert(std::time::Duration::from_millis(100));
        }
        
        info!("Backup restored successfully: {} records", metadata.total_records);
        Ok(metadata)
    }

    /// Получить список доступных backup файлов
    pub fn list_backups(&self) -> Result<Vec<crate::backup::BackupInfo>> {
        self.backup_manager.list_backups()
    }

    /// Очистить старые backup файлы
    pub fn cleanup_old_backups(&self, keep_count: usize) -> Result<usize> {
        self.backup_manager.cleanup_old_backups(keep_count)
    }

    /// Создать автоматический backup (для периодических задач)
    pub async fn auto_backup(&self) -> Result<()> {
        // Проверяем когда был последний backup
        let backups = self.list_backups()?;
        
        let should_backup = if let Some(latest) = backups.first() {
            // Backup если прошло больше 24 часов
            let age = chrono::Utc::now() - latest.metadata.created_at;
            age > chrono::Duration::hours(24)
        } else {
            // Первый backup
            true
        };
        
        if should_backup {
            let name = format!("auto_{}", chrono::Utc::now().format("%Y%m%d"));
            self.create_backup(Some(name)).await?;
            
            // Оставляем только последние 7 backup файлов
            self.cleanup_old_backups(7)?;
        }
        
        Ok(())
    }

    /// Обновить лимиты ресурсов на основе текущего использования
    pub async fn update_resource_limits(&self) -> Result<bool> {
        // Собираем статистику текущего использования
        let memory_stats = self.store.memory_stats();
        let (_cache_hits, _cache_misses, cache_total) = self.cache_stats();
        
        let current_limits = self.resource_manager.read().get_current_limits();
        
        // Примерная оценка размера кэша (в реальности нужно получать точные данные)
        let estimated_cache_size = cache_total * 1024 * 4; // Примерно 4 байта на float
        
        let resource_usage = ResourceUsage::new(
            memory_stats.total_vectors,
            current_limits.max_vectors,
            estimated_cache_size as usize,
            current_limits.cache_size_bytes,
        );
        
        // Проверяем необходимость масштабирования
        let scaling_occurred = self.resource_manager.write().update_limits_if_needed(&resource_usage)?;
        
        if scaling_occurred {
            let new_limits = self.resource_manager.read().get_current_limits();
            
            // Применяем новые лимиты к VectorStore
            if new_limits.max_vectors != current_limits.max_vectors {
                let mut store = Arc::clone(&self.store);
                if let Some(store_mut) = Arc::get_mut(&mut store) {
                    store_mut.set_max_elements(new_limits.max_vectors).await?;
                }
            }
            
            info!("🔄 Resource limits updated: {} vectors ({:+}), {:.1}MB cache ({:+.1}MB)",
                  new_limits.max_vectors, 
                  new_limits.max_vectors as i64 - current_limits.max_vectors as i64,
                  new_limits.cache_size_bytes as f64 / 1024.0 / 1024.0,
                  (new_limits.cache_size_bytes as i64 - current_limits.cache_size_bytes as i64) as f64 / 1024.0 / 1024.0);
        }
        
        Ok(scaling_occurred)
    }
    
    /// Получить текущие лимиты ресурсов
    pub fn get_current_resource_limits(&self) -> crate::resource_manager::CurrentLimits {
        self.resource_manager.read().get_current_limits()
    }
    
    /// Получить размер кэша в байтах
    pub async fn get_cache_size(&self) -> Result<usize> {
        // Получаем размер через метод size()
        let size = self.cache.size()?;
        Ok(size as usize)
    }
    
    /// Очистить кэш эмбеддингов
    pub async fn clear_cache(&self) -> Result<()> {
        self.cache.clear()
    }
    
    /// Получить статистику использования ресурсов
    pub fn get_resource_usage_stats(&self) -> ResourceUsage {
        let memory_stats = self.store.memory_stats();
        let (_cache_hits, _cache_misses, cache_total) = self.cache_stats();
        let current_limits = self.resource_manager.read().get_current_limits();
        
        let estimated_cache_size = cache_total * 1024 * 4;
        
        ResourceUsage::new(
            memory_stats.total_vectors,
            current_limits.max_vectors,
            estimated_cache_size as usize,
            current_limits.cache_size_bytes,
        )
    }
    
    /// Получить статистику масштабирования
    pub fn get_scaling_stats(&self) -> crate::resource_manager::ScalingStats {
        self.resource_manager.read().get_scaling_stats()
    }
    
    /// Ручная настройка лимитов ресурсов
    pub async fn set_resource_limits_manual(&self, max_vectors: usize, cache_size_bytes: usize) -> Result<()> {
        self.resource_manager.write().set_limits_manual(max_vectors, cache_size_bytes)?;
        
        // Применяем к VectorStore
        let mut store = Arc::clone(&self.store);
        if let Some(store_mut) = Arc::get_mut(&mut store) {
            store_mut.set_max_elements(max_vectors).await?;
        }
        
        info!("🎯 Manual resource limits set: {} vectors, {:.1}MB cache", 
              max_vectors, cache_size_bytes as f64 / 1024.0 / 1024.0);
        
        Ok(())
    }
    
    /// Получить конфигурацию памяти
    pub fn config(&self) -> &MemoryConfig {
        &self.config
    }
    
    /// Получить менеджер уведомлений для тестирования и прямого доступа
    pub fn notification_manager(&self) -> Option<&NotificationManager> {
        self.notification_manager.as_deref()
    }
    
    // ========== BATCH OPERATIONS API ==========
    
    /// Создать batch builder для оптимизированных batch операций
    pub fn batch(&self) -> BatchBuilder<'_> {
        BatchBuilder::new(self)
    }
    
    /// Вставить несколько записей одновременно
    pub async fn batch_insert(&self, mut records: Vec<Record>) -> Result<BatchInsertResult> {
        let total_records = records.len();
        let start_time = std::time::Instant::now();
        
        info!("Starting batch insert of {} records", total_records);
        
        // Сначала генерируем embeddings для записей, у которых они пустые
        let texts_to_embed: Vec<(usize, String)> = records.iter()
            .enumerate()
            .filter_map(|(i, r)| {
                if r.embedding.is_empty() {
                    Some((i, r.text.clone()))
                } else {
                    None
                }
            })
            .collect();
        
        if !texts_to_embed.is_empty() {
            info!("Generating embeddings for {} records", texts_to_embed.len());
            let texts: Vec<String> = texts_to_embed.iter()
                .map(|(_, text)| text.clone())
                .collect();
            let embeddings = self.batch_processor.embed_batch(texts).await?;
            
            // Обновляем embeddings в записях
            for ((idx, _), embedding) in texts_to_embed.iter().zip(embeddings.iter()) {
                records[*idx].embedding = embedding.clone();
            }
        }
        
        // Устанавливаем значения по умолчанию
        for record in &mut records {
            if record.id == uuid::Uuid::nil() {
                record.id = uuid::Uuid::new_v4();
            }
            if record.ts == chrono::DateTime::<chrono::Utc>::default() {
                record.ts = chrono::Utc::now();
            }
            record.last_access = record.ts;
        }
        
        // Добавляем записи в batch manager
        self.batch_manager.add_batch(records).await?;
        
        // Flush все батчи для немедленной вставки
        self.batch_manager.flush_all().await?;
        
        let duration = start_time.elapsed();
        let stats = self.batch_manager.stats();
        
        // Записываем метрики
        if let Some(metrics) = &self.metrics {
            metrics.record_batch_operation(
                "batch_insert",
                total_records,
                duration,
            );
        }
        
        // Создаем health метрику
        let _ = self.health_monitor.record_metric(crate::health::HealthMetric {
            component: ComponentType::VectorStore,
            metric_name: "batch_insert_rate".to_string(),
            value: (total_records as f64 / duration.as_secs_f64()),
            unit: "records/sec".to_string(),
            timestamp: chrono::Utc::now(),
            threshold_warning: Some(100.0),
            threshold_critical: Some(10.0),
        });
        
        Ok(BatchInsertResult {
            total_records,
            successful_records: stats.total_records as usize,
            failed_records: (stats.failed_batches * stats.avg_batch_size as u64) as usize,
            duration,
            records_per_second: total_records as f64 / duration.as_secs_f64(),
        })
    }
    
    /// Выполнить batch поиск для нескольких запросов
    pub async fn batch_search(&self, queries: Vec<String>, options: SearchOptions) -> Result<BatchSearchResult> {
        let total_queries = queries.len();
        let start_time = std::time::Instant::now();
        
        info!("Starting batch search for {} queries", total_queries);
        
        let mut all_results = Vec::new();
        let mut failed_queries = 0;
        
        // Параллельно выполняем поиски используя futures
        use futures::future::join_all;
        
        let search_futures: Vec<_> = queries.into_iter()
            .map(|query| {
                let opts = options.clone();
                async move {
                    self.search(&query)
                        .with_layers(&opts.layers)
                        .top_k(opts.top_k)
                        .min_score(opts.score_threshold)
                        .execute()
                        .await
                }
            })
            .collect();
        
        let results = join_all(search_futures).await;
        
        // Собираем результаты
        for result in results {
            match result {
                Ok(records) => all_results.push(records),
                Err(_) => failed_queries += 1,
            }
        }
        
        let duration = start_time.elapsed();
        let successful_queries = total_queries - failed_queries;
        
        // Записываем метрики
        if let Some(metrics) = &self.metrics {
            metrics.record_batch_operation(
                "batch_search",
                total_queries,
                duration,
            );
        }
        
        Ok(BatchSearchResult {
            total_queries,
            successful_queries,
            failed_queries,
            results: all_results,
            duration,
            queries_per_second: total_queries as f64 / duration.as_secs_f64(),
        })
    }
    
    /// Получить статистику batch операций
    pub fn batch_stats(&self) -> BatchStats {
        self.batch_manager.stats()
    }
    
    /// Вручную запустить flush всех pending batch операций
    pub async fn flush_batches(&self) -> Result<()> {
        self.batch_manager.flush_all().await
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
    
    pub fn with_project(mut self, project: &str) -> Self {
        self.options.project = Some(project.to_string());
        self
    }

    pub async fn execute(self) -> Result<Vec<Record>> {
        self.service.search_with_options(&self.query, self.options).await
    }
}

impl MemoryService {
    /// Создать streaming API для real-time обработки
    pub async fn create_streaming_api(self: Arc<Self>) -> Result<StreamingMemoryAPI> {
        let config = self.config.streaming_config.clone()
            .unwrap_or_default();
            
        info!("🌊 Creating streaming API with config: max_sessions={}, buffer_size={}", 
              config.max_concurrent_sessions, config.buffer_size);
        
        StreamingMemoryAPI::new(
            self,
            config
        ).await
    }
    
    /// Проверить поддержку streaming API
    pub fn has_streaming_support(&self) -> bool {
        self.config.streaming_config.is_some()
    }
    
    /// Получить конфигурацию streaming API
    pub fn get_streaming_config(&self) -> Option<&StreamingConfig> {
        self.config.streaming_config.as_ref()
    }
}

/// Builder для создания batch операций
pub struct BatchBuilder<'a> {
    service: &'a MemoryService,
    records: Vec<Record>,
}

impl<'a> BatchBuilder<'a> {
    fn new(service: &'a MemoryService) -> Self {
        Self {
            service,
            records: Vec::new(),
        }
    }
    
    /// Добавить одну запись в batch
    pub fn add_record(mut self, record: Record) -> Self {
        self.records.push(record);
        self
    }
    
    /// Добавить несколько записей в batch
    pub fn add_records(mut self, mut records: Vec<Record>) -> Self {
        self.records.append(&mut records);
        self
    }
    
    /// Создать и добавить запись из текста
    pub fn add_text(mut self, text: String, layer: Layer) -> Self {
        let record = Record {
            id: uuid::Uuid::new_v4(),
            text,
            embedding: vec![], // Будет вычислен при вставке
            layer,
            kind: "text".to_string(),
            tags: vec![],
            project: "default".to_string(),
            session: "batch".to_string(),
            score: 0.0,
            ts: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
            access_count: 0,
        };
        self.records.push(record);
        self
    }
    
    /// Создать и добавить несколько записей из текстов
    pub fn add_texts(mut self, texts: Vec<String>, layer: Layer) -> Self {
        for text in texts {
            self = self.add_text(text, layer);
        }
        self
    }
    
    /// Выполнить batch insert
    pub async fn insert(self) -> Result<BatchInsertResult> {
        self.service.batch_insert(self.records).await
    }
    
    /// Создать оптимизированный batch с группировкой по слоям
    pub fn optimize(self) -> BatchOperationBuilder {
        BatchOperationBuilder::new()
            .add_records(self.records)
            .optimize_for_locality()
    }
}