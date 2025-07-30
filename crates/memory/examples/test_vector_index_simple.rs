use anyhow::Result;
use memory::{MemoryService, MemoryConfig, Record, Layer};
use ai::AiConfig;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== SIMPLE VECTOR INDEX TEST ===\n");
    
    // Setup logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    // Create config
    let mut config = MemoryConfig::default();
    config.ai_config = AiConfig {
        models_dir: PathBuf::from("crates/memory/models"),
        embedding: ai::EmbeddingConfig {
            model_name: "bge-m3".to_string(),
            max_length: 512,
            batch_size: 8,
            use_gpu: false,
        },
        reranking: ai::RerankingConfig {
            model_name: "mxbai".to_string(),
            max_length: 512,
            batch_size: 8,
            use_gpu: false,
        },
    };
    
    println!("1. Creating MemoryService...");
    let memory_service = MemoryService::new(config).await?;
    
    // Insert a few test documents
    let test_docs = vec![
        "Machine learning is revolutionizing AI",
        "Deep learning uses neural networks",
        "Natural language processing is amazing",
    ];
    
    println!("\n2. Inserting {} documents...", test_docs.len());
    
    for (i, text) in test_docs.iter().enumerate() {
        let record = Record {
            id: uuid::Uuid::new_v4(),
            text: text.to_string(),
            embedding: vec![],
            layer: Layer::Interact,
            kind: "test".to_string(),
            tags: vec![format!("doc-{}", i)],
            project: "simple_test".to_string(),
            session: "test".to_string(),
            ts: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
            score: 0.0,
            access_count: 0,
        };
        
        memory_service.insert(record).await?;
        println!("   ✅ Inserted: '{}'", text);
    }
    
    // Search
    println!("\n3. Searching for 'neural networks'...");
    
    let results = memory_service
        .search("neural networks")
        .with_layer(Layer::Interact)
        .top_k(3)
        .execute()
        .await?;
    
    println!("\n📊 RESULTS: Found {} documents", results.len());
    
    if results.is_empty() {
        println!("❌ NO RESULTS - Vector index issue!");
    } else {
        for (i, result) in results.iter().enumerate() {
            println!("   {}. Score: {:.4} | Text: '{}'", 
                     i + 1, result.score, result.text);
        }
        println!("\n✅ Vector index V2 is working!");
    }
    
    Ok(())
}