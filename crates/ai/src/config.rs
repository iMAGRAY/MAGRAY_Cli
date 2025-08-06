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
    /// Embedding dimension (optional, defaults based on model)
    pub embedding_dim: Option<usize>,
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
            models_dir: PathBuf::from("models"), // Централизованная папка models в корне
            embedding: EmbeddingConfig::default(),
            reranking: RerankingConfig::default(),
        }
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_name: "qwen3emb".to_string(), // Qwen3 embedding model
            batch_size: 32,
            max_length: 512,
            use_gpu: true,  // Включаем GPU по умолчанию
            gpu_config: Some(crate::GpuConfig::auto_optimized()), // Автоматическая настройка GPU
            embedding_dim: Some(1024), // Qwen3 also 1024 dims
        }
    }
}

impl Default for RerankingConfig {
    fn default() -> Self {
        Self {
            model_name: "qwen3_reranker".to_string(), // Qwen3 reranker model
            batch_size: 16,
            max_length: 512,
            use_gpu: true,  // Включаем GPU по умолчанию
            gpu_config: Some(crate::GpuConfig::auto_optimized()), // Автоматическая настройка GPU
        }
    }
}