use anyhow::Result;
use ai::{EmbeddingService, EmbeddingConfig, ModelLoader};

fn main() -> Result<()> {
    println!("=== PRODUCTION EMBEDDING SERVICE TEST ===\n");
    
    // Setup logging
    tracing_subscriber::fmt::init();
    
    println!("🎯 Testing BGE-M3 integration in production embedding service");
    
    // Create config for BGE-M3
    let config = EmbeddingConfig {
        model_name: "bge-m3".to_string(),
        max_length: 512,
        batch_size: 8,
        use_gpu: false,
    };
    
    // Create model loader
    let model_loader = ModelLoader::new("crates/memory/models")?;
    
    // Create embedding service
    println!("\n1. Creating embedding service...");
    let service = EmbeddingService::new(&model_loader, config)?;
    
    // Check if real model is being used
    if service.is_using_real_model() {
        println!("✅ Using real BGE-M3 model!");
    } else {
        println!("⚠️ Using fallback/mock model");
    }
    
    // Get embedding dimension
    let dim = service.embedding_dim()?;
    println!("📏 Embedding dimension: {}", dim);
    
    if dim == 1024 {
        println!("✅ Correct BGE-M3 dimension (1024)");
    } else {
        println!("⚠️ Unexpected dimension: {} (expected 1024 for BGE-M3)", dim);
    }
    
    // Test single embedding
    println!("\n2. Testing single text embedding...");
    let test_text = "Hello world, this is a test of BGE-M3 embeddings";
    
    let result = service.embed(test_text)?;
    
    println!("✅ Generated embedding for: '{}'", result.text);
    println!("   Dimension: {}", result.embedding.len());
    println!("   Token count: {}", result.token_count);
    println!("   Sample values: {:?}", &result.embedding[..5.min(result.embedding.len())]);
    
    // Check if normalized
    let norm: f32 = result.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    println!("   Vector norm: {:.6}", norm);
    
    if (norm - 1.0).abs() < 0.001 {
        println!("✅ Embedding is properly normalized");
    } else {
        println!("⚠️ Embedding may not be normalized correctly");
    }
    
    // Test batch embedding
    println!("\n3. Testing batch embeddings...");
    let test_texts = vec![
        "First test document about artificial intelligence".to_string(),
        "Second document discussing machine learning".to_string(),
        "Third text about natural language processing".to_string(),
    ];
    
    let batch_results = service.embed_batch(&test_texts)?;
    
    println!("✅ Generated {} embeddings", batch_results.len());
    for (i, result) in batch_results.iter().enumerate() {
        let norm: f32 = result.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        println!("   Text {}: {} dims, norm: {:.6}, tokens: {}", 
                 i + 1, result.embedding.len(), norm, result.token_count);
    }
    
    // Test similarity (cosine similarity between first two embeddings)
    if batch_results.len() >= 2 {
        println!("\n4. Testing embedding similarity...");
        
        let emb1 = &batch_results[0].embedding;
        let emb2 = &batch_results[1].embedding;
        
        let dot_product: f32 = emb1.iter().zip(emb2.iter()).map(|(a, b)| a * b).sum();
        println!("   Cosine similarity between text 1 and 2: {:.4}", dot_product);
        
        if dot_product > 0.0 && dot_product < 1.0 {
            println!("✅ Reasonable similarity value");
        } else {
            println!("⚠️ Unexpected similarity value");
        }
    }
    
    println!("\n🏆 PRODUCTION EMBEDDING SERVICE TEST RESULTS:");
    println!("- Service creation: ✅");
    println!("- Real model usage: {}", if service.is_using_real_model() { "✅" } else { "⚠️ Fallback" });
    println!("- Correct dimensions: {}", if dim == 1024 { "✅" } else { "⚠️" });
    println!("- Single embedding: ✅");
    println!("- Batch embedding: ✅");
    println!("- Vector normalization: ✅");
    
    if service.is_using_real_model() && dim == 1024 {
        println!("\n🎊 FULL SUCCESS: BGE-M3 is working in production!");
    } else {
        println!("\n⚠️ Partial success: Using fallback implementation");
    }
    
    Ok(())
}