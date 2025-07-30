use ai::{
    EmbeddingService, RerankingService, ModelLoader,
    EmbeddingConfig, RerankingConfig
};
use std::path::PathBuf;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    println!("🚀 Testing Real ONNX Inference");
    
    // Setup model directory - assumes models are placed in project root
    let models_dir = PathBuf::from("models");
    
    if !models_dir.exists() {
        println!("⚠️  Models directory not found at {:?}", models_dir);
        println!("📋 To test real ONNX inference:");
        println!("   1. Create models/ directory");
        println!("   2. Download BGE-small-v1.5 ONNX model to models/bge-small-v1.5/");
        println!("   3. Download BGE-reranker-base ONNX model to models/bge-reranker-base/");
        println!("   4. Ensure tokenizer.json files are present");
        println!();
        println!("🔄 Running with mock implementations instead...");
        test_mock_implementations().await?;
        return Ok(());
    }
    
    let model_loader = ModelLoader::new(&models_dir)?;
    
    // Test embedding service
    println!("\n🔍 Testing Embedding Service");
    test_embedding_service(&model_loader).await?;
    
    // Test reranking service  
    println!("\n📊 Testing Reranking Service");
    test_reranking_service(&model_loader).await?;
    
    println!("\n✅ All tests completed successfully!");
    Ok(())
}

async fn test_embedding_service(model_loader: &ModelLoader) -> Result<(), Box<dyn std::error::Error>> {
    let config = EmbeddingConfig {
        model_name: "bge-small-v1.5".to_string(),
        max_length: 512,
        batch_size: 8,
        use_gpu: false,
    };
    
    let embedding_service = match EmbeddingService::new(model_loader, config.clone()) {
        Ok(service) => {
            println!("✓ Created real embedding service");
            service
        },
        Err(e) => {
            println!("⚠️  Failed to create real embedding service: {}", e);
            println!("🔄 Using mock service instead");
            EmbeddingService::new_mock(config)?
        }
    };
    
    // Test single embedding
    let test_text = "This is a test sentence for embedding generation.";
    let result = embedding_service.embed(test_text)?;
    
    println!("📝 Input text: {}", test_text);
    println!("📐 Embedding dimension: {}", result.embedding.len());
    println!("🔢 Token count: {}", result.token_count);
    println!("📊 First 5 embedding values: {:?}", &result.embedding[..5.min(result.embedding.len())]);
    
    // Test batch embedding
    let test_texts = vec![
        "Machine learning is transforming the world.".to_string(),
        "Natural language processing enables computers to understand text.".to_string(),
        "Deep learning models can generate high-quality embeddings.".to_string(),
    ];
    
    let batch_results = embedding_service.embed_batch(&test_texts)?;
    println!("\n📦 Batch embedding results:");
    for (i, result) in batch_results.iter().enumerate() {
        println!("  Text {}: {} tokens, embedding dim: {}", 
                 i + 1, result.token_count, result.embedding.len());
    }
    
    // Test embedding dimension consistency
    let dim = embedding_service.embedding_dim()?;
    println!("\n📏 Model embedding dimension: {}", dim);
    
    Ok(())
}

async fn test_reranking_service(model_loader: &ModelLoader) -> Result<(), Box<dyn std::error::Error>> {
    let config = RerankingConfig {
        model_name: "bge-reranker-base".to_string(),
        max_length: 512,
        batch_size: 8,
        use_gpu: false,
    };
    
    let reranking_service = match RerankingService::new(&config) {
        Ok(service) => {
            println!("✓ Created real reranking service");
            service
        },
        Err(e) => {
            println!("⚠️  Failed to create real reranking service: {}", e);
            println!("🔄 Using mock service instead");
            RerankingService::new_mock(config)?
        }
    };
    
    let query = "machine learning algorithms";
    let documents = vec![
        "Deep learning is a subset of machine learning that uses neural networks.".to_string(),
        "The weather today is sunny and warm.".to_string(),
        "Support vector machines are popular machine learning algorithms.".to_string(),
        "I love eating pizza for dinner.".to_string(),
        "Random forests and gradient boosting are ensemble learning methods.".to_string(),
    ];
    
    println!("🔍 Query: {}", query);
    println!("📚 Documents to rerank: {}", documents.len());
    
    let rerank_results = reranking_service.rerank(query, &documents)?;
    
    println!("\n🏆 Reranking results (sorted by relevance):");
    for (i, result) in rerank_results.iter().enumerate() {
        println!("  {}. Score: {:.4} | Doc {}: {}", 
                 i + 1, 
                 result.score, 
                 result.original_index,
                 &result.document[..60.min(result.document.len())]);
    }
    
    // Verify results are sorted by score
    let is_sorted = rerank_results.windows(2)
        .all(|window| window[0].score >= window[1].score);
    
    if is_sorted {
        println!("✓ Results are correctly sorted by relevance score");
    } else {
        println!("⚠️  Results are not sorted by relevance score");
    }
    
    Ok(())
}

async fn test_mock_implementations() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎭 Testing Mock Implementations");
    
    // Test mock embedding service
    let embedding_config = EmbeddingConfig {
        model_name: "mock-bge-small".to_string(),
        max_length: 512,
        batch_size: 8,
        use_gpu: false,
    };
    
    let embedding_service = EmbeddingService::new_mock(embedding_config)?;
    let result = embedding_service.embed("Test text for mock embedding")?;
    
    println!("✓ Mock embedding service working");
    println!("  - Embedding dimension: {}", result.embedding.len());
    println!("  - Token count: {}", result.token_count);
    
    // Test mock reranking service
    let reranking_config = RerankingConfig {
        model_name: "mock-bge-reranker".to_string(),
        max_length: 512,
        batch_size: 8,
        use_gpu: false,
    };
    
    let reranking_service = RerankingService::new_mock(reranking_config)?;
    let rerank_results = reranking_service.rerank(
        "test query",
        &vec!["document 1".to_string(), "document 2".to_string()]
    )?;
    
    println!("✓ Mock reranking service working");
    println!("  - Reranked {} documents", rerank_results.len());
    
    Ok(())
}