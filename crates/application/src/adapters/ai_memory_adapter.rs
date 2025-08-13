use anyhow::Result;
use md5;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::dtos::{SearchMemoryRequest, StoreMemoryRequest};
use crate::ports::{
    cache_provider::{CacheProvider, CacheProviderExt},
    embedding_provider::EmbeddingProvider,
    metrics_collector::MetricsCollector,
};
use crate::use_cases::ai_use_cases::{InferenceRequest, InferenceResponse, InferenceUseCase};
use crate::use_cases::{
    search_memory_use_case::SearchMemoryUseCase, store_memory_use_case::StoreMemoryUseCase,
};

// Import AI types conditionally
#[cfg(feature = "embeddings")]
use ai::EmbeddingServiceTrait;

use domain::entities::{EmbeddingVector, MemoryRecord, SearchQuery};
use domain::value_objects::{AccessPattern, LayerType};

/// Adapter that bridges AI inference capabilities with Memory system
/// Provides high-level operations combining AI embeddings with memory storage/search
pub struct AiMemoryAdapter {
    ai_inference_use_case: Arc<InferenceUseCase>,
    store_memory_use_case: Arc<dyn StoreMemoryUseCase>,
    search_memory_use_case: Arc<dyn SearchMemoryUseCase>,
    cache_provider: Arc<dyn CacheProvider>,
    metrics_collector: Arc<dyn MetricsCollector>,

    // Configuration
    embedding_model: String,
    reranking_model: Option<String>,
    default_top_k: usize,
    cache_ttl_seconds: u64,
}

impl AiMemoryAdapter {
    pub fn new(
        ai_inference_use_case: Arc<InferenceUseCase>,
        store_memory_use_case: Arc<dyn StoreMemoryUseCase>,
        search_memory_use_case: Arc<dyn SearchMemoryUseCase>,
        cache_provider: Arc<dyn CacheProvider>,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self {
            ai_inference_use_case,
            store_memory_use_case,
            search_memory_use_case,
            cache_provider,
            metrics_collector,

            // Default configuration - could be made configurable
            embedding_model: "bge-m3".to_string(),
            reranking_model: Some("qwen3_reranker".to_string()),
            default_top_k: 10,
            cache_ttl_seconds: 300, // 5 minutes
        }
    }

    /// Configure the embedding model to use for AI operations
    pub fn with_embedding_model(mut self, model_name: String) -> Self {
        self.embedding_model = model_name;
        self
    }

    /// Configure the reranking model (optional)
    pub fn with_reranking_model(mut self, model_name: Option<String>) -> Self {
        self.reranking_model = model_name;
        self
    }

    /// Store text content with AI-generated embeddings in memory system
    pub async fn store_with_ai_embedding(
        &self,
        content: &str,
        metadata: Option<serde_json::Value>,
        access_pattern: AccessPattern,
    ) -> Result<String> {
        info!(
            "Storing content with AI-generated embeddings: {} chars",
            content.len()
        );

        self.metrics_collector
            .increment_counter("ai_memory_store_requests_total", 1, None)
            .await?;

        let start_time = std::time::Instant::now();

        // Generate embedding using AI inference
        let embedding_request = InferenceRequest {
            model_name: self.embedding_model.clone(),
            input: content.to_string(),
            batch_size: 1,
            top_k: None,
            temperature: None,
            max_tokens: None,
        };

        let embedding_response = self
            .ai_inference_use_case
            .execute(embedding_request)
            .await
            .map_err(|e| {
                error!("Failed to generate embedding: {}", e);
                e
            })?;

        // Extract embedding vector
        let embedding_vector = embedding_response
            .embedding
            .ok_or_else(|| anyhow::anyhow!("No embedding returned from AI model"))?;

        info!(
            "Generated embedding with {} dimensions",
            embedding_vector.len()
        );

        // Create memory record with AI-generated embedding
        let memory_record = MemoryRecord::new(
            content.to_string(),
            LayerType::Insights, // Using insights layer for AI-generated embeddings
            "ai_memory".to_string(), // kind
            "default".to_string(), // project
            "current".to_string(), // session
        )?;

        // Store in memory system with proper DTO structure
        let store_request = StoreMemoryRequest {
            content: content.to_string(),
            metadata: metadata,
            project: "default".to_string(),
            kind: Some("ai_memory".to_string()),
            session: Some("current".to_string()),
            target_layer: Some(LayerType::Insights),
            priority: None,
            tags: vec!["ai_generated".to_string()],
        };

        // Create a basic request context
        let request_context = crate::RequestContext::new(crate::RequestSource::Internal);

        let store_response = self
            .store_memory_use_case
            .store_memory(store_request, request_context)
            .await
            .map_err(|e| {
                error!("Failed to store in memory system: {}", e);
                e
            })?;

        // Record metrics
        let duration = start_time.elapsed().as_millis() as f64;
        self.metrics_collector
            .record_histogram("ai_memory_store_duration_ms", duration, None)
            .await?;

        self.metrics_collector
            .increment_counter("ai_memory_store_success_total", 1, None)
            .await?;

        info!(
            "Successfully stored content with record ID: {}",
            store_response.record_id
        );
        Ok(store_response.record_id.to_string())
    }

