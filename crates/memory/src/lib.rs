mod batch_manager;
mod cache;
mod cache_lru;
mod cache_interface;
mod cache_migration;
pub mod fallback;
mod hnsw_index; // Модульная HNSW архитектура
pub mod health;
mod metrics;
mod notifications;
pub mod promotion;
mod ml_promotion;
mod service;
pub mod service_di; // New DI-based service
pub mod storage;
mod types;
mod vector_index_hnswlib; // Critical for vector storage
mod transaction;
mod backup;
pub mod migration;
pub mod api;
mod streaming;
mod flush_config;
pub mod gpu_accelerated;
pub mod resource_manager;
mod retry;
mod database_manager;
// Dependency Injection система
mod di_container;
pub mod di_memory_config;
pub use di_container::{DIContainer, DIPerformanceMetrics, DIContainerStats};
// Новая orchestration система
pub mod orchestration;
pub use batch_manager::{BatchOperationManager, BatchConfig, BatchOperationBuilder, BatchStats};
pub use cache::{EmbeddingCache, CacheConfig as SimpleCacheConfig};
pub use cache_lru::{EmbeddingCacheLRU, CacheConfig};

// Cache configuration type for service
#[derive(Debug, Clone)]
pub enum CacheConfigType {
    Simple,
    Lru(CacheConfig),
}
pub use storage::VectorStore;
pub use types::{Layer, PromotionConfig, Record, SearchOptions};
pub use service::{MemoryService, MemoryServiceConfig as MemoryConfig, default_config, BatchInsertResult, BatchSearchResult};
pub use service_di::{DIMemoryService};
pub use di_memory_config::{MemoryDIConfigurator};
pub use health::{HealthMonitor, HealthMonitorConfig as HealthConfig, ComponentType, AlertSeverity, SystemHealthStatus};
pub use api::{UnifiedMemoryAPI, MemoryContext, SearchOptions as ApiSearchOptions, MemoryResult, OptimizationResult, SystemHealth, DetailedHealth, SystemStats, CacheStats, IndexSizes};
pub use metrics::{MetricsCollector, MemoryMetrics, LatencyMetrics, LayerMetrics};
pub use transaction::{Transaction, TransactionManager, TransactionGuard};

/// Быстрое создание DI Memory Service с конфигурацией по умолчанию
pub async fn create_di_memory_service() -> anyhow::Result<DIMemoryService> {
    let config = default_config()?;
    DIMemoryService::new(config).await
}

// Профессиональная HNSW реализация - единственная векторная реализация
pub use vector_index_hnswlib::{VectorIndexHnswRs, HnswRsConfig, HnswRsStats};

// ML-based promotion system
pub use ml_promotion::{MLPromotionEngine, MLPromotionConfig, MLPromotionStats, PromotionFeatures, PromotionDecision, UsageTracker};

// Streaming API system
pub use streaming::{StreamingMemoryAPI, StreamingConfig, StreamingRequest, StreamingResponse, StreamingOperation, StreamingResult, SessionConfig, StreamingPriority, GlobalStreamingStats, StreamingInsertRecord, SessionAction};

// Re-export for backward compatibility
pub use types::Layer as MemoryLayer;

// Deprecated types removed in v0.3.0
// Use Layer enum and Record struct instead