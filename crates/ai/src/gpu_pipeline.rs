use anyhow::Result;
use std::sync::Arc;
<<<<<<< HEAD
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tracing::{info, debug, warn};
=======
use tokio::sync::{mpsc, Mutex, Semaphore};
use tracing::{info, debug, warn};
use std::time::{Duration, Instant};
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
use futures::stream::{FuturesUnordered, StreamExt};
use crate::embeddings_gpu::GpuEmbeddingService;

/// @component: {"k":"C","id":"gpu_pipeline_manager","t":"GPU pipeline for parallel batches","m":{"cur":95,"tgt":100,"u":"%"},"f":["gpu","pipeline","parallel"]}
pub struct GpuPipelineManager {
    /// GPU сервисы для параллельной обработки
    gpu_services: Vec<Arc<GpuEmbeddingService>>,
    /// Семафор для ограничения параллельных операций
    gpu_semaphore: Arc<Semaphore>,
<<<<<<< HEAD
=======
    /// Очередь батчей для обработки
    batch_queue: Arc<Mutex<Vec<PendingBatch>>>,
    /// Канал для результатов
    result_channel: mpsc::UnboundedSender<ProcessedBatch>,
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
    /// Статистика pipeline
    stats: Arc<Mutex<PipelineStats>>,
    /// Конфигурация pipeline
    config: PipelineConfig,
}

<<<<<<< HEAD
#[derive(Debug, Clone)]
=======
#[derive(Clone)]
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
pub struct PipelineConfig {
    /// Количество параллельных GPU потоков
    pub num_gpu_streams: usize,
    /// Максимальный размер батча
    pub max_batch_size: usize,
    /// Минимальный размер батча
    pub min_batch_size: usize,
    /// Таймаут на обработку батча
    pub batch_timeout: Duration,
    /// Использовать пинированную память для быстрых трансферов
    pub use_pinned_memory: bool,
    /// Префетчинг следующего батча
    pub enable_prefetch: bool,
    /// Количество батчей для префетчинга
    pub prefetch_count: usize,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            num_gpu_streams: 4,
            max_batch_size: 128,
            min_batch_size: 32,
            batch_timeout: Duration::from_secs(30),
            use_pinned_memory: true,
            enable_prefetch: true,
            prefetch_count: 2,
        }
    }
}

<<<<<<< HEAD
=======
struct PendingBatch {
    id: u64,
    texts: Vec<String>,
    priority: u8,
    created_at: Instant,
}
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c

pub struct ProcessedBatch {
    pub id: u64,
    pub embeddings: Vec<Vec<f32>>,
    pub processing_time: Duration,
    pub gpu_stream_id: usize,
}

#[derive(Debug, Default, Clone)]
pub struct PipelineStats {
    pub total_batches: u64,
    pub total_texts: u64,
    pub total_gpu_time_ms: u64,
    pub total_transfer_time_ms: u64,
    pub total_time_ms: u64,
    pub avg_batch_size: f32,
    pub avg_gpu_utilization: f32,
    pub pipeline_throughput: f32,
    pub active_streams: usize,
    pub gpu_utilization: Vec<f32>,
}

