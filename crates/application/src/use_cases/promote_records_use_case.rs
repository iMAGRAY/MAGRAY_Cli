//! Promote Records Use Case
//!
//! Бизнес-логика для продвижения записей между слоями памяти на основе
//! usage patterns, ML predictions и бизнес-правил.

use crate::dtos::{
    AnalyzePromotionRequest, AnalyzePromotionResponse, PromoteRecordsRequest,
    PromoteRecordsResponse, PromotionCandidate,
};
use crate::ports::metrics_collector::MetricsCollectorExt;
use crate::ports::{MetricsCollector, NotificationService};
use crate::{ApplicationError, ApplicationResult, RequestContext};
use async_trait::async_trait;
use domain::services::promotion_domain_service::PromotionDomainServiceTrait;
use domain::MemoryRepository;
use domain::PromotionCriteria;
use domain::{LayerType, RecordId};
use std::sync::Arc;
use tracing::{error, info, instrument, warn};

/// Use case для продвижения записей между слоями
#[async_trait]
pub trait PromoteRecordsUseCase: Send + Sync {
    /// Promote records based on usage patterns and ML predictions
    async fn promote_records(
        &self,
        request: PromoteRecordsRequest,
        context: RequestContext,
    ) -> ApplicationResult<PromoteRecordsResponse>;

    /// Analyze promotion candidates without executing promotion
    async fn analyze_promotion_candidates(
        &self,
        request: AnalyzePromotionRequest,
        context: RequestContext,
    ) -> ApplicationResult<AnalyzePromotionResponse>;

    /// Force promotion of specific records (administrative operation)
    async fn force_promote_records(
        &self,
        record_ids: Vec<String>,
        target_layer: LayerType,
        context: RequestContext,
    ) -> ApplicationResult<PromoteRecordsResponse>;

    /// Run automated promotion cycle (background job)
    async fn run_automated_promotion_cycle(
        &self,
        context: RequestContext,
    ) -> ApplicationResult<PromoteRecordsResponse>;
}

/// Implementation of promote records use case
pub struct PromoteRecordsUseCaseImpl {
    memory_repository: Arc<dyn MemoryRepository>,
    promotion_domain_service: Arc<dyn PromotionDomainServiceTrait>,
    metrics_collector: Arc<dyn MetricsCollector>,
    notification_service: Arc<dyn NotificationService>,
}

impl PromoteRecordsUseCaseImpl {
    pub fn new(
        memory_repository: Arc<dyn MemoryRepository>,
        promotion_domain_service: Arc<dyn PromotionDomainServiceTrait>,
        metrics_collector: Arc<dyn MetricsCollector>,
        notification_service: Arc<dyn NotificationService>,
    ) -> Self {
        Self {
            memory_repository,
            promotion_domain_service,
            metrics_collector,
            notification_service,
        }
    }
}

