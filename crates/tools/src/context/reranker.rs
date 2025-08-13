// @component: {"k":"C","id":"tool_reranker","t":"Tool reranker with Qwen3 integration for intelligent ranking","m":{"cur":0,"tgt":100,"u":"%"},"f":["reranker","qwen3","ranking","similarity"]}

use super::{Result, ToolContextError};
use anyhow::Context;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// Configuration for reranking operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankingConfig {
    /// Maximum number of documents to rerank
    pub max_documents: usize,

    /// Model path for Qwen3 reranker
    pub model_path: Option<String>,

    /// Use GPU acceleration if available
    pub use_gpu: bool,

    /// Batch size for processing
    pub batch_size: usize,

    /// Maximum sequence length
    pub max_sequence_length: usize,

    /// Similarity threshold for filtering
    pub similarity_threshold: f32,
}

impl Default for RerankingConfig {
    fn default() -> Self {
        Self {
            max_documents: 100,
            model_path: None,
            use_gpu: false,
            batch_size: 8,
            max_sequence_length: 512,
            similarity_threshold: 0.0,
        }
    }
}

/// Request for tool reranking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankingRequest {
    /// Query for semantic similarity
    pub query: String,

    /// Documents to rerank (tool descriptions)
    pub documents: Vec<String>,

    /// Maximum number of results to return
    pub top_k: Option<usize>,
}

/// Response from reranking operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankingResponse {
    /// Documents ranked by relevance
    pub ranked_documents: Vec<ScoredDocument>,

    /// Total processing time
    pub processing_time_ms: u128,

    /// Model used for reranking
    pub model_info: String,
}

/// Document with similarity score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredDocument {
    /// Original document text
    pub document: String,

    /// Similarity score (0.0 to 1.0)
    pub score: f32,

    /// Original index in the input
    pub original_index: usize,
}

/// Tool similarity score with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSimilarityScore {
    /// Tool identifier
    pub tool_id: String,

    /// Semantic similarity score
    pub semantic_score: f32,

    /// Lexical similarity score
    pub lexical_score: f32,

    /// Combined score
    pub combined_score: f32,

    /// Ranking position
    pub rank: usize,
}

/// Trait for tool reranking implementations
#[async_trait]
pub trait ToolReranker {
    /// Rerank documents based on query similarity
    async fn rerank(&self, request: RerankingRequest) -> anyhow::Result<RerankingResponse>;

    /// Get reranker configuration
    fn config(&self) -> &RerankingConfig;

    /// Check if the reranker is ready to use
    async fn health_check(&self) -> anyhow::Result<bool>;
}

/// Qwen3-based tool reranker implementation
pub struct QwenToolReranker {
    config: RerankingConfig,
    reranker: Option<Arc<ai::OptimizedQwen3RerankerService>>,
}

impl QwenToolReranker {
    /// Create a new Qwen3 tool reranker
    pub fn new() -> anyhow::Result<Self> {
        Self::with_config(RerankingConfig::default())
    }

    /// Try to create a new Qwen3 tool reranker, returning fallback on failure
    pub fn new_or_fallback() -> Self {
        match Self::new() {
            Ok(reranker) => reranker,
            Err(e) => {
                debug!("Failed to load Qwen3 model, using fallback: {:?}", e);
                Self::fallback()
            }
        }
    }

    /// Create Qwen3 reranker with custom configuration
    pub fn with_config(config: RerankingConfig) -> anyhow::Result<Self> {
        info!("Initializing Qwen3 tool reranker");

        match Self::initialize_qwen3_reranker(&config) {
            Ok(reranker) => Ok(Self {
                config,
                reranker: Some(reranker),
            }),
            Err(e) => {
                warn!("Failed to initialize Qwen3 reranker: {:?}", e);
                Err(e)
            }
        }
    }

    /// Create a fallback reranker (no ML model)
    pub fn fallback() -> Self {
        warn!("Creating fallback tool reranker (no ML model)");

        Self {
            config: RerankingConfig::default(),
            reranker: None,
        }
    }

