//! Memory Application Service
//!
//! Координирует memory-related операции, обеспечивает транзакционность
//! и управляет взаимодействием между use cases.

use crate::dtos::{
    AnalyzePromotionRequest, AnalyzePromotionResponse, BatchStoreMemoryRequest,
    BatchStoreMemoryResponse, PromoteRecordsRequest, PromoteRecordsResponse, StoreMemoryRequest,
    StoreMemoryResponse,
};
use crate::ports::{MetricsCollector, NotificationService};
use crate::use_cases::{PromoteRecordsUseCase, StoreMemoryUseCase};
use crate::{ApplicationError, ApplicationResult, RequestContext};
use async_trait::async_trait;
use domain::LayerType;
use std::sync::Arc;
use tracing::{error, info, instrument};

/// Application Service для координации memory operations
#[async_trait]
pub trait MemoryApplicationService: Send + Sync {
    /// Store single memory record with full workflow
    async fn store_memory_record(
        &self,
        request: StoreMemoryRequest,
        context: RequestContext,
    ) -> ApplicationResult<StoreMemoryResponse>;

    /// Store batch of memory records with transaction handling
    async fn store_memory_batch(
        &self,
        request: BatchStoreMemoryRequest,
        context: RequestContext,
    ) -> ApplicationResult<BatchStoreMemoryResponse>;

    /// Promote records with analysis and validation
    async fn promote_memory_records(
        &self,
        request: PromoteRecordsRequest,
        context: RequestContext,
    ) -> ApplicationResult<PromoteRecordsResponse>;

    /// Analyze promotion opportunities
    async fn analyze_promotion_opportunities(
        &self,
        request: AnalyzePromotionRequest,
        context: RequestContext,
    ) -> ApplicationResult<AnalyzePromotionResponse>;

    /// Complete memory workflow: store → analyze → promote (if beneficial)
    async fn complete_memory_workflow(
        &self,
        request: StoreMemoryRequest,
        context: RequestContext,
    ) -> ApplicationResult<CompleteWorkflowResponse>;

    /// Health check for all memory subsystems
    async fn health_check(&self, context: RequestContext) -> ApplicationResult<MemorySystemHealth>;
}

/// Implementation of memory application service
pub struct MemoryApplicationServiceImpl {
    store_use_case: Arc<dyn StoreMemoryUseCase>,
    promotion_use_case: Arc<dyn PromoteRecordsUseCase>,
    metrics_collector: Arc<dyn MetricsCollector>,
    notification_service: Arc<dyn NotificationService>,
    config: MemoryServiceConfig,
}

/// Configuration for memory application service
#[derive(Debug, Clone)]
pub struct MemoryServiceConfig {
    pub auto_promotion_enabled: bool,
    pub promotion_threshold: f64,
    pub batch_size_limit: usize,
    pub transaction_timeout_seconds: u64,
    pub enable_workflow_optimization: bool,
}

/// Complete workflow response
#[derive(Debug)]
pub struct CompleteWorkflowResponse {
    pub store_response: StoreMemoryResponse,
    pub promotion_analysis: Option<AnalyzePromotionResponse>,
    pub promotion_response: Option<PromoteRecordsResponse>,
    pub workflow_recommendations: Vec<String>,
    pub total_processing_time_ms: u64,
}

/// Memory system health
#[derive(Debug)]
pub struct MemorySystemHealth {
    pub overall_status: HealthStatus,
    pub store_subsystem: SubsystemHealth,
    pub promotion_subsystem: SubsystemHealth,
    pub cache_subsystem: SubsystemHealth,
    pub last_check_time: std::time::SystemTime,
    pub health_score: f64,
}

#[derive(Debug)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Critical,
}

#[derive(Debug)]
pub struct SubsystemHealth {
    pub status: HealthStatus,
    pub response_time_ms: u64,
    pub error_rate: f32,
    pub last_error: Option<String>,
    pub details: std::collections::HashMap<String, String>,
}

impl MemoryApplicationServiceImpl {
    pub fn new(
        store_use_case: Arc<dyn StoreMemoryUseCase>,
        promotion_use_case: Arc<dyn PromoteRecordsUseCase>,
        metrics_collector: Arc<dyn MetricsCollector>,
        notification_service: Arc<dyn NotificationService>,
        config: MemoryServiceConfig,
    ) -> Self {
        Self {
            store_use_case,
            promotion_use_case,
            metrics_collector,
            notification_service,
            config,
        }
    }

