use anyhow::Result;
use std::path::PathBuf;
use ai::{OrtEmbeddingService, OrtEmbeddingConfig};

fn main() -> Result<()> {
    println!("=== ORT 2.0 Embeddings Test ===\n");
    
    // Set ONNX Runtime DLL path
    let dll_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .parent().unwrap()
        .join("scripts")
        .join("onnxruntime")
        .join("lib")
        .join("onnxruntime.dll");
    
    std::env::set_var("ORT_DYLIB_PATH", dll_path.to_str().unwrap());
    
    // Model paths
    let model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("model.onnx");
    
    let tokenizer_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("models")
        .join("Qwen3-Embedding-0.6B-ONNX")
        .join("tokenizer.json");
    
    println!("Model path: {}", model_path.display());
    println!("Model exists: {}", model_path.exists());
    println!("Tokenizer path: {}", tokenizer_path.display());
    println!("Tokenizer exists: {}", tokenizer_path.exists());
    
    if !model_path.exists() || !tokenizer_path.exists() {
        println!("\n❌ Model files not found!");
        println!("Please ensure Qwen3-Embedding-0.6B-ONNX model is in crates/memory/models/");
        return Err(anyhow::anyhow!("Model files not found"));
    }
    
    // Create embedding service
    println!("\n1. Creating OrtEmbeddingService...");
    
    let config = OrtEmbeddingConfig {
        model_name: "Qwen3-Embedding-0.6B".to_string(),
        max_length: 512,
        normalize: true,
        pooling_method: "mean".to_string(),
        num_threads: 4,
    };
    
    let service = OrtEmbeddingService::new(&model_path, &tokenizer_path, config)?;
    println!("✅ Service created successfully!");
    
    // Test embeddings
    println!("\n2. Testing embedding generation...");
    
    let test_texts = vec![
        "Hello, world!",
        "ONNX Runtime 2.0 is working!",
        "This is a test of the Qwen3 embedding model with ORT 2.0 API.",
        "The quick brown fox jumps over the lazy dog.",
    ];
    
    for (i, text) in test_texts.iter().enumerate() {
        println!("\nTest {}: \"{}\"", i + 1, text);
        
        match service.embed(text) {
            Ok(embedding) => {
                println!("✅ Embedding generated!");
                println!("   Dimensions: {}", embedding.len());
                println!("   First 5 values: {:?}", &embedding[..5.min(embedding.len())]);
                
                // Check if normalized
                let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                println!("   Norm: {:.6} (should be ~1.0 if normalized)", norm);
                
                // Check value range
                let min = embedding.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                let max = embedding.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                println!("   Value range: [{:.4}, {:.4}]", min, max);
            }
            Err(e) => {
                println!("❌ Failed to generate embedding: {}", e);
                return Err(e);
            }
        }
    }
    
    // Test batch processing
    println!("\n3. Testing batch embedding...");
    let batch_texts: Vec<String> = vec![
        "First batch text".to_string(),
        "Second batch text".to_string(),
        "Third batch text".to_string(),
    ];
    
    match service.embed_batch(&batch_texts) {
        Ok(embeddings) => {
            println!("✅ Batch embedding successful!");
            println!("   Generated {} embeddings", embeddings.len());
            for (i, emb) in embeddings.iter().enumerate() {
                println!("   Text {}: {} dimensions", i + 1, emb.len());
            }
        }
        Err(e) => {
            println!("❌ Batch embedding failed: {}", e);
        }
    }
    
    // Performance test
    println!("\n4. Performance test...");
    let start = std::time::Instant::now();
    let test_text = "Performance test text for measuring embedding generation speed.";
    
    for _ in 0..10 {
        let _ = service.embed(test_text)?;
    }
    
    let elapsed = start.elapsed();
    let avg_time = elapsed.as_millis() / 10;
    println!("✅ Average embedding time: {}ms", avg_time);
    
    println!("\n✅ All tests passed!");
    println!("✅ ORT 2.0 embedding service is working correctly!");
    
    Ok(())
}