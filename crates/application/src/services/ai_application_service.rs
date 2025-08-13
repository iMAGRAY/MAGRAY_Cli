use anyhow::Result;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::use_cases::ai_use_cases::{
    AiServiceProvider, AiStatusRequest, AiStatusResponse, AiStatusUseCase, AiUseCaseFactory,
    InferenceRequest, InferenceResponse, InferenceUseCase, ListModelsRequest, ListModelsResponse,
    ListModelsUseCase, LoadModelRequest, LoadModelResponse, LoadModelUseCase,
};

use crate::ports::{cache_provider::CacheProvider, metrics_collector::MetricsCollector};

use crate::errors::ApplicationError;

// Import AI module types conditionally
#[cfg(any(feature = "cpu", feature = "gpu"))]
use ai::{CpuEmbeddingService, EmbeddingServiceTrait, ModelRegistry, OptimizedEmbeddingResult};

#[cfg(feature = "gpu")]
use ai::GpuEmbeddingService;

/// Application-level service statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceStats {
    pub total_requests: u64,
    pub total_tokens_processed: u64,
    pub average_latency_ms: f64,
    pub cache_hit_rate: f64,
}

#[cfg(feature = "reranking")]
use ai::{OptimizedQwen3RerankerService, RerankResult, RerankingService, RerankingServiceTrait};

// Mock implementations for testing
pub struct MockEmbeddingService;

impl MockEmbeddingService {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(any(feature = "cpu", feature = "gpu"))]
use async_trait::async_trait;

#[cfg(any(feature = "cpu", feature = "gpu"))]
#[async_trait]
impl EmbeddingServiceTrait for MockEmbeddingService {
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        Ok(vec![vec![0.1f32; 768]; texts.len()])
    }
}

pub struct MockRerankingService;

impl MockRerankingService {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(feature = "reranking")]
#[async_trait::async_trait]
impl RerankingServiceTrait for MockRerankingService {
    async fn rerank(
        &self,
        _query: &str,
        _documents: &[String],
        _top_k: Option<usize>,
    ) -> ai::Result<Vec<RerankResult>> {
        Ok(vec![])
    }
}

/// AI Application Service - coordinating layer for all AI operations
/// Implements the Application Service pattern from Domain-Driven Design
pub struct AiApplicationService {
    use_case_factory: Arc<AiUseCaseFactory>,
    cache_provider: Arc<dyn CacheProvider>,
    metrics_collector: Arc<dyn MetricsCollector>,

    // AI service components (optional based on features)
    #[cfg(feature = "embeddings")]
    embedding_service: Option<Arc<dyn EmbeddingServiceTrait>>,
    #[cfg(feature = "reranking")]
    reranking_service: Option<Arc<dyn RerankingServiceTrait>>,
    #[cfg(any(feature = "cpu", feature = "gpu"))]
    model_registry: Option<Arc<ModelRegistry>>,
}

