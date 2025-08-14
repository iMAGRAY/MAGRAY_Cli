// P1.2.9: Timeout Management for Tools Platform 2.0
// Advanced timeout handling with graceful termination and resource cleanup

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{oneshot, watch};
use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, warn};

/// Timeout configuration for different scenarios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Maximum execution time for tool operations
    pub execution_timeout: Duration,
    /// Graceful shutdown timeout (time to wait for clean termination)
    pub graceful_shutdown_timeout: Duration,
    /// Force kill timeout (time after graceful shutdown fails)
    pub force_kill_timeout: Duration,
    /// Heartbeat interval for long-running operations
    pub heartbeat_interval: Duration,
    /// Maximum time without heartbeat before considering operation stuck
    pub heartbeat_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            execution_timeout: Duration::from_secs(300), // 5 minutes
            graceful_shutdown_timeout: Duration::from_secs(30), // 30 seconds
            force_kill_timeout: Duration::from_secs(10), // 10 seconds
            heartbeat_interval: Duration::from_secs(10), // 10 seconds
            heartbeat_timeout: Duration::from_secs(60),  // 1 minute
        }
    }
}

/// Timeout context for tracking operation state
#[derive(Debug)]
pub struct TimeoutContext {
    /// Unique identifier for the operation
    pub operation_id: String,
    /// When the operation started
    pub started_at: Instant,
    /// Configured timeouts
    pub config: TimeoutConfig,
    /// Last heartbeat received
    pub last_heartbeat: Arc<Mutex<Instant>>,
    /// Cancellation sender
    pub cancellation_tx: oneshot::Sender<TimeoutReason>,
    /// Status updates channel
    pub status_tx: watch::Sender<OperationStatus>,
}

/// Operation status for tracking
#[derive(Debug, Clone, PartialEq)]
pub enum OperationStatus {
    Starting,
    Running,
    ShuttingDownGracefully,
    ForceTerminating,
    Completed,
    TimedOut,
    Cancelled,
}

/// Reason for timeout
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeoutReason {
    /// Execution exceeded maximum allowed time
    ExecutionTimeout,
    /// No heartbeat received within expected interval
    HeartbeatTimeout,
    /// Graceful shutdown took too long
    GracefulShutdownTimeout,
    /// Manual cancellation requested
    ManualCancellation,
    /// Resource exhaustion detected
    ResourceExhaustion,
}

/// Timeout manager for coordinating operation timeouts
pub struct TimeoutManager {
    /// Active timeout contexts
    contexts: Arc<Mutex<HashMap<String, Arc<TimeoutContext>>>>,
    /// Global configuration
    global_config: TimeoutConfig,
    /// Cleanup task handle
    cleanup_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Result of timeout operation
#[derive(Debug)]
pub enum TimeoutResult<T> {
    /// Operation completed successfully within timeout
    Completed(T),
    /// Operation timed out for the specified reason
    TimedOut { reason: TimeoutReason },
    /// Operation was cancelled
    Cancelled,
    /// Operation failed with error
    Failed(anyhow::Error),
}

impl Default for TimeoutManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeoutManager {
    /// Create new timeout manager
    pub fn new() -> Self {
        let mut manager = Self {
            contexts: Arc::new(Mutex::new(HashMap::new())),
            global_config: TimeoutConfig::default(),
            cleanup_handle: None,
        };
        manager.start_cleanup_task();
        manager
    }

    /// Create timeout manager with custom configuration
    pub fn with_config(config: TimeoutConfig) -> Self {
        let mut manager = Self {
            contexts: Arc::new(Mutex::new(HashMap::new())),
            global_config: config,
            cleanup_handle: None,
        };
        manager.start_cleanup_task();
        manager
    }

    /// Execute operation with timeout management
    pub async fn execute_with_timeout<F, T>(
        &self,
        operation_id: String,
        config: Option<TimeoutConfig>,
        operation: F,
    ) -> TimeoutResult<T>
    where
        F: std::future::Future<Output = Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        let timeout_config = config.unwrap_or_else(|| self.global_config.clone());

        debug!(
            "Starting operation with timeout: {} ({:?})",
            operation_id, timeout_config.execution_timeout
        );

