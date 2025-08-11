//! CQRS Handlers Implementation
//!
//! Конкретные обработчики команд и запросов для memory operations

use std::sync::Arc;
use async_trait::async_trait;
use tracing::{info, error, instrument};
use crate::{ApplicationResult, ApplicationError, RequestContext};
use crate::use_cases::*;
use crate::ports::{MetricsCollector, NotificationService};
use super::{CommandHandler, QueryHandler, Command, Query};
use super::commands::*;
use super::queries::*;

/// Handler for store memory commands
pub struct StoreMemoryCommandHandler {
    use_case: Arc<dyn StoreMemoryUseCase>,
    metrics: Arc<dyn MetricsCollector>,
}

/// Handler for batch store memory commands
pub struct BatchStoreMemoryCommandHandler {
    use_case: Arc<dyn StoreMemoryUseCase>,
    metrics: Arc<dyn MetricsCollector>,
}

/// Handler for promote records commands
pub struct PromoteRecordsCommandHandler {
    use_case: Arc<dyn PromoteRecordsUseCase>,
    metrics: Arc<dyn MetricsCollector>,
}

/// Handler for delete memory commands
pub struct DeleteMemoryCommandHandler {
    use_case: Arc<dyn StoreMemoryUseCase>,
    metrics: Arc<dyn MetricsCollector>,
}

/// Handler for search memory queries
pub struct SearchMemoryQueryHandler {
    use_case: Arc<dyn SearchMemoryUseCase>,
    metrics: Arc<dyn MetricsCollector>,
}

/// Handler for similarity search queries
pub struct SimilaritySearchQueryHandler {
    use_case: Arc<dyn SearchMemoryUseCase>,
    metrics: Arc<dyn MetricsCollector>,
}

/// Handler for retrieve memory queries
pub struct RetrieveMemoryQueryHandler {
    use_case: Arc<dyn StoreMemoryUseCase>,
    metrics: Arc<dyn MetricsCollector>,
}

/// Handler for analyze usage queries
pub struct AnalyzeUsageQueryHandler {
    use_case: Arc<dyn AnalyzeUsageUseCase>,
    metrics: Arc<dyn MetricsCollector>,
}

// Command Handler Implementations
impl StoreMemoryCommandHandler {
    pub fn new(
        use_case: Arc<dyn StoreMemoryUseCase>,
        metrics: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self { use_case, metrics }
    }
}

#[async_trait]
impl CommandHandler<StoreMemoryCommand> for StoreMemoryCommandHandler {
    #[instrument(skip(self, command), fields(content_length = command.content.len()))]
    async fn handle(&self, command: StoreMemoryCommand, context: RequestContext) -> ApplicationResult<crate::dtos::StoreMemoryResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Handling StoreMemoryCommand for request: {}", context.request_id);
        
        // Convert command to request DTO
        let request = crate::dtos::StoreMemoryRequest {
            content: command.content,
            metadata: command.metadata,
            project: command.project,
            target_layer: command.target_layer,
            priority: command.priority,
            tags: command.tags,
        };
        
        let result = self.use_case.store_memory(request, context.clone()).await;
        
        match &result {
            Ok(response) => {
                let duration = start_time.elapsed();
                
                // Record success metrics
                self.record_success_metrics("store_memory", duration.as_millis() as u64, &response).await?;
                
                info!(
                    "StoreMemoryCommand completed successfully: {} in {}ms",
                    response.record_id,
                    duration.as_millis()
                );
            }
            Err(e) => {
                let duration = start_time.elapsed();
                
                // Record failure metrics
                self.record_failure_metrics("store_memory", duration.as_millis() as u64, e).await?;
                
                error!("StoreMemoryCommand failed: {}", e);
            }
        }
        
