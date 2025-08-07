use anyhow::Result;
use tracing::info;

#[cfg(feature = "gpu")]
use tracing::warn;

#[cfg(feature = "gpu")]
use ort::execution_providers::{
    CUDAExecutionProvider, ExecutionProviderDispatch, TensorRTExecutionProvider,
};

#[cfg(all(feature = "gpu", windows))]
use ort::execution_providers::DirectMLExecutionProvider;

#[cfg(feature = "gpu")]
use ort::execution_providers::OpenVINOExecutionProvider;

use crate::gpu_detector::{GpuDetector, GpuOptimalParams};

/// GPU конфигурация для ONNX Runtime
#[derive(Debug, Clone)]
pub struct GpuConfig {
    /// ID устройства CUDA (обычно 0)
    pub device_id: i32,
    /// Размер GPU памяти для арены (в байтах)
    pub gpu_mem_limit: usize,
    /// Использовать TensorRT для дополнительной оптимизации
    pub use_tensorrt: bool,
    /// Размер памяти для TensorRT кэша
    pub tensorrt_cache_size: usize,
    /// Включить FP16 вычисления для ускорения
    pub enable_fp16: bool,
    /// Автоматически оптимизировать параметры
    pub auto_optimize: bool,
    /// Предпочитаемый тип провайдера
    pub preferred_provider: GpuProviderType,
    /// Использовать DirectML на Windows
    pub use_directml: bool,
    /// Использовать OpenVINO для Intel GPU/CPU
    pub use_openvino: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GpuProviderType {
    CUDA,
    DirectML,
    OpenVINO,
    Auto,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            device_id: 0,
            gpu_mem_limit: 2 * 1024 * 1024 * 1024, // 2GB
            use_tensorrt: false,
            tensorrt_cache_size: 1024 * 1024 * 1024, // 1GB
            enable_fp16: true,
            auto_optimize: true,
            preferred_provider: GpuProviderType::Auto,
            use_directml: cfg!(windows),
            use_openvino: true,
        }
    }
}

impl GpuConfig {
    /// Создать конфигурацию с автоматической оптимизацией
    pub fn auto_optimized() -> Self {
        let mut config = Self::default();

        // Определяем GPU
        let detector = GpuDetector::detect();

        if let Some(best_device) = detector.select_best_device() {
            config.device_id = best_device as i32;

            // Получаем информацию о выбранном устройстве
            if let Some(device) = detector.devices.iter().find(|d| d.index == best_device) {
                // Используем 80% доступной памяти
                config.gpu_mem_limit = (device.free_memory_mb as usize * 1024 * 1024 * 8) / 10;

                // Включаем TensorRT для мощных GPU (8GB+)
                config.use_tensorrt = device.total_memory_mb >= 8000;

                // Включаем FP16 для всех современных GPU (ускорение в 2x без потери качества)
                config.enable_fp16 = true;

                info!("🎯 Автоматически настроена GPU конфигурация:");
                info!("  - Устройство: GPU {} ({})", device.index, device.name);
                info!(
                    "  - Память: {} MB из {} MB",
                    config.gpu_mem_limit / 1024 / 1024,
                    device.free_memory_mb
                );
                info!(
                    "  - TensorRT: {}",
                    if config.use_tensorrt {
                        "включен"
                    } else {
                        "выключен"
                    }
                );
                info!(
                    "  - FP16: {}",
                    if config.enable_fp16 {
                        "включен"
                    } else {
                        "выключен"
                    }
                );
            }
        }

        config
    }

    /// Создать execution providers для GPU с автоматическим выбором лучшего
    #[cfg(feature = "gpu")]
    pub fn create_providers(&self) -> Result<Vec<ExecutionProviderDispatch>> {
        let mut providers = Vec::new();

        // Определяем какие провайдеры попробовать
        let provider_attempts = match self.preferred_provider {
            GpuProviderType::CUDA => vec![GpuProviderType::CUDA],
            GpuProviderType::DirectML => vec![GpuProviderType::DirectML],
            GpuProviderType::OpenVINO => vec![GpuProviderType::OpenVINO],
            GpuProviderType::Auto => {
                // Порядок приоритета: CUDA -> DirectML (Windows) -> OpenVINO -> CPU fallback
                let mut attempts = vec![GpuProviderType::CUDA];
                if cfg!(windows) && self.use_directml {
                    attempts.push(GpuProviderType::DirectML);
                }
                if self.use_openvino {
                    attempts.push(GpuProviderType::OpenVINO);
                }
                attempts
            }
        };

        info!("🔍 Попытка создания GPU providers: {:?}", provider_attempts);

        // Пробуем создать TensorRT provider если включен (только для CUDA)
        if self.use_tensorrt && provider_attempts.contains(&GpuProviderType::CUDA) {
            match self.create_tensorrt_provider() {
                Ok(provider) => {
                    info!(
                        "✅ TensorRT provider инициализирован для GPU {}",
                        self.device_id
                    );
                    providers.push(provider);
                }
                Err(e) => {
                    warn!("⚠️ TensorRT provider неудачен: {}", e);
                }
            }
        }

        // Пробуем создать основные GPU providers
        for provider_type in provider_attempts {
            match provider_type {
                GpuProviderType::CUDA => {
                    match self.create_cuda_provider() {
                        Ok(provider) => {
                            info!(
                                "✅ CUDA provider инициализирован для GPU {}",
                                self.device_id
                            );
                            info!(
                                "  📊 GPU memory limit: {} MB",
                                self.gpu_mem_limit / 1024 / 1024
                            );
                            providers.push(provider);
                            break; // Успешно создали, прекращаем попытки
                        }
                        Err(e) => {
                            warn!("⚠️ CUDA provider failed: {}. Trying next...", e);
                        }
                    }
                }
                GpuProviderType::DirectML => {
                    #[cfg(windows)]
                    match self.create_directml_provider() {
                        Ok(provider) => {
                            info!("✅ DirectML provider инициализирован");
                            providers.push(provider);
                            break;
                        }
                        Err(e) => {
                            warn!("⚠️ DirectML provider failed: {}. Trying next...", e);
                        }
                    }

                    #[cfg(not(windows))]
                    warn!("⚠️ DirectML доступен только на Windows");
                }
                GpuProviderType::OpenVINO => match self.create_openvino_provider() {
                    Ok(provider) => {
                        info!("✅ OpenVINO provider инициализирован");
                        providers.push(provider);
                        break;
                    }
                    Err(e) => {
                        warn!("⚠️ OpenVINO provider failed: {}. Trying next...", e);
                    }
                },
                GpuProviderType::Auto => unreachable!("Auto should be resolved earlier"),
            }
        }

        if providers.is_empty() {
            warn!("⚠️ Не удалось создать ни одного GPU provider. Fallback на CPU.");
        }

        Ok(providers)
    }

