//! Comprehensive test suite for retry functionality
//! 
//! Tests cover:
//! - RetryConfig creation and configuration
//! - Exponential backoff calculation with jitter
//! - Error classification (retryable/non-retryable)
//! - Integration tests with mock servers
//! - Provider-specific retry behavior
//! - Edge cases and failure scenarios

use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::timeout;

// Re-export test modules
mod retry_config_tests;
mod backoff_tests;
mod error_classification_tests;
mod integration_tests;
mod provider_specific_tests;
mod edge_case_tests;

pub use retry_config_tests::*;
pub use backoff_tests::*;
pub use error_classification_tests::*;
pub use integration_tests::*;
pub use provider_specific_tests::*;
pub use edge_case_tests::*;

/// Test utilities for retry testing
pub struct RetryTestUtils;

impl RetryTestUtils {
    /// Create a counter that increments on each call
    pub fn create_attempt_counter() -> Arc<Mutex<u32>> {
        Arc::new(Mutex::new(0))
    }

    /// Create a failing operation that succeeds after N attempts
    pub fn create_eventually_successful_operation<T: Send + 'static>(
        counter: Arc<Mutex<u32>>,
        fail_until: u32,
        success_value: T,
        error_message: &'static str,
        is_retryable: bool,
    ) -> impl Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, TestError>> + Send + 'static>>
           + Send
           + Sync {
        move || {
            let counter = counter.clone();
            let success_value = success_value;
            Box::pin(async move {
                let mut count = counter.lock().expect("Test operation should succeed");
                *count += 1;
                
                if *count <= fail_until {
                    Err(TestError {
                        message: error_message.to_string(),
                        is_retryable,
                    })
                } else {
                    Ok(success_value)
                }
            })
        }
    }

    /// Measure elapsed time for an operation
    pub async fn measure_time<F, T>(operation: F) -> (Duration, T)
    where
        F: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = operation.await;
        (start.elapsed(), result)
    }

    /// Create a timeout wrapper for tests
    pub async fn with_timeout<F, T>(duration: Duration, future: F) -> Result<T>
    where
        F: std::future::Future<Output = T>,
    {
        timeout(duration, future).await.map_err(|_| {
            anyhow::anyhow!("Test timeout after {:?}", duration)
        })
    }
}

/// Test error type implementing RetryableError
#[derive(Debug, Clone)]
pub struct TestError {
    pub message: String,
    pub is_retryable: bool,
}

impl crate::retry::RetryableError for TestError {
    fn is_retryable(&self) -> bool {
        self.is_retryable
    }

    fn error_type(&self) -> String {
        if self.is_retryable {
            "retryable_test_error".to_string()
        } else {
            "non_retryable_test_error".to_string()
        }
    }

    fn error_message(&self) -> String {
        self.message.clone()
    }
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TestError {}

/// Mock HTTP server for integration testing
pub struct MockServer {
    pub port: u16,
    pub responses: Vec<MockResponse>,
    pub request_count: Arc<Mutex<usize>>,
}

#[derive(Clone)]
pub struct MockResponse {
    pub status_code: u16,
    pub body: String,
    pub delay: Option<Duration>,
}

impl MockServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            responses: Vec::new(),
            request_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn add_response(&mut self, response: MockResponse) {
        self.responses.push(response);
    }

    pub fn get_request_count(&self) -> usize {
        *self.request_count.lock().expect("Test operation should succeed")
    }

    pub async fn start(&self) -> Result<()> {
        // In a real implementation, this would start an HTTP server
        // For now, we'll simulate server behavior in tests
        Ok(())
    }
}

// Common test constants
pub const DEFAULT_TEST_TIMEOUT: Duration = Duration::from_secs(30);
pub const SHORT_TEST_TIMEOUT: Duration = Duration::from_secs(5);
pub const VERY_SHORT_DELAY: Duration = Duration::from_millis(1);
pub const SHORT_DELAY: Duration = Duration::from_millis(10);
pub const MEDIUM_DELAY: Duration = Duration::from_millis(100);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retry::{RetryConfig, execute_with_retry};

    #[tokio::test]
    async fn test_retry_test_utils_counter() {
        let counter = RetryTestUtils::create_attempt_counter();
        
        // Test counter increments
        {
            let mut count = counter.lock().expect("Test operation should succeed");
            assert_eq!(*count, 0);
            *count += 1;
            assert_eq!(*count, 1);
        }
        
        assert_eq!(*counter.lock().expect("Test operation should succeed"), 1);
    }

    #[tokio::test]
    async fn test_retry_test_utils_eventually_successful() {
        let counter = RetryTestUtils::create_attempt_counter();
        let operation = RetryTestUtils::create_eventually_successful_operation(
            counter.clone(),
            2, // fail for 2 attempts
            "success",
            "test failure",
            true,
        );

        let config = RetryConfig::fast(); // Use fast config for testing
        let result = execute_with_retry(&config, operation).await;
        
        assert!(result.is_ok());
        assert_eq!(result.expect("Test operation should succeed"), "success");
        assert_eq!(*counter.lock().expect("Test operation should succeed"), 3); // 2 failures + 1 success
    }

    #[tokio::test]
    async fn test_retry_test_utils_timeout() {
        let result = RetryTestUtils::with_timeout(
            Duration::from_millis(10),
            async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                "completed"
            }
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timeout"));
    }

    #[tokio::test]
    async fn test_retry_test_utils_measure_time() {
        let delay = Duration::from_millis(50);
        let (elapsed, result) = RetryTestUtils::measure_time(async {
            tokio::time::sleep(delay).await;
            "measured"
        }).await;

        assert_eq!(result, "measured");
        // Allow some variance in timing
        assert!(elapsed >= delay && elapsed < delay + Duration::from_millis(20));
    }
}