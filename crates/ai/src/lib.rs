pub mod config;
pub mod embeddings;
pub mod embeddings_optimized;
pub mod embeddings_v2;
pub mod embeddings_v3;
// pub mod embeddings_real;
// pub mod embeddings_real_v2;
// pub mod embeddings_real_simple;
pub mod embeddings_ort2;
pub mod errors;
pub mod memory_pool;
pub mod models;
pub mod onnx_runtime_simple;
pub mod reranking;
pub mod reranker_mxbai;
pub mod reranker_mxbai_optimized;
pub mod tokenizer;
pub mod tokenization;

pub use config::{AiConfig, EmbeddingConfig, RerankingConfig};
pub use embeddings::{EmbeddingService, EmbeddingResult};
pub use embeddings_optimized::{OptimizedEmbeddingService, OptimizedEmbeddingResult, ServiceStats};
pub use memory_pool::{MemoryPool, PoolStats, GLOBAL_MEMORY_POOL};
pub use embeddings_v2::{EmbeddingServiceV2, EmbeddingResult as EmbeddingResultV2};
pub use embeddings_v3::{EmbeddingServiceV3, EmbeddingResult as EmbeddingResultV3};
// pub use embeddings_real::{RealEmbeddingService, RealEmbeddingConfig, ModelInfo, EmbeddingResult as RealEmbeddingResult};
// pub use embeddings_real_v2::{RealEmbeddingService as RealEmbeddingServiceV2, RealEmbeddingConfig as RealEmbeddingConfigV2};
// pub use embeddings_real_simple::RealEmbeddingServiceSimple;
pub use embeddings_ort2::{OrtEmbeddingService, OrtEmbeddingConfig};
pub use errors::AiError;
pub use models::ModelLoader;
pub use reranking::{RerankingService, RerankResult};
pub use reranker_mxbai::{MxbaiRerankerService, RerankResult as MxbaiRerankResult};
pub use reranker_mxbai_optimized::{OptimizedMxbaiRerankerService, OptimizedRerankResult, BatchRerankResult, RerankBatch, RerankServiceStats};
pub use tokenizer::{TokenizerService, TokenizedInput, SpecialTokens};
pub use tokenization::{OptimizedTokenizer, TokenizedInput as OptTokenizedInput, BatchTokenized};

/// Result type for AI operations
pub type Result<T> = std::result::Result<T, AiError>;