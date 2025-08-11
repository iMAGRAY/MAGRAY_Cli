//! Memory Queries for CQRS Implementation
//!
//! Запросы для чтения данных из memory системы

use serde::{Deserialize, Serialize};
use validator::Validate;
use crate::{ApplicationResult, RequestContext};
use crate::dtos::{
    RetrieveMemoryRequest, RetrieveMemoryResponse,
    SearchMemoryRequest, SearchMemoryResponse,
    SimilaritySearchRequest, SimilaritySearchResponse,
    ListMemoryRequest, ListMemoryResponse,
    AnalyzeUsageRequest, AnalyzeUsageResponse,
    GenerateInsightsRequest, GenerateInsightsResponse,
    SystemStatistics, HealthReport,
    PaginationParams, DateRange, SortOptions
};
use domain::value_objects::layer_type::LayerType;
use domain::entities::embedding_vector::EmbeddingVector;

/// Query to retrieve a single memory record
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RetrieveMemoryQuery {
    #[validate(length(min = 1))]
    pub record_id: String,
    pub include_embedding: bool,
    pub include_stats: bool,
}

/// Query to search memory records by content
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SearchMemoryQuery {
    #[validate(length(min = 1, max = 1000))]
    pub query: String,
    pub layers: Vec<LayerType>,
    pub limit: usize,
    pub include_embeddings: bool,
    pub project_filter: Option<String>,
    pub tag_filters: Vec<String>,
}

/// Query for similarity search using embedding vector
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SimilaritySearchQuery {
    pub query_embedding: Vec<f32>,
    pub limit: usize,
    pub threshold: Option<f64>,
    pub layers: Vec<LayerType>,
    pub include_vectors: bool,
}

/// Query to list memory records with filters
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ListMemoryQuery {
    #[validate]
    pub pagination: PaginationParams,
    pub project: Option<String>,
    pub layer: Option<LayerType>,
    pub tags: Vec<String>,
    pub date_range: Option<DateRange>,
    pub sort: SortOptions,
}

/// Query for memory usage analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeUsageQuery {
    pub time_window_hours: Option<u64>,
    pub layers: Option<Vec<LayerType>>,
    pub project_filter: Option<String>,
    pub include_patterns: bool,
    pub include_recommendations: bool,
}

/// Query to generate system insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateInsightsQuery {
    pub analysis_type: InsightType,
    pub time_range: Option<DateRange>,
    pub layers: Option<Vec<LayerType>>,
    pub detail_level: InsightDetailLevel,
}

/// Query for system statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSystemStatisticsQuery {
    pub include_performance_metrics: bool,
    pub include_layer_breakdown: bool,
    pub include_storage_details: bool,
    pub include_cache_stats: bool,
}

/// Query for health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetHealthReportQuery {
    pub include_detailed_checks: bool,
    pub check_connectivity: bool,
    pub check_performance: bool,
    pub include_recommendations: bool,
}

/// Query to get record history and access patterns
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct GetRecordHistoryQuery {
    #[validate(length(min = 1))]
    pub record_id: String,
    pub include_access_log: bool,
    pub include_promotion_history: bool,
    pub include_related_records: bool,
    pub max_history_entries: Option<usize>,
}

/// Query for memory layer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetLayerStatisticsQuery {
    pub layers: Vec<LayerType>,
    pub include_performance: bool,
    pub include_capacity: bool,
    pub include_trends: bool,
    pub time_window_hours: Option<u64>,
}

