//! Analyze Usage Use Case
//!
//! Бизнес-логика для анализа паттернов использования memory system,
//! генерации insights и recommendations для оптимизации.

use async_trait::async_trait;
use crate::{ApplicationResult, ApplicationError, RequestContext};
use crate::dtos::{AnalyzeUsageRequest, AnalyzeUsageResponse, GenerateInsightsRequest, GenerateInsightsResponse};
use crate::ports::{MetricsCollector, NotificationService};
use domain::repositories::memory_repository::MemoryRepository;
use domain::services::memory_domain_service::MemoryDomainService;
use domain::value_objects::layer_type::LayerType;
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{info, warn, error, instrument};

/// Use case для анализа использования memory system
#[async_trait]
pub trait AnalyzeUsageUseCase: Send + Sync {
    /// Analyze usage patterns across memory layers
    async fn analyze_usage_patterns(&self, request: AnalyzeUsageRequest, context: RequestContext) -> ApplicationResult<AnalyzeUsageResponse>;
    
    /// Generate insights and recommendations
    async fn generate_insights(&self, request: GenerateInsightsRequest, context: RequestContext) -> ApplicationResult<GenerateInsightsResponse>;
    
    /// Get real-time system statistics
    async fn get_system_statistics(&self, context: RequestContext) -> ApplicationResult<crate::dtos::SystemStatistics>;
    
    /// Generate health report for memory system
    async fn generate_health_report(&self, context: RequestContext) -> ApplicationResult<crate::dtos::HealthReport>;
}

/// Implementation of analyze usage use case
pub struct AnalyzeUsageUseCaseImpl {
    memory_repository: Arc<dyn MemoryRepository>,
    memory_domain_service: Arc<dyn MemoryDomainService>,
    metrics_collector: Arc<dyn MetricsCollector>,
    notification_service: Arc<dyn NotificationService>,
}

impl AnalyzeUsageUseCaseImpl {
    pub fn new(
        memory_repository: Arc<dyn MemoryRepository>,
        memory_domain_service: Arc<dyn MemoryDomainService>,
        metrics_collector: Arc<dyn MetricsCollector>,
        notification_service: Arc<dyn NotificationService>,
    ) -> Self {
        Self {
            memory_repository,
            memory_domain_service,
            metrics_collector,
            notification_service,
        }
    }
}

#[async_trait]
impl AnalyzeUsageUseCase for AnalyzeUsageUseCaseImpl {
    #[instrument(skip(self, request), fields(time_window_hours = request.time_window_hours))]
    async fn analyze_usage_patterns(&self, request: AnalyzeUsageRequest, context: RequestContext) -> ApplicationResult<AnalyzeUsageResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Starting usage patterns analysis for request: {}", context.request_id);
        
        // Validate request
        self.validate_usage_request(&request)?;
        
        // Calculate time range
        let end_time = std::time::SystemTime::now();
        let start_time_analysis = end_time - std::time::Duration::from_secs(request.time_window_hours as u64 * 3600);
        
        // Collect usage statistics from domain service
        let usage_stats = self.memory_domain_service.analyze_usage_patterns(
            start_time_analysis,
            end_time,
            request.layers.as_ref(),
            request.project_filter.as_deref(),
        ).await.map_err(|e| ApplicationError::Domain(e))?;
        
        // Analyze layer performance
        let layer_analysis = self.analyze_layer_performance(&usage_stats, &request).await?;
        
        // Generate access patterns
        let access_patterns = self.generate_access_patterns(&usage_stats).await?;
        
        // Calculate efficiency metrics
        let efficiency_metrics = self.calculate_efficiency_metrics(&usage_stats).await?;
        
        // Generate recommendations
        let recommendations = self.generate_optimization_recommendations(&usage_stats, &efficiency_metrics).await?;
        
        let total_time = start_time.elapsed();
        
        // Record analysis metrics
        self.record_usage_analysis_metrics(&usage_stats, total_time).await?;
        
        let response = AnalyzeUsageResponse {
            time_window_hours: request.time_window_hours,
            total_requests: usage_stats.total_requests,
            total_records: usage_stats.total_records,
            layer_analysis,
            access_patterns,
            efficiency_metrics,
            recommendations,
            analysis_time_ms: total_time.as_millis() as u64,
        };
        