#[async_trait]
impl PromoteRecordsUseCase for PromoteRecordsUseCaseImpl {
    #[instrument(skip(self, request), fields(criteria_count = request.criteria.len()))]
    async fn promote_records(
        &self,
        request: PromoteRecordsRequest,
        context: RequestContext,
    ) -> ApplicationResult<PromoteRecordsResponse> {
        let start_time = std::time::Instant::now();

        info!(
            "Starting promote records operation for request: {}",
            context.request_id
        );

        // Validate request
        self.validate_promotion_request(&request)?;

        // Convert criteria to domain objects
        let promotion_criteria = self.convert_promotion_criteria(&request.criteria)?;

        // Find promotion candidates using domain service
        let candidates_start = std::time::Instant::now();
        let candidates = self
            .promotion_domain_service
            .find_promotion_candidates(&promotion_criteria, request.max_candidates, request.dry_run)
            .await
            .map_err(|e| ApplicationError::Domain(e))?;
        let candidates_time = candidates_start.elapsed();

        let mut promoted_records = Vec::new();
        let mut failed_promotion_errors = Vec::new();
        let mut promotion_time = std::time::Duration::default();

        // Clone candidates for potential use in dry_run branch and save length
        let candidates_count = candidates.len();
        let candidates_for_dry_run = candidates.clone();

        if !request.dry_run && !candidates.is_empty() {
            // Convert domain objects to application DTOs for execution
            let promotion_candidates: Vec<crate::dtos::PromotionCandidate> = candidates
                .into_iter()
                .map(|recommendation| crate::dtos::PromotionCandidate {
                    record_id: recommendation.record_id.to_string(),
                    current_layer: recommendation.from_layer,
                    recommended_layer: recommendation.to_layer,
                    confidence_score: recommendation.confidence,
                    access_frequency: 0, // Mock value - would need domain data
                    last_accessed: chrono::Utc::now(), // Mock value
                    similarity_cluster: None,
                    business_value_score: recommendation.confidence, // Use confidence as business value
                    promotion_urgency: crate::dtos::PromotionUrgency::Medium,
                })
                .collect();

            // Execute promotions
            let execute_start = std::time::Instant::now();
            let promotion_results = self
                .execute_promotions(promotion_candidates, &context)
                .await?;
            promotion_time = execute_start.elapsed();

            for result in promotion_results {
                if result.success {
                    promoted_records.push(result.record_promotion);
                } else {
                    failed_promotion_errors.push(result.error_info);
                }
            }
        } else if request.dry_run {
            promoted_records = candidates_for_dry_run
                .into_iter()
                .map(|candidate| crate::dtos::RecordPromotion {
                    record_id: candidate.record_id.to_string(),
                    from_layer: candidate.from_layer,
                    to_layer: candidate.to_layer,
                    success: true,
                    promotion_score: candidate.confidence,
                    promotion_reason: candidate.reason.clone(),
                    estimated_benefit: candidate.confidence, // Use confidence as benefit estimate
                    processing_time_ms: 0,
                    error: None,
                })
                .collect();
        }

        let total_time = start_time.elapsed();

        // Record metrics
        self.record_promotion_metrics(
            promoted_records.len(),
            failed_promotion_errors.len(),
            total_time,
            candidates_time,
            promotion_time,
            request.dry_run,
        )
        .await?;

        if !request.dry_run && promoted_records.len() > 10 {
            self.send_promotion_notification(promoted_records.len(), &context)
                .await?;
        }

        let promoted_count = promoted_records.len();
        let failed_count = failed_promotion_errors.len();

        let response = PromoteRecordsResponse {
            analysis_id: context.request_id.to_string(),
            total_candidates: candidates_count,
            promoted_count,
            promoted_records,
            skipped_count: candidates_count - promoted_count - failed_count,
            failed_count,
            failed_promotions: failed_count,
            promotion_details: Vec::new(), // TODO: Implement promotion details
            analysis_time_ms: candidates_time.as_millis() as u64,
            dry_run: request.dry_run,
            total_processing_time_ms: total_time.as_millis() as u64,
            candidates_analysis_time_ms: candidates_time.as_millis() as u64,
            promotion_execution_time_ms: promotion_time.as_millis() as u64,
        };

        info!(
            "Promotion operation completed: {} promoted, {} failed, dry_run: {}",
            response.promoted_records.len(),
            response.failed_promotions,
            response.dry_run
        );

        Ok(response)
    }

