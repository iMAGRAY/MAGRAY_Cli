use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::{
    vector_index_hnswlib::{VectorIndexHnswRs, HnswRsConfig},
    types::Layer,
};

// @component: {"k":"C","id":"dynamic_dimension","t":"Dynamic dimension support для векторов","m":{"cur":0,"tgt":90,"u":"%"},"f":["dimension","dynamic","adaptation"]}

/// Менеджер динамических размерностей векторов
pub struct DynamicDimensionManager {
    /// Активные индексы по размерностям
    indices_by_dimension: Arc<RwLock<HashMap<usize, DimensionGroup>>>,
    /// Конфигурация управления размерностями
    config: DimensionConfig,
    /// Статистика использования размерностей
    stats: Arc<RwLock<DimensionStats>>,
}

/// Группа индексов одной размерности
#[allow(dead_code)]
struct DimensionGroup {
    pub dimension: usize,
    pub indices: HashMap<Layer, Arc<VectorIndexHnswRs>>,
    pub record_count: usize,
    pub created_at: std::time::Instant,
    pub last_used: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct DimensionConfig {
    /// Поддерживаемые размерности векторов
    pub supported_dimensions: Vec<usize>,
    /// Дефолтная размерность для новых векторов
    pub default_dimension: usize,
    /// Максимальное количество активных размерностей
    pub max_active_dimensions: usize,
    /// Автоматическое определение размерности
    pub auto_detect_dimension: bool,
    /// Конвертация векторов между размерностями
    pub enable_dimension_conversion: bool,
    /// Время жизни неиспользуемых индексов (минуты)
    pub unused_index_ttl_minutes: u64,
}

impl Default for DimensionConfig {
    fn default() -> Self {
        Self {
            supported_dimensions: vec![
                384,   // sentence-transformers/all-MiniLM-L6-v2
                512,   // OpenAI text-embedding-ada-002
                768,   // BERT, RoBERTa
                1024,  // Qwen3, BGE-M3 (текущий дефолт)
                1536,  // OpenAI text-embedding-3-small
                3072,  // OpenAI text-embedding-3-large
            ],
            default_dimension: 1024,
            max_active_dimensions: 3,
            auto_detect_dimension: true,
            enable_dimension_conversion: false, // Пока отключено
            unused_index_ttl_minutes: 60,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct DimensionStats {
    pub active_dimensions: HashMap<usize, DimensionUsageStats>,
    pub dimension_conversions: u64,
    pub auto_detections: u64,
    pub index_evictions: u64,
}

#[derive(Debug, Default, Clone)]
pub struct DimensionUsageStats {
    pub record_count: usize,
    pub search_count: u64,
    pub last_access: Option<std::time::Instant>,
    pub total_memory_mb: f64,
}

impl DynamicDimensionManager {
    pub fn new(config: DimensionConfig) -> Result<Self> {
        // Валидация конфигурации
        if config.supported_dimensions.is_empty() {
            return Err(anyhow!("At least one supported dimension must be specified"));
        }
        
        if !config.supported_dimensions.contains(&config.default_dimension) {
            return Err(anyhow!("Default dimension {} not in supported dimensions", config.default_dimension));
        }

        info!("🎯 DynamicDimensionManager initialized:");
        info!("  Supported dimensions: {:?}", config.supported_dimensions);
        info!("  Default dimension: {}", config.default_dimension);
        info!("  Max active: {}", config.max_active_dimensions);

        Ok(Self {
            indices_by_dimension: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(DimensionStats::default())),
        })
    }

    /// Получить или создать индекс для указанной размерности и слоя
    pub fn get_or_create_index(
        &self,
        dimension: usize,
        layer: Layer,
    ) -> Result<Arc<VectorIndexHnswRs>> {
        // Проверяем поддержку размерности
        if !self.is_dimension_supported(dimension) {
            if self.config.auto_detect_dimension {
                warn!("📏 Dimension {} not in supported list, auto-adding", dimension);
                // В реальной реализации здесь можно динамически добавлять
            } else {
                return Err(anyhow!("Dimension {} not supported", dimension));
            }
        }

        let mut indices = self.indices_by_dimension.write();
        
        // Создаём группу размерности если не существует
        if !indices.contains_key(&dimension) {
            self.create_dimension_group(&mut indices, dimension)?;
        }

        // Получаем группу
        let dimension_group = indices.get_mut(&dimension)
            .ok_or_else(|| anyhow!("Failed to create dimension group for {}", dimension))?;

        // Обновляем время последнего использования
        dimension_group.last_used = std::time::Instant::now();

        // Создаём индекс для слоя если не существует
        if let std::collections::hash_map::Entry::Vacant(e) = dimension_group.indices.entry(layer) {
            let index_config = HnswRsConfig {
                dimension,
                max_connections: 24,
                ef_construction: 400,
                ef_search: 100,
                max_elements: 1_000_000,
                max_layers: 16,
                use_parallel: true,
            };

            let index = Arc::new(VectorIndexHnswRs::new(index_config)?);
            e.insert(index.clone());

            info!("🔧 Created new index: dimension={}, layer={:?}", dimension, layer);
        }

        // Обновляем статистику
        self.update_dimension_stats(dimension);

        Ok(dimension_group.indices[&layer].clone())
    }

    /// Автоматическое определение размерности вектора
    pub fn detect_dimension(&self, vector: &[f32]) -> usize {
        let detected = vector.len();
        
        if self.config.auto_detect_dimension {
            let mut stats = self.stats.write();
            stats.auto_detections += 1;
        }

        // Если размерность поддерживается, возвращаем её
        if self.is_dimension_supported(detected) {
            debug!("📏 Auto-detected supported dimension: {}", detected);
            return detected;
        }

        // Иначе пытаемся найти ближайшую поддерживаемую
        let closest = self.find_closest_supported_dimension(detected);
        
        if closest != detected {
            warn!("📏 Vector dimension {} not supported, using closest: {}", detected, closest);
        }

        closest
    }

    /// Поиск ближайшей поддерживаемой размерности
    fn find_closest_supported_dimension(&self, target: usize) -> usize {
        self.config.supported_dimensions
            .iter()
            .min_by_key(|&&dim| ((dim as i32) - (target as i32)).abs())
            .copied()
            .unwrap_or(self.config.default_dimension)
    }

    /// Проверка поддержки размерности
    pub fn is_dimension_supported(&self, dimension: usize) -> bool {
        self.config.supported_dimensions.contains(&dimension)
    }

    /// Конвертация вектора к нужной размерности (если включено)
    pub fn convert_vector_dimension(&self, vector: Vec<f32>, target_dimension: usize) -> Result<Vec<f32>> {
        if !self.config.enable_dimension_conversion {
            return Err(anyhow!("Dimension conversion is disabled"));
        }

        let current_dimension = vector.len();
        
        if current_dimension == target_dimension {
            return Ok(vector);
        }

        // Регистрируем конвертацию
        {
            let mut stats = self.stats.write();
            stats.dimension_conversions += 1;
        }

        if current_dimension < target_dimension {
            // Расширяем вектор (padding нулями)
            let mut extended = vector;
            extended.resize(target_dimension, 0.0);
            
            info!("📏 Extended vector: {} -> {} dimensions", current_dimension, target_dimension);
            Ok(extended)
        } else {
            // Сжимаем вектор (truncation)
            let truncated = vector[..target_dimension].to_vec();
            
            warn!("📏 Truncated vector: {} -> {} dimensions", current_dimension, target_dimension);
            Ok(truncated)
        }
    }

    /// Очистка неиспользуемых индексов
    pub async fn cleanup_unused_indices(&self) -> Result<usize> {
        let ttl = std::time::Duration::from_secs(self.config.unused_index_ttl_minutes * 60);
        let mut indices = self.indices_by_dimension.write();
        let mut removed_count = 0;

        let now = std::time::Instant::now();
        
        // Находим группы для удаления
        let to_remove: Vec<usize> = indices
            .iter()
            .filter(|(_, group)| {
                group.record_count == 0 && now.duration_since(group.last_used) > ttl
            })
            .map(|(&dim, _)| dim)
            .collect();

        // Удаляем неиспользуемые группы
        for dimension in to_remove {
            indices.remove(&dimension);
            removed_count += 1;
            info!("🧹 Removed unused dimension group: {}", dimension);
        }

        // Обновляем статистику
        if removed_count > 0 {
            let mut stats = self.stats.write();
            stats.index_evictions += removed_count as u64;
        }

        Ok(removed_count)
    }

    /// Получение статистики по размерностям
    pub fn get_dimension_stats(&self) -> DimensionStats {
        let indices = self.indices_by_dimension.read();
        let mut stats = self.stats.write();

        // Обновляем статистику активных размерностей
        stats.active_dimensions.clear();
        
        for (&dimension, group) in indices.iter() {
            let total_records: usize = group.indices
                .values()
                .map(|index| index.len())
                .sum();

            let memory_estimate = (total_records * dimension * 4) as f64 / 1024.0 / 1024.0; // Приблизительно

            stats.active_dimensions.insert(dimension, DimensionUsageStats {
                record_count: total_records,
                search_count: 0, // Будет обновляться при поиске
                last_access: Some(group.last_used),
                total_memory_mb: memory_estimate,
            });
        }

        (*stats).clone()
    }

    /// Получение всех активных размерностей
    pub fn get_active_dimensions(&self) -> Vec<usize> {
        let indices = self.indices_by_dimension.read();
        indices.keys().copied().collect()
    }

    /// Получение информации о размерности
    pub fn get_dimension_info(&self, dimension: usize) -> Option<DimensionInfo> {
        let indices = self.indices_by_dimension.read();
        
        if let Some(group) = indices.get(&dimension) {
            let total_records: usize = group.indices
                .values()
                .map(|index| index.len())
                .sum();

            Some(DimensionInfo {
                dimension,
                layers: group.indices.keys().copied().collect(),
                total_records,
                created_at: group.created_at,
                last_used: group.last_used,
                memory_usage_mb: (total_records * dimension * 4) as f64 / 1024.0 / 1024.0,
            })
        } else {
            None
        }
    }

    /// Принудительная настройка размерности как поддерживаемой
    pub fn add_supported_dimension(&mut self, dimension: usize) -> Result<()> {
        if dimension == 0 {
            return Err(anyhow!("Invalid dimension: 0"));
        }

        if !self.config.supported_dimensions.contains(&dimension) {
            self.config.supported_dimensions.push(dimension);
            self.config.supported_dimensions.sort();
            
            info!("➕ Added supported dimension: {}", dimension);
        }

        Ok(())
    }

    /// Вспомогательные методы
    fn create_dimension_group(
        &self,
        indices: &mut HashMap<usize, DimensionGroup>,
        dimension: usize,
    ) -> Result<()> {
        // Проверяем лимит активных размерностей
        if indices.len() >= self.config.max_active_dimensions {
            // Находим наименее используемую группу для удаления
            if let Some(least_used_dim) = self.find_least_used_dimension(indices) {
                indices.remove(&least_used_dim);
                warn!("📉 Evicted dimension {} to make room for {}", least_used_dim, dimension);
            }
        }

        let group = DimensionGroup {
            dimension,
            indices: HashMap::new(),
            record_count: 0,
            created_at: std::time::Instant::now(),
            last_used: std::time::Instant::now(),
        };

        indices.insert(dimension, group);
        info!("🆕 Created new dimension group: {}", dimension);

        Ok(())
    }

    fn find_least_used_dimension(&self, indices: &HashMap<usize, DimensionGroup>) -> Option<usize> {
        indices
            .iter()
            .filter(|(_, group)| group.record_count == 0) // Только пустые группы
            .min_by_key(|(_, group)| group.last_used)
            .map(|(&dim, _)| dim)
    }

    fn update_dimension_stats(&self, dimension: usize) {
        let mut stats = self.stats.write();
        
        stats.active_dimensions.entry(dimension).or_default();
    }
}

#[derive(Debug)]
pub struct DimensionInfo {
    pub dimension: usize,
    pub layers: Vec<Layer>,
    pub total_records: usize,
    pub created_at: std::time::Instant,
    pub last_used: std::time::Instant,
    pub memory_usage_mb: f64,
}

/// Интеграционный wrapper для существующих компонентов
pub struct DimensionAwareVectorStore {
    dimension_manager: Arc<DynamicDimensionManager>,
}

impl DimensionAwareVectorStore {
    pub fn new(config: DimensionConfig) -> Result<Self> {
        let dimension_manager = Arc::new(DynamicDimensionManager::new(config)?);
        
        Ok(Self {
            dimension_manager,
        })
    }

    /// Добавление вектора с автоопределением размерности
    pub fn add_vector_adaptive(
        &self,
        id: String,
        vector: Vec<f32>,
        layer: Layer,
    ) -> Result<()> {
        let dimension = self.dimension_manager.detect_dimension(&vector);
        let target_dimension = if self.dimension_manager.is_dimension_supported(dimension) {
            dimension
        } else {
            self.dimension_manager.config.default_dimension
        };

        let final_vector = if dimension != target_dimension {
            self.dimension_manager.convert_vector_dimension(vector, target_dimension)?
        } else {
            vector
        };

        let index = self.dimension_manager.get_or_create_index(target_dimension, layer)?;
        index.add(id, final_vector)?;

        Ok(())
    }

    /// Поиск с автоматическим определением размерности запроса
    pub fn search_adaptive(
        &self,
        query: &[f32],
        layer: Layer,
        k: usize,
    ) -> Result<Vec<(String, f32)>> {
        let query_dimension = self.dimension_manager.detect_dimension(query);
        let target_dimension = if self.dimension_manager.is_dimension_supported(query_dimension) {
            query_dimension
        } else {
            self.dimension_manager.config.default_dimension
        };

        let final_query = if query_dimension != target_dimension {
            self.dimension_manager.convert_vector_dimension(query.to_vec(), target_dimension)?
        } else {
            query.to_vec()
        };

        let index = self.dimension_manager.get_or_create_index(target_dimension, layer)?;
        index.search(&final_query, k)
    }

    pub fn get_dimension_manager(&self) -> Arc<DynamicDimensionManager> {
        self.dimension_manager.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimension_detection() {
        let config = DimensionConfig::default();
        let manager = DynamicDimensionManager::new(config).unwrap();

        // Тест определения поддерживаемой размерности
        let vector_768 = vec![0.1; 768];
        assert_eq!(manager.detect_dimension(&vector_768), 768);

        // Тест неподдерживаемой размерности
        let vector_999 = vec![0.1; 999];
        let closest = manager.detect_dimension(&vector_999);
        assert!(manager.is_dimension_supported(closest));
    }

    #[test]
    fn test_dimension_conversion() {
        let mut config = DimensionConfig::default();
        config.enable_dimension_conversion = true;
        
        let manager = DynamicDimensionManager::new(config).unwrap();

        // Тест расширения
        let small_vector = vec![0.1, 0.2, 0.3];
        let extended = manager.convert_vector_dimension(small_vector, 5).unwrap();
        assert_eq!(extended.len(), 5);
        assert_eq!(extended[3], 0.0); // Padding нулями

        // Тест сжатия
        let large_vector = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let truncated = manager.convert_vector_dimension(large_vector, 3).unwrap();
        assert_eq!(truncated.len(), 3);
        assert_eq!(truncated[2], 0.3);
    }
}