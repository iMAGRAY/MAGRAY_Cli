#[cfg(feature = "gpu")]
use ai::{
    gpu_detector::GpuDetector, gpu_memory_pool::GPU_MEMORY_POOL, tensorrt_cache::TENSORRT_CACHE,
};
use anyhow::Result;
use clap::{Args, Subcommand};
use tracing::{info, warn};

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
        #[arg(default_value = "qwen3emb")]
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
            GpuSubcommand::Benchmark {
                batch_size,
                compare,
            } => self.run_benchmark(batch_size, compare).await,
            GpuSubcommand::Cache { ref action } => self.handle_cache(action).await,
            GpuSubcommand::Memory { ref action } => self.handle_memory(action),
            GpuSubcommand::Optimize { ref model } => self.optimize_model(model).await,
        }
    }

    /// Показать информацию о GPU
    fn show_info(&self) -> Result<()> {
        #[cfg(feature = "gpu")]
        {
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

        #[cfg(not(feature = "gpu"))]
        {
            warn!("GPU функциональность недоступна. Соберите с --features gpu");
            Ok(())
        }
    }

    /// Запустить бенчмарк
    async fn run_benchmark(&self, _batch_size: usize, _compare: bool) -> Result<()> {
        info!("🏃 Запуск бенчмарка GPU с batch_size={}", _batch_size);

        #[cfg(feature = "gpu")]
        {
            let detector = GpuDetector::detect();
            if detector.available {
                // TODO: Реализовать полноценный бенчмарк после настройки GPU
                info!("🚧 Бенчмарк находится в разработке");
            } else {
                warn!("❌ GPU не обнаружен! Используйте 'magray gpu info' для диагностики.");
            }
        }

        #[cfg(not(feature = "gpu"))]
        {
            warn!("GPU функциональность недоступна. Соберите с --features gpu");
        }

        Ok(())
    }

    /// Управление кэшем
    async fn handle_cache(&self, _action: &CacheAction) -> Result<()> {
        #[cfg(feature = "gpu")]
        {
            match _action {
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
                    info!(
                        "📦 Размер кэша TensorRT: {:.2} GB",
                        stats.total_size as f64 / 1024.0 / 1024.0 / 1024.0
                    );
                }
            }
        }

        #[cfg(not(feature = "gpu"))]
        {
            warn!("GPU функциональность недоступна. Соберите с --features gpu");
        }

        Ok(())
    }

    /// Управление памятью
    fn handle_memory(&self, _action: &MemoryAction) -> Result<()> {
        #[cfg(feature = "gpu")]
        {
            match _action {
                MemoryAction::Stats => {
                    let _ = GPU_MEMORY_POOL.print_stats();
                }
                MemoryAction::Clear => {
                    let _ = GPU_MEMORY_POOL.clear_unused();
                    info!("✅ Неиспользуемые буферы GPU очищены");
                }
            }
        }

        #[cfg(not(feature = "gpu"))]
        {
            warn!("GPU функциональность недоступна. Соберите с --features gpu");
        }

        Ok(())
    }

    /// Оптимизировать модель
    async fn optimize_model(&self, _model_name: &str) -> Result<()> {
        #[cfg(feature = "gpu")]
        {
            info!("🔧 Оптимизация модели {} для текущего GPU...", _model_name);

            let detector = GpuDetector::detect();
            if !detector.available {
                warn!("❌ GPU не обнаружен!");
                return Ok(());
            }

            // Загружаем модель если необходимо
            info!("📥 Проверка наличия модели...");
            use ai::model_downloader::MODEL_DOWNLOADER;
            let model_path = MODEL_DOWNLOADER.ensure_model(_model_name).await?;
            info!("✅ Модель загружена: {:?}", model_path);

            // Создаём оптимизированный сервис
            use ai::EmbeddingConfig;
            let config = EmbeddingConfig {
                model_name: _model_name.to_string(),
                use_gpu: true,
                ..Default::default()
            };

            info!("🚀 Создание оптимизированного сервиса...");
            use ai::auto_device_selector::SmartEmbeddingFactory;
            let (service, decision) = SmartEmbeddingFactory::create_optimized(config).await?;

            info!("✅ Модель оптимизирована!");
            info!(
                "  - Устройство: {}",
                if decision.use_gpu { "GPU" } else { "CPU" }
            );
            info!("  - Batch size: {}", decision.recommended_batch_size);

            // Тестовый запуск
            info!("\n🧪 Тестовый запуск...");
            let test_texts = vec!["Hello, world!".to_string()];
            let start = std::time::Instant::now();
            let _ = service.embed_batch(test_texts).await?;
            let elapsed = start.elapsed();

            info!("✅ Тест успешен! Время: {:.2} мс", elapsed.as_millis());
        }

        #[cfg(not(feature = "gpu"))]
        {
            warn!("GPU функциональность недоступна. Соберите с --features gpu");
        }

        Ok(())
    }
}

/// Расширение для красивого вывода решения
#[allow(dead_code)]
trait DecisionExt {
    fn print_decision(&self);
}

impl DecisionExt for ai::auto_device_selector::DeviceDecision {
    fn print_decision(&self) {
        info!("\n🤖 Результат автоматического выбора:");
        info!(
            "  - Выбрано устройство: {}",
            if self.use_gpu { "GPU 🎮" } else { "CPU 💻" }
        );
        info!("  - Причина: {}", self.reason);
        info!(
            "  - CPU производительность: {:.1} items/sec",
            self.cpu_score
        );
        if let Some(gpu_score) = self.gpu_score {
            info!("  - GPU производительность: {:.1} items/sec", gpu_score);
            let speedup = gpu_score / self.cpu_score;
            info!(
                "  - Ускорение GPU: {:.1}x {}",
                speedup,
                match speedup {
                    x if x > 10.0 => "🚀🚀🚀",
                    x if x > 5.0 => "🚀🚀",
                    x if x > 2.0 => "🚀",
                    _ => "⚡",
                }
            );
        }
        info!(
            "  - Рекомендуемый batch size: {}",
            self.recommended_batch_size
        );
    }
}
