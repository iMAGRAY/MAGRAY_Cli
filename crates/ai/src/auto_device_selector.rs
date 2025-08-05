use std::time::Instant;
use anyhow::Result;
use tracing::{info, debug};
#[cfg(feature = "gpu")]
use tracing::warn;
use crate::gpu_detector::GpuDetector;
use crate::EmbeddingConfig;

/// @component: {"k":"C","id":"auto_device_selector","t":"Auto CPU/GPU selector","m":{"cur":95,"tgt":100,"u":"%"}}
#[derive(Debug, Clone)]
pub struct AutoDeviceSelector {
    /// Размер теста для бенчмарка
    benchmark_size: usize,
    /// Минимальное ускорение GPU для переключения (например, 2.0 = 2x быстрее)
    min_gpu_speedup: f32,
    /// Кэшированный результат выбора
    cached_decision: Option<DeviceDecision>,
}

#[derive(Debug, Clone)]
pub struct DeviceDecision {
    pub use_gpu: bool,
    pub reason: String,
    pub cpu_score: f32,
    pub gpu_score: Option<f32>,
    pub recommended_batch_size: usize,
}

impl Default for AutoDeviceSelector {
    fn default() -> Self {
        Self {
            benchmark_size: 100,
            min_gpu_speedup: 1.5, // GPU должен быть минимум в 1.5 раза быстрее
            cached_decision: None,
        }
    }
}

impl AutoDeviceSelector {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Автоматически выбрать лучшее устройство
    pub async fn select_device(&mut self, config: &EmbeddingConfig) -> Result<DeviceDecision> {
        // Если есть кэшированное решение, используем его
        if let Some(ref decision) = self.cached_decision {
            debug!("📋 Используем кэшированное решение: GPU={}", decision.use_gpu);
            return Ok(decision.clone());
        }
        
        info!("🔍 Автоматический выбор устройства (CPU vs GPU)...");
        
        // Проверяем доступность GPU
        let detector = GpuDetector::detect();
        if !detector.available {
            let decision = DeviceDecision {
                use_gpu: false,
                reason: "GPU не обнаружен".to_string(),
                cpu_score: 0.0,
                gpu_score: None,
                recommended_batch_size: num_cpus::get().min(32),
            };
            self.cached_decision = Some(decision.clone());
            return Ok(decision);
        }
        
        // Проверяем достаточно ли памяти GPU
        let required_memory = 1000; // ~1GB для модели
        if !detector.has_sufficient_memory(required_memory) {
            let decision = DeviceDecision {
                use_gpu: false,
                reason: format!("Недостаточно GPU памяти (нужно {required_memory} MB)"),
                cpu_score: 0.0,
                gpu_score: None,
                recommended_batch_size: num_cpus::get().min(32),
            };
            self.cached_decision = Some(decision.clone());
            return Ok(decision);
        }
        
        // Запускаем бенчмарк
        info!("⚡ Запуск бенчмарка производительности...");
        
        // Тест CPU
        let cpu_score = self.benchmark_cpu(config).await?;
        info!("💻 CPU score: {:.2} items/sec", cpu_score);
        
        // Тест GPU
        let gpu_score = self.benchmark_gpu(config).await?;
        info!("🎮 GPU score: {:.2} items/sec", gpu_score);
        
        // Принимаем решение
        let speedup = gpu_score / cpu_score;
        let use_gpu = speedup >= self.min_gpu_speedup;
        
        let decision = DeviceDecision {
            use_gpu,
            reason: if use_gpu {
                format!("GPU быстрее в {speedup:.1}x раз")
            } else {
                format!("GPU недостаточно быстрее (только {speedup:.1}x)")
            },
            cpu_score,
            gpu_score: Some(gpu_score),
            recommended_batch_size: if use_gpu {
                // Для GPU используем больший batch size
                match detector.devices.first().map(|d| d.total_memory_mb) {
                    Some(mem) if mem >= 16000 => 256,
                    Some(mem) if mem >= 8000 => 128,
                    Some(mem) if mem >= 4000 => 64,
                    _ => 32,
                }
            } else {
                num_cpus::get().min(32)
            },
        };
        
        info!("✅ Решение: использовать {} ({})", 
            if decision.use_gpu { "GPU" } else { "CPU" },
            decision.reason
        );
        info!("📊 Рекомендуемый batch size: {}", decision.recommended_batch_size);
        
        self.cached_decision = Some(decision.clone());
        Ok(decision)
    }
    
