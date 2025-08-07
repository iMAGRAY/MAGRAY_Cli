//! Metrics Collector Port
//!
//! Абстракция для сбора и отправки метрик производительности и бизнес-событий.

use async_trait::async_trait;
use crate::ApplicationResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trait для metrics collection services
#[async_trait]
pub trait MetricsCollector: Send + Sync {
    /// Record a counter metric
    async fn increment_counter(&self, name: &str, value: u64, tags: Option<&MetricTags>) -> ApplicationResult<()>;
    
    /// Record a gauge metric
    async fn record_gauge(&self, name: &str, value: f64, tags: Option<&MetricTags>) -> ApplicationResult<()>;
    
    /// Record a histogram metric
    async fn record_histogram(&self, name: &str, value: f64, tags: Option<&MetricTags>) -> ApplicationResult<()>;
    
    /// Record timing information
    async fn record_timing(&self, name: &str, duration_ms: u64, tags: Option<&MetricTags>) -> ApplicationResult<()>;
    
    /// Record custom business event
    async fn record_event(&self, event: &BusinessEvent) -> ApplicationResult<()>;
    
    /// Flush all pending metrics
    async fn flush(&self) -> ApplicationResult<FlushResult>;
    
    /// Get metrics collector health
    async fn health_check(&self) -> ApplicationResult<MetricsHealth>;
    
    /// Get collector statistics
    async fn get_statistics(&self) -> ApplicationResult<CollectorStatistics>;
}

/// Metric tags for dimensions
pub type MetricTags = HashMap<String, String>;

/// Business event for tracking domain-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessEvent {
    pub event_type: EventType,
    pub event_name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub properties: EventProperties,
    pub metrics: EventMetrics,
    pub context: EventContext,
}

/// Types of business events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    /// Memory operations
    Memory,
    /// Search operations  
    Search,
    /// ML promotion events
    Promotion,
    /// User interactions
    UserAction,
    /// System events
    System,
    /// Performance events
    Performance,
    /// Error events
    Error,
    /// Custom event types
    Custom(String),
}

/// Event properties (string key-value pairs)
pub type EventProperties = HashMap<String, String>;

/// Event metrics (numeric measurements)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetrics {
    pub counters: HashMap<String, u64>,
    pub gauges: HashMap<String, f64>,
    pub timings: HashMap<String, u64>, // milliseconds
}

/// Event context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContext {
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub request_id: Option<String>,
    pub correlation_id: Option<String>,
    pub source: String,
    pub version: String,
    pub environment: String,
    pub additional_context: HashMap<String, serde_json::Value>,
}

/// Result of flush operation
#[derive(Debug, Clone)]
pub struct FlushResult {
    pub metrics_sent: usize,
    pub events_sent: usize,
    pub bytes_sent: usize,
    pub duration_ms: u64,
    pub errors: Vec<String>,
}

/// Metrics collector health status
#[derive(Debug, Clone)]
pub struct MetricsHealth {
    pub is_healthy: bool,
    pub buffer_size: usize,
    pub buffer_capacity: usize,
    pub buffer_utilization: f32,
    pub last_flush_time: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error: Option<String>,
    pub connection_status: ConnectionStatus,
}

/// Connection status to metrics backend
#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
    Error(String),
}

/// Collector statistics
#[derive(Debug, Clone)]
pub struct CollectorStatistics {
    pub total_metrics: u64,
    pub total_events: u64,
    pub metrics_per_second: f64,
    pub events_per_second: f64,
    pub success_rate: f32,
    pub average_flush_time_ms: u64,
    pub error_count: u64,
    pub uptime_seconds: u64,
    pub memory_usage_bytes: u64,
}

/// Convenience methods for common metrics patterns
pub trait MetricsCollectorExt: MetricsCollector {
    /// Record operation timing with automatic tags
    fn time_operation<F, T>(&self, operation_name: &str, f: F) -> impl std::future::Future<Output = ApplicationResult<T>>
    where
        F: std::future::Future<Output = ApplicationResult<T>>;
        
    /// Record memory operation metrics
    fn record_memory_operation(&self, operation: MemoryOperation) -> impl std::future::Future<Output = ApplicationResult<()>>;
    
