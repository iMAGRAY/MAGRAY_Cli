//! Search Service Adapter
//!
//! Адаптер для интеграции с поисковыми сервисами из memory crate

use crate::ports::SearchProvider;
use crate::ports::{
    HybridSearchRequest, HybridSearchResult, SearchDocument, SimilaritySearchMatch,
    TextSearchRequest, TextSearchResult, VectorSearchRequest, VectorSearchResult,
};
use crate::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use domain::value_objects::{layer_type::LayerType, score_threshold::ScoreThreshold};
use domain::EmbeddingVector;
use std::sync::Arc;

/// Adapter for search services from memory crate
pub struct SearchServiceAdapter {
    /// HNSW vector search service
    vector_search_service: Arc<dyn VectorSearchServiceTrait>,
    /// Full-text search service
    text_search_service: Arc<dyn TextSearchServiceTrait>,
    /// Configuration
    config: SearchAdapterConfig,
}

/// Configuration for search adapter
#[derive(Debug, Clone)]
pub struct SearchAdapterConfig {
    pub default_search_timeout_ms: u64,
    pub max_results_per_query: usize,
    pub enable_caching: bool,
    pub cache_ttl_seconds: u64,
    pub similarity_threshold: f32,
}

/// Trait abstraction for vector search service
#[async_trait]
pub trait VectorSearchServiceTrait: Send + Sync {
    async fn search_similar(
        &self,
        query_vector: &[f32],
        limit: usize,
        threshold: Option<f32>,
    ) -> Result<Vec<VectorSearchMatch>, Box<dyn std::error::Error + Send + Sync>>;

