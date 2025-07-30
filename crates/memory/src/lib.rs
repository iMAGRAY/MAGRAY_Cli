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
pub mod migration;
pub mod api;
pub mod gpu_accelerated;

// Основные компоненты памяти
pub use cache::EmbeddingCache;
pub use cache_lru::{EmbeddingCacheLRU, CacheConfig, CacheStatsReport};
pub use cache_interface::EmbeddingCacheInterface;
pub use cache_migration::{migrate_cache_to_lru, recommend_cache_config};
pub use fallback::{FallbackEmbeddingService, GracefulEmbeddingService, EmbeddingProvider, GracefulServiceStatus};
pub use health::{HealthMonitor, HealthConfig, ComponentType, AlertSeverity, SystemHealthStatus, HealthMetric, HealthAlert, ComponentPerformanceStats};
pub use metrics::{MetricsCollector, MemoryMetrics, LayerMetrics};
pub use promotion::{PromotionEngine, PromotionStats, PromotionPerformanceStats};
pub use service::{MemoryConfig, MemoryService, SearchBuilder, CacheConfigType};
pub use batch_manager::{BatchOperationManager, BatchConfig, BatchOperationBuilder, BatchStats};
pub use storage::VectorStore;
pub use types::{Layer, PromotionConfig, Record, SearchOptions};
pub use api::{UnifiedMemoryAPI, MemoryContext, SearchOptions as ApiSearchOptions, MemoryResult, OptimizationResult, SystemHealth, DetailedHealth, SystemStats};
pub use gpu_accelerated::{GpuBatchProcessor, BatchProcessorConfig, BatchProcessorStats};

// Профессиональная HNSW реализация - единственная векторная реализация
pub use vector_index_hnswlib::{VectorIndexHnswRs, HnswRsConfig, HnswRsStats};

// Re-export for backward compatibility
pub use types::Layer as MemoryLayer;

// Legacy types for compatibility with todo crate
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemLayer {
    Ephemeral,
    Short,
    Medium,
    Long,
    Semantic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemRef {
    pub layer: MemLayer,
    pub key: String,
    pub created_at: DateTime<Utc>,
}