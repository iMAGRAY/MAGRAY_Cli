use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

// Import AI module types
#[cfg(any(feature = "cpu", feature = "gpu"))]
use ai::{
    AiError, CpuEmbeddingService, EmbeddingServiceTrait, ModelInfo, ModelRegistry,
    OptimizedEmbeddingResult, ServiceStats,
};

#[cfg(feature = "gpu")]
use ai::GpuEmbeddingService;

#[cfg(feature = "reranking")]
use ai::{OptimizedQwen3RerankerService, RerankResult, RerankingService, RerankingServiceTrait};

use crate::errors::ApplicationError;
use crate::ports::{
    cache_provider::{CacheProvider, CacheProviderExt},
    metrics_collector::MetricsCollector,
};

// Request/Response DTOs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListModelsRequest {
    pub model_type: Option<String>, // "embedding", "reranking", etc.
    pub include_loaded: bool,
    pub include_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListModelsResponse {
    pub models: Vec<ModelInfoDto>,
    pub total_count: usize,
    pub loaded_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfoDto {
    pub name: String,
    pub model_type: String,
    pub loaded: bool,
    pub device: Option<String>,
    pub path: Option<PathBuf>,
    pub size_mb: Option<f64>,
    pub description: Option<String>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadModelRequest {
    pub model_name: String,
    pub force_reload: bool,
    pub prefer_gpu: bool,
    pub custom_path: Option<PathBuf>,
    pub device_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadModelResponse {
    pub model_name: String,
    pub device: String,
    pub memory_usage_mb: f64,
    pub load_time_ms: u64,
    pub capabilities: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub model_name: String,
    pub input: String,
    pub batch_size: usize,
    pub top_k: Option<usize>, // For reranking
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub model_name: String,
    pub result_type: String, // "embedding", "reranking", "text"
    pub embedding: Option<Vec<f32>>,
    pub scores: Option<Vec<f32>>,
    pub text_result: Option<String>,
    pub processing_time_ms: f64,
    pub device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStatusRequest {
    pub include_models: bool,
    pub include_system_info: bool,
    pub include_performance_stats: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiStatusResponse {
    pub system_healthy: bool,
    pub loaded_models_count: usize,
    pub loaded_models: Vec<ModelInfoDto>,
    pub total_memory_usage_mb: Option<f64>,
    pub gpu_available: Option<bool>,
    pub onnx_version: Option<String>,
    pub warnings: Option<Vec<String>>,
    pub performance_stats: Option<PerformanceStatsDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStatsDto {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: f64,
    pub cache_hit_rate: f64,
}

// Use Cases
pub struct ListModelsUseCase {
    #[cfg(any(feature = "cpu", feature = "gpu"))]
    model_registry: Arc<ModelRegistry>,
    cache_provider: Arc<dyn CacheProvider>,
    metrics_collector: Arc<dyn MetricsCollector>,
}

impl ListModelsUseCase {
    pub fn new(
        #[cfg(any(feature = "cpu", feature = "gpu"))] model_registry: Arc<ModelRegistry>,
        cache_provider: Arc<dyn CacheProvider>,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self {
            #[cfg(any(feature = "cpu", feature = "gpu"))]
            model_registry,
            cache_provider,
            metrics_collector,
        }
    }

    pub async fn execute(&self, request: ListModelsRequest) -> Result<ListModelsResponse> {
        let start_time = std::time::Instant::now();

        // Collect metrics
        self.metrics_collector
            .increment_counter("ai_list_models_requests_total", 1, None)
            .await?;

        #[cfg(any(feature = "cpu", feature = "gpu"))]
        {
            let cache_key = format!("models_list_{:?}", request);

            // Try cache first
            if let Ok(Some(cached_value)) = self.cache_provider.get_raw(&cache_key).await {
                if let Ok(response) = serde_json::from_value::<ListModelsResponse>(cached_value) {
                    self.metrics_collector
                        .increment_counter("ai_list_models_cache_hits", 1, None)
                        .await?;
                    return Ok(response);
                }
            }

            let mut models = Vec::new();
            let available_models = self.model_registry.get_available_models(None);
            // For now, treat all available models as loaded (registry doesn't distinguish)
            let loaded_models = available_models.clone();

            // Process loaded models
            if request.include_loaded {
                for model_info in loaded_models {
                    if let Some(ref filter_type) = request.model_type {
                        if format!("{:?}", model_info.model_type) != *filter_type {
                            continue;
                        }
                    }

                    models.push(ModelInfoDto {
                        name: model_info.name.clone(),
                        model_type: format!("{:?}", model_info.model_type),
                        loaded: true,
                        device: Some("CPU/GPU".to_string()),
                        path: None,
                        size_mb: None,
                        description: Some(model_info.description.clone()),
                        capabilities: Vec::new(),
                    });
                }
            }

            // Process available models
            if request.include_available {
                for model_info in available_models {
                    if let Some(ref filter_type) = request.model_type {
                        if format!("{:?}", model_info.model_type) != *filter_type {
                            continue;
                        }
                    }

                    // Skip if already in loaded models
                    if models.iter().any(|m| m.name == model_info.name) {
                        continue;
                    }

                    models.push(ModelInfoDto {
                        name: model_info.name.clone(),
                        model_type: format!("{:?}", model_info.model_type),
                        loaded: false,
                        device: None,
                        path: None,
                        size_mb: None,
                        description: Some(model_info.description.clone()),
                        capabilities: Vec::new(),
                    });
                }
            }

            let loaded_count = models.iter().filter(|m| m.loaded).count();

            let response = ListModelsResponse {
                total_count: models.len(),
                loaded_count,
                models,
            };

            // Cache the result
            if let Ok(cached_json_value) = serde_json::to_value(&response) {
                let _ = self
                    .cache_provider
                    .set_raw(&cache_key, cached_json_value, Some(60))
                    .await; // Cache for 1 minute
            }

            // Record metrics
            let duration = start_time.elapsed().as_millis() as f64;
            self.metrics_collector
                .record_histogram("ai_list_models_duration_ms", duration, None)
                .await?;

            Ok(response)
        }

        #[cfg(not(any(feature = "cpu", feature = "gpu")))]
        {
            let _ = request;
            Err(anyhow::anyhow!(
                "AI functionality not available in this build"
            ))
        }
    }
}

pub struct LoadModelUseCase {
    #[cfg(feature = "embeddings")]
    embedding_service: Option<Arc<dyn EmbeddingServiceTrait>>,
    #[cfg(feature = "reranking")]
    reranking_service: Option<Arc<dyn RerankingServiceTrait>>,
    cache_provider: Arc<dyn CacheProvider>,
    metrics_collector: Arc<dyn MetricsCollector>,
}

impl LoadModelUseCase {
    pub fn new(
        cache_provider: Arc<dyn CacheProvider>,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self {
            #[cfg(feature = "embeddings")]
            embedding_service: None,
            #[cfg(feature = "reranking")]
            reranking_service: None,
            cache_provider,
            metrics_collector,
        }
    }

    #[cfg(feature = "embeddings")]
    pub fn with_embedding_service(mut self, service: Arc<dyn EmbeddingServiceTrait>) -> Self {
        self.embedding_service = Some(service);
        self
    }

    #[cfg(feature = "reranking")]
    pub fn with_reranking_service(mut self, service: Arc<dyn RerankingServiceTrait>) -> Self {
        self.reranking_service = Some(service);
        self
    }

    pub async fn execute(&self, request: LoadModelRequest) -> Result<LoadModelResponse> {
        let start_time = std::time::Instant::now();

        let mut load_tags = std::collections::HashMap::new();
        load_tags.insert("model".to_string(), request.model_name.clone());

        self.metrics_collector
            .increment_counter("ai_load_model_requests_total", 1, Some(&load_tags))
            .await?;

        // Placeholder implementation - in real system would load actual models
        #[cfg(any(feature = "cpu", feature = "gpu"))]
        {
            // Simulate model loading
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            let device = if request.prefer_gpu && cfg!(feature = "gpu") {
                "GPU".to_string()
            } else {
                "CPU".to_string()
            };

            let load_time_ms = start_time.elapsed().as_millis() as u64;

            let response = LoadModelResponse {
                model_name: request.model_name.clone(),
                device,
                memory_usage_mb: 256.0, // Placeholder
                load_time_ms,
                capabilities: Some(vec!["embedding".to_string(), "inference".to_string()]),
            };

            // Record metrics
            let mut success_tags = std::collections::HashMap::new();
            success_tags.insert("model".to_string(), request.model_name.clone());

            self.metrics_collector
                .increment_counter("ai_load_model_success_total", 1, Some(&success_tags))
                .await?;
            self.metrics_collector
                .record_histogram(
                    "ai_load_model_duration_ms",
                    load_time_ms as f64,
                    Some(&success_tags),
                )
                .await?;

            Ok(response)
        }

        #[cfg(not(any(feature = "cpu", feature = "gpu")))]
        {
            let _ = request;
            Err(anyhow::anyhow!(
                "AI functionality not available in this build"
            ))
        }
    }
}

pub struct InferenceUseCase {
    #[cfg(feature = "embeddings")]
    embedding_service: Option<Arc<dyn EmbeddingServiceTrait>>,
    #[cfg(feature = "reranking")]
    reranking_service: Option<Arc<dyn RerankingServiceTrait>>,
    cache_provider: Arc<dyn CacheProvider>,
    metrics_collector: Arc<dyn MetricsCollector>,
}

impl InferenceUseCase {
    pub fn new(
        cache_provider: Arc<dyn CacheProvider>,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self {
            #[cfg(feature = "embeddings")]
            embedding_service: None,
            #[cfg(feature = "reranking")]
            reranking_service: None,
            cache_provider,
            metrics_collector,
        }
    }

    #[cfg(feature = "embeddings")]
    pub fn with_embedding_service(mut self, service: Arc<dyn EmbeddingServiceTrait>) -> Self {
        self.embedding_service = Some(service);
        self
    }

    #[cfg(feature = "reranking")]
    pub fn with_reranking_service(mut self, service: Arc<dyn RerankingServiceTrait>) -> Self {
        self.reranking_service = Some(service);
        self
    }

    pub async fn execute(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let start_time = std::time::Instant::now();

        let mut request_tags = std::collections::HashMap::new();
        request_tags.insert("model".to_string(), request.model_name.clone());

        self.metrics_collector
            .increment_counter("ai_inference_requests_total", 1, Some(&request_tags))
            .await?;

        // Try cache first for identical requests
        let cache_key = format!(
            "inference_{}_{}",
            request.model_name,
            format!("{:x}", md5::compute(&request.input))
        );

        if let Ok(Some(cached_value)) = self.cache_provider.get_raw(&cache_key).await {
            if let Ok(response) = serde_json::from_value::<InferenceResponse>(cached_value) {
                let mut cache_tags = std::collections::HashMap::new();
                cache_tags.insert("model".to_string(), request.model_name.clone());

                self.metrics_collector
                    .increment_counter("ai_inference_cache_hits", 1, Some(&cache_tags))
                    .await?;
                return Ok(response);
            }
        }

        // Determine inference type based on model name
        let result = if request.model_name.contains("embed") {
            self.handle_embedding_inference(&request).await
        } else if request.model_name.contains("rerank") {
            self.handle_reranking_inference(&request).await
        } else {
            self.handle_text_inference(&request).await
        };

        let processing_time_ms = start_time.elapsed().as_millis() as f64;

        match result {
            Ok(mut response) => {
                response.processing_time_ms = processing_time_ms;

                // Cache successful results
                if let Ok(cached_json_value) = serde_json::to_value(&response) {
                    let _ = self
                        .cache_provider
                        .set_raw(&cache_key, cached_json_value, Some(300))
                        .await; // Cache for 5 minutes
                }

                // Record success metrics
                let mut tags = std::collections::HashMap::new();
                tags.insert("model".to_string(), request.model_name.clone());

                self.metrics_collector
                    .increment_counter("ai_inference_success_total", 1, Some(&tags))
                    .await?;
                self.metrics_collector
                    .record_histogram(
                        "ai_inference_duration_ms",
                        processing_time_ms as f64,
                        Some(&tags),
                    )
                    .await?;

                Ok(response)
            }
            Err(e) => {
                let mut error_tags = std::collections::HashMap::new();
                error_tags.insert("model".to_string(), request.model_name.clone());

                self.metrics_collector
                    .increment_counter("ai_inference_errors_total", 1, Some(&error_tags))
                    .await?;
                Err(e)
            }
        }
    }

    #[cfg(feature = "embeddings")]
    async fn handle_embedding_inference(
        &self,
        request: &InferenceRequest,
    ) -> Result<InferenceResponse> {
        if let Some(ref embedding_service) = self.embedding_service {
            // Use embedding service to generate embeddings
            // Placeholder implementation
            let embedding = vec![0.1f32; 768]; // Mock 768-dimensional embedding

            Ok(InferenceResponse {
                model_name: request.model_name.clone(),
                result_type: "embedding".to_string(),
                embedding: Some(embedding),
                scores: None,
                text_result: None,
                processing_time_ms: 0.0, // Will be set by caller
                device: Some("CPU".to_string()),
            })
        } else {
            Err(anyhow::anyhow!("Embedding service not available"))
        }
    }

    #[cfg(not(feature = "embeddings"))]
    async fn handle_embedding_inference(
        &self,
        _request: &InferenceRequest,
    ) -> Result<InferenceResponse> {
        Err(anyhow::anyhow!(
            "Embedding functionality not available in this build"
        ))
    }

    #[cfg(feature = "reranking")]
    async fn handle_reranking_inference(
        &self,
        request: &InferenceRequest,
    ) -> Result<InferenceResponse> {
        if let Some(ref reranking_service) = self.reranking_service {
            // Mock reranking scores
            let scores = vec![0.9, 0.7, 0.5, 0.3];

            Ok(InferenceResponse {
                model_name: request.model_name.clone(),
                result_type: "reranking".to_string(),
                embedding: None,
                scores: Some(scores),
                text_result: None,
                processing_time_ms: 0.0, // Will be set by caller
                device: Some("CPU".to_string()),
            })
        } else {
            Err(anyhow::anyhow!("Reranking service not available"))
        }
    }

    #[cfg(not(feature = "reranking"))]
    async fn handle_reranking_inference(
        &self,
        _request: &InferenceRequest,
    ) -> Result<InferenceResponse> {
        Err(anyhow::anyhow!(
            "Reranking functionality not available in this build"
        ))
    }

    async fn handle_text_inference(&self, request: &InferenceRequest) -> Result<InferenceResponse> {
        // Mock text inference
        Ok(InferenceResponse {
            model_name: request.model_name.clone(),
            result_type: "text".to_string(),
            embedding: None,
            scores: None,
            text_result: Some(format!(
                "Generated response for: {}",
                request.input.chars().take(50).collect::<String>()
            )),
            processing_time_ms: 0.0, // Will be set by caller
            device: Some("CPU".to_string()),
        })
    }
}

pub struct AiStatusUseCase {
    #[cfg(any(feature = "cpu", feature = "gpu"))]
    model_registry: Option<Arc<ModelRegistry>>,
    cache_provider: Arc<dyn CacheProvider>,
    metrics_collector: Arc<dyn MetricsCollector>,
}

impl AiStatusUseCase {
    pub fn new(
        cache_provider: Arc<dyn CacheProvider>,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self {
            #[cfg(any(feature = "cpu", feature = "gpu"))]
            model_registry: None,
            cache_provider,
            metrics_collector,
        }
    }

    #[cfg(any(feature = "cpu", feature = "gpu"))]
    pub fn with_model_registry(mut self, registry: Arc<ModelRegistry>) -> Self {
        self.model_registry = Some(registry);
        self
    }

    pub async fn execute(&self, request: AiStatusRequest) -> Result<AiStatusResponse> {
        self.metrics_collector
            .increment_counter("ai_status_requests_total", 1, None)
            .await?;

        let mut loaded_models = Vec::new();
        let mut loaded_count = 0;
        let mut total_memory_mb: Option<f64> = None;

        #[cfg(any(feature = "cpu", feature = "gpu"))]
        {
            if let Some(ref registry) = self.model_registry {
                let models = registry.get_available_models(None);
                loaded_count = models.len();

                if request.include_models {
                    for model_info in models {
                        loaded_models.push(ModelInfoDto {
                            name: model_info.name.clone(),
                            model_type: format!("{:?}", model_info.model_type),
                            loaded: true,
                            device: Some("CPU/GPU".to_string()), // ModelInfo doesn't have device field
                            path: None,    // ModelInfo doesn't have path field
                            size_mb: None, // ModelInfo doesn't have size_mb field
                            description: Some(model_info.description.clone()),
                            capabilities: Vec::new(), // ModelInfo doesn't have capabilities field
                        });
                    }
                }

                // Calculate total memory usage
                total_memory_mb = Some(loaded_models.iter().filter_map(|m| m.size_mb).sum());
            }
        }

        let gpu_available = cfg!(feature = "gpu");
        let system_healthy = true; // Placeholder health check
        let onnx_version = Some("1.16.0".to_string()); // Placeholder

        let mut warnings = Vec::new();

        #[cfg(not(any(feature = "cpu", feature = "gpu")))]
        {
            warnings.push("AI functionality not available in this build".to_string());
        }

        if loaded_count == 0 && cfg!(any(feature = "cpu", feature = "gpu")) {
            warnings.push("No AI models currently loaded".to_string());
        }

        let performance_stats = if request.include_performance_stats {
            Some(PerformanceStatsDto {
                total_requests: 0,             // Would come from metrics collector
                successful_requests: 0,        // Would come from metrics collector
                failed_requests: 0,            // Would come from metrics collector
                average_response_time_ms: 0.0, // Would come from metrics collector
                cache_hit_rate: 0.0,           // Would come from cache provider
            })
        } else {
            None
        };

        Ok(AiStatusResponse {
            system_healthy,
            loaded_models_count: loaded_count,
            loaded_models,
            total_memory_usage_mb: total_memory_mb,
            gpu_available: Some(gpu_available),
            onnx_version,
            warnings: if warnings.is_empty() {
                None
            } else {
                Some(warnings)
            },
            performance_stats,
        })
    }
}

// Helper trait for service injection
pub trait AiServiceProvider: Send + Sync {
    #[cfg(feature = "embeddings")]
    fn get_embedding_service(&self) -> Option<Arc<dyn EmbeddingServiceTrait>>;

    #[cfg(feature = "reranking")]
    fn get_reranking_service(&self) -> Option<Arc<dyn RerankingServiceTrait>>;

    #[cfg(any(feature = "cpu", feature = "gpu"))]
    fn get_model_registry(&self) -> Option<Arc<ModelRegistry>>;
}

// Factory for creating use cases with proper dependencies
pub struct AiUseCaseFactory {
    cache_provider: Arc<dyn CacheProvider>,
    metrics_collector: Arc<dyn MetricsCollector>,
    service_provider: Arc<dyn AiServiceProvider>,
}

impl AiUseCaseFactory {
    pub fn new(
        cache_provider: Arc<dyn CacheProvider>,
        metrics_collector: Arc<dyn MetricsCollector>,
        service_provider: Arc<dyn AiServiceProvider>,
    ) -> Self {
        Self {
            cache_provider,
            metrics_collector,
            service_provider,
        }
    }

    pub fn create_list_models_use_case(&self) -> ListModelsUseCase {
        ListModelsUseCase::new(
            #[cfg(any(feature = "cpu", feature = "gpu"))]
            self.service_provider
                .get_model_registry()
                .unwrap_or_else(|| {
                    // Create a default model registry if not provided
                    let models_dir = std::path::PathBuf::from("models");
                    Arc::new(ModelRegistry::new(models_dir))
                }),
            self.cache_provider.clone(),
            self.metrics_collector.clone(),
        )
    }

    pub fn create_load_model_use_case(&self) -> LoadModelUseCase {
        let mut use_case =
            LoadModelUseCase::new(self.cache_provider.clone(), self.metrics_collector.clone());

        #[cfg(feature = "embeddings")]
        {
            if let Some(service) = self.service_provider.get_embedding_service() {
                use_case = use_case.with_embedding_service(service);
            }
        }

        #[cfg(feature = "reranking")]
        {
            if let Some(service) = self.service_provider.get_reranking_service() {
                use_case = use_case.with_reranking_service(service);
            }
        }

        use_case
    }

    pub fn create_inference_use_case(&self) -> InferenceUseCase {
        let mut use_case =
            InferenceUseCase::new(self.cache_provider.clone(), self.metrics_collector.clone());

        #[cfg(feature = "embeddings")]
        {
            if let Some(service) = self.service_provider.get_embedding_service() {
                use_case = use_case.with_embedding_service(service);
            }
        }

        #[cfg(feature = "reranking")]
        {
            if let Some(service) = self.service_provider.get_reranking_service() {
                use_case = use_case.with_reranking_service(service);
            }
        }

        use_case
    }

    pub fn create_ai_status_use_case(&self) -> AiStatusUseCase {
        let mut use_case =
            AiStatusUseCase::new(self.cache_provider.clone(), self.metrics_collector.clone());

        #[cfg(any(feature = "cpu", feature = "gpu"))]
        {
            if let Some(registry) = self.service_provider.get_model_registry() {
                use_case = use_case.with_model_registry(registry);
            }
        }

        use_case
    }
}