    pub fn with_default_config(
        store_use_case: Arc<dyn StoreMemoryUseCase>,
        promotion_use_case: Arc<dyn PromoteRecordsUseCase>,
        metrics_collector: Arc<dyn MetricsCollector>,
        notification_service: Arc<dyn NotificationService>,
    ) -> Self {
        Self::new(
            store_use_case,
            promotion_use_case,
            metrics_collector,
            notification_service,
            MemoryServiceConfig::default(),
        )
    }
}

#[async_trait]
impl MemoryApplicationService for MemoryApplicationServiceImpl {
    #[instrument(skip(self, request), fields(content_length = request.content.len()))]
    async fn store_memory_record(
        &self,
        request: StoreMemoryRequest,
        context: RequestContext,
    ) -> ApplicationResult<StoreMemoryResponse> {
        let start_time = std::time::Instant::now();

        info!("Processing store memory request: {}", context.request_id);

        // Validate request at application level
        self.validate_store_request(&request)?;

        let store_result = self
            .store_use_case
            .store_memory(request.clone(), context.clone())
            .await;

        match store_result {
            Ok(response) => {
                let total_time = start_time.elapsed();

                // Record success metrics
                self.record_operation_success("store_memory", total_time.as_millis() as u64)
                    .await?;

                info!(
                    "Successfully stored memory record: {} in {}ms",
                    response.record_id,
                    total_time.as_millis()
                );

                Ok(response)
            }
            Err(e) => {
                let total_time = start_time.elapsed();

                // Record failure metrics
                self.record_operation_failure("store_memory", &e, total_time.as_millis() as u64)
                    .await?;

                if self.is_critical_error(&e) {
                    self.send_error_notification("store_memory", &e, &context)
                        .await?;
                }

                error!("Failed to store memory record: {}", e);
                Err(e)
            }
        }
    }

    #[instrument(skip(self, request), fields(record_count = request.records.len()))]
    async fn store_memory_batch(
        &self,
        request: BatchStoreMemoryRequest,
        context: RequestContext,
    ) -> ApplicationResult<BatchStoreMemoryResponse> {
        let start_time = std::time::Instant::now();

        info!(
            "Processing batch store request: {} records",
            request.records.len()
        );

        // Validate batch request
        self.validate_batch_request(&request)?;

        let _transaction_guard = if self.config.transaction_timeout_seconds > 0 {
            Some(self.begin_transaction(&context).await?)
        } else {
            None
        };

        let batch_result = self
            .store_use_case
            .store_batch_memory(request, context.clone())
            .await;

        match batch_result {
            Ok(response) => {
                let total_time = start_time.elapsed();

                // Record batch metrics
                self.record_batch_metrics(&response, total_time.as_millis() as u64)
                    .await?;

                if response.successful > 50 {
                    self.send_batch_success_notification(&response, &context)
                        .await?;
                }

                info!(
                    "Batch store completed: {}/{} successful in {}ms",
                    response.successful,
                    response.total_requested,
                    total_time.as_millis()
                );

                Ok(response)
            }
            Err(e) => {
                let total_time = start_time.elapsed();

                // Record failure
                self.record_operation_failure("store_batch", &e, total_time.as_millis() as u64)
                    .await?;

                error!("Batch store failed: {}", e);
                Err(e)
            }
        }
    }

    #[instrument(skip(self, request), fields(criteria_count = request.criteria.len()))]
    async fn promote_memory_records(
        &self,
        request: PromoteRecordsRequest,
        context: RequestContext,
    ) -> ApplicationResult<PromoteRecordsResponse> {
        let start_time = std::time::Instant::now();

        info!(
            "Processing promotion request with {} criteria",
            request.criteria.len()
        );

        // Validate promotion request
        self.validate_promotion_request(&request)?;

        let promotion_result = self
            .promotion_use_case
            .promote_records(request.clone(), context.clone())
            .await;

        match promotion_result {
            Ok(response) => {
                let total_time = start_time.elapsed();

                // Record promotion metrics
                self.record_promotion_metrics(&response, total_time.as_millis() as u64)
                    .await?;

                if response.promoted_records.len() > 20 {
                    self.send_promotion_success_notification(&response, &context)
                        .await?;
                }

                info!(
                    "Promotion completed: {} records promoted, dry_run: {}",
                    response.promoted_records.len(),
                    response.dry_run
                );

                Ok(response)
            }
            Err(e) => {
                let total_time = start_time.elapsed();

                // Record failure
                self.record_operation_failure("promote_records", &e, total_time.as_millis() as u64)
                    .await?;

                error!("Promotion failed: {}", e);
                Err(e)
            }
        }
    }

