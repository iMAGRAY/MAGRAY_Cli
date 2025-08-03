use std::process::Command;
use std::str;
use serde::{Deserialize, Serialize};
use tracing::{info, debug};

/// @component: {"k":"C","id":"gpu_detector","t":"GPU detection and info","m":{"cur":95,"tgt":100,"u":"%"}}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDetector {
    pub available: bool,
    pub devices: Vec<GpuDevice>,
    pub cuda_version: String,
    pub driver_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuDevice {
    pub index: u32,
    pub name: String,
    pub compute_capability: String,
    pub total_memory_mb: u64,
    pub free_memory_mb: u64,
    pub temperature_c: Option<u32>,
    pub utilization_percent: Option<u32>,
    pub power_draw_w: Option<f32>,
}

impl GpuDetector {
    /// Определить доступные GPU через nvidia-smi
    pub fn detect() -> Self {
        // Проверяем наличие nvidia-smi
        let nvidia_smi_check = Command::new("nvidia-smi")
            .arg("--query")
            .output();
            
        match nvidia_smi_check {
            Ok(output) if output.status.success() => {
                info!("✅ nvidia-smi доступен, определяем GPU...");
                Self::detect_nvidia_gpus()
            }
            _ => {
                debug!("nvidia-smi не найден или не работает");
                Self::not_available()
            }
        }
    }
    
    /// Определить NVIDIA GPU через nvidia-smi
    fn detect_nvidia_gpus() -> Self {
        let mut detector = Self {
            available: false,
            devices: Vec::new(),
            cuda_version: String::from("N/A"),
            driver_version: String::from("N/A"),
        };
        
        // Получаем версию драйвера
        if let Ok(output) = Command::new("nvidia-smi")
            .args(&["--query-gpu=driver_version", "--format=csv,noheader,nounits"])
            .output()
        {
            if let Ok(driver) = str::from_utf8(&output.stdout) {
                detector.driver_version = driver.trim().to_string();
            }
        }
        
        // Получаем информацию о GPU
        let gpu_query = Command::new("nvidia-smi")
            .args(&[
                "--query-gpu=index,name,compute_cap,memory.total,memory.free,temperature.gpu,utilization.gpu,power.draw",
                "--format=csv,noheader,nounits"
            ])
            .output();
            
        if let Ok(output) = gpu_query {
            if output.status.success() {
                if let Ok(gpu_info) = str::from_utf8(&output.stdout) {
                    for line in gpu_info.lines() {
                        if let Some(device) = Self::parse_gpu_line(line) {
                            detector.devices.push(device);
                        }
                    }
                }
            }
        }
        
        // Получаем версию CUDA
        if let Ok(output) = Command::new("nvcc")
            .arg("--version")
            .output()
        {
            if let Ok(version_str) = str::from_utf8(&output.stdout) {
                if let Some(cuda_line) = version_str.lines()
                    .find(|line| line.contains("release"))
                {
                    if let Some(version) = cuda_line.split("release").nth(1) {
                        detector.cuda_version = version.split(',').next()
                            .unwrap_or("N/A")
                            .trim()
                            .to_string();
                    }
                }
            }
        }
        
        detector.available = !detector.devices.is_empty();
        
        if detector.available {
            info!("🎮 Найдено {} GPU устройств", detector.devices.len());
            info!("🔧 Драйвер: {}, CUDA: {}", detector.driver_version, detector.cuda_version);
        }
        
        detector
    }
    
    /// Парсить строку с информацией о GPU
    fn parse_gpu_line(line: &str) -> Option<GpuDevice> {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        
        if parts.len() >= 8 {
            Some(GpuDevice {
                index: parts[0].parse().unwrap_or(0),
                name: parts[1].to_string(),
                compute_capability: parts[2].to_string(),
                total_memory_mb: parts[3].parse().unwrap_or(0),
                free_memory_mb: parts[4].parse().unwrap_or(0),
                temperature_c: parts[5].parse().ok(),
                utilization_percent: parts[6].parse().ok(),
                power_draw_w: parts[7].parse().ok(),
            })
        } else {
            None
        }
    }
    
