//! Embedding Service Adapter
//!
//! Адаптер для интеграции с embedding services из AI crate

use std::sync::Arc;
use async_trait::async_trait;
use crate::{ApplicationResult, ApplicationError};
use crate::ports::EmbeddingProvider;
use domain::entities::embedding_vector::EmbeddingVector;

/// Adapter for embedding services from AI crate
pub struct EmbeddingServiceAdapter {
    /// CPU embedding service
    cpu_service: Arc<dyn CpuEmbeddingServiceTrait>,
    /// GPU embedding service (optional)
    gpu_service: Option<Arc<dyn GpuEmbeddingServiceTrait>>,
    /// Configuration
    config: EmbeddingAdapterConfig,
}

/// Configuration for embedding adapter
#[derive(Debug, Clone)]
pub struct EmbeddingAdapterConfig {
    pub prefer_gpu: bool,
    pub fallback_to_cpu: bool,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub batch_size: usize,
}

/// Trait abstraction for CPU embedding service
#[async_trait]
pub trait CpuEmbeddingServiceTrait: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error + Send + Sync>>;
    fn dimensions(&self) -> usize;
}

/// Trait abstraction for GPU embedding service
#[async_trait]
pub trait GpuEmbeddingServiceTrait: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error + Send + Sync>>;
    fn dimensions(&self) -> usize;
    async fn is_available(&self) -> bool;
}

impl EmbeddingServiceAdapter {
    pub fn new(
        cpu_service: Arc<dyn CpuEmbeddingServiceTrait>,
        gpu_service: Option<Arc<dyn GpuEmbeddingServiceTrait>>,
        config: EmbeddingAdapterConfig,
    ) -> Self {
        Self {
            cpu_service,
            gpu_service,
            config,
        }
    }

    pub fn cpu_only(cpu_service: Arc<dyn CpuEmbeddingServiceTrait>) -> Self {
        Self::new(
            cpu_service,
            None,
            EmbeddingAdapterConfig::default(),
        )
    }

    /// Check if GPU service is available and preferred
    async fn should_use_gpu(&self) -> bool {
        if !self.config.prefer_gpu {
            return false;
        }

        match &self.gpu_service {
            Some(gpu) => gpu.is_available().await,
            None => false,
        }
    }

    /// Generate embedding with fallback logic
    async fn embed_with_fallback(&self, text: &str) -> ApplicationResult<Vec<f32>> {
        if self.should_use_gpu().await {
            if let Some(ref gpu_service) = self.gpu_service {
                match gpu_service.embed(text).await {
                    Ok(embedding) => return Ok(embedding),
                    Err(e) => {
                        tracing::warn!("GPU embedding failed: {}, falling back to CPU", e);
                        if !self.config.fallback_to_cpu {
                            return Err(ApplicationError::infrastructure_with_source(
                                "GPU embedding failed and CPU fallback disabled",
                                e,
                            ));
                        }
                    }
                }
            }
        }

        // Use CPU service
        self.cpu_service
            .embed(text)
            .await
            .map_err(|e| ApplicationError::infrastructure_with_source("CPU embedding failed", e))
    }

    /// Generate batch embeddings with optimal service selection
    async fn embed_batch_with_fallback(&self, texts: &[String]) -> ApplicationResult<Vec<Vec<f32>>> {
        if self.should_use_gpu().await {
            if let Some(ref gpu_service) = self.gpu_service {
                match gpu_service.embed_batch(texts).await {
                    Ok(embeddings) => return Ok(embeddings),
                    Err(e) => {
                        tracing::warn!("GPU batch embedding failed: {}, falling back to CPU", e);
                        if !self.config.fallback_to_cpu {
                            return Err(ApplicationError::infrastructure_with_source(
                                "GPU batch embedding failed and CPU fallback disabled",
                                e,
                            ));
                        }
                    }
                }
            }
        }

        // Use CPU service
        self.cpu_service
            .embed_batch(texts)
            .await
            .map_err(|e| ApplicationError::infrastructure_with_source("CPU batch embedding failed", e))
    }
}

