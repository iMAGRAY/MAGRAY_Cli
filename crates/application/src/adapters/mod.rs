//! Adapters for Integration with Memory Services
//!
//! Адаптеры для интеграции Application Layer с существующими memory services,
//! реализуя port/adapter pattern для dependency inversion.

pub mod cache_service_adapter;
pub mod embedding_service_adapter;
pub mod memory_service_adapter;
pub mod metrics_collector_adapter;
pub mod notification_service_adapter;
pub mod search_service_adapter;

// AI-Memory integration adapter - conditional compilation
#[cfg(any(feature = "cpu", feature = "gpu"))]
pub mod ai_memory_adapter;

// Re-export adapters with explicit naming to avoid conflicts
pub use cache_service_adapter::{
    CacheServiceAdapter, CacheServiceTrait as CacheAdapterServiceTrait,
};
pub use embedding_service_adapter::{
    CpuEmbeddingServiceTrait, EmbeddingAdapterConfig, EmbeddingServiceAdapter,
    GpuEmbeddingServiceTrait,
};
pub use memory_service_adapter::{
    CacheServiceTrait as MemoryAdapterCacheServiceTrait, MemoryOrchestratorTrait,
    MemoryServiceAdapter,
};
pub use metrics_collector_adapter::MetricsCollectorAdapter;
pub use notification_service_adapter::NotificationServiceAdapter;
pub use search_service_adapter::{
    SearchAdapterConfig, SearchServiceAdapter, TextSearchServiceTrait, VectorSearchServiceTrait,
};

// AI-Memory adapter exports - conditional compilation
#[cfg(any(feature = "cpu", feature = "gpu"))]
pub use ai_memory_adapter::{
    AiMemoryAdapter, AiMemoryAdapterFactory, AiMemoryInsights, AiMemorySearchResult,
    BatchStoreItem, BatchStoreResult,
};
