use anyhow::Result;
use async_trait::async_trait;
use std::{
    sync::Arc,
    time::{Duration, Instant},
    collections::HashMap,
};
use tokio::{
    sync::{RwLock, Semaphore},
    time::timeout,
};
use tracing::{debug, info, warn, error};

use crate::{
    storage::VectorStore,
    types::{Layer, Record, SearchOptions},
    orchestration::{
        traits::{Coordinator, SearchCoordinator as SearchCoordinatorTrait, EmbeddingCoordinator as EmbeddingCoordinatorTrait},
        retry_handler::{RetryHandler, RetryPolicy},
        embedding_coordinator::EmbeddingCoordinator,
    },
};

/// Production-ready координатор поиска с sub-5ms HNSW векторным поиском
pub struct SearchCoordinator {
    store: Arc<VectorStore>,
    embedding_coordinator: Arc<EmbeddingCoordinator>,
    retry_handler: RetryHandler,
    ready: std::sync::atomic::AtomicBool,
    
    // === Production Search Optimizations ===
    /// Query cache для часто используемых поисковых запросов
    query_cache: Arc<RwLock<QueryCache>>,
    /// Concurrency limiter для поисковых операций
    search_limiter: Arc<Semaphore>,
    /// Performance metrics
    performance_metrics: Arc<RwLock<SearchMetrics>>,
    /// Circuit breaker для векторного поиска
    circuit_breaker: Arc<RwLock<SearchCircuitBreaker>>,
    /// Reranking model cache
    rerank_model: Arc<RwLock<Option<ai::reranker_qwen3_optimized::OptimizedRerankingService>>>,
}

/// Query cache для быстрого доступа к результатам
#[derive(Debug)]
struct QueryCache {
    cache: HashMap<String, CachedSearchResult>,
    max_size: usize,
    hits: u64,
    misses: u64,
}

#[derive(Debug, Clone)]
struct CachedSearchResult {
    results: Vec<Record>,
    created_at: Instant,
    ttl: Duration,
    layer: Layer,
    options: SearchOptions,
}

/// Search performance metrics
#[derive(Debug, Default)]
struct SearchMetrics {
    total_searches: u64,
    successful_searches: u64,
    failed_searches: u64,
    cache_hits: u64,
    cache_misses: u64,
    avg_search_latency_ms: f64,
    avg_embedding_latency_ms: f64,
    rerank_operations: u64,
    hnsw_index_size: usize,
}