impl AiApplicationService {
    pub fn new(
        cache_provider: Arc<dyn CacheProvider>,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Self {
        let service_provider = Arc::new(DefaultAiServiceProvider::new(
            #[cfg(feature = "embeddings")]
            None,
            #[cfg(feature = "reranking")]
            None,
            #[cfg(any(feature = "cpu", feature = "gpu"))]
            None,
        ));

        let use_case_factory = Arc::new(AiUseCaseFactory::new(
            cache_provider.clone(),
            metrics_collector.clone(),
            service_provider,
        ));

        Self {
            use_case_factory,
            cache_provider,
            metrics_collector,
            #[cfg(feature = "embeddings")]
            embedding_service: None,
            #[cfg(feature = "reranking")]
            reranking_service: None,
            #[cfg(any(feature = "cpu", feature = "gpu"))]
            model_registry: None,
        }
    }

    /// Initialize AI services based on available features
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing AI Application Service...");

        // Initialize model registry
        #[cfg(any(feature = "cpu", feature = "gpu"))]
        {
            let models_dir = std::path::PathBuf::from("models");
            let registry = Arc::new(ModelRegistry::new(models_dir));
            self.model_registry = Some(registry.clone());
            info!("Model registry initialized");
        }

        // Initialize embedding service based on available features
        #[cfg(feature = "embeddings")]
        {
            #[cfg(feature = "gpu")]
            {
                // Try to initialize GPU service first
                match self.initialize_gpu_embedding_service().await {
                    Ok(service) => {
                        self.embedding_service = Some(service);
                        info!("GPU embedding service initialized");
                    }
                    Err(e) => {
                        warn!(
                            "Failed to initialize GPU embedding service: {}, falling back to CPU",
                            e
                        );
                        self.embedding_service =
                            Some(self.initialize_cpu_embedding_service().await?);
                        info!("CPU embedding service initialized as fallback");
                    }
                }
            }

            #[cfg(not(feature = "gpu"))]
            {
                self.embedding_service = Some(self.initialize_cpu_embedding_service().await?);
                info!("CPU embedding service initialized");
            }
        }

        // Initialize reranking service
        #[cfg(feature = "reranking")]
        {
            match self.initialize_reranking_service().await {
                Ok(service) => {
                    self.reranking_service = Some(service);
                    info!("Reranking service initialized");
                }
                Err(e) => {
                    warn!("Failed to initialize reranking service: {}", e);
                }
            }
        }

        // Update use case factory with initialized services
        self.update_use_case_factory().await?;

        info!("AI Application Service initialization completed");
        Ok(())
    }

    /// List available AI models
    pub async fn list_models(&self, request: ListModelsRequest) -> Result<ListModelsResponse> {
        info!("Listing AI models with filter: {:?}", request.model_type);

        let use_case = self.use_case_factory.create_list_models_use_case();

        match use_case.execute(request).await {
            Ok(response) => {
                info!("Successfully listed {} models", response.models.len());
                Ok(response)
            }
            Err(e) => {
                error!("Failed to list models: {}", e);
                Err(e)
            }
        }
    }

    /// Load an AI model into memory
    pub async fn load_model(&self, request: LoadModelRequest) -> Result<LoadModelResponse> {
        info!("Loading model: {}", request.model_name);

        let use_case = self.use_case_factory.create_load_model_use_case();

        match use_case.execute(request).await {
            Ok(response) => {
                info!(
                    "Successfully loaded model: {} on device: {}",
                    response.model_name, response.device
                );
                Ok(response)
            }
            Err(e) => {
                error!("Failed to load model: {}", e);
                Err(e)
            }
        }
    }

    /// Run AI inference
    pub async fn run_inference(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        info!("Running inference with model: {}", request.model_name);

        let use_case = self.use_case_factory.create_inference_use_case();

        match use_case.execute(request).await {
            Ok(response) => {
                info!(
                    "Successfully completed inference in {:.2}ms",
                    response.processing_time_ms
                );
                Ok(response)
            }
            Err(e) => {
                error!("Failed to run inference: {}", e);
                Err(e)
            }
        }
    }

    /// Get AI system status
    pub async fn get_status(&self, request: AiStatusRequest) -> Result<AiStatusResponse> {
        info!("Getting AI system status");

        let use_case = self.use_case_factory.create_ai_status_use_case();

        match use_case.execute(request).await {
            Ok(response) => {
                info!(
                    "AI system status: {} models loaded, healthy: {}",
                    response.loaded_models_count, response.system_healthy
                );
                Ok(response)
            }
            Err(e) => {
                error!("Failed to get AI status: {}", e);
                Err(e)
            }
        }
    }

    /// Check if AI functionality is available
    pub fn is_ai_available(&self) -> bool {
        cfg!(any(feature = "cpu", feature = "gpu"))
    }

    /// Check if GPU acceleration is available
    pub fn is_gpu_available(&self) -> bool {
        cfg!(feature = "gpu")
    }

    /// Check if embedding functionality is available
    pub fn is_embedding_available(&self) -> bool {
        cfg!(feature = "embeddings")
    }

    /// Check if reranking functionality is available
    pub fn is_reranking_available(&self) -> bool {
        cfg!(feature = "reranking")
    }

