//! Unified retry module for all LLM providers
//!
//! This module provides a comprehensive retry framework with exponential backoff,
//! jitter, and provider-specific error detection for robust LLM API communication.
//!
//! ## Features
//! - Exponential backoff with configurable parameters
//! - Jitter support for load distribution
//! - Provider-specific error classification  
//! - Thread-safe and async-compatible
//! - Comprehensive logging and metrics
//!
//! ## Usage
//! ```rust
//! use retry::{RetryConfig, execute_with_retry, RetryableError};
//!
//! let config = RetryConfig::default();
//! let result = execute_with_retry(&config, || async {
//!     // Your async operation here
//!     Ok("success")
//! }).await?;
//! ```

use anyhow::{anyhow, Result};
use rand::Rng;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Configuration for retry behavior with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (excluding initial attempt)
    pub max_retries: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay cap to prevent excessive waiting
    pub max_delay: Duration,
    /// Multiplier for exponential backoff calculation
    pub backoff_multiplier: f64,
    /// Whether to add jitter for load distribution
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create conservative retry config for production
    pub fn conservative() -> Self {
        Self {
            max_retries: 2,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 1.5,
            jitter: true,
        }
    }

    /// Create aggressive retry config for high-availability scenarios
    pub fn aggressive() -> Self {
        Self {
            max_retries: 5,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(120),
            backoff_multiplier: 2.5,
            jitter: true,
        }
    }

    /// Create fast retry config for low-latency requirements
    pub fn fast() -> Self {
        Self {
            max_retries: 2,
            initial_delay: Duration::from_millis(25),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            jitter: false,
        }
    }

    /// Builder pattern for custom configuration
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }
}

/// Trait for errors that can be classified as retryable or non-retryable
pub trait RetryableError {
    /// Returns true if the error is retryable (transient)
    fn is_retryable(&self) -> bool;

    /// Returns the error type for logging purposes
    fn error_type(&self) -> String;

    /// Returns the error message
    fn error_message(&self) -> String;
}

/// Generic error wrapper for retry operations
#[derive(Debug)]
pub struct RetryError {
    pub error_type: String,
    pub message: String,
    pub status_code: Option<u16>,
    pub is_retryable: bool,
}

impl RetryError {
    pub fn new(error_type: String, message: String, is_retryable: bool) -> Self {
        Self {
            error_type,
            message,
            status_code: None,
            is_retryable,
        }
    }

    pub fn with_status_code(mut self, status_code: u16) -> Self {
        self.status_code = Some(status_code);
        self
    }

    /// Create retryable error from HTTP status code
    pub fn from_status_code(status_code: u16, message: String) -> Self {
        let (error_type, is_retryable) = match status_code {
            // Rate limiting - always retryable
            429 => ("rate_limit".to_string(), true),
            // Server errors - retryable
            500..=599 => ("server_error".to_string(), true),
            // Timeout - retryable
            408 => ("timeout".to_string(), true),
            // Client errors - generally not retryable
            400 => ("bad_request".to_string(), false),
            401 => ("unauthorized".to_string(), false),
            403 => ("forbidden".to_string(), false),
            404 => ("not_found".to_string(), false),
            _ => ("unknown".to_string(), false),
        };

        Self {
            error_type,
            message,
            status_code: Some(status_code),
            is_retryable,
        }
    }

    /// Create retryable error from reqwest error
    pub fn from_reqwest_error(error: reqwest::Error) -> Self {
        let is_retryable = error.is_timeout() || error.is_connect();
        Self {
            error_type: if error.is_timeout() {
                "timeout"
            } else if error.is_connect() {
                "network"
            } else {
                "request"
            }
            .to_string(),
            message: error.to_string(),
            status_code: error.status().map(|s| s.as_u16()),
            is_retryable,
        }
    }
}

impl RetryableError for RetryError {
    fn is_retryable(&self) -> bool {
        self.is_retryable
    }

    fn error_type(&self) -> String {
        self.error_type.clone()
    }

    fn error_message(&self) -> String {
        self.message.clone()
    }
}

