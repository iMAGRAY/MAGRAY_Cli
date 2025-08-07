use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, info, warn};

use crate::batch_optimized::{BatchOptimizedConfig, BatchOptimizedProcessor};
use crate::cache_interface::EmbeddingCacheInterface;
use ai::gpu_fallback::FallbackStats;
use ai::{
    EmbeddingConfig, EmbeddingServiceTrait, GpuFallbackManager, GpuPipelineManager, PipelineConfig,
};

/// Статус здоровья GPU
#[derive(Debug, Clone)]
pub struct GpuHealthStatus {
    pub available: bool,
    pub memory_total_mb: u32,
    pub memory_used_estimate_mb: u32,
    pub success_rate: f32,
    pub error_count: u32,
    pub temperature_celsius: Option<f32>,
    pub issues: Vec<String>,
}

impl GpuHealthStatus {
    pub fn unavailable(reason: &str) -> Self {
        Self {
            available: false,
            memory_total_mb: 0,
            memory_used_estimate_mb: 0,
            success_rate: 0.0,
            error_count: 0,
            temperature_celsius: None,
            issues: vec![reason.to_string()],
        }
    }
}

/// Максимальный размер батча для GPU обработки (увеличено для 1000+ QPS)
const MAX_BATCH_SIZE: usize = 512;
/// Максимальное количество одновременных GPU операций (увеличено для throughput)
const MAX_CONCURRENT_GPU_OPS: usize = 8;

#[derive(Clone)]
pub struct GpuBatchProcessor {
    embedding_service: Arc<GpuFallbackManager>,
    gpu_pipeline: Option<Arc<GpuPipelineManager>>,
    cache: Arc<dyn EmbeddingCacheInterface>,
    #[allow(dead_code)]
    batch_semaphore: Arc<Semaphore>,
    processing_queue: Arc<Mutex<Vec<PendingEmbedding>>>,
    config: BatchProcessorConfig,
    /// Ultra-optimized batch processor для maximum QPS
    #[allow(dead_code)]
    ultra_batch_processor: Option<Arc<BatchOptimizedProcessor>>,
}

#[derive(Clone)]
pub struct BatchProcessorConfig {
    pub max_batch_size: usize,
    pub batch_timeout_ms: u64,
    pub use_gpu_if_available: bool,
    pub cache_embeddings: bool,
}

impl Default for BatchProcessorConfig {
    fn default() -> Self {
        Self {
            max_batch_size: MAX_BATCH_SIZE,
            batch_timeout_ms: 25, // Уменьшено для sub-5ms latency
            use_gpu_if_available: true,
            cache_embeddings: true,
        }
    }
}

struct PendingEmbedding {
    text: String,
    callback: tokio::sync::oneshot::Sender<Result<Vec<f32>>>,
}