    /// Get service health information
    pub async fn get_health_info(&self) -> Result<AiHealthInfo> {
        let mut health = AiHealthInfo {
            ai_available: self.is_ai_available(),
            gpu_available: self.is_gpu_available(),
            embedding_available: self.is_embedding_available(),
            reranking_available: self.is_reranking_available(),
            models_loaded: 0,
            memory_usage_mb: 0.0,
            warnings: Vec::new(),
        };

        #[cfg(any(feature = "cpu", feature = "gpu"))]
        {
            if let Some(ref registry) = self.model_registry {
                health.models_loaded = registry.get_available_models(None).len();
                // Calculate approximate memory usage - simple estimation
                health.memory_usage_mb = (registry.get_available_models(None).len() as f64) * 100.0;
            }
        }

        // Check for warnings
        if !self.is_ai_available() {
            health
                .warnings
                .push("AI functionality not available in this build".to_string());
        }

        if self.is_ai_available() && health.models_loaded == 0 {
            health
                .warnings
                .push("No AI models currently loaded".to_string());
        }

        #[cfg(feature = "embeddings")]
        {
            if self.embedding_service.is_none() {
                health
                    .warnings
                    .push("Embedding service not initialized".to_string());
            }
        }

        #[cfg(feature = "reranking")]
        {
            if self.reranking_service.is_none() {
                health
                    .warnings
                    .push("Reranking service not initialized".to_string());
            }
        }

        Ok(health)
    }

    // Private helper methods for service initialization

    #[cfg(feature = "embeddings")]
    async fn initialize_cpu_embedding_service(&self) -> Result<Arc<dyn EmbeddingServiceTrait>> {
        // Initialize CPU embedding service
        // This would use the actual CpuEmbeddingService from ai crate
        let service = Arc::new(MockEmbeddingService::new());
        Ok(service)
    }

    #[cfg(all(feature = "embeddings", feature = "gpu"))]
    async fn initialize_gpu_embedding_service(&self) -> Result<Arc<dyn EmbeddingServiceTrait>> {
        // Try to initialize GPU embedding service
        // This would use the actual GpuEmbeddingService from ai crate
        let service = Arc::new(MockEmbeddingService::new());
        Ok(service)
    }

    #[cfg(feature = "reranking")]
    async fn initialize_reranking_service(&self) -> Result<Arc<dyn RerankingServiceTrait>> {
        // Initialize reranking service
        // This would use the actual OptimizedQwen3RerankerService from ai crate
        let service = Arc::new(MockRerankingService::new());
        Ok(service)
    }

    async fn update_use_case_factory(&mut self) -> Result<()> {
        // Update the service provider with initialized services
        let service_provider = Arc::new(DefaultAiServiceProvider::new(
            #[cfg(feature = "embeddings")]
            self.embedding_service.clone(),
            #[cfg(feature = "reranking")]
            self.reranking_service.clone(),
            #[cfg(any(feature = "cpu", feature = "gpu"))]
            self.model_registry.clone(),
        ));

        self.use_case_factory = Arc::new(AiUseCaseFactory::new(
            self.cache_provider.clone(),
            self.metrics_collector.clone(),
            service_provider,
        ));

        Ok(())
    }
}

/// Health information for AI services
#[derive(Debug, Clone)]
pub struct AiHealthInfo {
    pub ai_available: bool,
    pub gpu_available: bool,
    pub embedding_available: bool,
    pub reranking_available: bool,
    pub models_loaded: usize,
    pub memory_usage_mb: f64,
    pub warnings: Vec<String>,
}

/// Default implementation of AiServiceProvider
struct DefaultAiServiceProvider {
    #[cfg(feature = "embeddings")]
    embedding_service: Option<Arc<dyn EmbeddingServiceTrait>>,
    #[cfg(feature = "reranking")]
    reranking_service: Option<Arc<dyn RerankingServiceTrait>>,
    #[cfg(any(feature = "cpu", feature = "gpu"))]
    model_registry: Option<Arc<ModelRegistry>>,
}

