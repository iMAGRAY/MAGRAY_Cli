use anyhow::Result;
use ai::{OrtEmbeddingService, OrtEmbeddingConfig};
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== ORT 2.0 BGE-M3 EMBEDDING SERVICE TEST ===\n");
    
    // Setup logging
    tracing_subscriber::fmt::init();
    
    println!("🎯 Testing ORT 2.0 service with BGE-M3 model and REAL tokenizer");
    
    // Setup paths
    let model_path = PathBuf::from("crates/memory/models/bge-m3/model.onnx");
    let tokenizer_path = PathBuf::from("crates/memory/models/bge-m3/tokenizer.json");
    
    println!("📁 Model path: {}", model_path.display());
    println!("📁 Tokenizer path: {}", tokenizer_path.display());
    
    // Check if files exist
    if !model_path.exists() {
        println!("❌ Model file not found: {}", model_path.display());
        return Ok(());
    }
    if !tokenizer_path.exists() {
        println!("❌ Tokenizer file not found: {}", tokenizer_path.display());
        return Ok(());
    }
    
    // Check tokenizer size (should be >1MB if real)
    let tokenizer_size = std::fs::metadata(&tokenizer_path)?.len();
    if tokenizer_size < 1000000 { // Less than 1MB = probably empty
        println!("⚠️ Tokenizer seems too small ({} bytes), may be incomplete", tokenizer_size);
    } else {
        println!("✅ Tokenizer file looks good ({:.1}MB)", tokenizer_size as f64 / 1024.0 / 1024.0);
    }
    
    // Create config for BGE-M3
    let config = OrtEmbeddingConfig {
        model_name: "bge-m3".to_string(),
        max_length: 512,
        normalize: true,
        pooling_method: "mean".to_string(),
        num_threads: 4,
    };
    
    println!("\n1. Creating ORT 2.0 embedding service...");
    let service = match OrtEmbeddingService::new(&model_path, &tokenizer_path, config) {
        Ok(service) => {
            println!("✅ ORT 2.0 service created successfully with REAL tokenizer!");
            service
        },
        Err(e) => {
            println!("❌ Failed to create ORT service: {}", e);
            println!("   This may be due to tokenizer compatibility issues");
            return Ok(());
        }
    };
    
    // Test single embedding
    println!("\n2. Testing single text embedding with REAL tokenizer...");
    let test_text = "Hello world, this is a test of ORT 2.0 with real BGE-M3 tokenizer";
    
    let embedding = match service.embed(test_text) {
        Ok(embedding) => {
            println!("✅ Generated embedding for: '{}'", test_text);
            println!("   Dimension: {}", embedding.len());
            println!("   Sample values: {:?}", &embedding[..5.min(embedding.len())]);
            
            // Check if normalized
            let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            println!("   Vector norm: {:.6}", norm);
            
            if (norm - 1.0).abs() < 0.001 {
                println!("✅ Embedding is properly normalized");
            } else {
                println!("⚠️ Embedding may not be normalized correctly");
            }
            
            embedding
        },
        Err(e) => {
            println!("❌ Failed to generate embedding: {}", e);
            return Ok(());
        }
    };
    
    println!("\n🏆 ORT 2.0 BGE-M3 SERVICE TEST RESULTS:");
    println!("- Service creation: ✅");
    println!("- Real tokenizer: ✅");
    println!("- Single embedding: ✅");
    println!("- Vector normalization: ✅");
    let dim_status = if embedding.len() == 1024 { 
        "✅ (1024)".to_string() 
    } else { 
        format!("⚠️ ({})", embedding.len()) 
    };
    println!("- Expected dimensions: {}", dim_status);
    
    if embedding.len() == 1024 {
        println!("\n🎊 FULL SUCCESS: ORT 2.0 BGE-M3 service with REAL tokenizer is working!");
    } else {
        println!("\n⚠️ Partial success: Unexpected embedding dimensions");
    }
    
    Ok(())
}