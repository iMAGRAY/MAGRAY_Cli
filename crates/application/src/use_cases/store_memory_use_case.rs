//! Store Memory Use Case
//!
//! Бизнес-логика для сохранения записей в memory system с автоматическим 
//! определением слоя и generation embeddings.

use async_trait::async_trait;
use crate::{ApplicationResult, ApplicationError, RequestContext};
use crate::dtos::{StoreMemoryRequest, StoreMemoryResponse, BatchStoreMemoryRequest, BatchStoreMemoryResponse, RetrieveMemoryRequest, RetrieveMemoryResponse};
use crate::ports::{EmbeddingProvider, MetricsCollector, NotificationService};
use domain::entities::memory_record::MemoryRecord;
use domain::entities::record_id::RecordId;
use domain::entities::embedding_vector::EmbeddingVector;
use domain::repositories::memory_repository::MemoryRepository;
use domain::services::memory_domain_service::MemoryDomainService;
use domain::value_objects::layer_type::LayerType;
use std::sync::Arc;
use tracing::{info, warn, error, instrument};

/// Use case для сохранения записей в память
#[async_trait]
pub trait StoreMemoryUseCase: Send + Sync {
    /// Store single memory record
    async fn store_memory(&self, request: StoreMemoryRequest, context: RequestContext) -> ApplicationResult<StoreMemoryResponse>;
    
    /// Store multiple memory records in batch
    async fn store_batch_memory(&self, request: BatchStoreMemoryRequest, context: RequestContext) -> ApplicationResult<BatchStoreMemoryResponse>;
    
    /// Retrieve single memory record
    async fn retrieve_memory(&self, request: RetrieveMemoryRequest, context: RequestContext) -> ApplicationResult<RetrieveMemoryResponse>;
}

/// Implementation of store memory use case
pub struct StoreMemoryUseCaseImpl {
    memory_repository: Arc<dyn MemoryRepository>,
    memory_domain_service: Arc<dyn MemoryDomainService>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    metrics_collector: Arc<dyn MetricsCollector>,
    notification_service: Arc<dyn NotificationService>,
}

impl StoreMemoryUseCaseImpl {
    pub fn new(
        memory_repository: Arc<dyn MemoryRepository>,
        memory_domain_service: Arc<dyn MemoryDomainService>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        metrics_collector: Arc<dyn MetricsCollector>,
        notification_service: Arc<dyn NotificationService>,
    ) -> Self {
        Self {
            memory_repository,
            memory_domain_service,
            embedding_provider,
            metrics_collector,
            notification_service,
        }
    }
}

#[async_trait]
impl StoreMemoryUseCase for StoreMemoryUseCaseImpl {
    #[instrument(skip(self, request), fields(content_length = request.content.len()))]
    async fn store_memory(&self, request: StoreMemoryRequest, context: RequestContext) -> ApplicationResult<StoreMemoryResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Starting store memory operation for request: {}", context.request_id);
        
        // Validate request
        self.validate_store_request(&request)?;
        
        // Generate embedding
        let embedding_start = std::time::Instant::now();
        let embedding = self.generate_embedding(&request.content).await?;
        let embedding_time = embedding_start.elapsed();
        
        // Create record ID
        let record_id = RecordId::generate();
        
        // Determine target layer
        let target_layer = self.determine_target_layer(&request, &embedding).await?;
        
        // Create memory record
        let memory_record = self.create_memory_record(record_id.clone(), &request, embedding, target_layer)?;
        
        // Store in repository
        let store_start = std::time::Instant::now();
        self.memory_repository.store(&memory_record).await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to store memory record", e))?;
        let store_time = store_start.elapsed();
        
        let total_time = start_time.elapsed();
        
        // Record metrics
        self.record_store_metrics(&memory_record, total_time, embedding_time, store_time).await?;
        
        // Send notification for high-priority records
        if request.priority.unwrap_or(1) >= 3 {
            self.send_store_notification(&memory_record, &context).await?;
        }
        
        let response = StoreMemoryResponse {
            record_id: record_id.to_string(),
            layer: target_layer,
            embedding_dimensions: embedding.dimensions(),
            processing_time_ms: total_time.as_millis() as u64,
            estimated_retrieval_time_ms: self.estimate_retrieval_time(target_layer),
        };
        
        info!("Successfully stored memory record: {} in layer: {:?}", record_id, target_layer);
        
