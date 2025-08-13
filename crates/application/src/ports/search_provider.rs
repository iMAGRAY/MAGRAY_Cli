//! Search Provider Port  
//!
//! Абстракция для поисковых индексов и векторного поиска независимо от конкретной реализации.

use crate::ApplicationResult;
use async_trait::async_trait;
use domain::EmbeddingVector;
use domain::LayerType;
use serde::{Deserialize, Serialize};

/// Trait для search providers
#[async_trait]
pub trait SearchProvider: Send + Sync {
    /// Add document to search index
    async fn index_document(&self, document: &SearchDocument) -> ApplicationResult<IndexResult>;

    /// Add multiple documents to index
    async fn index_documents(
        &self,
        documents: &[SearchDocument],
    ) -> ApplicationResult<BatchIndexResult>;

    /// Remove document from index
    async fn remove_document(&self, document_id: &str) -> ApplicationResult<bool>;

    /// Update document in index
    async fn update_document(&self, document: &SearchDocument) -> ApplicationResult<IndexResult>;

    /// Perform vector similarity search
    async fn vector_search(
        &self,
        request: &VectorSearchRequest,
    ) -> ApplicationResult<VectorSearchResult>;

    /// Perform text-based search
    async fn text_search(&self, request: &TextSearchRequest)
        -> ApplicationResult<TextSearchResult>;

    /// Perform hybrid search (vector + text)
    async fn hybrid_search(
        &self,
        request: &HybridSearchRequest,
    ) -> ApplicationResult<HybridSearchResult>;

    /// Perform similarity search with raw embedding
    async fn similarity_search(
        &self,
        query_embedding: &domain::EmbeddingVector,
        limit: usize,
        threshold: Option<&domain::ScoreThreshold>,
    ) -> ApplicationResult<Vec<SimilaritySearchMatch>>;

    /// Get document by ID
    async fn get_document(&self, document_id: &str) -> ApplicationResult<Option<SearchDocument>>;

    /// Get index statistics
    async fn get_index_stats(&self) -> ApplicationResult<IndexStatistics>;

    /// Health check for search provider
    async fn health_check(&self) -> ApplicationResult<SearchHealth>;

    /// Optimize index (rebuild, compress, etc.)
    async fn optimize_index(&self) -> ApplicationResult<OptimizationResult>;
}

/// Document to be indexed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDocument {
    pub id: String,
    pub content: String,
    pub embedding: Option<EmbeddingVector>,
    pub metadata: DocumentMetadata,
    pub layer: LayerType,
}

/// Document metadata for indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub project: Option<String>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub content_type: String,
    pub language: Option<String>,
    pub author: Option<String>,
    pub custom_fields: std::collections::HashMap<String, serde_json::Value>,
}

/// Result of indexing operation
#[derive(Debug, Clone)]
pub struct IndexResult {
    pub document_id: String,
    pub success: bool,
    pub index_time_ms: u64,
    pub index_size_bytes: usize,
    pub error: Option<String>,
}

/// Batch indexing result
#[derive(Debug, Clone)]
pub struct BatchIndexResult {
    pub total_documents: usize,
    pub successful_indexes: usize,
    pub failed_indexes: usize,
    pub results: Vec<IndexResult>,
    pub total_time_ms: u64,
    pub total_size_bytes: usize,
}

/// Vector search request
#[derive(Debug, Clone)]
pub struct VectorSearchRequest {
    pub query_vector: EmbeddingVector,
    pub limit: usize,
    pub min_score: Option<f32>,
    pub filters: SearchFilters,
    pub include_vectors: bool,
    pub search_layers: Vec<LayerType>,
}

/// Text search request
#[derive(Debug, Clone)]
pub struct TextSearchRequest {
    pub query: String,
    pub limit: usize,
    pub min_score: Option<f32>,
    pub filters: SearchFilters,
    pub search_type: TextSearchType,
    pub search_layers: Vec<LayerType>,
}

/// Hybrid search request
#[derive(Debug, Clone)]
pub struct HybridSearchRequest {
    pub query: String,
    pub query_vector: Option<EmbeddingVector>,
    pub limit: usize,
    pub min_score: Option<f32>,
    pub filters: SearchFilters,
    pub weights: HybridSearchWeights,
    pub search_layers: Vec<LayerType>,
}

/// Search filters
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub project: Option<String>,
    pub tags: Vec<String>,
    pub date_range: Option<DateRange>,
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub custom_filters: std::collections::HashMap<String, serde_json::Value>,
}

