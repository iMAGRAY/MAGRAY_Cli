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

impl AiConfig {
    pub fn production() -> Self {
        Self {
            models_dir: PathBuf::from("/opt/magray/models"), // Production path
            embedding: EmbeddingConfig {
                model_name: "qwen3emb_prod".to_string(),
                batch_size: 64, // Больший batch для production
                max_length: 1024, // Больше tokens
                use_gpu: true,
                gpu_config: Some(crate::GpuConfig::auto_optimized()),
                embedding_dim: Some(1024),
            },
            reranking: RerankingConfig {
                model_name: "qwen3_reranker_prod".to_string(),
                batch_size: 32, // Больший batch
                max_length: 1024,
                use_gpu: true,
                gpu_config: Some(crate::GpuConfig::auto_optimized()),
            },
        }
    }

    pub fn minimal() -> Self {
        Self {
            models_dir: PathBuf::from("./models"), // Локальная папка
            embedding: EmbeddingConfig {
                model_name: "qwen3emb_small".to_string(),
                batch_size: 8, // Маленький batch
                max_length: 256, // Меньше tokens
                use_gpu: false, // CPU только
                gpu_config: None,
                embedding_dim: Some(512), // Меньшая размерность
            },
            reranking: RerankingConfig {
                model_name: "qwen3_reranker_small".to_string(),
                batch_size: 4,
                max_length: 256,
                use_gpu: false,
                gpu_config: None,
            },
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