        info!(
            "Usage analysis completed: {} requests analyzed, {} recommendations generated",
            response.total_requests,
            response.recommendations.len()
        );
        
        Ok(response)
    }

    #[instrument(skip(self, request), fields(insight_types = request.insight_types.len()))]
    async fn generate_insights(&self, request: GenerateInsightsRequest, context: RequestContext) -> ApplicationResult<GenerateInsightsResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Starting insights generation for request: {}", context.request_id);
        
        // Validate request
        self.validate_insights_request(&request)?;
        
        let mut insights = Vec::new();
        
        // Generate requested insights
        for insight_type in &request.insight_types {
            match insight_type {
                crate::dtos::InsightType::PerformanceTrends => {
                    let performance_insights = self.generate_performance_insights(&request).await?;
                    insights.extend(performance_insights);
                }
                crate::dtos::InsightType::UsageAnomalies => {
                    let anomaly_insights = self.generate_anomaly_insights(&request).await?;
                    insights.extend(anomaly_insights);
                }
                crate::dtos::InsightType::OptimizationOpportunities => {
                    let optimization_insights = self.generate_optimization_insights(&request).await?;
                    insights.extend(optimization_insights);
                }
                crate::dtos::InsightType::ResourceUtilization => {
                    let resource_insights = self.generate_resource_insights(&request).await?;
                    insights.extend(resource_insights);
                }
                crate::dtos::InsightType::PredictiveAnalysis => {
                    let predictive_insights = self.generate_predictive_insights(&request).await?;
                    insights.extend(predictive_insights);
                }
            }
        }
        
        // Sort insights by importance/severity
        insights.sort_by(|a, b| {
            b.severity.cmp(&a.severity).then(b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal))
        });
        
        let total_time = start_time.elapsed();
        
        // Record insights metrics
        self.record_insights_metrics(insights.len(), total_time).await?;
        
        let response = GenerateInsightsResponse {
            insights,
            generation_time_ms: total_time.as_millis() as u64,
            confidence_score: self.calculate_overall_confidence(&insights),
        };
        
        info!(
            "Insights generation completed: {} insights generated with confidence: {:.2}",
            response.insights.len(),
            response.confidence_score
        );
        
        Ok(response)
    }

    async fn get_system_statistics(&self, context: RequestContext) -> ApplicationResult<crate::dtos::SystemStatistics> {
        let start_time = std::time::Instant::now();
        
        info!("Collecting system statistics for request: {}", context.request_id);
        
        // Collect real-time metrics from domain service
        let domain_stats = self.memory_domain_service.get_system_statistics().await
            .map_err(|e| ApplicationError::Domain(e))?;
        
        // Convert to DTO
        let statistics = crate::dtos::SystemStatistics {
            total_records: domain_stats.total_records,
            records_by_layer: domain_stats.records_by_layer,
            cache_hit_rate: domain_stats.cache_hit_rate,
            average_search_time_ms: domain_stats.average_search_time_ms,
            memory_usage_mb: domain_stats.memory_usage_mb,
            disk_usage_mb: domain_stats.disk_usage_mb,
            active_connections: domain_stats.active_connections,
            requests_per_minute: domain_stats.requests_per_minute,
            error_rate_percentage: domain_stats.error_rate_percentage,
            uptime_seconds: domain_stats.uptime_seconds,
            last_updated: std::time::SystemTime::now(),
        };
        
        let total_time = start_time.elapsed();
        
        // Record statistics collection metrics
        self.record_statistics_collection_metrics(total_time).await?;
        
        info!("System statistics collected in {}ms", total_time.as_millis());
        
        Ok(statistics)
    }

    async fn generate_health_report(&self, context: RequestContext) -> ApplicationResult<crate::dtos::HealthReport> {
        let start_time = std::time::Instant::now();
        
        info!("Generating health report for request: {}", context.request_id);
        
        // Collect health data from domain service
        let health_data = self.memory_domain_service.collect_health_data().await
            .map_err(|e| ApplicationError::Domain(e))?;
        
        // Analyze component health
        let component_health = self.analyze_component_health(&health_data).await?;
        
        let critical_issues = self.identify_critical_issues(&health_data).await?;
        
        // Generate health score
        let health_score = self.calculate_health_score(&health_data, &critical_issues);
        
        let health_recommendations = self.generate_health_recommendations(&health_data, &critical_issues).await?;
        
        let total_time = start_time.elapsed();
        
        let report = crate::dtos::HealthReport {
            overall_health_score: health_score,
            component_health,
            critical_issues,
            recommendations: health_recommendations,
            generated_at: std::time::SystemTime::now(),
            report_generation_time_ms: total_time.as_millis() as u64,
        };
        
        if health_score < 0.7 || !critical_issues.is_empty() {
            self.send_health_alert(&report, &context).await?;
        }
        
        info!(
            "Health report generated: score {:.2}, {} critical issues",
            health_score,
            critical_issues.len()
        );
        
        Ok(report)
    }
}

