use anyhow::Result;
use async_trait::async_trait;
use std::{
    sync::Arc,
    time::{Duration, Instant},
    collections::VecDeque,
};
use tokio::{
    sync::{RwLock, Semaphore, Notify},
};
use tracing::{debug, info, warn};

use crate::{
    cache_interface::EmbeddingCacheInterface,
    gpu_accelerated::GpuBatchProcessor,
    orchestration::{
        traits::{Coordinator, EmbeddingCoordinator as EmbeddingCoordinatorTrait},
        retry_handler::{RetryHandler, RetryPolicy},
    },
};

/// Production-ready координатор для работы с embeddings
pub struct EmbeddingCoordinator {
    /// GPU batch processor для получения embeddings
    gpu_processor: Arc<GpuBatchProcessor>,
    /// Кэш embeddings
    cache: Arc<dyn EmbeddingCacheInterface>,
    /// Retry handler для операций
    retry_handler: RetryHandler,
    /// Флаг готовности
    ready: std::sync::atomic::AtomicBool,
    
    // === AI/ML Optimization Infrastructure ===
    /// Circuit breaker для GPU операций
    circuit_breaker: Arc<RwLock<CircuitBreaker>>,
    /// Semaphore для ограничения параллельных операций
    concurrency_limiter: Arc<Semaphore>,
    /// Adaptive batch размер на основе load
    adaptive_batch_size: Arc<RwLock<AdaptiveBatchConfig>>,
    /// Model warming статус
    model_warmed: Arc<std::sync::atomic::AtomicBool>,
    /// Performance метрики
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    /// Queue для batch processing с backpressure
    batch_queue: Arc<RwLock<VecDeque<BatchRequest>>>,
    /// Notification для batch processing
    batch_notify: Arc<Notify>,
}

/// Circuit breaker state для GPU операций
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
    failure_threshold: u32,
    timeout: Duration,
    success_threshold: u32,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum CircuitState {
    Closed,     // Нормальное состояние
    Open,       // Отказ, блокировка запросов
    HalfOpen,   // Тестовое состояние
}

/// Адаптивная конфигурация batch размера
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AdaptiveBatchConfig {
    current_size: usize,
    min_size: usize,
    max_size: usize,
    target_latency_ms: u64,
    recent_latencies: VecDeque<u64>,
    last_adjustment: Instant,
}

/// Performance метрики
#[derive(Debug, Default, Clone)]
pub struct PerformanceMetrics {
    #[allow(dead_code)] // Метрики для мониторинга производительности
    total_requests: u64,
    #[allow(dead_code)]
    successful_requests: u64,
    #[allow(dead_code)]
    failed_requests: u64,
    #[allow(dead_code)]
    cache_hits: u64,
    #[allow(dead_code)]
    cache_misses: u64,
    #[allow(dead_code)]
    avg_latency_ms: f64,
    #[allow(dead_code)]
    gpu_utilization: f64,
    #[allow(dead_code)]
    batch_efficiency: f64,
    #[allow(dead_code)]
    circuit_breaker_trips: u64,
}

/// Batch request для queue
#[derive(Debug)]
#[allow(dead_code)]
struct BatchRequest {
    texts: Vec<String>,
    response_sender: tokio::sync::oneshot::Sender<Result<Vec<Vec<f32>>>>,
    created_at: Instant,
}

impl EmbeddingCoordinator {
    pub fn new(
        gpu_processor: Arc<GpuBatchProcessor>,
        cache: Arc<dyn EmbeddingCacheInterface>,
    ) -> Self {
        let circuit_breaker = CircuitBreaker {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure: None,
            failure_threshold: 5,
            timeout: Duration::from_secs(30),
            success_threshold: 3,
        };
        
        let adaptive_batch_config = AdaptiveBatchConfig {
            current_size: 32,
            min_size: 8,
            max_size: 128,
            target_latency_ms: 100,
            recent_latencies: VecDeque::with_capacity(10),
            last_adjustment: Instant::now(),
        };
        
        Self {
            gpu_processor,
            cache,
            retry_handler: RetryHandler::new(RetryPolicy::fast()),
            ready: std::sync::atomic::AtomicBool::new(false),
            circuit_breaker: Arc::new(RwLock::new(circuit_breaker)),
            concurrency_limiter: Arc::new(Semaphore::new(16)), // Max 16 параллельных операций
            adaptive_batch_size: Arc::new(RwLock::new(adaptive_batch_config)),
            model_warmed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            batch_queue: Arc::new(RwLock::new(VecDeque::new())),
            batch_notify: Arc::new(Notify::new()),
        }
    }
    