    /// Создать заглушку когда GPU недоступен
    fn not_available() -> Self {
        Self {
            available: false,
            devices: Vec::new(),
            cuda_version: String::from("N/A"),
            driver_version: String::from("N/A"),
        }
    }
    
    /// Выбрать лучший GPU по свободной памяти
    pub fn select_best_device(&self) -> Option<u32> {
        self.devices
            .iter()
            .max_by_key(|d| d.free_memory_mb)
            .map(|d| d.index)
    }
    
    /// Получить общую свободную память на всех GPU
    pub fn total_free_memory_mb(&self) -> u64 {
        self.devices.iter().map(|d| d.free_memory_mb).sum()
    }
    
    /// Проверить достаточно ли памяти для модели
    pub fn has_sufficient_memory(&self, required_mb: u64) -> bool {
        self.devices.iter().any(|d| d.free_memory_mb >= required_mb)
    }
    
    /// Вывести подробную информацию о GPU
    pub fn print_detailed_info(&self) {
        if !self.available {
            info!("❌ GPU не обнаружен или не доступен");
            return;
        }
        
        info!("🎮 GPU информация:");
        info!("  🔧 Драйвер: {}", self.driver_version);
        info!("  🔧 CUDA: {}", self.cuda_version);
        
        for device in &self.devices {
            info!("  📊 GPU {}: {}", device.index, device.name);
            info!("    - Compute capability: {}", device.compute_capability);
            info!("    - Память: {}/{} MB", device.free_memory_mb, device.total_memory_mb);
            
            if let Some(temp) = device.temperature_c {
                info!("    - Температура: {}°C", temp);
            }
            if let Some(util) = device.utilization_percent {
                info!("    - Загрузка: {}%", util);
            }
            if let Some(power) = device.power_draw_w {
                info!("    - Потребление: {:.1}W", power);
            }
        }
    }
}

/// Оптимальные параметры для GPU
#[derive(Debug, Clone)]
pub struct GpuOptimalParams {
    pub batch_size: usize,
    pub max_sequence_length: usize,
    pub use_fp16: bool,
    pub memory_fraction: f32,
}

impl GpuOptimalParams {
    /// Рассчитать оптимальные параметры на основе доступной памяти
    pub fn calculate(available_memory_mb: u64, model_size_mb: u64) -> Self {
        // Оставляем 20% памяти для системы
        let usable_memory_mb = (available_memory_mb as f64 * 0.8) as u64;
        
        // Размер одного элемента в батче (приблизительно)
        let item_size_mb = 10; // 10MB на один элемент (embeddings + промежуточные тензоры)
        
        // Максимальный размер батча
        let max_batch_size = ((usable_memory_mb - model_size_mb) / item_size_mb).max(1) as usize;
        
        // Оптимальный размер батча (степень двойки для лучшей производительности)
        let optimal_batch_size = (1..=10)
            .map(|i| 1 << i) // 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024
            .filter(|&size| size <= max_batch_size)
            .last()
            .unwrap_or(1);
            
        Self {
            batch_size: optimal_batch_size,
            max_sequence_length: if usable_memory_mb > 4000 { 512 } else { 256 },
            use_fp16: true, // Включаем FP16 для всех GPU (ускорение в 2x без потери качества)
            memory_fraction: 0.8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gpu_detection() {
        let detector = GpuDetector::detect();
        println!("GPU available: {}", detector.available);
        detector.print_detailed_info();
    }
    
    #[test]
    fn test_optimal_params() {
        let params = GpuOptimalParams::calculate(8000, 1500);
        assert!(params.batch_size > 0);
        assert!(params.batch_size <= 512);
        println!("Optimal batch size for 8GB GPU: {}", params.batch_size);
    }
}