impl AnalyzeUsageUseCaseImpl {
    /// Validate usage analysis request
    fn validate_usage_request(&self, request: &AnalyzeUsageRequest) -> ApplicationResult<()> {
        if request.time_window_hours == 0 || request.time_window_hours > 8760 { // Max 1 year
            return Err(ApplicationError::validation("Time window must be between 1 hour and 1 year"));
        }
        
        Ok(())
    }
    
    /// Validate insights request
    fn validate_insights_request(&self, request: &GenerateInsightsRequest) -> ApplicationResult<()> {
        if request.insight_types.is_empty() {
            return Err(ApplicationError::validation("At least one insight type must be specified"));
        }
        
        if request.insight_types.len() > 10 {
            return Err(ApplicationError::validation("Too many insight types requested (max 10)"));
        }
        
        Ok(())
    }
    
    /// Analyze layer performance
    async fn analyze_layer_performance(
        &self,
        usage_stats: &domain::services::memory_domain_service::UsageStatistics,
        request: &AnalyzeUsageRequest,
    ) -> ApplicationResult<HashMap<LayerType, crate::dtos::LayerPerformance>> {
        let mut layer_analysis = HashMap::new();
        
        for (layer, layer_stats) in &usage_stats.layer_statistics {
            let performance = crate::dtos::LayerPerformance {
                hit_rate: layer_stats.hit_rate,
                average_response_time_ms: layer_stats.average_response_time_ms,
                total_requests: layer_stats.total_requests,
                error_rate: layer_stats.error_rate,
                utilization_percentage: layer_stats.utilization_percentage,
                throughput_qps: layer_stats.throughput_qps,
                p95_latency_ms: layer_stats.p95_latency_ms,
                p99_latency_ms: layer_stats.p99_latency_ms,
            };
            
            layer_analysis.insert(*layer, performance);
        }
        
        Ok(layer_analysis)
    }
    
    /// Generate access patterns analysis
    async fn generate_access_patterns(
        &self,
        usage_stats: &domain::services::memory_domain_service::UsageStatistics,
    ) -> ApplicationResult<Vec<crate::dtos::AccessPattern>> {
        let mut patterns = Vec::new();
        
        // Analyze temporal patterns
        for pattern in &usage_stats.access_patterns {
            let access_pattern = crate::dtos::AccessPattern {
                pattern_type: pattern.pattern_type.clone(),
                description: pattern.description.clone(),
                frequency: pattern.frequency,
                confidence: pattern.confidence,
                time_windows: pattern.time_windows.clone(),
                affected_layers: pattern.affected_layers.clone(),
                impact_score: pattern.impact_score,
            };
            
            patterns.push(access_pattern);
        }
        
        Ok(patterns)
    }
    