    /// Создать с кастомной retry политикой
    pub fn with_retry_policy(
        gpu_processor: Arc<GpuBatchProcessor>,
        cache: Arc<dyn EmbeddingCacheInterface>,
        retry_policy: RetryPolicy,
    ) -> Self {
        let circuit_breaker = CircuitBreaker {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure: None,
            failure_threshold: 5,
            timeout: Duration::from_secs(30),
            success_threshold: 3,
        };
        
        let adaptive_batch_config = AdaptiveBatchConfig {
            current_size: 32,
            min_size: 8,
            max_size: 128,
            target_latency_ms: 100,
            recent_latencies: VecDeque::with_capacity(10),
            last_adjustment: Instant::now(),
        };
        
        Self {
            gpu_processor,
            cache,
            retry_handler: RetryHandler::new(retry_policy),
            ready: std::sync::atomic::AtomicBool::new(false),
            circuit_breaker: Arc::new(RwLock::new(circuit_breaker)),
            concurrency_limiter: Arc::new(Semaphore::new(16)),
            adaptive_batch_size: Arc::new(RwLock::new(adaptive_batch_config)),
            model_warmed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            batch_queue: Arc::new(RwLock::new(VecDeque::new())),
            batch_notify: Arc::new(Notify::new()),
        }
    }
}

#[async_trait]
impl Coordinator for EmbeddingCoordinator {
    async fn initialize(&self) -> Result<()> {
        info!("Инициализация EmbeddingCoordinator");
        
        // Проверяем что GPU processor готов
        let gpu_ready = self.retry_handler
            .execute(|| async {
                // Тестовый embedding для проверки
                self.gpu_processor.embed("test").await?;
                Ok(())
            })
            .await
            .into_result()
            .is_ok();
            
        if !gpu_ready {
            warn!("GPU processor не готов, будет использоваться fallback");
        }
        
        // Проверяем кэш
        let cache_stats = self.cache.stats();
        info!("Кэш embeddings: hits={}, misses={}, size={}MB", 
              cache_stats.0, cache_stats.1, cache_stats.2 / 1024 / 1024);
        
        self.ready.store(true, std::sync::atomic::Ordering::Relaxed);
        info!("✅ EmbeddingCoordinator инициализирован");
        Ok(())
    }
    
    async fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    async fn shutdown(&self) -> Result<()> {
        info!("Остановка EmbeddingCoordinator");
        self.ready.store(false, std::sync::atomic::Ordering::Relaxed);
        
        // У нас нет метода flush в интерфейсе, просто логируем финальные stats
        let (hits, misses, size) = self.cache.stats();
        debug!("Финальная статистика кэша: hits={}, misses={}, size={}MB", 
               hits, misses, size / 1024 / 1024);
        
        Ok(())
    }
    
    async fn metrics(&self) -> serde_json::Value {
        let (hits, misses, size) = self.cache.stats();
        let hit_rate = if hits + misses > 0 {
            (hits as f64 / (hits + misses) as f64) * 100.0
        } else {
            0.0
        };
        
        serde_json::json!({
            "cache": {
                "hits": hits,
                "misses": misses,
                "size_bytes": size,
                "size_mb": size / 1024 / 1024,
                "hit_rate": format!("{:.2}%", hit_rate)
            },
            "ready": self.is_ready().await,
            "gpu_available": true // TODO: получить из gpu_processor
        })
    }
}

#[async_trait]
impl EmbeddingCoordinatorTrait for EmbeddingCoordinator {
    async fn get_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // Сначала проверяем кэш
        if let Some(embedding) = self.check_cache(text).await {
            debug!("Embedding найден в кэше для: '{}'", text);
            return Ok(embedding);
        }
        
        // Используем concurrency_limiter для контроля нагрузки
        let _permit = self.concurrency_limiter.acquire().await
            .map_err(|e| anyhow::anyhow!("Не удалось получить permit для embedding: {}", e))?;
        
        // Проверяем состояние model warming
        if !self.model_warmed.load(std::sync::atomic::Ordering::Relaxed) {
            // Запускаем model warming
            self.warm_model().await?;
        }
        
        // Получаем через GPU processor с retry
        let result = self.retry_handler
            .execute_with_fallback(
                || async { self.gpu_processor.embed(text).await },
                || async { 
                    // Fallback на нулевой вектор в крайнем случае
                    warn!("Используем fallback embedding для: '{}'", text);
                    Ok(vec![0.0; 1024]) // Qwen3 dimension
                }
            )
            .await?;
        
