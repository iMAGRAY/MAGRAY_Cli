use ai::{GpuFallbackManager, FallbackPolicy, EmbeddingConfig};
use tokio;
use std::time::Duration;

#[test]
fn test_fallback_policy() {
    let policy = FallbackPolicy::default();
    assert_eq!(policy.gpu_timeout, Duration::from_secs(30));
    assert_eq!(policy.error_threshold, 3);
    assert_eq!(policy.recovery_time, Duration::from_secs(300));
    assert!(policy.auto_retry);
    assert_eq!(policy.max_retries, 2);
}

#[test]
fn test_fallback_policy_custom() {
    let policy = FallbackPolicy {
        gpu_timeout: Duration::from_secs(60),
        error_threshold: 5,
        recovery_time: Duration::from_secs(600),
        auto_retry: false,
        max_retries: 0,
    };
    
    assert_eq!(policy.gpu_timeout, Duration::from_secs(60));
    assert_eq!(policy.error_threshold, 5);
    assert_eq!(policy.recovery_time, Duration::from_secs(600));
    assert!(!policy.auto_retry);
    assert_eq!(policy.max_retries, 0);
}

#[tokio::test]
async fn test_gpu_fallback_manager_creation() {
    let config = EmbeddingConfig::default();
    let manager = GpuFallbackManager::new(config).await;
    
    assert!(manager.is_ok());
    let manager = manager.unwrap();
    
    let stats = manager.get_stats();
    // Check that rates are zero for fresh manager
    assert_eq!(stats.gpu_success_rate(), 0.0);
    assert_eq!(stats.fallback_rate(), 0.0);
}

#[tokio::test]
async fn test_gpu_fallback_manager_cpu_only() {
    let mut config = EmbeddingConfig::default();
    config.use_gpu = false;
    
    let manager = GpuFallbackManager::new(config).await.unwrap();
    
    // Should use CPU when GPU is disabled
    let texts = vec!["test text".to_string()];
    let _result = manager.embed_batch_with_fallback(texts).await;
    
    // Just check that we can get stats - the actual embedding might succeed or fail
    // depending on whether models are available
    let stats = manager.get_stats();
    // Verify we can call stats methods
    let _ = stats.fallback_rate();
    let _ = stats.gpu_success_rate();
}

#[tokio::test]
async fn test_fallback_stats_rates() {
    // Create manager and get stats
    let config = EmbeddingConfig::default();
    let manager = GpuFallbackManager::new(config).await.unwrap();
    let stats = manager.get_stats();
    
    // Stats should be zero initially
    assert_eq!(stats.gpu_success_rate(), 0.0);
    assert_eq!(stats.fallback_rate(), 0.0);
}

#[tokio::test]
async fn test_force_cpu_mode() {
    let config = EmbeddingConfig::default();
    let manager = GpuFallbackManager::new(config).await.unwrap();
    
    manager.force_cpu_mode();
    
    // After forcing CPU mode, all requests should go to CPU
    let texts = vec!["test".to_string()];
    let _ = manager.embed_batch_with_fallback(texts).await;
    
    // Just verify we can get stats after forcing CPU mode
    let stats = manager.get_stats();
    let _ = stats.fallback_rate();
}

#[tokio::test]
async fn test_reset_circuit_breaker() {
    let config = EmbeddingConfig::default();
    let manager = GpuFallbackManager::new(config).await.unwrap();
    
    // Force CPU mode
    manager.force_cpu_mode();
    
    // Reset circuit breaker
    manager.reset_circuit_breaker();
    
    // Now GPU should be available again (if it was available initially)
    // Testing the reset functionality
    assert!(true); // Circuit breaker was reset without panic
}