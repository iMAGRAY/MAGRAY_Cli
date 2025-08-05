use anyhow::Result;
use clap::{Args, Subcommand};
use ai::{
    gpu_detector::GpuDetector,
    gpu_memory_pool::GPU_MEMORY_POOL,
    tensorrt_cache::TENSORRT_CACHE,
    model_downloader::MODEL_DOWNLOADER,
    auto_device_selector::{AutoDeviceSelector, SmartEmbeddingFactory},
    EmbeddingConfig,
};
use tracing::{info, warn, error};

/// @component: {"k":"C","id":"gpu_commands","t":"GPU management CLI","m":{"cur":95,"tgt":100,"u":"%"}}
#[derive(Debug, Args)]
pub struct GpuCommand {
    #[command(subcommand)]
    command: GpuSubcommand,
}

#[derive(Debug, Subcommand)]
enum GpuSubcommand {
    /// Показать информацию о доступных GPU
    #[command(visible_alias = "i")]
    Info,
    
    /// Протестировать производительность GPU
    #[command(visible_alias = "b")]
    Benchmark {
        /// Размер тестового батча
        #[arg(short, long, default_value = "100")]
        batch_size: usize,
        
        /// Сравнить с CPU
        #[arg(short, long)]
        compare: bool,
    },
    
    /// Управление кэшем
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
    
    /// Управление памятью GPU
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    
    /// Оптимизировать модель для текущего GPU
    #[command(visible_alias = "o")]
    Optimize {
        /// Имя модели для оптимизации
        #[arg(default_value = "bge-m3")]
        model: String,
    },
}

#[derive(Debug, Subcommand)]
enum CacheAction {
    /// Показать статистику кэша
    Stats,
    
    /// Очистить кэш TensorRT
    Clear,
    
    /// Показать размер кэша
    Size,
}

#[derive(Debug, Subcommand)]
enum MemoryAction {
    /// Показать статистику памяти
    Stats,
    
    /// Очистить неиспользуемые буферы
    Clear,
}

impl GpuCommand {
    pub async fn execute(self) -> Result<()> {
        match self.command {
            GpuSubcommand::Info => self.show_info(),
            GpuSubcommand::Benchmark { batch_size, compare } => {
                self.run_benchmark(batch_size, compare).await
            }
            GpuSubcommand::Cache { ref action } => self.handle_cache(action).await,
            GpuSubcommand::Memory { ref action } => self.handle_memory(action),
            GpuSubcommand::Optimize { ref model } => self.optimize_model(model).await,
        }
    }
    
    /// Показать информацию о GPU
    fn show_info(&self) -> Result<()> {
        let detector = GpuDetector::detect();
        detector.print_detailed_info();
        
        if !detector.available {
            warn!("💡 Подсказка: для включения GPU поддержки:");
            warn!("  1. Установите NVIDIA драйверы и CUDA Toolkit");
            warn!("  2. Пересоберите с: cargo build --release --features gpu");
            warn!("  3. Убедитесь что nvidia-smi доступна в PATH");
        }
        
        Ok(())
    }
    
    /// Запустить бенчмарк
    async fn run_benchmark(&self, batch_size: usize, compare: bool) -> Result<()> {
        info!("🏃 Запуск бенчмарка GPU с batch_size={}", batch_size);
        
        let detector = GpuDetector::detect();
        if !detector.available {
            error!("❌ GPU не обнаружен! Используйте 'magray gpu info' для диагностики.");
            return Ok(());
        }
        
        // Генерируем тестовые данные
        let test_texts: Vec<String> = (0..batch_size)
            .map(|i| format!("This is test text number {i} for benchmarking embedding performance on our optimized service with GPU acceleration."))
            .collect();
        
        // Конфигурация
        let config = EmbeddingConfig {
            model_name: "bge-m3".to_string(),
            use_gpu: true,
            batch_size,
            ..Default::default()
        };
        
        if compare {
            info!("\n📊 Сравнительный бенчмарк CPU vs GPU");
            
            // Автоматический выбор устройства
            let mut selector = AutoDeviceSelector::new();
            let decision = selector.select_device(&config).await?;
            decision.print_decision();
            
            info!("\n🏆 Рекомендация: использовать {}", 
                if decision.use_gpu { "GPU" } else { "CPU" }
            );
        } else {
            // Только GPU тест
            use ai::embeddings_gpu::GpuEmbeddingService;
            use std::time::Instant;
            
            info!("⏳ Загрузка модели...");
            let service = GpuEmbeddingService::new(config).await?;
            
            // Прогрев
            info!("🔥 Прогрев GPU...");
            let warmup_batch = test_texts.iter().take(10).cloned().collect();
            let _ = service.embed_batch(warmup_batch).await?;
            
            // Бенчмарк
            info!("⚡ Запуск бенчмарка...");
            let start = Instant::now();
            let embeddings = service.embed_batch(test_texts.clone()).await?;
            let elapsed = start.elapsed();
            
            // Результаты
            info!("\n📈 Результаты бенчмарка GPU:");
            info!("  - Обработано текстов: {}", batch_size);
            info!("  - Время выполнения: {:.2} сек", elapsed.as_secs_f64());
            info!("  - Скорость: {:.1} текстов/сек", batch_size as f64 / elapsed.as_secs_f64());
            info!("  - Среднее время: {:.2} мс/текст", elapsed.as_millis() as f64 / batch_size as f64);
            info!("  - Размерность эмбеддингов: {}", embeddings[0].len());
            
            // Метрики
            service.print_metrics();
        }
        
        Ok(())
    }
    
