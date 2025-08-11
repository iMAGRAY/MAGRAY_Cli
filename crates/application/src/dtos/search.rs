//! Search DTOs for semantic search operations

use serde::{Deserialize, Serialize};
use validator::Validate;
use domain::value_objects::layer_type::LayerType;
use domain::value_objects::score_threshold::ScoreThreshold;
use super::PaginationParams;

/// Semantic search request DTO
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct SearchRequest {
    /// Search query text
    #[validate(length(min = 1, max = 10000))]
    pub query: String,
    
    /// Maximum number of results
    #[validate(range(min = 1, max = 1000))]
    pub limit: u32,
    
    /// Minimum similarity threshold
    #[validate(range(min = 0.0, max = 1.0))]
    pub min_score: Option<f32>,
    
    /// Search filters
    pub filters: SearchFilters,
    
    /// Search options
    pub options: SearchOptions,
}

/// Search filters for narrowing results
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SearchFilters {
    /// Filter by project
    pub project: Option<String>,
    
    /// Filter by layers to search
    pub layers: Vec<LayerType>,
    
    /// Filter by tags (AND operation)
    pub required_tags: Vec<String>,
    
    /// Filter by tags (OR operation)
    pub optional_tags: Vec<String>,
    
    /// Date range filter
    pub date_range: Option<DateRange>,
    
    /// Exclude specific record IDs
    pub exclude_ids: Vec<String>,
}

/// Search configuration options
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchOptions {
    /// Enable re-ranking with LLM
    pub enable_reranking: bool,
    
    /// Include similarity explanations
    pub explain_scores: bool,
    
    /// Return embedding vectors
    pub include_embeddings: bool,
    
    /// Search strategy
    pub strategy: SearchStrategy,
    
    /// Hybrid search weights
    pub hybrid_weights: Option<HybridWeights>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SearchStrategy {
    /// Pure semantic similarity
    Semantic,
    /// Keyword matching
    Keyword,
    /// Hybrid semantic + keyword
    Hybrid,
    /// Neural reranking
    Neural,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HybridWeights {
    pub semantic_weight: f32,
    pub keyword_weight: f32,
    pub recency_weight: f32,
    pub popularity_weight: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DateRange {
    pub from: chrono::DateTime<chrono::Utc>,
    pub to: chrono::DateTime<chrono::Utc>,
}

/// Search response DTO
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total_found: u64,
    pub search_time_ms: u64,
    pub query_analysis: QueryAnalysis,
    pub layer_stats: LayerSearchStats,
}

/// Individual search result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub record_id: String,
    pub content: String,
    pub content_preview: String,
    pub score: f32,
    pub layer: LayerType,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    pub explanation: Option<ScoreExplanation>,
    pub embedding: Option<Vec<f32>>,
}

/// Score explanation for transparency
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScoreExplanation {
    pub semantic_score: f32,
    pub keyword_score: Option<f32>,
    pub recency_boost: f32,
    pub popularity_boost: f32,
    pub final_score: f32,
    pub reasoning: String,
}

/// Query analysis results
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryAnalysis {
    pub intent: QueryIntent,
    pub entities: Vec<String>,
    pub keywords: Vec<String>,
    pub complexity: QueryComplexity,
    pub suggested_filters: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum QueryIntent {
    Factual,
    Procedural,
    Conceptual,
    Comparative,
    Troubleshooting,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum QueryComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}

/// Search statistics per layer
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LayerSearchStats {
    pub cache_searched: bool,
    pub cache_hits: u64,
    pub cache_time_ms: u64,
    
    pub index_searched: bool,
    pub index_hits: u64,
    pub index_time_ms: u64,
    
    pub storage_searched: bool,
    pub storage_hits: u64,
    pub storage_time_ms: u64,
}

/// Multi-query search request
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct MultiSearchRequest {
    #[validate(length(min = 1, max = 10))]
    pub queries: Vec<SearchRequest>,
    
    /// Cross-query options
    pub options: MultiSearchOptions,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MultiSearchOptions {
    /// Enable result deduplication
    pub deduplicate: bool,
    
    /// Merge strategy for multiple queries
    pub merge_strategy: MergeStrategy,
    
    /// Execute queries in parallel
    pub parallel_execution: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MergeStrategy {
    /// Union of all results
    Union,
    /// Intersection of results
    Intersection,
    /// Weighted combination
    Weighted,
}

/// Multi-query search response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MultiSearchResponse {
    pub individual_results: Vec<SearchResponse>,
    pub merged_results: Vec<SearchResult>,
    pub total_search_time_ms: u64,
    pub deduplication_stats: Option<DeduplicationStats>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeduplicationStats {
    pub original_count: usize,
    pub deduplicated_count: usize,
    pub duplicates_removed: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            enable_reranking: false,
            explain_scores: false,
            include_embeddings: false,
            strategy: SearchStrategy::Semantic,
            hybrid_weights: None,
        }
    }
}

impl Default for HybridWeights {
    fn default() -> Self {
        Self {
            semantic_weight: 0.7,
            keyword_weight: 0.2,
            recency_weight: 0.05,
            popularity_weight: 0.05,
        }
    }
}

impl Default for MultiSearchOptions {
    fn default() -> Self {
        Self {
            deduplicate: true,
            merge_strategy: MergeStrategy::Union,
            parallel_execution: true,
        }
    }
}


/// Search memory request (simplified for use case)
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct SearchMemoryRequest {
    #[validate(length(min = 1, max = 1000))]
    pub query: String,
    
    pub limit: Option<usize>,
    pub similarity_threshold: Option<f64>,
    pub layers: Option<Vec<LayerType>>,
    pub project: Option<String>,
    pub filters: Option<std::collections::HashMap<String, String>>,
    pub use_cache: bool,
}

/// Search memory response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchMemoryResponse {
    pub results: Vec<SearchResult>,
    pub total_results: usize,
    pub search_time_ms: u64,
    pub query_hash: String,
    pub layers_searched: Vec<LayerType>,
}

/// Similarity search request (using raw embeddings)
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct SimilaritySearchRequest {
    #[validate(length(min = 1, max = 4096))]
    pub query_embedding: Vec<f32>,
    
    pub limit: Option<usize>,
    pub similarity_threshold: Option<f64>,
    pub metadata_filters: Option<std::collections::HashMap<String, String>>,
}

/// Similarity search response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimilaritySearchResponse {
    pub results: Vec<SimilarityResult>,
    pub total_results: usize,
    pub search_time_ms: u64,
    pub embedding_dimensions: usize,
}

/// Individual similarity search result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimilarityResult {
    pub record_id: String,
    pub embedding: Vec<f32>,
    pub similarity_score: f64,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

impl Default for SearchMemoryRequest {
    fn default() -> Self {
        Self {
            query: String::new(),
            limit: Some(10),
            similarity_threshold: Some(0.7),
            layers: None,
            project: None,
            filters: None,
            use_cache: true,
        }
    }
}