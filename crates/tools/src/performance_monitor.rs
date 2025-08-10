// @component: {"k":"C","id":"tool_performance_monitor","t":"Comprehensive tool performance monitoring and analytics","m":{"cur":5,"tgt":90,"u":"%"},"f":["monitoring","analytics","metrics","performance","alerting"]}

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

/// Comprehensive performance metrics for a tool
#[derive(Debug, Clone)]
pub struct ToolPerformanceMetrics {
    pub tool_name: String,
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub average_execution_time: Duration,
    pub min_execution_time: Duration,
    pub max_execution_time: Duration,
    pub p95_execution_time: Duration,
    pub p99_execution_time: Duration,
    pub success_rate: f32,
    pub error_rate: f32,
    pub throughput_per_minute: f32,
    pub last_execution: Option<Instant>,
    pub first_execution: Option<Instant>,
    pub recent_errors: Vec<ToolError>,
    pub performance_trend: PerformanceTrend,
}

/// Tool error tracking
#[derive(Debug, Clone)]
pub struct ToolError {
    pub timestamp: Instant,
    pub error_type: String,
    pub error_message: String,
    pub execution_time: Duration,
    pub context: String,
}

/// Performance trend analysis
#[derive(Debug, Clone)]
pub enum PerformanceTrend {
    Improving,
    Stable,
    Degrading,
    Volatile,
    Insufficient, // Not enough data
}

/// Performance alert levels
#[derive(Debug, Clone, PartialEq)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Performance alert
#[derive(Debug, Clone)]
pub struct PerformanceAlert {
    pub tool_name: String,
    pub alert_type: AlertType,
    pub level: AlertLevel,
    pub message: String,
    pub timestamp: Instant,
    pub metrics: ToolPerformanceSnapshot,
}

/// Types of performance alerts
#[derive(Debug, Clone)]
pub enum AlertType {
    HighErrorRate,
    SlowExecution,
    MemoryLeak,
    Timeout,
    CircuitBreakerTrip,
    ThroughputDrop,
    ConsistentFailures,
}

/// Snapshot of tool performance at a point in time
#[derive(Debug, Clone)]
pub struct ToolPerformanceSnapshot {
    pub timestamp: Instant,
    pub execution_time: Duration,
    pub success: bool,
    pub memory_usage: Option<u64>,
    pub cpu_usage: Option<f32>,
}

/// Performance monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub sample_window: Duration, // Window for calculating recent metrics
    pub alert_thresholds: AlertThresholds,
    pub max_error_history: usize,      // Max recent errors to keep
    pub enable_detailed_metrics: bool, // Enable CPU/Memory tracking
    pub enable_alerting: bool,         // Enable performance alerts
}

/// Alert threshold configuration
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    pub error_rate_warning: f32,  // Warning threshold for error rate (%)
    pub error_rate_critical: f32, // Critical threshold for error rate (%)
    pub execution_time_warning: Duration, // Warning threshold for execution time
    pub execution_time_critical: Duration, // Critical threshold for execution time
    pub throughput_drop_warning: f32, // Warning threshold for throughput drop (%)
    pub throughput_drop_critical: f32, // Critical threshold for throughput drop (%)
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            sample_window: Duration::from_secs(300), // 5 minutes
            alert_thresholds: AlertThresholds {
                error_rate_warning: 10.0,
                error_rate_critical: 25.0,
                execution_time_warning: Duration::from_secs(5),
                execution_time_critical: Duration::from_secs(15),
                throughput_drop_warning: 30.0,
                throughput_drop_critical: 50.0,
            },
            max_error_history: 50,
            enable_detailed_metrics: true,
            enable_alerting: true,
        }
    }
}

/// Tool performance monitor with advanced analytics
pub struct ToolPerformanceMonitor {
    /// Performance metrics for each tool
    metrics: Arc<RwLock<HashMap<String, ToolPerformanceMetrics>>>,

    /// Recent performance snapshots for trend analysis
    snapshots: Arc<Mutex<HashMap<String, Vec<ToolPerformanceSnapshot>>>>,

    /// Performance alerts
    alerts: Arc<Mutex<Vec<PerformanceAlert>>>,

    /// Monitor configuration
    config: MonitorConfig,

    /// Global statistics
    global_stats: Arc<Mutex<GlobalPerformanceStats>>,
}

/// Global performance statistics across all tools
#[derive(Debug, Default)]
pub struct GlobalPerformanceStats {
    pub total_tool_executions: u64,
    pub total_tool_failures: u64,
    pub average_system_load: f32,
    pub peak_concurrent_executions: u32,
    pub current_concurrent_executions: u32,
    pub system_uptime: Duration,
    pub start_time: Option<Instant>,
}

