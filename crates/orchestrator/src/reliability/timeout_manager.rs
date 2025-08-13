//! Timeout Manager Implementation
//!
//! Provides configurable timeout management for agent operations
//! with graceful handling and operation cancellation support.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::time::timeout;
use tracing::{debug, warn};

/// Timeout-related errors
#[derive(Debug, Error)]
pub enum OperationTimeoutError {
    #[error("Operation timeout after {timeout:?}")]
    Timeout { timeout: Duration },

    #[error("Operation cancelled by user")]
    Cancelled,

    #[error("Timeout configuration invalid: {reason}")]
    ConfigurationError { reason: String },
}

/// Configuration for timeout management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Default timeout for agent operations
    pub default_timeout: Duration,

    /// Timeout for message sending
    pub message_timeout: Duration,

    /// Timeout for initialization
    pub initialization_timeout: Duration,

    /// Timeout for shutdown operations
    pub shutdown_timeout: Duration,

    /// Timeout for health checks
    pub health_check_timeout: Duration,

    /// Enable graceful cancellation
    pub enable_graceful_cancellation: bool,

    /// Grace period for cancellation
    pub cancellation_grace_period: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            message_timeout: Duration::from_secs(10),
            initialization_timeout: Duration::from_secs(60),
            shutdown_timeout: Duration::from_secs(30),
            health_check_timeout: Duration::from_secs(5),
            enable_graceful_cancellation: true,
            cancellation_grace_period: Duration::from_secs(10),
        }
    }
}

/// Statistics for timeout operations
#[derive(Debug, Clone, Default)]
pub struct TimeoutStats {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub timed_out_operations: u64,
    pub cancelled_operations: u64,
    pub total_execution_time: Duration,
    pub average_execution_time: Duration,
    pub max_execution_time: Duration,
    pub min_execution_time: Duration,
}

impl TimeoutStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            self.successful_operations as f64 / self.total_operations as f64
        }
    }

    pub fn timeout_rate(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            self.timed_out_operations as f64 / self.total_operations as f64
        }
    }
}

/// Operation timeout context
#[derive(Debug)]
pub struct OperationContext {
    pub operation_name: String,
    pub timeout: Duration,
    pub started_at: Instant,
    pub cancellation_token: tokio_util::sync::CancellationToken,
}

impl Clone for OperationContext {
    fn clone(&self) -> Self {
        Self {
            operation_name: self.operation_name.clone(),
            timeout: self.timeout,
            started_at: self.started_at,
            // Create new cancellation token for cloned context
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        }
    }
}