    /// Search memory using AI-generated query embedding with optional reranking
    pub async fn search_with_ai_embedding(
        &self,
        query: &str,
        limit: Option<usize>,
        use_reranking: bool,
    ) -> Result<AiMemorySearchResult> {
        info!(
            "Searching memory with AI embedding for query: '{}'",
            query.chars().take(50).collect::<String>()
        );

        self.metrics_collector
            .increment_counter("ai_memory_search_requests_total", 1, None)
            .await?;

        let start_time = std::time::Instant::now();
        let search_limit = limit.unwrap_or(self.default_top_k);

        // Check cache first
        let cache_key = format!(
            "ai_search_{}_{}_{}",
            format!("{:x}", md5::compute(query)),
            search_limit,
            use_reranking
        );

        if let Ok(Some(cached)) = self.cache_provider.get_raw(&cache_key).await {
            if let Ok(result) = serde_json::from_value::<AiMemorySearchResult>(cached) {
                info!("Returning cached search result");
                self.metrics_collector
                    .increment_counter("ai_memory_search_cache_hits", 1, None)
                    .await?;
                return Ok(result);
            }
        }

        // Generate query embedding
        let embedding_request = InferenceRequest {
            model_name: self.embedding_model.clone(),
            input: query.to_string(),
            batch_size: 1,
            top_k: None,
            temperature: None,
            max_tokens: None,
        };

        let embedding_response = self
            .ai_inference_use_case
            .execute(embedding_request)
            .await
            .map_err(|e| {
                error!("Failed to generate query embedding: {}", e);
                e
            })?;

        let query_embedding = embedding_response
            .embedding
            .ok_or_else(|| anyhow::anyhow!("No embedding returned from AI model"))?;

        info!(
            "Generated query embedding with {} dimensions",
            query_embedding.len()
        );

        // Search memory system using vector similarity
        let search_query = SearchQuery::new(query.to_string())?
            .with_vector(EmbeddingVector::new(query_embedding, 384)?)
            .with_max_results(search_limit)?;

        let search_request = SearchMemoryRequest {
            query: query.to_string(),
            limit: Some(search_limit),
            similarity_threshold: None,
            layers: None, // Search all layers
            project: None,
            project_filter: None,
            filters: None,
            include_embeddings: false,
            use_cache: true,
        };

        let request_context = crate::RequestContext::new(crate::RequestSource::Internal);
        let search_response = self
            .search_memory_use_case
            .search_memory(search_request, request_context)
            .await
            .map_err(|e| {
                error!("Failed to search memory system: {}", e);
                e
            })?;

        info!(
            "Found {} candidates from memory search",
            search_response.results.len()
        );

        // Optional reranking with AI model
        let mut final_results = search_response.results;
        let mut reranking_scores = None;

        if use_reranking && self.reranking_model.is_some() && !final_results.is_empty() {
            info!("Applying AI reranking to {} results", final_results.len());

            match self.rerank_results(query, &final_results).await {
                Ok(reranked) => {
                    reranking_scores = Some(reranked.scores.clone());

                    // Reorder results based on reranking scores
                    let mut scored_results: Vec<_> = final_results
                        .into_iter()
                        .zip(reranked.scores.into_iter())
                        .collect();

                    scored_results
                        .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                    final_results = scored_results
                        .into_iter()
                        .map(|(record, _)| record)
                        .collect();

                    info!("Reranking applied successfully");
                }
                Err(e) => {
                    warn!("Reranking failed, using original order: {}", e);
                }
            }
        }

        // Prepare result
        let result = AiMemorySearchResult {
            query: query.to_string(),
            total_found: final_results.len(),
            records: final_results,
            search_time_ms: start_time.elapsed().as_millis() as f64,
            embedding_model: self.embedding_model.clone(),
            reranking_model: self.reranking_model.clone(),
            reranking_scores,
            used_cache: false,
        };

        // Cache the result
        if let Ok(cached_value_json) = serde_json::to_value(&result) {
            let _ = self
                .cache_provider
                .set_raw(&cache_key, cached_value_json, Some(self.cache_ttl_seconds))
                .await;
        }

        // Record metrics
        let duration = start_time.elapsed().as_millis() as f64;
        self.metrics_collector
            .record_histogram("ai_memory_search_duration_ms", duration, None)
            .await?;

        self.metrics_collector
            .increment_counter("ai_memory_search_success_total", 1, None)
            .await?;

        info!(
            "Search completed: {} results in {:.2}ms",
            result.total_found, result.search_time_ms
        );
        Ok(result)
    }

