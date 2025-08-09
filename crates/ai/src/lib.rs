pub mod auto_device_selector;
pub mod config;
pub mod errors;
pub mod memory_pool;

// Only include embeddings for cpu/gpu builds
#[cfg(feature = "embeddings")]
pub mod embeddings_cpu;

#[cfg(feature = "gpu")]
pub mod embeddings_gpu;

// GPU-related modules only for gpu builds
#[cfg(feature = "gpu")]
pub mod gpu_config;

#[cfg(feature = "gpu")]
pub mod gpu_detector;

#[cfg(feature = "gpu")]
pub mod gpu_fallback;

#[cfg(feature = "gpu")]
pub mod gpu_memory_pool;

#[cfg(feature = "gpu")]
pub mod gpu_pipeline;

#[cfg(feature = "gpu")]
pub mod tensorrt_cache;

// Model management for cpu/gpu builds
#[cfg(any(feature = "cpu", feature = "gpu"))]
pub mod model_downloader;

#[cfg(any(feature = "cpu", feature = "gpu"))]
pub mod model_registry;

#[cfg(any(feature = "cpu", feature = "gpu"))]
pub mod models;

#[cfg(any(feature = "cpu", feature = "gpu"))]
pub mod ort_setup;

// Reranking for cpu/gpu builds
#[cfg(feature = "reranking")]
pub mod reranker_qwen3;

#[cfg(feature = "reranking")]
pub mod reranker_qwen3_optimized;

#[cfg(feature = "reranking")]
pub mod reranking;

// Tokenization for embeddings
#[cfg(feature = "embeddings")]
pub mod tokenization;

#[cfg(feature = "embeddings")]
pub mod tokenizer;

// Core exports (always available)
pub use auto_device_selector::EmbeddingServiceTrait;
pub use config::{AiConfig, EmbeddingConfig, RerankingConfig};
pub use errors::AiError;

// Conditional exports based on features
#[cfg(feature = "embeddings")]
pub use embeddings_cpu::{CpuEmbeddingService, OptimizedEmbeddingResult, ServiceStats};

#[cfg(feature = "gpu")]
pub use embeddings_gpu::GpuEmbeddingService;

#[cfg(feature = "gpu")]
pub use gpu_config::{GpuConfig, GpuInfo};

#[cfg(feature = "gpu")]
pub use gpu_fallback::{FallbackPolicy, GpuFallbackManager};
#[cfg(feature = "gpu")]
pub use gpu_pipeline::{GpuPipelineManager, PipelineConfig, PipelineStats};

// Memory pool (always available)
pub use memory_pool::{
    get_input_buffer, get_pool_stats, return_input_buffer, MemoryPool, PoolStats, PooledBuffer,
    GLOBAL_MEMORY_POOL,
};

#[cfg(any(feature = "cpu", feature = "gpu"))]
pub use model_registry::{ModelInfo, ModelRegistry, ModelType, MODEL_REGISTRY};

#[cfg(any(feature = "cpu", feature = "gpu"))]
pub use models::ModelLoader;

#[cfg(feature = "reranking")]
pub use reranker_qwen3::{
    BatchRerankResult, OptimizedQwen3RerankerService, OptimizedRerankResult, RerankBatch,
    RerankServiceStats,
};

#[cfg(feature = "reranking")]
pub use reranking::{RerankResult, RerankingService};

#[cfg(feature = "embeddings")]
pub use tokenization::{BatchTokenized, OptimizedTokenizer, TokenizedInput as OptTokenizedInput};

#[cfg(feature = "embeddings")]
pub use tokenizer::{SpecialTokens, TokenizedInput, TokenizerService};

/// Result type for AI operations
pub type Result<T> = std::result::Result<T, AiError>;

#[cfg(test)]
fn _ort_available() -> bool {
    std::env::var("ORT_DYLIB_PATH").is_ok()
}

pub fn ort_available() -> bool {
    std::env::var("ORT_DYLIB_PATH").is_ok()
}

pub fn should_disable_ort() -> bool {
    std::env::var("MAGRAY_FORCE_NO_ORT").map(|v| matches!(v.to_lowercase().as_str(), "1"|"true"|"yes"|"y"))
        .unwrap_or(false)
}