        result
    }
    
    async fn before_handle(&self, command: &StoreMemoryCommand, context: &RequestContext) -> ApplicationResult<()> {
        // Pre-execution validation
        if command.content.is_empty() {
            return Err(ApplicationError::validation("Content cannot be empty"));
        }
        
        if command.content.len() > 100_000 {
            return Err(ApplicationError::validation("Content too large"));
        }
        
        Ok(())
    }
    
    async fn after_handle(&self, command: &StoreMemoryCommand, response: &crate::dtos::StoreMemoryResponse, context: &RequestContext) -> ApplicationResult<()> {
        // Post-execution actions
        info!(
            "Stored record {} with {} dimensions on layer {:?}",
            response.record_id,
            response.embedding_dimensions,
            response.layer
        );
        
        Ok(())
    }
}

impl BatchStoreMemoryCommandHandler {
    pub fn new(
        use_case: Arc<dyn StoreMemoryUseCase>,
        metrics: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self { use_case, metrics }
    }
}

#[async_trait]
impl CommandHandler<BatchStoreMemoryCommand> for BatchStoreMemoryCommandHandler {
    #[instrument(skip(self, command), fields(batch_size = command.records.len()))]
    async fn handle(&self, command: BatchStoreMemoryCommand, context: RequestContext) -> ApplicationResult<crate::dtos::BatchStoreMemoryResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Handling BatchStoreMemoryCommand with {} records", command.records.len());
        
        // Convert command to request DTO
        let request = crate::dtos::BatchStoreMemoryRequest {
            records: command.records,
            options: crate::dtos::BatchOptions {
                parallel_processing: command.parallel_processing,
                failure_tolerance: command.failure_tolerance,
                progress_reporting: command.progress_reporting,
            },
        };
        
        let result = self.use_case.store_batch_memory(request, context.clone()).await;
        
        match &result {
            Ok(response) => {
                let duration = start_time.elapsed();
                
                // Record batch metrics
                self.record_batch_metrics(response, duration.as_millis() as u64).await?;
                
                info!(
                    "BatchStoreMemoryCommand completed: {}/{} successful in {}ms",
                    response.successful,
                    response.total_requested,
                    duration.as_millis()
                );
            }
            Err(e) => {
                error!("BatchStoreMemoryCommand failed: {}", e);
            }
        }
        
        result
    }
}

impl PromoteRecordsCommandHandler {
    pub fn new(
        use_case: Arc<dyn PromoteRecordsUseCase>,
        metrics: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self { use_case, metrics }
    }
}

#[async_trait]
impl CommandHandler<PromoteRecordsCommand> for PromoteRecordsCommandHandler {
    #[instrument(skip(self, command), fields(dry_run = command.dry_run))]
    async fn handle(&self, command: PromoteRecordsCommand, context: RequestContext) -> ApplicationResult<crate::dtos::PromoteRecordsResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Handling PromoteRecordsCommand (dry_run: {})", command.dry_run);
        
        // Convert command to request DTO
        let request = crate::dtos::PromoteRecordsRequest {
            criteria: command.criteria,
            max_candidates: command.max_candidates,
            dry_run: command.dry_run,
        };
        
        let result = self.use_case.promote_records(request, context.clone()).await;
        
        match &result {
            Ok(response) => {
                let duration = start_time.elapsed();
                
                info!(
                    "PromoteRecordsCommand completed: {} records promoted in {}ms",
                    response.promoted_records.len(),
                    duration.as_millis()
                );
            }
            Err(e) => {
                error!("PromoteRecordsCommand failed: {}", e);
            }
        }
        
        result
    }
}

// Query Handler Implementations
impl SearchMemoryQueryHandler {
    pub fn new(
        use_case: Arc<dyn SearchMemoryUseCase>,
        metrics: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self { use_case, metrics }
    }
}

#[async_trait]
impl QueryHandler<SearchMemoryQuery> for SearchMemoryQueryHandler {
    #[instrument(skip(self, query), fields(query_text = %query.query, limit = query.limit))]
    async fn handle(&self, query: SearchMemoryQuery, context: RequestContext) -> ApplicationResult<crate::dtos::SearchMemoryResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Handling SearchMemoryQuery: '{}'", query.query);
        