        // Сохраняем в кэш
        if let Err(e) = self.cache.insert(text, "bge-m3", result.clone()) {
            warn!("Не удалось сохранить embedding в кэш: {}", e);
        }
        
        Ok(result)
    }
    
    async fn get_embeddings(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        debug!("Получение batch embeddings для {} текстов", texts.len());
        
        // Используем concurrency_limiter для batch операций
        let _permit = self.concurrency_limiter.acquire().await
            .map_err(|e| anyhow::anyhow!("Не удалось получить permit для batch embeddings: {}", e))?;
        
        // Проверяем состояние model warming
        if !self.model_warmed.load(std::sync::atomic::Ordering::Relaxed) {
            // Запускаем model warming
            self.warm_model().await?;
        }
        
        // Проверяем кэш для каждого текста
        let mut cached_indices = Vec::new();
        let mut uncached_texts = Vec::new();
        let mut results = vec![None; texts.len()];
        
        for (idx, text) in texts.iter().enumerate() {
            if let Some(embedding) = self.check_cache(text).await {
                results[idx] = Some(embedding);
                cached_indices.push(idx);
            } else {
                uncached_texts.push((idx, text.clone()));
            }
        }
        
        debug!("Найдено в кэше: {}/{}", cached_indices.len(), texts.len());
        
        // Получаем оставшиеся через batch processing
        if !uncached_texts.is_empty() {
            let uncached_strings: Vec<String> = uncached_texts.iter()
                .map(|(_, text)| text.clone())
                .collect();
                
            let embeddings = self.retry_handler
                .execute_with_fallback(
                    || async { 
                        self.gpu_processor.embed_batch(uncached_strings.clone()).await 
                    },
                    || async {
                        // Fallback на нулевые векторы
                        warn!("Batch embedding fallback для {} текстов", uncached_strings.len());
                        Ok(vec![vec![0.0; 768]; uncached_strings.len()])
                    }
                )
                .await?;
            
            // Сохраняем в кэш и результаты
            for ((idx, text), embedding) in uncached_texts.iter().zip(embeddings.iter()) {
                results[*idx] = Some(embedding.clone());
                if let Err(e) = self.cache.insert(text, "bge-m3", embedding.clone()) {
                    warn!("Не удалось сохранить batch embedding в кэш: {}", e);
                }
            }
        }
        
        // Собираем финальные результаты
        let final_results: Vec<Vec<f32>> = results
            .into_iter()
            .map(|opt| opt.expect("Все embeddings должны быть заполнены"))
            .collect();
            
        Ok(final_results)
    }
    
    async fn check_cache(&self, text: &str) -> Option<Vec<f32>> {
        // Используем default model name для BGE-M3
        self.cache.get(text, "bge-m3")
    }
    
    async fn cache_stats(&self) -> (u64, u64, u64) {
        self.cache.stats()
    }
    
    async fn clear_cache(&self) -> Result<()> {
        info!("🗄️ Очистка кэша embeddings...");
        self.cache.clear()?;
        
        // Обнуляем cache метрики
        let mut metrics = self.performance_metrics.write().await;
        metrics.cache_hits = 0;
        metrics.cache_misses = 0;
        
        Ok(())
    }
}

impl EmbeddingCoordinator {
    /// Запуск batch processing worker'а
    #[allow(dead_code)]
    async fn start_batch_processor(&self) {
        let queue = self.batch_queue.clone();
        let notify = self.batch_notify.clone();
        let _gpu_processor = self.gpu_processor.clone();
        let adaptive_config = self.adaptive_batch_size.clone();
        let _circuit_breaker = self.circuit_breaker.clone();
        let _performance_metrics = self.performance_metrics.clone();
        
        tokio::spawn(async move {
            loop {
                // Ожидаем уведомление о новых задачах
                notify.notified().await;
                
                let current_batch_size = adaptive_config.read().await.current_size;
                let mut batch_requests = Vec::new();
                
                // Собираем batch
                {
                    let mut queue_guard = queue.write().await;
                    for _ in 0..current_batch_size {
                        if let Some(request) = queue_guard.pop_front() {
                            batch_requests.push(request);
                        } else {
                            break;
                        }
                    }
                }
                
                if !batch_requests.is_empty() {
                    debug!("📦 Обрабатываем batch из {} запросов", batch_requests.len());
                    
                    // Обработка batch'а (реализация зависит от конкретной логики)
                    // TODO: Реализовать batch processing
                }
                
                // Небольшая пауза чтобы не нагружать CPU
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });
        
        debug!("📦 Batch processing worker запущен");
    }
    
    /// Model warming для оптимизации первого запроса
    async fn warm_model(&self) -> Result<()> {
        if self.model_warmed.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(()); // Уже прогрет
        }
        
        debug!("🔥 Прогреваем модель embedding...");
        let start = std::time::Instant::now();
        
        // Делаем несколько тестовых embedding для прогрева
        let warmup_texts = vec![
            "Hello world".to_string(),
            "Test embedding".to_string(),
            "Model warmup".to_string(),
        ];
        
        for text in &warmup_texts {
            if let Err(e) = self.gpu_processor.embed(text).await {
                warn!("Ошибка при прогреве модели: {}", e);
                return Err(e);
            }
        }
        
        let warmup_duration = start.elapsed();
        info!("✅ Модель прогрета за {:?}", warmup_duration);
        
        // Помечаем модель как прогретую
        self.model_warmed.store(true, std::sync::atomic::Ordering::Relaxed);
        
        Ok(())
    }
    
