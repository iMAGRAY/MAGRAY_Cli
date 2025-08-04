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
mod service_di; // New DI-based service
pub mod storage;
mod types;
mod vector_index_hnswlib; // Critical for vector storage
mod transaction;
mod backup;
mod incremental_backup;
mod optimized_rebuild;
mod dynamic_dimension;
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
mod di_memory_config;
// Новая orchestration система
pub mod orchestration;

// Основные компоненты памяти
pub use cache::EmbeddingCache;
pub use cache_lru::{EmbeddingCacheLRU, CacheConfig, CacheStatsReport};
pub use cache_interface::EmbeddingCacheInterface;
pub use cache_migration::{migrate_cache_to_lru, recommend_cache_config};
pub use fallback::{FallbackEmbeddingService, GracefulEmbeddingService, EmbeddingProvider, GracefulServiceStatus};
pub use health::{HealthMonitor, HealthConfig, ComponentType, AlertSeverity, SystemHealthStatus, HealthMetric, HealthAlert, ComponentPerformanceStats};
pub use notifications::{NotificationConfig, NotificationChannel, NotificationManager};
pub use metrics::{MetricsCollector, MemoryMetrics, LayerMetrics};
pub use promotion::{PromotionEngine, PromotionStats, PromotionPerformanceStats};
pub use service::{MemoryConfig, MemoryService, SearchBuilder, CacheConfigType, default_config, BatchBuilder, BatchInsertResult, BatchSearchResult};
pub use service_di::{DIMemoryService, DIMemoryServiceBuilder, MemorySystemStats};
pub use batch_manager::{BatchOperationManager, BatchConfig, BatchOperationBuilder, BatchStats};
pub use storage::VectorStore;
pub use types::{Layer, PromotionConfig, Record, SearchOptions};
pub use api::{UnifiedMemoryAPI, MemoryContext, SearchOptions as ApiSearchOptions, MemoryResult, OptimizationResult, SystemHealth, DetailedHealth, SystemStats, CacheStats, IndexSizes};

/// Быстрое создание DI Memory Service с конфигурацией по умолчанию
pub async fn create_di_memory_service() -> anyhow::Result<DIMemoryService> {
    let config = default_config()?;
    DIMemoryService::new(config).await
}
pub use gpu_accelerated::{GpuBatchProcessor, BatchProcessorConfig, BatchProcessorStats};
pub use backup::{BackupManager, BackupMetadata, BackupInfo};
pub use incremental_backup::{IncrementalBackupManager, IncrementalBackupMetadata, BackupType, DeltaInfo};
pub use optimized_rebuild::{OptimizedRebuildManager, RebuildConfig, RebuildStats, RebuildResult, RebuildMethod, RebuildProgress};
pub use dynamic_dimension::{DynamicDimensionManager, DimensionConfig, DimensionStats, DimensionInfo, DimensionAwareVectorStore};
pub use resource_manager::{ResourceManager, ResourceConfig, ResourceUsage, CurrentLimits, ScalingStats};
pub use flush_config::{FlushConfig, PerformanceMode};
pub use retry::{RetryManager, RetryConfig};
pub use database_manager::{DatabaseManager, DatabaseStats};
pub use transaction::{Transaction, TransactionManager, TransactionGuard, TransactionOp, TransactionStatus, RollbackAction};

// Dependency Injection система
pub use di_container::{DIContainer, DIContainerBuilder, DIContainerStats, Lifetime};
pub use di_memory_config::{MemoryDIConfigurator};

// Профессиональная HNSW реализация - единственная векторная реализация
pub use vector_index_hnswlib::{VectorIndexHnswRs, HnswRsConfig, HnswRsStats};

// ML-based promotion system
pub use ml_promotion::{MLPromotionEngine, MLPromotionConfig, MLPromotionStats, PromotionFeatures};

// Streaming API system
pub use streaming::{StreamingMemoryAPI, StreamingConfig, StreamingRequest, StreamingResponse, StreamingOperation, StreamingResult, SessionConfig, StreamingPriority, GlobalStreamingStats, StreamingInsertRecord, SessionAction};

// Re-export for backward compatibility
pub use types::Layer as MemoryLayer;

// Deprecated types removed in v0.3.0
// Use Layer enum and Record struct instead