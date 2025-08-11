//! CQRS (Command Query Responsibility Segregation) Implementation
//!
//! Разделяет команды (изменяющие состояние) и запросы (чтение данных)
//! для лучшей масштабируемости и производительности.

pub mod commands;
pub mod queries;
pub mod handlers;

// Re-export CQRS components
pub use commands::*;
pub use queries::*;
pub use handlers::*;

use async_trait::async_trait;
use crate::{ApplicationResult, RequestContext};
use std::sync::Arc;
use tracing::{info, instrument};

/// Base trait for all commands (modify state)
#[async_trait]
pub trait Command: Send + Sync {
    type Response: Send + Sync;
    
    /// Execute command and return response
    async fn execute(&self, context: RequestContext) -> ApplicationResult<Self::Response>;
    
    /// Validate command before execution
    fn validate(&self) -> ApplicationResult<()> {
        Ok(()) // Default implementation
    }
    
    /// Get command identifier for logging/metrics
    fn command_id(&self) -> &'static str;
    
    /// Get command version for compatibility
    fn version(&self) -> u32 {
        1
    }
}

/// Base trait for all queries (read data)
#[async_trait]
pub trait Query: Send + Sync {
    type Response: Send + Sync;
    
    /// Execute query and return response
    async fn execute(&self, context: RequestContext) -> ApplicationResult<Self::Response>;
    
    /// Validate query parameters
    fn validate(&self) -> ApplicationResult<()> {
        Ok(()) // Default implementation
    }
    
    /// Get query identifier for logging/metrics
    fn query_id(&self) -> &'static str;
    
    /// Check if query can be cached
    fn is_cacheable(&self) -> bool {
        false // Default: not cacheable
    }
    
    /// Get cache key for cacheable queries
    fn cache_key(&self) -> Option<String> {
        None
    }
}

/// Command handler trait
#[async_trait]
pub trait CommandHandler<C: Command>: Send + Sync {
    /// Handle command execution
    async fn handle(&self, command: C, context: RequestContext) -> ApplicationResult<C::Response>;
    
    /// Pre-execution hook
    async fn before_handle(&self, command: &C, context: &RequestContext) -> ApplicationResult<()> {
        Ok(())
    }
    
    /// Post-execution hook
    async fn after_handle(&self, command: &C, response: &C::Response, context: &RequestContext) -> ApplicationResult<()> {
        Ok(())
    }
}

/// Query handler trait
#[async_trait]
pub trait QueryHandler<Q: Query>: Send + Sync {
    /// Handle query execution
    async fn handle(&self, query: Q, context: RequestContext) -> ApplicationResult<Q::Response>;
    
    /// Pre-execution hook
    async fn before_handle(&self, query: &Q, context: &RequestContext) -> ApplicationResult<()> {
        Ok(())
    }
    
    /// Post-execution hook
    async fn after_handle(&self, query: &Q, response: &Q::Response, context: &RequestContext) -> ApplicationResult<()> {
        Ok(())
    }
}

/// CQRS Bus for routing commands and queries
pub struct CqrsBus {
    command_handlers: std::collections::HashMap<&'static str, Box<dyn std::any::Any + Send + Sync>>,
    query_handlers: std::collections::HashMap<&'static str, Box<dyn std::any::Any + Send + Sync>>,
    metrics: Arc<crate::ports::MetricsCollector>,
}

impl CqrsBus {
    pub fn new(metrics: Arc<crate::ports::MetricsCollector>) -> Self {
        Self {
            command_handlers: std::collections::HashMap::new(),
            query_handlers: std::collections::HashMap::new(),
            metrics,
        }
    }
    
    /// Register command handler
    pub fn register_command_handler<C: Command + 'static, H: CommandHandler<C> + 'static>(&mut self, handler: H) {
        let command_id = std::any::type_name::<C>();
        self.command_handlers.insert(command_id, Box::new(Arc::new(handler)));
    }
    
    /// Register query handler  
    pub fn register_query_handler<Q: Query + 'static, H: QueryHandler<Q> + 'static>(&mut self, handler: H) {
        let query_id = std::any::type_name::<Q>();
        self.query_handlers.insert(query_id, Box::new(Arc::new(handler)));
    }
    
    /// Send command for execution
    #[instrument(skip(self, command, context), fields(command_id = command.command_id()))]
    pub async fn send_command<C: Command + 'static>(&self, command: C, context: RequestContext) -> ApplicationResult<C::Response> {
        let start_time = std::time::Instant::now();
        let command_id = command.command_id();
        
        info!("Executing command: {}", command_id);
        
        // Validate command
        command.validate()?;
        
