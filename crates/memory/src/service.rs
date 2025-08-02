use anyhow::Result;
use std::path::{PathBuf, Path};
use std::sync::Arc;
use tracing::{debug, info, warn};
use dirs;

use crate::{
    cache::EmbeddingCache,
    cache_lru::{EmbeddingCacheLRU, CacheConfig as LruCacheConfig},
    cache_interface::EmbeddingCacheInterface,
    health::{HealthMonitor, HealthConfig, ComponentType, AlertSeverity, SystemHealthStatus},
    metrics::{MetricsCollector, LayerMetrics},
    promotion::{PromotionEngine, PromotionStats},
    storage::VectorStore,
    types::{Layer, PromotionConfig, Record, SearchOptions},
    gpu_accelerated::{GpuBatchProcessor, BatchProcessorConfig},
    backup::{BackupManager, BackupMetadata},
    resource_manager::{ResourceManager, ResourceConfig, ResourceUsage},
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
        ai_config: AiConfig::default(),
        health_config: HealthConfig::default(),
        cache_config: CacheConfigType::Lru(LruCacheConfig::default()),
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
    batch_processor: Arc<GpuBatchProcessor>,
    reranking_service: Option<Arc<RerankingService>>,
    metrics: Option<Arc<MetricsCollector>>,
    health_monitor: Arc<HealthMonitor>,
    backup_manager: Arc<BackupManager>,
    resource_manager: Arc<parking_lot::RwLock<ResourceManager>>,
    config: MemoryConfig,
}


pub struct MemoryConfig {
    pub db_path: PathBuf,
    pub cache_path: PathBuf,
    pub promotion: PromotionConfig,
    pub ai_config: AiConfig,
    pub health_config: HealthConfig,
    pub cache_config: CacheConfigType,
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

impl Default for MemoryConfig {
    fn default() -> Self {
        let base_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("magray");

        Self {
            db_path: base_dir.join("hnswdb"),
            cache_path: base_dir.join("cache").join("embeddings"),
            promotion: PromotionConfig::default(),
            ai_config: AiConfig::default(),
            health_config: HealthConfig::default(),
            cache_config: CacheConfigType::Lru(LruCacheConfig::default()),
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
        
        let store = Arc::new(store);
        
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
        let promotion_db = sled::open(config.db_path.join("promotion_indices"))?;
        let promotion = Arc::new(PromotionEngine::new(
            store.clone(),
            config.promotion.clone(),
            Arc::new(promotion_db)
        ).await?);

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
        
        info!("✅ PromotionEngine successfully integrated into MemoryService");
        info!("✅ Health monitoring system initialized and running");

        // Инициализируем backup manager
        let backup_dir = config.db_path.parent()
            .unwrap_or(&config.db_path)
            .join("backups");
        let backup_manager = Arc::new(BackupManager::new(backup_dir)?);

        Ok(Self {
            store,
            cache,
            promotion,
            batch_processor,
            reranking_service,
            metrics: None,
            health_monitor,
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
            self.store.insert(&record).await?;

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
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

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
        
        // Сортируем по новому комбинированному score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
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
        self.health_monitor.create_alert(component, severity, title, description)
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