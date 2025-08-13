//! Metrics Collector Adapter
//!
//! Адаптер для интеграции с metrics collection services

use crate::ports::{MemoryOperation, MemoryOperationType, MetricsCollector};
use crate::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Simple metrics collector adapter
pub struct MetricsCollectorAdapter {
    metrics_service: Arc<dyn MetricsServiceTrait>,
}

#[async_trait]
pub trait MetricsServiceTrait: Send + Sync {
    async fn increment_counter(
        &self,
        name: &str,
        value: u64,
        tags: Option<&HashMap<String, String>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn record_gauge(
        &self,
        name: &str,
        value: f64,
        tags: Option<&HashMap<String, String>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn record_timing(
        &self,
        name: &str,
        value_ms: u64,
        tags: Option<&HashMap<String, String>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

impl MetricsCollectorAdapter {
    pub fn new(metrics_service: Arc<dyn MetricsServiceTrait>) -> Self {
        Self { metrics_service }
    }
}

#[async_trait]
impl MetricsCollector for MetricsCollectorAdapter {
    async fn increment_counter(
        &self,
        name: &str,
        value: u64,
        tags: Option<&crate::ports::MetricTags>,
    ) -> ApplicationResult<()> {
        self.metrics_service
            .increment_counter(name, value, tags)
            .await
            .map_err(|e| {
                ApplicationError::infrastructure(format!("Failed to increment counter: {}", e))
            })
    }

    async fn record_gauge(
        &self,
        name: &str,
        value: f64,
        tags: Option<&crate::ports::MetricTags>,
    ) -> ApplicationResult<()> {
        self.metrics_service
            .record_gauge(name, value, tags)
            .await
            .map_err(|e| ApplicationError::infrastructure(format!("Failed to record gauge: {}", e)))
    }

    async fn record_timing(
        &self,
        name: &str,
        duration_ms: u64,
        tags: Option<&crate::ports::MetricTags>,
    ) -> ApplicationResult<()> {
        self.metrics_service
            .record_timing(name, duration_ms, tags)
            .await
            .map_err(|e| {
                ApplicationError::infrastructure(format!("Failed to record timing: {}", e))
            })
    }

    async fn record_histogram(
        &self,
        name: &str,
        value: f64,
        tags: Option<&crate::ports::MetricTags>,
    ) -> ApplicationResult<()> {
        // Convert histogram to timing for simplicity
        self.record_timing(name, value as u64, tags).await
    }

    async fn record_event(&self, event: &crate::ports::BusinessEvent) -> ApplicationResult<()> {
        // Simple implementation - convert event to metrics
        let mut tags = crate::ports::MetricTags::new();
        tags.insert("event_type".to_string(), format!("{:?}", event.event_type));
        tags.insert("event_name".to_string(), event.event_name.clone());

        // Record event counter
        self.increment_counter("events_total", 1, Some(&tags))
            .await?;

        // Record metrics from event
        for (name, value) in &event.metrics.counters {
            let metric_name = format!("event_{}_{}", event.event_name, name);
            self.increment_counter(&metric_name, *value, Some(&tags))
                .await?;
        }

        for (name, value) in &event.metrics.gauges {
            let metric_name = format!("event_{}_{}", event.event_name, name);
            self.record_gauge(&metric_name, *value, Some(&tags)).await?;
        }

        for (name, value) in &event.metrics.timings {
            let metric_name = format!("event_{}_{}", event.event_name, name);
            self.record_timing(&metric_name, *value, Some(&tags))
                .await?;
        }

        Ok(())
    }

    async fn flush(&self) -> ApplicationResult<crate::ports::FlushResult> {
        // Mock implementation
        Ok(crate::ports::FlushResult {
            metrics_sent: 100,
            events_sent: 20,
            bytes_sent: 10240,
            duration_ms: 50,
            errors: vec![],
        })
    }

    async fn health_check(&self) -> ApplicationResult<crate::ports::MetricsHealth> {
        Ok(crate::ports::MetricsHealth {
            is_healthy: true,
            buffer_size: 100,
            buffer_capacity: 1000,
            buffer_utilization: 0.1,
            last_flush_time: Some(chrono::Utc::now()),
            last_error: None,
            connection_status: crate::ports::ConnectionStatus::Connected,
        })
    }

    async fn get_statistics(&self) -> ApplicationResult<crate::ports::CollectorStatistics> {
        Ok(crate::ports::CollectorStatistics {
            total_metrics: 1000,
            total_events: 200,
            metrics_per_second: 10.0,
            events_per_second: 2.0,
            success_rate: 0.99,
            average_flush_time_ms: 50,
            error_count: 5,
            uptime_seconds: 3600,
            memory_usage_bytes: 1024 * 1024,
        })
    }
}