        Ok(response)
    }

    #[instrument(skip(self, request), fields(record_count = request.records.len()))]
    async fn store_batch_memory(&self, request: BatchStoreMemoryRequest, context: RequestContext) -> ApplicationResult<BatchStoreMemoryResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Starting batch store operation for {} records", request.records.len());
        
        // Validate batch request
        self.validate_batch_request(&request)?;
        
        let mut results = Vec::new();
        let mut successful = 0;
        let mut failed = 0;
        
        // Process records based on options
        if request.options.parallel_processing {
            results = self.process_batch_parallel(&request.records, &context).await?;
        } else {
            results = self.process_batch_sequential(&request.records, &context).await?;
        }
        
        // Count results
        for result in &results {
            if result.success {
                successful += 1;
            } else {
                failed += 1;
            }
        }
        
        let total_time = start_time.elapsed();
        
        // Record batch metrics
        self.record_batch_metrics(request.records.len(), successful, failed, total_time).await?;
        
        // Send notification for batch completion
        self.send_batch_notification(request.records.len(), successful, failed, &context).await?;
        
        let response = BatchStoreMemoryResponse {
            total_requested: request.records.len(),
            successful,
            failed,
            results,
            total_processing_time_ms: total_time.as_millis() as u64,
        };
        
        info!("Batch store completed: {}/{} successful", successful, request.records.len());
        
        Ok(response)
    }

    #[instrument(skip(self, request), fields(record_id = %request.record_id))]
    async fn retrieve_memory(&self, request: RetrieveMemoryRequest, context: RequestContext) -> ApplicationResult<RetrieveMemoryResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Retrieving memory record: {}", request.record_id);
        
        // Validate request
        if request.record_id.is_empty() {
            return Err(ApplicationError::validation("Record ID cannot be empty"));
        }
        
        // Parse record ID
        let record_id = RecordId::from_string(&request.record_id)
            .map_err(|e| ApplicationError::validation(format!("Invalid record ID format: {}", e)))?;
        
        // Retrieve from repository
        let memory_record = self.memory_repository.find_by_id(&record_id).await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to retrieve memory record", e))?
            .ok_or_else(|| ApplicationError::not_found(format!("Memory record not found: {}", request.record_id)))?;
        
        let total_time = start_time.elapsed();
        
        // Record metrics
        self.record_retrieve_metrics(&memory_record, total_time, request.include_stats).await?;
        
        // Build response
        let response = RetrieveMemoryResponse {
            record_id: memory_record.id().to_string(),
            content: memory_record.content().clone(),
            metadata: memory_record.metadata().cloned(),
            layer: *memory_record.layer(),
            created_at: memory_record.created_at(),
            last_accessed: memory_record.last_accessed(),
            access_count: memory_record.access_count(),
            embedding: if request.include_embedding {
                Some(memory_record.embedding().as_vec().clone())
            } else {
                None
            },
            stats: if request.include_stats {
                Some(crate::dtos::RecordStats {
                    retrieval_time_ms: total_time.as_millis() as u64,
                    cache_hit: matches!(memory_record.layer(), LayerType::Cache),
                    layer_promotion_candidate: self.is_promotion_candidate(&memory_record).await?,
                    similarity_scores: None, // TODO: Implement if needed
                })
            } else {
                None
            },
        };
        
        info!("Successfully retrieved memory record: {}", request.record_id);
        
        Ok(response)
    }
}

impl StoreMemoryUseCaseImpl {
    /// Validate single store request
    fn validate_store_request(&self, request: &StoreMemoryRequest) -> ApplicationResult<()> {
        if request.content.is_empty() {
            return Err(ApplicationError::validation("Content cannot be empty"));
        }
        
        if request.content.len() > 100_000 {
            return Err(ApplicationError::validation("Content too large (max 100,000 characters)"));
        }
        
        if request.tags.len() > 50 {
            return Err(ApplicationError::validation("Too many tags (max 50)"));
        }
        
        for tag in &request.tags {
            if tag.is_empty() || tag.len() > 100 {
                return Err(ApplicationError::validation("Invalid tag length"));
            }
        }
        
        Ok(())
    }
    
    /// Validate batch request
    fn validate_batch_request(&self, request: &BatchStoreMemoryRequest) -> ApplicationResult<()> {
        if request.records.is_empty() {
            return Err(ApplicationError::validation("No records to store"));
        }
        
        if request.records.len() > 100 {
            return Err(ApplicationError::validation("Too many records in batch (max 100)"));
        }
        
        for (index, record) in request.records.iter().enumerate() {
            self.validate_store_request(record)
                .map_err(|e| ApplicationError::validation(format!("Record {}: {}", index, e)))?;
        }
        
        Ok(())
    }
    