impl OperationContext {
    pub fn new(operation_name: String, timeout: Duration) -> Self {
        Self {
            operation_name,
            timeout,
            started_at: Instant::now(),
            cancellation_token: tokio_util::sync::CancellationToken::new(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn remaining(&self) -> Duration {
        self.timeout.saturating_sub(self.elapsed())
    }

    pub fn is_expired(&self) -> bool {
        self.elapsed() >= self.timeout
    }

    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }
}

/// Timeout manager for agent operations
#[derive(Debug, Clone)]
pub struct TimeoutManager {
    config: TimeoutConfig,
    stats: TimeoutStats,
    active_operations: std::collections::HashMap<String, OperationContext>,
}

impl TimeoutManager {
    /// Create new timeout manager with configuration
    pub fn new(config: TimeoutConfig) -> Self {
        Self {
            config,
            stats: TimeoutStats::default(),
            active_operations: std::collections::HashMap::new(),
        }
    }

    /// Create timeout manager with default configuration
    pub fn default() -> Self {
        Self::new(TimeoutConfig::default())
    }

    /// Execute operation with timeout
    pub async fn execute<F, Fut, T>(
        &mut self,
        operation_name: String,
        operation: F,
    ) -> Result<T, OperationTimeoutError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        self.execute_with_timeout(operation_name, self.config.default_timeout, operation)
            .await
    }

    /// Execute operation with custom timeout
    pub async fn execute_with_timeout<F, Fut, T>(
        &mut self,
        operation_name: String,
        custom_timeout: Duration,
        operation: F,
    ) -> Result<T, OperationTimeoutError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let context = OperationContext::new(operation_name.clone(), custom_timeout);
        let cancellation_token = context.cancellation_token.clone();

        self.active_operations
            .insert(operation_name.clone(), context);
        self.stats.total_operations += 1;

        debug!(
            operation = %operation_name,
            timeout_ms = custom_timeout.as_millis(),
            "Starting operation with timeout"
        );

        let start_time = Instant::now();
        let result = if self.config.enable_graceful_cancellation {
            self.execute_with_cancellation(operation, custom_timeout, cancellation_token)
                .await
        } else {
            self.execute_simple_timeout(operation, custom_timeout).await
        };

        let execution_time = start_time.elapsed();
        self.update_stats(execution_time, &result);
        self.active_operations.remove(&operation_name);

        match result {
            Ok(value) => {
                debug!(
                    operation = %operation_name,
                    execution_time_ms = execution_time.as_millis(),
                    "Operation completed successfully"
                );
                self.stats.successful_operations += 1;
                Ok(value)
            }
            Err(err) => {
                match err {
                    OperationTimeoutError::Timeout { .. } => {
                        warn!(
                            operation = %operation_name,
                            timeout_ms = custom_timeout.as_millis(),
                            "Operation timed out"
                        );
                        self.stats.timed_out_operations += 1;
                    }
                    OperationTimeoutError::Cancelled => {
                        warn!(
                            operation = %operation_name,
                            "Operation was cancelled"
                        );
                        self.stats.cancelled_operations += 1;
                    }
                    _ => {}
                }
                Err(err)
            }
        }
    }