    /// Record search metrics
    fn record_search_metrics(&self, metrics: SearchMetrics) -> impl std::future::Future<Output = ApplicationResult<()>>;
    
    /// Record promotion metrics
    fn record_promotion_metrics(&self, metrics: PromotionMetrics) -> impl std::future::Future<Output = ApplicationResult<()>>;
}

/// Memory operation metrics
#[derive(Debug, Clone)]
pub struct MemoryOperation {
    pub operation_type: MemoryOperationType,
    pub layer: String,
    pub record_count: usize,
    pub processing_time_ms: u64,
    pub bytes_processed: usize,
    pub success: bool,
    pub error: Option<String>,
}

/// Types of memory operations
#[derive(Debug, Clone)]
pub enum MemoryOperationType {
    Store,
    Retrieve,
    Search,
    Delete,
    Promote,
    Backup,
    Analyze,
}

/// Search operation metrics
#[derive(Debug, Clone)]
pub struct SearchMetrics {
    pub query_text: String,
    pub results_count: usize,
    pub search_time_ms: u64,
    pub layers_searched: Vec<String>,
    pub cache_hit: bool,
    pub reranked: bool,
    pub user_satisfaction: Option<f32>,
}

/// Promotion operation metrics
#[derive(Debug, Clone)]
pub struct PromotionMetrics {
    pub algorithm: String,
    pub candidates_analyzed: usize,
    pub records_promoted: usize,
    pub analysis_time_ms: u64,
    pub ml_accuracy: f32,
    pub performance_improvement: f32,
}

/// Default implementation of extended metrics methods
impl<T: MetricsCollector + ?Sized> MetricsCollectorExt for T {
    async fn time_operation<F, Fut>(&self, operation_name: &str, f: F) -> ApplicationResult<Fut::Output>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future,
    {
        let start_time = std::time::Instant::now();
        let result = f().await;
        let duration = start_time.elapsed();
        
        let _ = self.record_timing(
            operation_name, 
            duration.as_millis() as u64, 
            None
        ).await;
        
        Ok(result)
    }
    
    async fn record_memory_operation(&self, operation: MemoryOperation) -> ApplicationResult<()> {
        let mut tags = MetricTags::new();
        tags.insert("operation".to_string(), format!("{:?}", operation.operation_type).to_lowercase());
        tags.insert("layer".to_string(), operation.layer.clone());
        tags.insert("success".to_string(), operation.success.to_string());
        
        // Record counter
        self.increment_counter("memory_operations_total", 1, Some(&tags)).await?;
        
        // Record timing
        self.record_timing("memory_operation_duration", operation.processing_time_ms, Some(&tags)).await?;
        
        // Record processed data size
        self.record_gauge("memory_bytes_processed", operation.bytes_processed as f64, Some(&tags)).await?;
        
        // Record business event
        let event = BusinessEvent {
            event_type: EventType::Memory,
            event_name: "memory_operation".to_string(),
            timestamp: chrono::Utc::now(),
            properties: [
                ("operation_type".to_string(), format!("{:?}", operation.operation_type)),
                ("layer".to_string(), operation.layer),
                ("success".to_string(), operation.success.to_string()),
            ].into_iter().collect(),
            metrics: EventMetrics {
                counters: [("record_count".to_string(), operation.record_count as u64)].into_iter().collect(),
                gauges: [("bytes_processed".to_string(), operation.bytes_processed as f64)].into_iter().collect(),
                timings: [("processing_time_ms".to_string(), operation.processing_time_ms)].into_iter().collect(),
            },
            context: EventContext::default(),
        };
        
        self.record_event(&event).await
    }
    
