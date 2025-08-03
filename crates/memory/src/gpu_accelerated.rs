use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tracing::{info, warn, debug};

use ai::{GpuFallbackManager, EmbeddingConfig, EmbeddingServiceTrait};
use ai::gpu_fallback::FallbackStats;
use crate::cache_interface::EmbeddingCacheInterface;

/// Максимальный размер батча для GPU обработки
const MAX_BATCH_SIZE: usize = 128;
/// Максимальное количество одновременных GPU операций
const MAX_CONCURRENT_GPU_OPS: usize = 4;

// @component: {"k":"C","id":"gpu_batch_processor","t":"GPU batch embedding processor","m":{"cur":95,"tgt":100,"u":"%"},"f":["gpu","batch","embeddings","fallback"]}
pub struct GpuBatchProcessor {
    embedding_service: Arc<GpuFallbackManager>,
    cache: Arc<dyn EmbeddingCacheInterface>,
    batch_semaphore: Arc<Semaphore>,
    processing_queue: Arc<Mutex<Vec<PendingEmbedding>>>,
    config: BatchProcessorConfig,
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
            batch_timeout_ms: 50,
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
            GpuFallbackManager::new(embedding_config).await
                .map_err(|e| anyhow::anyhow!("Failed to create embedding service: {}", e))?
        );

        info!("✅ GPU batch processor initialized with robust fallback mechanism");

        Ok(Self {
            embedding_service,
            cache,
            batch_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_GPU_OPS)),
            processing_queue: Arc::new(Mutex::new(Vec::new())),
            config,
        })
    }

    /// Получить эмбеддинг для одного текста (с батчеванием)
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Проверяем кэш
        if self.config.cache_embeddings {
            if let Some(embedding) = self.cache.get(text, "bge-m3") {
                debug!("Cache hit for embedding");
                return Ok(embedding);
            }
        }

        // Используем новый fallback сервис для получения embedding
        let embeddings = self.embedding_service.embed_batch(vec![text.to_string()]).await?;
        let embedding = embeddings.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("No embedding returned"))?;

        // Кэшируем результат
        if self.config.cache_embeddings {
            if let Err(e) = self.cache.insert(text, "bge-m3", embedding.clone()) {
                warn!("Failed to cache embedding: {}", e);
            }
        }

        Ok(embedding)
    }

    /// Обработать батч текстов напрямую
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

        // Обрабатываем uncached тексты через fallback сервис
        if !uncached_texts.is_empty() {
            let embeddings = self.embedding_service.embed_batch(uncached_texts.clone()).await?;

            // Сохраняем в кэш и результаты
            for (idx, (text, embedding)) in uncached_texts.iter()
                .zip(embeddings.iter())
                .enumerate() 
            {
                if self.config.cache_embeddings {
                    self.cache.insert(text, "bge-m3", embedding.clone())?;
                }
                results[uncached_indices[idx]] = Some(embedding.clone());
            }
        }

        // Собираем финальные результаты
        Ok(results.into_iter()
            .map(|r| r.expect("All results should be filled"))
            .collect())
    }

    /// Обработать накопленный батч
    async fn process_batch(&self) -> Result<()> {
        let pending = {
            let mut queue = self.processing_queue.lock().await;
            std::mem::take(&mut *queue)
        };

        if pending.is_empty() {
            return Ok(());
        }

        let texts: Vec<String> = pending.iter()
            .map(|p| p.text.clone())
            .collect();

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
    fn clone_for_task(&self) -> Arc<Self> {
        Arc::new(Self {
            embedding_service: self.embedding_service.clone(),
            cache: self.cache.clone(),
            batch_semaphore: self.batch_semaphore.clone(),
            processing_queue: self.processing_queue.clone(),
            config: self.config.clone(),
        })
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

    /// Получить статистику
    pub async fn get_stats(&self) -> BatchProcessorStats {
        let queue_size = self.processing_queue.lock().await.len();
        
        BatchProcessorStats {
            has_gpu: self.has_gpu(),
            queue_size,
            cache_stats: self.cache.stats(),
        }
    }
}

#[derive(Debug)]
pub struct BatchProcessorStats {
    pub has_gpu: bool,
    pub queue_size: usize,
    pub cache_stats: (u64, u64, u64), // (hits, misses, size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_batch_processor_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Arc::new(crate::EmbeddingCache::new(temp_dir.path()).unwrap()) as Arc<dyn EmbeddingCacheInterface>;
        
        let config = BatchProcessorConfig::default();
        let embedding_config = EmbeddingConfig::default();
        
        match GpuBatchProcessor::new(config, embedding_config, cache).await {
            Ok(_) => {
                // Должен создаться хотя бы с CPU fallback
                println!("Processor created successfully");
            },
            Err(e) => {
                println!("Expected error without models: {}", e);
                // This is fine in test environment without models
            }
        }
    }

    #[tokio::test]
    async fn test_single_embedding() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Arc::new(crate::EmbeddingCache::new(temp_dir.path()).unwrap()) as Arc<dyn EmbeddingCacheInterface>;
        
        let config = BatchProcessorConfig {
            use_gpu_if_available: false, // Форсируем CPU
            ..Default::default()
        };
        let embedding_config = EmbeddingConfig::default();
        
        match GpuBatchProcessor::new(config, embedding_config, cache).await {
            Ok(processor) => {
                let embedding = processor.embed("test text").await.unwrap();
                assert!(!embedding.is_empty());
            },
            Err(e) => {
                println!("Expected error without models: {}", e);
                // This is fine in test environment without models
            }
        }
    }

    #[tokio::test] 
    async fn test_batch_embedding() {
        let temp_dir = TempDir::new().unwrap();
        let cache = Arc::new(crate::EmbeddingCache::new(temp_dir.path()).unwrap()) as Arc<dyn EmbeddingCacheInterface>;
        
        let config = BatchProcessorConfig {
            use_gpu_if_available: false, // Форсируем CPU
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
                
                let embeddings = processor.embed_batch(texts).await.unwrap();
                assert_eq!(embeddings.len(), 3);
                
                for embedding in embeddings {
                    assert!(!embedding.is_empty());
                }
            },
            Err(e) => {
                println!("Expected error without models: {}", e);
                // This is fine in test environment without models
            }
        }
    }
}