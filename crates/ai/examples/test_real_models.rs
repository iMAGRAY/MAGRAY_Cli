// Test loading real ONNX models
use ai::{AiConfig, EmbeddingService, RerankingService, ModelLoader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    let config = AiConfig::default();
    println!("Testing real ONNX model loading...");
    println!("Models directory: {:?}", config.models_dir);
    
    // Test model loader
    match ModelLoader::new(&config.models_dir) {
        Ok(loader) => {
            println!("✓ Model loader initialized");
            
            // Test embedding model
            println!("\nTesting embedding model: {}", config.embedding.model_name);
            match EmbeddingService::new(&loader, config.embedding.clone()) {
                Ok(embedding_service) => {
                    println!("✓ Real embedding service loaded successfully!");
                    
                    // Test embedding
                    match embedding_service.embed("Hello world") {
                        Ok(result) => {
                            println!("✓ Embedding generated: {} dimensions", result.embedding.len());
                        }
                        Err(e) => {
                            println!("✗ Embedding generation failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("✗ Failed to load real embedding service: {}", e);
                    println!("  Trying mock fallback...");
                    
                    match EmbeddingService::new_mock(config.embedding.clone()) {
                        Ok(_) => println!("✓ Mock embedding service works"),
                        Err(e) => println!("✗ Mock embedding service failed: {}", e),
                    }
                }
            }
            
            // Test reranking model
            println!("\nTesting reranking model: {}", config.reranking.model_name);
            match RerankingService::new(&loader, config.reranking.clone()) {
                Ok(reranking_service) => {
                    println!("✓ Real reranking service loaded successfully!");
                    
                    // Test reranking
                    let documents = vec![
                        "Hello world".to_string(),
                        "Machine learning is fun".to_string(),
                        "ONNX models are fast".to_string(),
                    ];
                    
                    match reranking_service.rerank("machine learning", &documents) {
                        Ok(results) => {
                            println!("✓ Reranking completed: {} results", results.len());
                            for (i, result) in results.iter().take(3).enumerate() {
                                println!("  {}. {:.3}: {}", i + 1, result.score, &result.document[..30.min(result.document.len())]);
                            }
                        }
                        Err(e) => {
                            println!("✗ Reranking failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("✗ Failed to load real reranking service: {}", e);
                    println!("  Trying mock fallback...");
                    
                    match RerankingService::new_mock(config.reranking.clone()) {
                        Ok(_) => println!("✓ Mock reranking service works"),
                        Err(e) => println!("✗ Mock reranking service failed: {}", e),
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ Model loader initialization failed: {}", e);
        }
    }
    
    Ok(())
}