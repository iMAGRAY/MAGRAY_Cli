use std::path::{Path, PathBuf};
use std::fs;
use std::time::SystemTime;
use anyhow::{Result, Context};
use tracing::{info, debug};
use serde::{Serialize, Deserialize};

/// @component: {"k":"C","id":"tensorrt_cache","t":"TensorRT model cache","m":{"cur":90,"tgt":100,"u":"%"}}
pub struct TensorRTCache {
    cache_dir: PathBuf,
    max_cache_size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheMetadata {
    model_name: String,
    model_hash: String,
    gpu_device: String,
    cuda_version: String,
    tensorrt_version: String,
    creation_time: SystemTime,
    last_access_time: SystemTime,
    access_count: u64,
    file_size: u64,
}

impl TensorRTCache {
    /// Создать новый кэш TensorRT
    pub fn new(cache_dir: impl AsRef<Path>) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        
        // Создаём директорию если не существует
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)?;
            info!("📁 Создана директория кэша TensorRT: {:?}", cache_dir);
        }
        
        Ok(Self {
            cache_dir,
            max_cache_size: 10 * 1024 * 1024 * 1024, // 10GB по умолчанию
        })
    }
    
    /// Получить путь к кэшированной модели
    pub fn get_cached_model(
        &self,
        model_name: &str,
        model_hash: &str,
        gpu_info: &crate::gpu_detector::GpuDevice,
    ) -> Result<Option<PathBuf>> {
        let cache_key = self.generate_cache_key(model_name, model_hash, gpu_info);
        let cache_path = self.cache_dir.join(&cache_key).with_extension("trt");
        let metadata_path = self.cache_dir.join(&cache_key).with_extension("json");
        
        if cache_path.exists() && metadata_path.exists() {
            // Обновляем метаданные
            if let Ok(mut metadata) = self.load_metadata(&metadata_path) {
                metadata.last_access_time = SystemTime::now();
                metadata.access_count += 1;
                let _ = self.save_metadata(&metadata_path, &metadata);
                
                info!("✅ Найдена кэшированная TensorRT модель: {}", cache_key);
                debug!("  - Количество обращений: {}", metadata.access_count);
                
                return Ok(Some(cache_path));
            }
        }
        
        Ok(None)
    }
    
    /// Сохранить оптимизированную модель в кэш
    pub fn save_model(
        &self,
        model_name: &str,
        model_hash: &str,
        gpu_info: &crate::gpu_detector::GpuDevice,
        model_data: &[u8],
    ) -> Result<PathBuf> {
        // Проверяем место в кэше
        self.ensure_cache_space(model_data.len() as u64)?;
        
        let cache_key = self.generate_cache_key(model_name, model_hash, gpu_info);
        let cache_path = self.cache_dir.join(&cache_key).with_extension("trt");
        let metadata_path = self.cache_dir.join(&cache_key).with_extension("json");
        
        // Сохраняем модель
        fs::write(&cache_path, model_data)
            .context("Ошибка при сохранении TensorRT модели")?;
        
        // Создаём метаданные
        let metadata = CacheMetadata {
            model_name: model_name.to_string(),
            model_hash: model_hash.to_string(),
            gpu_device: gpu_info.name.clone(),
            cuda_version: crate::gpu_detector::GpuDetector::detect().cuda_version,
            tensorrt_version: self.get_tensorrt_version(),
            creation_time: SystemTime::now(),
            last_access_time: SystemTime::now(),
            access_count: 1,
            file_size: model_data.len() as u64,
        };
        
        self.save_metadata(&metadata_path, &metadata)?;
        
        info!("💾 Сохранена TensorRT модель в кэш: {}", cache_key);
        info!("  - Размер: {:.1} MB", model_data.len() as f64 / 1024.0 / 1024.0);
        
        Ok(cache_path)
    }
    
    /// Генерировать уникальный ключ кэша
    fn generate_cache_key(
        &self,
        model_name: &str,
        model_hash: &str,
        gpu_info: &crate::gpu_detector::GpuDevice,
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        model_name.hash(&mut hasher);
        model_hash.hash(&mut hasher);
        gpu_info.name.hash(&mut hasher);
        gpu_info.compute_capability.hash(&mut hasher);
        
        format!("{}_{}_{:x}", 
            model_name.replace('/', "_"),
<<<<<<< HEAD
            gpu_info.name.replace([' ', '/'], "_"),
=======
            gpu_info.name.replace(' ', "_").replace('/', "_"),
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
            hasher.finish()
        )
    }
    
    /// Освободить место в кэше если необходимо
    fn ensure_cache_space(&self, required_size: u64) -> Result<()> {
        let current_size = self.get_cache_size()?;
        
        if current_size + required_size <= self.max_cache_size {
            return Ok(());
        }
        
        info!("🧹 Очистка кэша TensorRT для освобождения {} MB", 
            required_size as f64 / 1024.0 / 1024.0);
        
        // Собираем все файлы с метаданными
        let mut cache_entries = Vec::new();
        
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(metadata) = self.load_metadata(&path) {
                    let model_path = path.with_extension("trt");
                    if model_path.exists() {
                        cache_entries.push((path, model_path, metadata));
                    }
                }
            }
        }
        
        // Сортируем по времени последнего доступа (LRU)
        cache_entries.sort_by_key(|(_, _, metadata)| metadata.last_access_time);
        
        // Удаляем старые файлы пока не освободим место
        let mut freed_size = 0u64;
        for (metadata_path, model_path, metadata) in cache_entries {
            if freed_size >= required_size {
                break;
            }
            
            freed_size += metadata.file_size;
            
            fs::remove_file(&model_path)?;
            fs::remove_file(&metadata_path)?;
            
            info!("  - Удалён {}: {:.1} MB", 
                metadata.model_name, 
                metadata.file_size as f64 / 1024.0 / 1024.0
            );
        }
        
        Ok(())
    }
    
    /// Получить общий размер кэша
    fn get_cache_size(&self) -> Result<u64> {
        let mut total_size = 0u64;
        
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
            }
        }
        
        Ok(total_size)
    }
    
    /// Загрузить метаданные
    fn load_metadata(&self, path: &Path) -> Result<CacheMetadata> {
        let content = fs::read_to_string(path)?;
        let metadata = serde_json::from_str(&content)?;
        Ok(metadata)
    }
    
    /// Сохранить метаданные
    fn save_metadata(&self, path: &Path, metadata: &CacheMetadata) -> Result<()> {
        let content = serde_json::to_string_pretty(metadata)?;
        fs::write(path, content)?;
        Ok(())
    }
    
    /// Получить версию TensorRT
    fn get_tensorrt_version(&self) -> String {
        // В реальной реализации нужно получить версию из TensorRT API
        "8.6.1".to_string()
    }
    
    /// Очистить весь кэш
    pub fn clear_cache(&self) -> Result<()> {
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                fs::remove_file(path)?;
            }
        }
        
        info!("🧹 Кэш TensorRT полностью очищен");
        Ok(())
    }
    
    /// Получить статистику кэша
    pub fn get_stats(&self) -> Result<CacheStats> {
        let mut stats = CacheStats::default();
        
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(metadata) = self.load_metadata(&path) {
                    stats.total_models += 1;
                    stats.total_size += metadata.file_size;
                    stats.total_access_count += metadata.access_count;
                    
                    if metadata.last_access_time > SystemTime::now() - std::time::Duration::from_secs(86400) {
                        stats.active_models += 1;
                    }
                }
            }
        }
        
        Ok(stats)
    }
}