    /// Generate embedding for content
    async fn generate_embedding(&self, content: &str) -> ApplicationResult<EmbeddingVector> {
        let raw_embedding = self.embedding_provider.generate_embedding(content).await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to generate embedding", e))?;
        
        EmbeddingVector::new(raw_embedding)
            .map_err(|e| ApplicationError::Domain(e))
    }
    
    /// Determine target layer for storage
    async fn determine_target_layer(&self, request: &StoreMemoryRequest, embedding: &EmbeddingVector) -> ApplicationResult<LayerType> {
        // Use explicit target if provided
        if let Some(target_layer) = request.target_layer {
            return Ok(target_layer);
        }
        
        // Use domain service to determine optimal layer
        self.memory_domain_service.determine_initial_layer(
            &request.content,
            embedding,
            request.priority.unwrap_or(1),
            request.project.as_deref(),
        ).await.map_err(|e| ApplicationError::Domain(e))
    }
    
    /// Create memory record from request
    fn create_memory_record(
        &self,
        record_id: RecordId,
        request: &StoreMemoryRequest,
        embedding: EmbeddingVector,
        target_layer: LayerType,
    ) -> ApplicationResult<MemoryRecord> {
        MemoryRecord::new(
            record_id,
            request.content.clone(),
            embedding,
            target_layer,
            request.project.clone(),
            request.metadata.clone(),
            request.tags.clone(),
        ).map_err(|e| ApplicationError::Domain(e))
    }
    
    /// Process batch in parallel
    async fn process_batch_parallel(
        &self,
        records: &[StoreMemoryRequest],
        context: &RequestContext,
    ) -> ApplicationResult<Vec<crate::dtos::BatchStoreResult>> {
        use tokio::task::JoinSet;
        
        let mut join_set = JoinSet::new();
        
        for (index, record) in records.iter().enumerate() {
            let record = record.clone();
            let context = context.clone();
            let use_case = self.clone();
            
            join_set.spawn(async move {
                let result = use_case.store_memory(record, context).await;
                (index, result)
            });
        }
        
        let mut results = vec![crate::dtos::BatchStoreResult::default(); records.len()];
        
        while let Some(task_result) = join_set.join_next().await {
            match task_result {
                Ok((index, store_result)) => {
                    results[index] = match store_result {
                        Ok(response) => crate::dtos::BatchStoreResult {
                            index,
                            success: true,
                            record_id: Some(response.record_id),
                            error: None,
                            layer: Some(response.layer),
                        },
                        Err(e) => crate::dtos::BatchStoreResult {
                            index,
                            success: false,
                            record_id: None,
                            error: Some(e.to_string()),
                            layer: None,
                        },
                    };
                }
                Err(e) => {
                    error!("Task failed: {}", e);
                    return Err(ApplicationError::infrastructure(format!("Parallel processing failed: {}", e)));
                }
            }
        }
        
        Ok(results)
    }
    
    /// Process batch sequentially
    async fn process_batch_sequential(
        &self,
        records: &[StoreMemoryRequest],
        context: &RequestContext,
    ) -> ApplicationResult<Vec<crate::dtos::BatchStoreResult>> {
        let mut results = Vec::new();
        
        for (index, record) in records.iter().enumerate() {
            let result = self.store_memory(record.clone(), context.clone()).await;
            
            let batch_result = match result {
                Ok(response) => crate::dtos::BatchStoreResult {
                    index,
                    success: true,
                    record_id: Some(response.record_id),
                    error: None,
                    layer: Some(response.layer),
                },
                Err(e) => {
                    warn!("Failed to store record {}: {}", index, e);
                    crate::dtos::BatchStoreResult {
                        index,
                        success: false,
                        record_id: None,
                        error: Some(e.to_string()),
                        layer: None,
                    }
                }
            };
            
            results.push(batch_result);
        }
        
        Ok(results)
    }
    
    /// Record store metrics
    async fn record_store_metrics(
        &self,
        record: &MemoryRecord,
        total_time: std::time::Duration,
        embedding_time: std::time::Duration,
        store_time: std::time::Duration,
    ) -> ApplicationResult<()> {
        use crate::ports::{MemoryOperation, MemoryOperationType, MetricsCollectorExt};
        
        let operation = MemoryOperation {
            operation_type: MemoryOperationType::Store,
            layer: format!("{:?}", record.layer()),
            record_count: 1,
            processing_time_ms: total_time.as_millis() as u64,
            bytes_processed: record.content().len(),
            success: true,
            error: None,
        };
        
        self.metrics_collector.record_memory_operation(operation).await?;
        
        // Record detailed timing metrics
        self.metrics_collector.record_timing(
            "store_memory_embedding_generation",
            embedding_time.as_millis() as u64,
            None,
        ).await?;
        
        self.metrics_collector.record_timing(
            "store_memory_repository_store",
            store_time.as_millis() as u64,
            None,
        ).await?;
        
        Ok(())
    }
    
