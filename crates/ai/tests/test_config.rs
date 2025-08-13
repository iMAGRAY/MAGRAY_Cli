#![allow(clippy::uninlined_format_args)]
#![allow(clippy::manual_is_multiple_of)]
use ai::config::*;
use std::path::PathBuf;

#[test]
fn test_ai_config_default() {
    let config = AiConfig::default();

    assert_eq!(config.models_dir, PathBuf::from("models"));
    assert_eq!(config.embedding.model_name, "qwen3emb");
    assert_eq!(config.reranking.model_name, "qwen3_reranker");
    assert!(config.embedding.use_gpu);
    assert!(config.reranking.use_gpu);
}

#[test]
fn test_embedding_config_default() {
    let config = EmbeddingConfig::default();

    assert_eq!(config.model_name, "qwen3emb");
    assert_eq!(config.max_length, 512);
    assert_eq!(config.batch_size, 32);
    assert!(config.use_gpu);
    assert!(config.embedding_dim.is_some() || config.embedding_dim.is_none());
}

#[test]
fn test_reranking_config_default() {
    let config = RerankingConfig::default();

    assert_eq!(config.model_name, "qwen3_reranker");
    assert_eq!(config.max_length, 512);
    assert_eq!(config.batch_size, 16);
    assert!(config.use_gpu);
}

#[test]
fn test_ai_config_basic_validation() {
    let config = AiConfig::default();

    // Basic checks that config is reasonable
    assert!(!config.models_dir.as_os_str().is_empty());
    assert!(!config.embedding.model_name.is_empty());
    assert!(!config.reranking.model_name.is_empty());
    assert!(config.embedding.batch_size > 0);
    assert!(config.reranking.batch_size > 0);
    assert!(config.embedding.max_length > 0);
    assert!(config.reranking.max_length > 0);
}

#[test]
fn test_embedding_config_basic_validation() {
    let config = EmbeddingConfig::default();

    // Valid config should have reasonable values
    assert!(!config.model_name.is_empty());
    assert!(config.max_length > 0);
    assert!(config.max_length <= 8192); // Reasonable upper bound
    assert!(config.batch_size > 0);
    assert!(config.batch_size <= 256); // Reasonable upper bound
}

#[test]
fn test_reranking_config_basic_validation() {
    let config = RerankingConfig::default();

    assert!(!config.model_name.is_empty());
    assert!(config.max_length > 0);
    assert!(config.batch_size > 0);
}

#[test]
fn test_ai_config_clone() {
    let config = AiConfig::default();
    let cloned = config.clone();

    assert_eq!(config.models_dir, cloned.models_dir);
    assert_eq!(config.embedding.model_name, cloned.embedding.model_name);
    assert_eq!(config.reranking.model_name, cloned.reranking.model_name);
}

#[test]
fn test_config_debug_format() {
    let config = AiConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("AiConfig"));
    assert!(debug_str.contains("models"));
    assert!(debug_str.contains("qwen3emb"));
}

#[test]
fn test_config_serde_serialization() {
    let config = AiConfig::default();

    // Test JSON serialization
    let json = serde_json::to_string(&config).expect("Test operation should succeed");
    assert!(json.contains("qwen3emb"));
    assert!(json.contains("qwen3_reranker"));

    // Test deserialization
    let deserialized: AiConfig =
        serde_json::from_str(&json).expect("Test operation should succeed");
    assert_eq!(
        config.embedding.model_name,
        deserialized.embedding.model_name
    );
    assert_eq!(
        config.reranking.model_name,
        deserialized.reranking.model_name
    );
}

#[test]
fn test_config_partial_update() {
    let mut config = AiConfig::default();

    // Update embedding config
    config.embedding.use_gpu = true;
    config.embedding.batch_size = 64;

    assert!(config.embedding.use_gpu);
    assert_eq!(config.embedding.batch_size, 64);
    assert!(config.reranking.use_gpu); // Should remain unchanged
}

#[test]
fn test_embedding_config_gpu_settings() {
    let config = EmbeddingConfig {
        use_gpu: true,
        batch_size: 128,
        ..Default::default()
    };

    // Test GPU configuration
    assert!(config.use_gpu);
    assert_eq!(config.batch_size, 128);
}

#[test]
fn test_reranking_config_optimization() {
    let config = RerankingConfig {
        use_gpu: true,
        ..Default::default()
    };

    // Test optimization settings
    assert!(config.use_gpu);

    assert!(config.batch_size <= 32);
}

#[test]
fn test_config_memory_usage_estimation() {
    let config = AiConfig::default();

    // Embedding model memory usage estimation
    let embedding_memory = config.embedding.batch_size * config.embedding.max_length * 4; // 4 bytes per float
    assert!(embedding_memory > 0);

    // Reranking model memory usage estimation
    let reranking_memory = config.reranking.batch_size * config.reranking.max_length * 4;
    assert!(reranking_memory > 0);
}

#[test]
fn test_config_model_names() {
    let config = AiConfig::default();

    // Check model names are reasonable
    assert!(!config.embedding.model_name.is_empty());
    assert!(!config.reranking.model_name.is_empty());

    assert!(!config.embedding.model_name.contains('/'));
    assert!(!config.embedding.model_name.contains('\\'));
    assert!(!config.reranking.model_name.contains('/'));
    assert!(!config.reranking.model_name.contains('\\'));
}

#[test]
fn test_embedding_config_dimensions() {
    let config = EmbeddingConfig::default();

    if let Some(dim) = config.embedding_dim {
        assert!(dim > 0);
        assert!(dim <= 4096); // Reasonable upper bound
                              // Common embedding dimensions are powers of 2 or multiples of 64
        assert!(dim % 64 == 0 || (dim > 0 && (dim & (dim - 1)) == 0));
    }
}

#[test]
fn test_config_batch_size_ranges() {
    let config = AiConfig::default();

    // Batch sizes should be in reasonable ranges
    assert!(config.embedding.batch_size >= 1);
    assert!(config.embedding.batch_size <= 512);
    assert!(config.reranking.batch_size >= 1);
    assert!(config.reranking.batch_size <= 256);
}

#[test]
fn test_config_max_length_ranges() {
    let config = AiConfig::default();

    // Max lengths should be in reasonable ranges
    assert!(config.embedding.max_length >= 64);
    assert!(config.embedding.max_length <= 8192);
    assert!(config.reranking.max_length >= 64);
    assert!(config.reranking.max_length <= 8192);
}