        // Convert query to request DTO
        let request = crate::dtos::SearchMemoryRequest {
            query: query.query,
            layers: query.layers,
            limit: query.limit,
            include_embeddings: query.include_embeddings,
            project_filter: query.project_filter,
            tag_filters: query.tag_filters,
        };
        
        let result = self.use_case.search_memory(request, context.clone()).await;
        
        match &result {
            Ok(response) => {
                let duration = start_time.elapsed();
                
                info!(
                    "SearchMemoryQuery completed: {} results in {}ms",
                    response.results.len(),
                    duration.as_millis()
                );
                
                // Record query metrics
                self.record_query_metrics("search", duration.as_millis() as u64, response.results.len()).await?;
            }
            Err(e) => {
                error!("SearchMemoryQuery failed: {}", e);
            }
        }
        
        result
    }
}

impl SimilaritySearchQueryHandler {
    pub fn new(
        use_case: Arc<dyn SearchMemoryUseCase>,
        metrics: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self { use_case, metrics }
    }
}

#[async_trait]
impl QueryHandler<SimilaritySearchQuery> for SimilaritySearchQueryHandler {
    #[instrument(skip(self, query), fields(embedding_dims = query.query_embedding.len(), limit = query.limit))]
    async fn handle(&self, query: SimilaritySearchQuery, context: RequestContext) -> ApplicationResult<crate::dtos::SimilaritySearchResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Handling SimilaritySearchQuery with {} dimensions", query.query_embedding.len());
        
        // Convert query to request DTO
        let request = crate::dtos::SimilaritySearchRequest {
            query_embedding: query.query_embedding,
            limit: query.limit,
            threshold: query.threshold,
            layers: query.layers,
            include_vectors: query.include_vectors,
        };
        
        let result = self.use_case.similarity_search(request, context.clone()).await;
        
        match &result {
            Ok(response) => {
                let duration = start_time.elapsed();
                
                info!(
                    "SimilaritySearchQuery completed: {} results in {}ms",
                    response.results.len(),
                    duration.as_millis()
                );
                
                // Record query metrics
                self.record_query_metrics("similarity_search", duration.as_millis() as u64, response.results.len()).await?;
            }
            Err(e) => {
                error!("SimilaritySearchQuery failed: {}", e);
            }
        }
        
        result
    }
}

impl RetrieveMemoryQueryHandler {
    pub fn new(
        use_case: Arc<dyn StoreMemoryUseCase>,
        metrics: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self { use_case, metrics }
    }
}

#[async_trait]
impl QueryHandler<RetrieveMemoryQuery> for RetrieveMemoryQueryHandler {
    #[instrument(skip(self, query), fields(record_id = %query.record_id))]
    async fn handle(&self, query: RetrieveMemoryQuery, context: RequestContext) -> ApplicationResult<crate::dtos::RetrieveMemoryResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Handling RetrieveMemoryQuery for record: {}", query.record_id);
        
        // Convert query to request DTO
        let request = crate::dtos::RetrieveMemoryRequest {
            record_id: query.record_id,
            include_embedding: query.include_embedding,
            include_stats: query.include_stats,
        };
        
        let result = self.use_case.retrieve_memory(request, context.clone()).await;
        
        match &result {
            Ok(response) => {
                let duration = start_time.elapsed();
                
                info!(
                    "RetrieveMemoryQuery completed for record {} in {}ms",
                    response.record_id,
                    duration.as_millis()
                );
                
                // Record query metrics
                self.record_query_metrics("retrieve", duration.as_millis() as u64, 1).await?;
            }
            Err(e) => {
                error!("RetrieveMemoryQuery failed: {}", e);
            }
        }
        
        result
    }
}

impl AnalyzeUsageQueryHandler {
    pub fn new(
        use_case: Arc<dyn AnalyzeUsageUseCase>,
        metrics: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self { use_case, metrics }
    }
}

