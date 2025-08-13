#![allow(clippy::uninlined_format_args)]
//! Integration tests for Application Services

use application::dtos::*;
use application::ports::*;
use application::services::*;
use application::use_cases::*;
use application::{RequestContext, RequestSource};
use std::collections::HashMap;
use std::sync::Arc;

/// Mock metrics collector for testing
#[derive(Clone)]
struct MockMetricsCollector;

#[async_trait::async_trait]
impl MetricsCollector for MockMetricsCollector {
    async fn increment_counter(
        &self,
        _name: &str,
        _value: u64,
        _tags: Option<&HashMap<String, String>>,
    ) -> application::ApplicationResult<()> {
        Ok(())
    }

    async fn record_gauge(
        &self,
        _name: &str,
        _value: f64,
        _tags: Option<&HashMap<String, String>>,
    ) -> application::ApplicationResult<()> {
        Ok(())
    }

    async fn record_timing(
        &self,
        _name: &str,
        _value_ms: u64,
        _tags: Option<&HashMap<String, String>>,
    ) -> application::ApplicationResult<()> {
        Ok(())
    }

    async fn record_histogram(
        &self,
        _name: &str,
        _value: f64,
        _tags: Option<&HashMap<String, String>>,
    ) -> application::ApplicationResult<()> {
        Ok(())
    }

    async fn record_event(&self, _event: &BusinessEvent) -> application::ApplicationResult<()> {
        Ok(())
    }

    async fn flush(&self) -> application::ApplicationResult<FlushResult> {
        Ok(FlushResult {
            metrics_sent: 10,
            events_sent: 5,
            bytes_sent: 1024,
            duration_ms: 25,
            errors: vec![],
        })
    }

    async fn health_check(&self) -> application::ApplicationResult<MetricsHealth> {
        Ok(MetricsHealth {
            is_healthy: true,
            buffer_size: 50,
            buffer_capacity: 100,
            buffer_utilization: 0.5,
            last_flush_time: Some(chrono::Utc::now()),
            last_error: None,
            connection_status: ConnectionStatus::Connected,
        })
    }

    async fn get_statistics(&self) -> application::ApplicationResult<CollectorStatistics> {
        Ok(CollectorStatistics {
            total_metrics: 100,
            total_events: 50,
            metrics_per_second: 10.0,
            events_per_second: 5.0,
            success_rate: 0.99,
            average_flush_time_ms: 25,
            error_count: 1,
            uptime_seconds: 3600,
            memory_usage_bytes: 1024 * 1024,
        })
    }
}

/// Mock notification service for testing
#[derive(Clone)]
struct MockNotificationService;

#[async_trait::async_trait]
impl NotificationService for MockNotificationService {
    async fn send_notification(
        &self,
        _notification: &Notification,
    ) -> application::ApplicationResult<NotificationResult> {
        Ok(NotificationResult {
            notification_id: "test-notification".to_string(),
            status: DeliveryStatus::Delivered,
            delivery_attempts: vec![],
            total_processing_time_ms: 10,
        })
    }

    async fn send_batch_notifications(
        &self,
        _notifications: &[Notification],
    ) -> application::ApplicationResult<BatchNotificationResult> {
        Ok(BatchNotificationResult {
            batch_id: "batch-123".to_string(),
            total_notifications: _notifications.len(),
            successful_deliveries: _notifications.len(),
            failed_deliveries: 0,
            results: vec![],
            processing_time_ms: 50,
        })
    }

    async fn health_check(&self) -> application::ApplicationResult<NotificationHealth> {
        Ok(NotificationServiceHealth {
            is_healthy: true,
            active_subscriptions: 0,
            pending_notifications: 0,
            delivery_success_rate: 1.0,
            average_delivery_time_ms: 5,
            last_error: None,
            target_health: vec![],
        })
    }

    async fn subscribe(
        &self,
        _subscription: &NotificationSubscription,
    ) -> application::ApplicationResult<SubscriptionId> {
        Ok("test-subscription-123".to_string())
    }

