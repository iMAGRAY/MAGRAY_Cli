//! Integration tests for Application Services

use application::{RequestContext, RequestSource};
use application::services::*;
use application::use_cases::*;
use application::dtos::*;
use application::ports::*;
use std::sync::Arc;
use std::collections::HashMap;

/// Mock metrics collector for testing
#[derive(Clone)]
struct MockMetricsCollector;

#[async_trait::async_trait]
impl MetricsCollector for MockMetricsCollector {
    async fn increment_counter(&self, _name: &str, _value: u64, _tags: Option<&HashMap<String, String>>) -> application::ApplicationResult<()> {
        Ok(())
    }
    
    async fn record_gauge(&self, _name: &str, _value: f64, _tags: Option<&HashMap<String, String>>) -> application::ApplicationResult<()> {
        Ok(())
    }
    
    async fn record_timing(&self, _name: &str, _value_ms: u64, _tags: Option<&HashMap<String, String>>) -> application::ApplicationResult<()> {
        Ok(())
    }
    
    async fn record_memory_operation(&self, _operation: MemoryOperation) -> application::ApplicationResult<()> {
        Ok(())
    }
    
    async fn health_check(&self) -> application::ApplicationResult<MetricsHealth> {
        Ok(MetricsHealth {
            is_healthy: true,
            metrics_collected: 100,
            failed_collections: 0,
            last_collection: chrono::Utc::now(),
            buffer_size: 50,
            collection_rate: 10.0,
        })
    }
}

/// Mock notification service for testing
#[derive(Clone)]
struct MockNotificationService;

#[async_trait::async_trait]
impl NotificationService for MockNotificationService {
    async fn send_notification(&self, _notification: &Notification) -> application::ApplicationResult<()> {
        Ok(())
    }
    
    async fn send_batch_notifications(&self, _notifications: &[Notification]) -> application::ApplicationResult<()> {
        Ok(())
    }
    
    async fn health_check(&self) -> application::ApplicationResult<NotificationHealth> {
        Ok(NotificationHealth {
            is_healthy: true,
            queued_notifications: 0,
            failed_notifications: 0,
            last_notification_sent: Some(chrono::Utc::now()),
            response_time_ms: 5,
            error_rate: 0.0,
        })
    }
}

/// Mock store use case for testing
#[derive(Clone)]
struct MockStoreUseCase {
    should_fail: bool,
}

#[async_trait::async_trait]
impl StoreMemoryUseCase for MockStoreUseCase {
    async fn store_memory(&self, request: StoreMemoryRequest, _context: RequestContext) -> application::ApplicationResult<StoreMemoryResponse> {
        if self.should_fail {
            return Err(application::ApplicationError::infrastructure("Mock store failure"));
        }
        
        Ok(StoreMemoryResponse {
            record_id: format!("stored-{}", request.content.len()),
            layer: domain::value_objects::layer_type::LayerType::Cache,
            embedding_dimensions: 384,
            processing_time_ms: 50,
            estimated_retrieval_time_ms: 5,
        })
    }
    
    async fn store_batch_memory(&self, request: BatchStoreMemoryRequest, _context: RequestContext) -> application::ApplicationResult<BatchStoreMemoryResponse> {
        if self.should_fail {
            return Err(application::ApplicationError::infrastructure("Mock batch store failure"));
        }
        
        let total = request.records.len();
        let results = (0..total).map(|i| BatchStoreResult {
            index: i,
            success: true,
            record_id: Some(format!("batch-{}", i)),
            error: None,
            layer: Some(domain::value_objects::layer_type::LayerType::Cache),
        }).collect();
        
        Ok(BatchStoreMemoryResponse {
            total_requested: total,
            successful: total,
            failed: 0,
            results,
            total_processing_time_ms: total as u64 * 20,
        })
    }
    
