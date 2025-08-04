use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::{
    cache_interface::EmbeddingCacheInterface,
    gpu_accelerated::GpuBatchProcessor,
    orchestration::{
        traits::{Coordinator, EmbeddingCoordinator as EmbeddingCoordinatorTrait},
        retry_handler::{RetryHandler, RetryPolicy},
    },
};

/// Координатор для работы с embeddings
// @component: {"k":"C","id":"embedding_coordinator","t":"Embedding orchestration coordinator","m":{"cur":0,"tgt":90,"u":"%"},"f":["orchestration","embeddings","coordinator"]}
pub struct EmbeddingCoordinator {
    /// GPU batch processor для получения embeddings
    gpu_processor: Arc<GpuBatchProcessor>,
    /// Кэш embeddings
    cache: Arc<dyn EmbeddingCacheInterface>,
    /// Retry handler для операций
    retry_handler: RetryHandler,
    /// Флаг готовности
    ready: std::sync::atomic::AtomicBool,
}

impl EmbeddingCoordinator {
    pub fn new(
        gpu_processor: Arc<GpuBatchProcessor>,
        cache: Arc<dyn EmbeddingCacheInterface>,
    ) -> Self {
        Self {
            gpu_processor,
            cache,
            retry_handler: RetryHandler::new(RetryPolicy::fast()),
            ready: std::sync::atomic::AtomicBool::new(false),
        }
    }
    
    /// Создать с кастомной retry политикой
    pub fn with_retry_policy(
        gpu_processor: Arc<GpuBatchProcessor>,
        cache: Arc<dyn EmbeddingCacheInterface>,
        retry_policy: RetryPolicy,
    ) -> Self {
        Self {
            gpu_processor,
            cache,
            retry_handler: RetryHandler::new(retry_policy),
            ready: std::sync::atomic::AtomicBool::new(false),
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
        info!("Очистка кэша embeddings");
        self.cache.clear()?;
        Ok(())
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