    async fn unsubscribe(
        &self,
        _subscription_id: &SubscriptionId,
    ) -> application::ApplicationResult<()> {
        Ok(())
    }

    async fn get_delivery_status(
        &self,
        _notification_id: &str,
    ) -> application::ApplicationResult<DeliveryStatus> {
        Ok(DeliveryStatus::Delivered)
    }
}

/// Mock store use case for testing
#[derive(Clone)]
struct MockStoreUseCase {
    should_fail: bool,
}

#[async_trait::async_trait]
impl StoreMemoryUseCase for MockStoreUseCase {
    async fn store_memory(
        &self,
        request: StoreMemoryRequest,
        _context: RequestContext,
    ) -> application::ApplicationResult<StoreMemoryResponse> {
        if self.should_fail {
            return Err(application::ApplicationError::infrastructure(
                "Mock store failure",
            ));
        }

        Ok(StoreMemoryResponse {
            record_id: format!("stored-{}", request.content.len()),
            layer: domain::value_objects::layer_type::LayerType::Interact,
            embedding_dimensions: 384,
            processing_time_ms: 50,
            estimated_retrieval_time_ms: 5,
        })
    }

    async fn store_batch_memory(
        &self,
        request: BatchStoreMemoryRequest,
        _context: RequestContext,
    ) -> application::ApplicationResult<BatchStoreMemoryResponse> {
        if self.should_fail {
            return Err(application::ApplicationError::infrastructure(
                "Mock batch store failure",
            ));
        }

        let total = request.records.len();
        let results = (0..total)
            .map(|i| BatchStoreResult {
                index: i,
                success: true,
                record_id: Some(format!("batch-{}", i)),
                error: None,
                layer: Some(domain::value_objects::layer_type::LayerType::Interact),
            })
            .collect();

        Ok(BatchStoreMemoryResponse {
            total_requested: total,
            successful: total,
            failed: 0,
            results,
            total_processing_time_ms: total as u64 * 20,
        })
    }

    async fn retrieve_memory(
        &self,
        request: RetrieveMemoryRequest,
        _context: RequestContext,
    ) -> application::ApplicationResult<RetrieveMemoryResponse> {
        if self.should_fail {
            return Err(application::ApplicationError::not_found(
                "Record",
                "not found",
            ));
        }

        Ok(RetrieveMemoryResponse {
            record_id: request.record_id,
            content: "Retrieved content".to_string(),
            metadata: None,
            layer: domain::value_objects::layer_type::LayerType::Interact,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 1,
            embedding: None,
            stats: None,
        })
    }
}

/// Mock promotion use case for testing
#[derive(Clone)]
struct MockPromotionUseCase {
    should_fail: bool,
}

#[async_trait::async_trait]
impl PromoteRecordsUseCase for MockPromotionUseCase {
    async fn promote_records(
        &self,
        request: PromoteRecordsRequest,
        _context: RequestContext,
    ) -> application::ApplicationResult<PromoteRecordsResponse> {
        if self.should_fail {
            return Err(application::ApplicationError::infrastructure(
                "Mock promotion failure",
            ));
        }

        Ok(PromoteRecordsResponse {
            analysis_id: "analysis-test-123".to_string(),
            total_candidates: 10,
            promoted_count: 1,
            promoted_records: vec![RecordPromotion {
                record_id: "promoted-1".to_string(),
                from_layer: domain::value_objects::layer_type::LayerType::Assets,
                to_layer: domain::value_objects::layer_type::LayerType::Insights,
                success: true,
                promotion_score: 0.85,
                promotion_reason: "High access frequency".to_string(),
                estimated_benefit: 0.8,
                processing_time_ms: 50,
                error: None,
            }],
            skipped_count: 0,
            failed_count: 0,
            failed_promotions: 0,
            promotion_details: vec![],
            analysis_time_ms: 100,
            total_processing_time_ms: 150,
            candidates_analysis_time_ms: 75,
            promotion_execution_time_ms: 75,
            dry_run: request.dry_run,
        })
    }