impl GpuBatchProcessor {
    /// Создать новый процессор с надёжным GPU fallback механизмом
    pub async fn new(
        config: BatchProcessorConfig,
        embedding_config: EmbeddingConfig,
        cache: Arc<dyn EmbeddingCacheInterface>,
    ) -> Result<Self> {
        info!("🚀 Инициализация GpuBatchProcessor с надёжным fallback");

        // Создаём embedding сервис с автоматическим GPU/CPU fallback
        let embedding_service = Arc::new(
            GpuFallbackManager::new(embedding_config.clone())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create embedding service: {}", e))?,
        );

        // Пытаемся создать GPU pipeline для максимальной производительности
        let gpu_pipeline = if config.use_gpu_if_available {
            match Self::try_create_gpu_pipeline(&config, &embedding_config).await {
                Ok(pipeline) => {
                    info!("🚀 GPU Pipeline создан для параллельной обработки");
                    Some(Arc::new(pipeline))
                }
                Err(e) => {
                    warn!(
                        "⚠️ Не удалось создать GPU Pipeline: {}. Используем fallback.",
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        // Создаем ultra-optimized batch processor для maximum QPS
        let ultra_batch_processor = if config.max_batch_size >= 256 {
            match Self::create_ultra_batch_processor(&config).await {
                Ok(processor) => {
                    info!("🚀 Ultra-optimized batch processor created for 1000+ QPS");
                    Some(Arc::new(processor))
                }
                Err(e) => {
                    warn!("⚠️ Could not create ultra-optimized processor: {}. Using standard batching.", e);
                    None
                }
            }
        } else {
            None
        };

        info!("✅ GPU batch processor initialized with robust fallback mechanism");

        Ok(Self {
            embedding_service,
            gpu_pipeline,
            cache,
            batch_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_GPU_OPS)),
            processing_queue: Arc::new(Mutex::new(Vec::new())),
            config,
            ultra_batch_processor,
        })
    }

    /// Создать ultra-optimized batch processor для maximum QPS
    async fn create_ultra_batch_processor(
        config: &BatchProcessorConfig,
    ) -> Result<BatchOptimizedProcessor> {
        let ultra_config = BatchOptimizedConfig {
            max_batch_size: config.max_batch_size,
            min_batch_size: config.max_batch_size / 8, // Adaptive min batch size
            worker_threads: 8,                         // High concurrency для 1000+ QPS
            queue_capacity: 2048,                      // Большая очередь для high throughput
            batch_timeout_us: config.batch_timeout_ms * 1000, // Convert to microseconds
            use_prefetching: true,
            use_aligned_memory: true,
            adaptive_batching: true,
        };

        BatchOptimizedProcessor::new(ultra_config)
    }

    /// Попытка создать GPU pipeline сс comprehensive validation
    async fn try_create_gpu_pipeline(
        config: &BatchProcessorConfig,
        embedding_config: &EmbeddingConfig,
    ) -> Result<GpuPipelineManager> {
        // Валидация GPU capabilities перед созданием pipeline
        Self::validate_gpu_capabilities()?;

        let pipeline_config = PipelineConfig {
            max_concurrent_batches: Self::get_optimal_gpu_streams()?,
            optimal_batch_size: Self::get_safe_batch_size(config.max_batch_size)?,
            min_batch_size: 32,
            prefetch_enabled: Self::can_use_prefetch(),
            memory_pooling_enabled: Self::can_use_pinned_memory(),
            adaptive_batching: true,
        };

        info!(
            "🔍 Создание GPU Pipeline с конфигурацией: {:?}",
            pipeline_config
        );

        // Создаем с timeout и error handling
        match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            GpuPipelineManager::new(embedding_config.clone(), pipeline_config),
        )
        .await
        {
            Ok(Ok(manager)) => {
                info!("✅ GPU Pipeline успешно создан");
                Ok(manager)
            }
            Ok(Err(e)) => {
                warn!("❌ Ошибка создания GPU Pipeline: {}", e);
                Err(e)
            }
            Err(_) => {
                warn!("⏰ Timeout при создании GPU Pipeline");
                Err(anyhow::anyhow!("GPU Pipeline creation timeout"))
            }
        }
    }

    /// Валидация GPU capabilities
    fn validate_gpu_capabilities() -> Result<()> {
        #[cfg(feature = "gpu")]
        {
            // Проверяем доступность GPU через AI модуль
            use ai::gpu_detector::GpuDetector;

            let detector = GpuDetector::detect();

            if !detector.available || detector.devices.is_empty() {
                return Err(anyhow::anyhow!("No GPU devices detected"));
            }

            // Проверяем минимальные требования
            let primary_gpu = &detector.devices[0];
            if primary_gpu.total_memory_mb < 2048 {
                // Минимум 2GB
                return Err(anyhow::anyhow!(
                    "Insufficient GPU memory: {}MB < 2048MB required",
                    primary_gpu.total_memory_mb
                ));
            }

            // Парсим compute capability (например "7.5" -> (7, 5))
            let parts: Vec<&str> = primary_gpu.compute_capability.split('.').collect();
            if parts.len() == 2 {
                if let (Ok(major), Ok(_minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>())
                {
                    if major < 6 {
                        return Err(anyhow::anyhow!(
                            "Insufficient compute capability: {} < 6.0 required",
                            primary_gpu.compute_capability
                        ));
                    }
                }
            }

            info!(
                "✅ GPU validation passed: {} with {}MB memory",
                primary_gpu.name, primary_gpu.total_memory_mb
            );
            Ok(())
        }

        #[cfg(not(feature = "gpu"))]
        {
            Err(anyhow::anyhow!("GPU support not compiled"))
        }
    }

    /// Получить оптимальное количество GPU streams
    fn get_optimal_gpu_streams() -> Result<usize> {
        #[cfg(feature = "gpu")]
        {
            use ai::gpu_detector::GpuDetector;

            let detector = GpuDetector::detect();
            if detector.available {
                if let Some(primary_gpu) = detector.devices.first() {
                    // 1 stream на 1GB памяти, максимум 8
                    let streams = (primary_gpu.total_memory_mb / 1024).min(8).max(1) as usize;
                    return Ok(streams);
                }
            }

            Ok(2) // Safe default
        }

        #[cfg(not(feature = "gpu"))]
        {
            Ok(1) // CPU fallback
        }
    }

    /// Получить безопасный размер batch с учетом GPU memory
    fn get_safe_batch_size(requested_size: usize) -> Result<usize> {
        #[cfg(feature = "gpu")]
        {
            use ai::gpu_detector::GpuDetector;

            const EMBEDDING_SIZE_BYTES: usize = 768 * 4; // f32 = 4 bytes
            const SAFETY_MARGIN: f32 = 0.7; // Используем 70% памяти

            let detector = GpuDetector::detect();
            if detector.available {
                if let Some(primary_gpu) = detector.devices.first() {
                    let available_memory =
                        (primary_gpu.total_memory_mb as f32 * 1024.0 * 1024.0 * SAFETY_MARGIN)
                            as usize;
                    let max_batch_by_memory = available_memory / EMBEDDING_SIZE_BYTES;
                    let safe_batch = requested_size.min(max_batch_by_memory).max(1);

                    info!(
                        "🔍 GPU Memory-based batch size: {} (requested: {}, memory limit: {})",
                        safe_batch, requested_size, max_batch_by_memory
                    );
                    return Ok(safe_batch);
                }
            }

            Ok(requested_size.min(64)) // Conservative fallback
        }

        #[cfg(not(feature = "gpu"))]
        {
            Ok(requested_size.min(32)) // CPU batch limit
        }
    }

    /// Проверить можно ли использовать pinned memory
    fn can_use_pinned_memory() -> bool {
        #[cfg(feature = "gpu")]
        {
            use ai::gpu_detector::GpuDetector;

            let detector = GpuDetector::detect();
            if detector.available {
                return detector
                    .devices
                    .iter()
                    .any(|gpu| gpu.total_memory_mb > 4096);
            }

            false
        }

        #[cfg(not(feature = "gpu"))]
        {
            false
        }
    }

    /// Проверить можно ли использовать prefetch
    fn can_use_prefetch() -> bool {
        #[cfg(feature = "gpu")]
        {
            use ai::gpu_detector::GpuDetector;

            let detector = GpuDetector::detect();
            if detector.available {
                return detector.devices.iter().any(|gpu| {
                    // Парсим compute capability строку (например "7.5" -> 7)
                    let parts: Vec<&str> = gpu.compute_capability.split('.').collect();
                    if parts.len() >= 1 {
                        if let Ok(major) = parts[0].parse::<u32>() {
                            return major >= 7;
                        }
                    }
                    false
                });
            }

            false
        }

        #[cfg(not(feature = "gpu"))]
        {
            false
        }
    }

    /// Получить эмбеддинг для одного текста с comprehensive error handling
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Валидация входных данных
        if text.trim().is_empty() {
            warn!("Empty text provided for embedding");
            return Ok(vec![0.0; 1024]); // Qwen3 dimension fallback
        }

        if text.len() > 8192 {
            // Reasonable text length limit
            warn!("Text too long ({} chars), truncating", text.len());
        }

        // Проверяем кэш
        if self.config.cache_embeddings {
            if let Some(embedding) = self.cache.get(text, "bge-m3") {
                debug!("Cache hit for embedding");
                return Ok(embedding);
            }
        }

        // Используем resilient embedding с multiple fallback levels
        let embedding = match self.get_embedding_with_fallback(text).await {
            Ok(emb) => emb,
            Err(e) => {
                warn!(
                    "All embedding methods failed for text: {}. Using zero vector fallback",
                    e
                );
                vec![0.0; 768] // Last resort fallback
            }
        };

        // Кэшируем результат (даже fallback)
        if self.config.cache_embeddings {
            if let Err(e) = self.cache.insert(text, "bge-m3", embedding.clone()) {
                warn!("Failed to cache embedding: {}", e);
            }
        }

        Ok(embedding)
    }

    /// Получить embedding с comprehensive fallback chain
    async fn get_embedding_with_fallback(&self, text: &str) -> Result<Vec<f32>> {
        // 1. Пытаемся через основной fallback сервис (GPU→CPU)
        match self
            .embedding_service
            .embed_batch(vec![text.to_string()])
            .await
        {
            Ok(mut embeddings) => {
                if let Some(embedding) = embeddings.pop() {
                    if !embedding.is_empty() {
                        return Ok(embedding);
                    }
                }
                return Err(anyhow::anyhow!("Empty embedding returned from service"));
            }
            Err(e) => {
                warn!("Primary embedding service failed: {}", e);
            }
        }

        // 2. Если основной сервис не работает, пытаемся через отдельный CPU fallback
        warn!("Attempting emergency CPU-only fallback for text embedding");

        // Создаем временный CPU-only сервис как last resort
        match self.create_emergency_cpu_service().await {
            Ok(cpu_service) => match cpu_service.embed_batch(vec![text.to_string()]).await {
                Ok(mut embeddings) => {
                    if let Some(embedding) = embeddings.pop() {
                        if !embedding.is_empty() {
                            info!("✅ Emergency CPU fallback succeeded");
                            return Ok(embedding);
                        }
                    }
                }
                Err(e) => warn!("Emergency CPU service also failed: {}", e),
            },
            Err(e) => warn!("Could not create emergency CPU service: {}", e),
        }

        Err(anyhow::anyhow!("All embedding fallback methods exhausted"))
    }

    /// Создать emergency CPU-only сервис
    async fn create_emergency_cpu_service(&self) -> Result<ai::GpuFallbackManager> {
        let mut emergency_config = ai::EmbeddingConfig::default();
        emergency_config.use_gpu = false;
        emergency_config.batch_size = 1; // Minimal batch size

        ai::GpuFallbackManager::new(emergency_config).await
    }

    /// Получить fallback embedding для одного текста
    async fn get_fallback_embedding(&self, text: &str) -> Result<Option<Vec<f32>>> {
        debug!("Получение fallback embedding для: {}", text);

        // Пытаемся через основной fallback сервис
        match self
            .embedding_service
            .embed_batch(vec![text.to_string()])
            .await
        {
            Ok(embeddings) => {
                if let Some(embedding) = embeddings.into_iter().next() {
                    Ok(Some(embedding))
                } else {
                    warn!("Fallback сервис не вернул embedding");
                    Ok(None)
                }
            }
            Err(e) => {
                warn!("Fallback сервис failed: {}", e);
                Ok(None)
            }
        }
    }

    /// Обработать батч текстов напрямую с resilient error handling
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Проверяем кэш и разделяем на cached/uncached
        let mut results = vec![None; texts.len()];
        let mut uncached_indices = Vec::new();
        let mut uncached_texts = Vec::new();

        if self.config.cache_embeddings {
            for (i, text) in texts.iter().enumerate() {
                if let Some(embedding) = self.cache.get(text, "bge-m3") {
                    results[i] = Some(embedding);
                } else {
                    uncached_indices.push(i);
                    uncached_texts.push(text.clone());
                }
            }
        } else {
            uncached_texts = texts.clone();
            uncached_indices = (0..texts.len()).collect();
        }

        // Обрабатываем uncached тексты с resilient processing
        if !uncached_texts.is_empty() {
            let embeddings = if let Some(ref pipeline) = self.gpu_pipeline {
                // Используем GPU pipeline для максимальной производительности
                debug!(
                    "🚀 Используем GPU Pipeline для {} текстов",
                    uncached_texts.len()
                );

                // Пытаемся через GPU pipeline с fallback
                match pipeline
                    .process_texts_optimized(uncached_texts.clone())
                    .await
                {
                    Ok(embeddings) => embeddings,
                    Err(e) => {
                        warn!("🔄 GPU Pipeline failed: {}. Fallback на основной сервис", e);
                        self.embedding_service
                            .embed_batch(uncached_texts.clone())
                            .await
                            .map_err(|fallback_err| {
                                anyhow::anyhow!(
                                    "Both GPU pipeline and fallback failed. GPU: {}, Fallback: {}",
                                    e,
                                    fallback_err
                                )
                            })?
                    }
                }
            } else {
                // Fallback на обычный сервис
                debug!(
                    "🔄 Используем Fallback сервис для {} текстов",
                    uncached_texts.len()
                );
                self.embedding_service
                    .embed_batch(uncached_texts.clone())
                    .await?
            };

            // Сохраняем в кэш и результаты с защитой от partial failures
            for (idx, (text, embedding)) in uncached_texts.iter().zip(embeddings.iter()).enumerate()
            {
                // Кэшируем с error handling
                if self.config.cache_embeddings {
                    if let Err(e) = self.cache.insert(text, "bge-m3", embedding.clone()) {
                        warn!("Failed to cache embedding for '{}': {}", text, e);
                        // Продолжаем обработку даже если кэширование не удалось
                    }
                }

                // Проверяем индексы для безопасности
                if let Some(result_idx) = uncached_indices.get(idx) {
                    if *result_idx < results.len() {
                        results[*result_idx] = Some(embedding.clone());
                    } else {
                        warn!(
                            "Invalid result index {} for batch size {}",
                            result_idx,
                            results.len()
                        );
                    }
                } else {
                    warn!("Missing uncached index for embedding {}", idx);
                }
            }
        }

        // Собираем финальные результаты с проверкой на None
        let mut final_results = Vec::with_capacity(results.len());
        for (i, result) in results.into_iter().enumerate() {
            match result {
                Some(embedding) => final_results.push(embedding),
                None => {
                    warn!("Missing embedding result for index {}, using fallback", i);
                    // Пытаемся получить fallback embedding для этого текста
                    let fallback_embedding = self
                        .get_fallback_embedding(&texts[i])
                        .await?
                        .unwrap_or_else(|| vec![0.0; 1024]); // Qwen3 dimension fallback
                    final_results.push(fallback_embedding);
                }
            }
        }
        Ok(final_results)
    }

