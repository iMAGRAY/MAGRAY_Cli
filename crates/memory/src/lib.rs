mod batch_manager;
mod batch_optimized; // Ultra-optimized batch operations для 1000+ QPS
mod cache_interface;
mod cache_lru;
mod cache_migration;
pub mod fallback;
pub mod health;
pub mod hnsw_index; // Модульная HNSW архитектура
                    // pub mod layers; // ВРЕМЕННО ОТКЛЮЧЕНО для бенчмарка - проблемы с sqlx
pub mod gpu_ultra_accelerated; // GPU acceleration для 10x+ speedup
mod metrics;
pub mod ml_promotion; // Декомпозированная ML promotion система (SOLID compliant)
mod notifications;
pub mod promotion;
pub mod service_di; // REFACTORED модули в service_di/
pub mod service_di_facade;
pub mod simd_feature_detection; // Advanced CPU feature detection и adaptive algorithm selection
pub mod simd_fixed; // Исправленная SIMD реализация для debugging
pub mod simd_optimized; // SIMD оптимизации для векторных операций
pub mod simd_ultra_optimized; // Ultra-optimized SIMD для sub-1ms performance // FACADE для обратной совместимости

// Re-export для обратной совместимости
// pub use di::{DIMemoryService, DIMemoryServiceBuilder}; // ВРЕМЕННО ОТКЛЮЧЕНО
pub use service_di::service_config::default_config;
pub mod api;
mod backup;
mod database_manager;
mod flush_config;
pub mod gpu_accelerated;
pub mod migration;
pub mod resource_manager;
mod retry;
pub mod storage;
mod streaming;
pub mod transaction;
pub mod types;
mod vector_index_hnswlib; // Critical for vector storage
                          // НОВАЯ УПРОЩЕННАЯ DI СИСТЕМА (заменяет сложную di/ папку)
pub mod simple_di;

// COMPATIBILITY STUB для старой di системы
pub mod di {
    pub use crate::di_compatibility_stub::*;
}

// Подключаем заглушку совместимости
mod di_compatibility_stub;

// ОСНОВНОЙ DI API - используем упрощенную систему
pub use simple_di::{
    create_container, create_default_container, DIContainer, DIContainerBuilder, Lifetime,
};

// Legacy API types - заглушки для обратной совместимости
#[derive(Debug, Default)]
pub struct DIContainerStats {
    pub total_resolutions: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

#[derive(Debug, Default)]
pub struct DIPerformanceMetrics {
    pub total_resolutions: u64,
    pub average_resolution_time: std::time::Duration,
    pub cache_hits: u64,
    pub memory_usage: usize,
}
// Оркестрация системы памяти
pub mod orchestration;
// Специализированные сервисы (SOLID refactoring)
pub mod services;
// Utility functions и error handling
pub mod utils;
pub use batch_manager::{BatchConfig, BatchOperationBuilder, BatchOperationManager, BatchStats};
pub use batch_optimized::{
    AlignedBatchVectors, BatchOptimizedConfig, BatchOptimizedProcessor, BatchOptimizedStats,
};
pub use cache_lru::{
    CacheConfig as LruCacheConfig, CacheConfig, EmbeddingCacheLRU as EmbeddingCache,
};

// Cache configuration type for service - теперь только LRU
pub type CacheConfigType = LruCacheConfig;
pub use types::{Layer, PromotionConfig, Record, SearchOptions};
// Legacy MemoryService удален - используем DIMemoryService через unified_container
pub use service_di::{BatchInsertResult, BatchSearchResult};

// NEW: Refactored services based on SOLID principles
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
pub use api::{
    CacheStats, DetailedHealth, IndexSizes, MemoryContext, MemoryResult, OptimizationResult,
    SearchOptions as ApiSearchOptions, SystemHealth, SystemStats, UnifiedMemoryAPI,
};
pub use database_manager::DatabaseManager;
pub use gpu_ultra_accelerated::{
    benchmark_gpu_vs_cpu, GpuCosineProcessor, GpuDevice, GpuDeviceManager,
};
pub use health::{
    AlertSeverity, ComponentType, HealthMonitor, HealthMonitorConfig as HealthConfig,
    SystemHealthStatus,
};
pub use metrics::{LatencyMetrics, LayerMetrics, MemoryMetrics, MetricsCollector};
pub use notifications::{NotificationManager, NotificationManager as NotificationSystem};
pub use resource_manager::{ResourceConfig, ResourceManager};
pub use simd_feature_detection::{
    get_adaptive_selector, quick_cpu_info, AdaptiveAlgorithmSelector, AlgorithmStrategy, CpuInfo,
    SimdLevel, WorkloadProfile,
};
pub use simd_fixed::debug_simd_performance;
pub use simd_optimized::{
    batch_cosine_distance_optimized, cosine_distance_auto, cosine_distance_memory_optimized,
    run_comprehensive_benchmark,
};
pub use simd_ultra_optimized::{
    batch_cosine_distance_ultra, cosine_distance_ultra_optimized, test_ultra_optimized_performance,
    AlignedVector,
};
pub use storage::VectorStore;
pub use transaction::{Transaction, TransactionGuard, TransactionManager};

/// Быстрое создание DI Memory Service с конфигурацией по умолчанию - ВРЕМЕННО ОТКЛЮЧЕНО
/// DIMemoryService не существует - используйте RefactoredDIMemoryService
// pub async fn create_di_memory_service() -> anyhow::Result<DIMemoryService> {
//     let config = service_di::service_config::default_config()?;
//     DIMemoryService::new(config).await
// }

// Профессиональная HNSW реализация - единственная векторная реализация
pub use vector_index_hnswlib::{HnswRsConfig, HnswRsStats, VectorIndexHnswRs};

// HNSW index module exports
pub use hnsw_index::{HnswConfig, HnswStats, VectorIndex};

// ML-based promotion system
pub use ml_promotion::{
    MLPromotionConfig, MLPromotionEngine, MLPromotionStats, PromotionDecision, PromotionFeatures,
    UsageTracker,
};

// Streaming API system
pub use streaming::{
    GlobalStreamingStats, SessionAction, SessionConfig, StreamingConfig, StreamingInsertRecord,
    StreamingMemoryAPI, StreamingOperation, StreamingPriority, StreamingRequest, StreamingResponse,
    StreamingResult,
};

// Re-export for backward compatibility
pub use types::Layer as MemoryLayer;

// Utility functions для улучшенного error handling
// ВРЕМЕННО ОТКЛЮЧЕНО для исправления ошибок компиляции
// pub use utils::{production_helpers, test_helpers, ErrorUtils};

// Deprecated types removed in v0.3.0
// Use Layer enum and Record struct instead
