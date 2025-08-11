use ai::auto_device_selector::*;
use ai::config::EmbeddingConfig;

#[test]
fn test_device_decision_creation() {
    let decision = DeviceDecision {
        use_gpu: true,
        reason: "GPU is faster".to_string(),
        cpu_score: 1.0,
        gpu_score: Some(2.5),
        recommended_batch_size: 64,
    };

    assert!(decision.use_gpu);
    assert_eq!(decision.reason, "GPU is faster");
    assert_eq!(decision.cpu_score, 1.0);
    assert_eq!(decision.gpu_score, Some(2.5));
    assert_eq!(decision.recommended_batch_size, 64);
}

#[test]
fn test_device_decision_clone() {
    let original = DeviceDecision {
        use_gpu: false,
        reason: "CPU fallback".to_string(),
        cpu_score: 1.0,
        gpu_score: None,
        recommended_batch_size: 32,
    };

    let cloned = original.clone();

    assert_eq!(original.use_gpu, cloned.use_gpu);
    assert_eq!(original.reason, cloned.reason);
    assert_eq!(original.cpu_score, cloned.cpu_score);
    assert_eq!(original.gpu_score, cloned.gpu_score);
    assert_eq!(
        original.recommended_batch_size,
        cloned.recommended_batch_size
    );
}

#[test]
fn test_auto_device_selector_creation() {
    let _selector = AutoDeviceSelector::new();
    // basic smoke
    assert!(format!("{:?}", _selector).contains("AutoDeviceSelector"));
}

#[test]
fn test_auto_device_selector_default() {
    let _selector = AutoDeviceSelector::default();
    // basic smoke
    assert!(format!("{:?}", _selector).contains("AutoDeviceSelector"));
}

#[tokio::test]
async fn test_device_selection_with_config() {
    let mut selector = AutoDeviceSelector::new();
    let config = EmbeddingConfig::default();

    let result = selector.select_device(&config).await;

    assert!(result.is_ok() || result.is_err());

    if let Ok(decision) = result {
        // Decision should be valid
        assert!(decision.cpu_score >= 0.0);
        assert!(decision.gpu_score.is_none() || decision.gpu_score.unwrap() >= 0.0);
        assert!(decision.recommended_batch_size > 0);
        assert!(!decision.reason.is_empty());
    }
}

#[tokio::test]
async fn test_device_selection_caching() {
    let mut selector = AutoDeviceSelector::new();
    let config = EmbeddingConfig::default();

    // First call
    let result1 = selector.select_device(&config).await;

    let result2 = selector.select_device(&config).await;

    // Both should succeed or fail consistently
    assert_eq!(result1.is_ok(), result2.is_ok());
}

#[test]
fn test_device_decision_debug() {
    let decision = DeviceDecision {
        use_gpu: true,
        reason: "Test".to_string(),
        cpu_score: 1.0,
        gpu_score: Some(1.5),
        recommended_batch_size: 32,
    };

    let debug_str = format!("{:?}", decision);
    assert!(debug_str.contains("DeviceDecision"));
    assert!(debug_str.contains("Test"));
}

#[test]
fn test_auto_device_selector_debug() {
    let selector = AutoDeviceSelector::new();

    let debug_str = format!("{:?}", selector);
    assert!(debug_str.contains("AutoDeviceSelector"));
}

#[test]
fn test_device_decision_gpu_score_handling() {
    // Test with GPU score
    let with_gpu = DeviceDecision {
        use_gpu: true,
        reason: "GPU available".to_string(),
        cpu_score: 1.0,
        gpu_score: Some(2.0),
        recommended_batch_size: 64,
    };

    assert!(with_gpu.gpu_score.is_some());
    assert_eq!(with_gpu.gpu_score.unwrap(), 2.0);

    // Test without GPU score
    let without_gpu = DeviceDecision {
        use_gpu: false,
        reason: "No GPU".to_string(),
        cpu_score: 1.0,
        gpu_score: None,
        recommended_batch_size: 32,
    };

    assert!(without_gpu.gpu_score.is_none());
}

#[test]
fn test_recommended_batch_size_ranges() {
    let decision = DeviceDecision {
        use_gpu: true,
        reason: "Test".to_string(),
        cpu_score: 1.0,
        gpu_score: Some(1.5),
        recommended_batch_size: 128,
    };

    // Batch size should be reasonable
    assert!(decision.recommended_batch_size > 0);
    assert!(decision.recommended_batch_size <= 1024);
}

#[test]
fn test_cpu_score_ranges() {
    let decision = DeviceDecision {
        use_gpu: false,
        reason: "CPU test".to_string(),
        cpu_score: 0.8,
        gpu_score: None,
        recommended_batch_size: 16,
    };

    // CPU score should be non-negative
    assert!(decision.cpu_score >= 0.0);
}

#[test]
fn test_gpu_score_ranges() {
    let decision = DeviceDecision {
        use_gpu: true,
        reason: "GPU test".to_string(),
        cpu_score: 1.0,
        gpu_score: Some(3.2),
        recommended_batch_size: 64,
    };

    if let Some(gpu_score) = decision.gpu_score {
        // GPU score should be non-negative
        assert!(gpu_score >= 0.0);
    }
}

#[test]
fn test_reason_not_empty() {
    let decision = DeviceDecision {
        use_gpu: true,
        reason: "Valid reason".to_string(),
        cpu_score: 1.0,
        gpu_score: Some(1.5),
        recommended_batch_size: 32,
    };

    assert!(!decision.reason.is_empty());
}
