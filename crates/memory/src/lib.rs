mod batch_manager;
mod cache;
mod cache_lru;
mod cache_interface;
mod cache_migration;
pub mod fallback;
pub mod health;
mod metrics;
mod promotion;
mod service;
mod storage;
mod types;
mod vector_index_hnswlib; // Critical for vector storage
mod transaction;
mod backup;
mod incremental_backup;
mod optimized_rebuild;
mod dynamic_dimension;
pub mod migration;
pub mod api;
pub mod gpu_accelerated;
pub mod resource_manager;

// Основные компоненты памяти
pub use cache::EmbeddingCache;
pub use cache_lru::{EmbeddingCacheLRU, CacheConfig, CacheStatsReport};
pub use cache_interface::EmbeddingCacheInterface;
pub use cache_migration::{migrate_cache_to_lru, recommend_cache_config};
pub use fallback::{FallbackEmbeddingService, GracefulEmbeddingService, EmbeddingProvider, GracefulServiceStatus};
pub use health::{HealthMonitor, HealthConfig, ComponentType, AlertSeverity, SystemHealthStatus, HealthMetric, HealthAlert, ComponentPerformanceStats};
pub use metrics::{MetricsCollector, MemoryMetrics, LayerMetrics};
pub use promotion::{PromotionEngine, PromotionStats, PromotionPerformanceStats};
pub use service::{MemoryConfig, MemoryService, SearchBuilder, CacheConfigType, default_config};
pub use batch_manager::{BatchOperationManager, BatchConfig, BatchOperationBuilder, BatchStats};
pub use storage::VectorStore;
pub use types::{Layer, PromotionConfig, Record, SearchOptions};
pub use api::{UnifiedMemoryAPI, MemoryContext, SearchOptions as ApiSearchOptions, MemoryResult, OptimizationResult, SystemHealth, DetailedHealth, SystemStats, CacheStats, IndexSizes};
pub use gpu_accelerated::{GpuBatchProcessor, BatchProcessorConfig, BatchProcessorStats};
pub use backup::{BackupManager, BackupMetadata, BackupInfo};
pub use incremental_backup::{IncrementalBackupManager, IncrementalBackupMetadata, BackupType, DeltaInfo};
pub use optimized_rebuild::{OptimizedRebuildManager, RebuildConfig, RebuildStats, RebuildResult, RebuildMethod, RebuildProgress};
pub use dynamic_dimension::{DynamicDimensionManager, DimensionConfig, DimensionStats, DimensionInfo, DimensionAwareVectorStore};
pub use resource_manager::{ResourceManager, ResourceConfig, ResourceUsage, CurrentLimits, ScalingStats};

// Профессиональная HNSW реализация - единственная векторная реализация
pub use vector_index_hnswlib::{VectorIndexHnswRs, HnswRsConfig, HnswRsStats};

// Re-export for backward compatibility
pub use types::Layer as MemoryLayer;

// Legacy types for compatibility with todo crate
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(note = "Use Layer enum instead. MemLayer will be removed in v0.3.0")]
pub enum MemLayer {
    Ephemeral,
    Short,
    Medium,
    Long,
    Semantic,
}

impl MemLayer {
    /// Преобразует legacy MemLayer в современный Layer
    pub fn to_layer(&self) -> Layer {
        match self {
            MemLayer::Ephemeral => Layer::Interact,
            MemLayer::Short => Layer::Interact,
            MemLayer::Medium => Layer::Insights,
            MemLayer::Long => Layer::Insights,
            MemLayer::Semantic => Layer::Assets,
        }
    }
}

impl From<Layer> for MemLayer {
    fn from(layer: Layer) -> Self {
        match layer {
            Layer::Interact => MemLayer::Ephemeral,
            Layer::Insights => MemLayer::Medium,
            Layer::Assets => MemLayer::Semantic,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(note = "Use Record with modern Layer instead. MemRef will be removed in v0.3.0")]
pub struct MemRef {
    pub layer: MemLayer,
    pub key: String,
    pub created_at: DateTime<Utc>,
}

impl MemRef {
    /// Создает MemRef из современного Record
    pub fn from_record(record: &Record) -> Self {
        Self {
            layer: record.layer.into(),
            key: record.id.to_string(),
            created_at: record.ts,
        }
    }
}