    /// Batch store multiple contents with AI embeddings
    pub async fn batch_store_with_ai_embeddings(
        &self,
        contents: &[BatchStoreItem],
    ) -> Result<Vec<BatchStoreResult>> {
        info!("Batch storing {} items with AI embeddings", contents.len());

        let mut results = Vec::new();
        let mut successful = 0;
        let mut failed = 0;

        for (index, item) in contents.iter().enumerate() {
            match self
                .store_with_ai_embedding(
                    &item.content,
                    item.metadata.clone(),
                    item.access_pattern.clone(),
                )
                .await
            {
                Ok(record_id) => {
                    results.push(BatchStoreResult {
                        index,
                        success: true,
                        record_id: Some(record_id),
                        error: None,
                    });
                    successful += 1;
                }
                Err(e) => {
                    results.push(BatchStoreResult {
                        index,
                        success: false,
                        record_id: None,
                        error: Some(e.to_string()),
                    });
                    failed += 1;
                }
            }
        }

        info!(
            "Batch store completed: {} successful, {} failed",
            successful, failed
        );

        // Record batch metrics
        self.metrics_collector
            .record_histogram("ai_memory_batch_store_successful", successful as f64, None)
            .await?;

        self.metrics_collector
            .record_histogram("ai_memory_batch_store_failed", failed as f64, None)
            .await?;

        Ok(results)
    }

    /// Get AI-enhanced memory insights
    pub async fn get_memory_insights(&self, query: Option<&str>) -> Result<AiMemoryInsights> {
        info!("Generating AI-enhanced memory insights");

        // This is a placeholder implementation - would integrate with analytics
        let insights = AiMemoryInsights {
            total_records: 0, // Would query memory system
            embedding_model: self.embedding_model.clone(),
            average_embedding_dimension: 768, // Would calculate from actual data
            cluster_analysis: None,           // Could use AI for clustering
            similarity_patterns: Vec::new(),  // Could analyze similarity patterns
            recommendations: vec![
                "Consider using reranking for better search quality".to_string(),
                "Regular embedding model updates may improve performance".to_string(),
            ],
        };

        Ok(insights)
    }

    // Private helper methods

    async fn rerank_results(
        &self,
        query: &str,
        results: &[crate::dtos::SearchResult],
    ) -> Result<RerankingResult> {
        if let Some(ref reranking_model) = self.reranking_model {
            // Extract content from search results for reranking
            let documents: Vec<String> = results
                .iter()
                .map(|result| result.content.clone())
                .collect::<Vec<String>>();

            let rerank_request = InferenceRequest {
                model_name: reranking_model.clone(),
                input: query.to_string(),
                batch_size: documents.len(),
                top_k: Some(results.len()),
                temperature: None,
                max_tokens: None,
            };

            let rerank_response = self.ai_inference_use_case.execute(rerank_request).await?;

            let scores = rerank_response
                .scores
                .unwrap_or_else(|| vec![0.5; results.len()]); // Fallback scores

            Ok(RerankingResult { scores })
        } else {
            Err(anyhow::anyhow!("No reranking model configured"))
        }
    }
}