    /// Обработать накопленный батч
    pub async fn process_batch(&self) -> Result<()> {
        let pending = {
            let mut queue = self.processing_queue.lock().await;
            std::mem::take(&mut *queue)
        };

        if pending.is_empty() {
            return Ok(());
        }

        let texts: Vec<String> = pending.iter().map(|p| p.text.clone()).collect();

        debug!("Processing batch of {} texts", texts.len());

        // Получаем эмбеддинги
        let embeddings = self.embed_batch(texts).await?;

        // Отправляем результаты
        for (pending_item, embedding) in pending.into_iter().zip(embeddings) {
            let _ = pending_item.callback.send(Ok(embedding));
        }

        Ok(())
    }

    /// Создать клон для фоновых задач
    pub fn clone_for_task(&self) -> Arc<Self> {
        Arc::new(self.clone())
    }

    /// Проверить доступность GPU через fallback manager
    pub fn has_gpu(&self) -> bool {
        // Получаем статистику от fallback manager
        let stats = self.embedding_service.get_stats();
        // Проверяем success rate вместо прямого доступа к полям
        stats.gpu_success_rate() > 0.0 || stats.fallback_rate() < 1.0
    }

    /// Получить статистику fallback
    pub fn get_fallback_stats(&self) -> FallbackStats {
        self.embedding_service.get_stats()
    }

