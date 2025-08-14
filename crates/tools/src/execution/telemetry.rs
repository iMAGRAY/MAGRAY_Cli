// P1.2.10: Telemetry System for Tools Platform 2.0
// Comprehensive metrics collection, monitoring, and observability

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Telemetry event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TelemetryEvent {
    /// Tool execution started
    ToolExecutionStarted {
        tool_name: String,
        operation_id: String,
        timestamp: SystemTime,
        input_size_bytes: u64,
    },
    
    /// Tool execution completed
    ToolExecutionCompleted {
        tool_name: String,
        operation_id: String,
        timestamp: SystemTime,
        duration_ms: u64,
        success: bool,
        output_size_bytes: u64,
        error_message: Option<String>,
    },
    
    /// Resource usage measurement
    ResourceUsage {
        operation_id: String,
        timestamp: SystemTime,
        memory_mb: f64,
        cpu_percent: f64,
        disk_io_bytes: u64,
        network_io_bytes: u64,
    },
    
    /// Security event
    SecurityEvent {
        event_type: SecurityEventType,
        operation_id: String,
        timestamp: SystemTime,
        severity: SecuritySeverity,
        details: HashMap<String, String>,
    },
    
    /// Performance metrics
    PerformanceMetric {
        metric_name: String,
        operation_id: String,
        timestamp: SystemTime,
        value: f64,
        unit: String,
        tags: HashMap<String, String>,
    },
    
    /// Error event
    ErrorEvent {
        operation_id: String,
        timestamp: SystemTime,
        error_type: String,
        error_message: String,
        stack_trace: Option<String>,
        context: HashMap<String, String>,
    },
    
    /// System health check
    HealthCheck {
        timestamp: SystemTime,
        component: String,
        status: HealthStatus,
        latency_ms: Option<u64>,
        details: HashMap<String, String>,
    },
}

/// Security event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    CapabilityViolation,
    SandboxEscape,
    UnauthorizedAccess,
    SuspiciousActivity,
    PolicyViolation,
}

/// Security severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Health status for components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Degraded,
    Unhealthy,
}

/// Telemetry metrics aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: SystemTime,
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub average_duration_ms: f64,
    pub peak_memory_mb: f64,
    pub average_cpu_percent: f64,
    pub total_errors: u64,
    pub security_events: u64,
    pub tools_usage: HashMap<String, ToolUsageMetrics>,
}

/// Per-tool usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_duration_ms: f64,
    pub total_input_bytes: u64,
    pub total_output_bytes: u64,
    pub last_used: SystemTime,
}

/// Telemetry configuration
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Enable telemetry collection
    pub enabled: bool,
    /// Buffer size for events before flushing
    pub buffer_size: usize,
    /// Flush interval for buffered events
    pub flush_interval: Duration,
    /// Maximum retention period for events
    pub retention_period: Duration,
    /// Export events to external systems
    pub export_enabled: bool,
    /// Export endpoint URL
    pub export_endpoint: Option<String>,
    /// Sampling rate (0.0 to 1.0) for events
    pub sampling_rate: f64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_size: 1000,
            flush_interval: Duration::from_secs(60),
            retention_period: Duration::from_secs(86400), // 24 hours
            export_enabled: false,
            export_endpoint: None,
            sampling_rate: 1.0, // Collect all events by default
        }
    }
}

/// Main telemetry collector
pub struct TelemetryCollector {
    config: TelemetryConfig,
    events: Arc<Mutex<Vec<TelemetryEvent>>>,
    metrics: Arc<Mutex<MetricsAggregator>>,
    event_tx: mpsc::UnboundedSender<TelemetryEvent>,
    _background_task: tokio::task::JoinHandle<()>,
}

/// Metrics aggregator for real-time statistics
#[derive(Debug)]
struct MetricsAggregator {
    total_operations: u64,
    successful_operations: u64,
    failed_operations: u64,
    duration_sum_ms: u64,
    peak_memory_mb: f64,
    cpu_samples: Vec<f64>,
    total_errors: u64,
    security_events: u64,
    tools_usage: HashMap<String, ToolUsageTracker>,
    start_time: Instant,
}

