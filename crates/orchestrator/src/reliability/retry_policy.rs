//! Retry Policy Implementation
//!
//! Provides configurable retry logic with exponential backoff and jitter
//! for agent operations in production environments.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, warn};

/// Retry policy errors
#[derive(Debug, Error)]
pub enum RetryError {
    #[error("Maximum retry attempts ({max_attempts}) exceeded")]
    MaxAttemptsExceeded { max_attempts: u32 },

    #[error("Maximum retry duration ({max_duration:?}) exceeded")]
    MaxDurationExceeded { max_duration: Duration },

    #[error("Operation failed permanently: {reason}")]
    PermanentFailure { reason: String },

    #[error("Retry policy configuration invalid: {reason}")]
    ConfigurationError { reason: String },
}

/// Backoff strategies for retry delays
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackoffStrategy {
    /// Fixed delay between retries
    Fixed { delay: Duration },

    /// Linear backoff: delay * attempt_number
    Linear {
        initial_delay: Duration,
        increment: Duration,
    },

    /// Exponential backoff: initial_delay * base^attempt_number
    Exponential {
        initial_delay: Duration,
        base: f64,
        max_delay: Duration,
        jitter: bool,
    },

    /// Custom backoff with provided delays
    Custom { delays: Vec<Duration> },
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        Self::Exponential {
            initial_delay: Duration::from_millis(100),
            base: 2.0,
            max_delay: Duration::from_secs(30),
            jitter: true,
        }
    }
}

impl BackoffStrategy {
    /// Calculate delay for given attempt number (0-based)
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        match self {
            Self::Fixed { delay } => *delay,

            Self::Linear {
                initial_delay,
                increment,
            } => *initial_delay + *increment * attempt,

            Self::Exponential {
                initial_delay,
                base,
                max_delay,
                jitter,
            } => {
                let base_delay = initial_delay.as_millis() as f64 * base.powf(attempt as f64);
                let delay = Duration::from_millis(base_delay as u64).min(*max_delay);

                if *jitter {
                    self.add_jitter(delay)
                } else {
                    delay
                }
            }

            Self::Custom { delays } => delays
                .get(attempt as usize)
                .cloned()
                .unwrap_or_else(|| delays.last().cloned().unwrap_or(Duration::from_secs(1))),
        }
    }

    /// Add jitter to delay (±25% random variation)
    fn add_jitter(&self, delay: Duration) -> Duration {
        let mut rng = rand::thread_rng();
        let jitter_factor = rng.gen_range(0.75..=1.25);
        let jittered_millis = (delay.as_millis() as f64 * jitter_factor) as u64;
        Duration::from_millis(jittered_millis)
    }
}

/// Configuration for retry policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Maximum total duration for all retry attempts
    pub max_duration: Duration,

    /// Backoff strategy for retry delays
    pub backoff_strategy: BackoffStrategy,

    /// Whether to retry on specific error types
    pub retry_on_timeout: bool,
    pub retry_on_network_error: bool,
    pub retry_on_temporary_failure: bool,

    /// Custom error patterns that should be retried
    pub retryable_error_patterns: Vec<String>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            max_duration: Duration::from_secs(60),
            backoff_strategy: BackoffStrategy::default(),
            retry_on_timeout: true,
            retry_on_network_error: true,
            retry_on_temporary_failure: true,
            retryable_error_patterns: vec![
                "connection reset".to_string(),
                "network timeout".to_string(),
                "service unavailable".to_string(),
                "rate limit".to_string(),
            ],
        }
    }
}

/// Statistics for retry operations
#[derive(Debug, Clone, Default)]
pub struct RetryStats {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub total_retry_attempts: u64,
    pub average_attempts_per_operation: f64,
    pub total_retry_duration: Duration,
    pub max_retry_duration: Duration,
}

impl RetryStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            0.0
        } else {
            self.successful_operations as f64 / self.total_operations as f64
        }
    }
}

/// Retry policy implementation
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    config: RetryConfig,
    stats: RetryStats,
}

impl RetryPolicy {
    /// Create new retry policy with configuration
    pub fn new(config: RetryConfig) -> Self {
        Self {
            config,
            stats: RetryStats::default(),
        }
    }

    /// Create retry policy with default configuration
    pub fn default() -> Self {
        Self::new(RetryConfig::default())
    }

