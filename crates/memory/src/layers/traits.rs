//! Trait definitions для слоевой архитектуры Memory системы
//!
//! Эти traits обеспечивают четкое разделение ответственности между слоями:
//! - StorageLayer: Персистентность данных
//! - IndexLayer: Векторное индексирование 
//! - QueryLayer: Высокоуровневый поиск
//! - CacheLayer: Кэширование embeddings

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use uuid::Uuid;
use crate::{
    types::{Record, Layer, SearchOptions},
    backup::BackupMetadata,
};

// === STORAGE LAYER TRAIT ===

/// Storage Layer - ответственность за персистентность данных
/// 
/// Инкапсулирует все операции с физическим хранилищем (SQLite/RocksDB)
/// без знания о векторных операциях или бизнес-логике
#[async_trait]
pub trait StorageLayer: Send + Sync {
    /// Сохранить запись в хранилище
    async fn store(&self, record: &Record) -> Result<()>;

    /// Сохранить несколько записей батчем для производительности
    async fn store_batch(&self, records: &[&Record]) -> Result<usize>;

    /// Обновить существующую запись
    async fn update(&self, record: &Record) -> Result<()>;

    /// Удалить запись по ID
    async fn delete(&self, id: &Uuid, layer: Layer) -> Result<()>;

    /// Получить запись по ID
    async fn get(&self, id: &Uuid, layer: Layer) -> Result<Option<Record>>;

    /// Получить все записи из слоя с ограничением
    async fn list(&self, layer: Layer, limit: Option<usize>) -> Result<Vec<Record>>;

    /// Получить записи по метаданным фильтрам
    async fn filter_by_metadata(&self, filters: &HashMap<String, String>, layer: Layer) -> Result<Vec<Record>>;

    /// Создать backup всех данных
    async fn backup(&self, path: &str) -> Result<BackupMetadata>;

    /// Восстановить из backup
    async fn restore(&self, path: &str) -> Result<BackupMetadata>;

    /// Инициализировать слой хранения
    async fn init_layer(&self, layer: Layer) -> Result<()>;

    /// Получить статистику хранилища
    async fn storage_stats(&self) -> Result<StorageStats>;

    /// Оптимизировать хранилище (компактификация, VACUUM)
    async fn optimize(&self) -> Result<()>;
}

// === INDEX LAYER TRAIT ===

/// Index Layer - ответственность за векторное индексирование
///
/// Управляет HNSW индексами для быстрого векторного поиска
/// без знания о бизнес-логике или кэшировании
#[async_trait]
pub trait IndexLayer: Send + Sync {
    /// Построить индекс для слоя
    async fn build_index(&self, layer: Layer) -> Result<()>;