#[derive(Debug)]
struct ToolUsageTracker {
    total_executions: u64,
    successful_executions: u64,
    failed_executions: u64,
    duration_sum_ms: u64,
    total_input_bytes: u64,
    total_output_bytes: u64,
    last_used: SystemTime,
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new(TelemetryConfig::default())
    }
}

impl TelemetryCollector {
    /// Create new telemetry collector
    pub fn new(config: TelemetryConfig) -> Self {
        let events = Arc::new(Mutex::new(Vec::new()));
        let metrics = Arc::new(Mutex::new(MetricsAggregator::new()));
        
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        // Start background processing task
        let background_task = Self::start_background_task(
            config.clone(),
            events.clone(),
            metrics.clone(),
            event_rx,
        );
        
        Self {
            config,
            events,
            metrics,
            event_tx,
            _background_task: background_task,
        }
    }
    
    /// Record a telemetry event
    pub fn record_event(&self, event: TelemetryEvent) {
        if !self.config.enabled {
            return;
        }
        
        // Apply sampling
        if self.config.sampling_rate < 1.0 {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            if rng.gen::<f64>() > self.config.sampling_rate {
                return;
            }
        }
        
        if let Err(e) = self.event_tx.send(event) {
            error!("Failed to send telemetry event: {}", e);
        }
    }
    
    /// Record tool execution start
    pub fn record_tool_start(&self, tool_name: String, operation_id: String, input_size: u64) {
        let event = TelemetryEvent::ToolExecutionStarted {
            tool_name,
            operation_id,
            timestamp: SystemTime::now(),
            input_size_bytes: input_size,
        };
        self.record_event(event);
    }
    
    /// Record tool execution completion
    pub fn record_tool_completion(
        &self,
        tool_name: String,
        operation_id: String,
        duration: Duration,
        success: bool,
        output_size: u64,
        error_message: Option<String>,
    ) {
        let event = TelemetryEvent::ToolExecutionCompleted {
            tool_name,
            operation_id,
            timestamp: SystemTime::now(),
            duration_ms: duration.as_millis() as u64,
            success,
            output_size_bytes: output_size,
            error_message,
        };
        self.record_event(event);
    }
    
    /// Record resource usage
    pub fn record_resource_usage(
        &self,
        operation_id: String,
        memory_mb: f64,
        cpu_percent: f64,
        disk_io_bytes: u64,
        network_io_bytes: u64,
    ) {
        let event = TelemetryEvent::ResourceUsage {
            operation_id,
            timestamp: SystemTime::now(),
            memory_mb,
            cpu_percent,
            disk_io_bytes,
            network_io_bytes,
        };
        self.record_event(event);
    }
    
    /// Record security event
    pub fn record_security_event(
        &self,
        event_type: SecurityEventType,
        operation_id: String,
        severity: SecuritySeverity,
        details: HashMap<String, String>,
    ) {
        let event = TelemetryEvent::SecurityEvent {
            event_type,
            operation_id,
            timestamp: SystemTime::now(),
            severity,
            details,
        };
        self.record_event(event);
    }
    
    /// Record performance metric
    pub fn record_performance_metric(
        &self,
        metric_name: String,
        operation_id: String,
        value: f64,
        unit: String,
        tags: HashMap<String, String>,
    ) {
        let event = TelemetryEvent::PerformanceMetric {
            metric_name,
            operation_id,
            timestamp: SystemTime::now(),
            value,
            unit,
            tags,
        };
        self.record_event(event);
    }
    
    /// Record error event
    pub fn record_error(
        &self,
        operation_id: String,
        error_type: String,
        error_message: String,
        stack_trace: Option<String>,
        context: HashMap<String, String>,
    ) {
        let event = TelemetryEvent::ErrorEvent {
            operation_id,
            timestamp: SystemTime::now(),
            error_type,
            error_message,
            stack_trace,
            context,
        };
        self.record_event(event);
    }
    
