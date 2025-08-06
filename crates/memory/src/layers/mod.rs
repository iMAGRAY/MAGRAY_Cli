//! Слоевая архитектура для DIMemoryService - SOLID разделение ответственности
//!
//! Эта архитектура заменяет God Object DIMemoryService на четко разделенные слои:
//! - StorageLayer: Персистентность данных (SQLite/RocksDB)
//! - IndexLayer: HNSW векторное индексирование и поиск
//! - QueryLayer: Высокоуровневая бизнес-логика поиска
//! - CacheLayer: LRU кэширование и производственные оптимизации

pub mod traits;
pub mod storage;
pub mod index;
pub mod query;
pub mod cache;
pub mod orchestrator;

// Re-exports for convenience
pub use traits::*;
pub use orchestrator::LayeredDIContainer;

use anyhow::Result;
use std::sync::Arc;
use crate::types::{Record, Layer, SearchOptions};
use uuid::Uuid;
use std::collections::HashMap;

/// Builder для создания полной слоевой архитектуры
pub struct LayeredMemoryBuilder {
    storage_config: StorageConfig,
    index_config: IndexConfig,
    query_config: QueryConfig,
    cache_config: CacheConfig,
}

/// Конфигурация для storage layer
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub db_path: std::path::PathBuf,
    pub backup_path: std::path::PathBuf,
    pub use_rocksdb: bool,
    pub write_batch_size: usize,
}

/// Конфигурация для index layer  
#[derive(Debug, Clone)]
pub struct IndexConfig {
    pub dimension: usize,
    pub max_connections: usize,
    pub ef_construction: usize,
    pub ef_search: usize,
    pub use_simd: bool,
}

/// Конфигурация для query layer
#[derive(Debug, Clone)]
pub struct QueryConfig {
    pub default_top_k: usize,
    pub max_query_length: usize,
    pub enable_reranking: bool,
    pub similarity_threshold: f32,
}

/// Конфигурация для cache layer
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_size: usize,
    pub ttl_seconds: u64,
    pub enable_prefetch: bool,
    pub cache_path: Option<std::path::PathBuf>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            db_path: std::env::temp_dir().join("magray_storage.db"),
            backup_path: std::env::temp_dir().join("magray_backups"),
            use_rocksdb: false,
            write_batch_size: 1000,
        }
    }
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            dimension: 1024, // Qwen3 embedding dimension
            max_connections: 16,
            ef_construction: 200,
            ef_search: 100,
            use_simd: true,
        }
    }
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            default_top_k: 10,
            max_query_length: 8192,
            enable_reranking: true,
            similarity_threshold: 0.7,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 10_000,
            ttl_seconds: 3600,
            enable_prefetch: true,
            cache_path: Some(std::env::temp_dir().join("magray_cache")),
        }
    }
}

impl LayeredMemoryBuilder {
    pub fn new() -> Self {
        Self {
            storage_config: StorageConfig::default(),
            index_config: IndexConfig::default(),
            query_config: QueryConfig::default(),
            cache_config: CacheConfig::default(),
        }
    }

    pub fn with_storage_config(mut self, config: StorageConfig) -> Self {
        self.storage_config = config;
        self
    }

    pub fn with_index_config(mut self, config: IndexConfig) -> Self {
        self.index_config = config;
        self
    }

    pub fn with_query_config(mut self, config: QueryConfig) -> Self {
        self.query_config = config;
        self
    }

    pub fn with_cache_config(mut self, config: CacheConfig) -> Self {
        self.cache_config = config;
        self
    }

    /// Создать полную слоевую архитектуру с DI контейнером
    pub async fn build(self) -> Result<LayeredDIContainer> {
        tracing::info!("🏗️ Создание слоевой архитектуры Memory системы...");

        // Создаем слои в правильном порядке зависимостей
        let storage_layer = storage::SqliteStorageLayer::new(self.storage_config).await?;
        let cache_layer = cache::LRUCacheLayer::new(self.cache_config).await?;
        let index_layer = index::HNSWIndexLayer::new(self.index_config, Arc::clone(&storage_layer)).await?;
        let query_layer = query::SemanticQueryLayer::new(
            self.query_config,
            Arc::clone(&storage_layer),
            Arc::clone(&index_layer),
            Arc::clone(&cache_layer),
        ).await?;

        // Создаем orchestrator для координации слоев
        let orchestrator = orchestrator::LayerOrchestrator::new(
            Arc::clone(&storage_layer),
            Arc::clone(&index_layer),
            Arc::clone(&query_layer),
            Arc::clone(&cache_layer),
        ).await?;

        let container = LayeredDIContainer::new(
            storage_layer,
            index_layer,
            query_layer,
            cache_layer,
            orchestrator,
        );

        tracing::info!("✅ Слоевая архитектура создана успешно");
        Ok(container)
    }
}

impl Default for LayeredMemoryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_builder_default_construction() -> Result<()> {
        let builder = LayeredMemoryBuilder::default();
        
        // Проверяем что все конфигурации имеют разумные defaults
        assert_eq!(builder.storage_config.write_batch_size, 1000);
        assert_eq!(builder.index_config.dimension, 1024);
        assert_eq!(builder.query_config.default_top_k, 10);
        assert_eq!(builder.cache_config.max_size, 10_000);

        Ok(())
    }

    #[tokio::test]
    async fn test_builder_custom_config() -> Result<()> {
        let custom_storage = StorageConfig {
            write_batch_size: 500,
            ..StorageConfig::default()
        };

        let builder = LayeredMemoryBuilder::new()
            .with_storage_config(custom_storage.clone());

        assert_eq!(builder.storage_config.write_batch_size, 500);
        
        Ok(())
    }
}