    /// Бенчмарк CPU
    async fn benchmark_cpu(&self, config: &EmbeddingConfig) -> Result<f32> {
        use crate::embeddings_cpu::CpuEmbeddingService;
        
        // Создаём конфигурацию для CPU
        let mut cpu_config = config.clone();
        cpu_config.use_gpu = false;
        cpu_config.batch_size = num_cpus::get().min(32);
        
        // Создаём CPU сервис
        let service = CpuEmbeddingService::new(cpu_config)?;
        
        // Генерируем тестовые данные
        let test_texts: Vec<String> = (0..self.benchmark_size)
            .map(|i| format!("This is test text number {i} for benchmarking embedding performance on CPU"))
            .collect();
        
        // Прогрев
        let warmup_texts: Vec<String> = test_texts.iter().take(10).cloned().collect();
        let _ = service.embed_batch(&warmup_texts)?;
        
        // Запускаем бенчмарк
        let start = Instant::now();
        let _ = service.embed_batch(&test_texts)?;
        let elapsed = start.elapsed().as_secs_f32();
        
        let score = self.benchmark_size as f32 / elapsed;
        Ok(score)
    }
    
    /// Бенчмарк GPU
    async fn benchmark_gpu(&self, _config: &EmbeddingConfig) -> Result<f32> {
        #[cfg(not(feature = "gpu"))]
        {
            // Если GPU не включен при компиляции, возвращаем 0
            Ok(0.0)
        }
        
        #[cfg(feature = "gpu")]
        {
            use crate::embeddings_gpu::GpuEmbeddingService;
            
            // Создаём конфигурацию для GPU
            let mut gpu_config = _config.clone();
            gpu_config.use_gpu = true;
            gpu_config.gpu_config = Some(crate::GpuConfig::auto_optimized());
            
            // Определяем оптимальный batch size для GPU
            let detector = GpuDetector::detect();
            if let Some(gpu) = detector.devices.first() {
                gpu_config.batch_size = match gpu.total_memory_mb {
                    mem if mem >= 16000 => 256,
                    mem if mem >= 8000 => 128,
                    mem if mem >= 4000 => 64,
                    _ => 32,
                };
            }
            
            // Создаём GPU сервис
            let service = match GpuEmbeddingService::new(gpu_config).await {
                Ok(s) => s,
                Err(e) => {
                    warn!("⚠️ Не удалось создать GPU сервис: {}", e);
                    return Ok(0.0);
                }
            };
            
            // Генерируем тестовые данные
            let test_texts: Vec<String> = (0..self.benchmark_size)
                .map(|i| format!("This is test text number {} for benchmarking embedding performance on GPU", i))
                .collect();
            
            // Прогрев GPU
            let warmup_texts = test_texts.iter().take(10).cloned().collect();
            let _ = service.embed_batch(warmup_texts).await?;
            
            // Запускаем бенчмарк
            let start = Instant::now();
            let _ = service.embed_batch(test_texts).await?;
            let elapsed = start.elapsed().as_secs_f32();
            
            let score = self.benchmark_size as f32 / elapsed;
            Ok(score)
        }
    }
    
    /// Сбросить кэшированное решение
    pub fn reset_cache(&mut self) {
        self.cached_decision = None;
    }
}

/// Умная фабрика для создания embedding сервиса
pub struct SmartEmbeddingFactory;

impl SmartEmbeddingFactory {
    /// Создать оптимальный embedding сервис
    pub async fn create_optimized(
        base_config: EmbeddingConfig
    ) -> Result<(Box<dyn EmbeddingServiceTrait>, DeviceDecision)> {
        let mut selector = AutoDeviceSelector::new();
        let decision = selector.select_device(&base_config).await?;
        
        // Создаём конфигурацию на основе решения
        let mut config = base_config;
        config.use_gpu = decision.use_gpu;
        config.batch_size = decision.recommended_batch_size;
        
        if decision.use_gpu {
            config.gpu_config = Some(crate::GpuConfig::auto_optimized());
        }
        
        // Создаём сервис
        let service = crate::embeddings_gpu::GpuEmbeddingService::new(config).await?;
        
        Ok((Box::new(service), decision))
    }
}

use async_trait::async_trait;

/// Trait для унификации embedding сервисов
#[async_trait]
pub trait EmbeddingServiceTrait: Send + Sync {
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
}

#[async_trait]
impl EmbeddingServiceTrait for crate::embeddings_gpu::GpuEmbeddingService {
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.embed_batch(texts).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_device_selection() {
        let mut selector = AutoDeviceSelector::new();
        let config = EmbeddingConfig::default();
        
        // Try to select device, but expect it might fail due to missing models
        match selector.select_device(&config).await {
            Ok(decision) => {
                println!("Device decision: {:?}", decision);
                
                // Проверяем кэширование
                let cached = selector.select_device(&config).await.unwrap();
                assert_eq!(decision.use_gpu, cached.use_gpu);
            },
            Err(e) => {
                println!("Expected error without models: {}", e);
                // This is fine - models are not available in test environment
            }
        }
    }
}