    async fn analyze_promotion_candidates(
        &self,
        _request: AnalyzePromotionRequest,
        _context: RequestContext,
    ) -> application::ApplicationResult<AnalyzePromotionResponse> {
        if self.should_fail {
            return Err(application::ApplicationError::infrastructure(
                "Mock analysis failure",
            ));
        }

        Ok(AnalyzePromotionResponse {
            analysis_id: "analysis-123".to_string(),
            candidates: vec![PromotionCandidate {
                record_id: "candidate-1".to_string(),
                current_layer: domain::value_objects::layer_type::LayerType::Assets,
                recommended_layer: domain::value_objects::layer_type::LayerType::Insights,
                confidence_score: 0.85,
                access_frequency: 25,
                last_accessed: chrono::Utc::now() - chrono::Duration::hours(2),
                similarity_cluster: None,
                business_value_score: 0.7,
                promotion_urgency: PromotionUrgency::High,
            }],
            layer_statistics: LayerStatistics {
                cache_stats: LayerStats {
                    total_records: 100,
                    total_size_mb: 10.0,
                    average_access_frequency: 5.0,
                    hottest_records: vec![],
                    coldest_records: vec![],
                    utilization_percentage: 75.0,
                },
                index_stats: LayerStats {
                    total_records: 500,
                    total_size_mb: 50.0,
                    average_access_frequency: 3.0,
                    hottest_records: vec![],
                    coldest_records: vec![],
                    utilization_percentage: 60.0,
                },
                storage_stats: LayerStats {
                    total_records: 1000,
                    total_size_mb: 100.0,
                    average_access_frequency: 1.0,
                    hottest_records: vec![],
                    coldest_records: vec![],
                    utilization_percentage: 90.0,
                },
                cross_layer_patterns: CrossLayerPatterns {
                    promotion_velocity: 10.0,
                    demotion_velocity: 5.0,
                    layer_transition_matrix: vec![
                        vec![0.9, 0.1, 0.0],
                        vec![0.05, 0.85, 0.1],
                        vec![0.0, 0.02, 0.98],
                    ],
                    access_pattern_clusters: vec![],
                },
            },
            recommendations: vec![],
            ml_model_metrics: None,
            analysis_metadata: AnalysisMetadata {
                analysis_time_ms: 100,
                records_analyzed: 1000,
                data_quality_score: 0.95,
                analysis_completeness: 1.0,
                warnings: vec![],
                limitations: vec![],
            },
        })
    }

    async fn force_promote_records(
        &self,
        record_ids: Vec<String>,
        target_layer: domain::value_objects::layer_type::LayerType,
        _context: RequestContext,
    ) -> application::ApplicationResult<PromoteRecordsResponse> {
        Ok(PromoteRecordsResponse {
            promoted_records: record_ids
                .into_iter()
                .map(|id| RecordPromotion {
                    record_id: id,
                    from_layer: domain::value_objects::layer_type::LayerType::Assets,
                    to_layer: target_layer,
                    success: true,
                    promotion_score: 1.0,
                    promotion_reason: "Force promotion".to_string(),
                    estimated_benefit: 1.0,
                    processing_time_ms: 25,
                    error: None,
                })
                .collect(),
            analysis_id: "force-promotion".to_string(),
            total_candidates: 1,
            promoted_count: 1,
            skipped_count: 0,
            failed_count: 0,
            failed_promotions: 0,
            promotion_details: vec![],
            analysis_time_ms: 25,
            total_processing_time_ms: 50,
            candidates_analysis_time_ms: 10,
            promotion_execution_time_ms: 40,
            dry_run: false,
        })
    }

    async fn run_automated_promotion_cycle(
        &self,
        _context: RequestContext,
    ) -> application::ApplicationResult<PromoteRecordsResponse> {
        Ok(PromoteRecordsResponse {
            analysis_id: "auto-promotion".to_string(),
            total_candidates: 0,
            promoted_count: 0,
            promoted_records: vec![],
            skipped_count: 0,
            failed_count: 0,
            failed_promotions: 0,
            promotion_details: vec![],
            analysis_time_ms: 5,
            total_processing_time_ms: 10,
            candidates_analysis_time_ms: 5,
            promotion_execution_time_ms: 5,
            dry_run: false,
        })
    }
}