/// Date range filter
#[derive(Debug, Clone)]
pub struct DateRange {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

/// Text search types
#[derive(Debug, Clone)]
pub enum TextSearchType {
    /// Exact phrase matching
    Exact,
    /// Fuzzy matching with typos
    Fuzzy { max_edits: u32 },
    /// Boolean query (AND, OR, NOT)
    Boolean,
    /// Term frequency matching
    TfIdf,
    /// BM25 ranking
    Bm25,
}

/// Hybrid search weights
#[derive(Debug, Clone)]
pub struct HybridSearchWeights {
    pub vector_weight: f32,
    pub text_weight: f32,
    pub metadata_weight: f32,
    pub recency_weight: f32,
}

/// Vector search result
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub results: Vec<VectorSearchMatch>,
    pub total_found: usize,
    pub search_time_ms: u64,
    pub layer_stats: LayerSearchStats,
}

/// Text search result
#[derive(Debug, Clone)]
pub struct TextSearchResult {
    pub results: Vec<TextSearchMatch>,
    pub total_found: usize,
    pub search_time_ms: u64,
    pub layer_stats: LayerSearchStats,
    pub query_analysis: QueryAnalysis,
}

/// Hybrid search result
#[derive(Debug, Clone)]
pub struct HybridSearchResult {
    pub results: Vec<HybridSearchMatch>,
    pub total_found: usize,
    pub search_time_ms: u64,
    pub layer_stats: LayerSearchStats,
    pub query_analysis: QueryAnalysis,
    pub score_breakdown: ScoreBreakdown,
}

/// Vector search match
#[derive(Debug, Clone)]
pub struct VectorSearchMatch {
    pub document: SearchDocument,
    pub similarity_score: f32,
    pub distance: f32,
    pub vector_included: bool,
}

/// Text search match
#[derive(Debug, Clone)]
pub struct TextSearchMatch {
    pub document: SearchDocument,
    pub relevance_score: f32,
    pub highlights: Vec<TextHighlight>,
    pub explanation: Option<String>,
}

/// Hybrid search match
#[derive(Debug, Clone)]
pub struct HybridSearchMatch {
    pub document: SearchDocument,
    pub combined_score: f32,
    pub vector_score: Option<f32>,
    pub text_score: Option<f32>,
    pub metadata_score: f32,
    pub recency_score: f32,
    pub highlights: Vec<TextHighlight>,
    pub explanation: Option<String>,
}

/// Similarity search match (for raw embedding queries)
#[derive(Debug, Clone)]
pub struct SimilaritySearchMatch {
    pub record_id: String,
    pub embedding: Vec<f32>,
    pub similarity_score: f64,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

/// Text highlighting
#[derive(Debug, Clone)]
pub struct TextHighlight {
    pub field: String,
    pub fragments: Vec<String>,
    pub start_offset: usize,
    pub end_offset: usize,
}

/// Search statistics per layer
#[derive(Debug, Clone)]
pub struct LayerSearchStats {
    pub cache_searched: bool,
    pub cache_results: usize,
    pub cache_time_ms: u64,

    pub index_searched: bool,
    pub index_results: usize,
    pub index_time_ms: u64,