impl ToolPerformanceMonitor {
    pub fn new(config: MonitorConfig) -> Self {
        info!("📊 Initializing Tool Performance Monitor");
        let global_stats = GlobalPerformanceStats {
            start_time: Some(Instant::now()),
            ..Default::default()
        };
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            snapshots: Arc::new(Mutex::new(HashMap::new())),
            alerts: Arc::new(Mutex::new(Vec::new())),
            config,
            global_stats: Arc::new(Mutex::new(global_stats)),
        }
    }

    /// Record tool execution start
    pub async fn execution_started(&self, tool_name: &str) -> ExecutionTracker {
        {
            let mut global = self.global_stats.lock().await;
            global.current_concurrent_executions += 1;
            global.peak_concurrent_executions = global
                .peak_concurrent_executions
                .max(global.current_concurrent_executions);
        }

        ExecutionTracker {
            tool_name: tool_name.to_string(),
            start_time: Instant::now(),
            monitor: Arc::new(self.clone()),
        }
    }

    /// Record tool execution completion
    pub async fn execution_completed(
        &self,
        tool_name: &str,
        execution_time: Duration,
        success: bool,
        error_message: Option<String>,
        context: Option<String>,
    ) {
        // Update global stats
        {
            let mut global = self.global_stats.lock().await;
            global.total_tool_executions += 1;
            global.current_concurrent_executions =
                global.current_concurrent_executions.saturating_sub(1);
            if !success {
                global.total_tool_failures += 1;
            }
            if let Some(start_time) = global.start_time {
                global.system_uptime = start_time.elapsed();
            }
        }

        // Update tool-specific metrics
        {
            let mut metrics = self.metrics.write().await;
            let tool_metrics = metrics
                .entry(tool_name.to_string())
                .or_insert_with(|| self.create_initial_metrics(tool_name));

            self.update_tool_metrics(
                tool_metrics,
                execution_time,
                success,
                error_message,
                context,
            )
            .await;
        }

        // Create performance snapshot
        let snapshot = ToolPerformanceSnapshot {
            timestamp: Instant::now(),
            execution_time,
            success,
            memory_usage: if self.config.enable_detailed_metrics {
                self.get_current_memory_usage().await
            } else {
                None
            },
            cpu_usage: if self.config.enable_detailed_metrics {
                self.get_current_cpu_usage().await
            } else {
                None
            },
        };

        // Store snapshot for trend analysis
        {
            let mut snapshots = self.snapshots.lock().await;
            let tool_snapshots = snapshots
                .entry(tool_name.to_string())
                .or_insert_with(Vec::new);

            tool_snapshots.push(snapshot.clone());

            // Keep only recent snapshots
            let cutoff_time = Instant::now() - self.config.sample_window;
            tool_snapshots.retain(|s| s.timestamp > cutoff_time);
        }

        // Check for performance alerts
        if self.config.enable_alerting {
            self.check_performance_alerts(tool_name, &snapshot).await;
        }

        debug!(
            "📊 Recorded execution for {}: {:?}, success: {}",
            tool_name, execution_time, success
        );
    }

    /// Create initial metrics for a new tool
    fn create_initial_metrics(&self, tool_name: &str) -> ToolPerformanceMetrics {
        ToolPerformanceMetrics {
            tool_name: tool_name.to_string(),
            total_executions: 0,
            successful_executions: 0,
            failed_executions: 0,
            average_execution_time: Duration::from_millis(0),
            min_execution_time: Duration::from_secs(u64::MAX),
            max_execution_time: Duration::from_millis(0),
            p95_execution_time: Duration::from_millis(0),
            p99_execution_time: Duration::from_millis(0),
            success_rate: 0.0,
            error_rate: 0.0,
            throughput_per_minute: 0.0,
            last_execution: None,
            first_execution: None,
            recent_errors: Vec::new(),
            performance_trend: PerformanceTrend::Insufficient,
        }
    }

    /// Update tool metrics with new execution data
    async fn update_tool_metrics(
        &self,
        metrics: &mut ToolPerformanceMetrics,
        execution_time: Duration,
        success: bool,
        error_message: Option<String>,
        context: Option<String>,
    ) {
        // Update basic counters
        metrics.total_executions += 1;
        if success {
            metrics.successful_executions += 1;
        } else {
            metrics.failed_executions += 1;
        }

        // Update timing metrics
        let total_time = metrics.average_execution_time
            * metrics.total_executions.saturating_sub(1) as u32
            + execution_time;
        metrics.average_execution_time = total_time / metrics.total_executions as u32;
        metrics.min_execution_time = metrics.min_execution_time.min(execution_time);
        metrics.max_execution_time = metrics.max_execution_time.max(execution_time);

        // Update rates
        metrics.success_rate =
            (metrics.successful_executions as f32 / metrics.total_executions as f32) * 100.0;
        metrics.error_rate =
            (metrics.failed_executions as f32 / metrics.total_executions as f32) * 100.0;

        // Update timestamps
        let now = Instant::now();
        if metrics.first_execution.is_none() {
            metrics.first_execution = Some(now);
        }
        metrics.last_execution = Some(now);

        // Calculate throughput (executions per minute)
        if let Some(first_exec) = metrics.first_execution {
            let duration_minutes = first_exec.elapsed().as_secs_f32() / 60.0;
            if duration_minutes > 0.0 {
                metrics.throughput_per_minute = metrics.total_executions as f32 / duration_minutes;
            }
        }

        // Record error if failed
        if !success {
            if let Some(error_msg) = error_message {
                let error = ToolError {
                    timestamp: now,
                    error_type: "execution_failure".to_string(),
                    error_message: error_msg,
                    execution_time,
                    context: context.unwrap_or_else(|| "unknown".to_string()),
                };

                metrics.recent_errors.push(error);

                // Limit error history
                if metrics.recent_errors.len() > self.config.max_error_history {
                    metrics.recent_errors.remove(0);
                }
            }
        }

        // Update performance trend
        metrics.performance_trend = self.calculate_performance_trend(&metrics.tool_name).await;

        // Update percentile metrics (simplified calculation)
        self.update_percentile_metrics(metrics).await;
    }

    /// Calculate performance trend based on recent snapshots
    async fn calculate_performance_trend(&self, tool_name: &str) -> PerformanceTrend {
        let snapshots = self.snapshots.lock().await;
        let tool_snapshots = match snapshots.get(tool_name) {
            Some(snapshots) if snapshots.len() >= 10 => snapshots,
            _ => return PerformanceTrend::Insufficient,
        };

        // Split snapshots into two halves for comparison
        let mid_point = tool_snapshots.len() / 2;
        let first_half = &tool_snapshots[0..mid_point];
        let second_half = &tool_snapshots[mid_point..];

        let first_avg = first_half
            .iter()
            .map(|s| s.execution_time.as_millis() as f32)
            .sum::<f32>()
            / first_half.len() as f32;

        let second_avg = second_half
            .iter()
            .map(|s| s.execution_time.as_millis() as f32)
            .sum::<f32>()
            / second_half.len() as f32;

        let first_success_rate =
            first_half.iter().filter(|s| s.success).count() as f32 / first_half.len() as f32;

        let second_success_rate =
            second_half.iter().filter(|s| s.success).count() as f32 / second_half.len() as f32;

        // Calculate variance to detect volatility
        let variance = tool_snapshots
            .iter()
            .map(|s| {
                let diff = s.execution_time.as_millis() as f32 - second_avg;
                diff * diff
            })
            .sum::<f32>()
            / tool_snapshots.len() as f32;

        let coefficient_of_variation = variance.sqrt() / second_avg;

        // Determine trend
        if coefficient_of_variation > 0.8 {
            PerformanceTrend::Volatile
        } else if second_avg < first_avg * 0.9 && second_success_rate > first_success_rate {
            PerformanceTrend::Improving
        } else if second_avg > first_avg * 1.1 || second_success_rate < first_success_rate * 0.9 {
            PerformanceTrend::Degrading
        } else {
            PerformanceTrend::Stable
        }
    }

    /// Update percentile metrics (simplified calculation)
    async fn update_percentile_metrics(&self, metrics: &mut ToolPerformanceMetrics) {
        let snapshots = self.snapshots.lock().await;
        if let Some(tool_snapshots) = snapshots.get(&metrics.tool_name) {
            if tool_snapshots.len() >= 20 {
                let mut execution_times: Vec<Duration> =
                    tool_snapshots.iter().map(|s| s.execution_time).collect();
                execution_times.sort();

                let p95_index = (execution_times.len() as f32 * 0.95) as usize;
                let p99_index = (execution_times.len() as f32 * 0.99) as usize;

                metrics.p95_execution_time = execution_times
                    .get(p95_index.min(execution_times.len() - 1))
                    .copied()
                    .unwrap_or(Duration::from_millis(0));

                metrics.p99_execution_time = execution_times
                    .get(p99_index.min(execution_times.len() - 1))
                    .copied()
                    .unwrap_or(Duration::from_millis(0));
            }
        }
    }

    /// Check for performance alerts
    async fn check_performance_alerts(&self, tool_name: &str, snapshot: &ToolPerformanceSnapshot) {
        let metrics = self.metrics.read().await;
        if let Some(tool_metrics) = metrics.get(tool_name) {
            let mut alerts_to_add = Vec::new();

            // Check error rate
            if tool_metrics.error_rate >= self.config.alert_thresholds.error_rate_critical {
                alerts_to_add.push(PerformanceAlert {
                    tool_name: tool_name.to_string(),
                    alert_type: AlertType::HighErrorRate,
                    level: AlertLevel::Critical,
                    message: format!("Critical error rate: {:.1}%", tool_metrics.error_rate),
                    timestamp: Instant::now(),
                    metrics: snapshot.clone(),
                });
            } else if tool_metrics.error_rate >= self.config.alert_thresholds.error_rate_warning {
                alerts_to_add.push(PerformanceAlert {
                    tool_name: tool_name.to_string(),
                    alert_type: AlertType::HighErrorRate,
                    level: AlertLevel::Warning,
                    message: format!("High error rate: {:.1}%", tool_metrics.error_rate),
                    timestamp: Instant::now(),
                    metrics: snapshot.clone(),
                });
            }

            // Check execution time
            if snapshot.execution_time >= self.config.alert_thresholds.execution_time_critical {
                alerts_to_add.push(PerformanceAlert {
                    tool_name: tool_name.to_string(),
                    alert_type: AlertType::SlowExecution,
                    level: AlertLevel::Critical,
                    message: format!("Critical slow execution: {:?}", snapshot.execution_time),
                    timestamp: Instant::now(),
                    metrics: snapshot.clone(),
                });
            } else if snapshot.execution_time >= self.config.alert_thresholds.execution_time_warning
            {
                alerts_to_add.push(PerformanceAlert {
                    tool_name: tool_name.to_string(),
                    alert_type: AlertType::SlowExecution,
                    level: AlertLevel::Warning,
                    message: format!("Slow execution: {:?}", snapshot.execution_time),
                    timestamp: Instant::now(),
                    metrics: snapshot.clone(),
                });
            }

            // Add alerts
            if !alerts_to_add.is_empty() {
                let mut alerts = self.alerts.lock().await;
                for alert in alerts_to_add {
                    match alert.level {
                        AlertLevel::Critical | AlertLevel::Emergency => {
                            error!(
                                "🚨 Performance Alert: {} - {}",
                                alert.tool_name, alert.message
                            );
                        }
                        AlertLevel::Warning => {
                            warn!(
                                "⚠️ Performance Alert: {} - {}",
                                alert.tool_name, alert.message
                            );
                        }
                        AlertLevel::Info => {
                            info!(
                                "ℹ️ Performance Alert: {} - {}",
                                alert.tool_name, alert.message
                            );
                        }
                    }
                    alerts.push(alert);
                }

                // Limit alert history
                if alerts.len() > 1000 {
                    alerts.drain(0..500); // Remove oldest 500 alerts
                }
            }
        }
    }

    /// Get current memory usage (simplified)
    async fn get_current_memory_usage(&self) -> Option<u64> {
        // In a real implementation, this would get actual memory usage
        // For now, return None as placeholder
        None
    }

    /// Get current CPU usage (simplified)
    async fn get_current_cpu_usage(&self) -> Option<f32> {
        // In a real implementation, this would get actual CPU usage
        // For now, return None as placeholder
        None
    }

    /// Get comprehensive performance report
    pub async fn get_performance_report(&self) -> String {
        let metrics = self.metrics.read().await;
        let global = self.global_stats.lock().await;
        let alerts = self.alerts.lock().await;

        let mut report = format!(
            "📊 Tool Performance Monitor Report\n\n\
             🌐 Global Statistics:\n\
             • Total executions: {}\n\
             • Total failures: {}\n\
             • System uptime: {:?}\n\
             • Peak concurrent executions: {}\n\
             • Global success rate: {:.1}%\n\n",
            global.total_tool_executions,
            global.total_tool_failures,
            global.system_uptime,
            global.peak_concurrent_executions,
            if global.total_tool_executions > 0 {
                ((global.total_tool_executions - global.total_tool_failures) as f32
                    / global.total_tool_executions as f32)
                    * 100.0
            } else {
                0.0
            }
        );

        // Sort tools by total executions
        let mut tool_list: Vec<_> = metrics.iter().collect();
        tool_list.sort_by(|a, b| b.1.total_executions.cmp(&a.1.total_executions));

        report.push_str("🛠️ Tool Performance:\n");
        for (name, tool_metrics) in tool_list.iter().take(10) {
            let trend_icon = match tool_metrics.performance_trend {
                PerformanceTrend::Improving => "📈",
                PerformanceTrend::Stable => "➡️",
                PerformanceTrend::Degrading => "📉",
                PerformanceTrend::Volatile => "📊",
                PerformanceTrend::Insufficient => "❓",
            };

            report.push_str(&format!(
                "\n • {} {}: {} executions, {:.1}% success, {:?} avg ({:.1}/min) {}",
                trend_icon,
                name,
                tool_metrics.total_executions,
                tool_metrics.success_rate,
                tool_metrics.average_execution_time,
                tool_metrics.throughput_per_minute,
                match tool_metrics.performance_trend {
                    PerformanceTrend::Improving => "improving",
                    PerformanceTrend::Stable => "stable",
                    PerformanceTrend::Degrading => "degrading",
                    PerformanceTrend::Volatile => "volatile",
                    PerformanceTrend::Insufficient => "insufficient data",
                }
            ));
        }

        // Show recent alerts
        let recent_alerts: Vec<_> = alerts
            .iter()
            .filter(|a| a.timestamp.elapsed() < Duration::from_secs(3600)) // Last hour
            .collect();

        if !recent_alerts.is_empty() {
            report.push_str("\n\n🚨 Recent Alerts (Last Hour):\n");
            for alert in recent_alerts.iter().take(10) {
                let level_icon = match alert.level {
                    AlertLevel::Emergency => "🆘",
                    AlertLevel::Critical => "🚨",
                    AlertLevel::Warning => "⚠️",
                    AlertLevel::Info => "ℹ️",
                };

                report.push_str(&format!(
                    "\n {} {}: {}",
                    level_icon, alert.tool_name, alert.message
                ));
            }
        }

        report
    }

    /// Get tool-specific metrics
    pub async fn get_tool_metrics(&self, tool_name: &str) -> Option<ToolPerformanceMetrics> {
        let metrics = self.metrics.read().await;
        metrics.get(tool_name).cloned()
    }

    /// Clear all metrics (for testing)
    pub async fn clear_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.clear();

        let mut snapshots = self.snapshots.lock().await;
        snapshots.clear();

        let mut alerts = self.alerts.lock().await;
        alerts.clear();

        let mut global = self.global_stats.lock().await;
        *global = GlobalPerformanceStats::default();
        global.start_time = Some(Instant::now());
    }
}