fn create_test_context() -> RequestContext {
    RequestContext::new(RequestSource::Internal)
}

fn create_test_memory_service() -> MemoryApplicationServiceImpl {
    let store_use_case = Arc::new(MockStoreUseCase { should_fail: false });
    let promotion_use_case = Arc::new(MockPromotionUseCase { should_fail: false });
    let metrics_collector = Arc::new(MockMetricsCollector);
    let notification_service = Arc::new(MockNotificationService);

    MemoryApplicationServiceImpl::with_default_config(
        store_use_case,
        promotion_use_case,
        metrics_collector,
        notification_service,
    )
}

#[tokio::test]
async fn test_memory_application_service_store_record() {
    let service = create_test_memory_service();
    let context = create_test_context();

    let request = StoreMemoryRequest {
        content: "Test content for application service".to_string(),
        metadata: None,
        project: "integration-test".to_string(),
        kind: Some("test".to_string()),
        session: Some("session-123".to_string()),
        target_layer: None,
        priority: Some(3),
        tags: vec!["integration".to_string(), "test".to_string()],
    };

    let result = service.store_memory_record(request, context).await;

    assert!(result.is_ok());
    let response = result.expect("Test operation should succeed");
    assert!(response.record_id.starts_with("stored-"));
    assert_eq!(response.embedding_dimensions, 384);
    assert_eq!(response.processing_time_ms, 50);
}

#[tokio::test]
async fn test_memory_application_service_batch_store() {
    let service = create_test_memory_service();
    let context = create_test_context();

    let records = vec![
        StoreMemoryRequest {
            content: "Batch record 1".to_string(),
            metadata: None,
            project: "test".to_string(),
            kind: None,
            session: None,
            target_layer: None,
            priority: None,
            tags: vec![],
        },
        StoreMemoryRequest {
            content: "Batch record 2".to_string(),
            metadata: None,
            project: "test".to_string(),
            kind: None,
            session: None,
            target_layer: None,
            priority: None,
            tags: vec![],
        },
        StoreMemoryRequest {
            content: "Batch record 3".to_string(),
            metadata: None,
            project: "test".to_string(),
            kind: None,
            session: None,
            target_layer: None,
            priority: None,
            tags: vec![],
        },
    ];

    let batch_request = BatchStoreMemoryRequest {
        records,
        options: BatchOptions {
            parallel_processing: true,
            failure_tolerance: FailureTolerance::Partial,
            progress_reporting: true,
        },
    };

    let result = service.store_memory_batch(batch_request, context).await;

    assert!(result.is_ok());
    let response = result.expect("Test operation should succeed");
    assert_eq!(response.total_requested, 3);
    assert_eq!(response.successful, 3);
    assert_eq!(response.failed, 0);
    assert_eq!(response.results.len(), 3);
}

#[tokio::test]
async fn test_memory_application_service_promote_records() {
    let service = create_test_memory_service();
    let context = create_test_context();

    let criteria = vec![PromotionCriterion {
        min_access_frequency: Some(10),
        min_similarity_score: Some(0.75),
        min_score_threshold: Some(0.8),
        time_window_hours: Some(24),
        max_hours_since_access: Some(24),
        from_layer: Some(domain::value_objects::layer_type::LayerType::Assets),
        to_layer: domain::value_objects::layer_type::LayerType::Insights,
        target_layers: vec![domain::value_objects::layer_type::LayerType::Insights],
        project_filter: None,
        boost_recent_activity: true,
    }];

    let request = PromoteRecordsRequest {
        record_ids: None,
        from_layer: Some(domain::value_objects::layer_type::LayerType::Assets),
        to_layer: domain::value_objects::layer_type::LayerType::Insights,
        criteria,
        max_candidates: Some(50),
        force: false,
        dry_run: false,
    };

    let result = service.promote_memory_records(request, context).await;

    assert!(result.is_ok());
    let response = result.expect("Test operation should succeed");
    assert_eq!(response.promoted_records.len(), 1);
    assert_eq!(response.promoted_records[0].record_id, "promoted-1");
    assert!(!response.dry_run);
}