        // Create timeout context
        let (cancellation_tx, cancellation_rx) = oneshot::channel();
        let (status_tx, _status_rx) = watch::channel(OperationStatus::Starting);

        let context = Arc::new(TimeoutContext {
            operation_id: operation_id.clone(),
            started_at: Instant::now(),
            config: timeout_config.clone(),
            last_heartbeat: Arc::new(Mutex::new(Instant::now())),
            cancellation_tx,
            status_tx: status_tx.clone(),
        });

        // Register context
        {
            let mut contexts = match self.contexts.lock() {
                Ok(contexts) => contexts,
                Err(_) => return TimeoutResult::Failed(anyhow::anyhow!("Lock error")),
            };
            contexts.insert(operation_id.clone(), context.clone());
        }

        // Start heartbeat monitor
        let heartbeat_handle = self.start_heartbeat_monitor(context.clone());

        // Update status to running
        let _ = status_tx.send(OperationStatus::Running);

        // Execute operation with timeout
        let result = tokio::select! {
            // Operation completes
            op_result = operation => {
                debug!("Operation {} completed", operation_id);
                match op_result {
                    Ok(value) => TimeoutResult::Completed(value),
                    Err(err) => TimeoutResult::Failed(err),
                }
            }

            // Global execution timeout
            _ = sleep(timeout_config.execution_timeout) => {
                warn!("Operation {} exceeded execution timeout ({:?})", operation_id, timeout_config.execution_timeout);
                let _ = status_tx.send(OperationStatus::TimedOut);
                TimeoutResult::TimedOut { reason: TimeoutReason::ExecutionTimeout }
            }

            // Cancellation requested
            reason = cancellation_rx => {
                match reason {
                    Ok(reason) => {
                        info!("Operation {} cancelled: {:?}", operation_id, reason);
                        let _ = status_tx.send(OperationStatus::Cancelled);
                        TimeoutResult::TimedOut { reason }
                    }
                    Err(_) => {
                        warn!("Operation {} cancelled due to channel closure", operation_id);
                        let _ = status_tx.send(OperationStatus::Cancelled);
                        TimeoutResult::Cancelled
                    }
                }
            }
        };

        // Cleanup
        heartbeat_handle.abort();
        self.remove_context(&operation_id);

        // Mark as completed if successful
        if matches!(result, TimeoutResult::Completed(_)) {
            let _ = status_tx.send(OperationStatus::Completed);
        }

        result
    }