    /// Поиск похожих векторов через HNSW
    async fn search_vectors(
        &self,
        embedding: &[f32],
        layer: Layer,
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>>;

    /// Добавить вектор в индекс
    async fn add_to_index(&self, id: &Uuid, embedding: &[f32], layer: Layer) -> Result<()>;

    /// Обновить вектор в индексе
    async fn update_in_index(&self, id: &Uuid, embedding: &[f32], layer: Layer) -> Result<()>;

    /// Удалить вектор из индекса
    async fn remove_from_index(&self, id: &Uuid, layer: Layer) -> Result<()>;

    /// Пересоздать индекс для оптимизации
    async fn rebuild_index(&self, layer: Layer) -> Result<()>;

    /// Получить статистику индекса
    async fn index_stats(&self, layer: Layer) -> Result<IndexStats>;

    /// Проверить готовность индекса
    async fn is_index_ready(&self, layer: Layer) -> Result<bool>;
}

// === QUERY LAYER TRAIT ===

/// Query Layer - ответственность за высокоуровневую бизнес-логику поиска
///
/// Координирует поиск между storage, index и cache слоями
/// Реализует сложные алгоритмы ранжирования и фильтрации
#[async_trait]
pub trait QueryLayer: Send + Sync {
    /// Семантический поиск с полной бизнес-логикой
    async fn semantic_search(
        &self,
        query: &str,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>>;

    /// Поиск с pre-computed embedding для оптимизации
    async fn search_by_embedding(
        &self,
        embedding: &[f32],
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>>;

    /// Гибридный поиск (текстовый + векторный)
    async fn hybrid_search(
        &self,
        query: &str,
        text_filters: &HashMap<String, String>,
        layer: Layer,
        options: SearchOptions,
    ) -> Result<Vec<Record>>;

    /// Ранжирование результатов по сложным критериям
    async fn rank_results(
        &self,
        results: &mut Vec<Record>,
        criteria: &RankingCriteria,
    ) -> Result<()>;

    /// Получить embedding для текста (с кэшированием)
    async fn get_embedding(&self, text: &str) -> Result<Vec<f32>>;

    /// Batch получение embeddings
    async fn get_embeddings_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// Получить статистику запросов
    async fn query_stats(&self) -> Result<QueryStats>;
}

// === CACHE LAYER TRAIT ===

/// Cache Layer - ответственность за кэширование embeddings
///
/// LRU кэширование и производственные оптимизации
/// без знания о storage или бизнес-логике
#[async_trait]
pub trait CacheLayer: Send + Sync {
    /// Получить embedding из кэша
    async fn get(&self, key: &str) -> Result<Option<Vec<f32>>>;

    /// Сохранить embedding в кэш
    async fn put(&self, key: &str, embedding: Vec<f32>) -> Result<()>;

    /// Batch сохранение для оптимизации
    async fn put_batch(&self, entries: &[(&str, &[f32])]) -> Result<()>;

    /// Удалить из кэша
    async fn evict(&self, key: &str) -> Result<()>;

    /// Предварительная загрузка в кэш
    async fn prefetch(&self, keys: &[&str]) -> Result<()>;

    /// Очистить весь кэш
    async fn clear(&self) -> Result<()>;

    /// Получить статистику кэша (hits, misses, size)
    fn stats(&self) -> (u64, u64, u64);

    /// Оптимизировать кэш (удаление старых записей)
    async fn optimize(&self) -> Result<()>;

    /// Warming кэша для production
    async fn warm_cache(&self, popular_keys: &[&str]) -> Result<()>;
}

// === SUPPORTING TYPES ===

/// Результат векторного поиска с расстояниями
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub id: Uuid,
    pub distance: f32,
    pub layer: Layer,
}

/// Статистика storage слоя
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    pub total_records: u64,
    pub records_per_layer: HashMap<Layer, u64>,
    pub total_size_bytes: u64,
    pub fragmentation_ratio: f32,
    pub last_optimized: Option<chrono::DateTime<chrono::Utc>>,
}

/// Статистика index слоя
#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    pub total_vectors: u64,
    pub index_size_mb: f32,
    pub build_time_ms: u64,
    pub average_search_time_ms: f32,
    pub connectivity: f32,
    pub ef_construction: usize,
    pub ef_search: usize,
}

/// Статистика query слоя
#[derive(Debug, Clone, Default)]
pub struct QueryStats {
    pub total_queries: u64,
    pub cache_hit_rate: f32,
    pub average_response_time_ms: f32,
    pub slow_queries: u64, // > 100ms
    pub embedding_generation_time_ms: f32,
    pub reranking_time_ms: f32,
}

/// Критерии ранжирования для сложных запросов
#[derive(Debug, Clone)]
pub struct RankingCriteria {
    pub semantic_weight: f32,
    pub recency_weight: f32,
    pub popularity_weight: f32,
    pub diversity_boost: bool,
    pub custom_weights: HashMap<String, f32>,
}

impl Default for RankingCriteria {
    fn default() -> Self {
        Self {
            semantic_weight: 0.7,
            recency_weight: 0.2,
            popularity_weight: 0.1,
            diversity_boost: false,
            custom_weights: HashMap::new(),
        }
    }
}

// === LAYER HEALTH TRAIT ===

/// Health checking для каждого слоя
#[async_trait]
pub trait LayerHealth: Send + Sync {
    /// Проверка здоровья слоя
    async fn health_check(&self) -> Result<LayerHealthStatus>;

    /// Готовность к обслуживанию запросов
    async fn ready_check(&self) -> Result<bool>;
}

/// Статус здоровья слоя
#[derive(Debug, Clone)]
pub struct LayerHealthStatus {
    pub layer_name: String,
    pub healthy: bool,
    pub response_time_ms: f32,
    pub error_rate: f32,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub details: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ranking_criteria_default() {
        let criteria = RankingCriteria::default();
        assert_eq!(criteria.semantic_weight, 0.7);
        assert_eq!(criteria.recency_weight, 0.2);
        assert_eq!(criteria.popularity_weight, 0.1);
        assert!(!criteria.diversity_boost);
    }

    #[test]
    fn test_storage_stats_default() {
        let stats = StorageStats::default();
        assert_eq!(stats.total_records, 0);
        assert!(stats.records_per_layer.is_empty());
    }

    #[test]
    fn test_vector_search_result_creation() {
        let result = VectorSearchResult {
            id: Uuid::new_v4(),
            distance: 0.5,
            layer: Layer::Interact,
        };
        assert!(result.distance >= 0.0);
        assert_eq!(result.layer, Layer::Interact);
    }
}