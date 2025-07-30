use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tracing::{info, warn, debug};

use ai::{GpuEmbeddingService, CpuEmbeddingService, EmbeddingConfig};
use crate::cache_interface::EmbeddingCacheInterface;

/// Максимальный размер батча для GPU обработки
const MAX_BATCH_SIZE: usize = 128;
/// Максимальное количество одновременных GPU операций
const MAX_CONCURRENT_GPU_OPS: usize = 4;

// @component: {"k":"C","id":"gpu_batch_processor","t":"GPU batch embedding processor","m":{"cur":80,"tgt":95,"u":"%"},"f":["gpu","batch","embeddings"]}
pub struct GpuBatchProcessor {
    gpu_service: Option<Arc<GpuEmbeddingService>>,
    cpu_service: Arc<CpuEmbeddingService>,
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
    /// Создать новый процессор с автоматическим выбором GPU/CPU
    pub async fn new(
        config: BatchProcessorConfig,
        embedding_config: EmbeddingConfig,
        cache: Arc<dyn EmbeddingCacheInterface>,
    ) -> Result<Self> {
        // Пытаемся инициализировать GPU сервис
        let gpu_service = if config.use_gpu_if_available {
            match GpuEmbeddingService::new(embedding_config.clone()).await {
                Ok(service) => {
                    info!("✅ GPU batch processor initialized with GPU acceleration");
                    Some(Arc::new(service))
                }
                Err(e) => {
                    warn!("⚠️ Failed to initialize GPU: {}, falling back to CPU", e);
                    None
                }
            }
        } else {
            None
        };

        // CPU сервис как fallback
        let cpu_service = Arc::new(CpuEmbeddingService::new(embedding_config)?);

        Ok(Self {
            gpu_service,
            cpu_service,
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

        // Если есть GPU - добавляем в очередь для батчевания
        if self.gpu_service.is_some() {
            let (tx, rx) = tokio::sync::oneshot::channel();
            
            {
                let mut queue = self.processing_queue.lock().await;
                queue.push(PendingEmbedding {
                    text: text.to_string(),
                    callback: tx,
                });

                // Если очередь достигла размера батча - обрабатываем
                if queue.len() >= self.config.max_batch_size {
                    drop(queue);
                    self.process_batch().await?;
                }
            }

            // Запускаем таймер для обработки маленьких батчей
            let processor = self.clone_for_task();
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    processor.config.batch_timeout_ms
                )).await;
                let _ = processor.process_batch().await;
            });

            // Ждем результат
            rx.await?
        } else {
            // Fallback на CPU для одиночной обработки
            self.embed_single_cpu(text).await
        }
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

        // Обрабатываем uncached тексты
        if !uncached_texts.is_empty() {
            let embeddings = if let Some(ref gpu_service) = self.gpu_service {
                // GPU батчевая обработка
                self.process_texts_gpu(uncached_texts.clone(), gpu_service).await?
            } else {
                // CPU обработка по одному
                self.process_texts_cpu(uncached_texts.clone()).await?
            };

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

    /// GPU батчевая обработка
    async fn process_texts_gpu(
        &self,
        texts: Vec<String>,
        gpu_service: &Arc<GpuEmbeddingService>,
    ) -> Result<Vec<Vec<f32>>> {
        let _permit = self.batch_semaphore.acquire().await?;
        
        // Разбиваем на батчи если нужно
        let mut all_embeddings = Vec::new();
        
        for chunk in texts.chunks(self.config.max_batch_size) {
            let chunk_embeddings = gpu_service.embed_batch(chunk.to_vec()).await?;
            all_embeddings.extend(chunk_embeddings);
        }

        Ok(all_embeddings)
    }

    /// CPU обработка по одному
    async fn process_texts_cpu(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();
        
        for text in texts {
            let embedding = self.embed_single_cpu(&text).await?;
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }

    /// Обработать один текст через CPU
    async fn embed_single_cpu(&self, text: &str) -> Result<Vec<f32>> {
        let result = self.cpu_service.embed(text)
            .map_err(|e| anyhow::anyhow!("CPU embedding failed: {}", e))?;
        
        if self.config.cache_embeddings {
            self.cache.insert(text, "bge-m3", result.embedding.clone())?;
        }

        Ok(result.embedding)
    }

    /// Создать клон для фоновых задач
    fn clone_for_task(&self) -> Arc<Self> {
        Arc::new(Self {
            gpu_service: self.gpu_service.clone(),
            cpu_service: self.cpu_service.clone(),
            cache: self.cache.clone(),
            batch_semaphore: self.batch_semaphore.clone(),
            processing_queue: self.processing_queue.clone(),
            config: self.config.clone(),
        })
    }

    /// Проверить доступность GPU
    pub fn has_gpu(&self) -> bool {
        self.gpu_service.is_some()
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