//! Search Memory Use Case
//!
//! Бизнес-логика для поиска записей в memory system с семантическим 
//! поиском и фильтрацией по слоям.

use async_trait::async_trait;
use crate::{ApplicationResult, ApplicationError, RequestContext};
use crate::dtos::{SearchMemoryRequest, SearchMemoryResponse, SimilaritySearchRequest, SimilaritySearchResponse};
use crate::ports::{EmbeddingProvider, SearchProvider, MetricsCollector, CacheProvider};
use domain::entities::embedding_vector::EmbeddingVector;
use domain::entities::search_query::SearchQuery;
use domain::repositories::search_repository::SearchRepository;
use domain::services::search_domain_service::SearchDomainService;
use domain::value_objects::layer_type::LayerType;
use domain::value_objects::score_threshold::ScoreThreshold;
use std::sync::Arc;
use tracing::{info, warn, error, instrument};

/// Use case для поиска записей в памяти
#[async_trait]
pub trait SearchMemoryUseCase: Send + Sync {
    /// Search memory records by text query
    async fn search_memory(&self, request: SearchMemoryRequest, context: RequestContext) -> ApplicationResult<SearchMemoryResponse>;
    
    /// Search memory records by semantic similarity
    async fn similarity_search(&self, request: SimilaritySearchRequest, context: RequestContext) -> ApplicationResult<SimilaritySearchResponse>;
    
    /// Get cached search results if available
    async fn get_cached_search(&self, query_hash: &str, context: RequestContext) -> ApplicationResult<Option<SearchMemoryResponse>>;
}

/// Implementation of search memory use case
pub struct SearchMemoryUseCaseImpl {
    search_repository: Arc<dyn SearchRepository>,
    search_domain_service: Arc<dyn SearchDomainService>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    search_provider: Arc<dyn SearchProvider>,
    cache_provider: Arc<dyn CacheProvider>,
    metrics_collector: Arc<dyn MetricsCollector>,
}

impl SearchMemoryUseCaseImpl {
    pub fn new(
        search_repository: Arc<dyn SearchRepository>,
        search_domain_service: Arc<dyn SearchDomainService>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        search_provider: Arc<dyn SearchProvider>,
        cache_provider: Arc<dyn CacheProvider>,
        metrics_collector: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self {
            search_repository,
            search_domain_service,
            embedding_provider,
            search_provider,
            cache_provider,
            metrics_collector,
        }
    }
}

#[async_trait]
impl SearchMemoryUseCase for SearchMemoryUseCaseImpl {
    #[instrument(skip(self, request), fields(query_length = request.query.len()))]
    async fn search_memory(&self, request: SearchMemoryRequest, context: RequestContext) -> ApplicationResult<SearchMemoryResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Starting search memory operation for request: {}", context.request_id);
        
        // Validate request
        self.validate_search_request(&request)?;
        
        // Create query hash for caching
        let query_hash = self.create_query_hash(&request);
        
        // Check cache first
        if request.use_cache {
            if let Some(cached_response) = self.get_cached_search(&query_hash, context.clone()).await? {
                info!("Returning cached search results for query: {}", request.query);
                self.record_cache_hit(&query_hash).await?;
                return Ok(cached_response);
            }
        }
        
        // Generate query embedding
        let embedding_start = std::time::Instant::now();
        let query_embedding = self.generate_query_embedding(&request.query).await?;
        let embedding_time = embedding_start.elapsed();
        
        // Create search query domain object
        let search_query = self.create_search_query(&request, query_embedding)?;
        
        // Execute search across layers
        let search_start = std::time::Instant::now();
        let search_results = self.execute_layered_search(&search_query, &request).await?;
        let search_time = search_start.elapsed();
        
        // Post-process results
        let processed_results = self.post_process_results(search_results, &request).await?;
        
        let total_time = start_time.elapsed();
        
        // Record metrics
        self.record_search_metrics(&request, processed_results.len(), total_time, embedding_time, search_time).await?;
        
