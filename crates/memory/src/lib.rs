#![cfg_attr(feature = "minimal", allow(dead_code, unused_imports, unused_variables))]

#[cfg(not(feature = "minimal"))]
mod batch_manager;
#[cfg(all(not(feature = "minimal"), feature = "vector-search"))]
mod batch_optimized; // Ultra-optimized batch operations для 1000+ QPS
#[cfg(not(feature = "minimal"))]
mod cache_interface;
#[cfg(not(feature = "minimal"))]
mod cache_lru;
#[cfg(not(feature = "minimal"))]
mod cache_migration;
#[cfg(not(feature = "minimal"))]
pub mod fallback;
#[cfg(not(feature = "minimal"))]
pub mod health;
#[cfg(all(not(feature = "minimal"), feature = "hnsw-index"))]
pub mod hnsw_index; // Модульная HNSW архитектура
                    // pub mod layers; // ВРЕМЕННО ОТКЛЮЧЕНО для бенчмарка - проблемы с sqlx
#[cfg(all(not(feature = "minimal"), feature = "gpu-acceleration"))]
pub mod gpu_ultra_accelerated; // GPU acceleration для 10x+ speedup
#[cfg(not(feature = "minimal"))]
mod metrics;
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
pub mod ml_promotion; // Декомпозированная ML promotion система (SOLID compliant)
#[cfg(not(feature = "minimal"))]
mod notifications;
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
pub mod promotion;
#[cfg(not(feature = "minimal"))]
pub mod service_di; // REFACTORED модули в service_di/
#[cfg(not(feature = "minimal"))]
pub mod service_di_facade;
#[cfg(all(not(feature = "minimal"), feature = "num_cpus"))]
pub mod simd_feature_detection; // Advanced CPU feature detection и adaptive algorithm selection
#[cfg(not(feature = "minimal"))]
pub mod simd_fixed; // Исправленная SIMD реализация для debugging
#[cfg(all(not(feature = "minimal"), feature = "rayon"))]
pub mod simd_optimized; // SIMD оптимизации для векторных операций
#[cfg(not(feature = "minimal"))]
pub mod simd_ultra_optimized; // Ultra-optimized SIMD для sub-1ms performance // FACADE для обратной совместимости

// Re-export для обратной совместимости
// pub use di::{DIMemoryService, DIMemoryServiceBuilder}; // ВРЕМЕННО ОТКЛЮЧЕНО
#[cfg(not(feature = "minimal"))]
pub use service_di::service_config::default_config;
#[cfg(not(feature = "minimal"))]
pub mod api;
#[cfg(all(not(feature = "minimal"), feature = "backup-restore"))]
mod backup;
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
mod database_manager;
#[cfg(not(feature = "minimal"))]
mod flush_config;
#[cfg(not(feature = "minimal"))]
pub mod gpu_accelerated;
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
pub mod migration;
#[cfg(not(feature = "minimal"))]
pub mod resource_manager;
#[cfg(not(feature = "minimal"))]
mod retry;
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
pub mod storage;
#[cfg(not(feature = "minimal"))]
mod streaming;
#[cfg(not(feature = "minimal"))]
pub mod transaction;
#[cfg(not(feature = "minimal"))]
pub mod types;
#[cfg(all(not(feature = "minimal"), feature = "hnsw-index"))]
mod vector_index_hnswlib; // Critical for vector storage
                          
// Экспорт единой DI-системы
pub mod di;

// container factory/config are not exported from unified di; remove these re-exports

// ОСНОВНОЙ DI API (в минимальном профиле доступен базовый контейнер)
#[cfg(not(feature = "minimal"))]
pub use di::{
    UnifiedContainer as DIContainer,
    UnifiedContainerBuilder as DIContainerBuilder,
    Lifetime,
};
#[cfg(feature = "minimal")]
pub use di::{
    UnifiedContainer as DIContainer,
    UnifiedContainerBuilder as DIContainerBuilder,
    Lifetime,
};

// Legacy API types - перенаправляем на реальные типы из di
pub use di::{DIContainerStats, DIPerformanceMetrics};
// Оркестрация системы памяти (отключаем по умолчанию для минимальной сборки)
#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
pub mod orchestration;
// Специализированные сервисы (SOLID refactoring) (отключаем по умолчанию)
#[cfg(all(not(feature = "minimal"), feature = "services-modules"))]
pub mod services;
// Utility functions и error handling
#[cfg(not(feature = "minimal"))]
pub mod utils;
#[cfg(all(not(feature = "minimal"), feature = "vector-search"))]
pub use batch_optimized::{
    AlignedBatchVectors, BatchOptimizedConfig, BatchOptimizedProcessor, BatchOptimizedStats,
};
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
pub use batch_manager::{BatchConfig, BatchOperationBuilder, BatchOperationManager, BatchStats};
#[cfg(not(feature = "minimal"))]
pub use cache_lru::{
    CacheConfig as LruCacheConfig, CacheConfig, EmbeddingCacheLRU as EmbeddingCache,
};