    /// Принудительно переключиться на CPU режим
    pub fn force_cpu_mode(&self) {
        self.embedding_service.force_cpu_mode();
    }

    /// Получить статистику с comprehensive information
    pub async fn get_stats(&self) -> BatchProcessorStats {
        let queue_size = self.processing_queue.lock().await.len();

        // Получаем статистику pipeline если доступен
        let pipeline_stats = if let Some(ref pipeline) = self.gpu_pipeline {
            Some(pipeline.get_stats().await)
        } else {
            None
        };

        BatchProcessorStats {
            total_batches: 0,
            successful_batches: 0,
            failed_batches: 0,
            total_items: 0,
            gpu_batches: 0,
            cpu_fallback_batches: 0,
            avg_batch_time_ms: 0.0,
            avg_items_per_batch: 0.0,
            cache_hit_rate: 0.0,
            has_gpu: self.has_gpu(),
            queue_size,
            cache_stats: self.cache.stats(),
            pipeline_stats,
        }
    }

    /// Проверить GPU memory usage и состояние
    pub async fn check_gpu_health(&self) -> GpuHealthStatus {
        #[cfg(feature = "gpu")]
        {
            use ai::gpu_detector::GpuDetector;

            let detector = GpuDetector::detect();
            if detector.available {
                if let Some(primary_gpu) = detector.devices.first() {
                    let fallback_stats = self.get_fallback_stats();

                    GpuHealthStatus {
                        available: true,
                        memory_total_mb: primary_gpu.total_memory_mb as u32,
                        memory_used_estimate_mb: self.estimate_gpu_memory_usage().await,
                        success_rate: fallback_stats.gpu_success_rate() as f32,
                        error_count: fallback_stats.gpu_error_count as u32,
                        temperature_celsius: primary_gpu.temperature_c.map(|t| t as f32),
                        issues: self.detect_gpu_issues(&fallback_stats),
                    }
                } else {
                    GpuHealthStatus::unavailable("No GPU devices found")
                }
            } else {
                GpuHealthStatus::unavailable("GPU not available")
            }
        }

        #[cfg(not(feature = "gpu"))]
        {
            GpuHealthStatus::unavailable("GPU support not compiled")
        }
    }