        let response = SearchMemoryResponse {
            results: processed_results,
            total_results: processed_results.len(),
            search_time_ms: total_time.as_millis() as u64,
            query_hash: query_hash.clone(),
            layers_searched: request.layers.clone().unwrap_or_else(|| vec![LayerType::Cache, LayerType::Index, LayerType::Storage]),
        };
        
        // Cache results if requested
        if request.use_cache {
            self.cache_search_results(&query_hash, &response).await?;
        }
        
        info!("Search completed: {} results in {}ms", response.total_results, total_time.as_millis());
        
        Ok(response)
    }

    #[instrument(skip(self, request), fields(embedding_dims = request.query_embedding.len()))]
    async fn similarity_search(&self, request: SimilaritySearchRequest, context: RequestContext) -> ApplicationResult<SimilaritySearchResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Starting similarity search operation for request: {}", context.request_id);
        
        // Validate request
        self.validate_similarity_request(&request)?;
        
        // Create embedding vector
        let query_embedding = EmbeddingVector::new(request.query_embedding.clone())
            .map_err(|e| ApplicationError::Domain(e))?;
        
        // Create search query
        let search_query = SearchQuery::new(
            "".to_string(), // No text for similarity search
            query_embedding.clone(),
        ).map_err(|e| ApplicationError::Domain(e))?;
        
        // Execute similarity search
        let search_start = std::time::Instant::now();
        let search_results = self.execute_similarity_search(&search_query, &request).await?;
        let search_time = search_start.elapsed();
        
        let total_time = start_time.elapsed();
        
        // Record metrics
        self.record_similarity_metrics(&request, search_results.len(), total_time, search_time).await?;
        
        let response = SimilaritySearchResponse {
            results: search_results,
            total_results: search_results.len(),
            search_time_ms: total_time.as_millis() as u64,
            embedding_dimensions: query_embedding.dimensions(),
        };
        
        info!("Similarity search completed: {} results in {}ms", response.total_results, total_time.as_millis());
        
        Ok(response)
    }

    async fn get_cached_search(&self, query_hash: &str, _context: RequestContext) -> ApplicationResult<Option<SearchMemoryResponse>> {
        match self.cache_provider.get_search_results(query_hash).await {
            Ok(cached_results) => Ok(cached_results),
            Err(e) => {
                warn!("Failed to retrieve cached search results: {}", e);
                Ok(None)
            }
        }
    }
}

impl SearchMemoryUseCaseImpl {
    /// Validate search request
    fn validate_search_request(&self, request: &SearchMemoryRequest) -> ApplicationResult<()> {
        if request.query.is_empty() {
            return Err(ApplicationError::validation("Search query cannot be empty"));
        }
        
        if request.query.len() > 1000 {
            return Err(ApplicationError::validation("Search query too long (max 1000 characters)"));
        }
        
        if let Some(limit) = request.limit {
            if limit == 0 || limit > 1000 {
                return Err(ApplicationError::validation("Limit must be between 1 and 1000"));
            }
        }
        
        if let Some(threshold) = request.similarity_threshold {
            if threshold < 0.0 || threshold > 1.0 {
                return Err(ApplicationError::validation("Similarity threshold must be between 0.0 and 1.0"));
            }
        }
        
        Ok(())
    }
    
    /// Validate similarity search request
    fn validate_similarity_request(&self, request: &SimilaritySearchRequest) -> ApplicationResult<()> {
        if request.query_embedding.is_empty() {
            return Err(ApplicationError::validation("Query embedding cannot be empty"));
        }
        
        if request.query_embedding.len() > 4096 {
            return Err(ApplicationError::validation("Query embedding dimensions too large (max 4096)"));
        }
        
        if let Some(limit) = request.limit {
            if limit == 0 || limit > 1000 {
                return Err(ApplicationError::validation("Limit must be between 1 and 1000"));
            }
        }
        
        if let Some(threshold) = request.similarity_threshold {
            if threshold < 0.0 || threshold > 1.0 {
                return Err(ApplicationError::validation("Similarity threshold must be between 0.0 and 1.0"));
            }
        }
        
        Ok(())
    }
    
