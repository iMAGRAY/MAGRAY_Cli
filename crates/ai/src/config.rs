use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Path to models directory
    pub models_dir: PathBuf,
    /// Embedding model configuration
    pub embedding: EmbeddingConfig,
    /// Reranking model configuration
    pub reranking: RerankingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Model name (directory name in models/)
    pub model_name: String,
    /// Batch size for processing
    pub batch_size: usize,
    /// Maximum sequence length
    pub max_length: usize,
    /// Use GPU if available
    pub use_gpu: bool,
    /// GPU configuration (if use_gpu is true)
    #[serde(skip)]
    pub gpu_config: Option<crate::GpuConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankingConfig {
    /// Model name (directory name in models/)
    pub model_name: String,
    /// Batch size for processing
    pub batch_size: usize,
    /// Maximum sequence length
    pub max_length: usize,
    /// Use GPU if available
    pub use_gpu: bool,
    /// GPU configuration (if use_gpu is true)
    #[serde(skip)]
    pub gpu_config: Option<crate::GpuConfig>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            models_dir: PathBuf::from("crates/memory/models"),
            embedding: EmbeddingConfig::default(),
            reranking: RerankingConfig::default(),
        }
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_name: "bge-m3".to_string(), // BGE-M3 выдает 1024-размерные эмбеддинги
            batch_size: 32,
            max_length: 512,
            use_gpu: false,
            gpu_config: None,
        }
    }
}

impl Default for RerankingConfig {
    fn default() -> Self {
        Self {
            model_name: "bge-reranker-v2-m3_dynamic_int8_onnx".to_string(),
            batch_size: 16,
            max_length: 512,
            use_gpu: false,
            gpu_config: None,
        }
    }
}