    /// Record health check
    pub fn record_health_check(
        &self,
        component: String,
        status: HealthStatus,
        latency_ms: Option<u64>,
        details: HashMap<String, String>,
    ) {
        let event = TelemetryEvent::HealthCheck {
            timestamp: SystemTime::now(),
            component,
            status,
            latency_ms,
            details,
        };
        self.record_event(event);
    }
    
    /// Get current metrics snapshot
    pub fn get_metrics(&self) -> Option<MetricsSnapshot> {
        let metrics = self.metrics.lock().ok()?;
        Some(metrics.snapshot())
    }
    
    /// Get recent events
    pub fn get_recent_events(&self, limit: usize) -> Vec<TelemetryEvent> {
        if let Ok(events) = self.events.lock() {
            events.iter()
                .rev()
                .take(limit)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Export events to external system
    pub async fn export_events(&self, endpoint: &str) -> Result<()> {
        let events = {
            let events = self.events.lock()
                .map_err(|_| anyhow::anyhow!("Lock error"))?;
            events.clone()
        };
        
        if events.is_empty() {
            return Ok(());
        }
        
        // Serialize events to JSON
        let json_payload = serde_json::to_string(&events)?;
        
        // Send to external endpoint (simplified HTTP client)
        let client = reqwest::Client::new();
        let response = client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .body(json_payload)
            .send()
            .await?;
        
        if response.status().is_success() {
            info!("Successfully exported {} telemetry events", events.len());
            
            // Clear exported events
            if let Ok(mut events) = self.events.lock() {
                events.clear();
            }
        } else {
            error!("Failed to export telemetry events: HTTP {}", response.status());
        }
        
        Ok(())
    }
    
    /// Clear old events based on retention policy
    pub fn cleanup_old_events(&self) {
        if let Ok(mut events) = self.events.lock() {
            let cutoff_time = SystemTime::now()
                .checked_sub(self.config.retention_period)
                .unwrap_or(SystemTime::UNIX_EPOCH);
            
            let initial_count = events.len();
            events.retain(|event| {
                match event {
                    TelemetryEvent::ToolExecutionStarted { timestamp, .. } => *timestamp > cutoff_time,
                    TelemetryEvent::ToolExecutionCompleted { timestamp, .. } => *timestamp > cutoff_time,
                    TelemetryEvent::ResourceUsage { timestamp, .. } => *timestamp > cutoff_time,
                    TelemetryEvent::SecurityEvent { timestamp, .. } => *timestamp > cutoff_time,
                    TelemetryEvent::PerformanceMetric { timestamp, .. } => *timestamp > cutoff_time,
                    TelemetryEvent::ErrorEvent { timestamp, .. } => *timestamp > cutoff_time,
                    TelemetryEvent::HealthCheck { timestamp, .. } => *timestamp > cutoff_time,
                }
            });
            
            let cleaned_count = initial_count - events.len();
            if cleaned_count > 0 {
                debug!("Cleaned up {} old telemetry events", cleaned_count);
            }
        }
    }
    
    /// Start background processing task
    fn start_background_task(
        config: TelemetryConfig,
        events: Arc<Mutex<Vec<TelemetryEvent>>>,
        metrics: Arc<Mutex<MetricsAggregator>>,
        mut event_rx: mpsc::UnboundedReceiver<TelemetryEvent>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut flush_interval = tokio::time::interval(config.flush_interval);
            let mut cleanup_interval = tokio::time::interval(Duration::from_secs(3600)); // Cleanup hourly
            
            loop {
                tokio::select! {
                    // Process incoming events
                    Some(event) = event_rx.recv() => {
                        Self::process_event(&events, &metrics, event);
                    }
                    
                    // Periodic flush
                    _ = flush_interval.tick() => {
                        Self::flush_events(&config, &events).await;
                    }
                    
                    // Periodic cleanup
                    _ = cleanup_interval.tick() => {
                        Self::cleanup_events(&config, &events);
                    }
                }
            }
        })
    }
    