    /// Create query hash for caching
    fn create_query_hash(&self, request: &SearchMemoryRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        request.query.hash(&mut hasher);
        request.layers.hash(&mut hasher);
        request.limit.hash(&mut hasher);
        request.similarity_threshold.map(|t| (t * 1000000.0) as u64).hash(&mut hasher);
        request.filters.hash(&mut hasher);
        
        format!("search_{:x}", hasher.finish())
    }
    
    /// Generate embedding for query
    async fn generate_query_embedding(&self, query: &str) -> ApplicationResult<EmbeddingVector> {
        let raw_embedding = self.embedding_provider.generate_embedding(query).await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to generate query embedding", e))?;
        
        EmbeddingVector::new(raw_embedding)
            .map_err(|e| ApplicationError::Domain(e))
    }
    
    /// Create search query domain object
    fn create_search_query(&self, request: &SearchMemoryRequest, embedding: EmbeddingVector) -> ApplicationResult<SearchQuery> {
        SearchQuery::new(request.query.clone(), embedding)
            .map_err(|e| ApplicationError::Domain(e))
    }
    
    /// Execute layered search
    async fn execute_layered_search(
        &self,
        query: &SearchQuery,
        request: &SearchMemoryRequest,
    ) -> ApplicationResult<Vec<crate::dtos::SearchResult>> {
        let layers = request.layers.clone().unwrap_or_else(|| vec![LayerType::Cache, LayerType::Index, LayerType::Storage]);
        let limit = request.limit.unwrap_or(10);
        
        let threshold = request.similarity_threshold
            .map(|t| ScoreThreshold::new(t).map_err(|e| ApplicationError::Domain(e)))
            .transpose()?;
        
        // Search through domain service with layered approach
        let domain_results = self.search_domain_service.search_across_layers(
            query,
            &layers,
            limit,
            threshold.as_ref(),
            request.project.as_deref(),
            request.filters.as_ref(),
        ).await.map_err(|e| ApplicationError::Domain(e))?;
        
        // Convert domain results to DTOs
        let mut results = Vec::new();
        for domain_result in domain_results {
            let result = crate::dtos::SearchResult {
                record_id: domain_result.record_id().to_string(),
                content: domain_result.content().clone(),
                similarity_score: domain_result.similarity_score(),
                layer: domain_result.layer(),
                metadata: domain_result.metadata().cloned(),
                tags: domain_result.tags().clone(),
                last_accessed: domain_result.last_accessed().clone(),
                project: domain_result.project().cloned(),
            };
            results.push(result);
        }
        
        Ok(results)
    }
    
    /// Execute similarity search
    async fn execute_similarity_search(
        &self,
        query: &SearchQuery,
        request: &SimilaritySearchRequest,
    ) -> ApplicationResult<Vec<crate::dtos::SimilarityResult>> {
        let limit = request.limit.unwrap_or(10);
        
        let threshold = request.similarity_threshold
            .map(|t| ScoreThreshold::new(t).map_err(|e| ApplicationError::Domain(e)))
            .transpose()?;
        
        // Search through search provider
        let search_results = self.search_provider.similarity_search(
            query.embedding(),
            limit,
            threshold.as_ref(),
        ).await?;
        
        // Convert to DTOs
        let mut results = Vec::new();
        for result in search_results {
            let similarity_result = crate::dtos::SimilarityResult {
                record_id: result.record_id,
                embedding: result.embedding,
                similarity_score: result.similarity_score,
                metadata: result.metadata,
            };
            results.push(similarity_result);
        }
        
        Ok(results)
    }
    
    /// Post-process search results
    async fn post_process_results(
        &self,
        results: Vec<crate::dtos::SearchResult>,
        request: &SearchMemoryRequest,
    ) -> ApplicationResult<Vec<crate::dtos::SearchResult>> {
        let mut processed = results;
        
        // Apply custom filtering if specified
        if let Some(filters) = &request.filters {
            processed = self.apply_custom_filters(processed, filters).await?;
        }
        
        // Sort by relevance (similarity score descending, then by layer priority)
        processed.sort_by(|a, b| {
            let score_cmp = b.similarity_score.partial_cmp(&a.similarity_score).unwrap_or(std::cmp::Ordering::Equal);
            if score_cmp == std::cmp::Ordering::Equal {
                self.compare_layer_priority(&a.layer, &b.layer)
            } else {
                score_cmp
            }
        });
        
        // Apply final limit if necessary
        if let Some(limit) = request.limit {
            processed.truncate(limit);
        }
        
        Ok(processed)
    }
    