#[tokio::test]
async fn test_memory_application_service_analyze_promotion() {
    let service = create_test_memory_service();
    let context = create_test_context();

    let request = AnalyzePromotionRequest {
        source_layers: vec![domain::value_objects::layer_type::LayerType::Assets],
        target_layer: domain::value_objects::layer_type::LayerType::Insights,
        analysis_depth: AnalysisDepth::Standard,
        time_window_hours: 48,
        include_ml_predictions: true,
    };

    let result = service
        .analyze_promotion_opportunities(request, context)
        .await;

    assert!(result.is_ok());
    let response = result.expect("Test operation should succeed");
    assert_eq!(response.candidates.len(), 1);
    assert_eq!(response.candidates[0].record_id, "candidate-1");
    assert_eq!(response.analysis_metadata.records_analyzed, 1000);
}

#[tokio::test]
async fn test_memory_application_service_complete_workflow() {
    let service = create_test_memory_service();
    let context = create_test_context();

    let request = StoreMemoryRequest {
        content: "Workflow test content".to_string(),
        metadata: None,
        project: "workflow-test".to_string(),
        kind: Some("workflow".to_string()),
        session: Some("workflow-session".to_string()),
        target_layer: None,
        priority: Some(4), // High priority to trigger analysis
        tags: vec!["workflow".to_string()],
    };

    let result = service.complete_memory_workflow(request, context).await;

    assert!(result.is_ok());
    let response = result.expect("Test operation should succeed");
    assert!(response.store_response.record_id.starts_with("stored-"));
    assert!(response.promotion_analysis.is_some());
    assert!(!response.workflow_recommendations.is_empty());
}

#[tokio::test]
async fn test_memory_application_service_health_check() {
    let service = create_test_memory_service();
    let context = create_test_context();

    let result = service.health_check(context).await;

    assert!(result.is_ok());
    let health = result.expect("Test operation should succeed");
    assert!(matches!(health.overall_status, HealthStatus::Healthy));
    assert!(health.health_score > 0.8);
    assert!(matches!(
        health.store_subsystem.status,
        HealthStatus::Healthy
    ));
    assert!(matches!(
        health.promotion_subsystem.status,
        HealthStatus::Healthy
    ));
    assert!(matches!(
        health.cache_subsystem.status,
        HealthStatus::Healthy
    ));
}

#[tokio::test]
async fn test_memory_service_config_defaults() {
    let config = MemoryServiceConfig::default();

    assert!(config.auto_promotion_enabled);
    assert_eq!(config.promotion_threshold, 0.8);
    assert_eq!(config.batch_size_limit, 1000);
    assert_eq!(config.transaction_timeout_seconds, 300);
    assert!(config.enable_workflow_optimization);
}

#[tokio::test]
async fn test_application_error_handling() {
    let store_use_case = Arc::new(MockStoreUseCase { should_fail: true });
    let promotion_use_case = Arc::new(MockPromotionUseCase { should_fail: false });
    let metrics_collector = Arc::new(MockMetricsCollector);
    let notification_service = Arc::new(MockNotificationService);

    let service = MemoryApplicationServiceImpl::with_default_config(
        store_use_case,
        promotion_use_case,
        metrics_collector,
        notification_service,
    );

    let context = create_test_context();
    let request = StoreMemoryRequest {
        content: "This should fail".to_string(),
        metadata: None,
        project: "test".to_string(),
        kind: None,
        session: None,
        target_layer: None,
        priority: None,
        tags: vec![],
    };

    let result = service.store_memory_record(request, context).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Mock store failure"));
}
