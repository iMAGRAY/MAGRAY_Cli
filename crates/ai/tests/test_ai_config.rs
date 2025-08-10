use ai::{AiConfig, EmbeddingConfig, RerankingConfig};
use std::path::PathBuf;

#[test]
fn test_ai_config_default() {
    let config = AiConfig::default();

    // Проверяем значения по умолчанию
    assert_eq!(config.models_dir, PathBuf::from("models"));

    // Проверяем конфигурацию эмбеддингов
    assert_eq!(config.embedding.model_name, "qwen3emb");
    assert_eq!(config.embedding.batch_size, 32);
    assert_eq!(config.embedding.max_length, 512);
    assert!(config.embedding.use_gpu);
    assert_eq!(config.embedding.embedding_dim, Some(1024));

    // Проверяем конфигурацию ранжирования
    assert_eq!(config.reranking.model_name, "qwen3_reranker");
    assert_eq!(config.reranking.batch_size, 16);
    assert_eq!(config.reranking.max_length, 512);
    assert!(config.reranking.use_gpu);
}

#[test]
fn test_embedding_config_default() {
    let config = EmbeddingConfig::default();

    assert_eq!(config.model_name, "qwen3emb");
    assert_eq!(config.batch_size, 32);
    assert_eq!(config.max_length, 512);
    assert!(config.use_gpu);
    assert_eq!(config.embedding_dim, Some(1024));
}

#[test]
fn test_embedding_config_custom() {
    let config = EmbeddingConfig {
        model_name: "custom-model".to_string(),
        batch_size: 64,
        max_length: 1024,
        use_gpu: false,
        embedding_dim: Some(768),
        ..Default::default()
    };

    assert_eq!(config.model_name, "custom-model");
    assert_eq!(config.batch_size, 64);
    assert_eq!(config.max_length, 1024);
    assert!(!config.use_gpu);
    assert_eq!(config.embedding_dim, Some(768));
}

#[test]
fn test_reranking_config_default() {
    let config = RerankingConfig::default();

    assert_eq!(config.model_name, "qwen3_reranker");
    assert_eq!(config.batch_size, 16);
    assert_eq!(config.max_length, 512);
    assert!(config.use_gpu);
}

#[test]
fn test_reranking_config_custom() {
    let config = RerankingConfig {
        model_name: "custom-reranker".to_string(),
        batch_size: 8,
        max_length: 256,
        use_gpu: false,
        ..Default::default()
    };

    assert_eq!(config.model_name, "custom-reranker");
    assert_eq!(config.batch_size, 8);
    assert_eq!(config.max_length, 256);
    assert!(!config.use_gpu);
}

#[test]
fn test_ai_config_serialization() {
    let config = AiConfig::default();

    // Сериализация в JSON
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("qwen3emb"));
    assert!(json.contains("qwen3_reranker"));

    // Десериализация обратно
    let deserialized: AiConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.models_dir, config.models_dir);
    assert_eq!(
        deserialized.embedding.model_name,
        config.embedding.model_name
    );
}

#[test]
fn test_config_with_custom_models_dir() {
    let config = AiConfig {
        models_dir: PathBuf::from("/custom/models"),
        ..Default::default()
    };

    assert_eq!(config.models_dir, PathBuf::from("/custom/models"));
}

#[test]
fn test_gpu_config_none() {
    let config = EmbeddingConfig {
        use_gpu: false,
        gpu_config: None,
        ..Default::default()
    };

    assert!(!config.use_gpu);
    assert!(config.gpu_config.is_none());
}