    /// Apply custom filters
    async fn apply_custom_filters(
        &self,
        results: Vec<crate::dtos::SearchResult>,
        filters: &std::collections::HashMap<String, String>,
    ) -> ApplicationResult<Vec<crate::dtos::SearchResult>> {
        let mut filtered = Vec::new();
        
        for result in results {
            let mut should_include = true;
            
            // Apply metadata filters
            if let Some(metadata) = &result.metadata {
                for (key, expected_value) in filters {
                    if let Some(actual_value) = metadata.get(key) {
                        if actual_value != expected_value {
                            should_include = false;
                            break;
                        }
                    } else {
                        should_include = false;
                        break;
                    }
                }
            } else if !filters.is_empty() {
                should_include = false;
            }
            
            if should_include {
                filtered.push(result);
            }
        }
        
        Ok(filtered)
    }
    
    /// Compare layer priority (Cache > Index > Storage)
    fn compare_layer_priority(&self, a: &LayerType, b: &LayerType) -> std::cmp::Ordering {
        let priority_a = match a {
            LayerType::Cache => 3,
            LayerType::Index => 2,
            LayerType::Storage => 1,
        };
        
        let priority_b = match b {
            LayerType::Cache => 3,
            LayerType::Index => 2,
            LayerType::Storage => 1,
        };
        
        priority_a.cmp(&priority_b)
    }
    
    /// Cache search results
    async fn cache_search_results(&self, query_hash: &str, response: &SearchMemoryResponse) -> ApplicationResult<()> {
        match self.cache_provider.cache_search_results(query_hash, response).await {
            Ok(_) => {
                info!("Successfully cached search results for query: {}", query_hash);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to cache search results: {}", e);
                Ok(()) // Don't fail the entire operation due to caching issues
            }
        }
    }
    
    /// Record cache hit metrics
    async fn record_cache_hit(&self, query_hash: &str) -> ApplicationResult<()> {
        self.metrics_collector.increment_counter(
            "search_cache_hits_total",
            1,
            Some(vec![("query_hash".to_string(), query_hash.to_string())]),
        ).await?;
        
        Ok(())
    }
    
    /// Record search metrics
    async fn record_search_metrics(
        &self,
        request: &SearchMemoryRequest,
        result_count: usize,
        total_time: std::time::Duration,
        embedding_time: std::time::Duration,
        search_time: std::time::Duration,
    ) -> ApplicationResult<()> {
        use crate::ports::{MemoryOperation, MemoryOperationType};
        
        let operation = MemoryOperation {
            operation_type: MemoryOperationType::Search,
            layer: "multiple".to_string(),
            record_count: result_count,
            processing_time_ms: total_time.as_millis() as u64,
            bytes_processed: request.query.len(),
            success: true,
            error: None,
        };
        
        self.metrics_collector.record_memory_operation(operation).await?;
        
        // Record detailed timing
        self.metrics_collector.record_timing(
            "search_embedding_generation",
            embedding_time.as_millis() as u64,
            None,
        ).await?;
        
        self.metrics_collector.record_timing(
            "search_execution",
            search_time.as_millis() as u64,
            None,
        ).await?;
        
        Ok(())
    }
    
    /// Record similarity search metrics
    async fn record_similarity_metrics(
        &self,
        request: &SimilaritySearchRequest,
        result_count: usize,
        total_time: std::time::Duration,
        search_time: std::time::Duration,
    ) -> ApplicationResult<()> {
        self.metrics_collector.increment_counter(
            "similarity_search_operations_total",
            1,
            None,
        ).await?;
        
        self.metrics_collector.record_gauge(
            "similarity_search_result_count",
            result_count as f64,
            None,
        ).await?;
        
        self.metrics_collector.record_timing(
            "similarity_search_duration",
            total_time.as_millis() as u64,
            None,
        ).await?;
        
        Ok(())
    }
}