    /// Создать CUDA provider с обработкой ошибок
    #[cfg(feature = "gpu")]
    fn create_cuda_provider(&self) -> Result<ExecutionProviderDispatch> {
        let provider = CUDAExecutionProvider::default()
            .with_device_id(self.device_id)
            .with_memory_limit(self.gpu_mem_limit)
            .build();

        Ok(provider)
    }

    /// Создать TensorRT provider с обработкой ошибок
    #[cfg(feature = "gpu")]
    fn create_tensorrt_provider(&self) -> Result<ExecutionProviderDispatch> {
        let provider = TensorRTExecutionProvider::default()
            .with_device_id(self.device_id)
            .with_max_workspace_size(self.tensorrt_cache_size)
            .with_fp16(self.enable_fp16)
            .with_engine_cache(true)
            .with_engine_cache_path("./tensorrt_cache")
            .with_timing_cache(true)
            .with_force_sequential_engine_build(false) // Параллельная сборка
            .build();

        Ok(provider)
    }

    /// Создать DirectML provider (Windows только)
    #[cfg(all(feature = "gpu", windows))]
    fn create_directml_provider(&self) -> Result<ExecutionProviderDispatch> {
        let provider = DirectMLExecutionProvider::default()
            .with_device_id(self.device_id)
            .build();
        Ok(provider)
    }

    /// DirectML provider stub для non-Windows
    #[cfg(all(feature = "gpu", not(windows)))]
    fn create_directml_provider(&self) -> Result<ExecutionProviderDispatch> {
        Err(anyhow::anyhow!("DirectML доступен только на Windows"))
    }

    /// Создать OpenVINO provider
    #[cfg(feature = "gpu")]
    fn create_openvino_provider(&self) -> Result<ExecutionProviderDispatch> {
        let provider = OpenVINOExecutionProvider::default()
            .with_device_type("GPU") // Используем GPU, fallback на CPU автоматический
            .with_cache_dir("./openvino_cache")
            .with_num_threads(num_cpus::get())
            .build();
        Ok(provider)
    }

    /// Создать execution providers для GPU (stub для non-GPU builds)
    #[cfg(not(feature = "gpu"))]
    pub fn create_providers(&self) -> Result<Vec<()>> {
        info!("ℹ️ GPU поддержка не включена. Используйте --features gpu при сборке");
        Ok(Vec::new())
    }

    /// Получить оптимальные параметры для текущего GPU
    pub fn get_optimal_params(&self, model_size_mb: u64) -> GpuOptimalParams {
        let detector = GpuDetector::detect();

        if let Some(device) = detector
            .devices
            .iter()
            .find(|d| d.index == self.device_id as u32)
        {
            GpuOptimalParams::calculate(device.free_memory_mb, model_size_mb)
        } else {
            // Параметры по умолчанию если GPU не найден
            GpuOptimalParams {
                batch_size: 32,
                max_sequence_length: 256,
                use_fp16: self.enable_fp16,
                memory_fraction: 0.8,
            }
        }
    }
}

/// Информация о доступных GPU (legacy compatibility)
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub available: bool,
    pub device_count: usize,
    pub device_name: String,
    pub total_memory: usize,
    pub cuda_version: String,
}

impl GpuInfo {
    /// Проверить доступность GPU
    pub fn detect() -> Self {
        let detector = GpuDetector::detect();

        Self {
            available: detector.available,
            device_count: detector.devices.len(),
            device_name: detector
                .devices
                .first()
                .map(|d| d.name.clone())
                .unwrap_or_else(|| "N/A".to_string()),
            total_memory: detector
                .devices
                .first()
                .map(|d| (d.total_memory_mb * 1024 * 1024) as usize)
                .unwrap_or(0),
            cuda_version: detector.cuda_version,
        }
    }

    /// Вывести информацию о GPU
    pub fn print_info(&self) {
        let detector = GpuDetector::detect();
        detector.print_detailed_info();
    }
}