    /// Управление кэшем
    async fn handle_cache(&self, action: &CacheAction) -> Result<()> {
        match action {
            CacheAction::Stats => {
                let stats = TENSORRT_CACHE.get_stats()?;
                stats.print();
            }
            CacheAction::Clear => {
                TENSORRT_CACHE.clear_cache()?;
                info!("✅ Кэш TensorRT очищен");
            }
            CacheAction::Size => {
                let stats = TENSORRT_CACHE.get_stats()?;
                info!("📦 Размер кэша TensorRT: {:.2} GB", 
                    stats.total_size as f64 / 1024.0 / 1024.0 / 1024.0
                );
            }
        }
        Ok(())
    }
    
    /// Управление памятью
    fn handle_memory(&self, action: &MemoryAction) -> Result<()> {
        match action {
            MemoryAction::Stats => {
                GPU_MEMORY_POOL.print_stats();
            }
            MemoryAction::Clear => {
                GPU_MEMORY_POOL.clear_unused();
                info!("✅ Неиспользуемые буферы GPU очищены");
            }
        }
        Ok(())
    }
    
    /// Оптимизировать модель
    async fn optimize_model(&self, model_name: &String) -> Result<()> {
        info!("🔧 Оптимизация модели {} для текущего GPU...", model_name);
        
        let detector = GpuDetector::detect();
        if !detector.available {
            error!("❌ GPU не обнаружен!");
            return Ok(());
        }
        
        // Загружаем модель если необходимо
        info!("📥 Проверка наличия модели...");
        let model_path = MODEL_DOWNLOADER.ensure_model(model_name).await?;
        info!("✅ Модель загружена: {:?}", model_path);
        
        // Создаём оптимизированный сервис
        let config = EmbeddingConfig {
            model_name: model_name.clone(),
            use_gpu: true,
            ..Default::default()
        };
        
        info!("🚀 Создание оптимизированного сервиса...");
        let (service, decision) = SmartEmbeddingFactory::create_optimized(config).await?;
        
        info!("✅ Модель оптимизирована!");
        info!("  - Устройство: {}", if decision.use_gpu { "GPU" } else { "CPU" });
        info!("  - Batch size: {}", decision.recommended_batch_size);
        
        // Тестовый запуск
        info!("\n🧪 Тестовый запуск...");
        let test_texts = vec!["Hello, world!".to_string()];
        let start = std::time::Instant::now();
        let _ = service.embed_batch(test_texts).await?;
        let elapsed = start.elapsed();
        
        info!("✅ Тест успешен! Время: {:.2} мс", elapsed.as_millis());
        
        Ok(())
    }
}

/// Расширение для красивого вывода решения
trait DecisionExt {
    fn print_decision(&self);
}

impl DecisionExt for ai::auto_device_selector::DeviceDecision {
    fn print_decision(&self) {
        info!("\n🤖 Результат автоматического выбора:");
        info!("  - Выбрано устройство: {}", if self.use_gpu { "GPU 🎮" } else { "CPU 💻" });
        info!("  - Причина: {}", self.reason);
        info!("  - CPU производительность: {:.1} items/sec", self.cpu_score);
        if let Some(gpu_score) = self.gpu_score {
            info!("  - GPU производительность: {:.1} items/sec", gpu_score);
            let speedup = gpu_score / self.cpu_score;
            info!("  - Ускорение GPU: {:.1}x {}", 
                speedup,
                match speedup {
                    x if x > 10.0 => "🚀🚀🚀",
                    x if x > 5.0 => "🚀🚀",
                    x if x > 2.0 => "🚀",
                    _ => "⚡",
                }
            );
        }
        info!("  - Рекомендуемый batch size: {}", self.recommended_batch_size);
    }
}