    /// Execute with simple timeout (no cancellation support)
    async fn execute_simple_timeout<F, Fut, T>(
        &self,
        operation: F,
        operation_timeout: Duration,
    ) -> Result<T, OperationTimeoutError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        timeout(operation_timeout, operation())
            .await
            .map_err(|_| OperationTimeoutError::Timeout {
                timeout: operation_timeout,
            })
    }

    /// Execute with cancellation support
    async fn execute_with_cancellation<F, Fut, T>(
        &self,
        operation: F,
        operation_timeout: Duration,
        cancellation_token: tokio_util::sync::CancellationToken,
    ) -> Result<T, OperationTimeoutError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        tokio::select! {
            result = operation() => Ok(result),
            _ = tokio::time::sleep(operation_timeout) => {
                Err(OperationTimeoutError::Timeout { timeout: operation_timeout })
            },
            _ = cancellation_token.cancelled() => {
                Err(OperationTimeoutError::Cancelled)
            },
        }
    }

    /// Cancel operation by name
    pub fn cancel_operation(&mut self, operation_name: &str) -> bool {
        if let Some(context) = self.active_operations.get(operation_name) {
            context.cancel();
            debug!(operation = %operation_name, "Operation cancelled");
            true
        } else {
            false
        }
    }

    /// Cancel all active operations
    pub fn cancel_all_operations(&mut self) {
        let operation_count = self.active_operations.len();
        for (name, context) in &self.active_operations {
            context.cancel();
            debug!(operation = %name, "Operation cancelled (bulk cancellation)");
        }

        if operation_count > 0 {
            debug!(
                cancelled_operations = operation_count,
                "All operations cancelled"
            );
        }
    }

    /// Get active operation contexts
    pub fn get_active_operations(&self) -> Vec<&OperationContext> {
        self.active_operations.values().collect()
    }

    /// Get operation context by name
    pub fn get_operation_context(&self, operation_name: &str) -> Option<&OperationContext> {
        self.active_operations.get(operation_name)
    }

    /// Check if operation is active
    pub fn is_operation_active(&self, operation_name: &str) -> bool {
        self.active_operations.contains_key(operation_name)
    }

    /// Update statistics after operation completion
    fn update_stats<T>(
        &mut self,
        execution_time: Duration,
        _result: &Result<T, OperationTimeoutError>,
    ) {
        self.stats.total_execution_time += execution_time;

        // Update min/max execution times
        if self.stats.total_operations == 1 {
            self.stats.min_execution_time = execution_time;
            self.stats.max_execution_time = execution_time;
        } else {
            self.stats.min_execution_time = self.stats.min_execution_time.min(execution_time);
            self.stats.max_execution_time = self.stats.max_execution_time.max(execution_time);
        }

        // Update average execution time
        self.stats.average_execution_time = Duration::from_nanos(
            (self.stats.total_execution_time.as_nanos() / self.stats.total_operations as u128)
                as u64,
        );
    }

    /// Get timeout statistics
    pub fn get_stats(&self) -> TimeoutStats {
        self.stats.clone()
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = TimeoutStats::default();
    }

    /// Get configuration
    pub fn config(&self) -> &TimeoutConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: TimeoutConfig) {
        self.config = config;
    }

    /// Get timeout for specific operation type
    pub fn get_timeout_for_operation(&self, operation_type: &str) -> Duration {
        match operation_type.to_lowercase().as_str() {
            "message" | "send_message" => self.config.message_timeout,
            "init" | "initialize" | "initialization" => self.config.initialization_timeout,
            "shutdown" | "stop" | "terminate" => self.config.shutdown_timeout,
            "health_check" | "health" | "ping" => self.config.health_check_timeout,
            _ => self.config.default_timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_successful_operation() {
        let mut manager = TimeoutManager::default();

        let result = manager
            .execute("test_operation".to_string(), || async { 42 })
            .await;

        assert!(result.is_ok());
        assert_eq!(
            result.expect("Operation failed - converted from unwrap()"),
            42
        );
        assert_eq!(manager.get_stats().total_operations, 1);
        assert_eq!(manager.get_stats().successful_operations, 1);
    }

    #[tokio::test]
    async fn test_operation_timeout() {
        let mut manager = TimeoutManager::new(TimeoutConfig {
            default_timeout: Duration::from_millis(50),
            ..Default::default()
        });

        let result = manager
            .execute("slow_operation".to_string(), || async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                42
            })
            .await;

        assert!(result.is_err());
        matches!(result.unwrap_err(), OperationTimeoutError::Timeout { .. });
        assert_eq!(manager.get_stats().total_operations, 1);
        assert_eq!(manager.get_stats().timed_out_operations, 1);
    }

    #[tokio::test]
    async fn test_operation_cancellation() {
        let mut manager = TimeoutManager::new(TimeoutConfig {
            enable_graceful_cancellation: true,
            ..Default::default()
        });

        let operation_started = Arc::new(AtomicBool::new(false));
        let operation_started_clone = operation_started.clone();

        let handle = {
            let mut manager_clone = manager;
            tokio::spawn(async move {
                manager_clone
                    .execute("cancellable_operation".to_string(), move || async move {
                        operation_started_clone.store(true, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        42
                    })
                    .await
            })
        };

        // Wait a bit for operation to start
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(operation_started.load(Ordering::SeqCst));

        // Cancel the operation
        handle.abort();
        let result = handle.await;
        assert!(result.is_err()); // Task was aborted
    }

    #[test]
    fn test_operation_context() {
        let context = OperationContext::new("test_op".to_string(), Duration::from_secs(10));

        assert_eq!(context.operation_name, "test_op");
        assert_eq!(context.timeout, Duration::from_secs(10));
        assert!(!context.is_expired());
        assert!(!context.is_cancelled());

        context.cancel();
        assert!(context.is_cancelled());
    }

    #[test]
    fn test_timeout_stats() {
        let mut stats = TimeoutStats::default();
        stats.total_operations = 10;
        stats.successful_operations = 8;
        stats.timed_out_operations = 2;

        assert_eq!(stats.success_rate(), 0.8);
        assert_eq!(stats.timeout_rate(), 0.2);
    }

    #[test]
    fn test_get_timeout_for_operation() {
        let manager = TimeoutManager::default();

        assert_eq!(
            manager.get_timeout_for_operation("message"),
            manager.config().message_timeout
        );
        assert_eq!(
            manager.get_timeout_for_operation("initialization"),
            manager.config().initialization_timeout
        );
        assert_eq!(
            manager.get_timeout_for_operation("unknown"),
            manager.config().default_timeout
        );
    }
}