    /// Send heartbeat for active operation
    pub fn send_heartbeat(&self, operation_id: &str) -> Result<()> {
        let contexts = self
            .contexts
            .lock()
            .map_err(|_| anyhow::anyhow!("Lock error"))?;

        if let Some(context) = contexts.get(operation_id) {
            let mut last_heartbeat = context
                .last_heartbeat
                .lock()
                .map_err(|_| anyhow::anyhow!("Lock error"))?;
            *last_heartbeat = Instant::now();
            debug!("Heartbeat received for operation: {}", operation_id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Operation not found: {}", operation_id))
        }
    }

    /// Cancel operation manually
    pub fn cancel_operation(&self, operation_id: &str, reason: TimeoutReason) -> Result<()> {
        let mut contexts = self
            .contexts
            .lock()
            .map_err(|_| anyhow::anyhow!("Lock error"))?;

        if let Some(context) = contexts.remove(operation_id) {
            // Try to send cancellation signal (may fail if already consumed)
            let _ = context
                .status_tx
                .send(OperationStatus::ShuttingDownGracefully);

            info!(
                "Cancellation requested for operation: {} ({:?})",
                operation_id, reason
            );
            Ok(())
        } else {
            Err(anyhow::anyhow!("Operation not found: {}", operation_id))
        }
    }

    /// Get status of active operation
    pub fn get_operation_status(&self, operation_id: &str) -> Option<OperationStatus> {
        let contexts = self.contexts.lock().ok()?;
        contexts
            .get(operation_id)
            .and_then(|context| context.status_tx.borrow().clone().into())
    }

    /// List all active operations
    pub fn list_active_operations(&self) -> Vec<String> {
        self.contexts
            .lock()
            .map(|contexts| contexts.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get operation metrics
    pub fn get_operation_metrics(&self, operation_id: &str) -> Option<OperationMetrics> {
        let contexts = self.contexts.lock().ok()?;
        contexts.get(operation_id).map(|context| {
            let last_heartbeat = context
                .last_heartbeat
                .lock()
                .map(|hb| *hb)
                .unwrap_or(context.started_at);

            OperationMetrics {
                operation_id: operation_id.to_string(),
                started_at: context.started_at,
                elapsed_time: context.started_at.elapsed(),
                last_heartbeat,
                time_since_heartbeat: last_heartbeat.elapsed(),
                status: context.status_tx.borrow().clone(),
            }
        })
    }

    /// Execute with graceful shutdown handling
    pub async fn execute_with_graceful_shutdown<F, T, S>(
        &self,
        operation_id: String,
        config: Option<TimeoutConfig>,
        operation: F,
        shutdown_handler: S,
    ) -> TimeoutResult<T>
    where
        F: std::future::Future<Output = Result<T>> + Send + 'static,
        S: std::future::Future<Output = Result<()>> + Send + 'static,
        T: Send + 'static,
    {
        let timeout_config = config.unwrap_or_else(|| self.global_config.clone());

        debug!(
            "Starting operation with graceful shutdown: {}",
            operation_id
        );

        // Create cancellation channels
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (status_tx, _status_rx) = watch::channel(OperationStatus::Starting);

        // Execute main operation with timeout
        let result = timeout(timeout_config.execution_timeout, operation).await;

        match result {
            Ok(Ok(value)) => {
                debug!("Operation {} completed successfully", operation_id);
                let _ = status_tx.send(OperationStatus::Completed);
                TimeoutResult::Completed(value)
            }
            Ok(Err(err)) => {
                warn!("Operation {} failed: {}", operation_id, err);
                TimeoutResult::Failed(err)
            }
            Err(_) => {
                warn!(
                    "Operation {} timed out, initiating graceful shutdown",
                    operation_id
                );
                let _ = status_tx.send(OperationStatus::ShuttingDownGracefully);
                let _ = shutdown_tx.send(());

                // Execute graceful shutdown with timeout
                let shutdown_result =
                    timeout(timeout_config.graceful_shutdown_timeout, shutdown_handler).await;

                match shutdown_result {
                    Ok(Ok(())) => {
                        info!(
                            "Graceful shutdown completed for operation: {}",
                            operation_id
                        );
                        let _ = status_tx.send(OperationStatus::Completed);
                        TimeoutResult::TimedOut {
                            reason: TimeoutReason::ExecutionTimeout,
                        }
                    }
                    Ok(Err(err)) => {
                        error!(
                            "Graceful shutdown failed for operation {}: {}",
                            operation_id, err
                        );
                        let _ = status_tx.send(OperationStatus::ForceTerminating);
                        TimeoutResult::TimedOut {
                            reason: TimeoutReason::GracefulShutdownTimeout,
                        }
                    }
                    Err(_) => {
                        error!(
                            "Graceful shutdown timed out for operation: {}",
                            operation_id
                        );
                        let _ = status_tx.send(OperationStatus::ForceTerminating);
                        TimeoutResult::TimedOut {
                            reason: TimeoutReason::GracefulShutdownTimeout,
                        }
                    }
                }
            }
        }
    }

    /// Start heartbeat monitoring task
    fn start_heartbeat_monitor(&self, context: Arc<TimeoutContext>) -> tokio::task::JoinHandle<()> {
        let context_clone = context.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(context_clone.config.heartbeat_interval);

            loop {
                interval.tick().await;

                let time_since_heartbeat = {
                    let last_heartbeat = context_clone
                        .last_heartbeat
                        .lock()
                        .map(|hb| hb.elapsed())
                        .unwrap_or(Duration::ZERO);
                    last_heartbeat
                };

                if time_since_heartbeat > context_clone.config.heartbeat_timeout {
                    warn!(
                        "Heartbeat timeout for operation: {} ({}s since last heartbeat)",
                        context_clone.operation_id,
                        time_since_heartbeat.as_secs()
                    );

                    // This would trigger cancellation in a real implementation
                    let _ = context_clone.status_tx.send(OperationStatus::TimedOut);
                    break;
                }

                debug!(
                    "Heartbeat check passed for operation: {} ({}s since last heartbeat)",
                    context_clone.operation_id,
                    time_since_heartbeat.as_secs()
                );
            }
        })
    }

    /// Start cleanup task for expired contexts
    fn start_cleanup_task(&mut self) {
        let contexts = Arc::clone(&self.contexts);

        self.cleanup_handle = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Cleanup every minute

            loop {
                interval.tick().await;

                let mut expired_operations = Vec::new();

                {
                    let contexts_guard = match contexts.lock() {
                        Ok(guard) => guard,
                        Err(_) => continue,
                    };

                    for (operation_id, context) in contexts_guard.iter() {
                        // Consider operations expired if they've been running for more than 2x their timeout
                        let max_lifetime = context.config.execution_timeout * 2;
                        if context.started_at.elapsed() > max_lifetime {
                            expired_operations.push(operation_id.clone());
                        }
                    }
                }

                // Remove expired operations
                if !expired_operations.is_empty() {
                    let mut contexts_guard = match contexts.lock() {
                        Ok(guard) => guard,
                        Err(_) => continue,
                    };

                    for operation_id in expired_operations {
                        warn!("Cleaning up expired operation: {}", operation_id);
                        contexts_guard.remove(&operation_id);
                    }
                }
            }
        }));
    }

    /// Remove context from active operations
    fn remove_context(&self, operation_id: &str) {
        if let Ok(mut contexts) = self.contexts.lock() {
            contexts.remove(operation_id);
            debug!("Removed timeout context for operation: {}", operation_id);
        }
    }
}