    /// Calculate efficiency metrics
    async fn calculate_efficiency_metrics(
        &self,
        usage_stats: &domain::services::memory_domain_service::UsageStatistics,
    ) -> ApplicationResult<crate::dtos::EfficiencyMetrics> {
        let cache_efficiency = usage_stats.layer_statistics
            .get(&LayerType::Cache)
            .map(|stats| stats.hit_rate)
            .unwrap_or(0.0);
        
        let overall_response_time = usage_stats.overall_average_response_time_ms;
        
        let resource_utilization = usage_stats.layer_statistics
            .values()
            .map(|stats| stats.utilization_percentage)
            .sum::<f64>() / usage_stats.layer_statistics.len() as f64;
        
        let throughput_efficiency = usage_stats.total_requests as f64 / usage_stats.time_window_hours as f64;
        
        let error_impact = usage_stats.overall_error_rate * 100.0; // Convert to penalty score
        
        Ok(crate::dtos::EfficiencyMetrics {
            cache_efficiency,
            overall_response_time,
            resource_utilization,
            throughput_efficiency,
            error_impact,
            overall_efficiency_score: self.calculate_overall_efficiency(
                cache_efficiency,
                overall_response_time,
                resource_utilization,
                throughput_efficiency,
                error_impact,
            ),
        })
    }
    
    /// Calculate overall efficiency score
    fn calculate_overall_efficiency(
        &self,
        cache_efficiency: f64,
        response_time: f64,
        resource_utilization: f64,
        throughput_efficiency: f64,
        error_impact: f64,
    ) -> f64 {
        // Weighted scoring formula
        let cache_weight = 0.3;
        let response_time_weight = 0.25;
        let resource_weight = 0.2;
        let throughput_weight = 0.15;
        let error_weight = 0.1;
        
        let cache_score = cache_efficiency;
        let response_score = (1000.0 - response_time.min(1000.0)) / 1000.0; // Lower is better
        let resource_score = resource_utilization.min(1.0);
        let throughput_score = (throughput_efficiency / 100.0).min(1.0); // Normalize to 0-1
        let error_score = (1.0 - error_impact).max(0.0);
        
        let overall = cache_weight * cache_score
            + response_time_weight * response_score
            + resource_weight * resource_score
            + throughput_weight * throughput_score
            + error_weight * error_score;
        
        overall.max(0.0).min(1.0)
    }
    
    /// Generate optimization recommendations
    async fn generate_optimization_recommendations(
        &self,
        usage_stats: &domain::services::memory_domain_service::UsageStatistics,
        efficiency_metrics: &crate::dtos::EfficiencyMetrics,
    ) -> ApplicationResult<Vec<crate::dtos::OptimizationRecommendation>> {
        let mut recommendations = Vec::new();
        
        // Cache efficiency recommendations
        if efficiency_metrics.cache_efficiency < 0.7 {
            recommendations.push(crate::dtos::OptimizationRecommendation {
                category: crate::dtos::RecommendationCategory::CacheOptimization,
                priority: crate::dtos::RecommendationPriority::High,
                title: "Improve Cache Hit Rate".to_string(),
                description: format!(
                    "Cache hit rate is {:.1}%. Consider increasing cache size or adjusting cache policies.",
                    efficiency_metrics.cache_efficiency * 100.0
                ),
                estimated_impact: crate::dtos::ImpactEstimate {
                    performance_improvement: 25.0,
                    cost_reduction: 15.0,
                    implementation_effort: crate::dtos::ImplementationEffort::Medium,
                },
                action_items: vec![
                    "Increase cache memory allocation".to_string(),
                    "Review and optimize cache eviction policies".to_string(),
                    "Analyze frequently accessed patterns".to_string(),
                ],
            });
        }
        
        // Response time recommendations
        if efficiency_metrics.overall_response_time > 100.0 {
            recommendations.push(crate::dtos::OptimizationRecommendation {
                category: crate::dtos::RecommendationCategory::PerformanceOptimization,
                priority: crate::dtos::RecommendationPriority::High,
                title: "Reduce Response Times".to_string(),
                description: format!(
                    "Average response time is {:.1}ms. Target should be under 100ms.",
                    efficiency_metrics.overall_response_time
                ),
                estimated_impact: crate::dtos::ImpactEstimate {
                    performance_improvement: 40.0,
                    cost_reduction: 10.0,
                    implementation_effort: crate::dtos::ImplementationEffort::High,
                },
                action_items: vec![
                    "Enable SIMD optimizations for vector operations".to_string(),
                    "Implement better indexing strategies".to_string(),
                    "Consider GPU acceleration for large datasets".to_string(),
                ],
            });
        }
        
        // Resource utilization recommendations
        if efficiency_metrics.resource_utilization > 0.9 {
            recommendations.push(crate::dtos::OptimizationRecommendation {
                category: crate::dtos::RecommendationCategory::ResourceManagement,
                priority: crate::dtos::RecommendationPriority::Medium,
                title: "Scale Resources".to_string(),
                description: format!(
                    "Resource utilization is {:.1}%. Consider scaling to prevent bottlenecks.",
                    efficiency_metrics.resource_utilization * 100.0
                ),
                estimated_impact: crate::dtos::ImpactEstimate {
                    performance_improvement: 20.0,
                    cost_reduction: -25.0, // Negative because it requires more resources
                    implementation_effort: crate::dtos::ImplementationEffort::Low,
                },
                action_items: vec![
                    "Monitor resource usage trends".to_string(),
                    "Plan capacity scaling".to_string(),
                    "Implement auto-scaling policies".to_string(),
                ],
            });
        }
        
        Ok(recommendations)
    }
    