    pub storage_searched: bool,
    pub storage_results: usize,
    pub storage_time_ms: u64,
}

/// Query analysis results
#[derive(Debug, Clone)]
pub struct QueryAnalysis {
    pub normalized_query: String,
    pub extracted_terms: Vec<String>,
    pub detected_language: Option<String>,
    pub query_type: QueryType,
    pub complexity_score: f32,
}

/// Query types
#[derive(Debug, Clone)]
pub enum QueryType {
    Simple,
    Complex,
    Boolean,
    Phrase,
    Wildcard,
    Regex,
}

/// Score breakdown for hybrid search
#[derive(Debug, Clone)]
pub struct ScoreBreakdown {
    pub vector_contribution: f32,
    pub text_contribution: f32,
    pub metadata_contribution: f32,
    pub recency_contribution: f32,
    pub normalization_factor: f32,
}

/// Index statistics
#[derive(Debug, Clone)]
pub struct IndexStatistics {
    pub total_documents: u64,
    pub total_size_bytes: u64,
    pub index_size_bytes: u64,
    pub average_document_size: u64,
    pub vector_dimensions: Option<usize>,
    pub last_optimization: Option<chrono::DateTime<chrono::Utc>>,
    pub fragmentation_ratio: f32,
    pub layer_distribution: LayerDistribution,
    pub performance_metrics: IndexPerformanceMetrics,
}

/// Document distribution across layers
#[derive(Debug, Clone)]
pub struct LayerDistribution {
    pub cache_documents: u64,
    pub index_documents: u64,
    pub storage_documents: u64,
}

/// Index performance metrics
#[derive(Debug, Clone)]
pub struct IndexPerformanceMetrics {
    pub average_search_time_ms: f64,
    pub average_index_time_ms: f64,
    pub searches_per_second: f64,
    pub indexes_per_second: f64,
    pub cache_hit_rate: f32,
}

/// Search provider health
#[derive(Debug, Clone)]
pub struct SearchHealth {
    pub is_healthy: bool,
    pub index_status: IndexStatus,
    pub memory_usage_mb: f64,
    pub disk_usage_mb: f64,
    pub response_time_ms: u64,
    pub error_rate: f32,
    pub last_error: Option<String>,
}

/// Index status
#[derive(Debug, Clone)]
pub enum IndexStatus {
    Ready,
    Indexing,
    Optimizing,
    Corrupted,
    ReadOnly,
    Unavailable,
}

/// Index optimization result
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    pub optimization_type: OptimizationType,
    pub duration_ms: u64,
    pub size_before_bytes: u64,
    pub size_after_bytes: u64,
    pub compression_ratio: f32,
    pub performance_improvement: f32,
}

/// Types of index optimization
#[derive(Debug, Clone)]
pub enum OptimizationType {
    Compact,
    Reindex,
    Merge,
    Vacuum,
    Full,
}

impl Default for HybridSearchWeights {
    fn default() -> Self {
        Self {
            vector_weight: 0.6,
            text_weight: 0.3,
            metadata_weight: 0.05,
            recency_weight: 0.05,
        }
    }
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            title: None,
            project: None,
            tags: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content_type: "text/plain".to_string(),
            language: None,
            author: None,
            custom_fields: std::collections::HashMap::new(),
        }
    }
}

impl SearchDocument {
    pub fn new(id: &str, content: &str) -> Self {
        Self {
            id: id.to_string(),
            content: content.to_string(),
            embedding: None,
            metadata: DocumentMetadata::default(),
            layer: LayerType::Assets,
        }
    }

    pub fn with_embedding(mut self, embedding: EmbeddingVector) -> Self {
        self.embedding = Some(embedding);
        self
    }

    pub fn with_layer(mut self, layer: LayerType) -> Self {
        self.layer = layer;
        self
    }

    pub fn with_project(mut self, project: &str) -> Self {
        self.metadata.project = Some(project.to_string());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.metadata.tags = tags;
        self
    }
}

