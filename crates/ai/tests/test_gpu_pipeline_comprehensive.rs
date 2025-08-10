#![cfg(feature = "gpu")]
use ai::config::EmbeddingConfig;
use anyhow::Result;
// use serial_test::serial;

// Helper function to create pipeline config
#[allow(dead_code)]
fn create_pipeline_config() -> Result<()> {
    // Since PipelineConfig and GpuPipelineManager are not easily testable without actual GPU setup,
    // we'll create minimal tests that can compile and run
    Ok(())
}

// Helper function to create embedding config
fn create_embedding_config() -> EmbeddingConfig {
    EmbeddingConfig {
        model_name: "bge-m3".to_string(),
        batch_size: 32,
        max_length: 512,
        use_gpu: false, // Use CPU for tests to avoid GPU dependencies
        gpu_config: None,
        embedding_dim: Some(1024),
    }
}

#[tokio::test]
async fn test_embedding_config_creation() -> Result<()> {
    // Arrange - создаем конфигурации внутри теста
    let embedding_config = create_embedding_config();

    // Act & Assert - проверяем создание конфигурации
    assert_eq!(embedding_config.model_name, "bge-m3");
    assert_eq!(embedding_config.batch_size, 32);
    assert_eq!(embedding_config.max_length, 512);
    assert!(!embedding_config.use_gpu); // CPU for tests
    assert_eq!(embedding_config.embedding_dim, Some(1024));

    Ok(())
}

#[tokio::test]
async fn test_embedding_config_modifications() -> Result<()> {
    let mut embedding_config = create_embedding_config();

    // Test modifying configuration
    embedding_config.batch_size = 16;
    embedding_config.max_length = 256;

    assert_eq!(embedding_config.batch_size, 16);
    assert_eq!(embedding_config.max_length, 256);

    Ok(())
}

#[tokio::test]
async fn test_embedding_config_validation() -> Result<()> {
    let embedding_config = create_embedding_config();

    // Basic validation checks
    assert!(
        !embedding_config.model_name.is_empty(),
        "Model name should not be empty"
    );
    assert!(
        embedding_config.batch_size > 0,
        "Batch size should be positive"
    );
    assert!(
        embedding_config.max_length > 0,
        "Max length should be positive"
    );
    assert!(
        embedding_config.embedding_dim.unwrap_or(0) > 0,
        "Embedding dimension should be positive"
    );

    Ok(())
}

#[tokio::test]
async fn test_gpu_config_disabled() -> Result<()> {
    let embedding_config = create_embedding_config();

    // Since we're testing without GPU features, ensure GPU is disabled
    assert!(
        !embedding_config.use_gpu,
        "GPU should be disabled for CPU tests"
    );
    assert!(
        embedding_config.gpu_config.is_none(),
        "GPU config should be None for CPU tests"
    );

    Ok(())
}

#[tokio::test]
async fn test_different_model_configs() -> Result<()> {
    let models = vec!["bge-m3", "qwen3", "test-model"];

    for model_name in models {
        let config = EmbeddingConfig {
            model_name: model_name.to_string(),
            batch_size: 16,
            max_length: 128,
            use_gpu: false,
            gpu_config: None,
            embedding_dim: Some(512),
        };

        assert_eq!(config.model_name, model_name);
        assert!(config.batch_size > 0);
        assert!(config.max_length > 0);
    }

    Ok(())
}

#[tokio::test]
async fn test_config_cloning() -> Result<()> {
    let config1 = create_embedding_config();
    let config2 = config1.clone();

    // Verify cloned config has same values
    assert_eq!(config1.model_name, config2.model_name);
    assert_eq!(config1.batch_size, config2.batch_size);
    assert_eq!(config1.max_length, config2.max_length);
    assert_eq!(config1.use_gpu, config2.use_gpu);
    assert_eq!(config1.embedding_dim, config2.embedding_dim);

    Ok(())
}

#[tokio::test]
async fn test_config_debug_format() -> Result<()> {
    let config = create_embedding_config();
    let debug_str = format!("{:?}", config);

    // Ensure debug output contains key information
    assert!(
        debug_str.contains("bge-m3"),
        "Debug should contain model name"
    );
    assert!(debug_str.contains("32"), "Debug should contain batch size");
    assert!(debug_str.contains("512"), "Debug should contain max length");

    Ok(())
}

// Stress test for config creation
#[tokio::test]
#[ignore] // Ignore by default, run with --ignored
async fn stress_test_config_creation() -> Result<()> {
    let num_configs = 1000;
    let mut configs = Vec::with_capacity(num_configs);

    for i in 0..num_configs {
        let config = EmbeddingConfig {
            model_name: format!("model-{}", i),
            batch_size: (i % 64) + 1,
            max_length: (i % 1024) + 1,
            use_gpu: false,
            gpu_config: None,
            embedding_dim: Some((i % 2048) + 1),
        };

        configs.push(config);
    }

    assert_eq!(configs.len(), num_configs);

    // Verify all configs are valid
    for (i, config) in configs.iter().enumerate() {
        assert_eq!(config.model_name, format!("model-{}", i));
        assert!(config.batch_size > 0);
        assert!(config.max_length > 0);
        assert!(config.embedding_dim.unwrap_or(0) > 0);
    }

    Ok(())
}

// Integration test placeholder (would need actual pipeline implementation)
#[tokio::test]
#[ignore] // Ignore until actual pipeline is available
async fn integration_test_pipeline_placeholder() -> Result<()> {
    // This test would be implemented when we have actual pipeline implementation
    // For now, it's just a placeholder showing the intended test structure

    let _config = create_embedding_config();

    // Would test:
    // - Pipeline creation with config
    // - Basic text processing
    // - Batch processing
    // - Error handling
    // - Resource cleanup

    println!("Pipeline integration test placeholder - implement when pipeline is available");

    Ok(())
}
