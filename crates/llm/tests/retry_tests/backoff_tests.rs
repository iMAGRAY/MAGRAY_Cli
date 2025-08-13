//! Tests for exponential backoff calculation

#[cfg(test)]
mod tests {
    use crate::retry::*;
    use std::time::Duration;

    // Note: calculate_backoff_delay is private, so we test it through public interface
    // In a real implementation, you'd either make it public for testing or use integration tests

    #[tokio::test]
    async fn test_backoff_through_retry_execution() {
        // This test verifies backoff behavior indirectly through timing
        let config = RetryConfig::new()
            .with_max_retries(2)
            .with_initial_delay(Duration::from_millis(10))
            .with_backoff_multiplier(2.0)
            .with_jitter(false);

        let start = std::time::Instant::now();
        let result = execute_with_retry(&config, || {
            Box::pin(async {
                Err::<(), _>(crate::retry::RetryError {
                    error_type: "test".to_string(),
                    message: "always fails".to_string(),
                    status_code: None,
                    is_retryable: true,
                })
            })
        }).await;

        let elapsed = start.elapsed();
        
        // Should fail after: initial_delay (10ms) + backoff_delay (20ms) = ~30ms minimum
        assert!(result.is_err());
        assert!(elapsed >= Duration::from_millis(25)); // Allow some variance
    }
}