/// Mock search provider for testing
#[cfg(feature = "test-utils")]
pub struct MockSearchProvider {
    documents: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, SearchDocument>>>,
    operations: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

#[cfg(feature = "test-utils")]
impl MockSearchProvider {
    pub fn new() -> Self {
        Self {
            documents: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            operations: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn add_mock_document(&self, document: SearchDocument) {
        self.documents
            .lock()
            .expect("Operation should succeed")
            .insert(document.id.clone(), document);
    }

    pub fn get_operations(&self) -> Vec<String> {
        self.operations
            .lock()
            .expect("Operation should succeed")
            .clone()
    }

    fn record_operation(&self, operation: &str) {
        self.operations
            .lock()
            .expect("Operation should succeed")
            .push(operation.to_string());
    }
}

#[cfg(feature = "test-utils")]
#[async_trait]
impl SearchProvider for MockSearchProvider {
    async fn index_document(&self, document: &SearchDocument) -> ApplicationResult<IndexResult> {
        self.record_operation(&format!("index_document:{}", document.id));

        self.documents
            .lock()
            .expect("Operation should succeed")
            .insert(document.id.clone(), document.clone());

        Ok(IndexResult {
            document_id: document.id.clone(),
            success: true,
            index_time_ms: 10,
            index_size_bytes: document.content.len(),
            error: None,
        })
    }

    async fn index_documents(
        &self,
        documents: &[SearchDocument],
    ) -> ApplicationResult<BatchIndexResult> {
        self.record_operation(&format!("index_documents:{}", documents.len()));

        let mut results = Vec::new();
        let start_time = std::time::Instant::now();

        for document in documents {
            results.push(self.index_document(document).await?);
        }

        Ok(BatchIndexResult {
            total_documents: documents.len(),
            successful_indexes: results.len(),
            failed_indexes: 0,
            results,
            total_time_ms: start_time.elapsed().as_millis() as u64,
            total_size_bytes: documents.iter().map(|d| d.content.len()).sum(),
        })
    }

    async fn remove_document(&self, document_id: &str) -> ApplicationResult<bool> {
        self.record_operation(&format!("remove_document:{}", document_id));
        Ok(self
            .documents
            .lock()
            .expect("Operation should succeed")
            .remove(document_id)
            .is_some())
    }

    async fn update_document(&self, document: &SearchDocument) -> ApplicationResult<IndexResult> {
        self.record_operation(&format!("update_document:{}", document.id));
        self.index_document(document).await
    }

    async fn vector_search(
        &self,
        request: &VectorSearchRequest,
    ) -> ApplicationResult<VectorSearchResult> {
        self.record_operation(&format!("vector_search:limit={}", request.limit));

        let documents = self.documents.lock().expect("Operation should succeed");
        let mut results = Vec::new();

        for document in documents.values() {
            if let Some(embedding) = &document.embedding {
                // Mock similarity calculation
                let similarity = 0.9 - (results.len() as f32 * 0.1);
                if similarity >= request.min_score.unwrap_or(0.0) {
                    results.push(VectorSearchMatch {
                        document: document.clone(),
                        similarity_score: similarity,
                        distance: 1.0 - similarity,
                        vector_included: request.include_vectors,
                    });
                }

                if results.len() >= request.limit {
                    break;
                }
            }
        }

        Ok(VectorSearchResult {
            results,
            total_found: documents.len(),
            search_time_ms: 50,
            layer_stats: LayerSearchStats {
                cache_searched: true,
                cache_results: 0,
                cache_time_ms: 10,
                index_searched: true,
                index_results: documents.len(),
                index_time_ms: 40,
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
        self.record_operation(&format!("text_search:{}", request.query));

        let documents = self.documents.lock().expect("Operation should succeed");
        let mut results = Vec::new();

        for document in documents.values() {
            if document
                .content
                .to_lowercase()
                .contains(&request.query.to_lowercase())
            {
                let score = 0.8 - (results.len() as f32 * 0.1);
                if score >= request.min_score.unwrap_or(0.0) {
                    results.push(TextSearchMatch {
                        document: document.clone(),
                        relevance_score: score,
                        highlights: vec![TextHighlight {
                            field: "content".to_string(),
                            fragments: vec![
                                document.content[..50.min(document.content.len())].to_string()
                            ],
                            start_offset: 0,
                            end_offset: 50.min(document.content.len()),
                        }],
                        explanation: Some("Mock text match".to_string()),
                    });
                }

                if results.len() >= request.limit {
                    break;
                }
            }
        }

        Ok(TextSearchResult {
            results,
            total_found: documents.len(),
            search_time_ms: 30,
            layer_stats: LayerSearchStats {
                cache_searched: true,
                cache_results: 0,
                cache_time_ms: 5,
                index_searched: true,
                index_results: documents.len(),
                index_time_ms: 25,
                storage_searched: false,
                storage_results: 0,
                storage_time_ms: 0,
            },
            query_analysis: QueryAnalysis {
                normalized_query: request.query.to_lowercase(),
                extracted_terms: request
                    .query
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect(),
                detected_language: Some("en".to_string()),
                query_type: QueryType::Simple,
                complexity_score: 0.5,
            },
        })
    }

    async fn hybrid_search(
        &self,
        request: &HybridSearchRequest,
    ) -> ApplicationResult<HybridSearchResult> {
        self.record_operation(&format!("hybrid_search:{}", request.query));

        // Mock hybrid search by combining vector and text results
        let documents = self.documents.lock().expect("Operation should succeed");
        let mut results = Vec::new();

        for document in documents.values() {
            let text_score = if document
                .content
                .to_lowercase()
                .contains(&request.query.to_lowercase())
            {
                0.8
            } else {
                0.2
            };

            let vector_score = if document.embedding.is_some() {
                0.9
            } else {
                0.0
            };

            let combined_score = (vector_score * request.weights.vector_weight)
                + (text_score * request.weights.text_weight);

            if combined_score >= request.min_score.unwrap_or(0.0) {
                results.push(HybridSearchMatch {
                    document: document.clone(),
                    combined_score,
                    vector_score: Some(vector_score),
                    text_score: Some(text_score),
                    metadata_score: 0.5,
                    recency_score: 0.7,
                    highlights: vec![],
                    explanation: Some("Mock hybrid match".to_string()),
                });
            }

            if results.len() >= request.limit {
                break;
            }
        }

        // Sort by combined score
        results.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .expect("Operation should succeed")
        });

        Ok(HybridSearchResult {
            results,
            total_found: documents.len(),
            search_time_ms: 75,
            layer_stats: LayerSearchStats {
                cache_searched: true,
                cache_results: 0,
                cache_time_ms: 15,
                index_searched: true,
                index_results: documents.len(),
                index_time_ms: 60,
                storage_searched: false,
                storage_results: 0,
                storage_time_ms: 0,
            },
            query_analysis: QueryAnalysis {
                normalized_query: request.query.to_lowercase(),
                extracted_terms: request
                    .query
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect(),
                detected_language: Some("en".to_string()),
                query_type: QueryType::Simple,
                complexity_score: 0.7,
            },
            score_breakdown: ScoreBreakdown {
                vector_contribution: request.weights.vector_weight,
                text_contribution: request.weights.text_weight,
                metadata_contribution: request.weights.metadata_weight,
                recency_contribution: request.weights.recency_weight,
                normalization_factor: 1.0,
            },
        })
    }

    async fn get_document(&self, document_id: &str) -> ApplicationResult<Option<SearchDocument>> {
        self.record_operation(&format!("get_document:{}", document_id));
        Ok(self
            .documents
            .lock()
            .expect("Operation should succeed")
            .get(document_id)
            .cloned())
    }

    async fn get_index_stats(&self) -> ApplicationResult<IndexStatistics> {
        let documents = self.documents.lock().expect("Operation should succeed");
        Ok(IndexStatistics {
            total_documents: documents.len() as u64,
            total_size_bytes: documents.values().map(|d| d.content.len() as u64).sum(),
            index_size_bytes: documents.len() as u64 * 1024, // Mock value
            average_document_size: 1024,
            vector_dimensions: Some(384), // Mock dimension
            last_optimization: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
            fragmentation_ratio: 0.1,
            layer_distribution: LayerDistribution {
                cache_documents: 0,
                index_documents: documents.len() as u64,
                storage_documents: 0,
            },
            performance_metrics: IndexPerformanceMetrics {
                average_search_time_ms: 25.0,
                average_index_time_ms: 10.0,
                searches_per_second: 100.0,
                indexes_per_second: 50.0,
                cache_hit_rate: 0.8,
            },
        })
    }

    async fn health_check(&self) -> ApplicationResult<SearchHealth> {
        Ok(SearchHealth {
            is_healthy: true,
            index_status: IndexStatus::Ready,
            memory_usage_mb: 128.0,
            disk_usage_mb: 512.0,
            response_time_ms: 10,
            error_rate: 0.0,
            last_error: None,
        })
    }

    async fn optimize_index(&self) -> ApplicationResult<OptimizationResult> {
        self.record_operation("optimize_index");

        Ok(OptimizationResult {
            optimization_type: OptimizationType::Compact,
            duration_ms: 5000,
            size_before_bytes: 1024000,
            size_after_bytes: 819200,
            compression_ratio: 0.8,
            performance_improvement: 0.15,
        })
    }

    async fn similarity_search(
        &self,
        query_embedding: &domain::EmbeddingVector,
        limit: usize,
        threshold: Option<&domain::ScoreThreshold>,
    ) -> ApplicationResult<Vec<SimilaritySearchMatch>> {
        self.record_operation(&format!("similarity_search:limit={}", limit));

        let documents = self.documents.lock().expect("Operation should succeed");
        let mut results = Vec::new();

        for document in documents.values() {
            if let Some(embedding) = &document.embedding {
                // Mock similarity calculation
                let similarity = 0.95 - (results.len() as f64 * 0.05);

                let min_threshold = threshold.map(|t| t.value()).unwrap_or(0.0) as f64;
                if similarity >= min_threshold {
                    results.push(SimilaritySearchMatch {
                        record_id: document.id.clone(),
                        embedding: embedding.as_vec().clone(),
                        similarity_score: similarity,
                        metadata: Some(
                            [
                                ("layer".to_string(), format!("{:?}", document.layer)),
                                (
                                    "content_length".to_string(),
                                    document.content.len().to_string(),
                                ),
                            ]
                            .into(),
                        ),
                    });
                }

                if results.len() >= limit {
                    break;
                }
            }
        }

        // Sort by similarity descending
        results.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .expect("Operation should succeed")
        });

        Ok(results)
    }
}