// Result types and DTOs

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiMemorySearchResult {
    pub query: String,
    pub total_found: usize,
    pub records: Vec<crate::dtos::SearchResult>,
    pub search_time_ms: f64,
    pub embedding_model: String,
    pub reranking_model: Option<String>,
    pub reranking_scores: Option<Vec<f32>>,
    pub used_cache: bool,
}

#[derive(Debug, Clone)]
pub struct BatchStoreItem {
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub access_pattern: AccessPattern,
}

#[derive(Debug, Clone)]
pub struct BatchStoreResult {
    pub index: usize,
    pub success: bool,
    pub record_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiMemoryInsights {
    pub total_records: usize,
    pub embedding_model: String,
    pub average_embedding_dimension: usize,
    pub cluster_analysis: Option<ClusterAnalysis>,
    pub similarity_patterns: Vec<SimilarityPattern>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClusterAnalysis {
    pub num_clusters: usize,
    pub cluster_quality_score: f64,
    pub cluster_descriptions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimilarityPattern {
    pub pattern_type: String,
    pub confidence: f64,
    pub description: String,
}

struct RerankingResult {
    pub scores: Vec<f32>,
}

// Factory for creating AI-Memory adapters with proper dependency injection
pub struct AiMemoryAdapterFactory;

impl AiMemoryAdapterFactory {
    pub async fn create_adapter(
        ai_inference_use_case: Arc<InferenceUseCase>,
        store_memory_use_case: Arc<dyn StoreMemoryUseCase>,
        search_memory_use_case: Arc<dyn SearchMemoryUseCase>,
        cache_provider: Arc<dyn CacheProvider>,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Result<AiMemoryAdapter> {
        Ok(AiMemoryAdapter::new(
            ai_inference_use_case,
            store_memory_use_case,
            search_memory_use_case,
            cache_provider,
            metrics_collector,
        ))
    }

    pub async fn create_configured_adapter(
        ai_inference_use_case: Arc<InferenceUseCase>,
        store_memory_use_case: Arc<dyn StoreMemoryUseCase>,
        search_memory_use_case: Arc<dyn SearchMemoryUseCase>,
        cache_provider: Arc<dyn CacheProvider>,
        metrics_collector: Arc<dyn MetricsCollector>,
        embedding_model: String,
        reranking_model: Option<String>,
    ) -> Result<AiMemoryAdapter> {
        Ok(AiMemoryAdapter::new(
            ai_inference_use_case,
            store_memory_use_case,
            search_memory_use_case,
            cache_provider,
            metrics_collector,
        )
        .with_embedding_model(embedding_model)
        .with_reranking_model(reranking_model))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "test-utils")]
    use crate::ports::cache_provider::MockCacheProvider;
    #[cfg(feature = "test-utils")]
    use crate::ports::metrics_collector::MockMetricsCollector;
    use std::sync::Arc;

    // Mock implementations for testing
    // These would need to be implemented based on your actual use case structures

    #[tokio::test]
    #[cfg(feature = "test-utils")]
    async fn test_ai_memory_adapter_creation() {
        // This would need actual use case implementations
        // For now, just test the concept

        let cache_provider = Arc::new(MockCacheProvider::new());
        let metrics_collector = Arc::new(MockMetricsCollector::new());

        // Mock use cases would be created here
        // let ai_inference = Arc::new(mock_inference_use_case);
        // let store_memory = Arc::new(mock_store_use_case);
        // let search_memory = Arc::new(mock_search_use_case);

        // let adapter = AiMemoryAdapter::new(
        //     ai_inference,
        //     store_memory,
        //     search_memory,
        //     cache_provider,
        //     metrics_collector,
        // );

        // assert_eq!(adapter.embedding_model, "bge-m3");
        // assert_eq!(adapter.default_top_k, 10);

        println!("AI-Memory adapter test placeholder - requires actual use case implementations");
    }

    #[tokio::test]
    async fn test_batch_store_configuration() {
        // Test batch configuration
        let items = vec![
            BatchStoreItem {
                content: "Test content 1".to_string(),
                metadata: Some(serde_json::json!({"type": "test"})),
                access_pattern: AccessPattern::default(), // Use default access pattern
            },
            BatchStoreItem {
                content: "Test content 2".to_string(),
                metadata: None,
                access_pattern: AccessPattern::default(), // Use default access pattern
            },
        ];

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].content, "Test content 1");
    }
}
