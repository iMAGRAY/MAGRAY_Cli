//! Index Layer Implementation - HNSW векторное индексирование
//!
//! HNSWIndexLayer инкапсулирует все операции с HNSW индексами
//! для быстрого векторного поиска с O(log n) сложностью.
//!
//! RESPONSIBILITIES:
//! - HNSW index management per layer
//! - Vector similarity search 
//! - Index building and optimization
//! - SIMD optimized distance calculations
//! - Memory-efficient index storage

use anyhow::{Result, Context};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::{debug, info, warn, error};
use chrono::{DateTime, Utc};

use crate::{
    types::Layer,
    layers::{IndexLayer, VectorSearchResult, IndexStats, LayerHealth, LayerHealthStatus, StorageLayer, IndexConfig},
    hnsw_index::{VectorIndex, HnswConfig},
    simd_optimized::optimized_cosine_distance_batch,
};

/// HNSW implementation для Index Layer
/// 
/// Фокусируется ТОЛЬКО на векторном индексировании:
/// - Efficient HNSW index per layer
/// - SIMD optimized similarity search
/// - Memory management for large indexes
/// - Index rebuilding and optimization
pub struct HNSWIndexLayer {
    config: IndexConfig,
    indexes: Arc<RwLock<HashMap<Layer, VectorIndex>>>,
    storage_layer: Arc<dyn StorageLayer>,
    stats: Arc<RwLock<InternalIndexStats>>,
    simd_enabled: bool,
}

/// Внутренние статистики для отслеживания производительности индексов
#[derive(Debug, Default)]
struct InternalIndexStats {
    total_searches: u64,
    total_insertions: u64,
    total_deletions: u64,
    index_rebuilds: u64,
    last_build_times: HashMap<Layer, u64>, // в миллисекундах
    search_times: Vec<f32>, // последние 1000 поисков для rolling average
    simd_operations: u64,
}

impl HNSWIndexLayer {
    /// Создать новый HNSW index layer с интеграцией со storage
    pub async fn new(
        config: IndexConfig,
        storage_layer: Arc<dyn StorageLayer>,
    ) -> Result<Arc<Self>> {
        info!("🔍 Инициализация HNSW Index Layer с dimension={}", config.dimension);

        // Проверяем SIMD capabilities
        let simd_enabled = config.use_simd && Self::check_simd_support();
        if simd_enabled {
            info!("⚡ SIMD оптимизации включены для векторных операций");
        } else {
            warn!("⚠️ SIMD оптимизации недоступны, используется fallback");
        }

        let layer = Arc::new(Self {
            config: config.clone(),
            indexes: Arc::new(RwLock::new(HashMap::new())),
            storage_layer,
            stats: Arc::new(RwLock::new(InternalIndexStats::default())),
            simd_enabled,
        });

        // Инициализируем индексы для всех слоев
        for memory_layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            layer.create_index_for_layer(memory_layer).await?;
        }

        info!("✅ HNSW Index Layer успешно инициализирован");
        Ok(layer)
    }

    /// Создать HNSW индекс для конкретного слоя
    async fn create_index_for_layer(&self, layer: Layer) -> Result<()> {
        debug!("🔧 Создание HNSW индекса для слоя {:?}", layer);

        let hnsw_config = HnswConfig {
            dimension: self.config.dimension,
            max_connections: self.config.max_connections,
            ef_construction: self.config.ef_construction,
            ef_search: self.config.ef_search,
            use_simd: self.simd_enabled,
        };

        let index = VectorIndex::new(hnsw_config)
            .context("Не удалось создать HNSW индекс")?;

        {
            let mut indexes = self.indexes.write().await;
            indexes.insert(layer, index);
        }

        debug!("✅ HNSW индекс для слоя {:?} создан", layer);
        Ok(())
    }

    /// Проверить поддержку SIMD инструкций
    fn check_simd_support() -> bool {
        // В реальной реализации здесь должна быть проверка CPU features
        // Для простоты возвращаем true если компиляция с target-feature
        cfg!(target_feature = "avx2") || cfg!(target_feature = "sse2")
    }

    /// Получить индекс для слоя
    async fn get_index(&self, layer: Layer) -> Result<VectorIndex> {
        let indexes = self.indexes.read().await;
        indexes.get(&layer)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Индекс для слоя {:?} не найден", layer))
    }

    /// Increment internal stats (non-blocking)
    fn increment_stat(&self, stat_type: IndexStatType, value: u64) {
        let stats = Arc::clone(&self.stats);
        tokio::spawn(async move {
            if let Ok(mut stats) = stats.try_write() {
                match stat_type {
                    IndexStatType::Search => {
                        stats.total_searches += value;
                        stats.simd_operations += if value > 0 { 1 } else { 0 };
                    },
                    IndexStatType::Insert => stats.total_insertions += value,
                    IndexStatType::Delete => stats.total_deletions += value,
                    IndexStatType::Rebuild => stats.index_rebuilds += value,
                }
            }
        });
    }

    /// Record search time для rolling average
    fn record_search_time(&self, time_ms: f32) {
        let stats = Arc::clone(&self.stats);
        tokio::spawn(async move {
            if let Ok(mut stats) = stats.try_write() {
                stats.search_times.push(time_ms);
                if stats.search_times.len() > 1000 {
                    stats.search_times.remove(0); // Keep only last 1000
                }
            }
        });
    }

    /// Вычислить average search time
    async fn calculate_average_search_time(&self) -> f32 {
        let stats = self.stats.read().await;
        if stats.search_times.is_empty() {
            return 0.0;
        }
        stats.search_times.iter().sum::<f32>() / stats.search_times.len() as f32
    }
}

