mod batch_manager;
mod cache;
mod cache_lru;
pub mod fallback;
pub mod health;
mod metrics;
mod promotion;
mod service;
mod storage;
mod types;
// Профессиональная HNSW реализация - единственная векторная реализация
mod vector_index_hnswlib;
pub mod migration;
pub mod api;

// Основные компоненты памяти
pub use cache::EmbeddingCache;
pub use cache_lru::{EmbeddingCacheLRU, CacheConfig};
pub use fallback::{FallbackEmbeddingService, GracefulEmbeddingService, EmbeddingProvider, GracefulServiceStatus};
pub use health::{HealthMonitor, HealthConfig, ComponentType, AlertSeverity, SystemHealthStatus, HealthMetric, HealthAlert, ComponentPerformanceStats};
pub use metrics::{MetricsCollector, MemoryMetrics, LayerMetrics};
pub use promotion::{PromotionEngine, PromotionStats, PromotionPerformanceStats};
pub use service::{MemoryConfig, MemoryService, SearchBuilder};
pub use batch_manager::{BatchOperationManager, BatchConfig, BatchOperationBuilder, BatchStats};
pub use storage::VectorStore;
pub use types::{Layer, PromotionConfig, Record, SearchOptions};
pub use api::{UnifiedMemoryAPI, MemoryContext, SearchOptions as ApiSearchOptions, MemoryResult, OptimizationResult, SystemHealth, DetailedHealth, SystemStats};

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