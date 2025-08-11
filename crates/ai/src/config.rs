use common::service_traits::{ConfigTrait, ConfigurationProfile};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Conditional GPU Config type
#[cfg(feature = "gpu")]
type OptionalGpuConfig = Option<crate::GpuConfig>;

#[cfg(not(feature = "gpu"))]
type OptionalGpuConfig = Option<()>;

// Helper function to create default GPU config
#[cfg(feature = "gpu")]
fn default_gpu_config() -> OptionalGpuConfig {
    Some(crate::GpuConfig::auto_optimized())
}

#[cfg(not(feature = "gpu"))]
fn default_gpu_config() -> OptionalGpuConfig {
    None
}

fn no_gpu_config() -> OptionalGpuConfig {
    None
}

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
    pub gpu_config: OptionalGpuConfig,
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
    pub gpu_config: OptionalGpuConfig,
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

// Реализация общих трейтов для устранения дублирования
impl ConfigTrait for AiConfig {}

impl ConfigurationProfile<AiConfig> for AiConfig {
    fn production() -> Self {
        Self {
            models_dir: PathBuf::from("/opt/magray/models"), // Production path
            embedding: EmbeddingConfig::production(),
            reranking: RerankingConfig::production(),
        }
    }

    fn minimal() -> Self {
        Self {
            models_dir: PathBuf::from("./models"), // Локальная папка
            embedding: EmbeddingConfig {
                model_name: "qwen3emb_small".to_string(),
                batch_size: 8,   // Маленький batch
                max_length: 256, // Меньше tokens
                use_gpu: false,  // CPU только
                gpu_config: no_gpu_config(),
                embedding_dim: Some(512), // Меньшая размерность
            },
            reranking: RerankingConfig {
                model_name: "qwen3_reranker_small".to_string(),
                batch_size: 4,
                max_length: 256,
                use_gpu: false,
                gpu_config: no_gpu_config(),
            },
        }
    }

    fn validate_profile(config: &AiConfig) -> Result<(), common::ConfigError> {
        if config.embedding.batch_size == 0 {
            return Err(common::ConfigError::InvalidValue {
                config_key: "embedding.batch_size".to_string(),
                value: "0".to_string(),
                reason: "Batch size must be greater than 0".to_string(),
            });
        }
        if config.reranking.batch_size == 0 {
            return Err(common::ConfigError::InvalidValue {
                config_key: "reranking.batch_size".to_string(),
                value: "0".to_string(),
                reason: "Batch size must be greater than 0".to_string(),
            });
        }
        Ok(())
    }
}

impl ConfigurationProfile<EmbeddingConfig> for EmbeddingConfig {
    fn production() -> Self {
        Self {
            model_name: "qwen3emb_prod".to_string(),
            batch_size: 64,
            max_length: 1024,
            use_gpu: true,
            gpu_config: default_gpu_config(),
            embedding_dim: Some(1024),
        }
    }

    fn minimal() -> Self {
        Self {
            model_name: "qwen3emb_small".to_string(),
            batch_size: 8,
            max_length: 256,
            use_gpu: false,
            gpu_config: no_gpu_config(),
            embedding_dim: Some(512),
        }
    }

    fn validate_profile(_config: &EmbeddingConfig) -> Result<(), common::ConfigError> {
        Ok(())
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_name: "qwen3emb".to_string(), // Qwen3 embedding model
            batch_size: 32,
            max_length: 512,
            use_gpu: true,                    // Включаем GPU по умолчанию
            gpu_config: default_gpu_config(), // Автоматическая настройка GPU
            embedding_dim: Some(1024),        // Qwen3 also 1024 dims
        }
    }
}

impl ConfigurationProfile<RerankingConfig> for RerankingConfig {
    fn production() -> Self {
        Self {
            model_name: "qwen3_reranker_prod".to_string(),
            batch_size: 32,
            max_length: 1024,
            use_gpu: true,
            gpu_config: default_gpu_config(),
        }
    }

    fn minimal() -> Self {
        Self {
            model_name: "qwen3_reranker_small".to_string(),
            batch_size: 4,
            max_length: 256,
            use_gpu: false,
            gpu_config: no_gpu_config(),
        }
    }

    fn validate_profile(_config: &RerankingConfig) -> Result<(), common::ConfigError> {
        Ok(())
    }
}

impl Default for RerankingConfig {
    fn default() -> Self {
        Self {
            model_name: "qwen3_reranker".to_string(), // Qwen3 reranker model
            batch_size: 16,
            max_length: 512,
            use_gpu: true,                    // Включаем GPU по умолчанию
            gpu_config: default_gpu_config(), // Автоматическая настройка GPU
        }
    }
}