#[async_trait]
impl EmbeddingProvider for EmbeddingServiceAdapter {
    async fn generate_embedding(&self, text: &str) -> ApplicationResult<Vec<f32>> {
        if text.is_empty() {
            return Err(ApplicationError::validation("Text cannot be empty"));
        }

        if text.len() > 10_000 {
            return Err(ApplicationError::validation("Text too long for embedding"));
        }

        // Add timeout and retry logic
        let mut last_error = None;
        
        for attempt in 1..=self.config.max_retries {
            match tokio::time::timeout(
                std::time::Duration::from_secs(self.config.timeout_seconds),
                self.embed_with_fallback(text),
            )
            .await
            {
                Ok(Ok(embedding)) => {
                    tracing::debug!("Generated embedding with {} dimensions on attempt {}", embedding.len(), attempt);
                    return Ok(embedding);
                }
                Ok(Err(e)) => {
                    tracing::warn!("Embedding attempt {} failed: {}", attempt, e);
                    last_error = Some(e);
                }
                Err(_) => {
                    let timeout_error = ApplicationError::infrastructure("Embedding generation timeout");
                    tracing::warn!("Embedding attempt {} timed out", attempt);
                    last_error = Some(timeout_error);
                }
            }

            if attempt < self.config.max_retries {
                // Exponential backoff
                let delay = std::time::Duration::from_millis(100 * (2_u64.pow(attempt - 1)));
                tokio::time::sleep(delay).await;
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ApplicationError::infrastructure("Embedding generation failed after all retries")
        }))
    }

    async fn generate_embeddings_batch(&self, texts: &[String]) -> ApplicationResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        if texts.len() > self.config.batch_size {
            return Err(ApplicationError::validation(
                format!("Batch size {} exceeds limit {}", texts.len(), self.config.batch_size)
            ));
        }

        // Validate all texts
        for (i, text) in texts.iter().enumerate() {
            if text.is_empty() {
                return Err(ApplicationError::validation(format!("Text at index {} is empty", i)));
            }
            if text.len() > 10_000 {
                return Err(ApplicationError::validation(format!("Text at index {} is too long", i)));
            }
        }

        // Process batch with timeout and retry
        let mut last_error = None;
        
        for attempt in 1..=self.config.max_retries {
            match tokio::time::timeout(
                std::time::Duration::from_secs(self.config.timeout_seconds * 2), // Double timeout for batch
                self.embed_batch_with_fallback(texts),
            )
            .await
            {
                Ok(Ok(embeddings)) => {
                    tracing::debug!(
                        "Generated batch embeddings: {} texts, {} dimensions each, attempt {}",
                        embeddings.len(),
                        embeddings.first().map(|e| e.len()).unwrap_or(0),
                        attempt
                    );
                    return Ok(embeddings);
                }
                Ok(Err(e)) => {
                    tracing::warn!("Batch embedding attempt {} failed: {}", attempt, e);
                    last_error = Some(e);
                }
                Err(_) => {
                    let timeout_error = ApplicationError::infrastructure("Batch embedding generation timeout");
                    tracing::warn!("Batch embedding attempt {} timed out", attempt);
                    last_error = Some(timeout_error);
                }
            }

            if attempt < self.config.max_retries {
                let delay = std::time::Duration::from_millis(200 * (2_u64.pow(attempt - 1)));
                tokio::time::sleep(delay).await;
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ApplicationError::infrastructure("Batch embedding generation failed after all retries")
        }))
    }

    async fn get_embedding_dimensions(&self) -> ApplicationResult<usize> {
        if self.should_use_gpu().await {
            if let Some(ref gpu_service) = self.gpu_service {
                return Ok(gpu_service.dimensions());
            }
        }
        
        Ok(self.cpu_service.dimensions())
    }

    async fn health_check(&self) -> ApplicationResult<crate::ports::EmbeddingHealth> {
        let start_time = std::time::Instant::now();
        
        // Test embedding generation with a simple text
        let test_result = self.generate_embedding("test").await;
        let response_time = start_time.elapsed();
        
        let (is_healthy, last_error) = match test_result {
            Ok(_) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        
        let gpu_available = self.should_use_gpu().await;
        
        Ok(crate::ports::EmbeddingHealth {
            is_healthy,
            gpu_available,
            cpu_available: true, // CPU service is always available
            response_time_ms: response_time.as_millis() as u64,
            model_loaded: true, // Assume models are loaded if we can generate embeddings
            last_error,
            dimensions: self.get_embedding_dimensions().await.unwrap_or(0),
            throughput_texts_per_second: if is_healthy { 
                1000.0 / response_time.as_millis() as f64 
            } else { 
                0.0 
            },
        })
    }

    async fn get_supported_models(&self) -> ApplicationResult<Vec<String>> {
        // This would depend on the actual services
        Ok(vec![
            "cpu-embedding-model".to_string(),
            "gpu-embedding-model".to_string(),
        ])
    }

    async fn preload_model(&self, model_name: &str) -> ApplicationResult<()> {
        // Model preloading logic would go here
        tracing::info!("Preloading model: {}", model_name);
        Ok(())
    }
}

impl Default for EmbeddingAdapterConfig {
    fn default() -> Self {
        Self {
            prefer_gpu: true,
            fallback_to_cpu: true,
            timeout_seconds: 30,
            max_retries: 3,
            batch_size: 100,
        }
    }
}

impl EmbeddingAdapterConfig {
    pub fn cpu_only() -> Self {
        Self {
            prefer_gpu: false,
            fallback_to_cpu: true,
            ..Default::default()
        }
    }

    pub fn gpu_only() -> Self {
        Self {
            prefer_gpu: true,
            fallback_to_cpu: false,
            ..Default::default()
        }
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    pub fn with_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }
}