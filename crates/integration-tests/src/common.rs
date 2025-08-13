//! Common utilities for integration tests

use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Test fixture manager for setting up test data and environments
pub struct TestFixture {
    pub temp_dir: std::path::PathBuf,
    pub config_files: HashMap<String, std::path::PathBuf>,
    pub test_data: HashMap<String, serde_json::Value>,
}

impl TestFixture {
    /// Create new test fixture with isolated environment
    pub async fn new(test_name: &str) -> Result<Self> {
        let temp_dir = std::env::temp_dir()
            .join("magray_integration_tests")
            .join(test_name)
            .join(format!("run_{}", uuid::Uuid::new_v4()));

        tokio::fs::create_dir_all(&temp_dir).await?;
        info!("Created test fixture directory: {}", temp_dir.display());

        Ok(Self {
            temp_dir,
            config_files: HashMap::new(),
            test_data: HashMap::new(),
        })
    }

    /// Create a configuration file for testing
    pub async fn create_config_file(
        &mut self,
        name: &str,
        content: &str,
    ) -> Result<std::path::PathBuf> {
        let config_path = self.temp_dir.join(format!("{}.toml", name));
        tokio::fs::write(&config_path, content).await?;
        self.config_files
            .insert(name.to_string(), config_path.clone());
        info!("Created config file: {} at {}", name, config_path.display());
        Ok(config_path)
    }

    /// Create test data file (JSON)
    pub async fn create_test_data(
        &mut self,
        name: &str,
        data: serde_json::Value,
    ) -> Result<std::path::PathBuf> {
        let data_path = self.temp_dir.join(format!("{}.json", name));
        let content = serde_json::to_string_pretty(&data)?;
        tokio::fs::write(&data_path, content).await?;
        self.test_data.insert(name.to_string(), data);
        info!(
            "Created test data file: {} at {}",
            name,
            data_path.display()
        );
        Ok(data_path)
    }

    /// Get path within fixture directory
    pub fn path(&self, relative_path: &str) -> std::path::PathBuf {
        self.temp_dir.join(relative_path)
    }