// Cache configuration type for service - теперь только LRU
#[cfg(not(feature = "minimal"))]
pub type CacheConfigType = LruCacheConfig;
#[cfg(not(feature = "minimal"))]
pub use types::{Layer, PromotionConfig, Record, SearchOptions};
// Legacy MemoryService удален - используем DIMemoryService через unified_container
// Batch result types доступны только при включенных orchestration-modules
#[cfg(all(not(feature = "minimal"), feature = "orchestration-modules"))]
pub use service_di::{BatchInsertResult, BatchSearchResult};

// NEW: Refactored services based on SOLID principles
#[cfg(all(not(feature = "minimal"), feature = "services-modules"))]
pub use services::{
    CacheService,
    CacheServiceTrait,
    CoordinatorService,
    CoordinatorServiceTrait,
    // Service implementations
    CoreMemoryService,
    // Trait interfaces
    CoreMemoryServiceTrait,
    MonitoringService,
    MonitoringServiceTrait,
    ResilienceService,
    ResilienceServiceTrait,
    ServiceCollection,
    // Service factory and collections
    ServiceFactory,
    ServiceFactoryConfig,
};

// NEW: Refactored DIMemoryService using SOLID composition instead of God Object
#[cfg(all(not(feature = "minimal"), feature = "services-modules"))]
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
// MemoryDIConfigurator moved to di/unified_container.rs
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
pub use database_manager::DatabaseManager;
#[cfg(all(not(feature = "minimal"), feature = "gpu-acceleration"))]
pub use gpu_ultra_accelerated::{
    benchmark_gpu_vs_cpu, GpuCosineProcessor, GpuDevice, GpuDeviceManager,
};
#[cfg(not(feature = "minimal"))]
pub use health::{
    AlertSeverity, ComponentType, HealthMonitor, HealthMonitorConfig as HealthConfig,
    SystemHealthStatus,
};
#[cfg(not(feature = "minimal"))]
pub use metrics::{LatencyMetrics, LayerMetrics, MemoryMetrics, MetricsCollector};
#[cfg(not(feature = "minimal"))]
pub use notifications::{NotificationManager, NotificationManager as NotificationSystem};
#[cfg(not(feature = "minimal"))]
pub use resource_manager::{ResourceConfig, ResourceManager};
#[cfg(all(not(feature = "minimal"), feature = "num_cpus"))]
pub use simd_feature_detection::{
    get_adaptive_selector, quick_cpu_info, AdaptiveAlgorithmSelector, AlgorithmStrategy, CpuInfo,
    SimdLevel, WorkloadProfile,
};
#[cfg(not(feature = "minimal"))]
pub use simd_fixed::debug_simd_performance;
#[cfg(all(not(feature = "minimal"), feature = "rayon"))]
pub use simd_optimized::{
    batch_cosine_distance_optimized, cosine_distance_auto, cosine_distance_memory_optimized,
    run_comprehensive_benchmark,
};
#[cfg(not(feature = "minimal"))]
pub use simd_ultra_optimized::{
    batch_cosine_distance_ultra, cosine_distance_ultra_optimized, test_ultra_optimized_performance,
    AlignedVector,
};
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
pub use storage::VectorStore;
#[cfg(not(feature = "minimal"))]
pub use transaction::{Transaction, TransactionGuard, TransactionManager};

/// Быстрое создание DI Memory Service с конфигурацией по умолчанию - ВРЕМЕННО ОТКЛЮЧЕНО
/// DIMemoryService не существует - используйте RefactoredDIMemoryService
// pub async fn create_di_memory_service() -> anyhow::Result<DIMemoryService> {
//     let config = service_di::service_config::default_config()?;
//     DIMemoryService::new(config).await
// }

// Профессиональная HNSW реализация - единственная векторная реализация
#[cfg(all(not(feature = "minimal"), feature = "hnsw-index"))]
pub use vector_index_hnswlib::{HnswRsConfig, HnswRsStats, VectorIndexHnswRs};

// HNSW index module exports
#[cfg(all(not(feature = "minimal"), feature = "hnsw-index"))]
pub use hnsw_index::{HnswConfig, HnswStats, VectorIndex};

// ML-based promotion system
#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
pub use ml_promotion::{
    MLPromotionConfig, MLPromotionEngine, MLPromotionStats, PromotionDecision, PromotionFeatures,
    UsageTracker,
};

// Streaming API system
#[cfg(not(feature = "minimal"))]
pub use streaming::{
    GlobalStreamingStats, SessionAction, SessionConfig, StreamingConfig, StreamingInsertRecord,
    StreamingMemoryAPI, StreamingOperation, StreamingPriority, StreamingRequest, StreamingResponse,
    StreamingResult,
};

// Re-export for backward compatibility
#[cfg(not(feature = "minimal"))]
pub use types::Layer as MemoryLayer;

// Utility functions для улучшенного error handling
// ВРЕМЕННО ОТКЛЮЧЕНО для исправления ошибок компиляции
// pub use utils::{production_helpers, test_helpers, ErrorUtils};

// Deprecated types removed in v0.3.0
// Use Layer enum and Record struct instead
