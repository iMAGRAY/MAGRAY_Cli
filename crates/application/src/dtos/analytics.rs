//! Analytics DTOs for usage analysis and reporting

use serde::{Deserialize, Serialize};
use validator::Validate;
use domain::value_objects::layer_type::LayerType;

/// Usage analysis request DTO
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct UsageAnalysisRequest {
    /// Time range for analysis
    pub time_range: TimeRange,
    
    /// Analysis dimensions
    pub dimensions: Vec<AnalysisDimension>,
    
    /// Aggregation level
    pub aggregation: AggregationLevel,
    
    /// Filters
    pub filters: AnalysisFilters,
    
    /// Report format preferences
    pub format_options: FormatOptions,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeRange {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
    pub timezone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AnalysisDimension {
    AccessPatterns,
    LayerPerformance,
    SearchQueries,
    PromotionEffectiveness,
    ResourceUtilization,
    UserBehavior,
    ContentAnalysis,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AggregationLevel {
    Minute,
    Hour,
    Day,
    Week,
    Month,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AnalysisFilters {
    pub projects: Vec<String>,
    pub layers: Vec<LayerType>,
    pub users: Vec<String>,
    pub tags: Vec<String>,
    pub record_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FormatOptions {
    pub include_charts: bool,
    pub include_raw_data: bool,
    pub include_recommendations: bool,
    pub chart_format: ChartFormat,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ChartFormat {
    Svg,
    Png,
    Json, // Chart.js format
}

/// Usage analysis response DTO
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsageAnalysisResponse {
    pub analysis_id: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub time_range: TimeRange,
    pub summary: UsageSummary,
    pub detailed_metrics: DetailedMetrics,
    pub insights: Vec<AnalysisInsight>,
    pub recommendations: Vec<AnalysisRecommendation>,
    pub charts: Option<Vec<ChartData>>,
    pub raw_data: Option<serde_json::Value>,
}

/// High-level usage summary
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsageSummary {
    pub total_operations: u64,
    pub unique_records_accessed: u64,
    pub average_operations_per_day: f64,
    pub peak_usage_time: chrono::DateTime<chrono::Utc>,
    pub layer_distribution: LayerUsageDistribution,
    pub performance_overview: PerformanceOverview,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LayerUsageDistribution {
    pub cache_usage_percent: f32,
    pub index_usage_percent: f32,
    pub storage_usage_percent: f32,
    pub cross_layer_queries_percent: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerformanceOverview {
    pub average_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub cache_hit_rate: f32,
    pub error_rate: f32,
}

/// Detailed metrics breakdown
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetailedMetrics {
    pub access_patterns: AccessPatternMetrics,
    pub layer_performance: LayerPerformanceMetrics,
    pub search_analytics: SearchAnalytics,
    pub promotion_metrics: PromotionMetrics,
    pub resource_metrics: ResourceMetrics,
}

/// Access pattern analysis
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessPatternMetrics {
    pub temporal_patterns: Vec<TemporalPattern>,
    pub hottest_records: Vec<HotRecord>,
    pub access_clusters: Vec<AccessCluster>,
    pub user_behavior_patterns: Vec<UserBehaviorPattern>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TemporalPattern {
    pub period: String, // "hourly", "daily", "weekly"
    pub pattern_data: Vec<TimeSeriesPoint>,
    pub peak_hours: Vec<u8>,
    pub low_activity_hours: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeSeriesPoint {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub value: f64,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HotRecord {
    pub record_id: String,
    pub access_count: u64,
    pub unique_users: u64,
    pub layer: LayerType,
    pub content_preview: String,
    pub popularity_trend: PopularityTrend,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PopularityTrend {
    Rising,
    Stable,
    Declining,
    Seasonal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessCluster {
    pub cluster_id: String,
    pub record_count: u64,
    pub access_pattern_description: String,
    pub representative_records: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserBehaviorPattern {
    pub user_id: String,
    pub session_count: u64,
    pub average_session_length_minutes: f64,
    pub preferred_content_types: Vec<String>,
    pub search_patterns: Vec<String>,
}

/// Layer performance metrics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LayerPerformanceMetrics {
    pub cache_performance: LayerPerformance,
    pub index_performance: LayerPerformance,
    pub storage_performance: LayerPerformance,
    pub layer_transitions: LayerTransitionMetrics,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LayerPerformance {
    pub average_response_time_ms: f64,
    pub throughput_ops_per_second: f64,
    pub hit_rate: f32,
    pub error_rate: f32,
    pub capacity_utilization: f32,
    pub performance_trend: Vec<TimeSeriesPoint>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LayerTransitionMetrics {
    pub promotions: u64,
    pub demotions: u64,
    pub transition_success_rate: f32,
    pub average_transition_time_ms: f64,
}

/// Search analytics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchAnalytics {
    pub query_patterns: Vec<QueryPattern>,
    pub popular_queries: Vec<PopularQuery>,
    pub search_effectiveness: SearchEffectiveness,
    pub user_satisfaction_metrics: UserSatisfactionMetrics,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryPattern {
    pub pattern_type: String,
    pub frequency: u64,
    pub examples: Vec<String>,
    pub effectiveness_score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PopularQuery {
    pub query_text: String,
    pub frequency: u64,
    pub average_results: f64,
    pub average_response_time_ms: f64,
    pub satisfaction_score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchEffectiveness {
    pub average_results_per_query: f64,
    pub zero_result_rate: f32,
    pub click_through_rate: f32,
    pub average_result_relevance: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSatisfactionMetrics {
    pub average_satisfaction_score: f32,
    pub satisfaction_trend: Vec<TimeSeriesPoint>,
    pub common_dissatisfaction_reasons: Vec<String>,
}

/// Promotion effectiveness metrics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromotionMetrics {
    pub promotion_activity: PromotionActivity,
    pub effectiveness_scores: EffectivenessScores,
    pub ml_model_performance: MLModelPerformance,
    pub cost_benefit_analysis: CostBenefitAnalysis,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromotionActivity {
    pub total_promotions: u64,
    pub successful_promotions: u64,
    pub failed_promotions: u64,
    pub average_promotion_time_ms: f64,
    pub promotion_frequency_trend: Vec<TimeSeriesPoint>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EffectivenessScores {
    pub accuracy_improvement: f32,
    pub response_time_improvement: f32,
    pub user_satisfaction_improvement: f32,
    pub resource_efficiency_improvement: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MLModelPerformance {
    pub prediction_accuracy: f32,
    pub false_positive_rate: f32,
    pub false_negative_rate: f32,
    pub model_drift_score: f32,
    pub retraining_recommendation: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CostBenefitAnalysis {
    pub promotion_costs: f64,
    pub performance_benefits: f64,
    pub roi_percentage: f32,
    pub break_even_point_days: Option<u32>,
}

/// Resource utilization metrics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResourceMetrics {
    pub memory_usage: ResourceUsageMetrics,
    pub cpu_usage: ResourceUsageMetrics,
    pub storage_usage: ResourceUsageMetrics,
    pub network_usage: ResourceUsageMetrics,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResourceUsageMetrics {
    pub average_utilization: f32,
    pub peak_utilization: f32,
    pub utilization_trend: Vec<TimeSeriesPoint>,
    pub efficiency_score: f32,
}

/// Analysis insights
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisInsight {
    pub insight_type: InsightType,
    pub title: String,
    pub description: String,
    pub confidence_level: ConfidenceLevel,
    pub impact_level: ImpactLevel,
    pub supporting_data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InsightType {
    Performance,
    Usage,
    Optimization,
    Anomaly,
    Trend,
    Prediction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConfidenceLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ImpactLevel {
    Minimal,
    Low,
    Medium,
    High,
    Critical,
}

/// Analysis recommendations
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisRecommendation {
    pub recommendation_type: RecommendationType,
    pub priority: RecommendationPriority,
    pub title: String,
    pub description: String,
    pub estimated_impact: ImpactEstimate,
    pub implementation_effort: ImplementationEffort,
    pub timeline_estimate: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RecommendationType {
    Configuration,
    Optimization,
    Scaling,
    Monitoring,
    UserExperience,
    DataQuality,
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
    pub performance_impact: String,
    pub cost_impact: String,
    pub user_experience_impact: String,
    pub quantified_benefit: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ImplementationEffort {
    Trivial,    // < 1 hour
    Low,        // 1-4 hours
    Medium,     // 1-3 days
    High,       // 1-2 weeks
    VeryHigh,   // > 2 weeks
}

/// Chart data for visualizations
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChartData {
    pub chart_id: String,
    pub chart_type: ChartType,
    pub title: String,
    pub data: serde_json::Value,
    pub options: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ChartType {
    Line,
    Bar,
    Pie,
    Scatter,
    Heatmap,
    Histogram,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            include_charts: true,
            include_raw_data: false,
            include_recommendations: true,
            chart_format: ChartFormat::Json,
        }
    }
}


/// Request to analyze memory usage patterns
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct AnalyzeUsageRequest {
    #[validate(range(min = 1, max = 8760))]
    pub time_window_hours: u32,
    
    pub layers: Option<Vec<LayerType>>,
    pub project_filter: Option<String>,
    pub include_detailed_breakdown: bool,
}

/// Response with usage analysis results
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalyzeUsageResponse {
    pub time_window_hours: u32,
    pub total_requests: u64,
    pub total_records: u64,
    pub layer_analysis: std::collections::HashMap<LayerType, LayerPerformance>,
    pub access_patterns: Vec<AccessPattern>,
    pub efficiency_metrics: EfficiencyMetrics,
    pub recommendations: Vec<OptimizationRecommendation>,
    pub analysis_time_ms: u64,
}

/// Access pattern analysis
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessPattern {
    pub pattern_type: String,
    pub description: String,
    pub frequency: u64,
    pub confidence: f64,
    pub time_windows: Vec<String>,
    pub affected_layers: Vec<LayerType>,
    pub impact_score: f64,
}

/// Efficiency metrics for the memory system
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EfficiencyMetrics {
    pub cache_efficiency: f64,
    pub overall_response_time: f64,
    pub resource_utilization: f64,
    pub throughput_efficiency: f64,
    pub error_impact: f64,
    pub overall_efficiency_score: f64,
}

/// Optimization recommendation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptimizationRecommendation {
    pub category: RecommendationCategory,
    pub priority: RecommendationPriority,
    pub title: String,
    pub description: String,
    pub estimated_impact: ImpactEstimate,
    pub action_items: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RecommendationCategory {
    CacheOptimization,
    PerformanceOptimization,
    ResourceManagement,
    ConfigurationTuning,
    ArchitectureImprovement,
}

/// Request to generate insights
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct GenerateInsightsRequest {
    pub insight_types: Vec<InsightType>,
    pub time_window_hours: Option<u32>,
    pub include_predictions: bool,
    pub confidence_threshold: Option<f64>,
}

/// Response with generated insights
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenerateInsightsResponse {
    pub insights: Vec<Insight>,
    pub generation_time_ms: u64,
    pub confidence_score: f64,
}

/// Individual insight
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Insight {
    pub insight_type: InsightType,
    pub title: String,
    pub description: String,
    pub severity: InsightSeverity,
    pub confidence: f64,
    pub recommendations: Vec<String>,
    pub metrics: Option<std::collections::HashMap<String, f64>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InsightType {
    PerformanceTrends,
    UsageAnomalies,
    OptimizationOpportunities,
    ResourceUtilization,
    PredictiveAnalysis,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum InsightSeverity {
    Info = 1,
    Warning = 2,
    High = 3,
    Critical = 4,
}

/// System statistics DTO
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemStatistics {
    pub total_records: u64,
    pub records_by_layer: std::collections::HashMap<LayerType, u64>,
    pub cache_hit_rate: f64,
    pub average_search_time_ms: f64,
    pub memory_usage_mb: f64,
    pub disk_usage_mb: f64,
    pub active_connections: u32,
    pub requests_per_minute: f64,
    pub error_rate_percentage: f64,
    pub uptime_seconds: u64,
    pub last_updated: std::time::SystemTime,
}

/// Health report DTO
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HealthReport {
    pub overall_health_score: f64,
    pub component_health: std::collections::HashMap<String, ComponentHealth>,
    pub critical_issues: Vec<CriticalIssue>,
    pub recommendations: Vec<HealthRecommendation>,
    pub generated_at: std::time::SystemTime,
    pub report_generation_time_ms: u64,
}

/// Component health status
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentHealth {
    pub status: String,
    pub health_score: f64,
    pub last_check: std::time::SystemTime,
    pub error_count: u32,
    pub warning_count: u32,
    pub details: std::collections::HashMap<String, String>,
}

/// Critical issue
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CriticalIssue {
    pub issue_type: String,
    pub description: String,
    pub severity: IssueSeverity,
    pub first_detected: std::time::SystemTime,
    pub last_occurrence: std::time::SystemTime,
    pub occurrence_count: u32,
    pub affected_components: Vec<String>,
    pub resolution_steps: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Health recommendation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HealthRecommendation {
    pub priority: IssueSeverity,
    pub title: String,
    pub description: String,
    pub action_items: Vec<String>,
    pub estimated_resolution_time_hours: u32,
}

/// Re-export common pattern from ImpactEstimate for use cases
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImpactEstimate {
    pub performance_improvement: f64,
    pub cost_reduction: f64,
    pub implementation_effort: ImplementationEffort,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ImplementationEffort {
    Low,
    Medium,
    High,
}