    /// Process a single telemetry event
    fn process_event(
        events: &Arc<Mutex<Vec<TelemetryEvent>>>,
        metrics: &Arc<Mutex<MetricsAggregator>>,
        event: TelemetryEvent,
    ) {
        // Store event
        if let Ok(mut events) = events.lock() {
            events.push(event.clone());
        }
        
        // Update metrics
        if let Ok(mut metrics) = metrics.lock() {
            metrics.update(&event);
        }
    }
    
    /// Flush events to external systems
    async fn flush_events(
        config: &TelemetryConfig,
        events: &Arc<Mutex<Vec<TelemetryEvent>>>,
    ) {
        if config.export_enabled {
            if let Some(ref endpoint) = config.export_endpoint {
                debug!("Flushing telemetry events to: {}", endpoint);
                // Export logic would go here
            }
        }
    }
    
    /// Clean up old events
    fn cleanup_events(
        config: &TelemetryConfig,
        events: &Arc<Mutex<Vec<TelemetryEvent>>>,
    ) {
        if let Ok(mut events) = events.lock() {
            let cutoff_time = SystemTime::now()
                .checked_sub(config.retention_period)
                .unwrap_or(SystemTime::UNIX_EPOCH);
            
            let initial_count = events.len();
            events.retain(|event| {
                let event_time = match event {
                    TelemetryEvent::ToolExecutionStarted { timestamp, .. } => *timestamp,
                    TelemetryEvent::ToolExecutionCompleted { timestamp, .. } => *timestamp,
                    TelemetryEvent::ResourceUsage { timestamp, .. } => *timestamp,
                    TelemetryEvent::SecurityEvent { timestamp, .. } => *timestamp,
                    TelemetryEvent::PerformanceMetric { timestamp, .. } => *timestamp,
                    TelemetryEvent::ErrorEvent { timestamp, .. } => *timestamp,
                    TelemetryEvent::HealthCheck { timestamp, .. } => *timestamp,
                };
                event_time > cutoff_time
            });
            
            let cleaned = initial_count - events.len();
            if cleaned > 0 {
                debug!("Cleaned {} old telemetry events", cleaned);
            }
        }
    }
}