    #[instrument(skip(self, request), fields(layers_count = request.source_layers.len()))]
    async fn analyze_promotion_candidates(
        &self,
        request: AnalyzePromotionRequest,
        context: RequestContext,
    ) -> ApplicationResult<AnalyzePromotionResponse> {
        let start_time = std::time::Instant::now();

        info!(
            "Starting analyze promotion candidates for request: {}",
            context.request_id
        );

        // Validate request
        self.validate_analysis_request(&request)?;

        // Create analysis criteria
        let analysis_criteria = self.create_analysis_criteria(&request)?;

        // Analyze promotion opportunities
        let analysis_results = self
            .promotion_domain_service
            .analyze_promotion_opportunities(
                &analysis_criteria,
                Some(&request.source_layers),
                request.time_window_hours,
            )
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        // Store the count before consuming the recommendations
        let recommendations_count = analysis_results.recommendations.len();

        // Convert to DTOs
        let candidates: Vec<crate::dtos::PromotionCandidate> = analysis_results
            .recommendations
            .into_iter()
            .filter(|r| r.recommended)
            .map(|candidate| crate::dtos::PromotionCandidate {
                record_id: candidate.record_id.to_string(),
                current_layer: candidate.from_layer,
                recommended_layer: candidate.to_layer,
                confidence_score: candidate.confidence,
                access_frequency: 0, // Mock value - needs implementation
                last_accessed: chrono::Utc::now(), // Mock value - needs implementation
                similarity_cluster: None,
                business_value_score: 0.5, // Mock value - needs implementation
                promotion_urgency: crate::dtos::PromotionUrgency::Medium,
            })
            .collect();

        // Mock layer statistics
        let _layer_stats: std::collections::HashMap<
            domain::LayerType,
            crate::dtos::LayerAnalysisStats,
        > = std::collections::HashMap::new();

        let total_time = start_time.elapsed();

        // Record analysis metrics
        self.record_analysis_metrics(candidates.len(), total_time)
            .await?;

        let response = AnalyzePromotionResponse {
            analysis_id: context.request_id.to_string(),
            candidates,
            layer_statistics: crate::dtos::LayerStatistics {
                cache_stats: crate::dtos::LayerStats {
                    total_records: 0,
                    total_size_mb: 0.0,
                    average_access_frequency: 0.0,
                    hottest_records: Vec::new(),
                    coldest_records: Vec::new(),
                    utilization_percentage: 0.0,
                },
                index_stats: crate::dtos::LayerStats {
                    total_records: 0,
                    total_size_mb: 0.0,
                    average_access_frequency: 0.0,
                    hottest_records: Vec::new(),
                    coldest_records: Vec::new(),
                    utilization_percentage: 0.0,
                },
                storage_stats: crate::dtos::LayerStats {
                    total_records: 0,
                    total_size_mb: 0.0,
                    average_access_frequency: 0.0,
                    hottest_records: Vec::new(),
                    coldest_records: Vec::new(),
                    utilization_percentage: 0.0,
                },
                cross_layer_patterns: crate::dtos::CrossLayerPatterns {
                    promotion_velocity: 0.0,
                    demotion_velocity: 0.0,
                    layer_transition_matrix: vec![vec![0.0; 3]; 3],
                    access_pattern_clusters: Vec::new(),
                },
            },
            recommendations: Vec::new(),
            ml_model_metrics: None,
            analysis_metadata: crate::dtos::AnalysisMetadata {
                analysis_time_ms: total_time.as_millis() as u64,
                records_analyzed: recommendations_count as u64,
                data_quality_score: 1.0,
                analysis_completeness: 1.0,
                warnings: Vec::new(),
                limitations: Vec::new(),
            },
        };

        info!(
            "Promotion analysis completed: {} candidates found, {} records analyzed",
            response.candidates.len(),
            response.analysis_metadata.records_analyzed
        );

        Ok(response)
    }

    async fn force_promote_records(
        &self,
        record_ids: Vec<String>,
        target_layer: LayerType,
        context: RequestContext,
    ) -> ApplicationResult<PromoteRecordsResponse> {
        let start_time = std::time::Instant::now();

        info!(
            "Starting force promote for {} records to {:?}",
            record_ids.len(),
            target_layer
        );

        if record_ids.is_empty() {
            return Err(ApplicationError::validation("No record IDs provided"));
        }

        if record_ids.len() > 50 {
            return Err(ApplicationError::validation(
                "Too many records for force promotion (max 50)",
            ));
        }

        // Convert string IDs to RecordId objects
        let domain_ids: Result<Vec<RecordId>, _> = record_ids
            .iter()
            .map(|id| RecordId::from_string(id))
            .collect();

        let domain_ids = domain_ids.map_err(|e| ApplicationError::Domain(e))?;

        // Execute force promotions through domain service
        let promotion_results = self
            .promotion_domain_service
            .force_promote_records(&domain_ids, target_layer)
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        let mut promoted_records = Vec::new();
        let mut failed_count = 0;

        for result in promotion_results {
            if result.success {
                promoted_records.push(crate::dtos::RecordPromotion {
                    record_id: result.record_id.to_string(),
                    from_layer: result.from_layer,
                    to_layer: result.target_layer.unwrap_or(result.from_layer),
                    success: true,
                    promotion_score: 1.0, // Force promotion always gets max score
                    promotion_reason: "Administrative force promotion".to_string(),
                    estimated_benefit: 0.0, // No prediction for forced promotion
                    processing_time_ms: 0,
                    error: None,
                });
            } else {
                failed_count += 1;
                warn!(
                    "Failed to force promote record {}: {}",
                    result.record_id, result.reason
                );
            }
        }

        let total_time = start_time.elapsed();

        // Record force promotion metrics
        self.record_force_promotion_metrics(promoted_records.len(), failed_count, total_time)
            .await?;

        self.send_force_promotion_notification(
            record_ids.len(),
            promoted_records.len(),
            target_layer,
            &context,
        )
        .await?;

        let response = PromoteRecordsResponse {
            analysis_id: context.request_id.to_string(),
            total_candidates: record_ids.len(),
            promoted_count: promoted_records.len(),
            promoted_records,
            skipped_count: 0,
            failed_count: failed_count,
            failed_promotions: failed_count,
            promotion_details: Vec::new(), // TODO: Implement promotion details
            analysis_time_ms: 0,
            dry_run: false,
            total_processing_time_ms: total_time.as_millis() as u64,
            candidates_analysis_time_ms: 0,
            promotion_execution_time_ms: total_time.as_millis() as u64,
        };

        info!(
            "Force promotion completed: {} promoted, {} failed out of {} requested",
            response.promoted_records.len(),
            response.failed_promotions,
            record_ids.len()
        );

        Ok(response)
    }

