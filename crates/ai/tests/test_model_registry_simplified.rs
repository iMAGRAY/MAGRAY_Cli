use ai::model_registry::{ModelRegistry, ModelInfo, ModelType};
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_model_registry_creation() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ModelRegistry::new(temp_dir.path().to_path_buf());
    
    // Should have some default models registered
    let model_info = registry.get_model_info("qwen3emb");
    assert!(model_info.is_some());
}

#[test]
fn test_model_registry_get_model_info() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ModelRegistry::new(temp_dir.path().to_path_buf());
    
    // Test known models
    let qwen_info = registry.get_model_info("qwen3emb");
    assert!(qwen_info.is_some());
    
    if let Some(info) = qwen_info {
        assert_eq!(info.name, "qwen3emb");
        assert!(info.embedding_dim > 0);
        assert!(info.max_length > 0);
        assert!(!info.description.is_empty());
    }
    
    // Test unknown model
    let unknown_info = registry.get_model_info("nonexistent-model");
    assert!(unknown_info.is_none());
}

#[test]
fn test_model_info_creation() {
    let info = ModelInfo {
        name: "test-model".to_string(),
        model_type: ModelType::Embedding,
        embedding_dim: 768,
        max_length: 512,
        description: "Test model".to_string(),
        is_default: false,
    };
    
    assert_eq!(info.name, "test-model");
    assert_eq!(info.model_type, ModelType::Embedding);
    assert_eq!(info.embedding_dim, 768);
    assert_eq!(info.max_length, 512);
    assert_eq!(info.description, "Test model");
    assert!(!info.is_default);
}

#[test]
fn test_model_info_clone() {
    let original = ModelInfo {
        name: "test-model".to_string(),
        model_type: ModelType::Embedding,
        embedding_dim: 1024,
        max_length: 256,
        description: "Test model".to_string(),
        is_default: true,
    };
    
    let cloned = original.clone();
    
    assert_eq!(original.name, cloned.name);
    assert_eq!(original.model_type, cloned.model_type);
    assert_eq!(original.embedding_dim, cloned.embedding_dim);
    assert_eq!(original.max_length, cloned.max_length);
    assert_eq!(original.description, cloned.description);
    assert_eq!(original.is_default, cloned.is_default);
}

#[test]
fn test_model_info_debug() {
    let info = ModelInfo {
        name: "debug-model".to_string(),
        model_type: ModelType::Reranker,
        embedding_dim: 384,
        max_length: 128,
        description: "Debug test model".to_string(),
        is_default: false,
    };
    
    let debug_str = format!("{:?}", info);
    assert!(debug_str.contains("ModelInfo"));
    assert!(debug_str.contains("debug-model"));
    assert!(debug_str.contains("Reranker"));
    assert!(debug_str.contains("384"));
    assert!(debug_str.contains("128"));
}

#[test]
fn test_model_type_variants() {
    // Test that all model types can be created
    let embedding = ModelType::Embedding;
    let reranker = ModelType::Reranker;
    
    assert_ne!(embedding, reranker);
    
    // Test debug formatting
    let embedding_debug = format!("{:?}", embedding);
    let reranker_debug = format!("{:?}", reranker);
    
    assert_eq!(embedding_debug, "Embedding");
    assert_eq!(reranker_debug, "Reranker");
}

#[test]
fn test_model_type_clone() {
    let original = ModelType::Embedding;
    let cloned = original.clone();
    
    assert_eq!(original, cloned);
}

#[test]
fn test_model_type_equality() {
    let emb1 = ModelType::Embedding;
    let emb2 = ModelType::Embedding;
    let reranker = ModelType::Reranker;
    
    assert_eq!(emb1, emb2);
    assert_ne!(emb1, reranker);
}

#[test]
fn test_model_registry_different_directories() {
    let temp_dir1 = TempDir::new().unwrap();
    let temp_dir2 = TempDir::new().unwrap();
    
    let registry1 = ModelRegistry::new(temp_dir1.path().to_path_buf());
    let registry2 = ModelRegistry::new(temp_dir2.path().to_path_buf());
    
    // Both should have the same default models
    let model1 = registry1.get_model_info("qwen3emb");
    let model2 = registry2.get_model_info("qwen3emb");
    
    assert!(model1.is_some());
    assert!(model2.is_some());
    
    if let (Some(m1), Some(m2)) = (model1, model2) {
        assert_eq!(m1.name, m2.name);
        assert_eq!(m1.model_type, m2.model_type);
        assert_eq!(m1.embedding_dim, m2.embedding_dim);
    }
}

#[test]
fn test_model_registry_path_handling() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_path_buf();
    
    // Test with absolute path
    let registry = ModelRegistry::new(path.clone());
    assert!(registry.get_model_info("qwen3emb").is_some());
    
    // Test with different path formats
    let registry2 = ModelRegistry::new(temp_dir.path().join("subdir"));
    assert!(registry2.get_model_info("qwen3emb").is_some());
}

#[test]
fn test_model_info_field_access() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ModelRegistry::new(temp_dir.path().to_path_buf());
    
    if let Some(info) = registry.get_model_info("qwen3emb") {
        // Test all field access
        let _ = &info.name;
        let _ = &info.model_type;
        let _ = info.embedding_dim;
        let _ = info.max_length;
        let _ = &info.description;
        let _ = info.is_default;
        
        assert!(true); // If we get here, all fields are accessible
    }
}

#[test]
fn test_model_registry_consistency() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ModelRegistry::new(temp_dir.path().to_path_buf());
    
    // Multiple calls should return same info
    let info1 = registry.get_model_info("qwen3emb");
    let info2 = registry.get_model_info("qwen3emb");
    
    match (info1, info2) {
        (Some(i1), Some(i2)) => {
            assert_eq!(i1.name, i2.name);
            assert_eq!(i1.model_type, i2.model_type);
            assert_eq!(i1.embedding_dim, i2.embedding_dim);
            assert_eq!(i1.max_length, i2.max_length);
            assert_eq!(i1.description, i2.description);
            assert_eq!(i1.is_default, i2.is_default);
        }
        _ => panic!("Expected both calls to return Some"),
    }
}

#[test]
fn test_model_registry_bgem3_model() {
    let temp_dir = TempDir::new().unwrap();
    let registry = ModelRegistry::new(temp_dir.path().to_path_buf());
    
    // Test BGE-M3 model if it exists
    if let Some(info) = registry.get_model_info("bge-m3") {
        assert_eq!(info.name, "bge-m3");
        assert_eq!(info.model_type, ModelType::Embedding);
        assert!(info.embedding_dim > 0);
        assert!(info.max_length > 0);
        assert!(!info.description.is_empty());
    }
}