    /// Record batch metrics
    async fn record_batch_metrics(
        &self,
        total_records: usize,
        successful: usize,
        failed: usize,
        total_time: std::time::Duration,
    ) -> ApplicationResult<()> {
        self.metrics_collector.increment_counter(
            "batch_store_operations_total",
            1,
            None,
        ).await?;
        
        self.metrics_collector.record_gauge(
            "batch_store_success_rate",
            successful as f64 / total_records as f64,
            None,
        ).await?;
        
        self.metrics_collector.record_timing(
            "batch_store_duration",
            total_time.as_millis() as u64,
            None,
        ).await?;
        
        Ok(())
    }
    
    /// Send store notification
    async fn send_store_notification(&self, record: &MemoryRecord, context: &RequestContext) -> ApplicationResult<()> {
        use crate::ports::{Notification, NotificationLevel, NotificationCategory};
        
        let notification = Notification::info(
            "High Priority Memory Record Stored",
            &format!("Record {} stored in {:?} layer", record.id(), record.layer()),
        )
        .with_target(crate::ports::NotificationTarget::Log {
            level: log::Level::Info,
            target: "store_memory".to_string(),
        });
        
        self.notification_service.send_notification(&notification).await?;
        Ok(())
    }
    
    /// Send batch notification
    async fn send_batch_notification(
        &self,
        total: usize,
        successful: usize,
        failed: usize,
        context: &RequestContext,
    ) -> ApplicationResult<()> {
        use crate::ports::{Notification, NotificationLevel};
        
        let level = if failed == 0 {
            NotificationLevel::Info
        } else if failed < total / 2 {
            NotificationLevel::Warning
        } else {
            NotificationLevel::Error
        };
        
        let notification = Notification {
            level,
            title: "Batch Store Operation Completed".to_string(),
            message: format!(
                "Batch store completed: {}/{} successful, {} failed",
                successful, total, failed
            ),
            ..Notification::info("", "")
        }
        .with_target(crate::ports::NotificationTarget::Log {
            level: log::Level::Info,
            target: "batch_store".to_string(),
        });
        
        self.notification_service.send_notification(&notification).await?;
        Ok(())
    }
    
    /// Record retrieve metrics
    async fn record_retrieve_metrics(
        &self,
        record: &MemoryRecord,
        total_time: std::time::Duration,
        include_stats: bool,
    ) -> ApplicationResult<()> {
        use crate::ports::{MemoryOperation, MemoryOperationType};
        
        let operation = MemoryOperation {
            operation_type: MemoryOperationType::Retrieve,
            layer: format!("{:?}", record.layer()),
            record_count: 1,
            processing_time_ms: total_time.as_millis() as u64,
            bytes_processed: record.content().len(),
            success: true,
            error: None,
        };
        
        self.metrics_collector.record_memory_operation(operation).await?;
        
        if include_stats {
            self.metrics_collector.record_timing(
                "retrieve_memory_with_stats",
                total_time.as_millis() as u64,
                None,
            ).await?;
        }
        
        Ok(())
    }
    
    /// Check if record is a promotion candidate
    async fn is_promotion_candidate(&self, record: &MemoryRecord) -> ApplicationResult<bool> {
        // Use domain service to check promotion eligibility
        self.memory_domain_service.is_promotion_candidate(
            record,
            record.access_count() as f64,
            &chrono::Utc::now(),
        ).await.map_err(|e| ApplicationError::Domain(e))
    }
    
    /// Estimate retrieval time based on layer
    fn estimate_retrieval_time(&self, layer: LayerType) -> u64 {
        match layer {
            LayerType::Cache => 5,    // 5ms
            LayerType::Index => 25,   // 25ms
            LayerType::Storage => 100, // 100ms
        }
    }
}

// Clone implementation for parallel processing
impl Clone for StoreMemoryUseCaseImpl {
    fn clone(&self) -> Self {
        Self {
            memory_repository: self.memory_repository.clone(),
            memory_domain_service: self.memory_domain_service.clone(),
            embedding_provider: self.embedding_provider.clone(),
            metrics_collector: self.metrics_collector.clone(),
            notification_service: self.notification_service.clone(),
        }
    }
}

impl Default for crate::dtos::BatchStoreResult {
    fn default() -> Self {
        Self {
            index: 0,
            success: false,
            record_id: None,
            error: None,
            layer: None,
        }
    }
}