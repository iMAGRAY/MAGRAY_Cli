use anyhow::Result;
use ai::MxbaiRerankerService;
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("=== MXBAI RERANKER ONLY TEST ===\n");
    
    // Setup logging
    tracing_subscriber::fmt::init();
    
    println!("🎯 Testing MXBai reranker directly");
    
    // Setup paths
    let model_path = PathBuf::from("crates/memory/models/mxbai_rerank_base_v2/model.onnx");
    
    println!("📁 Model path: {}", model_path.display());
    
    // Check if model exists
    if !model_path.exists() {
        println!("❌ Model file not found: {}", model_path.display());
        return Ok(());
    }
    
    println!("\n1. Creating MXBai reranker service...");
    let service = match MxbaiRerankerService::new(model_path) {
        Ok(service) => {
            println!("✅ MXBai service created successfully");
            service
        },
        Err(e) => {
            println!("❌ Failed to create MXBai service: {}", e);
            return Ok(());
        }
    };
    
    // Test reranking
    println!("\n2. Testing reranking...");
    let query = "machine learning algorithms";
    let documents = vec![
        "Deep learning neural networks".to_string(),
        "Database management systems".to_string(),
        "Natural language processing".to_string(),
    ];
    
    println!("   Query: '{}'", query);
    println!("   Documents: {} items", documents.len());
    
    let results = match service.rerank(query, &documents, Some(3)) {
        Ok(results) => {
            println!("✅ Reranking completed");
            results
        },
        Err(e) => {
            println!("❌ Reranking failed: {}", e);
            return Ok(());
        }
    };
    
    println!("\n3. Results:");
    for (rank, result) in results.iter().enumerate() {
        println!("   {}. [Doc {}] Score: {:.4}", 
                 rank + 1, result.index + 1, result.score);
        println!("      '{}'", result.document);
    }
    
    println!("\n🏆 MXBAI TEST RESULTS:");
    println!("- Service creation: ✅");
    println!("- Model loading: ✅");
    println!("- Reranking: ✅");
    
    println!("\n✅ MXBai reranker is working!");
    
    Ok(())
}