impl DefaultAiServiceProvider {
    pub fn new(
        #[cfg(feature = "embeddings")] embedding_service: Option<Arc<dyn EmbeddingServiceTrait>>,
        #[cfg(feature = "reranking")] reranking_service: Option<Arc<dyn RerankingServiceTrait>>,
        #[cfg(any(feature = "cpu", feature = "gpu"))] model_registry: Option<Arc<ModelRegistry>>,
    ) -> Self {
        Self {
            #[cfg(feature = "embeddings")]
            embedding_service,
            #[cfg(feature = "reranking")]
            reranking_service,
            #[cfg(any(feature = "cpu", feature = "gpu"))]
            model_registry,
        }
    }
}

impl AiServiceProvider for DefaultAiServiceProvider {
    #[cfg(feature = "embeddings")]
    fn get_embedding_service(&self) -> Option<Arc<dyn EmbeddingServiceTrait>> {
        self.embedding_service.clone()
    }

    #[cfg(feature = "reranking")]
    fn get_reranking_service(&self) -> Option<Arc<dyn RerankingServiceTrait>> {
        self.reranking_service.clone()
    }

    #[cfg(any(feature = "cpu", feature = "gpu"))]
    fn get_model_registry(&self) -> Option<Arc<ModelRegistry>> {
        self.model_registry.clone()
    }
}

// Mock services for testing and placeholder implementations

// MockEmbeddingService already defined above

#[cfg(feature = "embeddings")]

// Additional methods not in trait
impl MockEmbeddingService {
    pub async fn embed_query(&self, _query: &str) -> Result<Vec<f32>, ai::AiError> {
        // Mock implementation - returns a 768-dimensional embedding
        Ok(vec![0.1f32; 768])
    }

    pub fn get_stats(&self) -> ServiceStats {
        ServiceStats {
            total_requests: 0,
            total_tokens_processed: 0,
            average_latency_ms: 0.0,
            cache_hit_rate: 0.0,
        }
    }
}

// MockRerankingService already defined above

// Integration helper for DI container registration
pub struct AiServiceRegistration;

impl AiServiceRegistration {
    /// Register AI services in the DI container
    pub fn register_services<T: crate::ports::ServiceContainer>(
        container: &mut T,
        cache_provider: Arc<dyn CacheProvider>,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Result<()> {
        // Register AI Application Service
        let mut ai_service = AiApplicationService::new(cache_provider, metrics_collector);

        // Initialize in async context would be:
        // ai_service.initialize().await?;

        container.register_singleton(Arc::new(ai_service))?;

        info!("AI services registered in DI container");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "test-utils")]
    use crate::ports::cache_provider::MockCacheProvider;
    #[cfg(feature = "test-utils")]
    use crate::ports::metrics_collector::MockMetricsCollector;

    #[tokio::test]
    #[cfg(feature = "test-utils")]
    async fn test_ai_service_creation() {
        let cache_provider = Arc::new(MockCacheProvider::new());
        let metrics_collector = Arc::new(MockMetricsCollector::new());

        let service = AiApplicationService::new(cache_provider, metrics_collector);

        assert_eq!(
            service.is_ai_available(),
            cfg!(any(feature = "cpu", feature = "gpu"))
        );
        assert_eq!(service.is_gpu_available(), cfg!(feature = "gpu"));
    }

    #[tokio::test]
    #[cfg(feature = "test-utils")]
    async fn test_health_info() {
        let cache_provider = Arc::new(MockCacheProvider::new());
        let metrics_collector = Arc::new(MockMetricsCollector::new());

        let service = AiApplicationService::new(cache_provider, metrics_collector);

        let health = service
            .get_health_info()
            .await
            .expect("Operation should succeed");

        assert_eq!(
            health.ai_available,
            cfg!(any(feature = "cpu", feature = "gpu"))
        );
        assert_eq!(health.gpu_available, cfg!(feature = "gpu"));
        assert_eq!(health.embedding_available, cfg!(feature = "embeddings"));
        assert_eq!(health.reranking_available, cfg!(feature = "reranking"));
    }
}