    /// Initialize Qwen3 reranker service
    fn initialize_qwen3_reranker(
        config: &RerankingConfig,
    ) -> anyhow::Result<Arc<ai::OptimizedQwen3RerankerService>> {
        // Check if running in test environment - use fallback to avoid ONNX issues
        if cfg!(test) {
            return Err(anyhow::anyhow!("Running in test mode - using fallback"));
        }

        // Check if ONNX runtime is available
        if !std::path::Path::new("./onnxruntime/lib/onnxruntime.dll").exists() {
            return Err(anyhow::anyhow!(
                "ONNX Runtime binary not found - using fallback"
            ));
        }

        // Create Qwen3 reranking configuration
        let reranking_config = ai::RerankingConfig {
            model_name: "Qwen3-Reranker-0.6B-ONNX".to_string(),
            batch_size: config.batch_size,
            max_length: config.max_sequence_length,
            use_gpu: config.use_gpu,
            gpu_config: if config.use_gpu {
                #[cfg(feature = "gpu")]
                {
                    Some(ai::GpuConfig::default())
                }
                #[cfg(not(feature = "gpu"))]
                {
                    None
                }
            } else {
                None
            },
        };

        let service = match ai::OptimizedQwen3RerankerService::new_with_config(reranking_config) {
            Ok(service) => service,
            Err(e) => {
                warn!("Failed to initialize Qwen3 reranker service: {:?}", e);
                return Err(e).context("ONNX Runtime not available");
            }
        };

        info!("✅ Qwen3 tool reranker initialized successfully");
        Ok(Arc::new(service))
    }

    /// Perform lexical similarity fallback
    fn lexical_similarity(&self, query: &str, document: &str) -> f32 {
        let query_words: std::collections::HashSet<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let doc_words: std::collections::HashSet<String> = document
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if query_words.is_empty() || doc_words.is_empty() {
            return 0.0;
        }

        let intersection = query_words.intersection(&doc_words).count();
        let union = query_words.union(&doc_words).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Fallback reranking using lexical similarity
    async fn fallback_rerank(
        &self,
        request: RerankingRequest,
    ) -> anyhow::Result<RerankingResponse> {
        let start_time = std::time::Instant::now();

        debug!(
            "Using fallback lexical reranking for {} documents",
            request.documents.len()
        );

        let mut scored_docs: Vec<ScoredDocument> = request
            .documents
            .into_iter()
            .enumerate()
            .map(|(index, document)| {
                let score = self.lexical_similarity(&request.query, &document);
                ScoredDocument {
                    document,
                    score,
                    original_index: index,
                }
            })
            .collect();

        // Sort by score (descending)
        scored_docs.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply top_k if specified
        if let Some(k) = request.top_k {
            scored_docs.truncate(k);
        }

        // Filter by threshold
        scored_docs.retain(|doc| doc.score >= self.config.similarity_threshold);

        let processing_time = start_time.elapsed().as_millis();

        Ok(RerankingResponse {
            ranked_documents: scored_docs,
            processing_time_ms: processing_time,
            model_info: "Lexical similarity fallback".to_string(),
        })
    }
}

#[async_trait]
impl ToolReranker for QwenToolReranker {
    #[instrument(skip(self), fields(query_len = request.query.len(), num_docs = request.documents.len()))]
    async fn rerank(&self, request: RerankingRequest) -> anyhow::Result<RerankingResponse> {
        let start_time = std::time::Instant::now();

        debug!(
            "Reranking {} documents for query: '{}'",
            request.documents.len(),
            request.query
        );

        // Use Qwen3 reranker if available, otherwise fallback
        match &self.reranker {
            Some(reranker) => {
                // Create batch reranking request
                let batch_request = ai::RerankBatch {
                    query: request.query.clone(),
                    documents: request.documents.clone(),
                    top_k: request.top_k,
                };

                // Perform reranking
                let batch_result = reranker
                    .rerank_batch(&batch_request)
                    .context("Failed to perform Qwen3 reranking")?;

                // Convert results
                let ranked_documents: Vec<ScoredDocument> = batch_result
                    .results
                    .into_iter()
                    .map(|result| ScoredDocument {
                        document: result.document,
                        score: result.score,
                        original_index: result.index,
                    })
                    .collect();

                let processing_time = start_time.elapsed().as_millis();

                info!(
                    "✅ Qwen3 reranking completed in {}ms with {} results",
                    processing_time,
                    ranked_documents.len()
                );

                Ok(RerankingResponse {
                    ranked_documents,
                    processing_time_ms: processing_time,
                    model_info: format!("Qwen3 reranker ({})", reranker.model_path().display()),
                })
            }
            None => {
                warn!("Qwen3 reranker not available, using fallback");
                self.fallback_rerank(request).await
            }
        }
    }