    async fn run_automated_promotion_cycle(
        &self,
        context: RequestContext,
    ) -> ApplicationResult<PromoteRecordsResponse> {
        let start_time = std::time::Instant::now();

        info!("Starting automated promotion cycle");

        // Run automated promotion through domain service
        let cycle_results = self
            .promotion_domain_service
            .run_promotion_cycle()
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        // Clone cycle_results for metrics before consuming promotion_details
        let cycle_results_for_metrics = cycle_results.clone();

        let promoted_records: Vec<crate::dtos::RecordPromotion> = cycle_results
            .promotion_details
            .into_iter()
            .filter(|p| p.success)
            .map(|promotion| crate::dtos::RecordPromotion {
                record_id: promotion.record_id().to_string(),
                from_layer: promotion.from_layer(),
                to_layer: promotion.to_layer(),
                success: true,
                promotion_score: promotion.score(),
                promotion_reason: promotion.reason().to_string(),
                estimated_benefit: promotion.estimated_benefit(),
                processing_time_ms: promotion.execution_time_ms,
                error: None,
            })
            .collect();

        let total_time = start_time.elapsed();

        // Record automated cycle metrics
        self.record_automated_cycle_metrics(&cycle_results_for_metrics, total_time)
            .await?;

        let response = PromoteRecordsResponse {
            analysis_id: context.request_id.to_string(),
            total_candidates: cycle_results_for_metrics.total_candidates as usize,
            promoted_count: promoted_records.len(),
            promoted_records,
            skipped_count: cycle_results_for_metrics.skipped_promotions as usize,
            failed_count: cycle_results_for_metrics.failed_promotions as usize,
            failed_promotions: cycle_results_for_metrics.failed_promotions as usize,
            promotion_details: Vec::new(), // Convert from domain PromotionDetail to DTO PromotionDetail
            analysis_time_ms: cycle_results_for_metrics.analysis_time_ms,
            dry_run: false,
            total_processing_time_ms: total_time.as_millis() as u64,
            candidates_analysis_time_ms: cycle_results_for_metrics.analysis_time_ms,
            promotion_execution_time_ms: cycle_results_for_metrics.execution_time_ms,
        };

        info!(
            "Automated promotion cycle completed: {} promoted, {} failed",
            response.promoted_records.len(),
            response.failed_promotions
        );

        Ok(response)
    }
}

impl PromoteRecordsUseCaseImpl {
    /// Validate promotion request
    fn validate_promotion_request(&self, request: &PromoteRecordsRequest) -> ApplicationResult<()> {
        if request.criteria.is_empty() {
            return Err(ApplicationError::validation(
                "No promotion criteria provided",
            ));
        }

        if request.criteria.len() > 10 {
            return Err(ApplicationError::validation(
                "Too many promotion criteria (max 10)",
            ));
        }

        if let Some(max_candidates) = request.max_candidates {
            if max_candidates == 0 || max_candidates > 1000 {
                return Err(ApplicationError::validation(
                    "Max candidates must be between 1 and 1000",
                ));
            }
        }

        Ok(())
    }