#[derive(Debug, Default)]
pub struct CacheStats {
    pub total_models: usize,
    pub active_models: usize,
    pub total_size: u64,
    pub total_access_count: u64,
}

impl CacheStats {
    pub fn print(&self) {
        info!("📊 Статистика кэша TensorRT:");
        info!("  - Всего моделей: {}", self.total_models);
        info!("  - Активных моделей (24ч): {}", self.active_models);
        info!("  - Общий размер: {:.1} GB", self.total_size as f64 / 1024.0 / 1024.0 / 1024.0);
        info!("  - Общее количество обращений: {}", self.total_access_count);
        
        if self.total_models > 0 {
            info!("  - Среднее количество обращений: {:.1}", 
                self.total_access_count as f64 / self.total_models as f64);
        }
    }
}

lazy_static::lazy_static! {
    /// Глобальный кэш TensorRT
    pub static ref TENSORRT_CACHE: TensorRTCache = {
        let cache_dir = std::env::var("TENSORRT_CACHE_DIR")
            .unwrap_or_else(|_| ".tensorrt_cache".to_string());
        
        TensorRTCache::new(cache_dir)
            .expect("Не удалось создать кэш TensorRT")
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[test]
    fn test_cache_key_generation() {
        let temp_dir = TempDir::new().unwrap();
        let cache = TensorRTCache::new(temp_dir.path()).unwrap();
        
        let gpu_info = crate::gpu_detector::GpuDevice {
            index: 0,
            name: "NVIDIA GeForce RTX 3090".to_string(),
            total_memory_mb: 24576,
            free_memory_mb: 20000,
            compute_capability: "8.6".to_string(),
            temperature_c: Some(45),
            utilization_percent: Some(10),
            power_draw_w: Some(250.0),
        };
        
        let key1 = cache.generate_cache_key("model1", "hash1", &gpu_info);
        let key2 = cache.generate_cache_key("model1", "hash2", &gpu_info);
        let key3 = cache.generate_cache_key("model2", "hash1", &gpu_info);
        
        assert_ne!(key1, key2);
        assert_ne!(key1, key3);
        assert_ne!(key2, key3);
    }
}