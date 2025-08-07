//! Promotion DTOs for ML-driven layer promotion

use serde::{Deserialize, Serialize};
use validator::Validate;
use domain::value_objects::layer_type::LayerType;
use domain::value_objects::promotion_criteria::PromotionCriteria;

/// Promote records request DTO
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct PromoteRecordsRequest {
    /// Specific record IDs to promote (optional)
    pub record_ids: Option<Vec<String>>,
    
    /// Source layer for bulk promotion
    pub from_layer: Option<LayerType>,
    
    /// Target layer for promotion
    pub to_layer: LayerType,
    
    /// Custom promotion criteria
    pub criteria: Option<CustomPromotionCriteria>,
    
    /// Force promotion (bypass ML recommendation)
    pub force: bool,
    
    /// Dry run mode (preview only)
    pub dry_run: bool,
}

/// Custom promotion criteria override
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct CustomPromotionCriteria {
    /// Minimum access frequency
    #[validate(range(min = 1))]
    pub min_access_count: Option<u64>,
    
    /// Time window for analysis
    pub analysis_window_hours: Option<u64>,
    
    /// Minimum similarity score threshold
    #[validate(range(min = 0.0, max = 1.0))]
    pub min_similarity_score: Option<f32>,
    
    /// Project filter
    pub project_filter: Option<String>,
    
    /// Tag filters
    pub tag_filters: Vec<String>,
}

/// Promotion response DTO
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromoteRecordsResponse {
    pub analysis_id: String,
    pub total_candidates: usize,
    pub promoted_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub promotion_details: Vec<PromotionDetail>,
    pub analysis_time_ms: u64,
    pub dry_run: bool,
}