    /// Validate analysis request
    fn validate_analysis_request(
        &self,
        request: &AnalyzePromotionRequest,
    ) -> ApplicationResult<()> {
        if request.time_window_hours == 0 || request.time_window_hours > 8760 {
            // Max 1 year
            return Err(ApplicationError::validation(
                "Time window must be between 1 hour and 1 year",
            ));
        }

        Ok(())
    }

    /// Convert request criteria to domain objects
    fn convert_promotion_criteria(
        &self,
        criteria: &[crate::dtos::PromotionCriterion],
    ) -> ApplicationResult<Vec<PromotionCriteria>> {
        criteria
            .iter()
            .map(|c| {
                let min_access_count = c.min_access_frequency.unwrap_or(5) as u32;
                let max_access_interval =
                    chrono::Duration::hours(c.time_window_hours.unwrap_or(24) as i64);
                let min_age = chrono::Duration::hours(1); // Default 1 hour
                let min_importance_score = c.min_score_threshold.unwrap_or(0.5);
                let require_acceleration = c.boost_recent_activity;

                PromotionCriteria::new(
                    min_access_count,
                    max_access_interval,
                    min_age,
                    min_importance_score,
                    require_acceleration,
                )
                .map_err(|e| ApplicationError::Domain(e))
            })
            .collect()
    }

    /// Create analysis criteria
    fn create_analysis_criteria(
        &self,
        request: &AnalyzePromotionRequest,
    ) -> ApplicationResult<PromotionCriteria> {
        PromotionCriteria::new(
            1, // Default min access frequency
            chrono::Duration::hours(request.time_window_hours as i64),
            chrono::Duration::hours(1), // min age
            0.5,                        // Default min importance score
            request.include_ml_predictions,
        )
        .map_err(|e| ApplicationError::Domain(e))
    }

    /// Execute promotions
    async fn execute_promotions(
        &self,
        candidates: Vec<PromotionCandidate>,
        context: &RequestContext,
    ) -> ApplicationResult<Vec<PromotionExecutionResult>> {
        let mut results = Vec::new();

        for candidate in candidates {
            let result = match self.execute_single_promotion(&candidate).await {
                Ok(promotion) => PromotionExecutionResult {
                    success: true,
                    record_promotion: promotion,
                    error_info: None,
                },
                Err(e) => PromotionExecutionResult {
                    success: false,
                    record_promotion: crate::dtos::RecordPromotion {
                        record_id: candidate.record_id,
                        from_layer: candidate.current_layer,
                        to_layer: candidate.recommended_layer,
                        success: false,
                        promotion_score: 0.0,
                        promotion_reason: "Promotion failed".to_string(),
                        estimated_benefit: 0.0,
                        processing_time_ms: 0,
                        error: Some("Promotion failed".to_string()),
                    },
                    error_info: Some(e.to_string()),
                },
            };

            results.push(result);
        }

        Ok(results)
    }

    /// Execute single promotion
    async fn execute_single_promotion(
        &self,
        candidate: &PromotionCandidate,
    ) -> ApplicationResult<crate::dtos::RecordPromotion> {
        // Promote through domain service
        let promotion_result = self
            .promotion_domain_service
            .promote_record(
                RecordId::from_string(candidate.record_id())
                    .map_err(|e| ApplicationError::Domain(e))?,
                candidate.target_layer(),
            )
            .await
            .map_err(|e| ApplicationError::Domain(e))?;

        Ok(crate::dtos::RecordPromotion {
            record_id: promotion_result.record_id().to_string(),
            from_layer: promotion_result.from_layer(),
            to_layer: promotion_result
                .to_layer()
                .unwrap_or(candidate.target_layer()),
            success: true,
            promotion_score: candidate.promotion_score(),
            promotion_reason: candidate.promotion_reason(),
            estimated_benefit: candidate.estimated_benefit(),
            processing_time_ms: 0, // TODO: Add timing measurement
            error: None,
        })
    }