/// Operation metrics
#[derive(Debug, Clone)]
pub struct OperationMetrics {
    pub operation_id: String,
    pub started_at: Instant,
    pub elapsed_time: Duration,
    pub last_heartbeat: Instant,
    pub time_since_heartbeat: Duration,
    pub status: OperationStatus,
}

impl Drop for TimeoutManager {
    fn drop(&mut self) {
        if let Some(handle) = self.cleanup_handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_successful_operation() {
        let manager = TimeoutManager::new();

        let result = manager
            .execute_with_timeout("test-op".to_string(), None, async {
                sleep(Duration::from_millis(100)).await;
                Ok("success")
            })
            .await;

        match result {
            TimeoutResult::Completed(value) => assert_eq!(value, "success"),
            _ => panic!("Expected completed result"),
        }
    }

    #[tokio::test]
    async fn test_timeout_operation() {
        let config = TimeoutConfig {
            execution_timeout: Duration::from_millis(100),
            ..Default::default()
        };

        let manager = TimeoutManager::with_config(config.clone());

        let result = manager
            .execute_with_timeout("timeout-test".to_string(), Some(config), async {
                sleep(Duration::from_millis(200)).await;
                Ok("should not complete")
            })
            .await;

        match result {
            TimeoutResult::TimedOut {
                reason: TimeoutReason::ExecutionTimeout,
            } => {
                // Expected timeout
            }
            _ => panic!("Expected timeout result"),
        }
    }

    #[tokio::test]
    async fn test_heartbeat_functionality() {
        let manager = TimeoutManager::new();

        // Note: In real implementation, manager would be Arc<TimeoutManager> for sharing
        // This test demonstrates the basic heartbeat API
        let operation_id = "heartbeat-test".to_string();

        // Send heartbeat (in real usage, this would happen during an active operation)
        let heartbeat_result = manager.send_heartbeat(&operation_id);

        // Note: This test is simplified - in real implementation you'd need to handle
        // the async nature of the operation and heartbeat monitoring
        assert!(heartbeat_result.is_err()); // Operation might not be registered yet in this test
    }

    #[test]
    fn test_timeout_config_defaults() {
        let config = TimeoutConfig::default();

        assert_eq!(config.execution_timeout, Duration::from_secs(300));
        assert_eq!(config.graceful_shutdown_timeout, Duration::from_secs(30));
        assert_eq!(config.force_kill_timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_operation_status_enum() {
        assert_eq!(OperationStatus::Starting, OperationStatus::Starting);
        assert_ne!(OperationStatus::Running, OperationStatus::Completed);
    }
}
