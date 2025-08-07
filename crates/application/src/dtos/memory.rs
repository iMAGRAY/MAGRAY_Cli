//! Memory DTOs for Store/Retrieve operations

use serde::{Deserialize, Serialize};
use validator::Validate;
use domain::entities::memory_record::MemoryRecord;
use domain::value_objects::layer_type::LayerType;
use super::{PaginationParams, PaginationMeta};

/// Store memory request DTO
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct StoreMemoryRequest {
    /// Text content to store
    #[validate(length(min = 1, max = 100000))]
    pub content: String,
    
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
    
    /// Project context
    pub project: Option<String>,
    
    /// Explicit layer preference
    pub target_layer: Option<LayerType>,
    
    /// Priority hint
    pub priority: Option<u8>,
    
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Store memory response DTO
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StoreMemoryResponse {
    pub record_id: String,
    pub layer: LayerType,
    pub embedding_dimensions: usize,
    pub processing_time_ms: u64,
    pub estimated_retrieval_time_ms: u64,
}

/// Batch store memory request
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct BatchStoreMemoryRequest {
    #[validate(length(min = 1, max = 100))]
    pub records: Vec<StoreMemoryRequest>,
    
    /// Batch processing options
    pub options: BatchOptions,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchOptions {
    pub parallel_processing: bool,
    pub failure_tolerance: FailureTolerance,
    pub progress_reporting: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FailureTolerance {
    /// Stop on first failure
    Strict,
    /// Continue processing, return partial results
    Partial,
    /// Best effort, ignore individual failures
    BestEffort,
}

/// Batch store response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchStoreMemoryResponse {
    pub total_requested: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<BatchStoreResult>,
    pub total_processing_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchStoreResult {
    pub index: usize,
    pub success: bool,
    pub record_id: Option<String>,
    pub error: Option<String>,
    pub layer: Option<LayerType>,
}

/// Retrieve memory request DTO
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct RetrieveMemoryRequest {
    #[validate(length(min = 1))]
    pub record_id: String,
    
    /// Include embedding vector in response
    pub include_embedding: bool,
    
    /// Include access statistics
    pub include_stats: bool,
}

/// Retrieve memory response DTO
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RetrieveMemoryResponse {
    pub record_id: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub layer: LayerType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub access_count: u64,
    pub embedding: Option<Vec<f32>>,
    pub stats: Option<RecordStats>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecordStats {
    pub retrieval_time_ms: u64,
    pub cache_hit: bool,
    pub layer_promotion_candidate: bool,
    pub similarity_scores: Option<Vec<SimilarityScore>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimilarityScore {
    pub record_id: String,
    pub score: f32,
    pub layer: LayerType,
}

/// List memory records request
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct ListMemoryRequest {
    /// Pagination parameters
    #[validate]
    pub pagination: PaginationParams,
    
    /// Filter by project
    pub project: Option<String>,
    
    /// Filter by layer
    pub layer: Option<LayerType>,
    
    /// Filter by tags
    pub tags: Vec<String>,
    
    /// Date range filter
    pub date_range: Option<DateRange>,
    
    /// Sort options
    pub sort: SortOptions,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DateRange {
    pub from: chrono::DateTime<chrono::Utc>,
    pub to: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SortOptions {
    pub field: SortField,
    pub direction: SortDirection,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SortField {
    CreatedAt,
    LastAccessed,
    AccessCount,
    Relevance,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// List memory response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListMemoryResponse {
    pub records: Vec<MemoryRecordSummary>,
    pub pagination: PaginationMeta,
    pub total_size_mb: f64,
    pub layer_distribution: LayerDistribution,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryRecordSummary {
    pub record_id: String,
    pub content_preview: String, // First 200 chars
    pub layer: LayerType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub access_count: u64,
    pub tags: Vec<String>,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LayerDistribution {
    pub cache_count: u64,
    pub index_count: u64,
    pub storage_count: u64,
}

impl Default for BatchOptions {
    fn default() -> Self {
        Self {
            parallel_processing: true,
            failure_tolerance: FailureTolerance::Partial,
            progress_reporting: false,
        }
    }
}

impl Default for SortOptions {
    fn default() -> Self {
        Self {
            field: SortField::LastAccessed,
            direction: SortDirection::Desc,
        }
    }
}