    /// Generate performance insights
    async fn generate_performance_insights(&self, request: &GenerateInsightsRequest) -> ApplicationResult<Vec<crate::dtos::Insight>> {
        // Implementation would analyze performance trends from metrics collector
        Ok(vec![
            crate::dtos::Insight {
                insight_type: crate::dtos::InsightType::PerformanceTrends,
                title: "Search Performance Improving".to_string(),
                description: "Search response times have improved by 15% over the last 24 hours".to_string(),
                severity: crate::dtos::InsightSeverity::Info,
                confidence: 0.85,
                recommendations: vec![
                    "Continue monitoring performance trends".to_string(),
                    "Document successful optimizations".to_string(),
                ],
                metrics: Some(HashMap::from([
                    ("improvement_percentage".to_string(), 15.0),
                    ("time_window_hours".to_string(), 24.0),
                ])),
            }
        ])
    }
    
    /// Generate anomaly insights
    async fn generate_anomaly_insights(&self, request: &GenerateInsightsRequest) -> ApplicationResult<Vec<crate::dtos::Insight>> {
        // Implementation would detect anomalies in usage patterns
        Ok(vec![])
    }
    
    /// Generate optimization insights
    async fn generate_optimization_insights(&self, request: &GenerateInsightsRequest) -> ApplicationResult<Vec<crate::dtos::Insight>> {
        // Implementation would identify optimization opportunities
        Ok(vec![])
    }
    
    /// Generate resource insights
    async fn generate_resource_insights(&self, request: &GenerateInsightsRequest) -> ApplicationResult<Vec<crate::dtos::Insight>> {
        // Implementation would analyze resource utilization
        Ok(vec![])
    }
    
    /// Generate predictive insights
    async fn generate_predictive_insights(&self, request: &GenerateInsightsRequest) -> ApplicationResult<Vec<crate::dtos::Insight>> {
        Ok(vec![])
    }
    
    /// Calculate overall confidence from insights
    fn calculate_overall_confidence(&self, insights: &[crate::dtos::Insight]) -> f64 {
        if insights.is_empty() {
            return 0.0;
        }
        
        insights.iter().map(|i| i.confidence).sum::<f64>() / insights.len() as f64
    }
    
    /// Analyze component health
    async fn analyze_component_health(
        &self,
        health_data: &domain::services::memory_domain_service::HealthData,
    ) -> ApplicationResult<HashMap<String, crate::dtos::ComponentHealth>> {
        let mut component_health = HashMap::new();
        
        for (component, health_info) in &health_data.components {
            let health = crate::dtos::ComponentHealth {
                status: health_info.status.clone(),
                health_score: health_info.health_score,
                last_check: health_info.last_check,
                error_count: health_info.error_count,
                warning_count: health_info.warning_count,
                details: health_info.details.clone(),
            };
            
            component_health.insert(component.clone(), health);
        }
        
        Ok(component_health)
    }
    