    /// Record promotion metrics
    async fn record_promotion_metrics(
        &self,
        promoted_count: usize,
        failed_count: usize,
        total_time: std::time::Duration,
        candidates_time: std::time::Duration,
        promotion_time: std::time::Duration,
        dry_run: bool,
    ) -> ApplicationResult<()> {
        use crate::ports::MemoryOperation;

        let operation_type = if dry_run {
            crate::ports::MemoryOperationType::Analyze
        } else {
            crate::ports::MemoryOperationType::Promote
        };

        let operation = MemoryOperation {
            operation_type,
            layer: "multiple".to_string(),
            record_count: promoted_count + failed_count,
            processing_time_ms: total_time.as_millis() as u64,
            bytes_processed: 0, // Not applicable for promotions
            success: failed_count == 0,
            error: if failed_count > 0 {
                Some(format!("{} promotions failed", failed_count))
            } else {
                None
            },
        };

        self.metrics_collector
            .record_memory_operation(operation)
            .await?;

        // Record detailed metrics
        self.metrics_collector
            .record_gauge(
                "promotion_success_rate",
                if promoted_count + failed_count > 0 {
                    promoted_count as f64 / (promoted_count + failed_count) as f64
                } else {
                    1.0
                },
                None,
            )
            .await?;

        Ok(())
    }

    /// Record analysis metrics
    async fn record_analysis_metrics(
        &self,
        candidates_count: usize,
        total_time: std::time::Duration,
    ) -> ApplicationResult<()> {
        self.metrics_collector
            .increment_counter("promotion_analysis_operations_total", 1, None)
            .await?;

        self.metrics_collector
            .record_gauge("promotion_candidates_found", candidates_count as f64, None)
            .await?;

        Ok(())
    }

    /// Record force promotion metrics
    async fn record_force_promotion_metrics(
        &self,
        promoted_count: usize,
        failed_count: usize,
        total_time: std::time::Duration,
    ) -> ApplicationResult<()> {
        self.metrics_collector
            .increment_counter("force_promotion_operations_total", 1, None)
            .await?;

        self.metrics_collector
            .record_gauge(
                "force_promotion_success_rate",
                if promoted_count + failed_count > 0 {
                    promoted_count as f64 / (promoted_count + failed_count) as f64
                } else {
                    1.0
                },
                None,
            )
            .await?;

        Ok(())
    }

    /// Record automated cycle metrics
    async fn record_automated_cycle_metrics(
        &self,
        results: &domain::services::promotion_domain_service::PromotionCycleResults,
        total_time: std::time::Duration,
    ) -> ApplicationResult<()> {
        self.metrics_collector
            .increment_counter("automated_promotion_cycles_total", 1, None)
            .await?;

        self.metrics_collector
            .record_gauge(
                "automated_promotion_efficiency",
                results.efficiency_score,
                None,
            )
            .await?;

        Ok(())
    }

    /// Send promotion notification
    async fn send_promotion_notification(
        &self,
        promoted_count: usize,
        context: &RequestContext,
    ) -> ApplicationResult<()> {
        use crate::ports::{Notification, NotificationLevel};

        let notification = Notification::info(
            "Bulk Memory Promotion Completed",
            &format!("{} records promoted between memory layers", promoted_count),
        );

        self.notification_service
            .send_notification(&notification)
            .await?;
        Ok(())
    }

    /// Send force promotion notification
    async fn send_force_promotion_notification(
        &self,
        requested_count: usize,
        promoted_count: usize,
        target_layer: LayerType,
        context: &RequestContext,
    ) -> ApplicationResult<()> {
        use crate::ports::{Notification, NotificationLevel};

        let level = if promoted_count == requested_count {
            NotificationLevel::Info
        } else {
            NotificationLevel::Warning
        };

        let notification = Notification {
            level,
            title: "Force Promotion Completed".to_string(),
            message: format!(
                "Force promoted {}/{} records to {:?} layer",
                promoted_count, requested_count, target_layer
            ),
            ..Notification::info("", "")
        };

        self.notification_service
            .send_notification(&notification)
            .await?;
        Ok(())
    }
}

/// Internal struct for promotion execution results
struct PromotionExecutionResult {
    success: bool,
    record_promotion: crate::dtos::RecordPromotion,
    error_info: Option<String>,
}
