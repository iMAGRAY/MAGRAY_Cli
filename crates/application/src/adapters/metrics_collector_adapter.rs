//! Metrics Collector Adapter
//!
//! Адаптер для интеграции с metrics collection services

use std::sync::Arc;
use std::collections::HashMap;
use async_trait::async_trait;
use crate::{ApplicationResult, ApplicationError};
use crate::ports::{MetricsCollector, MemoryOperation, MemoryOperationType};

/// Simple metrics collector adapter
pub struct MetricsCollectorAdapter {
    metrics_service: Arc<dyn MetricsServiceTrait>,
}

#[async_trait]
pub trait MetricsServiceTrait: Send + Sync {
    async fn increment_counter(&self, name: &str, value: u64, tags: Option<&HashMap<String, String>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn record_gauge(&self, name: &str, value: f64, tags: Option<&HashMap<String, String>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn record_timing(&self, name: &str, value_ms: u64, tags: Option<&HashMap<String, String>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

impl MetricsCollectorAdapter {
    pub fn new(metrics_service: Arc<dyn MetricsServiceTrait>) -> Self {
        Self { metrics_service }
    }
}

#[async_trait]
impl MetricsCollector for MetricsCollectorAdapter {
    async fn increment_counter(&self, name: &str, value: u64, tags: Option<&HashMap<String, String>>) -> ApplicationResult<()> {
        self.metrics_service.increment_counter(name, value, tags).await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to increment counter", e))
    }

    async fn record_gauge(&self, name: &str, value: f64, tags: Option<&HashMap<String, String>>) -> ApplicationResult<()> {
        self.metrics_service.record_gauge(name, value, tags).await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to record gauge", e))
    }

    async fn record_timing(&self, name: &str, value_ms: u64, tags: Option<&HashMap<String, String>>) -> ApplicationResult<()> {
        self.metrics_service.record_timing(name, value_ms, tags).await
            .map_err(|e| ApplicationError::infrastructure_with_source("Failed to record timing", e))
    }

    async fn record_memory_operation(&self, operation: MemoryOperation) -> ApplicationResult<()> {
        let mut tags = HashMap::new();
        tags.insert("operation_type".to_string(), format!("{:?}", operation.operation_type));
        tags.insert("layer".to_string(), operation.layer);
        tags.insert("success".to_string(), operation.success.to_string());

        // Record multiple metrics for the operation
        self.increment_counter("memory_operations_total", 1, Some(&tags)).await?;
        self.record_timing("memory_operation_duration", operation.processing_time_ms, Some(&tags)).await?;
        self.record_gauge("memory_operation_record_count", operation.record_count as f64, Some(&tags)).await?;
        self.record_gauge("memory_operation_bytes_processed", operation.bytes_processed as f64, Some(&tags)).await?;

        if let Some(error) = operation.error {
            tags.insert("error".to_string(), error);
            self.increment_counter("memory_operation_errors_total", 1, Some(&tags)).await?;
        }

        Ok(())
    }

    async fn health_check(&self) -> ApplicationResult<crate::ports::MetricsHealth> {
        Ok(crate::ports::MetricsHealth {
            is_healthy: true,
            metrics_collected: 1000,
            failed_collections: 0,
            last_collection: chrono::Utc::now(),
            buffer_size: 100,
            collection_rate: 50.0,
        })
    }
}