/// Query for similar records
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct FindSimilarRecordsQuery {
    #[validate(length(min = 1))]
    pub reference_record_id: String,
    pub similarity_threshold: f64,
    pub limit: usize,
    pub layers: Vec<LayerType>,
    pub include_explanation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightType {
    UsagePatterns,
    PerformanceAnalysis,
    StorageOptimization,
    AccessFrequency,
    LayerEfficiency,
    PredictiveAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsightDetailLevel {
    Summary,
    Detailed,
    Comprehensive,
}

// Query implementations using the CQRS macro
crate::impl_cqrs!(query RetrieveMemoryQuery => RetrieveMemoryResponse);
crate::impl_cqrs!(query SearchMemoryQuery => SearchMemoryResponse);
crate::impl_cqrs!(query SimilaritySearchQuery => SimilaritySearchResponse);
crate::impl_cqrs!(query ListMemoryQuery => ListMemoryResponse);
crate::impl_cqrs!(query AnalyzeUsageQuery => AnalyzeUsageResponse);
crate::impl_cqrs!(query GenerateInsightsQuery => GenerateInsightsResponse);
crate::impl_cqrs!(query GetSystemStatisticsQuery => SystemStatistics);
crate::impl_cqrs!(query GetHealthReportQuery => HealthReport);
crate::impl_cqrs!(query GetRecordHistoryQuery => RecordHistoryResponse);
crate::impl_cqrs!(query GetLayerStatisticsQuery => LayerStatisticsResponse);
crate::impl_cqrs!(query FindSimilarRecordsQuery => SimilarRecordsResponse);

/// Response for record history query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordHistoryResponse {
    pub record_id: String,
    pub access_log: Vec<AccessLogEntry>,
    pub promotion_history: Vec<PromotionLogEntry>,
    pub related_records: Vec<RelatedRecord>,
    pub creation_date: chrono::DateTime<chrono::Utc>,
    pub last_modified: chrono::DateTime<chrono::Utc>,
    pub total_accesses: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessLogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub access_type: AccessType,
    pub response_time_ms: u64,
    pub layer: LayerType,
    pub source_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessType {
    Read,
    Search,
    Update,
    Promote,
    Backup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionLogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub from_layer: LayerType,
    pub to_layer: LayerType,
    pub reason: String,
    pub confidence_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedRecord {
    pub record_id: String,
    pub similarity_score: f64,
    pub relation_type: RelationType,
    pub content_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationType {
    Similar,
    Related,
    Reference,
    Duplicate,
}

/// Response for layer statistics query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStatisticsResponse {
    pub layer_stats: std::collections::HashMap<LayerType, LayerStats>,
    pub overall_performance: PerformanceMetrics,
    pub capacity_analysis: CapacityAnalysis,
    pub trends: Option<TrendAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStats {
    pub record_count: u64,
    pub total_size_bytes: u64,
    pub average_record_size: u64,
    pub access_frequency: f64,
    pub hit_rate: f32,
    pub average_response_time_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub queries_per_second: f64,
    pub average_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub error_rate: f32,
    pub throughput_mbps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityAnalysis {
    pub used_capacity_bytes: u64,
    pub total_capacity_bytes: u64,
    pub utilization_percent: f32,
    pub estimated_growth_rate: f64,
    pub time_to_capacity_days: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub usage_trend: TrendDirection,
    pub performance_trend: TrendDirection,
    pub capacity_trend: TrendDirection,
    pub predictions: Vec<TrendPrediction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
    Volatile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendPrediction {
    pub metric: String,
    pub predicted_value: f64,
    pub confidence: f32,
    pub time_horizon_days: u32,
}

/// Response for similar records query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarRecordsResponse {
    pub reference_record_id: String,
    pub similar_records: Vec<SimilarRecord>,
    pub search_performance: SearchPerformance,
    pub similarity_analysis: Option<SimilarityAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarRecord {
    pub record_id: String,
    pub content_preview: String,
    pub similarity_score: f64,
    pub layer: LayerType,
    pub explanation: Option<String>,
    pub common_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPerformance {
    pub search_time_ms: u64,
    pub records_examined: u64,
    pub similarity_calculations: u64,
    pub cache_hits: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityAnalysis {
    pub similarity_distribution: Vec<f64>,
    pub cluster_analysis: Option<ClusterInfo>,
    pub feature_importance: Vec<FeatureWeight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    pub cluster_id: u32,
    pub cluster_size: usize,
    pub cluster_centroid: Vec<f32>,
    pub intra_cluster_similarity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureWeight {
    pub feature_name: String,
    pub importance: f64,
    pub contribution: f64,
}

// Conversion implementations
impl From<RetrieveMemoryRequest> for RetrieveMemoryQuery {
    fn from(request: RetrieveMemoryRequest) -> Self {
        Self {
            record_id: request.record_id,
            include_embedding: request.include_embedding,
            include_stats: request.include_stats,
        }
    }
}

impl From<SearchMemoryRequest> for SearchMemoryQuery {
    fn from(request: SearchMemoryRequest) -> Self {
        Self {
            query: request.query,
            layers: request.layers,
            limit: request.limit,
            include_embeddings: request.include_embeddings,
            project_filter: request.project_filter,
            tag_filters: request.tag_filters,
        }
    }
}

impl From<SimilaritySearchRequest> for SimilaritySearchQuery {
    fn from(request: SimilaritySearchRequest) -> Self {
        Self {
            query_embedding: request.query_embedding,
            limit: request.limit,
            threshold: request.threshold,
            layers: request.layers,
            include_vectors: request.include_vectors,
        }
    }
}

impl From<ListMemoryRequest> for ListMemoryQuery {
    fn from(request: ListMemoryRequest) -> Self {
        Self {
            pagination: request.pagination,
            project: request.project,
            layer: request.layer,
            tags: request.tags,
            date_range: request.date_range,
            sort: request.sort,
        }
    }
}

impl From<AnalyzeUsageRequest> for AnalyzeUsageQuery {
    fn from(request: AnalyzeUsageRequest) -> Self {
        Self {
            time_window_hours: request.time_window_hours,
            layers: request.layers,
            project_filter: request.project_filter,
            include_patterns: true,
            include_recommendations: true,
        }
    }
}

impl From<GenerateInsightsRequest> for GenerateInsightsQuery {
    fn from(request: GenerateInsightsRequest) -> Self {
        Self {
            analysis_type: InsightType::UsagePatterns, // Default mapping
            time_range: request.time_range,
            layers: request.layers,
            detail_level: InsightDetailLevel::Detailed,
        }
    }
}

impl RetrieveMemoryQuery {
    pub fn new(record_id: &str) -> Self {
        Self {
            record_id: record_id.to_string(),
            include_embedding: false,
            include_stats: false,
        }
    }

    pub fn with_embedding(mut self) -> Self {
        self.include_embedding = true;
        self
    }

    pub fn with_stats(mut self) -> Self {
        self.include_stats = true;
        self
    }
}

impl SearchMemoryQuery {
    pub fn new(query: &str, limit: usize) -> Self {
        Self {
            query: query.to_string(),
            layers: vec![LayerType::Cache, LayerType::Index, LayerType::Storage],
            limit,
            include_embeddings: false,
            project_filter: None,
            tag_filters: Vec::new(),
        }
    }

    pub fn in_layers(mut self, layers: Vec<LayerType>) -> Self {
        self.layers = layers;
        self
    }

    pub fn with_embeddings(mut self) -> Self {
        self.include_embeddings = true;
        self
    }

    pub fn for_project(mut self, project: &str) -> Self {
        self.project_filter = Some(project.to_string());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tag_filters = tags;
        self
    }
}

impl SimilaritySearchQuery {
    pub fn new(query_embedding: Vec<f32>, limit: usize) -> Self {
        Self {
            query_embedding,
            limit,
            threshold: None,
            layers: vec![LayerType::Cache, LayerType::Index],
            include_vectors: false,
        }
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = Some(threshold);
        self
    }

    pub fn with_vectors(mut self) -> Self {
        self.include_vectors = true;
        self
    }

    pub fn in_layers(mut self, layers: Vec<LayerType>) -> Self {
        self.layers = layers;
        self
    }
}

impl GenerateInsightsQuery {
    pub fn new(analysis_type: InsightType) -> Self {
        Self {
            analysis_type,
            time_range: None,
            layers: None,
            detail_level: InsightDetailLevel::Detailed,
        }
    }

    pub fn with_time_range(mut self, time_range: DateRange) -> Self {
        self.time_range = Some(time_range);
        self
    }

    pub fn for_layers(mut self, layers: Vec<LayerType>) -> Self {
        self.layers = Some(layers);
        self
    }

    pub fn with_detail_level(mut self, level: InsightDetailLevel) -> Self {
        self.detail_level = level;
        self
    }
}

impl Default for InsightType {
    fn default() -> Self {
        Self::UsagePatterns
    }
}

impl Default for InsightDetailLevel {
    fn default() -> Self {
        Self::Detailed
    }
}