    async fn record_search_metrics(&self, metrics: SearchMetrics) -> ApplicationResult<()> {
        let mut tags = MetricTags::new();
        tags.insert("cache_hit".to_string(), metrics.cache_hit.to_string());
        tags.insert("reranked".to_string(), metrics.reranked.to_string());
        
        // Record search counter
        self.increment_counter("search_operations_total", 1, Some(&tags)).await?;
        
        // Record search timing
        self.record_timing("search_duration", metrics.search_time_ms, Some(&tags)).await?;
        
        // Record results count
        self.record_gauge("search_results_count", metrics.results_count as f64, Some(&tags)).await?;
        
        // Record business event
        let event = BusinessEvent {
            event_type: EventType::Search,
            event_name: "search_operation".to_string(),
            timestamp: chrono::Utc::now(),
            properties: [
                ("cache_hit".to_string(), metrics.cache_hit.to_string()),
                ("reranked".to_string(), metrics.reranked.to_string()),
                ("layers_searched".to_string(), metrics.layers_searched.join(",")),
            ].into_iter().collect(),
            metrics: EventMetrics {
                counters: [("results_count".to_string(), metrics.results_count as u64)].into_iter().collect(),
                gauges: metrics.user_satisfaction
                    .map(|s| [("user_satisfaction".to_string(), s as f64)].into_iter().collect())
                    .unwrap_or_default(),
                timings: [("search_time_ms".to_string(), metrics.search_time_ms)].into_iter().collect(),
            },
            context: EventContext::default(),
        };
        
        self.record_event(&event).await
    }
    
    async fn record_promotion_metrics(&self, metrics: PromotionMetrics) -> ApplicationResult<()> {
        let mut tags = MetricTags::new();
        tags.insert("algorithm".to_string(), metrics.algorithm.clone());
        
        // Record promotion counter
        self.increment_counter("promotion_operations_total", 1, Some(&tags)).await?;
        
        // Record timing
        self.record_timing("promotion_analysis_duration", metrics.analysis_time_ms, Some(&tags)).await?;
        
        // Record ML accuracy
        self.record_gauge("promotion_ml_accuracy", metrics.ml_accuracy as f64, Some(&tags)).await?;
        
        // Record business event
        let event = BusinessEvent {
            event_type: EventType::Promotion,
            event_name: "promotion_operation".to_string(),
            timestamp: chrono::Utc::now(),
            properties: [
                ("algorithm".to_string(), metrics.algorithm),
            ].into_iter().collect(),
            metrics: EventMetrics {
                counters: [
                    ("candidates_analyzed".to_string(), metrics.candidates_analyzed as u64),
                    ("records_promoted".to_string(), metrics.records_promoted as u64),
                ].into_iter().collect(),
                gauges: [
                    ("ml_accuracy".to_string(), metrics.ml_accuracy as f64),
                    ("performance_improvement".to_string(), metrics.performance_improvement as f64),
                ].into_iter().collect(),
                timings: [("analysis_time_ms".to_string(), metrics.analysis_time_ms)].into_iter().collect(),
            },
            context: EventContext::default(),
        };
        
        self.record_event(&event).await
    }
}

impl BusinessEvent {
    /// Create a new business event
    pub fn new(event_type: EventType, event_name: &str) -> Self {
        Self {
            event_type,
            event_name: event_name.to_string(),
            timestamp: chrono::Utc::now(),
            properties: HashMap::new(),
            metrics: EventMetrics::default(),
            context: EventContext::default(),
        }
    }
    
    /// Add property to event
    pub fn with_property(mut self, key: &str, value: &str) -> Self {
        self.properties.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Add counter metric
    pub fn with_counter(mut self, key: &str, value: u64) -> Self {
        self.metrics.counters.insert(key.to_string(), value);
        self
    }
    
    /// Add gauge metric
    pub fn with_gauge(mut self, key: &str, value: f64) -> Self {
        self.metrics.gauges.insert(key.to_string(), value);
        self
    }
    
    /// Add timing metric
    pub fn with_timing(mut self, key: &str, value_ms: u64) -> Self {
        self.metrics.timings.insert(key.to_string(), value_ms);
        self
    }
}

impl Default for EventMetrics {
    fn default() -> Self {
        Self {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            timings: HashMap::new(),
        }
    }
}

impl Default for EventContext {
    fn default() -> Self {
        Self {
            user_id: None,
            session_id: None,
            request_id: None,
            correlation_id: None,
            source: "application".to_string(),
            version: "1.0.0".to_string(),
            environment: "development".to_string(),
            additional_context: HashMap::new(),
        }
    }
}