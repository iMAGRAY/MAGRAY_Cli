pub mod auto_device_selector;
pub mod config;
pub mod errors;
pub mod memory_pool;

#[cfg(feature = "embeddings")]
pub mod embeddings_cpu;

#[cfg(feature = "gpu")]
pub mod embeddings_gpu;

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

#[cfg(any(feature = "cpu", feature = "gpu"))]
pub mod model_downloader;

#[cfg(any(feature = "cpu", feature = "gpu"))]
pub mod model_registry;

#[cfg(any(feature = "cpu", feature = "gpu"))]
pub mod models;

#[cfg(any(feature = "cpu", feature = "gpu"))]
pub mod ort_setup;

#[cfg(feature = "reranking")]
pub mod reranker_qwen3;

#[cfg(feature = "reranking")]
pub mod reranker_qwen3_optimized;

#[cfg(feature = "reranking")]
pub mod reranking;

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

use lazy_static::lazy_static;
use parking_lot::RwLock;
use std::collections::HashMap;

#[cfg(feature = "onnx")]
use crate::models::OnnxSession;

#[cfg(feature = "onnx")]
lazy_static! {
    /// Глобальный пул прогретых ONNX-сессий (держит их в памяти на протяжении процесса)
    static ref WARMED_MODELS: RwLock<HashMap<String, Arc<OnnxSession>>> = RwLock::new(HashMap::new());
}

use std::sync::Arc;

/// Внутренняя реализация прогрева (только если доступен onnx)
#[cfg(feature = "onnx")]
fn warmup_models_impl(model_names: &[&str]) -> anyhow::Result<()> {
    use crate::model_registry::MODEL_REGISTRY;

    let mut guard = WARMED_MODELS.write();
    for &name in model_names {
        if guard.contains_key(name) {
            continue;
        }
        let model_path = MODEL_REGISTRY.get_model_path(name).join("model.onnx");
        if !model_path.exists() {
            continue;
        }
        // Создаем real/fallback сессию
        let session = models::OnnxSession::new(name.to_string(), model_path.clone(), false)
            .unwrap_or_else(|e| {
                models::OnnxSession::new_fallback(
                    name.to_string(),
                    model_path.clone(),
                    e.to_string(),
                )
            });
        guard.insert(name.to_string(), Arc::new(session));
    }
    Ok(())
}

/// Прогреть и удерживать в памяти указанные модели (embedding и reranker)
pub fn warmup_models(model_names: &[&str]) -> anyhow::Result<()> {
    #[cfg(feature = "onnx")]
    {
        warmup_models_impl(model_names)
    }
    #[cfg(not(feature = "onnx"))]
    {
        // Без onnx просто no-op
        let _ = model_names;
        Ok(())
    }
}

/// Получить прогретую сессию, если она есть (только при onnx)
#[cfg(feature = "onnx")]
pub fn get_warmed_session(name: &str) -> Option<Arc<OnnxSession>> {
    WARMED_MODELS.read().get(name).cloned()
}

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
    std::env::var("MAGRAY_FORCE_NO_ORT")
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "y"))
        .unwrap_or(false)
}
