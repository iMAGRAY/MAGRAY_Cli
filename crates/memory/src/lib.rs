mod batch_manager;
mod batch_optimized; // Ultra-optimized batch operations для 1000+ QPS
mod cache_lru;
mod cache_interface;
mod cache_migration;
pub mod fallback;
mod hnsw_index; // Модульная HNSW архитектура
pub mod health;
// pub mod layers; // ВРЕМЕННО ОТКЛЮЧЕНО для бенчмарка - проблемы с sqlx
mod metrics;
mod notifications;
pub mod promotion;
mod ml_promotion;
pub mod simd_optimized; // SIMD оптимизации для векторных операций
pub mod simd_fixed; // Исправленная SIMD реализация для debugging
pub mod simd_ultra_optimized; // Ultra-optimized SIMD для sub-1ms performance
pub mod gpu_ultra_accelerated; // GPU acceleration для 10x+ speedup
pub mod service_di; // REFACTORED модули в service_di/
pub mod service_di_facade; // FACADE для обратной совместимости
pub mod service_di_refactored; // Рефакторированная архитектура

// Re-export для обратной совместимости
pub use service_di_facade::{DIMemoryService, DIMemoryServiceBuilder};
pub mod storage;
pub mod types;
mod vector_index_hnswlib; // Critical for vector storage
pub mod transaction;
mod backup;
pub mod migration;
pub mod api;
mod streaming;
mod flush_config;
pub mod gpu_accelerated;
pub mod resource_manager;
mod retry;
mod database_manager;
// Refactored Dependency Injection система (SOLID compliant)
pub mod di; // Новая модульная архитектура
mod di_container; // Legacy facade для обратной совместимости
pub mod di_memory_config;
// Re-export главного API (из нового di модуля)
pub use di::{DIContainer, DIContainerBuilder, create_default_container};
// Legacy compatibility
pub use di_container::{DIPerformanceMetrics, DIContainerStats, Lifetime};
// Оркестрация системы памяти
pub mod orchestration;
// Специализированные сервисы (SOLID refactoring)
pub mod services;
pub use batch_manager::{BatchOperationManager, BatchConfig, BatchOperationBuilder, BatchStats};
pub use batch_optimized::{BatchOptimizedProcessor, BatchOptimizedConfig, BatchOptimizedStats, AlignedBatchVectors};
pub use cache_lru::{EmbeddingCacheLRU as EmbeddingCache, CacheConfig as LruCacheConfig, CacheConfig};

// Cache configuration type for service - теперь только LRU
pub type CacheConfigType = LruCacheConfig;
pub use storage::VectorStore;
pub use types::{Layer, PromotionConfig, Record, SearchOptions};
// Legacy MemoryService удален - используем DIMemoryService (DEPRECATED)
pub use service_di_refactored::{DIMemoryService as MemoryService, MemoryServiceConfig, MemoryConfig, default_config};
pub use service_di::{BatchInsertResult, BatchSearchResult};

// NEW: Refactored services based on SOLID principles
pub use services::{
    // Trait interfaces
    CoreMemoryServiceTrait, CoordinatorServiceTrait, ResilienceServiceTrait, 
    MonitoringServiceTrait, CacheServiceTrait,
    // Service implementations
    CoreMemoryService, CoordinatorService, ResilienceService, 
    MonitoringService, CacheService,
    // Service factory and collections
    ServiceFactory, ServiceCollection, ServiceFactoryConfig,
};

// NEW: Refactored DIMemoryService using SOLID composition instead of God Object
pub use services::{RefactoredDIMemoryService, RefactoredDIMemoryServiceBuilder};

// ВРЕМЕННО ОТКЛЮЧЕНО - НОВАЯ СЛОЕВАЯ АРХИТЕКТУРА
// pub use layers::{
//     // Trait definitions
//     StorageLayer, IndexLayer, QueryLayer, CacheLayer, LayerHealth,
//     // Concrete implementations  
//     LayeredMemoryBuilder, LayeredDIContainer,
//     // Configuration types
//     StorageConfig, IndexConfig, QueryConfig, CacheConfig,
//     // Result types
//     VectorSearchResult, StorageStats, IndexStats, QueryStats, RankingCriteria,
//     LayerHealthStatus,
// };
pub use di_memory_config::{MemoryDIConfigurator};
pub use health::{HealthMonitor, HealthMonitorConfig as HealthConfig, ComponentType, AlertSeverity, SystemHealthStatus};
pub use api::{UnifiedMemoryAPI, MemoryContext, SearchOptions as ApiSearchOptions, MemoryResult, OptimizationResult, SystemHealth, DetailedHealth, SystemStats, CacheStats, IndexSizes};
pub use resource_manager::{ResourceManager, ResourceConfig};
pub use notifications::{NotificationManager, NotificationManager as NotificationSystem};
pub use database_manager::DatabaseManager;
pub use metrics::{MetricsCollector, MemoryMetrics, LatencyMetrics, LayerMetrics};
pub use transaction::{Transaction, TransactionManager, TransactionGuard};
pub use simd_optimized::{cosine_distance_auto, cosine_distance_memory_optimized, batch_cosine_distance_optimized, run_comprehensive_benchmark};
pub use simd_fixed::{debug_simd_performance};
pub use simd_ultra_optimized::{cosine_distance_ultra_optimized, AlignedVector, batch_cosine_distance_ultra, test_ultra_optimized_performance};
pub use gpu_ultra_accelerated::{GpuDevice, GpuCosineProcessor, GpuDeviceManager, benchmark_gpu_vs_cpu};

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