use ai::{
    EmbeddingService, RerankingService, ModelLoader,  
    EmbeddingConfig, RerankingConfig
};
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_mock_embedding_service() {
    let config = EmbeddingConfig {
        model_name: "test-model".to_string(),
        max_length: 512,
        batch_size: 4,
        use_gpu: false,
    };
    
    let service = EmbeddingService::new_mock(config).unwrap();
    
    // Test single embedding
    let result = service.embed("Hello world").unwrap();
    assert_eq!(result.text, "Hello world");
    assert_eq!(result.embedding.len(), 768); // Mock dimension
    assert!(result.token_count > 0);
    
    // Test batch embedding
    let texts = vec!["Text 1".to_string(), "Text 2".to_string()];
    let results = service.embed_batch(&texts).unwrap();
    assert_eq!(results.len(), 2);
    
    // Test embedding dimension
    let dim = service.embedding_dim().unwrap();
    assert_eq!(dim, 768);
}

#[tokio::test]
async fn test_mock_reranking_service() {
    let config = RerankingConfig {
        model_name: "test-reranker".to_string(),
        max_length: 512,
        batch_size: 4,
        use_gpu: false,
    };
    
    let service = RerankingService::new_mock(config).unwrap();
    
    let query = "machine learning";
    let documents = vec![
        "Deep learning algorithms".to_string(),
        "Cooking recipes".to_string(),
        "Neural networks".to_string(),
    ];
    
    let results = service.rerank(query, &documents).unwrap();
    assert_eq!(results.len(), 3);
    
    // Verify results are sorted by score (descending)
    for i in 1..results.len() {
        assert!(results[i-1].score >= results[i].score);
    }
    
    // Verify original indices are preserved
    for (_i, result) in results.iter().enumerate() {
        assert!(result.original_index < documents.len());
        assert_eq!(result.query, query);
    }
}

#[tokio::test]
async fn test_model_loader_without_models() {
    let temp_dir = TempDir::new().unwrap();
    let models_dir = temp_dir.path().to_path_buf();
    
    let loader = ModelLoader::new(&models_dir).unwrap();
    
    // Should create directory
    assert!(models_dir.exists());
    
    // Should return empty model list
    let models = loader.list_models().unwrap();
    assert!(models.is_empty());
    
    // Should return false for non-existent model
    assert!(!loader.model_exists("non-existent-model"));
    
    // Should fail to load non-existent model but create mock fallback
    let config = EmbeddingConfig {
        model_name: "non-existent".to_string(),
        max_length: 512,
        batch_size: 4,
        use_gpu: false,
    };
    
    // This should fail since model doesn't exist
    let result = EmbeddingService::new(&loader, config.clone());
    assert!(result.is_err());
    
    // But mock should work
    let mock_service = EmbeddingService::new_mock(config).unwrap();
    let embedding = mock_service.embed("test").unwrap();
    assert_eq!(embedding.embedding.len(), 768);
}

#[tokio::test]
async fn test_onnx_session_debug_info() {
    use ai::models::OnnxSession;
    
    // Test mock session
    let mock_session = OnnxSession::new_mock(
        "test-model".to_string(),
        PathBuf::from("test_model.onnx")
    );
    
    assert!(mock_session.is_mock());
    assert_eq!(mock_session.model_name(), "test-model");
    
    let input_info = mock_session.get_input_info().unwrap();
    assert!(!input_info.is_empty());
    
    let output_info = mock_session.get_output_info().unwrap();
    assert!(!output_info.is_empty());
}

#[tokio::test]
async fn test_embedding_consistency() {
    let config = EmbeddingConfig {
        model_name: "test-model".to_string(),
        max_length: 512,
        batch_size: 4,
        use_gpu: false,
    };
    
    let service = EmbeddingService::new_mock(config).unwrap();
    
    let text = "Consistent test text";
    
    // Generate embedding multiple times
    let result1 = service.embed(text).unwrap();
    let result2 = service.embed(text).unwrap();
    
    // Mock embeddings should be deterministic (hash-based)
    assert_eq!(result1.embedding, result2.embedding);
    assert_eq!(result1.token_count, result2.token_count);
}

#[tokio::test]
async fn test_reranking_score_range() {
    let config = RerankingConfig {
        model_name: "test-reranker".to_string(),
        max_length: 512,
        batch_size: 4,
        use_gpu: false,
    };
    
    let service = RerankingService::new_mock(config).unwrap();
    
    let query = "test query";
    let documents = vec![
        "Very relevant document about testing".to_string(),
        "Somewhat related content".to_string(),
        "Completely unrelated content about cooking".to_string(),
    ];
    
    let results = service.rerank(query, &documents).unwrap();
    
    // All scores should be in [0, 1] range
    for result in &results {
        assert!(result.score >= 0.0);
        assert!(result.score <= 1.0);
    }
    
    // Most relevant should have highest score
    let most_relevant = results.iter()
        .find(|r| r.document.contains("testing"))
        .expect("Should find the most relevant document");
    
    let least_relevant = results.iter()
        .find(|r| r.document.contains("cooking"))
        .expect("Should find the least relevant document");
    
    // This should generally be true for the mock implementation
    assert!(most_relevant.score >= least_relevant.score);
}