    /// Clean up fixture - best effort cleanup on Windows
    pub async fn cleanup(&self) -> Result<()> {
        if self.temp_dir.exists() {
            // On Windows, try multiple times with delay for file handles to close
            for attempt in 0..3 {
                match tokio::fs::remove_dir_all(&self.temp_dir).await {
                    Ok(_) => {
                        info!("Cleaned up fixture directory: {}", self.temp_dir.display());
                        return Ok(());
                    }
                    Err(e) if attempt < 2 => {
                        warn!("Cleanup attempt {} failed: {}, retrying...", attempt + 1, e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }
                    Err(e) => {
                        warn!(
                            "Final cleanup attempt failed: {}. Directory may remain: {}",
                            e,
                            self.temp_dir.display()
                        );
                        // Don't fail the test due to cleanup issues
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Performance metrics collector for integration tests
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub test_name: String,
    pub start_time: Instant,
    pub measurements: HashMap<String, f64>,
    pub timing_marks: HashMap<String, Instant>,
    pub counters: HashMap<String, u64>,
}

impl PerformanceMetrics {
    pub fn new(test_name: &str) -> Self {
        Self {
            test_name: test_name.to_string(),
            start_time: Instant::now(),
            measurements: HashMap::new(),
            timing_marks: HashMap::new(),
            counters: HashMap::new(),
        }
    }

    /// Mark a timing point
    pub fn mark(&mut self, name: &str) {
        self.timing_marks.insert(name.to_string(), Instant::now());
    }

    /// Measure duration since mark
    pub fn measure_since_mark(&mut self, name: &str, mark: &str) -> Option<Duration> {
        if let Some(mark_time) = self.timing_marks.get(mark) {
            let duration = mark_time.elapsed();
            self.measurements
                .insert(format!("{}_duration_ms", name), duration.as_millis() as f64);
            Some(duration)
        } else {
            warn!("Mark '{}' not found for measurement '{}'", mark, name);
            None
        }
    }

    /// Record a measurement
    pub fn record(&mut self, name: &str, value: f64) {
        self.measurements.insert(name.to_string(), value);
    }

    /// Increment a counter
    pub fn increment(&mut self, name: &str) {
        let current = self.counters.get(name).unwrap_or(&0);
        self.counters.insert(name.to_string(), current + 1);
    }

    /// Get total test duration
    pub fn total_duration(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Generate performance report
    pub fn report(&self) -> String {
        let mut report = String::new();
        report.push_str(&format!("Performance Report: {}\n", self.test_name));
        report.push_str(&format!("Total Duration: {:?}\n", self.total_duration()));

        if !self.measurements.is_empty() {
            report.push_str("\nMeasurements:\n");
            for (name, value) in &self.measurements {
                report.push_str(&format!("  {}: {:.2}\n", name, value));
            }
        }

        if !self.counters.is_empty() {
            report.push_str("\nCounters:\n");
            for (name, value) in &self.counters {
                report.push_str(&format!("  {}: {}\n", name, value));
            }
        }

        report
    }
}

/// Memory usage tracker
pub struct MemoryTracker {
    initial_usage: Option<u64>,
    peak_usage: u64,
    measurements: Vec<(Instant, u64)>,
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            initial_usage: Self::get_memory_usage(),
            peak_usage: 0,
            measurements: Vec::new(),
        }
    }

    /// Get current memory usage in bytes (approximate)
    fn get_memory_usage() -> Option<u64> {
        // This is a simplified implementation
        // In production, you might want to use more accurate memory tracking
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<u64>() {
                                return Some(kb * 1024); // Convert KB to bytes
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Record current memory usage
    pub fn record(&mut self) {
        if let Some(usage) = Self::get_memory_usage() {
            self.measurements.push((Instant::now(), usage));
            if usage > self.peak_usage {
                self.peak_usage = usage;
            }
        }
    }

    /// Get memory growth since start
    pub fn memory_growth_mb(&self) -> Option<f64> {
        if let (Some(initial), Some(current)) = (self.initial_usage, Self::get_memory_usage()) {
            Some((current as f64 - initial as f64) / (1024.0 * 1024.0))
        } else {
            None
        }
    }

    /// Get peak memory usage
    pub fn peak_usage_mb(&self) -> f64 {
        self.peak_usage as f64 / (1024.0 * 1024.0)
    }
}

/// Component health checker
pub struct HealthChecker {
    checks: HashMap<String, Box<dyn ComponentHealthCheck>>,
}

// Use a boxed future instead of async trait for dyn compatibility
pub trait ComponentHealthCheck: Send + Sync {
    fn check_health(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<HealthStatus>> + Send + '_>>;
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub message: String,
    pub metrics: HashMap<String, f64>,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            checks: HashMap::new(),
        }
    }

    pub fn add_check(&mut self, name: &str, check: Box<dyn ComponentHealthCheck>) {
        self.checks.insert(name.to_string(), check);
    }

    /// Run all health checks
    pub async fn check_all(&self) -> HashMap<String, HealthStatus> {
        let mut results = HashMap::new();

        for (name, check) in &self.checks {
            match check.check_health().await {
                Ok(status) => {
                    results.insert(name.clone(), status);
                }
                Err(e) => {
                    results.insert(
                        name.clone(),
                        HealthStatus {
                            healthy: false,
                            message: format!("Health check failed: {}", e),
                            metrics: HashMap::new(),
                        },
                    );
                }
            }
        }

        results
    }

    /// Check if all components are healthy
    pub async fn all_healthy(&self) -> bool {
        let results = self.check_all().await;
        results.values().all(|status| status.healthy)
    }
}

/// Test timeout wrapper
pub async fn with_timeout<F, T>(duration: Duration, operation_name: &str, future: F) -> Result<T>
where
    F: std::future::Future<Output = Result<T>>,
{
    match tokio::time::timeout(duration, future).await {
        Ok(result) => result,
        Err(_) => {
            anyhow::bail!(
                "Operation '{}' timed out after {:?}",
                operation_name,
                duration
            )
        }
    }
}

/// Retry mechanism for flaky operations
pub async fn retry_with_backoff<F, T, E>(
    mut operation: F,
    max_attempts: usize,
    initial_delay: Duration,
    operation_name: &str,
) -> Result<T>
where
    F: FnMut() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send>>,
    E: std::fmt::Display + Send + Sync + 'static,
{
    let mut delay = initial_delay;

    for attempt in 1..=max_attempts {
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    info!(
                        "Operation '{}' succeeded on attempt {}/{}",
                        operation_name, attempt, max_attempts
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt == max_attempts {
                    anyhow::bail!(
                        "Operation '{}' failed after {} attempts. Last error: {}",
                        operation_name,
                        max_attempts,
                        e
                    );
                }

                warn!(
                    "Operation '{}' failed on attempt {}/{}: {}. Retrying in {:?}",
                    operation_name, attempt, max_attempts, e, delay
                );

                tokio::time::sleep(delay).await;
                delay = Duration::from_millis((delay.as_millis() as f64 * 1.5) as u64);
            }
        }
    }

    unreachable!()
}

/// Test assertion helpers
pub mod assertions {
    use super::*;

    /// Assert performance metrics meet criteria
    pub fn assert_performance_criteria(
        metrics: &PerformanceMetrics,
        criteria: &HashMap<String, f64>,
    ) -> Result<()> {
        for (metric_name, expected_max) in criteria {
            if let Some(actual_value) = metrics.measurements.get(metric_name) {
                if actual_value > expected_max {
                    anyhow::bail!(
                        "Performance criteria failed: {} = {:.2} > {:.2} (max)",
                        metric_name,
                        actual_value,
                        expected_max
                    );
                }
            } else {
                anyhow::bail!("Performance metric '{}' not found", metric_name);
            }
        }
        Ok(())
    }

    /// Assert memory usage is within bounds
    pub fn assert_memory_bounds(tracker: &MemoryTracker, max_growth_mb: f64) -> Result<()> {
        if let Some(growth) = tracker.memory_growth_mb() {
            if growth > max_growth_mb {
                anyhow::bail!(
                    "Memory growth {:.2}MB exceeds limit {:.2}MB",
                    growth,
                    max_growth_mb
                );
            }
        }
        Ok(())
    }

    /// Assert all components are healthy
    pub async fn assert_all_healthy(health_checker: &HealthChecker) -> Result<()> {
        let results = health_checker.check_all().await;
        let unhealthy: Vec<_> = results
            .iter()
            .filter(|(_, status)| !status.healthy)
            .collect();

        if !unhealthy.is_empty() {
            let mut error_msg = String::from("Unhealthy components found:\n");
            for (name, status) in unhealthy {
                error_msg.push_str(&format!("  - {}: {}\n", name, status.message));
            }
            anyhow::bail!(error_msg);
        }

        Ok(())
    }
}
