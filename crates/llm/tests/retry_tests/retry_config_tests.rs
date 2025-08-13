//! Tests for RetryConfig functionality

use crate::retry::*;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert_eq!(config.max_delay, Duration::from_secs(60));
        assert_eq!(config.backoff_multiplier, 2.0);
        assert!(config.jitter);
    }

    #[test]
    fn test_retry_config_conservative() {
        let config = RetryConfig::conservative();
        
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.initial_delay, Duration::from_millis(500));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert_eq!(config.backoff_multiplier, 1.5);
        assert!(config.jitter);
    }

    #[test]
    fn test_retry_config_aggressive() {
        let config = RetryConfig::aggressive();
        
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay, Duration::from_millis(50));
        assert_eq!(config.max_delay, Duration::from_secs(120));
        assert_eq!(config.backoff_multiplier, 2.5);
        assert!(config.jitter);
    }

    #[test]
    fn test_retry_config_fast() {
        let config = RetryConfig::fast();
        
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.initial_delay, Duration::from_millis(25));
        assert_eq!(config.max_delay, Duration::from_secs(5));
        assert_eq!(config.backoff_multiplier, 2.0);
        assert!(!config.jitter); // Fast config has no jitter
    }

    #[test]
    fn test_retry_config_builder_pattern() {
        let config = RetryConfig::new()
            .with_max_retries(10)
            .with_initial_delay(Duration::from_millis(200))
            .with_max_delay(Duration::from_secs(30))
            .with_backoff_multiplier(1.8)
            .with_jitter(false);
        
        assert_eq!(config.max_retries, 10);
        assert_eq!(config.initial_delay, Duration::from_millis(200));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert_eq!(config.backoff_multiplier, 1.8);
        assert!(!config.jitter);
    }

    #[test]
    fn test_retry_config_chaining() {
        let config = RetryConfig::conservative()
            .with_max_retries(7)
            .with_jitter(false);
        
        // Should preserve conservative settings except overridden ones
        assert_eq!(config.max_retries, 7); // overridden
        assert_eq!(config.initial_delay, Duration::from_millis(500)); // from conservative
        assert_eq!(config.max_delay, Duration::from_secs(30)); // from conservative
        assert_eq!(config.backoff_multiplier, 1.5); // from conservative
        assert!(!config.jitter); // overridden
    }

    #[test]
    fn test_retry_config_edge_cases() {
        // Test zero retries
        let config = RetryConfig::new().with_max_retries(0);
        assert_eq!(config.max_retries, 0);

        // Test very small delays
        let config = RetryConfig::new()
            .with_initial_delay(Duration::from_millis(1))
            .with_max_delay(Duration::from_millis(2));
        assert_eq!(config.initial_delay, Duration::from_millis(1));
        assert_eq!(config.max_delay, Duration::from_millis(2));

        // Test very large multiplier
        let config = RetryConfig::new().with_backoff_multiplier(10.0);
        assert_eq!(config.backoff_multiplier, 10.0);

        // Test zero multiplier (edge case)
        let config = RetryConfig::new().with_backoff_multiplier(0.0);
        assert_eq!(config.backoff_multiplier, 0.0);
    }

    #[test]
    fn test_retry_config_realistic_scenarios() {
        // Web API scenario
        let web_api_config = RetryConfig::new()
            .with_max_retries(3)
            .with_initial_delay(Duration::from_millis(500))
            .with_max_delay(Duration::from_secs(10))
            .with_backoff_multiplier(2.0)
            .with_jitter(true);
        
        assert!(web_api_config.jitter);
        assert_eq!(web_api_config.max_retries, 3);

        // Database scenario
        let db_config = RetryConfig::conservative()
            .with_max_retries(5)
            .with_initial_delay(Duration::from_millis(100));
        
        assert_eq!(db_config.max_retries, 5);
        assert_eq!(db_config.initial_delay, Duration::from_millis(100));

        // High-frequency trading scenario
        let hft_config = RetryConfig::fast()
            .with_max_retries(1)
            .with_initial_delay(Duration::from_millis(5));
        
        assert_eq!(hft_config.max_retries, 1);
        assert_eq!(hft_config.initial_delay, Duration::from_millis(5));
    }
}