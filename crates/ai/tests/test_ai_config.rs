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
    let mut config = EmbeddingConfig::default();
    config.model_name = "custom-model".to_string();
    config.batch_size = 64;
    config.max_length = 1024;
    config.use_gpu = false;
    config.embedding_dim = Some(768);

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
    let mut config = RerankingConfig::default();
    config.model_name = "custom-reranker".to_string();
    config.batch_size = 8;
    config.max_length = 256;
    config.use_gpu = false;

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
    let mut config = AiConfig::default();
    config.models_dir = PathBuf::from("/custom/models");

    assert_eq!(config.models_dir, PathBuf::from("/custom/models"));
}

#[test]
fn test_gpu_config_none() {
    let mut config = EmbeddingConfig::default();
    config.use_gpu = false;
    config.gpu_config = None;

    assert!(!config.use_gpu);
    assert!(config.gpu_config.is_none());
}
