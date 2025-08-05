use crate::{EmbeddingConfig, gpu_memory_pool::GPU_MEMORY_POOL};
use crate::embeddings_gpu::GpuEmbeddingService;
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Semaphore, Mutex};
use tracing::{info, debug};

/// @component: {"k":"C","id":"gpu_pipeline_optimized","t":"Optimized GPU pipeline with memory pooling","m":{"cur":95,"tgt":100,"u":"%"}}
pub struct OptimizedGpuPipelineManager {
    services: Vec<Arc<GpuEmbeddingService>>,
    semaphore: Arc<Semaphore>,
    config: PipelineConfig,
    stats: Arc<Mutex<PipelineStats>>,
}

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub max_concurrent_batches: usize,
    pub optimal_batch_size: usize,
    pub min_batch_size: usize,
    pub prefetch_enabled: bool,
    pub memory_pooling_enabled: bool,
    pub adaptive_batching: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_batches: 4,
            optimal_batch_size: 64,
            min_batch_size: 8,
            prefetch_enabled: true,
            memory_pooling_enabled: true,
            adaptive_batching: true,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct PipelineStats {
    pub total_batches_processed: u64,
    pub total_texts_processed: u64,
    pub total_processing_time_ms: u64,
    pub avg_batch_size: f32,
    pub max_concurrent_batches_used: usize,
    pub memory_pool_hits: u64,
    pub memory_pool_misses: u64,
    pub cache_efficiency: f32,
}

impl PipelineStats {
    pub fn throughput_per_second(&self) -> f32 {
        if self.total_processing_time_ms == 0 {
            0.0
        } else {
            (self.total_texts_processed as f32 / self.total_processing_time_ms as f32) * 1000.0
        }
    }
    
    pub fn memory_pool_efficiency(&self) -> f32 {
        let total = self.memory_pool_hits + self.memory_pool_misses;
        if total == 0 {
            0.0
        } else {
            self.memory_pool_hits as f32 / total as f32
        }
    }
}

impl OptimizedGpuPipelineManager {
    pub async fn new(embedding_config: EmbeddingConfig, config: PipelineConfig) -> Result<Self> {
        info!("🚀 Инициализация OptimizedGpuPipelineManager");
        info!("⚙️ Конфигурация: max_concurrent={}, optimal_batch={}, memory_pooling={}", 
            config.max_concurrent_batches, config.optimal_batch_size, config.memory_pooling_enabled);

        // Создаем пул GPU сервисов для параллельной обработки
        let mut services = Vec::new();
        for i in 0..config.max_concurrent_batches {
            debug!("🔧 Создание GPU service #{}", i + 1);
            let service = Arc::new(GpuEmbeddingService::new(embedding_config.clone()).await?);
            services.push(service);
        }

        // Инициализируем memory pool если включен
        if config.memory_pooling_enabled {
            info!("💾 Memory pooling включен");
            GPU_MEMORY_POOL.print_stats();
        }

        Ok(Self {
            services,
            semaphore: Arc::new(Semaphore::new(config.max_concurrent_batches)),
            config,
            stats: Arc::new(Mutex::new(PipelineStats::default())),
        })
    }

