mod batch_manager;
mod cache;
mod cache_lru;
pub mod fallback;
mod metrics;
mod promotion;
pub mod promotion_optimized;
mod service;
mod storage;
mod types;
// Профессиональная HNSW реализация - единственная векторная реализация
mod vector_index_hnswlib;
pub mod migration;

// Основные компоненты памяти
pub use cache::EmbeddingCache;
pub use cache_lru::{EmbeddingCacheLRU, CacheConfig};
pub use fallback::{FallbackEmbeddingService, GracefulEmbeddingService, EmbeddingProvider, GracefulServiceStatus};
pub use metrics::{MetricsCollector, MemoryMetrics, LayerMetrics};
pub use promotion::{PromotionEngine, PromotionStats};
pub use promotion_optimized::{OptimizedPromotionEngine, OptimizedPromotionStats, PromotionPerformanceStats};
pub use service::{MemoryConfig, MemoryService, SearchBuilder};
pub use batch_manager::{BatchOperationManager, BatchConfig, BatchOperationBuilder, BatchStats};
pub use storage::VectorStore;
pub use types::{Layer, PromotionConfig, Record, SearchOptions};

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