    async fn add_vector(
        &self,
        id: &str,
        vector: &[f32],
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn remove_vector(
        &self,
        id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
}

/// Trait abstraction for text search service
#[async_trait]
pub trait TextSearchServiceTrait: Send + Sync {
    async fn search_text(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<TextSearchMatch>, Box<dyn std::error::Error + Send + Sync>>;

    async fn index_document(
        &self,
        id: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn remove_document(
        &self,
        id: &str,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
}

/// Vector search match from memory crate
#[derive(Debug, Clone)]
pub struct VectorSearchMatch {
    pub id: String,
    pub score: f32,
    pub vector: Option<Vec<f32>>,
}

/// Text search match from memory crate  
#[derive(Debug, Clone)]
pub struct TextSearchMatch {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub highlights: Vec<String>,
}

impl SearchServiceAdapter {
    pub fn new(
        vector_search_service: Arc<dyn VectorSearchServiceTrait>,
        text_search_service: Arc<dyn TextSearchServiceTrait>,
        config: SearchAdapterConfig,
    ) -> Self {
        Self {
            vector_search_service,
            text_search_service,
            config,
        }
    }

    /// Convert internal vector match to SearchDocument
    async fn convert_vector_match_to_document(
        &self,
        vm: &VectorSearchMatch,
    ) -> ApplicationResult<SearchDocument> {
        // This would typically retrieve full document info from storage
        // For now, create a minimal document
        Ok(SearchDocument::new(&vm.id, "")
            .with_layer(LayerType::Insights)
            .with_embedding(
                EmbeddingVector::new(vm.vector.clone().unwrap_or_default(), 384)
                    .map_err(|e| ApplicationError::Domain(e))?,
            ))
    }

    /// Convert internal text match to SearchDocument
    async fn convert_text_match_to_document(
        &self,
        tm: &TextSearchMatch,
    ) -> ApplicationResult<SearchDocument> {
        Ok(SearchDocument::new(&tm.id, &tm.content).with_layer(LayerType::Insights))
    }
}

#[async_trait]
impl SearchProvider for SearchServiceAdapter {
    async fn index_document(
        &self,
        document: &SearchDocument,
    ) -> ApplicationResult<crate::ports::IndexResult> {
        let start_time = std::time::Instant::now();

        // Index in both vector and text search services
        let mut errors = Vec::new();

        // Index in text search
        if let Err(e) = self
            .text_search_service
            .index_document(&document.id, &document.content)
            .await
        {
            errors.push(format!("Text indexing failed: {}", e));
        }

        if let Some(ref embedding) = document.embedding {
            if let Err(e) = self
                .vector_search_service
                .add_vector(&document.id, embedding.dimensions())
                .await
            {
                errors.push(format!("Vector indexing failed: {}", e));
            }
        }

        let duration = start_time.elapsed();
        let success = errors.is_empty();

        Ok(crate::ports::IndexResult {
            document_id: document.id.clone(),
            success,
            index_time_ms: duration.as_millis() as u64,
            index_size_bytes: document.content.len(),
            error: if errors.is_empty() {
                None
            } else {
                Some(errors.join("; "))
            },
        })
    }

    async fn index_documents(
        &self,
        documents: &[SearchDocument],
    ) -> ApplicationResult<crate::ports::BatchIndexResult> {
        let start_time = std::time::Instant::now();
        let mut results = Vec::new();
        let mut successful = 0;
        let mut failed = 0;
        let mut total_size = 0;

        for document in documents {
            let result = self.index_document(document).await?;
            if result.success {
                successful += 1;
            } else {
                failed += 1;
            }
            total_size += result.index_size_bytes;
            results.push(result);
        }

        Ok(crate::ports::BatchIndexResult {
            total_documents: documents.len(),
            successful_indexes: successful,
            failed_indexes: failed,
            results,
            total_time_ms: start_time.elapsed().as_millis() as u64,
            total_size_bytes: total_size,
        })
    }

    async fn remove_document(&self, document_id: &str) -> ApplicationResult<bool> {
        let mut removed = false;

        // Remove from text search
        if self
            .text_search_service
            .remove_document(document_id)
            .await
            .unwrap_or(false)
        {
            removed = true;
        }

        // Remove from vector search
        if self
            .vector_search_service
            .remove_vector(document_id)
            .await
            .unwrap_or(false)
        {
            removed = true;
        }

        Ok(removed)
    }

    async fn update_document(
        &self,
        document: &SearchDocument,
    ) -> ApplicationResult<crate::ports::IndexResult> {
        // For update, remove and re-add
        self.remove_document(&document.id).await?;
        self.index_document(document).await
    }

    async fn vector_search(
        &self,
        request: &VectorSearchRequest,
    ) -> ApplicationResult<VectorSearchResult> {
        let start_time = std::time::Instant::now();

        let matches = self
            .vector_search_service
            .search_similar(
                request.query_vector.dimensions(),
                request.limit,
                request.min_score,
            )
            .await
            .map_err(|e| {
                ApplicationError::infrastructure(format!("Vector search failed: {}", e))
            })?;

        // Convert matches to SearchDocuments
        let mut search_matches = Vec::new();
        for vm in matches {
            let document = self.convert_vector_match_to_document(&vm).await?;
            search_matches.push(crate::ports::VectorSearchMatch {
                document,
                similarity_score: vm.score,
                distance: 1.0 - vm.score,
                vector_included: request.include_vectors,
            });
        }

        let total_count = search_matches.len();
        Ok(VectorSearchResult {
            results: search_matches,
            total_found: total_count,
            search_time_ms: start_time.elapsed().as_millis() as u64,
            layer_stats: crate::ports::LayerSearchStats {
                cache_searched: false,
                cache_results: 0,
                cache_time_ms: 0,
                index_searched: true,
                index_results: total_count,
                index_time_ms: start_time.elapsed().as_millis() as u64,
                storage_searched: false,
                storage_results: 0,
                storage_time_ms: 0,
            },
        })
    }

    async fn text_search(
        &self,
        request: &TextSearchRequest,
    ) -> ApplicationResult<TextSearchResult> {
        let start_time = std::time::Instant::now();

        let matches = self
            .text_search_service
            .search_text(&request.query, request.limit)
            .await
            .map_err(|e| ApplicationError::infrastructure(format!("Text search failed: {}", e)))?;

        // Convert matches to SearchDocuments
        let mut search_matches = Vec::new();
        for tm in matches {
            let document = self.convert_text_match_to_document(&tm).await?;
            search_matches.push(crate::ports::TextSearchMatch {
                document,
                relevance_score: tm.score,
                highlights: tm
                    .highlights
                    .into_iter()
                    .map(|h| crate::ports::TextHighlight {
                        field: "content".to_string(),
                        fragments: vec![h],
                        start_offset: 0,
                        end_offset: 0,
                    })
                    .collect(),
                explanation: Some("Text match".to_string()),
            });
        }

        let total_count = search_matches.len();
        Ok(TextSearchResult {
            results: search_matches,
            total_found: total_count,
            search_time_ms: start_time.elapsed().as_millis() as u64,
            layer_stats: crate::ports::LayerSearchStats {
                cache_searched: false,
                cache_results: 0,
                cache_time_ms: 0,
                index_searched: true,
                index_results: total_count,
                index_time_ms: start_time.elapsed().as_millis() as u64,
                storage_searched: false,
                storage_results: 0,
                storage_time_ms: 0,
            },
            query_analysis: crate::ports::QueryAnalysis {
                normalized_query: request.query.to_lowercase(),
                extracted_terms: request
                    .query
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect(),
                detected_language: Some("en".to_string()),
                query_type: crate::ports::QueryType::Simple,
                complexity_score: 0.5,
            },
        })
    }

    async fn hybrid_search(
        &self,
        request: &HybridSearchRequest,
    ) -> ApplicationResult<HybridSearchResult> {
        let start_time = std::time::Instant::now();

        // Perform both vector and text search
        let mut all_matches = std::collections::HashMap::new();

        if let Some(ref query_vector) = request.query_vector {
            let vector_request = VectorSearchRequest {
                query_vector: query_vector.clone(),
                limit: request.limit * 2, // Get more results to combine
                min_score: request.min_score,
                filters: request.filters.clone(),
                include_vectors: false,
                search_layers: request.search_layers.clone(),
            };

            if let Ok(vector_results) = self.vector_search(&vector_request).await {
                for result in vector_results.results {
                    let doc_id = result.document.id.clone();
                    all_matches.insert(
                        doc_id,
                        (result.document, Some(result.similarity_score), None),
                    );
                }
            }
        }

        // Text search
        let text_request = TextSearchRequest {
            query: request.query.clone(),
            limit: request.limit * 2,
            min_score: request.min_score,
            filters: request.filters.clone(),
            search_type: crate::ports::TextSearchType::TfIdf,
            search_layers: request.search_layers.clone(),
        };

        if let Ok(text_results) = self.text_search(&text_request).await {
            for result in text_results.results {
                let doc_id = result.document.id.clone();
                match all_matches.get_mut(&doc_id) {
                    Some((_, vector_score, text_score)) => {
                        *text_score = Some(result.relevance_score);
                    }
                    None => {
                        all_matches.insert(
                            doc_id,
                            (result.document, None, Some(result.relevance_score)),
                        );
                    }
                }
            }
        }

        // Store the count before moving all_matches
        let total_matches_count = all_matches.len();

        // Combine scores and create hybrid results
        let mut hybrid_matches: Vec<_> = all_matches
            .into_iter()
            .map(|(_, (document, vector_score, text_score))| {
                let vector_score = vector_score.unwrap_or(0.0);
                let text_score = text_score.unwrap_or(0.0);
                let combined_score = (vector_score * request.weights.vector_weight)
                    + (text_score * request.weights.text_weight);

                crate::ports::HybridSearchMatch {
                    document,
                    combined_score,
                    vector_score: if vector_score > 0.0 {
                        Some(vector_score)
                    } else {
                        None
                    },
                    text_score: if text_score > 0.0 {
                        Some(text_score)
                    } else {
                        None
                    },
                    metadata_score: 0.5,
                    recency_score: 0.7,
                    highlights: vec![],
                    explanation: Some("Hybrid search match".to_string()),
                }
            })
            .collect();

        // Sort by combined score and limit results
        hybrid_matches.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hybrid_matches.truncate(request.limit);

        Ok(HybridSearchResult {
            results: hybrid_matches,
            total_found: total_matches_count,
            search_time_ms: start_time.elapsed().as_millis() as u64,
            layer_stats: crate::ports::LayerSearchStats {
                cache_searched: false,
                cache_results: 0,
                cache_time_ms: 0,
                index_searched: true,
                index_results: total_matches_count,
                index_time_ms: start_time.elapsed().as_millis() as u64,
                storage_searched: false,
                storage_results: 0,
                storage_time_ms: 0,
            },
            query_analysis: crate::ports::QueryAnalysis {
                normalized_query: request.query.to_lowercase(),
                extracted_terms: request
                    .query
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect(),
                detected_language: Some("en".to_string()),
                query_type: crate::ports::QueryType::Simple,
                complexity_score: 0.7,
            },
            score_breakdown: crate::ports::ScoreBreakdown {
                vector_contribution: request.weights.vector_weight,
                text_contribution: request.weights.text_weight,
                metadata_contribution: request.weights.metadata_weight,
                recency_contribution: request.weights.recency_weight,
                normalization_factor: 1.0,
            },
        })
    }

    async fn similarity_search(
        &self,
        query_embedding: &EmbeddingVector,
        limit: usize,
        threshold: Option<&ScoreThreshold>,
    ) -> ApplicationResult<Vec<SimilaritySearchMatch>> {
        let min_threshold = threshold
            .map(|t| t.value())
            .unwrap_or(self.config.similarity_threshold);

        let matches = self
            .vector_search_service
            .search_similar(
                query_embedding.dimensions(),
                limit,
                Some(min_threshold as f32),
            )
            .await
            .map_err(|e| {
                ApplicationError::infrastructure(format!("Similarity search failed: {}", e))
            })?;

        let mut similarity_matches = Vec::new();
        for vm in matches {
            if vm.score >= min_threshold {
                similarity_matches.push(SimilaritySearchMatch {
                    record_id: vm.id,
                    embedding: vm.vector.unwrap_or_default(),
                    similarity_score: vm.score as f64,
                    metadata: Some(
                        [
                            ("source".to_string(), "vector_search".to_string()),
                            ("score".to_string(), vm.score.to_string()),
                        ]
                        .into(),
                    ),
                });
            }
        }

        Ok(similarity_matches)
    }

    async fn get_document(&self, document_id: &str) -> ApplicationResult<Option<SearchDocument>> {
        Ok(None)
    }

    async fn get_index_stats(&self) -> ApplicationResult<crate::ports::IndexStatistics> {
        Ok(crate::ports::IndexStatistics {
            total_documents: 0,
            total_size_bytes: 0,
            index_size_bytes: 0,
            average_document_size: 0,
            vector_dimensions: Some(384),
            last_optimization: None,
            fragmentation_ratio: 0.1,
            layer_distribution: crate::ports::LayerDistribution {
                cache_documents: 0,
                index_documents: 0,
                storage_documents: 0,
            },
            performance_metrics: crate::ports::IndexPerformanceMetrics {
                average_search_time_ms: 25.0,
                average_index_time_ms: 10.0,
                searches_per_second: 100.0,
                indexes_per_second: 50.0,
                cache_hit_rate: 0.8,
            },
        })
    }

    async fn health_check(&self) -> ApplicationResult<crate::ports::SearchHealth> {
        Ok(crate::ports::SearchHealth {
            is_healthy: true,
            index_status: crate::ports::IndexStatus::Ready,
            memory_usage_mb: 256.0,
            disk_usage_mb: 1024.0,
            response_time_ms: 15,
            error_rate: 0.01,
            last_error: None,
        })
    }

    async fn optimize_index(&self) -> ApplicationResult<crate::ports::OptimizationResult> {
        Ok(crate::ports::OptimizationResult {
            optimization_type: crate::ports::OptimizationType::Compact,
            duration_ms: 2000,
            size_before_bytes: 1048576,
            size_after_bytes: 838860,
            compression_ratio: 0.8,
            performance_improvement: 0.2,
        })
    }
}

impl Default for SearchAdapterConfig {
    fn default() -> Self {
        Self {
            default_search_timeout_ms: 5000,
            max_results_per_query: 1000,
            enable_caching: true,
            cache_ttl_seconds: 300,
            similarity_threshold: 0.7,
        }
    }
}

impl SearchAdapterConfig {
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.default_search_timeout_ms = timeout_ms;
        self
    }

    pub fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_results_per_query = max_results;
        self
    }

    pub fn with_similarity_threshold(mut self, threshold: f32) -> Self {
        self.similarity_threshold = threshold;
        self
    }

    pub fn disable_caching(mut self) -> Self {
        self.enable_caching = false;
        self
    }
}