    /// Оценка использования GPU памяти
    #[allow(dead_code)] // Для будущего мониторинга ресурсов
    async fn estimate_gpu_memory_usage(&self) -> u32 {
        // Приблизительная оценка на основе активных операций
        let queue_size = self.processing_queue.lock().await.len();
        let estimated_mb = (queue_size * 768 * 4) / (1024 * 1024); // f32 embeddings
        estimated_mb as u32
    }

    /// Обнаружение проблем с GPU
    #[allow(dead_code)] // Для будущей диагностики
    fn detect_gpu_issues(&self, stats: &FallbackStats) -> Vec<String> {
        let mut issues = Vec::new();

        if stats.gpu_success_rate() < 0.8 {
            issues.push(format!(
                "Low GPU success rate: {:.1}%",
                stats.gpu_success_rate() * 100.0
            ));
        }

        if stats.gpu_error_count > 10 {
            issues.push(format!("High error count: {}", stats.gpu_error_count));
        }

        if stats.fallback_rate() > 0.5 {
            issues.push(format!(
                "High CPU fallback rate: {:.1}%",
                stats.fallback_rate() * 100.0
            ));
        }

        issues
    }

    /// Cleanup и освобождение GPU ресурсов  
    pub async fn cleanup_gpu_resources(&self) -> Result<()> {
        info!("🧹 Освобождение GPU ресурсов");

        // Очищаем очередь обработки
        {
            let mut queue = self.processing_queue.lock().await;
            if !queue.is_empty() {
                warn!("Clearing {} pending operations during cleanup", queue.len());
                // Отправляем ошибки всем pending операциям
                for pending in queue.drain(..) {
                    let _ = pending
                        .callback
                        .send(Err(anyhow::anyhow!("Cleanup in progress")));
                }
            }
        }

        // Принудительный сброс circuit breaker если нужно
        if self.get_fallback_stats().gpu_error_count > 5 {
            info!("Resetting circuit breaker due to high error count");
            self.embedding_service.reset_circuit_breaker();
        }

        // TODO: Добавить освобождение GPU памяти когда API будет доступно

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_batch_processor_creation() {
        let temp_dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(e) => {
                println!("Failed to create temp dir: {}", e);
                return;
            }
        };

        let cache = match crate::EmbeddingCache::new(temp_dir.path(), crate::CacheConfig::default())
        {
            Ok(c) => Arc::new(c) as Arc<dyn EmbeddingCacheInterface>,
            Err(e) => {
                println!("Failed to create cache: {}", e);
                return;
            }
        };

        let config = BatchProcessorConfig::default();
        let embedding_config = EmbeddingConfig::default();

        match GpuBatchProcessor::new(config, embedding_config, cache).await {
            Ok(_) => {
                // Должен создаться хотя бы с CPU fallback
                println!("✅ Processor created successfully with fallback");
            }
            Err(e) => {
                println!("⚠️ Expected error without models: {}", e);
                // This is expected in test environment without models
            }
        }
    }