    /// Execute operation with retry logic
    pub async fn execute<F, Fut, T, E>(&mut self, operation: F) -> Result<T, RetryError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display + std::fmt::Debug,
    {
        let start_time = Instant::now();
        let mut attempt = 0;

        self.stats.total_operations += 1;

        loop {
            debug!(
                attempt = attempt + 1,
                "Executing operation with retry policy"
            );

            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        self.stats.total_retry_attempts += attempt as u64;
                        let duration = start_time.elapsed();
                        self.stats.total_retry_duration += duration;
                        self.stats.max_retry_duration = self.stats.max_retry_duration.max(duration);

                        debug!(
                            attempts = attempt + 1,
                            duration_ms = duration.as_millis(),
                            "Operation succeeded after retries"
                        );
                    }

                    self.stats.successful_operations += 1;
                    self.update_average_attempts();
                    return Ok(result);
                }
                Err(error) => {
                    let should_retry = self.should_retry(&error, attempt);

                    if !should_retry {
                        warn!(
                            error = %error,
                            attempt = attempt + 1,
                            "Operation failed permanently, not retrying"
                        );
                        self.stats.failed_operations += 1;
                        self.update_average_attempts();
                        return Err(RetryError::PermanentFailure {
                            reason: error.to_string(),
                        });
                    }

                    attempt += 1;

                    // Check if we've exceeded maximum attempts
                    if attempt >= self.config.max_attempts {
                        warn!(
                            max_attempts = self.config.max_attempts,
                            "Maximum retry attempts exceeded"
                        );
                        self.stats.failed_operations += 1;
                        self.stats.total_retry_attempts += attempt as u64;
                        self.update_average_attempts();
                        return Err(RetryError::MaxAttemptsExceeded {
                            max_attempts: self.config.max_attempts,
                        });
                    }

                    // Check if we've exceeded maximum duration
                    if start_time.elapsed() > self.config.max_duration {
                        warn!(
                            max_duration = ?self.config.max_duration,
                            "Maximum retry duration exceeded"
                        );
                        self.stats.failed_operations += 1;
                        self.stats.total_retry_attempts += attempt as u64;
                        self.update_average_attempts();
                        return Err(RetryError::MaxDurationExceeded {
                            max_duration: self.config.max_duration,
                        });
                    }

                    // Calculate and apply backoff delay
                    let delay = self.config.backoff_strategy.calculate_delay(attempt - 1);
                    debug!(
                        attempt = attempt,
                        delay_ms = delay.as_millis(),
                        error = %error,
                        "Operation failed, retrying after delay"
                    );

                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    /// Check if error should be retried
    fn should_retry<E: std::fmt::Display + std::fmt::Debug>(
        &self,
        error: &E,
        attempt: u32,
    ) -> bool {
        let error_string = error.to_string().to_lowercase();

        // Always respect max attempts
        if attempt >= self.config.max_attempts {
            return false;
        }

        // Check for retryable error patterns
        for pattern in &self.config.retryable_error_patterns {
            if error_string.contains(&pattern.to_lowercase()) {
                return true;
            }
        }

        // Check for common error types
        if self.config.retry_on_timeout && error_string.contains("timeout") {
            return true;
        }

        if self.config.retry_on_network_error
            && (error_string.contains("network")
                || error_string.contains("connection")
                || error_string.contains("dns"))
        {
            return true;
        }

        if self.config.retry_on_temporary_failure
            && (error_string.contains("temporary")
                || error_string.contains("unavailable")
                || error_string.contains("rate limit"))
        {
            return true;
        }

        false
    }

    /// Update average attempts per operation
    fn update_average_attempts(&mut self) {
        if self.stats.total_operations > 0 {
            self.stats.average_attempts_per_operation =
                (self.stats.total_operations + self.stats.total_retry_attempts) as f64
                    / self.stats.total_operations as f64;
        }
    }

    /// Get retry statistics
    pub fn get_stats(&self) -> RetryStats {
        self.stats.clone()
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = RetryStats::default();
    }

    /// Get configuration
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, config: RetryConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_successful_operation_no_retries() {
        let mut policy = RetryPolicy::default();

        let result = policy.execute(|| async { Ok::<i32, &str>(42) }).await;

        assert!(result.is_ok());
        assert_eq!(
            result.expect("Operation failed - converted from unwrap()"),
            42
        );
        assert_eq!(policy.get_stats().total_operations, 1);
        assert_eq!(policy.get_stats().successful_operations, 1);
        assert_eq!(policy.get_stats().total_retry_attempts, 0);
    }

    #[tokio::test]
    async fn test_retry_until_success() {
        let mut policy = RetryPolicy::new(RetryConfig {
            max_attempts: 5,
            backoff_strategy: BackoffStrategy::Fixed {
                delay: Duration::from_millis(10),
            },
            ..Default::default()
        });

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = policy
            .execute(move || {
                let count = counter_clone.fetch_add(1, Ordering::SeqCst);
                async move {
                    if count < 2 {
                        Err("temporary failure")
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(
            result.expect("Operation failed - converted from unwrap()"),
            42
        );
        assert_eq!(policy.get_stats().total_operations, 1);
        assert_eq!(policy.get_stats().successful_operations, 1);
        assert_eq!(policy.get_stats().total_retry_attempts, 2);
    }

    #[tokio::test]
    async fn test_max_attempts_exceeded() {
        let mut policy = RetryPolicy::new(RetryConfig {
            max_attempts: 2,
            backoff_strategy: BackoffStrategy::Fixed {
                delay: Duration::from_millis(1),
            },
            ..Default::default()
        });

        let result = policy
            .execute(|| async { Err::<i32, &str>("always fails") })
            .await;

        assert!(result.is_err());
        matches!(result.unwrap_err(), RetryError::MaxAttemptsExceeded { .. });
        assert_eq!(policy.get_stats().total_operations, 1);
        assert_eq!(policy.get_stats().failed_operations, 1);
        assert_eq!(policy.get_stats().total_retry_attempts, 2);
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        let strategy = BackoffStrategy::Exponential {
            initial_delay: Duration::from_millis(100),
            base: 2.0,
            max_delay: Duration::from_secs(10),
            jitter: false,
        };

        assert_eq!(strategy.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(1), Duration::from_millis(200));
        assert_eq!(strategy.calculate_delay(2), Duration::from_millis(400));
        assert_eq!(strategy.calculate_delay(10), Duration::from_secs(10)); // max_delay
    }

    #[test]
    fn test_linear_backoff_calculation() {
        let strategy = BackoffStrategy::Linear {
            initial_delay: Duration::from_millis(100),
            increment: Duration::from_millis(50),
        };

        assert_eq!(strategy.calculate_delay(0), Duration::from_millis(100));
        assert_eq!(strategy.calculate_delay(1), Duration::from_millis(150));
        assert_eq!(strategy.calculate_delay(2), Duration::from_millis(200));
    }

    #[test]
    fn test_retry_stats_success_rate() {
        let mut stats = RetryStats::default();
        stats.total_operations = 10;
        stats.successful_operations = 8;

        assert_eq!(stats.success_rate(), 0.8);
    }
}