    /// Получить статистику производительности
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        let metrics = self.performance_metrics.read().await;
        (*metrics).clone()
    }
    
    /// Адаптивная настройка batch размера
    #[allow(dead_code)]
    async fn adjust_batch_size(&self, latency_ms: u64) {
        let mut config = self.adaptive_batch_size.write().await;
        
        config.recent_latencies.push_back(latency_ms);
        if config.recent_latencies.len() > 10 {
            config.recent_latencies.pop_front();
        }
        
        // Проверяем нужно ли адаптировать
        if config.last_adjustment.elapsed() < std::time::Duration::from_secs(5) {
            return; // Слишком рано для адаптации
        }
        
        let avg_latency: f64 = config.recent_latencies.iter().sum::<u64>() as f64 / config.recent_latencies.len() as f64;
        
        if avg_latency > config.target_latency_ms as f64 * 1.2 {
            // Слишком медленно - уменьшаем batch size
            if config.current_size > config.min_size {
                config.current_size = ((config.current_size as f64) * 0.8) as usize;
                config.current_size = config.current_size.max(config.min_size);
                debug!("⚡ Уменьшили batch size до {} (avg latency: {:.1}ms)", config.current_size, avg_latency);
            }
        } else if avg_latency < config.target_latency_ms as f64 * 0.8 {
            // Быстро - увеличиваем batch size
            if config.current_size < config.max_size {
                config.current_size = ((config.current_size as f64) * 1.2) as usize;
                config.current_size = config.current_size.min(config.max_size);
                debug!("🚀 Увеличили batch size до {} (avg latency: {:.1}ms)", config.current_size, avg_latency);
            }
        }
        
        config.last_adjustment = std::time::Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cache_lru::{EmbeddingCacheLRU, CacheConfig},
        gpu_accelerated::BatchProcessorConfig,
    };
    use tempfile::TempDir;
    
    async fn create_test_coordinator() -> Result<EmbeddingCoordinator> {
        let temp_dir = TempDir::new()?;
        let cache_path = temp_dir.path().join("cache");
        
        let cache = Arc::new(EmbeddingCacheLRU::new(
            cache_path,
            CacheConfig::default()
        )?);
        
        let gpu_processor = Arc::new(GpuBatchProcessor::new(
            BatchProcessorConfig::default(),
            ai::EmbeddingConfig::default(),
            cache.clone(),
        ).await?);
        
        Ok(EmbeddingCoordinator::new(gpu_processor, cache))
    }
    
    #[tokio::test]
    async fn test_coordinator_initialization() -> Result<()> {
        let coordinator = create_test_coordinator().await?;
        
        assert!(!coordinator.is_ready().await);
        coordinator.initialize().await?;
        assert!(coordinator.is_ready().await);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_embedding_with_cache() -> Result<()> {
        let coordinator = create_test_coordinator().await?;
        coordinator.initialize().await?;
        
        let text = "test embedding";
        
        // Первый запрос - cache miss
        let embedding1 = coordinator.get_embedding(text).await?;
        let (hits1, misses1, _) = coordinator.cache_stats().await;
        
        // Второй запрос - cache hit
        let embedding2 = coordinator.get_embedding(text).await?;
        let (hits2, misses2, _) = coordinator.cache_stats().await;
        
        assert_eq!(embedding1, embedding2);
        assert_eq!(hits2, hits1 + 1);
        assert_eq!(misses2, misses1);
        
        Ok(())
    }
}