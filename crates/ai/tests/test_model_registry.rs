use ai::{ModelInfo, ModelRegistry, ModelType, MODEL_REGISTRY};
use tempfile::TempDir;

#[test]
fn test_model_registry_singleton() {
    let registry1 = &MODEL_REGISTRY;
    let registry2 = &MODEL_REGISTRY;

    // Проверяем что это один и тот же экземпляр
    assert!(std::ptr::eq(registry1, registry2));
}

#[test]
fn test_model_registry_get_model_info() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ModelRegistry::new(temp_dir.path().to_path_buf());

    // qwen3emb должна быть в реестре по умолчанию
    let model = registry.get_model_info("qwen3emb");
    assert!(model.is_some());

    let model_info = model.unwrap();
    assert_eq!(model_info.name, "qwen3emb");
    assert_eq!(model_info.model_type, ModelType::Embedding);
    assert_eq!(model_info.embedding_dim, 1024);
    assert!(model_info.is_default);
}

#[test]
fn test_model_registry_default_models() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ModelRegistry::new(temp_dir.path().to_path_buf());

    // Проверяем наличие моделей по умолчанию (только Qwen3)
    assert!(registry.get_model_info("qwen3emb").is_some());
    assert!(registry.get_model_info("qwen3_reranker").is_some());
}

#[test]
fn test_model_registry_get_default_model() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ModelRegistry::new(temp_dir.path().to_path_buf());

    let default_embedding = registry.get_default_model(ModelType::Embedding);
    assert!(default_embedding.is_some());
    assert_eq!(default_embedding.unwrap().name, "qwen3emb");

    let default_reranker = registry.get_default_model(ModelType::Reranker);
    assert!(default_reranker.is_some());
    assert_eq!(default_reranker.unwrap().name, "qwen3_reranker");
}

#[test]
fn test_model_info() {
    let model = ModelInfo {
        name: "test-model".to_string(),
        model_type: ModelType::Embedding,
        embedding_dim: 384,
        max_length: 512,
        description: "Test model description".to_string(),
        is_default: false,
    };

    assert_eq!(model.name, "test-model");
    assert_eq!(model.model_type, ModelType::Embedding);
    assert_eq!(model.embedding_dim, 384);
    assert_eq!(model.max_length, 512);
    assert_eq!(model.description, "Test model description");
    assert!(!model.is_default);
}

#[test]
fn test_model_registry_register_model() {
    let temp_dir = TempDir::new().unwrap();
    let mut registry = ModelRegistry::new(temp_dir.path().to_path_buf());

    let custom_model = ModelInfo {
        name: "custom-embedding".to_string(),
        model_type: ModelType::Embedding,
        embedding_dim: 512,
        max_length: 256,
        description: "Custom embedding model".to_string(),
        is_default: false,
    };

    registry.register_model(custom_model.clone());

    let retrieved = registry.get_model_info("custom-embedding");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "custom-embedding");
}

#[test]
fn test_model_type_equality() {
    assert_eq!(ModelType::Embedding, ModelType::Embedding);
    assert_eq!(ModelType::Reranker, ModelType::Reranker);
    assert_ne!(ModelType::Embedding, ModelType::Reranker);
}

#[test]
fn test_model_path() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ModelRegistry::new(temp_dir.path().to_path_buf());

    let model_path = registry.get_model_path("qwen3emb");
    assert_eq!(model_path, temp_dir.path().join("qwen3emb"));
}

#[test]
fn test_model_availability() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ModelRegistry::new(temp_dir.path().to_path_buf());

    // Модель не должна быть доступна без файлов
    assert!(!registry.is_model_available("qwen3emb"));

    // Создаём файлы модели
    let model_dir = temp_dir.path().join("qwen3emb");
    std::fs::create_dir_all(&model_dir).unwrap();
    std::fs::write(model_dir.join("model.onnx"), b"dummy").unwrap();
    std::fs::write(model_dir.join("tokenizer.json"), b"{}").unwrap();

    // Теперь модель должна быть доступна
    assert!(registry.is_model_available("qwen3emb"));
}

#[test]
fn test_get_available_models() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ModelRegistry::new(temp_dir.path().to_path_buf());

    // Без файлов моделей список должен быть пустым
    let available = registry.get_available_models(None);
    assert!(available.is_empty());

    // Создаём файлы для одной модели
    let model_dir = temp_dir.path().join("qwen3emb");
    std::fs::create_dir_all(&model_dir).unwrap();
    std::fs::write(model_dir.join("model.onnx"), b"dummy").unwrap();
    std::fs::write(model_dir.join("tokenizer.json"), b"{}").unwrap();

    // Теперь должна быть одна доступная модель
    let available = registry.get_available_models(Some(ModelType::Embedding));
    assert_eq!(available.len(), 1);
    assert_eq!(available[0].name, "qwen3emb");
}

#[test]
fn test_get_recommendations() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ModelRegistry::new(temp_dir.path().to_path_buf());

    // Без установленных моделей должны быть рекомендации
    let recommendations = registry.get_recommendations();
    assert!(!recommendations.is_empty());

    // Должна быть рекомендация загрузить модели по умолчанию
    assert!(recommendations.iter().any(|r| r.contains("qwen3emb")));
    assert!(recommendations.iter().any(|r| r.contains("qwen3_reranker")));
}