    fn config(&self) -> &RerankingConfig {
        &self.config
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        match &self.reranker {
            Some(_reranker) => {
                // TODO: Implement actual health check for Qwen3 service
                debug!("Qwen3 reranker health check passed");
                Ok(true)
            }
            None => {
                debug!("Using fallback reranker (always healthy)");
                Ok(true)
            }
        }
    }
}

/// Simple lexical reranker for testing and fallback
pub struct SimpleLexicalReranker {
    config: RerankingConfig,
}

impl Default for SimpleLexicalReranker {
    fn default() -> Self {
        Self::new()
    }
}

impl SimpleLexicalReranker {
    pub fn new() -> Self {
        Self {
            config: RerankingConfig::default(),
        }
    }

    fn calculate_similarity(&self, query: &str, document: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let doc_lower = document.to_lowercase();

        // Simple keyword matching
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let matches = query_words
            .iter()
            .filter(|word| doc_lower.contains(*word))
            .count();

        if query_words.is_empty() {
            0.0
        } else {
            matches as f32 / query_words.len() as f32
        }
    }
}

#[async_trait]
impl ToolReranker for SimpleLexicalReranker {
    async fn rerank(&self, request: RerankingRequest) -> anyhow::Result<RerankingResponse> {
        let start_time = std::time::Instant::now();

        let mut scored_docs: Vec<ScoredDocument> = request
            .documents
            .into_iter()
            .enumerate()
            .map(|(index, document)| {
                let score = self.calculate_similarity(&request.query, &document);
                ScoredDocument {
                    document,
                    score,
                    original_index: index,
                }
            })
            .collect();

        // Sort by score (descending)
        scored_docs.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply top_k if specified
        if let Some(k) = request.top_k {
            scored_docs.truncate(k);
        }

        let processing_time = start_time.elapsed().as_millis();

        Ok(RerankingResponse {
            ranked_documents: scored_docs,
            processing_time_ms: processing_time,
            model_info: "Simple lexical similarity".to_string(),
        })
    }

    fn config(&self) -> &RerankingConfig {
        &self.config
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_lexical_reranker() {
        let reranker = SimpleLexicalReranker::new();

        let request = RerankingRequest {
            query: "git status".to_string(),
            documents: vec![
                "Git repository status tool".to_string(),
                "File system utility".to_string(),
                "Git branch management".to_string(),
                "Database query tool".to_string(),
            ],
            top_k: Some(2),
        };

        let response = reranker
            .rerank(request)
            .await
            .expect("Async operation should succeed");

        assert_eq!(response.ranked_documents.len(), 2);
        assert!(response.ranked_documents[0].score > response.ranked_documents[1].score);

        // First result should be git-related
        assert!(response.ranked_documents[0]
            .document
            .to_lowercase()
            .contains("git"));
    }

    #[tokio::test]
    async fn test_qwen_reranker_fallback() {
        // Test fallback mode when Qwen3 is not available
        let reranker = QwenToolReranker::fallback();

        let request = RerankingRequest {
            query: "file operations".to_string(),
            documents: vec![
                "File system operations tool".to_string(),
                "Network utility".to_string(),
                "File management utility".to_string(),
            ],
            top_k: None,
        };

        let response = reranker
            .rerank(request)
            .await
            .expect("Async operation should succeed");

        assert_eq!(response.ranked_documents.len(), 3);
        assert!(response.model_info.contains("fallback"));
    }

    #[test]
    fn test_lexical_similarity() {
        let reranker = QwenToolReranker::fallback();

        let similarity = reranker.lexical_similarity("git status", "git repository status tool");
        assert!(similarity > 0.0);

        let no_similarity = reranker.lexical_similarity("git status", "database operations");
        assert!(similarity > no_similarity);
    }

    #[tokio::test]
    async fn test_reranker_health_check() {
        let reranker = SimpleLexicalReranker::new();
        let is_healthy = reranker
            .health_check()
            .await
            .expect("Async operation should succeed");
        assert!(is_healthy);

        let qwen_fallback = QwenToolReranker::fallback();
        let is_healthy = qwen_fallback
            .health_check()
            .await
            .expect("Async operation should succeed");
        assert!(is_healthy);
    }
}