#[async_trait]
impl QueryHandler<AnalyzeUsageQuery> for AnalyzeUsageQueryHandler {
    #[instrument(skip(self, query), fields(time_window = query.time_window_hours))]
    async fn handle(&self, query: AnalyzeUsageQuery, context: RequestContext) -> ApplicationResult<crate::dtos::AnalyzeUsageResponse> {
        let start_time = std::time::Instant::now();
        
        info!("Handling AnalyzeUsageQuery");
        
        // Convert query to request DTO
        let request = crate::dtos::AnalyzeUsageRequest {
            time_window_hours: query.time_window_hours,
            layers: query.layers,
            project_filter: query.project_filter,
        };
        
        let result = self.use_case.analyze_usage_patterns(request, context.clone()).await;
        
        match &result {
            Ok(response) => {
                let duration = start_time.elapsed();
                
                info!(
                    "AnalyzeUsageQuery completed: {} patterns found in {}ms",
                    response.patterns.len(),
                    duration.as_millis()
                );
            }
            Err(e) => {
                error!("AnalyzeUsageQuery failed: {}", e);
            }
        }
        
        result
    }
}

impl StoreMemoryCommandHandler {
    async fn record_success_metrics(&self, operation: &str, duration_ms: u64, response: &crate::dtos::StoreMemoryResponse) -> ApplicationResult<()> {
        let mut tags = std::collections::HashMap::new();
        tags.insert("operation".to_string(), operation.to_string());
        tags.insert("layer".to_string(), format!("{:?}", response.layer));
        
        self.metrics.increment_counter("command_executions_successful", 1, Some(&tags)).await?;
        self.metrics.record_timing("command_duration", duration_ms, Some(&tags)).await?;
        self.metrics.record_gauge("embedding_dimensions", response.embedding_dimensions as f64, Some(&tags)).await?;
        
        Ok(())
    }
    
    async fn record_failure_metrics(&self, operation: &str, duration_ms: u64, error: &ApplicationError) -> ApplicationResult<()> {
        let mut tags = std::collections::HashMap::new();
        tags.insert("operation".to_string(), operation.to_string());
        tags.insert("error_type".to_string(), format!("{:?}", error));
        
        self.metrics.increment_counter("command_executions_failed", 1, Some(&tags)).await?;
        self.metrics.record_timing("command_duration", duration_ms, Some(&tags)).await?;
        
        Ok(())
    }
}

impl BatchStoreMemoryCommandHandler {
    async fn record_batch_metrics(&self, response: &crate::dtos::BatchStoreMemoryResponse, duration_ms: u64) -> ApplicationResult<()> {
        let success_rate = response.successful as f64 / response.total_requested as f64;
        
        let mut tags = std::collections::HashMap::new();
        tags.insert("operation".to_string(), "batch_store".to_string());
        
        self.metrics.increment_counter("batch_operations", 1, Some(&tags)).await?;
        self.metrics.record_gauge("batch_success_rate", success_rate, Some(&tags)).await?;
        self.metrics.record_gauge("batch_size", response.total_requested as f64, Some(&tags)).await?;
        self.metrics.record_timing("batch_duration", duration_ms, Some(&tags)).await?;
        
        Ok(())
    }
}

impl SearchMemoryQueryHandler {
    async fn record_query_metrics(&self, query_type: &str, duration_ms: u64, result_count: usize) -> ApplicationResult<()> {
        let mut tags = std::collections::HashMap::new();
        tags.insert("query_type".to_string(), query_type.to_string());
        
        self.metrics.increment_counter("query_executions", 1, Some(&tags)).await?;
        self.metrics.record_timing("query_duration", duration_ms, Some(&tags)).await?;
        self.metrics.record_gauge("query_result_count", result_count as f64, Some(&tags)).await?;
        
        Ok(())
    }
}