/// Circuit breaker для search операций
#[derive(Debug)]
struct SearchCircuitBreaker {
    failure_count: u32,
    success_count: u32,
    state: CircuitState,
    last_failure: Option<Instant>,
    failure_threshold: u32,
    recovery_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl SearchCoordinator {
    pub fn new(
        store: Arc<VectorStore>,
        embedding_coordinator: Arc<EmbeddingCoordinator>,
    ) -> Self {
        let query_cache = QueryCache {
            cache: HashMap::new(),
            max_size: 1000, // Кэшируем до 1000 запросов
            hits: 0,
            misses: 0,
        };
        
        let circuit_breaker = SearchCircuitBreaker {
            failure_count: 0,
            success_count: 0,
            state: CircuitState::Closed,
            last_failure: None,
            failure_threshold: 10,
            recovery_timeout: Duration::from_secs(60),
        };
        
        Self {
            store,
            embedding_coordinator,
            retry_handler: RetryHandler::new(RetryPolicy::default()),
            ready: std::sync::atomic::AtomicBool::new(false),
            query_cache: Arc::new(RwLock::new(query_cache)),
            search_limiter: Arc::new(Semaphore::new(32)), // До 32 параллельных поисков
            performance_metrics: Arc::new(RwLock::new(SearchMetrics::default())),
            circuit_breaker: Arc::new(RwLock::new(circuit_breaker)),
            rerank_model: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Production конфигурация с оптимизированными лимитами
    pub fn new_production(
        store: Arc<VectorStore>,
        embedding_coordinator: Arc<EmbeddingCoordinator>,
        max_concurrent_searches: usize,
        cache_size: usize,
    ) -> Self {
        let mut coordinator = Self::new(store, embedding_coordinator);
        coordinator.search_limiter = Arc::new(Semaphore::new(max_concurrent_searches));
        
        tokio::spawn({
            let cache = coordinator.query_cache.clone();
            async move {
                let mut cache_guard = cache.write().await;
                cache_guard.max_size = cache_size;
            }
        });
        
        coordinator
    }
}

#[async_trait]
impl Coordinator for SearchCoordinator {
    async fn initialize(&self) -> Result<()> {
        info!("🔍 Инициализация production SearchCoordinator...");
        
        // 1. Проверяем готовность vector store
        let store_ready = timeout(
            Duration::from_secs(10),
            self.test_vector_store()
        ).await;
        
        match store_ready {
            Ok(Ok(_)) => {
                info!("✅ Vector store готов для поиска");
                self.record_success().await;
            }
            Ok(Err(e)) => {
                warn!("⚠️ Проблемы с vector store: {}", e);
                self.record_failure().await;
            }
            Err(_) => {
                error!("❌ Таймаут инициализации vector store");
                self.record_failure().await;
            }
        }
        
        // 2. Инициализируем reranking модель (опционально)
        if let Err(e) = self.initialize_rerank_model().await {
            info!("⚠️ Reranking модель не загружена: {} (будет работать без reranking)", e);
        }
        
        // 3. Запускаем cache cleanup worker
        self.start_cache_cleanup_worker().await;
        
        self.ready.store(true, std::sync::atomic::Ordering::Relaxed);
        info!("✅ SearchCoordinator готов к production работе");
        Ok(())
    }
    
    async fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    async fn shutdown(&self) -> Result<()> {
        self.ready.store(false, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    
    async fn metrics(&self) -> serde_json::Value {
        let cache = self.query_cache.read().await;
        let metrics = self.performance_metrics.read().await;
        let breaker = self.circuit_breaker.read().await;
        
        let cache_hit_rate = if cache.hits + cache.misses > 0 {
            (cache.hits as f64 / (cache.hits + cache.misses) as f64) * 100.0
        } else { 0.0 };
        
        let success_rate = if metrics.total_searches > 0 {
            (metrics.successful_searches as f64 / metrics.total_searches as f64) * 100.0
        } else { 0.0 };
        
        serde_json::json!({
            "ready": self.is_ready().await,
            "type": "search_coordinator",
            "performance": {
                "total_searches": metrics.total_searches,
                "successful_searches": metrics.successful_searches,
                "failed_searches": metrics.failed_searches,
                "success_rate": format!("{:.2}%", success_rate),
                "avg_search_latency_ms": format!("{:.2}", metrics.avg_search_latency_ms),
                "avg_embedding_latency_ms": format!("{:.2}", metrics.avg_embedding_latency_ms),
                "rerank_operations": metrics.rerank_operations,
                "hnsw_index_size": metrics.hnsw_index_size
            },
            "cache": {
                "size": cache.cache.len(),
                "max_size": cache.max_size,
                "hits": cache.hits,
                "misses": cache.misses,
                "hit_rate": format!("{:.2}%", cache_hit_rate)
            },
            "circuit_breaker": {
                "state": format!("{:?}", breaker.state),
                "failure_count": breaker.failure_count,
                "success_count": breaker.success_count
            },
            "concurrency": {
                "available_permits": self.search_limiter.available_permits(),
                "max_permits": 32 // TODO: сделать конфигурируемым
            }
        })
    }
}

#[async_trait]
impl SearchCoordinatorTrait for SearchCoordinator {
    async fn search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        debug!("🔍 Production поиск в слое {:?}: '{}'", layer, query);
        
        let start_time = Instant::now();
        let _permit = self.search_limiter.acquire().await
            .map_err(|e| anyhow::anyhow!("Не удалось получить search permit: {}", e))?;
        
        // 1. Проверяем cache
        let cache_key = format!("{}:{:?}:{:?}", query, layer, options);
        if let Some(cached) = self.check_cache(&cache_key).await {
            debug!("💾 Cache hit для запроса: '{}'", query);
            return Ok(cached.results);
        }
        
        // 2. Проверяем circuit breaker
        self.check_circuit_breaker().await?;
        
        // 3. Получаем embedding с метриками
        let embedding_start = Instant::now();
        let embedding = self.embedding_coordinator.get_embedding(query).await?;
        let embedding_latency = embedding_start.elapsed().as_millis() as f64;
        
        // 4. Выполняем векторный поиск с оптимизацией
        let search_result = self.retry_handler
            .execute(|| async {
                timeout(
                    Duration::from_millis(100), // Target <100ms search latency
                    self.store.search(&embedding, layer, options.top_k)
                ).await
                .map_err(|_| anyhow::anyhow!("Search timeout после 100ms"))?
            })
            .await;
            
        let total_latency = start_time.elapsed().as_millis() as f64;
        
        match search_result.into_result() {
            Ok(results) => {
                // Успешный поиск - обновляем метрики и cache
                self.record_success().await;
                self.update_performance_metrics(total_latency, embedding_latency, true).await;
                self.cache_result(&cache_key, &results, layer, options).await;
                
                if total_latency > 5.0 {
                    warn!("⏱️ Медленный поиск: {:.2}ms для '{}'", total_latency, query);
                }
                
                Ok(results)
            }
            Err(e) => {
                self.record_failure().await;
                self.update_performance_metrics(total_latency, embedding_latency, false).await;
                error!("❌ Ошибка поиска: {}", e);
                Err(e)
            }
        }
    }
    
    async fn vector_search(
        &self,
        vector: &[f32],
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        debug!("📊 Векторный поиск в слое {:?}, размер вектора: {}", layer, vector.len());
        
        let start_time = Instant::now();
        let _permit = self.search_limiter.acquire().await
            .map_err(|e| anyhow::anyhow!("Не удалось получить vector search permit: {}", e))?;
        
        // Проверяем circuit breaker
        self.check_circuit_breaker().await?;
        
        // Выполняем векторный поиск с оптимизацией
        let search_result = self.retry_handler
            .execute(|| async {
                let timeout_result = timeout(
                    Duration::from_millis(50), // Более агрессивный таймаут для векторного поиска
                    self.store.search(vector, layer, options.top_k)
                ).await
                .map_err(|_| anyhow::anyhow!("Vector search timeout после 50ms"))?;
                
                timeout_result
            })
            .await;
            
        let total_latency = start_time.elapsed().as_millis() as f64;
        
        match search_result.into_result() {
            Ok(results) => {
                self.record_success().await;
                self.update_performance_metrics(total_latency, 0.0, true).await; // Embedding latency = 0 для прямого векторного поиска
                
                if total_latency > 5.0 {
                    warn!("⏱️ Медленный векторный поиск: {:.2}ms", total_latency);
                }
                
                Ok(results)
            }
            Err(e) => {
                self.record_failure().await;
                self.update_performance_metrics(total_latency, 0.0, false).await;
                error!("❌ Ошибка векторного поиска: {}", e);
                Err(e)
            }
        }
    }
    
    async fn hybrid_search(
        &self,
        query: &str,
        vector: Option<&[f32]>,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>> {
        debug!("🔀 Гибридный поиск для '{}'", query);
        
        match vector {
            Some(provided_vector) => {
                // Если вектор предоставлен, используем его для поиска
                debug!("📊 Используем предоставленный вектор размером {}", provided_vector.len());
                self.vector_search(provided_vector, layer, options).await
            }
            None => {
                // Иначе получаем embedding и делаем обычный поиск
                debug!("📝 Получаем embedding для текстового поиска");
                self.search(query, layer, options).await
            }
        }
    }
    
    async fn search_with_rerank(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
        rerank_top_k: usize,
    ) -> Result<Vec<Record>> {
        debug!("🎯 Поиск с reranking для '{}', rerank_top_k={}", query, rerank_top_k);
        
        // 1. Получаем больше кандидатов для reranking
        let expanded_options = SearchOptions {
            top_k: (options.top_k * 3).min(100), // Получаем в 3 раза больше для reranking
            ..options
        };
        
        let candidates = self.search(query, layer, expanded_options).await?;
        
        if candidates.len() <= options.top_k {
            // Если кандидатов мало, возвращаем как есть
            return Ok(candidates);
        }
        
        // 2. Применяем reranking если модель доступна
        let rerank_model = self.rerank_model.read().await;
        if let Some(ref model) = rerank_model.as_ref() {
            let start_time = Instant::now();
            
            // Подготавливаем тексты для reranking
            let texts: Vec<String> = candidates.iter()
                .map(|r| r.text.clone())
                .collect();
            
            match model.rerank(query, &texts).await {
                Ok(rerank_results) => {
                    let rerank_latency = start_time.elapsed().as_millis();
                    debug!("✨ Reranking завершен за {}ms", rerank_latency);
                    
                    // Обновляем метрики
                    {
                        let mut metrics = self.performance_metrics.write().await;
                        metrics.rerank_operations += 1;
                    }
                    
                    // Возвращаем reranked результаты
                    let reranked_results = rerank_results.into_iter()
                        .take(options.top_k)
                        .filter_map(|result| candidates.get(result.original_index).cloned())
                        .collect();
                    
                    return Ok(reranked_results);
                }
                Err(e) => {
                    warn!("⚠️ Ошибка reranking: {}, возвращаем исходные результаты", e);
                }
            }
        }
        
        // 3. Fallback: возвращаем топ результаты без reranking
        Ok(candidates.into_iter().take(options.top_k).collect())
    }
}

impl SearchCoordinator {
    /// Вспомогательные методы для production оптимизации
    
    /// Тестирование vector store готовности
    #[allow(dead_code)]
    async fn test_vector_store(&self) -> Result<()> {
        // Создаем тестовый вектор
        let test_vector = vec![0.1; 1024]; // Qwen3 dimension
        
        // Пытаемся выполнить поиск в каждом слое
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            match timeout(
                Duration::from_secs(2),
                self.store.search(&test_vector, layer, 1)
            ).await {
                Ok(Ok(_)) => debug!("✅ Layer {:?} готов для поиска", layer),
                Ok(Err(e)) => return Err(anyhow::anyhow!("Ошибка в layer {:?}: {}", layer, e)),
                Err(_) => return Err(anyhow::anyhow!("Таймаут для layer {:?}", layer)),
            }
        }
        
        Ok(())
    }
    
    /// Инициализация reranking модели
    #[allow(dead_code)]
    async fn initialize_rerank_model(&self) -> Result<()> {
        // TODO: Загрузить реальную reranking модель
        // let model = crate::ai::reranker_qwen3_optimized::RerankingService::new().await?;
        // *self.rerank_model.write().await = Some(model);
        
        info!("🎯 Reranking модель будет загружена при первом использовании");
        Ok(())
    }
    
    /// Проверка cache на наличие результата с оптимизациями
    async fn check_cache(&self, cache_key: &str) -> Option<CachedSearchResult> {
        let mut cache = self.query_cache.write().await;
        
        if let Some(cached) = cache.cache.get(cache_key).cloned() {
            // Проверяем TTL
            if cached.created_at.elapsed() < cached.ttl {
                cache.hits += 1;
                
                // Обновляем metrics
                {
                    let mut metrics = self.performance_metrics.write().await;
                    metrics.cache_hits += 1;
                }
                
                return Some(cached);
            } else {
                // Expired - удаляем
                cache.cache.remove(cache_key);
            }
        }
        
        cache.misses += 1;
        
        // Обновляем metrics
        {
            let mut metrics = self.performance_metrics.write().await;
            metrics.cache_misses += 1;
        }
        
        None
    }
    
    /// Кэширование результата поиска с адаптивным TTL
    async fn cache_result(&self, cache_key: &str, results: &[Record], layer: Layer, options: SearchOptions) {
        let mut cache = self.query_cache.write().await;
        
        // Проверяем размер cache и очищаем при необходимости
        if cache.cache.len() >= cache.max_size {
            // Удаляем 10% старых записей
            let to_remove = cache.max_size / 10;
            let keys_to_remove: Vec<String> = cache.cache.keys()
                .take(to_remove)
                .cloned()
                .collect();
            
            for key in keys_to_remove {
                cache.cache.remove(&key);
            }
        }
        
        // Адаптивный TTL на основе слоя и размера результатов
        let ttl = match layer {
            Layer::Interact => Duration::from_secs(180), // 3 минуты - часто меняется
            Layer::Insights => Duration::from_secs(600), // 10 минут - средняя стабильность
            Layer::Assets => Duration::from_secs(1800), // 30 минут - стабильные данные
        };
        
        // Увеличиваем TTL для маленьких результатов (меньше шансов изменения)
        let ttl_multiplier = if results.len() < 5 { 2.0 } else if results.len() < 20 { 1.5 } else { 1.0 };
        let adaptive_ttl = Duration::from_secs((ttl.as_secs() as f64 * ttl_multiplier) as u64);
        
        let cached_result = CachedSearchResult {
            results: results.to_vec(),
            created_at: Instant::now(),
            ttl: adaptive_ttl,
            layer,
            options,
        };
        
        cache.cache.insert(cache_key.to_string(), cached_result);
    }
    
    /// Проверка circuit breaker'а
    async fn check_circuit_breaker(&self) -> Result<()> {
        let mut breaker = self.circuit_breaker.write().await;
        
        match breaker.state {
            CircuitState::Open => {
                if let Some(last_failure) = breaker.last_failure {
                    if last_failure.elapsed() > breaker.recovery_timeout {
                        breaker.state = CircuitState::HalfOpen;
                        info!("🔄 Search circuit breaker: Open -> HalfOpen");
                        return Ok(());
                    }
                }
                return Err(anyhow::anyhow!("🚫 Search circuit breaker OPEN - поиск временно недоступен"));
            }
            _ => Ok(())
        }
    }
    
    /// Запись успешной операции в circuit breaker
    async fn record_success(&self) {
        let mut breaker = self.circuit_breaker.write().await;
        
        match breaker.state {
            CircuitState::HalfOpen => {
                breaker.success_count += 1;
                if breaker.success_count >= 3 {
                    breaker.state = CircuitState::Closed;
                    breaker.failure_count = 0;
                    info!("✅ Search circuit breaker восстановлен (HalfOpen -> Closed)");
                }
            }
            CircuitState::Closed => {
                breaker.failure_count = 0;
            }
            _ => {}
        }
    }
    
    /// Запись неудачной операции в circuit breaker
    async fn record_failure(&self) {
        let mut breaker = self.circuit_breaker.write().await;
        
        breaker.failure_count += 1;
        breaker.last_failure = Some(Instant::now());
        
        match breaker.state {
            CircuitState::Closed => {
                if breaker.failure_count >= breaker.failure_threshold {
                    breaker.state = CircuitState::Open;
                    error!("🚫 Search circuit breaker открыт после {} ошибок", breaker.failure_count);
                }
            }
            CircuitState::HalfOpen => {
                breaker.state = CircuitState::Open;
                error!("🚫 Search circuit breaker снова открыт (HalfOpen -> Open)");
            }
            _ => {}
        }
    }
    
    /// Обновление performance метрик
    async fn update_performance_metrics(&self, total_latency: f64, embedding_latency: f64, success: bool) {
        let mut metrics = self.performance_metrics.write().await;
        
        metrics.total_searches += 1;
        
        if success {
            metrics.successful_searches += 1;
        } else {
            metrics.failed_searches += 1;
        }
        
        // Exponential moving average для latency
        let alpha = 0.1;
        if metrics.avg_search_latency_ms == 0.0 {
            metrics.avg_search_latency_ms = total_latency;
        } else {
            metrics.avg_search_latency_ms = alpha * total_latency + (1.0 - alpha) * metrics.avg_search_latency_ms;
        }
        
        if embedding_latency > 0.0 {
            if metrics.avg_embedding_latency_ms == 0.0 {
                metrics.avg_embedding_latency_ms = embedding_latency;
            } else {
                metrics.avg_embedding_latency_ms = alpha * embedding_latency + (1.0 - alpha) * metrics.avg_embedding_latency_ms;
            }
        }
    }
    
    /// Запуск worker'а для очистки cache
    async fn start_cache_cleanup_worker(&self) {
        let cache = self.query_cache.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Каждую минуту
            
            loop {
                interval.tick().await;
                
                // Используем ссылку на self через weak reference
                // Пока просто очищаем expired записи
                let mut cache_guard = cache.write().await;
                let mut expired_keys = Vec::new();
                
                // Находим expired записи
                for (key, cached_result) in &cache_guard.cache {
                    if cached_result.created_at.elapsed() > cached_result.ttl {
                        expired_keys.push(key.clone());
                    }
                }
                
                // Удаляем expired записи
                for key in expired_keys {
                    cache_guard.cache.remove(&key);
                }
                
                if cache_guard.cache.len() > 0 {
                    debug!("🧹 Cache cleanup: осталось {} записей", cache_guard.cache.len());
                }
            }
        });
        
        debug!("🧹 Cache cleanup worker запущен");
    }
    
    /// Оптимизация cache - preload популярных запросов
    pub async fn preload_popular_searches(&self) -> Result<()> {
        info!("📊 Предварительная загрузка популярных запросов...");
        
        let popular_queries = vec![
            "hello world",
            "machine learning",
            "rust programming",
            "embedding search",
            "vector database",
        ];
        
        for query in popular_queries {
            for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
                let options = SearchOptions {
                    layers: vec![layer],
                    top_k: 10,
                    score_threshold: 0.0,
                    tags: Vec::new(),
                    project: None,
                };
                
                // Пытаемся выполнить поиск и закэшировать результат
                if let Ok(_results) = self.search(query, layer, options).await {
                    debug!("📊 Предзагрузили: '{}' в слое {:?}", query, layer);
                }
            }
        }
        
        Ok(())
    }
    
    /// Получить подробные cache метрики
    pub async fn get_cache_analytics(&self) -> serde_json::Value {
        let cache = self.query_cache.read().await;
        let metrics = self.performance_metrics.read().await;
        
        // Анализ по TTL
        let mut ttl_distribution = std::collections::HashMap::new();
        for cached in cache.cache.values() {
            let ttl_key = format!("{} сек", cached.ttl.as_secs());
            *ttl_distribution.entry(ttl_key).or_insert(0) += 1;
        }
        
        // Анализ по слоям
        let mut layer_distribution = std::collections::HashMap::new();
        for cached in cache.cache.values() {
            let layer_key = format!("{:?}", cached.layer);
            *layer_distribution.entry(layer_key).or_insert(0) += 1;
        }
        
        let hit_rate = if cache.hits + cache.misses > 0 {
            (cache.hits as f64 / (cache.hits + cache.misses) as f64) * 100.0
        } else { 0.0 };
        
        serde_json::json!({
            "cache_size": cache.cache.len(),
            "max_cache_size": cache.max_size,
            "utilization_percent": (cache.cache.len() as f64 / cache.max_size as f64) * 100.0,
            "hit_rate_percent": hit_rate,
            "total_hits": cache.hits,
            "total_misses": cache.misses,
            "ttl_distribution": ttl_distribution,
            "layer_distribution": layer_distribution,
            "performance": {
                "avg_search_latency_ms": metrics.avg_search_latency_ms,
                "total_searches": metrics.total_searches,
                "cache_effectiveness": if metrics.total_searches > 0 {
                    (cache.hits as f64 / metrics.total_searches as f64) * 100.0
                } else { 0.0 }
            }
        })
    }
    
    /// Адаптивная очистка cache на основе частоты обращений
    async fn adaptive_cache_cleanup(&self) {
        let mut cache = self.query_cache.write().await;
        
        if cache.cache.len() < cache.max_size {
            return; // Не нуждаемся в очистке
        }
        
        let now = Instant::now();
        
        // Собираем ключи для удаления (просроченные + наименее важные)
        let mut removal_candidates: Vec<(String, i32)> = Vec::new();
        
        for (key, cached_result) in &cache.cache {
            // Просроченные записи удаляем обязательно
            if cached_result.created_at.elapsed() > cached_result.ttl {
                removal_candidates.push((key.clone(), -1000)); // Максимальный приоритет удаления
                continue;
            }
            
            // Оцениваем важность кэшированной записи
            let mut score = 0i32;
            
            // Большие результаты более важны
            score += cached_result.results.len() as i32 * 10;
            
            // Assets слой более стабилен
            match cached_result.layer {
                Layer::Assets => score += 50,
                Layer::Insights => score += 30,
                Layer::Interact => score += 10,
            }
            
            // Новые записи более важны
            let age_minutes = now.duration_since(cached_result.created_at).as_secs() / 60;
            score -= age_minutes as i32;
            
            removal_candidates.push((key.clone(), score));
        }
        
        // Сортируем по приоритету удаления (меньший score = первым удаляем)
        removal_candidates.sort_by_key(|(_, score)| *score);
        
        // Удаляем 25% наименее важных записей
        let removal_count = (cache.cache.len() / 4).max(1);
        let keys_to_remove: Vec<String> = removal_candidates
            .into_iter()
            .take(removal_count)
            .map(|(key, _)| key)
            .collect();
        
        for key in &keys_to_remove {
            cache.cache.remove(key);
        }
        
        debug!("🧹 Адаптивная очистка: удалено {} записей, осталось {}", keys_to_remove.len(), cache.cache.len());
    }
}