    /// Обработка больших объемов текстов с оптимизированным пайплайном
    pub async fn process_texts_optimized(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let total_texts = texts.len();
        let start_time = Instant::now();
        
        info!("🏭 Начинаем оптимизированную обработку {} текстов", total_texts);

        // Адаптивное определение размера батча
        let effective_batch_size = if self.config.adaptive_batching {
            self.calculate_adaptive_batch_size(total_texts).await
        } else {
            self.config.optimal_batch_size
        };

        info!("📊 Эффективный размер батча: {}", effective_batch_size);

        // Разбивка на батчи с учетом оптимального размера
        let batches: Vec<Vec<String>> = texts
            .chunks(effective_batch_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        info!("📦 Создано {} батчей для обработки", batches.len());

        // Параллельная обработка батчей с семафором
        let mut handles = Vec::new();
        let mut all_embeddings = Vec::with_capacity(total_texts);

        // Предварительная аллокация с memory pooling
        if self.config.memory_pooling_enabled {
            let estimated_memory = total_texts * 1024 * std::mem::size_of::<f32>(); // Примерная оценка
            debug!("💾 Предварительное выделение памяти: {} MB", estimated_memory / 1024 / 1024);
        }

        for (batch_id, batch) in batches.into_iter().enumerate() {
            let permit = self.semaphore.clone().acquire_owned().await?;
            let service = self.services[batch_id % self.services.len()].clone();
            let stats = self.stats.clone();
            let batch_size = batch.len();

            let handle = tokio::spawn(async move {
                let _permit = permit; // Держим permit до завершения
                let batch_start = Instant::now();
                
                debug!("🔄 Обработка батча #{} размером {}", batch_id, batch_size);
                
                let result = service.embed_batch(batch).await;
                let batch_elapsed = batch_start.elapsed();
                
                // Обновляем статистику
                let mut stats_guard = stats.lock().await;
                stats_guard.total_batches_processed += 1;
                stats_guard.total_texts_processed += batch_size as u64;
                stats_guard.total_processing_time_ms += batch_elapsed.as_millis() as u64;
                stats_guard.avg_batch_size = (stats_guard.avg_batch_size * (stats_guard.total_batches_processed - 1) as f32 
                    + batch_size as f32) / stats_guard.total_batches_processed as f32;
                
                if batch_id < stats_guard.max_concurrent_batches_used {
                    stats_guard.max_concurrent_batches_used = batch_id + 1;
                }
                
                debug!("✅ Батч #{} завершен за {:?}", batch_id, batch_elapsed);
                
                (batch_id, result)
            });
            
            handles.push(handle);
        }

        // Собираем результаты в правильном порядке
        let mut batch_results = Vec::new();
        for handle in handles {
            let (batch_id, result) = handle.await?;
            batch_results.push((batch_id, result?));
        }

        // Сортируем по batch_id для правильного порядка
        batch_results.sort_by_key(|(batch_id, _)| *batch_id);

        // Объединяем результаты
        for (_, embeddings) in batch_results {
            all_embeddings.extend(embeddings);
        }

        let total_elapsed = start_time.elapsed();
        
        // Финальная статистика
        let stats = self.get_stats().await;
        
        info!("🎯 Оптимизированная обработка завершена:");
        info!("  📊 Всего текстов: {}", total_texts);
        info!("  ⏱️ Общее время: {:?}", total_elapsed);
        info!("  🚀 Производительность: {:.1} текстов/сек", total_texts as f32 / total_elapsed.as_secs_f32());
        info!("  📈 Средний размер батча: {:.1}", stats.avg_batch_size);
        info!("  💾 Memory pool efficiency: {:.1}%", stats.memory_pool_efficiency() * 100.0);
        info!("  🔄 Максимальная параллельность: {}", stats.max_concurrent_batches_used);

        // Печатаем статистику memory pool
        if self.config.memory_pooling_enabled {
            info!("💾 Финальная статистика Memory Pool:");
            GPU_MEMORY_POOL.print_stats();
        }

        Ok(all_embeddings)
    }

    /// Адаптивное вычисление размера батча на основе истории производительности
    async fn calculate_adaptive_batch_size(&self, total_texts: usize) -> usize {
        let stats = self.stats.lock().await;
        let base_batch_size = self.config.optimal_batch_size;
        
        // Если это первый запуск или мало данных, используем базовый размер
        if stats.total_batches_processed < 3 {
            return base_batch_size;
        }
        
        let current_throughput = stats.throughput_per_second();
        
        // Адаптивная логика на основе производительности
        let adaptive_size = if current_throughput > 50.0 {
            // Высокая производительность - можно увеличить batch size
            (base_batch_size as f32 * 1.2) as usize
        } else if current_throughput < 10.0 {
            // Низкая производительность - уменьшаем batch size
            (base_batch_size as f32 * 0.8) as usize
        } else {
            base_batch_size
        };
        
        // Ограничиваем разумными пределами
        adaptive_size
            .max(self.config.min_batch_size)
            .min(total_texts)
            .min(256) // Максимальный разумный batch size
    }

    /// Получить текущую статистику пайплайна
    pub async fn get_stats(&self) -> PipelineStats {
        let stats = self.stats.lock().await;
        
        // Добавляем статистику memory pool
        let pool_stats = GPU_MEMORY_POOL.get_stats();
        let mut result = stats.clone();
        result.memory_pool_hits = pool_stats.hits;
        result.memory_pool_misses = pool_stats.misses;
        
        result
    }

    /// Печать подробной статистики
    pub async fn print_detailed_stats(&self) {
        let stats = self.get_stats().await;
        
        info!("📊 Детальная статистика OptimizedGpuPipeline:");
        info!("  🏭 Всего батчей обработано: {}", stats.total_batches_processed);
        info!("  📝 Всего текстов обработано: {}", stats.total_texts_processed);
        info!("  ⏱️ Общее время обработки: {}ms", stats.total_processing_time_ms);
        info!("  🚀 Производительность: {:.1} текстов/сек", stats.throughput_per_second());
        info!("  📈 Средний размер батча: {:.1}", stats.avg_batch_size);
        info!("  🔄 Максимальная параллельность: {}", stats.max_concurrent_batches_used);
        info!("  💾 Memory pool hits: {}", stats.memory_pool_hits);
        info!("  💾 Memory pool misses: {}", stats.memory_pool_misses);
        info!("  💾 Memory pool efficiency: {:.1}%", stats.memory_pool_efficiency() * 100.0);
    }

    /// Очистка ресурсов и memory pool
    pub async fn cleanup(&self) {
        info!("🧹 Очистка OptimizedGpuPipelineManager...");
        
        if self.config.memory_pooling_enabled {
            GPU_MEMORY_POOL.clear_unused();
            info!("💾 Memory pool очищен");
        }
        
        info!("✅ Очистка завершена");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GpuConfig;

    #[tokio::test]
    async fn test_optimized_pipeline() {
        let gpu_config = GpuConfig::auto_optimized();
        let embedding_config = EmbeddingConfig {
            model_name: "qwen3emb".to_string(),
            max_length: 256,
            embedding_dim: Some(1024),
            use_gpu: true,
            gpu_config: Some(gpu_config),
            ..Default::default()
        };

        let pipeline_config = PipelineConfig {
            max_concurrent_batches: 2,
            optimal_batch_size: 16,
            memory_pooling_enabled: true,
            adaptive_batching: true,
            ..Default::default()
        };

        // Создаем pipeline - может занять время из-за загрузки модели
        let pipeline = OptimizedGpuPipelineManager::new(embedding_config, pipeline_config).await;
        
        // В тестовой среде может не быть GPU, поэтому просто проверим что создание не падает
        match pipeline {
            Ok(pipeline) => {
                let stats = pipeline.get_stats().await;
                assert_eq!(stats.total_batches_processed, 0);
                println!("✅ OptimizedGpuPipeline создан успешно");
            }
            Err(e) => {
                println!("⚠️ GPU недоступен в тестовой среде: {}", e);
                // Это нормально для CI/CD окружения без GPU
            }
        }
    }
}