enum IndexStatType {
    Search,
    Insert, 
    Delete,
    Rebuild,
}

#[async_trait]
impl IndexLayer for HNSWIndexLayer {
    async fn build_index(&self, layer: Layer) -> Result<()> {
        let start_time = std::time::Instant::now();
        info!("🏗️ Построение HNSW индекса для слоя {:?}", layer);

        // Получаем все записи из storage
        let records = self.storage_layer.list(layer, None).await
            .context("Не удалось получить записи из storage для построения индекса")?;

        if records.is_empty() {
            debug!("📭 Нет записей для индексирования в слое {:?}", layer);
            return Ok(());
        }

        // Создаем новый индекс
        self.create_index_for_layer(layer).await?;
        let mut index = self.get_index(layer).await?;

        debug!("📊 Добавление {} векторов в индекс слоя {:?}", records.len(), layer);

        // Добавляем все векторы в индекс
        for record in &records {
            if let Err(e) = index.add_vector(record.id, &record.embedding) {
                warn!("⚠️ Не удалось добавить вектор {} в индекс: {}", record.id, e);
                // Продолжаем с остальными векторами
            }
        }

        // Обновляем индекс в хранилище
        {
            let mut indexes = self.indexes.write().await;
            indexes.insert(layer, index);
        }

        let build_time_ms = start_time.elapsed().as_millis() as u64;

        // Обновляем статистики
        {
            let mut stats = self.stats.write().await;
            stats.last_build_times.insert(layer, build_time_ms);
        }

        self.increment_stat(IndexStatType::Rebuild, 1);

        info!("✅ Индекс для слоя {:?} построен за {}мс ({} векторов)", 
              layer, build_time_ms, records.len());
        Ok(())
    }

    async fn search_vectors(
        &self,
        embedding: &[f32],
        layer: Layer,
        top_k: usize,
    ) -> Result<Vec<VectorSearchResult>> {
        let start_time = std::time::Instant::now();
        
        debug!("🔍 HNSW поиск в слое {:?}, top_k={}", layer, top_k);

        // Проверяем размерность embedding
        if embedding.len() != self.config.dimension {
            return Err(anyhow::anyhow!(
                "Неверная размерность embedding: {} (ожидалось {})",
                embedding.len(),
                self.config.dimension
            ));
        }

        let index = self.get_index(layer).await?;

        // Выполняем HNSW поиск
        let search_results = if self.simd_enabled {
            // Используем SIMD оптимизированный поиск
            debug!("⚡ Используем SIMD оптимизированный поиск");
            index.search_with_simd(embedding, top_k)
                .context("SIMD поиск не удался")?
        } else {
            // Fallback на обычный поиск
            debug!("🔄 Используем обычный поиск (без SIMD)");
            index.search(embedding, top_k)
                .context("Обычный поиск не удался")?
        };

        // Конвертируем результаты в наш формат
        let mut results = Vec::new();
        for (id, distance) in search_results {
            results.push(VectorSearchResult {
                id,
                distance,
                layer,
            });
        }

        let search_time_ms = start_time.elapsed().as_millis() as f32;
        self.record_search_time(search_time_ms);
        self.increment_stat(IndexStatType::Search, 1);

        debug!("⚡ HNSW поиск завершен за {:.2}мс, найдено {} результатов", 
               search_time_ms, results.len());

        Ok(results)
    }

    async fn add_to_index(&self, id: &Uuid, embedding: &[f32], layer: Layer) -> Result<()> {
        debug!("➕ Добавление вектора {} в индекс слоя {:?}", id, layer);

        // Проверяем размерность
        if embedding.len() != self.config.dimension {
            return Err(anyhow::anyhow!(
                "Неверная размерность embedding: {} (ожидалось {})",
                embedding.len(),
                self.config.dimension
            ));
        }

        let mut index = self.get_index(layer).await?;
        
        index.add_vector(*id, embedding)
            .context("Не удалось добавить вектор в HNSW индекс")?;

        // Обновляем индекс в хранилище
        {
            let mut indexes = self.indexes.write().await;
            indexes.insert(layer, index);
        }

        self.increment_stat(IndexStatType::Insert, 1);
        debug!("✅ Вектор {} добавлен в индекс", id);
        Ok(())
    }