    async fn analyze_promotion_opportunities(
        &self,
        request: AnalyzePromotionRequest,
        context: RequestContext,
    ) -> ApplicationResult<AnalyzePromotionResponse> {
        let start_time = std::time::Instant::now();

        info!("Analyzing promotion opportunities");

        let analysis_result = self
            .promotion_use_case
            .analyze_promotion_candidates(request, context.clone())
            .await;

        match analysis_result {
            Ok(response) => {
                let total_time = start_time.elapsed();

                // Record analysis metrics
                self.record_analysis_metrics(&response, total_time.as_millis() as u64)
                    .await?;

                info!(
                    "Promotion analysis completed: {} candidates found in {}ms",
                    response.candidates.len(),
                    total_time.as_millis()
                );

                Ok(response)
            }
            Err(e) => {
                error!("Promotion analysis failed: {}", e);
                Err(e)
            }
        }
    }

    #[instrument(skip(self, request), fields(content_length = request.content.len()))]
    async fn complete_memory_workflow(
        &self,
        request: StoreMemoryRequest,
        context: RequestContext,
    ) -> ApplicationResult<CompleteWorkflowResponse> {
        let start_time = std::time::Instant::now();

        info!("Starting complete memory workflow");

        // Step 1: Store the record
        let store_response = self
            .store_memory_record(request.clone(), context.clone())
            .await?;

        let mut promotion_analysis = None;
        let mut promotion_response = None;
        let mut workflow_recommendations = Vec::new();

        if self.config.auto_promotion_enabled {
            if self.should_analyze_for_promotion(&request, &store_response) {
                let analysis_request = self.create_analysis_request(&request, &store_response)?;

                match self
                    .analyze_promotion_opportunities(analysis_request, context.clone())
                    .await
                {
                    Ok(analysis) => {
                        promotion_analysis = Some(analysis);

                        if let Some(ref analysis) = promotion_analysis {
                            if self.should_auto_promote(&analysis) {
                                let promotion_request = self.create_promotion_request(&analysis)?;

                                match self
                                    .promote_memory_records(promotion_request, context.clone())
                                    .await
                                {
                                    Ok(response) => {
                                        promotion_response = Some(response);
                                        workflow_recommendations.push(
                                            "Record automatically promoted based on analysis"
                                                .to_string(),
                                        );
                                    }
                                    Err(e) => {
                                        workflow_recommendations
                                            .push(format!("Auto-promotion failed: {}", e));
                                    }
                                }
                            } else {
                                workflow_recommendations.push(
                                    "Record not suitable for immediate promotion".to_string(),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        workflow_recommendations.push(format!("Promotion analysis skipped: {}", e));
                    }
                }
            } else {
                workflow_recommendations
                    .push("Record does not qualify for promotion analysis".to_string());
            }
        } else {
            workflow_recommendations.push("Auto-promotion is disabled".to_string());
        }

        let total_time = start_time.elapsed();

        // Record complete workflow metrics
        self.record_workflow_metrics(
            promotion_analysis.is_some(),
            promotion_response.is_some(),
            total_time.as_millis() as u64,
        )
        .await?;

        info!(
            "Complete memory workflow finished in {}ms with {} recommendations",
            total_time.as_millis(),
            workflow_recommendations.len()
        );

        Ok(CompleteWorkflowResponse {
            store_response,
            promotion_analysis,
            promotion_response,
            workflow_recommendations,
            total_processing_time_ms: total_time.as_millis() as u64,
        })
    }

    async fn health_check(&self, context: RequestContext) -> ApplicationResult<MemorySystemHealth> {
        let start_time = std::time::Instant::now();

        info!("Performing memory system health check");

        // Check store subsystem
        let store_health = self.check_store_subsystem().await?;

        // Check promotion subsystem
        let promotion_health = self.check_promotion_subsystem().await?;

        // Check cache subsystem
        let cache_health = self.check_cache_subsystem().await?;

        // Calculate overall health
        let overall_status =
            self.determine_overall_health(&store_health, &promotion_health, &cache_health);
        let health_score =
            self.calculate_health_score(&store_health, &promotion_health, &cache_health);

        let total_time = start_time.elapsed();

        let system_health = MemorySystemHealth {
            overall_status,
            store_subsystem: store_health,
            promotion_subsystem: promotion_health,
            cache_subsystem: cache_health,
            last_check_time: std::time::SystemTime::now(),
            health_score,
        };

        // Record health check metrics
        self.record_health_check_metrics(&system_health, total_time.as_millis() as u64)
            .await?;

        info!(
            "Health check completed in {}ms: overall status = {:?}, score = {:.2}",
            total_time.as_millis(),
            system_health.overall_status,
            health_score
        );

        Ok(system_health)
    }
}

impl MemoryApplicationServiceImpl {
    /// Validate store request at application level
    fn validate_store_request(&self, request: &StoreMemoryRequest) -> ApplicationResult<()> {
        // Additional application-level validation
        if request.content.len() > 100_000 {
            return Err(ApplicationError::validation(
                "Content exceeds maximum size for single record",
            ));
        }

        if request.tags.len() > 100 {
            return Err(ApplicationError::validation(
                "Too many tags for single record",
            ));
        }

        Ok(())
    }

    /// Validate batch request
    fn validate_batch_request(&self, request: &BatchStoreMemoryRequest) -> ApplicationResult<()> {
        if request.records.len() > self.config.batch_size_limit {
            return Err(ApplicationError::validation(format!(
                "Batch size {} exceeds limit {}",
                request.records.len(),
                self.config.batch_size_limit
            )));
        }

        Ok(())
    }

    /// Validate promotion request
    fn validate_promotion_request(&self, request: &PromoteRecordsRequest) -> ApplicationResult<()> {
        if request.criteria.is_empty() {
            return Err(ApplicationError::validation(
                "No promotion criteria specified",
            ));
        }

        Ok(())
    }

    /// Begin transaction for batch operations
    async fn begin_transaction(
        &self,
        context: &RequestContext,
    ) -> ApplicationResult<TransactionGuard> {
        // Implementation would create a transaction context
        Ok(TransactionGuard::new(context.request_id))
    }

    /// Check if record should be analyzed for promotion
    fn should_analyze_for_promotion(
        &self,
        request: &StoreMemoryRequest,
        response: &StoreMemoryResponse,
    ) -> bool {
        request.priority.unwrap_or(1) >= 3
            || response.estimated_retrieval_time_ms > 50
            || !request.project.is_empty()
    }

    /// Create analysis request from store context
    fn create_analysis_request(
        &self,
        request: &StoreMemoryRequest,
        response: &StoreMemoryResponse,
    ) -> ApplicationResult<AnalyzePromotionRequest> {
        Ok(AnalyzePromotionRequest {
            source_layers: vec![response.layer],
            target_layer: domain::LayerType::Insights,
            analysis_depth: crate::dtos::AnalysisDepth::Standard,
            time_window_hours: 24,
            include_ml_predictions: true,
        })
    }

    /// Check if promotion should be executed automatically
    fn should_auto_promote(&self, analysis: &AnalyzePromotionResponse) -> bool {
        !analysis.candidates.is_empty()
            && analysis
                .candidates
                .iter()
                .any(|c| c.confidence_score >= self.config.promotion_threshold as f32)
    }

    /// Create promotion request from analysis
    fn create_promotion_request(
        &self,
        analysis: &AnalyzePromotionResponse,
    ) -> ApplicationResult<PromoteRecordsRequest> {
        let high_confidence_candidates: Vec<_> = analysis
            .candidates
            .iter()
            .filter(|c| c.confidence_score >= self.config.promotion_threshold as f32)
            .take(10) // Limit auto-promotions
            .collect();

        if high_confidence_candidates.is_empty() {
            return Err(ApplicationError::business_logic(
                "No suitable candidates for auto-promotion",
            ));
        }

        // Create criteria based on analysis
        let criteria = vec![crate::dtos::PromotionCriterion {
            min_access_frequency: Some(1),
            min_similarity_score: Some(0.7),
            min_score_threshold: Some(self.config.promotion_threshold as f32),
            time_window_hours: Some(24),
            max_hours_since_access: Some(168),
            from_layer: Some(domain::LayerType::Interact),
            to_layer: domain::LayerType::Insights,
            target_layers: vec![domain::LayerType::Interact],
            project_filter: None,
            boost_recent_activity: true,
        }];

        Ok(PromoteRecordsRequest {
            criteria,
            max_candidates: Some(high_confidence_candidates.len()),
            dry_run: false,
            force: false,
            from_layer: Some(domain::LayerType::Interact),
            to_layer: domain::LayerType::Insights,
            record_ids: None,
        })
    }

    /// Check store subsystem health
    async fn check_store_subsystem(&self) -> ApplicationResult<SubsystemHealth> {
        // Implementation would check store subsystem
        Ok(SubsystemHealth {
            status: HealthStatus::Healthy,
            response_time_ms: 15,
            error_rate: 0.01,
            last_error: None,
            details: [("component".to_string(), "store".to_string())].into(),
        })
    }

    /// Check promotion subsystem health
    async fn check_promotion_subsystem(&self) -> ApplicationResult<SubsystemHealth> {
        // Implementation would check promotion subsystem
        Ok(SubsystemHealth {
            status: HealthStatus::Healthy,
            response_time_ms: 25,
            error_rate: 0.02,
            last_error: None,
            details: [("component".to_string(), "promotion".to_string())].into(),
        })
    }

    /// Check cache subsystem health
    async fn check_cache_subsystem(&self) -> ApplicationResult<SubsystemHealth> {
        // Implementation would check cache subsystem
        Ok(SubsystemHealth {
            status: HealthStatus::Healthy,
            response_time_ms: 5,
            error_rate: 0.001,
            last_error: None,
            details: [("component".to_string(), "cache".to_string())].into(),
        })
    }

    /// Determine overall system health
    fn determine_overall_health(
        &self,
        store: &SubsystemHealth,
        promotion: &SubsystemHealth,
        cache: &SubsystemHealth,
    ) -> HealthStatus {
        let subsystems = [store, promotion, cache];

        if subsystems
            .iter()
            .any(|s| matches!(s.status, HealthStatus::Critical))
        {
            HealthStatus::Critical
        } else if subsystems
            .iter()
            .any(|s| matches!(s.status, HealthStatus::Unhealthy))
        {
            HealthStatus::Unhealthy
        } else if subsystems
            .iter()
            .any(|s| matches!(s.status, HealthStatus::Degraded))
        {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }

    /// Calculate overall health score
    fn calculate_health_score(
        &self,
        store: &SubsystemHealth,
        promotion: &SubsystemHealth,
        cache: &SubsystemHealth,
    ) -> f64 {
        let subsystems = [store, promotion, cache];
        let total_score: f64 = subsystems
            .iter()
            .map(|s| match s.status {
                HealthStatus::Healthy => 1.0,
                HealthStatus::Degraded => 0.7,
                HealthStatus::Unhealthy => 0.4,
                HealthStatus::Critical => 0.1,
            })
            .sum();

        total_score / subsystems.len() as f64
    }

    /// Check if error is critical
    fn is_critical_error(&self, error: &ApplicationError) -> bool {
        matches!(error, ApplicationError::Infrastructure { .. })
    }

    // Metrics and notification methods
    async fn record_operation_success(
        &self,
        operation: &str,
        duration_ms: u64,
    ) -> ApplicationResult<()> {
        self.metrics_collector
            .increment_counter(&format!("{}_operations_successful", operation), 1, None)
            .await?;

        self.metrics_collector
            .record_timing(&format!("{}_duration", operation), duration_ms, None)
            .await?;

        Ok(())
    }

    async fn record_operation_failure(
        &self,
        operation: &str,
        error: &ApplicationError,
        duration_ms: u64,
    ) -> ApplicationResult<()> {
        self.metrics_collector
            .increment_counter(&format!("{}_operations_failed", operation), 1, None)
            .await?;

        self.metrics_collector
            .record_timing(&format!("{}_duration", operation), duration_ms, None)
            .await?;

        Ok(())
    }

    async fn record_batch_metrics(
        &self,
        response: &BatchStoreMemoryResponse,
        duration_ms: u64,
    ) -> ApplicationResult<()> {
        self.metrics_collector
            .record_gauge(
                "batch_store_success_rate",
                response.successful as f64 / response.total_requested as f64,
                None,
            )
            .await?;

        self.metrics_collector
            .record_gauge("batch_store_size", response.total_requested as f64, None)
            .await?;

        Ok(())
    }

    async fn record_promotion_metrics(
        &self,
        response: &PromoteRecordsResponse,
        duration_ms: u64,
    ) -> ApplicationResult<()> {
        self.metrics_collector
            .record_gauge(
                "promotion_records_count",
                response.promoted_count as f64,
                None,
            )
            .await?;

        Ok(())
    }

    async fn record_analysis_metrics(
        &self,
        response: &AnalyzePromotionResponse,
        duration_ms: u64,
    ) -> ApplicationResult<()> {
        self.metrics_collector
            .record_gauge(
                "promotion_analysis_candidates",
                response.candidates.len() as f64,
                None,
            )
            .await?;

        Ok(())
    }

    async fn record_workflow_metrics(
        &self,
        analysis_performed: bool,
        promotion_performed: bool,
        duration_ms: u64,
    ) -> ApplicationResult<()> {
        self.metrics_collector
            .increment_counter("complete_workflow_operations", 1, None)
            .await?;

        self.metrics_collector
            .record_gauge(
                "workflow_analysis_performed",
                if analysis_performed { 1.0 } else { 0.0 },
                None,
            )
            .await?;

        Ok(())
    }

    async fn record_health_check_metrics(
        &self,
        health: &MemorySystemHealth,
        duration_ms: u64,
    ) -> ApplicationResult<()> {
        self.metrics_collector
            .record_gauge("memory_system_health_score", health.health_score, None)
            .await?;

        Ok(())
    }

    async fn send_error_notification(
        &self,
        operation: &str,
        error: &ApplicationError,
        context: &RequestContext,
    ) -> ApplicationResult<()> {
        use crate::ports::{Notification, NotificationLevel};

        let notification = Notification {
            level: NotificationLevel::Error,
            title: format!("Memory Operation Failed: {}", operation),
            message: error.to_string(),
            ..Notification::info("", "")
        };

        self.notification_service
            .send_notification(&notification)
            .await?;
        Ok(())
    }

    async fn send_batch_success_notification(
        &self,
        response: &BatchStoreMemoryResponse,
        context: &RequestContext,
    ) -> ApplicationResult<()> {
        use crate::ports::{Notification, NotificationLevel};

        let notification = Notification::info(
            "Large Batch Store Completed",
            &format!(
                "Successfully processed {} out of {} records",
                response.successful, response.total_requested
            ),
        );

        self.notification_service
            .send_notification(&notification)
            .await?;
        Ok(())
    }

    async fn send_promotion_success_notification(
        &self,
        response: &PromoteRecordsResponse,
        context: &RequestContext,
    ) -> ApplicationResult<()> {
        use crate::ports::{Notification, NotificationLevel};

        let notification = Notification::info(
            "Bulk Promotion Completed",
            &format!(
                "Promoted {} records between memory layers",
                response.promoted_count
            ),
        );

        self.notification_service
            .send_notification(&notification)
            .await?;
        Ok(())
    }
}

/// Transaction guard for managing transactions
pub struct TransactionGuard {
    request_id: uuid::Uuid,
}

impl TransactionGuard {
    pub fn new(request_id: uuid::Uuid) -> Self {
        Self { request_id }
    }
}

impl Drop for TransactionGuard {
    fn drop(&mut self) {
        // Cleanup transaction on drop
    }
}

impl Default for MemoryServiceConfig {
    fn default() -> Self {
        Self {
            auto_promotion_enabled: true,
            promotion_threshold: 0.8,
            batch_size_limit: 1000,
            transaction_timeout_seconds: 300, // 5 minutes
            enable_workflow_optimization: true,
        }
    }
}