    async fn retrieve_memory(&self, request: RetrieveMemoryRequest, _context: RequestContext) -> application::ApplicationResult<RetrieveMemoryResponse> {
        if self.should_fail {
            return Err(application::ApplicationError::not_found("Record not found"));
        }
        
        Ok(RetrieveMemoryResponse {
            record_id: request.record_id,
            content: "Retrieved content".to_string(),
            metadata: None,
            layer: domain::value_objects::layer_type::LayerType::Cache,
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
    async fn promote_records(&self, request: PromoteRecordsRequest, _context: RequestContext) -> application::ApplicationResult<PromoteRecordsResponse> {
        if self.should_fail {
            return Err(application::ApplicationError::infrastructure("Mock promotion failure"));
        }
        
        Ok(PromoteRecordsResponse {
            promoted_records: vec![
                PromotedRecord {
                    record_id: "promoted-1".to_string(),
                    from_layer: domain::value_objects::layer_type::LayerType::Storage,
                    to_layer: domain::value_objects::layer_type::LayerType::Index,
                    promotion_score: 0.85,
                    promotion_reason: "High access frequency".to_string(),
                }
            ],
            total_candidates: 10,
            promotion_time_ms: 150,
            dry_run: request.dry_run,
        })
    }
    
    async fn analyze_promotion_candidates(&self, _request: AnalyzePromotionRequest, _context: RequestContext) -> application::ApplicationResult<AnalyzePromotionResponse> {
        if self.should_fail {
            return Err(application::ApplicationError::infrastructure("Mock analysis failure"));
        }
        
        Ok(AnalyzePromotionResponse {
            candidates: vec![
                PromotionCandidate {
                    record_id: "candidate-1".to_string(),
                    current_layer: domain::value_objects::layer_type::LayerType::Storage,
                    recommended_layer: domain::value_objects::layer_type::LayerType::Index,
                    confidence_score: 0.85,
                    access_frequency: 25.0,
                    last_access: chrono::Utc::now() - chrono::Duration::hours(2),
                    promotion_benefit: 0.7,
                }
            ],
            analysis_time_ms: 100,
            total_records_analyzed: 1000,
            recommendation: "Promote high-confidence candidates".to_string(),
        })
    }
    
    async fn force_promote_records(&self, record_ids: Vec<String>, target_layer: domain::value_objects::layer_type::LayerType, _context: RequestContext) -> application::ApplicationResult<PromoteRecordsResponse> {
        Ok(PromoteRecordsResponse {
            promoted_records: record_ids.into_iter().map(|id| PromotedRecord {
                record_id: id,
                from_layer: domain::value_objects::layer_type::LayerType::Storage,
                to_layer: target_layer,
                promotion_score: 1.0,
                promotion_reason: "Force promotion".to_string(),
            }).collect(),
            total_candidates: 1,
            promotion_time_ms: 50,
            dry_run: false,
        })
    }
    
    async fn run_automated_promotion_cycle(&self, _context: RequestContext) -> application::ApplicationResult<PromoteRecordsResponse> {
        Ok(PromoteRecordsResponse {
            promoted_records: vec![],
            total_candidates: 0,
            promotion_time_ms: 10,
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
        project: Some("integration-test".to_string()),
        target_layer: None,
        priority: Some(3),
        tags: vec!["integration".to_string(), "test".to_string()],
    };
    
    let result = service.store_memory_record(request, context).await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
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
            project: None,
            target_layer: None,
            priority: None,
            tags: vec![],
        },
        StoreMemoryRequest {
            content: "Batch record 2".to_string(),
            metadata: None,
            project: None,
            target_layer: None,
            priority: None,
            tags: vec![],
        },
        StoreMemoryRequest {
            content: "Batch record 3".to_string(),
            metadata: None,
            project: None,
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
    let response = result.unwrap();
    assert_eq!(response.total_requested, 3);
    assert_eq!(response.successful, 3);
    assert_eq!(response.failed, 0);
    assert_eq!(response.results.len(), 3);
}

#[tokio::test]
async fn test_memory_application_service_promote_records() {
    let service = create_test_memory_service();
    let context = create_test_context();
    
    let criteria = vec![
        PromotionCriterion {
            min_access_frequency: 10,
            min_score_threshold: 0.8,
            max_hours_since_access: Some(24),
            target_layers: vec![domain::value_objects::layer_type::LayerType::Index],
            project_filter: None,
        }
    ];
    
    let request = PromoteRecordsRequest {
        criteria,
        max_candidates: Some(50),
        dry_run: false,
    };
    
    let result = service.promote_memory_records(request, context).await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.promoted_records.len(), 1);
    assert_eq!(response.promoted_records[0].record_id, "promoted-1");
    assert!(!response.dry_run);
}

#[tokio::test]
async fn test_memory_application_service_analyze_promotion() {
    let service = create_test_memory_service();
    let context = create_test_context();
    
    let request = AnalyzePromotionRequest {
        layers: Some(vec![domain::value_objects::layer_type::LayerType::Storage]),
        time_window_hours: Some(48),
        min_access_frequency: Some(5),
        min_score_threshold: Some(0.7),
        max_hours_since_access: Some(168),
    };
    
    let result = service.analyze_promotion_opportunities(request, context).await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.candidates.len(), 1);
    assert_eq!(response.candidates[0].record_id, "candidate-1");
    assert_eq!(response.total_records_analyzed, 1000);
}

#[tokio::test]
async fn test_memory_application_service_complete_workflow() {
    let service = create_test_memory_service();
    let context = create_test_context();
    
    let request = StoreMemoryRequest {
        content: "Workflow test content".to_string(),
        metadata: None,
        project: Some("workflow-test".to_string()),
        target_layer: None,
        priority: Some(4), // High priority to trigger analysis
        tags: vec!["workflow".to_string()],
    };
    
    let result = service.complete_memory_workflow(request, context).await;
    
    assert!(result.is_ok());
    let response = result.unwrap();
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
    let health = result.unwrap();
    assert!(matches!(health.overall_status, HealthStatus::Healthy));
    assert!(health.health_score > 0.8);
    assert!(matches!(health.store_subsystem.status, HealthStatus::Healthy));
    assert!(matches!(health.promotion_subsystem.status, HealthStatus::Healthy));
    assert!(matches!(health.cache_subsystem.status, HealthStatus::Healthy));
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
        project: None,
        target_layer: None,
        priority: None,
        tags: vec![],
    };
    
    let result = service.store_memory_record(request, context).await;
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Mock store failure"));
}