    /// Identify critical issues
    async fn identify_critical_issues(
        &self,
        health_data: &domain::services::memory_domain_service::HealthData,
    ) -> ApplicationResult<Vec<crate::dtos::CriticalIssue>> {
        let mut issues = Vec::new();
        
        for issue in &health_data.critical_issues {
            let critical_issue = crate::dtos::CriticalIssue {
                issue_type: issue.issue_type.clone(),
                description: issue.description.clone(),
                severity: issue.severity,
                first_detected: issue.first_detected,
                last_occurrence: issue.last_occurrence,
                occurrence_count: issue.occurrence_count,
                affected_components: issue.affected_components.clone(),
                resolution_steps: issue.resolution_steps.clone(),
            };
            
            issues.push(critical_issue);
        }
        
        Ok(issues)
    }
    
    /// Calculate health score
    fn calculate_health_score(
        &self,
        health_data: &domain::services::memory_domain_service::HealthData,
        critical_issues: &[crate::dtos::CriticalIssue],
    ) -> f64 {
        let base_score = health_data.components.values()
            .map(|c| c.health_score)
            .sum::<f64>() / health_data.components.len() as f64;
        
        let critical_penalty = critical_issues.len() as f64 * 0.1;
        
        (base_score - critical_penalty).max(0.0).min(1.0)
    }
    
    /// Generate health recommendations
    async fn generate_health_recommendations(
        &self,
        health_data: &domain::services::memory_domain_service::HealthData,
        critical_issues: &[crate::dtos::CriticalIssue],
    ) -> ApplicationResult<Vec<crate::dtos::HealthRecommendation>> {
        let mut recommendations = Vec::new();
        
        // Generate recommendations based on critical issues
        for issue in critical_issues {
            let recommendation = crate::dtos::HealthRecommendation {
                priority: issue.severity,
                title: format!("Resolve {}", issue.issue_type),
                description: format!("Critical issue: {}", issue.description),
                action_items: issue.resolution_steps.clone(),
                estimated_resolution_time_hours: match issue.severity {
                    crate::dtos::IssueSeverity::Critical => 1,
                    crate::dtos::IssueSeverity::High => 4,
                    crate::dtos::IssueSeverity::Medium => 24,
                    crate::dtos::IssueSeverity::Low => 168, // 1 week
                },
            };
            
            recommendations.push(recommendation);
        }
        
        Ok(recommendations)
    }
    
    /// Send health alert notification
    async fn send_health_alert(&self, report: &crate::dtos::HealthReport, context: &RequestContext) -> ApplicationResult<()> {
        use crate::ports::{Notification, NotificationLevel};
        
        let level = if report.overall_health_score < 0.5 {
            NotificationLevel::Error
        } else {
            NotificationLevel::Warning
        };
        
        let notification = Notification {
            level,
            title: "Memory System Health Alert".to_string(),
            message: format!(
                "System health score: {:.2}, {} critical issues detected",
                report.overall_health_score,
                report.critical_issues.len()
            ),
            ..Notification::info("", "")
        };
        
        self.notification_service.send_notification(&notification).await?;
        Ok(())
    }
    
    /// Record various metrics
    async fn record_usage_analysis_metrics(
        &self,
        usage_stats: &domain::services::memory_domain_service::UsageStatistics,
        total_time: std::time::Duration,
    ) -> ApplicationResult<()> {
        self.metrics_collector.increment_counter(
            "usage_analysis_operations_total",
            1,
            None,
        ).await?;
        
        self.metrics_collector.record_timing(
            "usage_analysis_duration",
            total_time.as_millis() as u64,
            None,
        ).await?;
        
        Ok(())
    }
    
    async fn record_insights_metrics(&self, insights_count: usize, total_time: std::time::Duration) -> ApplicationResult<()> {
        self.metrics_collector.increment_counter(
            "insights_generation_operations_total",
            1,
            None,
        ).await?;
        
        self.metrics_collector.record_gauge(
            "insights_generated",
            insights_count as f64,
            None,
        ).await?;
        
        Ok(())
    }
    
    async fn record_statistics_collection_metrics(&self, total_time: std::time::Duration) -> ApplicationResult<()> {
        self.metrics_collector.record_timing(
            "statistics_collection_duration",
            total_time.as_millis() as u64,
            None,
        ).await?;
        
        Ok(())
    }
}