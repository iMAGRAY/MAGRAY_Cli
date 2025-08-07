//! Embedding Provider Port
//!
//! Абстракция для AI embedding services независимо от конкретной реализации.

use async_trait::async_trait;
use crate::ApplicationResult;

/// Trait для embedding generation services
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for single text
    async fn generate_embedding(&self, text: &str) -> ApplicationResult<Vec<f32>>;
    
    /// Generate embeddings for batch of texts
    async fn generate_batch_embeddings(&self, texts: &[String]) -> ApplicationResult<Vec<Vec<f32>>>;
    
    /// Get embedding dimensions for this provider
    fn embedding_dimensions(&self) -> usize;
    
    /// Get provider model name/identifier
    fn model_identifier(&self) -> &str;
    
    /// Check if provider supports batching
    fn supports_batching(&self) -> bool;
    
    /// Get maximum batch size supported
    fn max_batch_size(&self) -> usize;
    
    /// Health check for embedding service
    async fn health_check(&self) -> ApplicationResult<ProviderHealth>;
    
    /// Get provider performance metrics
    async fn get_metrics(&self) -> ApplicationResult<EmbeddingMetrics>;
}

/// Health status of embedding provider
#[derive(Debug, Clone)]
pub struct ProviderHealth {
    pub is_healthy: bool,
    pub response_time_ms: u64,
    pub error_rate: f32,
    pub last_error: Option<String>,
    pub uptime_seconds: u64,
}

/// Performance metrics for embedding provider
#[derive(Debug, Clone)]
pub struct EmbeddingMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub tokens_processed: u64,
    pub cache_hit_rate: f32,
    pub model_version: String,
}

/// Embedding provider configuration
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub provider_type: EmbeddingProviderType,
    pub model_name: String,
    pub max_tokens: usize,
    pub batch_size: usize,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
    pub use_cache: bool,
    pub cache_ttl_seconds: u64,
}

/// Types of embedding providers
#[derive(Debug, Clone, PartialEq)]
pub enum EmbeddingProviderType {
    /// Local ONNX model (CPU)
    LocalCpu,
    /// Local ONNX model (GPU)
    LocalGpu,
    /// OpenAI embeddings API
    OpenAI,
    /// Azure OpenAI embeddings
    AzureOpenAI,
    /// Local BGE-M3 model
    BgeM3,
    /// Custom provider
    Custom(String),
}

/// Embedding request with optional metadata
#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    pub text: String,
    pub context: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub priority: EmbeddingPriority,
}

/// Priority levels for embedding requests
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum EmbeddingPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Batch embedding request
#[derive(Debug, Clone)]
pub struct BatchEmbeddingRequest {
    pub requests: Vec<EmbeddingRequest>,
    pub batch_options: BatchOptions,
}

/// Options for batch processing
#[derive(Debug, Clone)]
pub struct BatchOptions {
    pub parallel_processing: bool,
    pub fail_fast: bool,
    pub progress_callback: Option<ProgressCallback>,
}

/// Progress callback type for batch operations
pub type ProgressCallback = Box<dyn Fn(usize, usize) + Send + Sync>;

/// Embedding response with metadata
#[derive(Debug, Clone)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
    pub processing_time_ms: u64,
    pub model_version: String,
    pub cache_hit: bool,
    pub token_count: usize,
}

/// Batch embedding response
#[derive(Debug, Clone)]
pub struct BatchEmbeddingResponse {
    pub responses: Vec<EmbeddingResponse>,
    pub total_processing_time_ms: u64,
    pub successful_count: usize,
    pub failed_count: usize,
    pub errors: Vec<(usize, String)>, // (index, error_message)
}

/// Mock embedding provider for testing
#[cfg(feature = "test-utils")]
pub struct MockEmbeddingProvider {
    pub dimensions: usize,
    pub model_name: String,
    pub responses: std::collections::VecDeque<ApplicationResult<Vec<f32>>>,
    pub call_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(feature = "test-utils")]
impl MockEmbeddingProvider {
    pub fn new(dimensions: usize) -> Self {
        Self {
            dimensions,
            model_name: "mock-embedding-model".to_string(),
            responses: std::collections::VecDeque::new(),
            call_count: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
    
    pub fn with_responses(mut self, responses: Vec<ApplicationResult<Vec<f32>>>) -> Self {
        self.responses = responses.into();
        self
    }
    
    pub fn call_count(&self) -> usize {
        self.call_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(feature = "test-utils")]
#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn generate_embedding(&self, _text: &str) -> ApplicationResult<Vec<f32>> {
        self.call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if let Some(response) = self.responses.front() {
            response.clone()
        } else {
            // Return random embedding for testing
            Ok((0..self.dimensions).map(|_| rand::random::<f32>()).collect())
        }
    }
    
    async fn generate_batch_embeddings(&self, texts: &[String]) -> ApplicationResult<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.generate_embedding(text).await?);
        }
        Ok(results)
    }
    
    fn embedding_dimensions(&self) -> usize {
        self.dimensions
    }
    
    fn model_identifier(&self) -> &str {
        &self.model_name
    }
    
    fn supports_batching(&self) -> bool {
        true
    }
    
    fn max_batch_size(&self) -> usize {
        100
    }
    
    async fn health_check(&self) -> ApplicationResult<ProviderHealth> {
        Ok(ProviderHealth {
            is_healthy: true,
            response_time_ms: 10,
            error_rate: 0.0,
            last_error: None,
            uptime_seconds: 3600,
        })
    }
    
    async fn get_metrics(&self) -> ApplicationResult<EmbeddingMetrics> {
        let call_count = self.call_count() as u64;
        Ok(EmbeddingMetrics {
            total_requests: call_count,
            successful_requests: call_count,
            failed_requests: 0,
            average_response_time_ms: 15.0,
            p95_response_time_ms: 25.0,
            p99_response_time_ms: 40.0,
            tokens_processed: call_count * 100, // Assume 100 tokens per request
            cache_hit_rate: 0.0,
            model_version: "mock-v1.0".to_string(),
        })
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider_type: EmbeddingProviderType::LocalCpu,
            model_name: "default-embedding-model".to_string(),
            max_tokens: 8192,
            batch_size: 32,
            timeout_seconds: 30,
            retry_attempts: 3,
            use_cache: true,
            cache_ttl_seconds: 3600,
        }
    }
}

impl Default for BatchOptions {
    fn default() -> Self {
        Self {
            parallel_processing: true,
            fail_fast: false,
            progress_callback: None,
        }
    }
}