/// Individual promotion detail
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromotionDetail {
    pub record_id: String,
    pub from_layer: LayerType,
    pub to_layer: LayerType,
    pub success: bool,
    pub ml_confidence: f32,
    pub access_pattern_score: f32,
    pub similarity_boost: f32,
    pub promotion_reason: PromotionReason,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PromotionReason {
    HighAccessFrequency,
    SemanticSimilarity,
    RecentPopularity,
    ManualOverride,
    HybridML,
}

/// Analyze promotion candidates request
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalyzePromotionRequest {
    /// Layers to analyze
    pub source_layers: Vec<LayerType>,
    
    /// Target layer for analysis
    pub target_layer: LayerType,
    
    /// Analysis depth
    pub analysis_depth: AnalysisDepth,
    
    /// Time window for analysis
    pub time_window_hours: u64,
    
    /// Include ML predictions
    pub include_ml_predictions: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AnalysisDepth {
    Quick,      // Basic statistics only
    Standard,   // Include access patterns  
    Deep,       // Full ML analysis
    Exhaustive, // Complete pattern analysis
}

/// Promotion analysis response
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalyzePromotionResponse {
    pub analysis_id: String,
    pub candidates: Vec<PromotionCandidate>,
    pub layer_statistics: LayerStatistics,
    pub recommendations: Vec<PromotionRecommendation>,
    pub ml_model_metrics: Option<MLModelMetrics>,
    pub analysis_metadata: AnalysisMetadata,
}

/// Promotion candidate details
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromotionCandidate {
    pub record_id: String,
    pub current_layer: LayerType,
    pub recommended_layer: LayerType,
    pub confidence_score: f32,
    pub access_frequency: u64,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub similarity_cluster: Option<String>,
    pub business_value_score: f32,
    pub promotion_urgency: PromotionUrgency,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PromotionUrgency {
    Low,
    Medium,
    High,
    Critical,
}

/// Layer-wide statistics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LayerStatistics {
    pub cache_stats: LayerStats,
    pub index_stats: LayerStats,
    pub storage_stats: LayerStats,
    pub cross_layer_patterns: CrossLayerPatterns,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LayerStats {
    pub total_records: u64,
    pub total_size_mb: f64,
    pub average_access_frequency: f64,
    pub hottest_records: Vec<String>, // Top 10 record IDs
    pub coldest_records: Vec<String>, // Bottom 10 record IDs
    pub utilization_percentage: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrossLayerPatterns {
    pub promotion_velocity: f64, // Records/hour promoted
    pub demotion_velocity: f64,  // Records/hour demoted
    pub layer_transition_matrix: Vec<Vec<f32>>, // 3x3 matrix
    pub access_pattern_clusters: Vec<AccessCluster>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessCluster {
    pub cluster_id: String,
    pub record_count: u64,
    pub pattern_description: String,
    pub recommended_layer: LayerType,
}

/// ML-based promotion recommendations
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromotionRecommendation {
    pub recommendation_type: RecommendationType,
    pub priority: RecommendationPriority,
    pub description: String,
    pub impact_estimate: ImpactEstimate,
    pub action_required: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RecommendationType {
    BulkPromotion,
    IndividualPromotion,
    LayerRebalancing,
    CapacityAdjustment,
    AccessPatternOptimization,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImpactEstimate {
    pub performance_improvement_percent: f32,
    pub memory_usage_change_mb: i64,
    pub estimated_cost_change: f32,
    pub user_experience_impact: UserExperienceImpact,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserExperienceImpact {
    Negligible,
    Minor,
    Moderate,
    Significant,
    Major,
}

/// ML model performance metrics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MLModelMetrics {
    pub model_version: String,
    pub accuracy: f32,
    pub precision: f32,
    pub recall: f32,
    pub f1_score: f32,
    pub training_date: chrono::DateTime<chrono::Utc>,
    pub feature_importance: Vec<FeatureImportance>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeatureImportance {
    pub feature_name: String,
    pub importance_score: f32,
    pub description: String,
}

/// Analysis metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisMetadata {
    pub analysis_time_ms: u64,
    pub records_analyzed: u64,
    pub data_quality_score: f32,
    pub analysis_completeness: f32,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

impl Default for AnalysisDepth {
    fn default() -> Self {
        Self::Standard
    }
}

// Additional DTOs for promotion use case

/// Simplified promotion request DTO for use case
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct PromoteRecordsRequest {
    pub criteria: Vec<PromotionCriterion>,
    pub max_candidates: Option<usize>,
    pub dry_run: bool,
}

/// Individual promotion criterion
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromotionCriterion {
    pub min_access_frequency: u64,
    pub min_score_threshold: f64,
    pub max_hours_since_access: Option<u64>,
    pub target_layers: Vec<LayerType>,
    pub project_filter: Option<String>,
}

/// Simplified promotion response DTO
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromoteRecordsResponse {
    pub promoted_records: Vec<RecordPromotion>,
    pub failed_promotions: usize,
    pub dry_run: bool,
    pub total_processing_time_ms: u64,
    pub candidates_analysis_time_ms: u64,
    pub promotion_execution_time_ms: u64,
}

/// Individual record promotion result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecordPromotion {
    pub record_id: String,
    pub from_layer: LayerType,
    pub to_layer: LayerType,
    pub promotion_score: f64,
    pub promotion_reason: String,
    pub estimated_benefit: f64,
}

/// Analyze promotion request DTO
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct AnalyzePromotionRequest {
    pub layers: Option<Vec<LayerType>>,
    pub time_window_hours: Option<u32>,
    pub min_access_frequency: Option<u64>,
    pub min_score_threshold: Option<f64>,
    pub max_hours_since_access: Option<u64>,
}

/// Analyze promotion response DTO
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalyzePromotionResponse {
    pub candidates: Vec<PromotionCandidate>,
    pub layer_statistics: std::collections::HashMap<LayerType, LayerAnalysisStats>,
    pub analysis_time_ms: u64,
    pub total_records_analyzed: u64,
    pub recommendations_generated: usize,
}

/// Promotion candidate details
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromotionCandidate {
    pub record_id: String,
    pub current_layer: LayerType,
    pub recommended_layer: LayerType,
    pub promotion_score: f64,
    pub access_frequency: u64,
    pub last_access_hours_ago: u64,
    pub predicted_benefit: f64,
    pub confidence_score: f64,
    pub reasons: Vec<String>,
}

/// Layer analysis statistics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LayerAnalysisStats {
    pub record_count: u64,
    pub avg_access_frequency: f64,
    pub promotion_candidates: usize,
    pub demotion_candidates: usize,
    pub utilization_percentage: f64,
}

impl Default for PromoteRecordsRequest {
    fn default() -> Self {
        Self {
            criteria: Vec::new(),
            max_candidates: Some(100),
            dry_run: false,
        }
    }
}