impl SimilaritySearchQueryHandler {
    async fn record_query_metrics(&self, query_type: &str, duration_ms: u64, result_count: usize) -> ApplicationResult<()> {
        let mut tags = std::collections::HashMap::new();
        tags.insert("query_type".to_string(), query_type.to_string());
        
        self.metrics.increment_counter("query_executions", 1, Some(&tags)).await?;
        self.metrics.record_timing("query_duration", duration_ms, Some(&tags)).await?;
        self.metrics.record_gauge("query_result_count", result_count as f64, Some(&tags)).await?;
        
        Ok(())
    }
}

impl RetrieveMemoryQueryHandler {
    async fn record_query_metrics(&self, query_type: &str, duration_ms: u64, result_count: usize) -> ApplicationResult<()> {
        let mut tags = std::collections::HashMap::new();
        tags.insert("query_type".to_string(), query_type.to_string());
        
        self.metrics.increment_counter("query_executions", 1, Some(&tags)).await?;
        self.metrics.record_timing("query_duration", duration_ms, Some(&tags)).await?;
        self.metrics.record_gauge("query_result_count", result_count as f64, Some(&tags)).await?;
        
        Ok(())
    }
}

/// Handler registry for managing all CQRS handlers
pub struct HandlerRegistry {
    // Command handlers
    store_memory_handler: Arc<StoreMemoryCommandHandler>,
    batch_store_handler: Arc<BatchStoreMemoryCommandHandler>,
    promote_records_handler: Arc<PromoteRecordsCommandHandler>,
    
    // Query handlers
    search_memory_handler: Arc<SearchMemoryQueryHandler>,
    similarity_search_handler: Arc<SimilaritySearchQueryHandler>,
    retrieve_memory_handler: Arc<RetrieveMemoryQueryHandler>,
    analyze_usage_handler: Arc<AnalyzeUsageQueryHandler>,
}

impl HandlerRegistry {
    pub fn new(
        store_use_case: Arc<dyn StoreMemoryUseCase>,
        search_use_case: Arc<dyn SearchMemoryUseCase>,
        promotion_use_case: Arc<dyn PromoteRecordsUseCase>,
        analytics_use_case: Arc<dyn AnalyzeUsageUseCase>,
        metrics: Arc<dyn MetricsCollector>,
    ) -> Self {
        Self {
            store_memory_handler: Arc::new(StoreMemoryCommandHandler::new(store_use_case.clone(), metrics.clone())),
            batch_store_handler: Arc::new(BatchStoreMemoryCommandHandler::new(store_use_case.clone(), metrics.clone())),
            promote_records_handler: Arc::new(PromoteRecordsCommandHandler::new(promotion_use_case, metrics.clone())),
            search_memory_handler: Arc::new(SearchMemoryQueryHandler::new(search_use_case.clone(), metrics.clone())),
            similarity_search_handler: Arc::new(SimilaritySearchQueryHandler::new(search_use_case.clone(), metrics.clone())),
            retrieve_memory_handler: Arc::new(RetrieveMemoryQueryHandler::new(store_use_case, metrics.clone())),
            analyze_usage_handler: Arc::new(AnalyzeUsageQueryHandler::new(analytics_use_case, metrics)),
        }
    }
    
    /// Register all handlers with the CQRS bus
    pub fn register_all_handlers(&self, bus: &mut super::CqrsBus) {
        // Register command handlers
        bus.register_command_handler(self.store_memory_handler.as_ref().clone());
        bus.register_command_handler(self.batch_store_handler.as_ref().clone());
        bus.register_command_handler(self.promote_records_handler.as_ref().clone());
        
        // Register query handlers
        bus.register_query_handler(self.search_memory_handler.as_ref().clone());
        bus.register_query_handler(self.similarity_search_handler.as_ref().clone());
        bus.register_query_handler(self.retrieve_memory_handler.as_ref().clone());
        bus.register_query_handler(self.analyze_usage_handler.as_ref().clone());
    }
}