        // Find handler
        let type_id = std::any::type_name::<C>();
        let handler_any = self.command_handlers.get(type_id)
            .ok_or_else(|| crate::ApplicationError::infrastructure(format!("No handler registered for command: {}", command_id)))?;
        
        let handler = handler_any.downcast_ref::<Arc<dyn CommandHandler<C>>>()
            .ok_or_else(|| crate::ApplicationError::infrastructure(format!("Handler type mismatch for command: {}", command_id)))?;
        
        // Execute command
        let result = handler.handle(command, context.clone()).await;
        
        // Record metrics
        let duration = start_time.elapsed();
        self.record_command_metrics(command_id, duration.as_millis() as u64, result.is_ok()).await?;
        
        result
    }
    
    /// Send query for execution
    #[instrument(skip(self, query, context), fields(query_id = query.query_id()))]
    pub async fn send_query<Q: Query + 'static>(&self, query: Q, context: RequestContext) -> ApplicationResult<Q::Response> {
        let start_time = std::time::Instant::now();
        let query_id = query.query_id();
        
        info!("Executing query: {}", query_id);
        
        // Validate query
        query.validate()?;
        
        if query.is_cacheable() {
            if let Some(cache_key) = query.cache_key() {
                // TODO: Implement caching logic
            }
        }
        
        // Find handler
        let type_id = std::any::type_name::<Q>();
        let handler_any = self.query_handlers.get(type_id)
            .ok_or_else(|| crate::ApplicationError::infrastructure(format!("No handler registered for query: {}", query_id)))?;
        
        let handler = handler_any.downcast_ref::<Arc<dyn QueryHandler<Q>>>()
            .ok_or_else(|| crate::ApplicationError::infrastructure(format!("Handler type mismatch for query: {}", query_id)))?;
        
        // Execute query
        let result = handler.handle(query, context.clone()).await;
        
        // Record metrics
        let duration = start_time.elapsed();
        self.record_query_metrics(query_id, duration.as_millis() as u64, result.is_ok()).await?;
        
        result
    }
    
    async fn record_command_metrics(&self, command_id: &str, duration_ms: u64, success: bool) -> ApplicationResult<()> {
        let mut tags = std::collections::HashMap::new();
        tags.insert("command_id".to_string(), command_id.to_string());
        tags.insert("success".to_string(), success.to_string());
        
        self.metrics.increment_counter("cqrs_commands_total", 1, Some(&tags)).await?;
        self.metrics.record_timing("cqrs_command_duration", duration_ms, Some(&tags)).await?;
        
        Ok(())
    }
    
    async fn record_query_metrics(&self, query_id: &str, duration_ms: u64, success: bool) -> ApplicationResult<()> {
        let mut tags = std::collections::HashMap::new();
        tags.insert("query_id".to_string(), query_id.to_string());
        tags.insert("success".to_string(), success.to_string());
        
        self.metrics.increment_counter("cqrs_queries_total", 1, Some(&tags)).await?;
        self.metrics.record_timing("cqrs_query_duration", duration_ms, Some(&tags)).await?;
        
        Ok(())
    }
}

/// Builder for CQRS Bus
pub struct CqrsBusBuilder {
    metrics: Arc<crate::ports::MetricsCollector>,
}

impl CqrsBusBuilder {
    pub fn new(metrics: Arc<crate::ports::MetricsCollector>) -> Self {
        Self { metrics }
    }
    
    pub fn build(self) -> CqrsBus {
        CqrsBus::new(self.metrics)
    }
    
    pub fn with_memory_handlers(mut self) -> Self {
        // Register memory-related handlers
        self
    }
    
    pub fn with_search_handlers(mut self) -> Self {
        // Register search-related handlers
        self
    }
    
    pub fn with_analytics_handlers(mut self) -> Self {
        // Register analytics-related handlers
        self
    }
}

/// Macro to generate command/query implementations
#[macro_export]
macro_rules! impl_cqrs {
    (command $name:ident => $response:ty) => {
        #[async_trait::async_trait]
        impl crate::cqrs::Command for $name {
            type Response = $response;
            
            async fn execute(&self, context: crate::RequestContext) -> crate::ApplicationResult<Self::Response> {
                // Default implementation delegates to handler
                unimplemented!("Commands should be executed through CqrsBus")
            }
            
            fn command_id(&self) -> &'static str {
                stringify!($name)
            }
        }
    };
    
    (query $name:ident => $response:ty) => {
        #[async_trait::async_trait]
        impl crate::cqrs::Query for $name {
            type Response = $response;
            
            async fn execute(&self, context: crate::RequestContext) -> crate::ApplicationResult<Self::Response> {
                // Default implementation delegates to handler
                unimplemented!("Queries should be executed through CqrsBus")
            }
            
            fn query_id(&self) -> &'static str {
                stringify!($name)
            }
        }
    };
}