    async fn update_in_index(&self, id: &Uuid, embedding: &[f32], layer: Layer) -> Result<()> {
        debug!("🔄 Обновление вектора {} в индексе слоя {:?}", id, layer);

        // Сначала удаляем старый вектор
        if let Err(e) = self.remove_from_index(id, layer).await {
            warn!("⚠️ Не удалось удалить старый вектор {}: {}", id, e);
            // Продолжаем с добавлением нового
        }

        // Добавляем новый вектор
        self.add_to_index(id, embedding, layer).await?;
        
        debug!("✅ Вектор {} обновлен в индексе", id);
        Ok(())
    }

    async fn remove_from_index(&self, id: &Uuid, layer: Layer) -> Result<()> {
        debug!("🗑️ Удаление вектора {} из индекса слоя {:?}", id, layer);

        let mut index = self.get_index(layer).await?;
        
        index.remove_vector(*id)
            .context("Не удалось удалить вектор из HNSW индекса")?;

        // Обновляем индекс в хранилище
        {
            let mut indexes = self.indexes.write().await;
            indexes.insert(layer, index);
        }

        self.increment_stat(IndexStatType::Delete, 1);
        debug!("✅ Вектор {} удален из индекса", id);
        Ok(())
    }

    async fn rebuild_index(&self, layer: Layer) -> Result<()> {
        info!("🔄 Пересоздание индекса для слоя {:?}", layer);

        // Удаляем старый индекс
        {
            let mut indexes = self.indexes.write().await;
            indexes.remove(&layer);
        }

        // Создаем новый индекс с данными из storage
        self.build_index(layer).await?;
        
        info!("✅ Индекс для слоя {:?} пересоздан", layer);
        Ok(())
    }

    async fn index_stats(&self, layer: Layer) -> Result<IndexStats> {
        let index = self.get_index(layer).await?;
        let stats = self.stats.read().await;
        
        let build_time_ms = stats.last_build_times.get(&layer).copied().unwrap_or(0);
        let average_search_time = self.calculate_average_search_time().await;
        
        Ok(IndexStats {
            total_vectors: index.total_vectors() as u64,
            index_size_mb: index.memory_usage_mb(),
            build_time_ms,
            average_search_time_ms: average_search_time,
            connectivity: index.average_connectivity(),
            ef_construction: self.config.ef_construction,
            ef_search: self.config.ef_search,
        })
    }

    async fn is_index_ready(&self, layer: Layer) -> Result<bool> {
        let indexes = self.indexes.read().await;
        match indexes.get(&layer) {
            Some(index) => Ok(index.total_vectors() > 0),
            None => Ok(false),
        }
    }
}

#[async_trait]
impl LayerHealth for HNSWIndexLayer {
    async fn health_check(&self) -> Result<LayerHealthStatus> {
        let start = std::time::Instant::now();
        
        let mut healthy = true;
        let mut details = HashMap::new();
        
        // Проверяем все индексы
        let indexes = self.indexes.read().await;
        for (layer, index) in indexes.iter() {
            let vector_count = index.total_vectors();
            details.insert(
                format!("index_{:?}_vectors", layer),
                vector_count.to_string(),
            );
            
            if vector_count == 0 {
                details.insert(
                    format!("index_{:?}_status", layer),
                    "empty".to_string(),
                );
            } else {
                details.insert(
                    format!("index_{:?}_status", layer),
                    "ready".to_string(),
                );
            }
        }

        // Тестируем простой поиск
        let test_embedding = vec![0.0; self.config.dimension];
        if let Err(e) = self.search_vectors(&test_embedding, Layer::Interact, 1).await {
            healthy = false;
            details.insert("test_search_error".to_string(), e.to_string());
        }

        let response_time_ms = start.elapsed().as_millis() as f32;
        let stats = self.stats.read().await;

        Ok(LayerHealthStatus {
            layer_name: "HNSWIndexLayer".to_string(),
            healthy,
            response_time_ms,
            error_rate: if healthy { 0.0 } else { 1.0 },
            last_check: Utc::now(),
            details,
        })
    }