impl GpuPipelineManager {
    /// Создать новый pipeline manager с несколькими GPU потоками
    pub async fn new(
        config: PipelineConfig,
        embedding_config: crate::EmbeddingConfig,
    ) -> Result<Self> {
        info!("🚀 Инициализация GPU Pipeline Manager");
        info!("  - GPU потоков: {}", config.num_gpu_streams);
        info!("  - Max batch size: {}", config.max_batch_size);
        info!("  - Pinned memory: {}", config.use_pinned_memory);
        info!("  - Prefetch: {}", config.enable_prefetch);
        
        // Создаём несколько GPU сервисов для параллельной работы
        let mut gpu_services = Vec::new();
        for i in 0..config.num_gpu_streams {
            let mut service_config = embedding_config.clone();
            // Каждый сервис работает на своём CUDA stream
            if let Some(ref mut gpu_cfg) = service_config.gpu_config {
                // Распределяем память равномерно между потоками
                gpu_cfg.gpu_mem_limit /= config.num_gpu_streams;
            }
            
            match GpuEmbeddingService::new(service_config).await {
                Ok(service) => {
                    info!("✅ GPU поток {} инициализирован", i);
                    gpu_services.push(Arc::new(service));
                }
                Err(e) => {
                    warn!("⚠️ Не удалось создать GPU поток {}: {}", i, e);
                    if i == 0 {
                        return Err(anyhow::anyhow!("Failed to create any GPU service: {}", e));
                    }
                }
            }
        }
        
        if gpu_services.is_empty() {
            return Err(anyhow::anyhow!("No GPU services could be initialized"));
        }
        
        let actual_streams = gpu_services.len();
        info!("✅ Инициализировано {} GPU потоков", actual_streams);
        
<<<<<<< HEAD
        Ok(Self {
            gpu_services,
            gpu_semaphore: Arc::new(Semaphore::new(actual_streams)),
=======
        // Канал для результатов
        let (tx, _rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            gpu_services,
            gpu_semaphore: Arc::new(Semaphore::new(actual_streams)),
            batch_queue: Arc::new(Mutex::new(Vec::new())),
            result_channel: tx,
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
            stats: Arc::new(Mutex::new(PipelineStats {
                gpu_utilization: vec![0.0; actual_streams],
                ..Default::default()
            })),
            config,
        })
    }
    
    /// Обработать батчи параллельно на GPU
    pub async fn process_batches_parallel(
        &self,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>> {
        let start_time = Instant::now();
        let total_texts = texts.len();
        
        // Разбиваем на оптимальные батчи
        let batches = self.create_batches(texts);
        let num_batches = batches.len();
        
        info!("📊 Обработка {} текстов в {} батчах", total_texts, num_batches);
        
        // Создаём futures для параллельной обработки
        let mut futures = FuturesUnordered::new();
        
        for (batch_id, batch) in batches.into_iter().enumerate() {
            let gpu_service = self.select_gpu_service(batch_id).await;
            let semaphore = self.gpu_semaphore.clone();
            let stats = self.stats.clone();
            
            futures.push(async move {
                // Захватываем permit для GPU
                let _permit = semaphore.acquire().await.unwrap();
                let batch_start = Instant::now();
                
                debug!("🔄 Батч {} начал обработку на GPU потоке", batch_id);
                
                // Обрабатываем батч
                let result = gpu_service.embed_batch(batch.clone()).await;
                
                let batch_time = batch_start.elapsed();
                
                // Обновляем статистику
                let mut stats_guard = stats.lock().await;
                stats_guard.total_batches += 1;
                stats_guard.total_texts += batch.len() as u64;
                stats_guard.total_gpu_time_ms += batch_time.as_millis() as u64;
                drop(stats_guard);
                
                debug!("✅ Батч {} обработан за {:?}", batch_id, batch_time);
                
                (batch_id, result)
            });
        }
        
        // Собираем результаты в правильном порядке
        let mut results = vec![None; num_batches];
        
        while let Some((batch_id, batch_result)) = futures.next().await {
            match batch_result {
                Ok(embeddings) => {
                    results[batch_id] = Some(embeddings);
                }
                Err(e) => {
                    warn!("❌ Ошибка обработки батча {}: {}", batch_id, e);
                    return Err(e);
                }
            }
        }
        
        // Объединяем результаты
        let mut all_embeddings = Vec::with_capacity(total_texts);
<<<<<<< HEAD
        for embeddings in results.into_iter().flatten() {
            all_embeddings.extend(embeddings);
=======
        for batch_result in results {
            if let Some(embeddings) = batch_result {
                all_embeddings.extend(embeddings);
            }
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        }
        
        let total_time = start_time.elapsed();
        
        // Обновляем статистику pipeline
        let mut stats = self.stats.lock().await;
        stats.pipeline_throughput = (total_texts as f32 / total_time.as_secs_f32()) * 1000.0;
        stats.avg_gpu_utilization = (stats.total_gpu_time_ms as f32 / total_time.as_millis() as f32) 
            / self.gpu_services.len() as f32;
        
        info!("⚡ Pipeline обработал {} текстов за {:?}", total_texts, total_time);
        info!("  - Throughput: {:.1} texts/sec", stats.pipeline_throughput);
        info!("  - GPU utilization: {:.1}%", stats.avg_gpu_utilization * 100.0);
        
        Ok(all_embeddings)
    }
    
    /// Создать батчи оптимального размера
    fn create_batches(&self, texts: Vec<String>) -> Vec<Vec<String>> {
        let mut batches = Vec::new();
        
        for chunk in texts.chunks(self.config.max_batch_size) {
            batches.push(chunk.to_vec());
        }
        
        // Балансируем последний батч если он слишком маленький
        if batches.len() > 1 {
            let last_size = batches.last().map(|b| b.len()).unwrap_or(0);
            if last_size < self.config.max_batch_size / 4 {
                // Перераспределяем элементы между последними двумя батчами
                let mut last_batch = batches.pop().unwrap();
                let mut prev_batch = batches.pop().unwrap();
                
                let total = last_batch.len() + prev_batch.len();
                let new_size = total / 2;
                
                while prev_batch.len() > new_size {
                    if let Some(text) = prev_batch.pop() {
                        last_batch.insert(0, text);
                    }
                }
                
                batches.push(prev_batch);
                batches.push(last_batch);
            }
        }
        
        batches
    }
    
    /// Выбрать GPU сервис для обработки (round-robin)
    async fn select_gpu_service(&self, batch_id: usize) -> Arc<GpuEmbeddingService> {
        let service_id = batch_id % self.gpu_services.len();
        self.gpu_services[service_id].clone()
    }
    
    /// Обработать батчи с префетчингом для максимальной производительности
    pub async fn process_with_prefetch(
        &self,
        texts: Vec<String>,
    ) -> Result<Vec<Vec<f32>>> {
        let start = Instant::now();
        let total_texts = texts.len();
        debug!("🚀 GPU Pipeline processing {} texts with {} GPU services", 
            total_texts, self.gpu_services.len());
        
        if self.gpu_services.is_empty() {
            return Err(anyhow::anyhow!("No GPU services available"));
        }
        
        // Если текстов мало или только один GPU, используем простой путь
        if total_texts <= self.config.min_batch_size || self.gpu_services.len() == 1 {
            let gpu_service = self.gpu_services.first().unwrap();
            let embeddings = gpu_service.embed_batch(texts).await?;
            
            // Обновляем статистику
            let mut stats = self.stats.lock().await;
            stats.total_batches += 1;
            stats.total_texts += total_texts as u64;
            stats.total_time_ms += start.elapsed().as_millis() as u64;
            
            return Ok(embeddings);
        }
        
        // Параллельная обработка с multiple GPU services
<<<<<<< HEAD
        let chunk_size = total_texts.div_ceil(self.gpu_services.len());
=======
        let chunk_size = (total_texts + self.gpu_services.len() - 1) / self.gpu_services.len();
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        let chunk_size = chunk_size.max(self.config.min_batch_size);
        
        let mut handles = Vec::new();
        let mut chunk_start = 0;
        
        for (idx, gpu_service) in self.gpu_services.iter().enumerate() {
            if chunk_start >= total_texts {
                break;
            }
            
            let chunk_end = (chunk_start + chunk_size).min(total_texts);
            let chunk: Vec<String> = texts[chunk_start..chunk_end].to_vec();
            let chunk_len = chunk.len();
            
            if chunk.is_empty() {
                break;
            }
            
            let gpu_service = gpu_service.clone();
            let stats = self.stats.clone();
            let gpu_idx = idx;
            
            let handle = tokio::spawn(async move {
                let chunk_start = Instant::now();
                debug!("GPU[{}] processing {} texts", gpu_idx, chunk_len);
                
                let result = gpu_service.embed_batch(chunk).await;
                
                let duration = chunk_start.elapsed();
                debug!("GPU[{}] completed {} texts in {:?}", gpu_idx, chunk_len, duration);
                
                // Обновляем статистику
                if result.is_ok() {
                    let mut stats_guard = stats.lock().await;
                    if gpu_idx < stats_guard.gpu_utilization.len() {
                        stats_guard.gpu_utilization[gpu_idx] += duration.as_millis() as f32;
                    }
                }
                
                result
            });
            
            handles.push(handle);
            chunk_start = chunk_end;
        }
        
        // Собираем результаты
        let mut all_embeddings = Vec::with_capacity(total_texts);
        
        for handle in handles {
            match handle.await {
                Ok(Ok(embeddings)) => {
                    all_embeddings.extend(embeddings);
                }
                Ok(Err(e)) => {
                    return Err(anyhow::anyhow!("GPU processing failed: {}", e));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Task join failed: {}", e));
                }
            }
        }
        
        // Обновляем общую статистику
        let duration = start.elapsed();
        let mut stats = self.stats.lock().await;
        stats.total_batches += 1;
        stats.total_texts += total_texts as u64;
        stats.total_time_ms += duration.as_millis() as u64;
        stats.avg_batch_size = stats.total_texts as f32 / stats.total_batches as f32;
        
        info!("✅ GPU Pipeline обработал {} текстов за {:?} ({:.1} texts/sec)", 
            total_texts, duration, total_texts as f64 / duration.as_secs_f64());
        
        Ok(all_embeddings)
    }
    
    /// Получить статистику pipeline
    pub async fn get_stats(&self) -> PipelineStats {
        self.stats.lock().await.clone()
    }
    
    /// Оптимизировать параметры pipeline на основе текущей производительности
    pub async fn auto_tune(&mut self) {
        let stats = self.get_stats().await;
        
        // Если GPU утилизация низкая, увеличиваем размер батча
        if stats.avg_gpu_utilization < 0.7 && self.config.max_batch_size < 256 {
            self.config.max_batch_size = (self.config.max_batch_size * 3 / 2).min(256);
            info!("📈 Увеличен размер батча до {}", self.config.max_batch_size);
        }
        
        // Если GPU утилизация слишком высокая, уменьшаем батч
        if stats.avg_gpu_utilization > 0.95 && self.config.max_batch_size > 32 {
            self.config.max_batch_size = (self.config.max_batch_size * 2 / 3).max(32);
            info!("📉 Уменьшен размер батча до {}", self.config.max_batch_size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_batch_creation() {
        let config = PipelineConfig {
            max_batch_size: 10,
            ..Default::default()
        };
        
        let embedding_config = crate::EmbeddingConfig::default();
        
        // Тест может fail без GPU, это нормально
        match GpuPipelineManager::new(config.clone(), embedding_config).await {
            Ok(manager) => {
                let texts: Vec<String> = (0..25).map(|i| format!("Text {}", i)).collect();
                let batches = manager.create_batches(texts);
                
                assert_eq!(batches.len(), 3);
                assert_eq!(batches[0].len(), 10);
                assert_eq!(batches[1].len(), 8); // Сбалансировано
                assert_eq!(batches[2].len(), 7); // Сбалансировано
            }
            Err(e) => {
                println!("Expected error without GPU: {}", e);
            }
        }
    }
}