impl Clone for ToolPerformanceMonitor {
    fn clone(&self) -> Self {
        Self {
            metrics: Arc::clone(&self.metrics),
            snapshots: Arc::clone(&self.snapshots),
            alerts: Arc::clone(&self.alerts),
            config: self.config.clone(),
            global_stats: Arc::clone(&self.global_stats),
        }
    }
}

/// Execution tracker for automatic metric collection
pub struct ExecutionTracker {
    tool_name: String,
    start_time: Instant,
    monitor: Arc<ToolPerformanceMonitor>,
}

impl ExecutionTracker {
    /// Complete execution with success
    pub async fn success(self, context: Option<String>) {
        let execution_time = self.start_time.elapsed();
        self.monitor
            .execution_completed(&self.tool_name, execution_time, true, None, context)
            .await;
    }

    /// Complete execution with failure
    pub async fn failure(self, error_message: String, context: Option<String>) {
        let execution_time = self.start_time.elapsed();
        self.monitor
            .execution_completed(
                &self.tool_name,
                execution_time,
                false,
                Some(error_message),
                context,
            )
            .await;
    }
}

impl Drop for ExecutionTracker {
    fn drop(&mut self) {
        // If tracker is dropped without explicit completion, record as failure
        let execution_time = self.start_time.elapsed();
        let monitor = Arc::clone(&self.monitor);
        let tool_name = self.tool_name.clone();

        tokio::spawn(async move {
            monitor
                .execution_completed(
                    &tool_name,
                    execution_time,
                    false,
                    Some("Execution tracker dropped without completion".to_string()),
                    Some("implicit_failure".to_string()),
                )
                .await;
        });
    }
}

impl Default for ToolPerformanceMonitor {
    fn default() -> Self {
        Self::new(MonitorConfig::default())
    }
}