/// Execute an async operation with retry logic and exponential backoff
///
/// This is the primary retry function that handles:
/// - Exponential backoff calculation with jitter
/// - Error classification and retry decisions
/// - Comprehensive logging and metrics
/// - Thread-safe operation
pub async fn execute_with_retry<F, T, E>(config: &RetryConfig, operation: F) -> Result<T>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'static>>
        + Send
        + Sync,
    E: RetryableError + Send + 'static,
{
    let start_time = Instant::now();
    let mut last_error = None;
    let mut total_delay = Duration::ZERO;

    for attempt in 0..=config.max_retries {
        let attempt_start = Instant::now();

        debug!(
            "Retry attempt {}/{} (total elapsed: {:?})",
            attempt + 1,
            config.max_retries + 1,
            start_time.elapsed()
        );

        match operation().await {
            Ok(result) => {
                let total_elapsed = start_time.elapsed();
                if attempt > 0 {
                    info!(
                        "Operation succeeded after {} retries (total time: {:?}, delays: {:?})",
                        attempt, total_elapsed, total_delay
                    );
                } else {
                    debug!("Operation succeeded on first attempt ({:?})", total_elapsed);
                }
                return Ok(result);
            }
            Err(error) => {
                let attempt_elapsed = attempt_start.elapsed();

                // Check if we should retry
                if attempt == config.max_retries || !error.is_retryable() {
                    error!(
                        "Operation failed permanently: {} (attempt {}/{}, elapsed: {:?})",
                        error.error_message(),
                        attempt + 1,
                        config.max_retries + 1,
                        attempt_elapsed
                    );
                    last_error = Some(error);
                    break;
                }

                // Calculate backoff delay
                let delay = calculate_backoff_delay(config, attempt);
                total_delay += delay;

                warn!(
                    "Operation failed, retrying: {} (attempt {}/{}, delay: {:?}, attempt time: {:?})",
                    error.error_message(),
                    attempt + 1,
                    config.max_retries + 1,
                    delay,
                    attempt_elapsed
                );

                // Store error and wait before retry
                last_error = Some(error);
                tokio::time::sleep(delay).await;
            }
        }
    }

    // All retries exhausted
    if let Some(error) = last_error {
        let total_elapsed = start_time.elapsed();
        Err(anyhow!(
            "Operation failed after {} attempts over {:?}: {} (type: {}, retryable: {})",
            config.max_retries + 1,
            total_elapsed,
            error.error_message(),
            error.error_type(),
            error.is_retryable()
        ))
    } else {
        Err(anyhow!("Unexpected error in retry logic"))
    }
}

/// Calculate backoff delay with exponential backoff and optional jitter
fn calculate_backoff_delay(config: &RetryConfig, attempt: u32) -> Duration {
    let base_delay = config.initial_delay.as_millis() as f64;
    let exponential_delay = base_delay * config.backoff_multiplier.powi(attempt as i32);

    let mut delay =
        Duration::from_millis(exponential_delay.min(config.max_delay.as_millis() as f64) as u64);

    // Apply jitter if enabled
    if config.jitter {
        let jitter_range = delay.as_millis() as f64 * 0.1; // ±10% jitter
        let jitter = rand::thread_rng().gen_range(-jitter_range..jitter_range);
        let jittered_delay = (delay.as_millis() as f64 + jitter).max(0.0) as u64;
        delay = Duration::from_millis(jittered_delay);
    }

    delay
}