impl MetricsAggregator {
    fn new() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            duration_sum_ms: 0,
            peak_memory_mb: 0.0,
            cpu_samples: Vec::new(),
            total_errors: 0,
            security_events: 0,
            tools_usage: HashMap::new(),
            start_time: Instant::now(),
        }
    }
    
    fn update(&mut self, event: &TelemetryEvent) {
        match event {
            TelemetryEvent::ToolExecutionCompleted {
                tool_name,
                duration_ms,
                success,
                output_size_bytes,
                ..
            } => {
                self.total_operations += 1;
                if *success {
                    self.successful_operations += 1;
                } else {
                    self.failed_operations += 1;
                }
                self.duration_sum_ms += duration_ms;
                
                // Update tool-specific metrics
                let tool_tracker = self.tools_usage.entry(tool_name.clone()).or_insert(ToolUsageTracker {
                    total_executions: 0,
                    successful_executions: 0,
                    failed_executions: 0,
                    duration_sum_ms: 0,
                    total_input_bytes: 0,
                    total_output_bytes: 0,
                    last_used: SystemTime::now(),
                });
                
                tool_tracker.total_executions += 1;
                if *success {
                    tool_tracker.successful_executions += 1;
                } else {
                    tool_tracker.failed_executions += 1;
                }
                tool_tracker.duration_sum_ms += duration_ms;
                tool_tracker.total_output_bytes += output_size_bytes;
                tool_tracker.last_used = SystemTime::now();
            }
            
            TelemetryEvent::ResourceUsage { memory_mb, cpu_percent, .. } => {
                self.peak_memory_mb = self.peak_memory_mb.max(*memory_mb);
                self.cpu_samples.push(*cpu_percent);
                
                // Keep only last 1000 CPU samples to prevent unbounded growth
                if self.cpu_samples.len() > 1000 {
                    self.cpu_samples.remove(0);
                }
            }
            
            TelemetryEvent::ErrorEvent { .. } => {
                self.total_errors += 1;
            }
            
            TelemetryEvent::SecurityEvent { .. } => {
                self.security_events += 1;
            }
            
            _ => {}
        }
    }
    
    fn snapshot(&self) -> MetricsSnapshot {
        let average_duration = if self.total_operations > 0 {
            self.duration_sum_ms as f64 / self.total_operations as f64
        } else {
            0.0
        };
        
        let average_cpu = if !self.cpu_samples.is_empty() {
            self.cpu_samples.iter().sum::<f64>() / self.cpu_samples.len() as f64
        } else {
            0.0
        };
        
        let mut tools_usage = HashMap::new();
        for (tool_name, tracker) in &self.tools_usage {
            let avg_duration = if tracker.total_executions > 0 {
                tracker.duration_sum_ms as f64 / tracker.total_executions as f64
            } else {
                0.0
            };
            
            tools_usage.insert(tool_name.clone(), ToolUsageMetrics {
                total_executions: tracker.total_executions,
                successful_executions: tracker.successful_executions,
                failed_executions: tracker.failed_executions,
                average_duration_ms: avg_duration,
                total_input_bytes: tracker.total_input_bytes,
                total_output_bytes: tracker.total_output_bytes,
                last_used: tracker.last_used,
            });
        }
        
        MetricsSnapshot {
            timestamp: SystemTime::now(),
            total_operations: self.total_operations,
            successful_operations: self.successful_operations,
            failed_operations: self.failed_operations,
            average_duration_ms: average_duration,
            peak_memory_mb: self.peak_memory_mb,
            average_cpu_percent: average_cpu,
            total_errors: self.total_errors,
            security_events: self.security_events,
            tools_usage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_telemetry_collector_creation() {
        let config = TelemetryConfig::default();
        let collector = TelemetryCollector::new(config);
        
        // Collector should be created successfully
        assert!(collector.config.enabled);
    }

    #[tokio::test]
    async fn test_event_recording() {
        let collector = TelemetryCollector::default();
        
        collector.record_tool_start(
            "test-tool".to_string(),
            "op-123".to_string(),
            1024,
        );
        
        collector.record_tool_completion(
            "test-tool".to_string(),
            "op-123".to_string(),
            Duration::from_millis(500),
            true,
            2048,
            None,
        );
        
        // Give background task time to process
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let events = collector.get_recent_events(10);
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_metrics_aggregation() {
        let collector = TelemetryCollector::default();
        
        // Record some operations
        collector.record_tool_completion(
            "tool1".to_string(),
            "op1".to_string(),
            Duration::from_millis(100),
            true,
            512,
            None,
        );
        
        collector.record_tool_completion(
            "tool2".to_string(),
            "op2".to_string(),
            Duration::from_millis(200),
            false,
            0,
            Some("Test error".to_string()),
        );
        
        // Give background task time to process
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let metrics = collector.get_metrics().unwrap();
        assert_eq!(metrics.total_operations, 2);
        assert_eq!(metrics.successful_operations, 1);
        assert_eq!(metrics.failed_operations, 1);
        assert_eq!(metrics.average_duration_ms, 150.0);
    }

    #[test]
    fn test_security_severity_serialization() {
        let severity = SecuritySeverity::Critical;
        let json = serde_json::to_string(&severity).unwrap();
        let deserialized: SecuritySeverity = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            SecuritySeverity::Critical => {} // Expected
            _ => panic!("Serialization failed"),
        }
    }
}

// Add reqwest to Cargo.toml for HTTP export functionality
// Add rand to Cargo.toml for sampling