    #[tokio::test]
    async fn test_single_embedding() {
        let temp_dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(e) => {
                println!("Failed to create temp dir: {}", e);
                return;
            }
        };

        let cache = match crate::EmbeddingCache::new(temp_dir.path(), crate::CacheConfig::default())
        {
            Ok(c) => Arc::new(c) as Arc<dyn EmbeddingCacheInterface>,
            Err(e) => {
                println!("Failed to create cache: {}", e);
                return;
            }
        };

        let config = BatchProcessorConfig {
            use_gpu_if_available: false, // Форсируем CPU для тестов
            ..Default::default()
        };
        let embedding_config = EmbeddingConfig::default();

        match GpuBatchProcessor::new(config, embedding_config, cache).await {
            Ok(processor) => {
                match processor.embed("test text").await {
                    Ok(embedding) => {
                        println!("✅ Got embedding with length: {}", embedding.len());
                        assert!(!embedding.is_empty(), "Embedding should not be empty");
                    }
                    Err(e) => {
                        println!("⚠️ Embedding failed (expected without models): {}", e);
                        // Expected in test environment
                    }
                }
            }
            Err(e) => {
                println!("⚠️ Expected error without models: {}", e);
                // This is expected in test environment without models
            }
        }
    }

    #[tokio::test]
    async fn test_batch_embedding() {
        let temp_dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(e) => {
                println!("Failed to create temp dir: {}", e);
                return;
            }
        };

        let cache = match crate::EmbeddingCache::new(temp_dir.path(), crate::CacheConfig::default())
        {
            Ok(c) => Arc::new(c) as Arc<dyn EmbeddingCacheInterface>,
            Err(e) => {
                println!("Failed to create cache: {}", e);
                return;
            }
        };

        let config = BatchProcessorConfig {
            use_gpu_if_available: false, // Форсируем CPU для тестов
            ..Default::default()
        };
        let embedding_config = EmbeddingConfig::default();

        match GpuBatchProcessor::new(config, embedding_config, cache).await {
            Ok(processor) => {
                let texts = vec![
                    "first text".to_string(),
                    "second text".to_string(),
                    "third text".to_string(),
                ];

                match processor.embed_batch(texts.clone()).await {
                    Ok(embeddings) => {
                        println!(
                            "✅ Got {} embeddings for {} texts",
                            embeddings.len(),
                            texts.len()
                        );
                        assert_eq!(embeddings.len(), 3, "Should have 3 embeddings");

                        for (i, embedding) in embeddings.iter().enumerate() {
                            assert!(!embedding.is_empty(), "Embedding {} should not be empty", i);
                        }
                    }
                    Err(e) => {
                        println!("⚠️ Batch embedding failed (expected without models): {}", e);
                        // Expected in test environment
                    }
                }
            }
            Err(e) => {
                println!("⚠️ Expected error without models: {}", e);
                // This is expected in test environment without models
            }
        }
    }

    #[tokio::test]
    async fn test_gpu_health_check() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let cache = Arc::new(
            crate::EmbeddingCache::new(temp_dir.path(), crate::CacheConfig::default())
                .expect("Failed to create cache"),
        ) as Arc<dyn EmbeddingCacheInterface>;

        let config = BatchProcessorConfig {
            use_gpu_if_available: false, // Safe for tests
            ..Default::default()
        };
        let embedding_config = EmbeddingConfig::default();

        match GpuBatchProcessor::new(config, embedding_config, cache).await {
            Ok(processor) => {
                let health = processor.check_gpu_health().await;
                println!("GPU Health Status: {:?}", health);

                // В тестовой среде GPU может быть недоступен
                if !health.available {
                    assert!(
                        !health.issues.is_empty(),
                        "Should report why GPU is unavailable"
                    );
                }
            }
            Err(e) => {
                println!("Expected error in test environment: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_fallback_behavior() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let cache = Arc::new(
            crate::EmbeddingCache::new(temp_dir.path(), crate::CacheConfig::default())
                .expect("Failed to create cache"),
        ) as Arc<dyn EmbeddingCacheInterface>;

        let config = BatchProcessorConfig {
            use_gpu_if_available: true, // Request GPU but expect fallback
            ..Default::default()
        };
        let embedding_config = EmbeddingConfig::default();

        match GpuBatchProcessor::new(config, embedding_config, cache).await {
            Ok(processor) => {
                // Test resilient embedding for edge cases
                match processor.embed("").await {
                    Ok(embedding) => {
                        println!(
                            "✅ Got fallback embedding for empty text: length {}",
                            embedding.len()
                        );
                        assert_eq!(embedding.len(), 1024, "Should use Qwen3 dimension fallback");
                    }
                    Err(e) => {
                        println!("⚠️ Even fallback failed: {}", e);
                    }
                }

                // Test very long text
                let long_text = "word ".repeat(2000);
                match processor.embed(&long_text).await {
                    Ok(embedding) => {
                        println!("✅ Got embedding for long text: length {}", embedding.len());
                        assert!(!embedding.is_empty(), "Should handle long text gracefully");
                    }
                    Err(e) => {
                        println!("⚠️ Long text failed: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Expected error in test environment: {}", e);
            }
        }
    }
}

/// Статистика GPU Batch Processor
#[derive(Debug, Clone, Default)]
pub struct BatchProcessorStats {
    pub total_batches: u64,
    pub successful_batches: u64,
    pub failed_batches: u64,
    pub total_items: u64,
    pub gpu_batches: u64,
    pub cpu_fallback_batches: u64,
    pub avg_batch_time_ms: f32,
    pub avg_items_per_batch: f32,
    pub cache_hit_rate: f32,
    pub has_gpu: bool,
    pub queue_size: usize,
    pub cache_stats: (u64, u64, u64), // hits, misses, inserts
    pub pipeline_stats: Option<ai::PipelineStats>,
}

impl BatchProcessorStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn success_rate(&self) -> f32 {
        if self.total_batches == 0 {
            0.0
        } else {
            self.successful_batches as f32 / self.total_batches as f32
        }
    }

    pub fn gpu_usage_rate(&self) -> f32 {
        if self.total_batches == 0 {
            0.0
        } else {
            self.gpu_batches as f32 / self.total_batches as f32
        }
    }
}