    async fn ready_check(&self) -> Result<bool> {
        let indexes = self.indexes.read().await;
        
        // Проверяем что все основные индексы созданы
        for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
            if !indexes.contains_key(&layer) {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::storage::SqliteStorageLayer;
    use crate::layers::StorageConfig;
    use crate::types::{Record, RecordMetadata};
    use tempfile::tempdir;

    async fn create_test_components() -> Result<(Arc<HNSWIndexLayer>, Arc<SqliteStorageLayer>)> {
        let temp_dir = tempdir()?;
        let storage_config = StorageConfig {
            db_path: temp_dir.path().join("test.db"),
            backup_path: temp_dir.path().join("backups"),
            use_rocksdb: false,
            write_batch_size: 100,
        };

        let storage_layer = SqliteStorageLayer::new(storage_config).await?;
        
        let index_config = IndexConfig {
            dimension: 3,
            max_connections: 8,
            ef_construction: 40,
            ef_search: 20,
            use_simd: false, // Отключаем для тестов
        };

        let index_layer = HNSWIndexLayer::new(index_config, Arc::clone(&storage_layer)).await?;
        
        Ok((index_layer, storage_layer))
    }

    fn create_test_record(id: Uuid, embedding: Vec<f32>) -> Record {
        Record {
            id,
            layer: Layer::Interact,
            content: format!("Test content for {}", id),
            embedding,
            metadata: RecordMetadata::default(),
        }
    }

    #[tokio::test]
    async fn test_index_creation() -> Result<()> {
        let (index_layer, _) = create_test_components().await?;
        assert!(index_layer.ready_check().await?);
        Ok(())
    }

    #[tokio::test]
    async fn test_index_operations() -> Result<()> {
        let (index_layer, storage_layer) = create_test_components().await?;
        
        let record1 = create_test_record(Uuid::new_v4(), vec![0.1, 0.2, 0.3]);
        let record2 = create_test_record(Uuid::new_v4(), vec![0.4, 0.5, 0.6]);

        // Сохраняем в storage
        storage_layer.store(&record1).await?;
        storage_layer.store(&record2).await?;

        // Строим индекс
        index_layer.build_index(Layer::Interact).await?;

        // Проверяем что индекс готов
        assert!(index_layer.is_index_ready(Layer::Interact).await?);

        // Тестируем поиск
        let search_results = index_layer.search_vectors(
            &[0.15, 0.25, 0.35],
            Layer::Interact,
            2,
        ).await?;

        assert!(!search_results.is_empty());
        assert!(search_results.len() <= 2);

        // Проверяем что результаты содержат правильные ID
        let result_ids: Vec<Uuid> = search_results.iter().map(|r| r.id).collect();
        assert!(result_ids.contains(&record1.id) || result_ids.contains(&record2.id));

        Ok(())
    }

    #[tokio::test]
    async fn test_index_crud() -> Result<()> {
        let (index_layer, _) = create_test_components().await?;
        
        let id = Uuid::new_v4();
        let embedding = vec![0.1, 0.2, 0.3];

        // Test add
        index_layer.add_to_index(&id, &embedding, Layer::Interact).await?;

        // Test search
        let results = index_layer.search_vectors(&embedding, Layer::Interact, 1).await?;
        assert!(!results.is_empty());
        assert_eq!(results[0].id, id);

        // Test update
        let new_embedding = vec![0.2, 0.3, 0.4];
        index_layer.update_in_index(&id, &new_embedding, Layer::Interact).await?;

        // Test remove
        index_layer.remove_from_index(&id, Layer::Interact).await?;

        // Verify removal
        let results_after_remove = index_layer.search_vectors(&embedding, Layer::Interact, 1).await?;
        // Index может быть пустым или содержать другие векторы, но не наш удаленный
        for result in results_after_remove {
            assert_ne!(result.id, id);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_health_check() -> Result<()> {
        let (index_layer, _) = create_test_components().await?;
        
        let health = index_layer.health_check().await?;
        assert!(health.healthy);
        assert!(health.response_time_ms >= 0.0);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_index_stats() -> Result<()> {
        let (index_layer, storage_layer) = create_test_components().await?;
        
        // Добавляем несколько записей
        for i in 0..5 {
            let record = create_test_record(
                Uuid::new_v4(), 
                vec![i as f32 * 0.1, (i + 1) as f32 * 0.1, (i + 2) as f32 * 0.1]
            );
            storage_layer.store(&record).await?;
        }

        // Строим индекс
        index_layer.build_index(Layer::Interact).await?;

        let stats = index_layer.index_stats(Layer::Interact).await?;
        assert_eq!(stats.total_vectors, 5);
        assert!(stats.build_time_ms > 0);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_embedding_dimension_validation() -> Result<()> {
        let (index_layer, _) = create_test_components().await?;
        
        let id = Uuid::new_v4();
        let wrong_dimension_embedding = vec![0.1, 0.2]; // Должно быть 3

        let result = index_layer.add_to_index(&id, &wrong_dimension_embedding, Layer::Interact).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("размерность"));

        Ok(())
    }
}