/// Simplified retry for streaming operations (no exponential backoff)
///
/// Streaming operations have different retry characteristics:
/// - Faster retry cycles due to connection nature
/// - Less aggressive backoff to maintain near-real-time experience
/// - Limited retry attempts to prevent user experience degradation
pub async fn execute_streaming_with_retry<F, T, E>(
    max_retries: u32,
    base_delay: Duration,
    operation: F,
) -> Result<T>
where
    F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, E>> + Send + 'static>>
        + Send
        + Sync,
    E: RetryableError + Send + 'static,
{
    let mut last_error = None;

    for attempt in 0..=max_retries {
        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    info!("Streaming operation recovered after {} retries", attempt);
                }
                return Ok(result);
            }
            Err(error) => {
                if attempt == max_retries || !error.is_retryable() {
                    last_error = Some(error);
                    break;
                }

                warn!(
                    "Streaming operation failed, retrying: {} (attempt {}/{})",
                    error.error_message(),
                    attempt + 1,
                    max_retries + 1
                );

                last_error = Some(error);
                tokio::time::sleep(base_delay).await;
            }
        }
    }

    if let Some(error) = last_error {
        Err(anyhow!(
            "Streaming operation failed after {} attempts: {}",
            max_retries + 1,
            error.error_message()
        ))
    } else {
        Err(anyhow!("Unexpected error in streaming retry logic"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    // Test error implementation
    #[derive(Debug)]
    struct TestError {
        message: String,
        is_retryable: bool,
    }

    impl RetryableError for TestError {
        fn is_retryable(&self) -> bool {
            self.is_retryable
        }

        fn error_type(&self) -> String {
            "test_error".to_string()
        }

        fn error_message(&self) -> String {
            self.message.clone()
        }
    }

    #[tokio::test]
    async fn test_successful_operation_no_retry() {
        let config = RetryConfig::default();
        let result =
            execute_with_retry(&config, || Box::pin(async { Ok::<i32, TestError>(42) })).await;

        assert!(result.is_ok());
        assert_eq!(
            result.expect("Operation failed - converted from unwrap()"),
            42
        );
    }

    #[tokio::test]
    async fn test_retry_with_eventual_success() {
        let config = RetryConfig::default();
        let counter = Arc::new(Mutex::new(0));

        let result = execute_with_retry(&config, || {
            let counter = counter.clone();
            Box::pin(async move {
                let mut count = counter.lock().expect("Lock should not be poisoned");
                *count += 1;

                if *count < 3 {
                    Err(TestError {
                        message: "temporary failure".to_string(),
                        is_retryable: true,
                    })
                } else {
                    Ok(42)
                }
            })
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(
            result.expect("Operation failed - converted from unwrap()"),
            42
        );
        assert_eq!(*counter.lock().expect("Lock should not be poisoned"), 3);
    }

    #[tokio::test]
    async fn test_non_retryable_error() {
        let config = RetryConfig::default();

        let result: Result<i32> = execute_with_retry(&config, || {
            Box::pin(async {
                Err(TestError {
                    message: "permanent failure".to_string(),
                    is_retryable: false,
                })
            })
        })
        .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("permanent failure"));
    }

    #[tokio::test]
    async fn test_max_retries_exceeded() {
        let config = RetryConfig::new().with_max_retries(2);
        let counter = Arc::new(Mutex::new(0));

        let result: Result<i32> = execute_with_retry(&config, || {
            let counter = counter.clone();
            Box::pin(async move {
                let mut count = counter.lock().expect("Lock should not be poisoned");
                *count += 1;

                Err(TestError {
                    message: "always fails".to_string(),
                    is_retryable: true,
                })
            })
        })
        .await;

        assert!(result.is_err());
        assert_eq!(*counter.lock().expect("Lock should not be poisoned"), 3); // Initial + 2 retries
    }

    #[test]
    fn test_backoff_calculation() {
        let config = RetryConfig::new()
            .with_initial_delay(Duration::from_millis(100))
            .with_backoff_multiplier(2.0)
            .with_jitter(false);

        let delay1 = calculate_backoff_delay(&config, 0);
        let delay2 = calculate_backoff_delay(&config, 1);
        let delay3 = calculate_backoff_delay(&config, 2);

        assert_eq!(delay1, Duration::from_millis(100));
        assert_eq!(delay2, Duration::from_millis(200));
        assert_eq!(delay3, Duration::from_millis(400));
    }

    #[test]
    fn test_max_delay_cap() {
        let config = RetryConfig::new()
            .with_initial_delay(Duration::from_millis(1000))
            .with_backoff_multiplier(10.0)
            .with_max_delay(Duration::from_millis(2000))
            .with_jitter(false);

        let delay = calculate_backoff_delay(&config, 5); // Would be 100s without cap
        assert!(delay <= Duration::from_millis(2000));
    }

    #[test]
    fn test_retry_config_builders() {
        let conservative = RetryConfig::conservative();
        assert_eq!(conservative.max_retries, 2);
        assert!(conservative.jitter);

        let aggressive = RetryConfig::aggressive();
        assert_eq!(aggressive.max_retries, 5);
        assert!(aggressive.jitter);

        let fast = RetryConfig::fast();
        assert_eq!(fast.max_retries, 2);
        assert!(!fast.jitter);
    }

    #[test]
    fn test_retry_error_creation() {
        let error = RetryError::from_status_code(429, "Rate limited".to_string());
        assert!(error.is_retryable());
        assert_eq!(error.error_type(), "rate_limit");

        let error = RetryError::from_status_code(400, "Bad request".to_string());
        assert!(!error.is_retryable());
        assert_eq!(error.error_type(), "bad_request");
    }
}
